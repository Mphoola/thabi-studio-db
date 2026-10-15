use crate::{
    connections::ConnectionStatus,
    database::{self, QueryResult, TestResult},
    errors::AppError,
    profiles::{self, ProfileInput},
    state::AppState,
};
use tauri::State;

/// Rows kept per statement until the results grid adds paging (Phase 8).
const MAX_ROWS: usize = 1_000;

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

#[tauri::command]
pub async fn connect_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<ConnectionStatus, AppError> {
    let lookup = id.clone();
    let (_, params) = state
        .blocking(move |connection, credentials| {
            profiles::connect_params(connection, credentials, &lookup)
        })
        .await?;
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

#[tauri::command]
pub async fn list_databases(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let handle = state.connections.session(&id).await?;
    let mut guard = handle.lock().await;
    guard
        .as_mut()
        .ok_or_else(AppError::not_connected)?
        .list_databases()
        .await
}

/// Runs user-authored SQL on the profile's session, as written.
#[tauri::command]
pub async fn run_query(
    id: String,
    sql: String,
    state: State<'_, AppState>,
) -> Result<QueryResult, AppError> {
    let handle = state.connections.session(&id).await?;
    let mut guard = handle.lock().await;
    guard
        .as_mut()
        .ok_or_else(AppError::not_connected)?
        .execute(&sql, MAX_ROWS)
        .await
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
