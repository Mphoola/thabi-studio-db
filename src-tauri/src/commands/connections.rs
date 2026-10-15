use crate::{
    connections::{ConnectionStatus, SessionHandle},
    database::{self, DbObject, ObjectKind, ObjectStructure, QueryResult, Session, TestResult},
    errors::AppError,
    profiles::{self, ProfileInput},
    state::AppState,
};
use tauri::State;

/// Rows kept per statement until the results grid adds paging (Phase 8).
const MAX_ROWS: usize = 1_000;
/// Rows shown by the explorer's read-only data preview.
const PREVIEW_ROWS: usize = 100;

/// Tests the values currently in the connection form. Nothing is saved.
/// `profile_id` lets an edited profile reuse its saved password.
#[tauri::command]
pub async fn test_connection_input(
    input: ProfileInput,
    profile_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<TestResult, AppError> {
    let validated = profiles::validate(input)?;
    let params = state
        .blocking(move |connection, credentials| {
            profiles::connect_params_for_input(
                connection,
                credentials,
                profile_id.as_deref(),
                &validated,
            )
        })
        .await?;
    database::test_connection(&params).await
}

/// Connects a saved profile. `database` opens a different database than the
/// profile's default for this connection only (PostgreSQL can only browse
/// the database it is connected to); any open connection is replaced.
#[tauri::command]
pub async fn connect_connection_profile(
    id: String,
    database: Option<String>,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, AppError> {
    let lookup = id.clone();
    let (_, mut params) = state
        .blocking(move |connection, credentials| {
            profiles::connect_params(connection, credentials, &lookup)
        })
        .await?;
    if let Some(database) = database.filter(|d| !d.trim().is_empty()) {
        state.connections.disconnect(&id).await?;
        params.database = Some(database);
    }
    let status = state.connections.connect(&id, &params).await?;
    // Best effort: the connection is open even if the timestamp can't be saved.
    let _ = state
        .blocking(move |connection, _| profiles::mark_connected(connection, &id))
        .await;
    Ok(status)
}

#[tauri::command]
pub async fn disconnect_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.connections.disconnect(&id).await
}

#[tauri::command]
pub async fn list_connections(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionStatus>, AppError> {
    Ok(state.connections.statuses().await)
}

/// Errors that mean the session itself is gone rather than the request failing.
fn is_connection_failure(error: &AppError) -> bool {
    matches!(
        error.kind,
        "connection_closed" | "connection_refused" | "unreachable" | "connection" | "timeout"
    )
}

/// Runs explorer work on a profile's session. If the session turns out to
/// be dead, it is closed and the caller gets a "connection lost" error so
/// the UI can offer to reconnect instead of showing stale data.
async fn with_session<T>(
    state: &AppState,
    id: &str,
    work: impl AsyncFnOnce(&mut Session) -> Result<T, AppError>,
) -> Result<T, AppError> {
    let handle: SessionHandle = state.connections.session(id).await?;
    let result = {
        let mut guard = handle.lock().await;
        let session = guard.as_mut().ok_or_else(AppError::not_connected)?;
        work(session).await
    };
    match result {
        Err(error) if is_connection_failure(&error) => {
            let _ = state.connections.disconnect(id).await;
            Err(database::error::connection_lost())
        }
        other => other,
    }
}

#[tauri::command]
pub async fn list_databases(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    with_session(&state, &id, async |s| s.list_databases().await).await
}

/// Schemas in a database, or null when the engine has none (MySQL).
#[tauri::command]
pub async fn list_schemas(
    id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Option<Vec<String>>, AppError> {
    with_session(&state, &id, async |s| s.list_schemas(&database).await).await
}

#[tauri::command]
pub async fn list_objects(
    id: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<DbObject>, AppError> {
    let schema = schema.unwrap_or_default();
    with_session(&state, &id, async |s| {
        s.list_objects(&database, &schema).await
    })
    .await
}

#[tauri::command]
pub async fn describe_object(
    id: String,
    database: String,
    schema: Option<String>,
    name: String,
    kind: String,
    state: State<'_, AppState>,
) -> Result<ObjectStructure, AppError> {
    let schema = schema.unwrap_or_default();
    let kind = ObjectKind::from_code(&kind);
    with_session(&state, &id, async |s| {
        s.describe(&database, &schema, &name, kind).await
    })
    .await
}

/// First 100 rows of a table or view, read-only.
#[tauri::command]
pub async fn preview_object(
    id: String,
    database: String,
    schema: Option<String>,
    name: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, AppError> {
    let schema = schema.unwrap_or_default();
    with_session(&state, &id, async |s| {
        s.preview(&database, &schema, &name, PREVIEW_ROWS).await
    })
    .await
}

/// Runs one user-authored statement, as written, on the query tab's own
/// session. Never retried: if the connection drops, the outcome is unknown
/// and the user decides what to do.
#[tauri::command]
pub async fn run_statement(
    id: String,
    tab_id: String,
    database: Option<String>,
    sql: String,
    timeout_seconds: Option<u64>,
    state: State<'_, AppState>,
) -> Result<QueryResult, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let timeout = timeout_seconds
        .filter(|s| *s > 0)
        .map(std::time::Duration::from_secs);
    let result = tab.run(&params, database, &sql, MAX_ROWS, timeout).await;
    if let Err(error) = &result
        && is_connection_failure(error)
    {
        // The tab's session is unusable; open a new one next time.
        state.connections.close_query_tab(&id, &tab_id).await;
        return Err(database::error::connection_lost());
    }
    result
}

/// Stops the statement running in a query tab. Returns false if nothing ran.
#[tauri::command]
pub async fn cancel_statement(
    id: String,
    tab_id: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    Ok(
        match state.connections.existing_query_tab(&id, &tab_id).await {
            Some((tab, params)) => tab.cancel(&params),
            None => false,
        },
    )
}

/// Closes a query tab's session (when the tab closes).
#[tauri::command]
pub async fn close_query_tab(
    id: String,
    tab_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.connections.close_query_tab(&id, &tab_id).await;
    Ok(())
}

/// Trusts an SSH server's host key after the user checked its fingerprint.
/// Returns the stored fingerprint.
#[tauri::command]
pub async fn trust_ssh_host_key(
    host: String,
    port: u16,
    public_key: String,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    state
        .blocking(move |connection, _| {
            crate::ssh::trust_host_key(connection, &host, port, &public_key)
        })
        .await
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgetHostKeysResult {
    /// ~/.ssh/known_hosts still has a key for this server; only the user
    /// can remove it there (ssh-keygen -R).
    still_in_openssh_known_hosts: bool,
}

/// Forgets the SSH host keys Thabi Studio saved for a server.
#[tauri::command]
pub async fn forget_ssh_host_keys(
    host: String,
    port: u16,
    state: State<'_, AppState>,
) -> Result<ForgetHostKeysResult, AppError> {
    state
        .blocking(move |connection, _| {
            Ok(ForgetHostKeysResult {
                still_in_openssh_known_hosts: crate::ssh::forget_host_keys(
                    connection, &host, port,
                )?,
            })
        })
        .await
}
