use crate::{
    commands::connection_profiles::connection_parameters, errors::AppError, state::AppState,
};
use tauri::State;

fn parameters(
    state: &AppState,
    profile_id: &str,
) -> Result<super::connection_profiles::ConnectionParameters, AppError> {
    let database = state.database.lock().map_err(|_| AppError::storage())?;
    connection_parameters(&database, &*state.credentials, profile_id)
}

#[tauri::command]
pub async fn test_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let parameters = parameters(&state, &id)?;
    state
        .connections
        .test(&parameters.profile, parameters.password.as_deref())
        .await
}

#[tauri::command]
pub async fn connect_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<super::super::connections::ConnectionStatus, AppError> {
    let parameters = parameters(&state, &id)?;
    state
        .connections
        .connect(&parameters.profile, parameters.password.as_deref())
        .await
}

#[tauri::command]
pub async fn disconnect_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state.connections.disconnect_profile(&id).await
}

#[tauri::command]
pub async fn list_connected_profiles(state: State<'_, AppState>) -> Result<Vec<String>, AppError> {
    Ok(state.connections.connected_profiles().await)
}
