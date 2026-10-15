use crate::{
    errors::AppError,
    profiles::{self, ConnectionProfile, ProfileInput, ProfileWriteResult},
    state::AppState,
};
use tauri::State;

#[tauri::command]
pub async fn list_connection_profiles(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionProfile>, AppError> {
    state
        .blocking(|connection, _| profiles::list_profiles(connection))
        .await
}

#[tauri::command]
pub async fn create_connection_profile(
    input: ProfileInput,
    state: State<'_, AppState>,
) -> Result<ProfileWriteResult, AppError> {
    state
        .blocking(move |connection, credentials| {
            profiles::create_profile(connection, credentials, input)
        })
        .await
}

#[tauri::command]
pub async fn update_connection_profile(
    id: String,
    input: ProfileInput,
    state: State<'_, AppState>,
) -> Result<ProfileWriteResult, AppError> {
    let profile = profiles::validate(input)?;
    // Settings are changing, so close any open connection that used the old ones.
    state.connections.disconnect(&id).await?;
    state
        .blocking(move |connection, credentials| {
            profiles::update_validated_profile(connection, credentials, &id, profile)
        })
        .await
}

#[tauri::command]
pub async fn duplicate_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<ProfileWriteResult, AppError> {
    state
        .blocking(move |connection, credentials| {
            profiles::duplicate_profile(connection, credentials, &id)
        })
        .await
}

#[tauri::command]
pub async fn delete_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    // Close the live connection before its saved credentials disappear.
    state.connections.disconnect(&id).await?;
    state
        .blocking(move |connection, credentials| {
            profiles::delete_profile(connection, credentials, &id)
        })
        .await
}

#[tauri::command]
pub async fn select_connection_profile(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state
        .blocking(move |connection, _| profiles::select_profile(connection, &id))
        .await
}
