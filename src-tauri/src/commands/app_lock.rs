use crate::{
    app_lock::{self, LockStatus},
    app_recovery,
    errors::AppError,
    library, profiles,
    state::AppState,
};
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::{AppHandle, Emitter, State};

/// Tells every window the lock changed, so they all lock or unlock together.
pub(crate) fn announce(app: &AppHandle, state: &AppState) -> LockStatus {
    let status = state.app_lock.status();
    let _ = app.emit("app-lock-changed", status);
    status
}

/// Reads whether a password is set (once), then reports the lock.
#[tauri::command]
pub async fn app_lock_status(state: State<'_, AppState>) -> Result<LockStatus, AppError> {
    if !state.app_lock.is_loaded() {
        let stored = state
            .blocking(|_, credentials| app_lock::read_hash(credentials))
            .await?;
        state.app_lock.load(stored.is_some());
    }
    Ok(state.app_lock.status())
}

/// Checks `password` against the saved hash, counting wrong guesses.
pub(crate) async fn check_password(
    state: &AppState,
    password: String,
    field: &'static str,
) -> Result<(), AppError> {
    state.app_lock.check_attempt()?;
    let correct = state
        .blocking(move |_, credentials| {
            Ok(app_lock::read_hash(credentials)?
                .is_some_and(|stored| app_lock::verify(&stored, &password)))
        })
        .await?;
    state.app_lock.record(correct);
    if correct {
        Ok(())
    } else {
        state.app_lock.check_attempt()?;
        Err(app_lock::wrong_password(field))
    }
}

#[tauri::command]
pub async fn unlock_app(
    password: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LockStatus, AppError> {
    check_password(&state, password, "password").await?;
    state.app_lock.unlock();
    Ok(announce(&app, &state))
}

#[tauri::command]
pub fn lock_app(app: AppHandle, state: State<'_, AppState>) -> LockStatus {
    state.app_lock.lock();
    announce(&app, &state)
}

/// A field error when `password` is too weak to be the app password.
pub(crate) fn check_strength(password: &str) -> Result<(), AppError> {
    match app_lock::strength_error(password) {
        Some(problem) => Err(AppError::validation(BTreeMap::from([(
            "password".to_owned(),
            problem.to_owned(),
        )]))),
        None => Ok(()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordSet {
    pub status: LockStatus,
    /// New recovery codes, shown once. Only when the password is first set;
    /// changing it keeps the codes already handed out.
    pub recovery_codes: Option<Vec<String>>,
}

/// Sets or changes the password. Changing needs the current one.
#[tauri::command]
pub async fn set_app_password(
    current: Option<String>,
    password: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<PasswordSet, AppError> {
    check_strength(&password)?;
    let first = !state.app_lock.status().enabled;
    if !first {
        check_password(&state, current.unwrap_or_default(), "current").await?;
    }
    let recovery_codes = state
        .blocking(move |_, credentials| {
            let codes = if first {
                // Nothing from an earlier password carries over.
                app_recovery::delete_all(credentials)?;
                let codes = app_recovery::generate_codes();
                app_recovery::store_codes(credentials, &codes)?;
                Some(codes)
            } else {
                None
            };
            app_lock::store_hash(credentials, &app_lock::hash(&password)?)?;
            Ok(codes)
        })
        .await?;
    state.app_lock.set_enabled(true);
    Ok(PasswordSet {
        status: announce(&app, &state),
        recovery_codes,
    })
}

#[tauri::command]
pub async fn remove_app_password(
    current: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LockStatus, AppError> {
    check_password(&state, current, "current").await?;
    state
        .blocking(|_, credentials| {
            app_recovery::delete_all(credentials)?;
            app_lock::delete_hash(credentials)
        })
        .await?;
    state.app_lock.set_enabled(false);
    Ok(announce(&app, &state))
}

/// For a forgotten password: closes every connection and deletes every saved
/// connection secret and the query history (which can hold values) along with
/// the app password, so a reset reveals nothing the password protected. Saved
/// connections and saved queries stay; database passwords are typed again.
#[tauri::command]
pub async fn reset_app_password(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LockStatus, AppError> {
    state.connections.disconnect_all().await;
    state
        .blocking(|connection, credentials| {
            profiles::forget_all_secrets(connection, credentials)?;
            library::clear_history(connection)?;
            app_recovery::delete_all(credentials)?;
            app_lock::delete_hash(credentials)
        })
        .await?;
    state.app_lock.set_enabled(false);
    Ok(announce(&app, &state))
}
