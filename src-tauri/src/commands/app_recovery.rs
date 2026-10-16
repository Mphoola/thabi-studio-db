use crate::{
    app_lock::{self, LockStatus},
    app_recovery::{self, QuestionInput, RecoveryOptions},
    commands::app_lock::{announce, check_password, check_strength},
    errors::AppError,
    state::AppState,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

/// What the lock screen can offer when the password is forgotten.
#[tauri::command]
pub async fn recovery_options(state: State<'_, AppState>) -> Result<RecoveryOptions, AppError> {
    state
        .blocking(|_, credentials| app_recovery::options(credentials))
        .await
}

/// Ends a recovery: the new password replaces the forgotten one and unlocks.
async fn finish(
    app: &AppHandle,
    state: &AppState,
    password: String,
) -> Result<LockStatus, AppError> {
    state
        .blocking(move |_, credentials| {
            app_lock::store_hash(credentials, &app_lock::hash(&password)?)
        })
        .await?;
    state.app_lock.set_enabled(true);
    Ok(announce(app, state))
}

fn wrong(field: &str, message: &str) -> AppError {
    AppError::validation(BTreeMap::from([(field.to_owned(), message.to_owned())]))
}

/// Sets a new password with one unused recovery code, which is then used up.
#[tauri::command]
pub async fn recover_with_code(
    code: String,
    password: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LockStatus, AppError> {
    check_strength(&password)?;
    state.app_lock.check_attempt()?;
    let correct = state
        .blocking(move |_, credentials| app_recovery::use_code(credentials, &code))
        .await?;
    state.app_lock.record(correct);
    if !correct {
        state.app_lock.check_attempt()?;
        return Err(wrong(
            "code",
            "That code isn't right, or it was already used.",
        ));
    }
    finish(&app, &state, password).await
}

/// Sets a new password when every security question is answered correctly.
#[tauri::command]
pub async fn recover_with_answers(
    answers: Vec<String>,
    password: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<LockStatus, AppError> {
    check_strength(&password)?;
    state.app_lock.check_attempt()?;
    let correct = state
        .blocking(move |_, credentials| app_recovery::check_answers(credentials, &answers))
        .await?;
    state.app_lock.record(correct);
    if !correct {
        state.app_lock.check_attempt()?;
        // Which answer was wrong isn't said, so they can't be guessed one by one.
        return Err(wrong("answers", "One or more answers aren't right."));
    }
    finish(&app, &state, password).await
}

/// New codes replace every old one. Needs the current password.
#[tauri::command]
pub async fn regenerate_recovery_codes(
    current: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    check_password(&state, current, "current").await?;
    state
        .blocking(|_, credentials| {
            let codes = app_recovery::generate_codes();
            app_recovery::store_codes(credentials, &codes)?;
            Ok(codes)
        })
        .await
}

#[tauri::command]
pub async fn set_security_questions(
    current: String,
    questions: Vec<QuestionInput>,
    state: State<'_, AppState>,
) -> Result<RecoveryOptions, AppError> {
    app_recovery::validate_questions(&questions)?;
    check_password(&state, current, "current").await?;
    state
        .blocking(move |_, credentials| {
            app_recovery::store_questions(credentials, &questions)?;
            app_recovery::options(credentials)
        })
        .await
}

#[tauri::command]
pub async fn remove_security_questions(
    current: String,
    state: State<'_, AppState>,
) -> Result<RecoveryOptions, AppError> {
    check_password(&state, current, "current").await?;
    state
        .blocking(|_, credentials| {
            app_recovery::delete_questions(credentials)?;
            app_recovery::options(credentials)
        })
        .await
}

/// Writes codes just shown to a text file the person chose.
#[tauri::command]
pub async fn save_recovery_codes(path: String, codes: Vec<String>) -> Result<(), AppError> {
    let invalid = || AppError::new("validation", "Choose where to save the codes again.");
    let mut path = PathBuf::from(path);
    if !path.is_absolute() || !path.parent().is_some_and(Path::is_dir) {
        return Err(invalid());
    }
    path.set_extension("txt");
    // Only codes in their own format are written, never arbitrary text.
    let well_formed = !codes.is_empty()
        && codes.len() <= app_recovery::CODE_COUNT
        && codes.iter().all(|code| {
            code.len() == 11
                && code.chars().enumerate().all(|(i, c)| {
                    if i == 5 {
                        c == '-'
                    } else {
                        c.is_ascii_hexdigit()
                    }
                })
        });
    if !well_formed {
        return Err(invalid());
    }
    let text = app_recovery::codes_file(&codes);
    tauri::async_runtime::spawn_blocking(move || std::fs::write(&path, text))
        .await
        .map_err(|_| AppError::background_task())?
        .map_err(|_| AppError::new("io", "The codes couldn't be saved there."))
}
