use crate::{
    connections::{ConnectionStatus, SessionHandle},
    database::{self, DbObject, ObjectKind, ObjectStructure, QueryResult, Session, TestResult},
    errors::AppError,
    profiles::{self, ProfileInput},
    state::AppState,
};
use tauri::State;

/// Default rows fetched per statement; the user can raise it in Settings.
const DEFAULT_MAX_ROWS: usize = 10_000;
/// Upper bound on rows kept per statement, whatever the setting says.
const MAX_ROWS_LIMIT: usize = 1_000_000;

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

/// Reads a table or view into an object tab's own session, so its rows
/// scroll, sort and export like a query result. Bounded by `max_rows`.
#[tauri::command]
#[allow(clippy::too_many_arguments)] // Tauri maps each argument from the frontend.
pub async fn open_object_data(
    id: String,
    tab_id: String,
    database: String,
    schema: Option<String>,
    name: String,
    max_rows: Option<usize>,
    filters: Option<Vec<database::filter::RowFilter>>,
    state: State<'_, AppState>,
) -> Result<QueryResult, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let filter =
        database::filter::where_clause(params.engine, filters.as_deref().unwrap_or_default())?;
    let max_rows = max_rows
        .unwrap_or(DEFAULT_MAX_ROWS)
        .clamp(1, MAX_ROWS_LIMIT);
    // Ask for one extra row so the result knows when there are more.
    let sql = database::object_data_sql(
        params.engine,
        &database,
        schema.as_deref().unwrap_or_default(),
        &name,
        &filter,
        max_rows + 1,
    );
    let result = tab.run(&params, Some(database), &sql, max_rows, None).await;
    if let Err(error) = &result
        && is_connection_failure(error)
    {
        state.connections.close_query_tab(&id, &tab_id).await;
        return Err(database::error::connection_lost());
    }
    result
}

/// The statements staged row edits would run, for the user to review.
#[tauri::command]
pub async fn preview_row_changes(
    id: String,
    tab_id: String,
    target: database::edit::TableTarget,
    changes: Vec<database::edit::RowChange>,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let (_, params) = state.connections.query_tab(&id, &tab_id).await?;
    database::edit::statements(params.engine, &target, &changes)
}

/// Applies staged row edits in one transaction on the table tab's session.
/// All or nothing; returns how many statements ran.
#[tauri::command]
pub async fn apply_row_changes(
    id: String,
    tab_id: String,
    target: database::edit::TableTarget,
    changes: Vec<database::edit::RowChange>,
    state: State<'_, AppState>,
) -> Result<usize, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let statements = database::edit::statements(params.engine, &target, &changes)?;
    let result = tab
        .apply(&params, Some(target.database.clone()), &statements)
        .await;
    if let Err(error) = &result
        && is_connection_failure(error)
    {
        state.connections.close_query_tab(&id, &tab_id).await;
        // If it dropped during COMMIT, the server may or may not have saved.
        return Err(AppError::new(
            "connection_lost",
            "The connection was lost while saving. Reconnect and reload the table to see whether the changes were saved.",
        ));
    }
    result
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
    max_rows: Option<usize>,
    state: State<'_, AppState>,
) -> Result<QueryResult, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    let max_rows = max_rows
        .unwrap_or(DEFAULT_MAX_ROWS)
        .clamp(1, MAX_ROWS_LIMIT);
    let timeout = timeout_seconds
        .filter(|s| *s > 0)
        .map(std::time::Duration::from_secs);
    let ran_in = database.clone().or_else(|| params.database.clone());
    let started = std::time::Instant::now();
    let result = tab.run(&params, database, &sql, max_rows, timeout).await;
    record_history(&state, &id, ran_in, &sql, started.elapsed(), &result).await;
    if let Err(error) = &result
        && is_connection_failure(error)
    {
        // The tab's session is unusable; open a new one next time.
        state.connections.close_query_tab(&id, &tab_id).await;
        return Err(database::error::connection_lost());
    }
    result
}

/// Adds a run to query history (if enabled). Failing to record never fails
/// the query itself.
async fn record_history(
    state: &AppState,
    profile_id: &str,
    database: Option<String>,
    sql: &str,
    elapsed: std::time::Duration,
    result: &Result<QueryResult, AppError>,
) {
    let (status, row_count, error) = match result {
        Ok(r) if !r.columns.is_empty() => ("completed", Some(r.total_rows as i64), None),
        Ok(r) => ("completed", r.rows_affected.map(|n| n as i64), None),
        Err(e) if e.kind == "query_cancelled" => ("cancelled", None, Some(e.message.clone())),
        Err(e) if e.kind == "query_timed_out" => ("timed_out", None, Some(e.message.clone())),
        Err(e) => (
            "failed",
            None,
            Some(e.detail.clone().unwrap_or_else(|| e.message.clone())),
        ),
    };
    let profile_id = profile_id.to_owned();
    let sql = sql.to_owned();
    let _ = state
        .blocking(move |connection, _| {
            let name: String = connection
                .query_row(
                    "SELECT name FROM connection_profiles WHERE id = ?1",
                    [&profile_id],
                    |row| row.get(0),
                )
                .unwrap_or_default();
            crate::library::record(
                connection,
                &crate::library::NewHistoryEntry {
                    profile_id: &profile_id,
                    connection_name: &name,
                    database_name: database.as_deref().unwrap_or_default(),
                    query_text: &sql,
                    duration_ms: elapsed.as_millis() as i64,
                    status,
                    row_count,
                    error_summary: error.as_deref(),
                },
            )
        })
        .await;
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

async fn stored_result(
    state: &AppState,
    id: &str,
    tab_id: &str,
    result_id: &str,
) -> Result<std::sync::Arc<crate::results::StoredResult>, AppError> {
    let (tab, _) = state
        .connections
        .existing_query_tab(id, tab_id)
        .await
        .ok_or_else(|| {
            AppError::new(
                "result_expired",
                "This result is no longer kept. Run the query again.",
            )
        })?;
    tab.result(result_id)
}

/// A page of a kept result, in the grid's current sort order.
#[tauri::command]
pub async fn fetch_rows(
    id: String,
    tab_id: String,
    result_id: String,
    offset: usize,
    limit: usize,
    sort: Option<crate::results::Sort>,
    state: State<'_, AppState>,
) -> Result<Vec<Vec<database::Cell>>, AppError> {
    let result = stored_result(&state, &id, &tab_id, &result_id).await?;
    tauri::async_runtime::spawn_blocking(move || result.page(offset, limit, sort))
        .await
        .map_err(|_| AppError::background_task())
}

/// Selected cells as TSV or CSV text for the clipboard.
#[tauri::command]
pub async fn copy_rows(
    id: String,
    tab_id: String,
    result_id: String,
    scope: crate::results::Scope,
    sort: Option<crate::results::Sort>,
    format: crate::results::TextFormat,
    state: State<'_, AppState>,
) -> Result<String, AppError> {
    let result = stored_result(&state, &id, &tab_id, &result_id).await?;
    tauri::async_runtime::spawn_blocking(move || result.to_text(&scope, sort, format))
        .await
        .map_err(|_| AppError::background_task())
}

/// Writes a kept result to a CSV file the user chose in the save dialog.
#[tauri::command]
pub async fn export_csv(
    id: String,
    tab_id: String,
    result_id: String,
    path: String,
    scope: crate::results::Scope,
    sort: Option<crate::results::Sort>,
    state: State<'_, AppState>,
) -> Result<crate::results::ExportSummary, AppError> {
    let result = stored_result(&state, &id, &tab_id, &result_id).await?;
    let path = crate::results::export_path(&path)?;
    tauri::async_runtime::spawn_blocking(move || {
        let rows = result.export_csv(&path, &scope, sort)?;
        Ok(crate::results::ExportSummary {
            rows,
            path: path.to_string_lossy().into_owned(),
        })
    })
    .await
    .map_err(|_| AppError::background_task())?
}
