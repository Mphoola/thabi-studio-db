use crate::{
    errors::AppError,
    settings::{self, AppSettings, LegacyPreferences},
    state::AppState,
};
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
    state
        .blocking(|connection, _| settings::read_settings(connection))
        .await
}

#[tauri::command]
pub async fn import_legacy_preferences(
    preferences: LegacyPreferences,
    state: State<'_, AppState>,
) -> Result<AppSettings, AppError> {
    state
        .blocking(move |connection, _| settings::import_preferences(connection, preferences))
        .await
}

#[tauri::command]
pub async fn set_app_setting(
    key: String,
    value: Value,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    state
        .blocking(move |connection, _| settings::write_setting(connection, &key, &value))
        .await
}
