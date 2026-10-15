use crate::{
    errors::AppError,
    library::{self, HistoryEntry, SavedQuery, SavedQueryInput, WorkspaceTab},
    state::AppState,
};
use tauri::State;

#[tauri::command]
pub async fn list_history(
    search: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<HistoryEntry>, AppError> {
    state
        .blocking(move |connection, _| {
            library::list_history(
                connection,
                search.as_deref().unwrap_or_default(),
                limit.unwrap_or(100),
                offset.unwrap_or(0),
            )
        })
        .await
}

#[tauri::command]
pub async fn delete_history_entry(id: i64, state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .blocking(move |connection, _| library::delete_history_entry(connection, id))
        .await
}

/// Deletes every history entry. Returns how many were removed.
#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<usize, AppError> {
    state
        .blocking(|connection, _| library::clear_history(connection))
        .await
}

#[tauri::command]
pub async fn list_saved_queries(state: State<'_, AppState>) -> Result<Vec<SavedQuery>, AppError> {
    state
        .blocking(|connection, _| library::list_saved(connection))
        .await
}

#[tauri::command]
pub async fn save_query(
    input: SavedQueryInput,
    state: State<'_, AppState>,
) -> Result<SavedQuery, AppError> {
    state
        .blocking(move |connection, _| library::save_query(connection, input))
        .await
}

#[tauri::command]
pub async fn delete_saved_query(id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    state
        .blocking(move |connection, _| library::delete_saved(connection, &id))
        .await
}

/// Stores the open query tabs so unsaved SQL survives quitting.
#[tauri::command]
pub async fn save_workspace_tabs(
    tabs: Vec<WorkspaceTab>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state
        .blocking(move |connection, _| library::save_tabs(connection, &tabs))
        .await
}

#[tauri::command]
pub async fn load_workspace_tabs(
    state: State<'_, AppState>,
) -> Result<Vec<WorkspaceTab>, AppError> {
    state
        .blocking(|connection, _| library::load_tabs(connection))
        .await
}
