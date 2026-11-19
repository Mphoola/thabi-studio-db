//! Server activity, kill, server facts and storage (Phase 15). They use the
//! connection's main session: none of them belongs to a query tab.

use crate::commands::connections::with_session;
use crate::database::admin::{self, ProcessInfo, ServerFact, TableSize};
use crate::{errors::AppError, state::AppState};
use tauri::State;

#[tauri::command]
pub async fn list_processes(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProcessInfo>, AppError> {
    with_session(&state, &id, async |s| admin::processes(s).await).await
}

/// Cancels a session's query, or ends the session. Refused on a read-only
/// connection: stopping someone else's work is a change to the server.
#[tauri::command]
pub async fn kill_process(
    id: String,
    process_id: String,
    terminate: bool,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    if state.connections.params(&id).await?.read_only {
        return Err(AppError::new(
            "read_only",
            "This connection is read-only. Change its safe mode to stop other sessions.",
        ));
    }
    with_session(&state, &id, async |s| {
        admin::kill(s, &process_id, terminate).await
    })
    .await
}

#[tauri::command]
pub async fn server_overview(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ServerFact>, AppError> {
    with_session(&state, &id, async |s| admin::overview(s).await).await
}

#[tauri::command]
pub async fn storage_overview(
    id: String,
    database: String,
    schema: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TableSize>, AppError> {
    let schema = schema.unwrap_or_default();
    with_session(&state, &id, async |s| {
        admin::storage(s, &database, &schema).await
    })
    .await
}
