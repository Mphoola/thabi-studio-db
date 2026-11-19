//! Users and permissions (Phase 15).

use crate::commands::connections::{refuse_read_only, with_session};
use crate::database::users::{self, UserAction, UserInfo};
use crate::{errors::AppError, state::AppState};
use tauri::State;

#[tauri::command]
pub async fn list_users(
    id: String,
    database: String,
    state: State<'_, AppState>,
) -> Result<Vec<UserInfo>, AppError> {
    with_session(&state, &id, async |s| users::list_users(s, &database).await).await
}

#[tauri::command]
pub async fn user_grants(
    id: String,
    database: String,
    name: String,
    host: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    with_session(&state, &id, async |s| {
        users::grants(s, &database, &name, &host).await
    })
    .await
}

/// The statements a user change will run, with any password masked.
#[tauri::command]
pub async fn preview_user_action(
    id: String,
    action: UserAction,
    state: State<'_, AppState>,
) -> Result<Vec<String>, AppError> {
    let params = state.connections.params(&id).await?;
    refuse_read_only(&params)?;
    Ok(users::user_script(params.engine, &action)?.1)
}

/// Runs a user change on a tab's session in `database`, statement by
/// statement: none of the three engines puts these in a transaction.
#[tauri::command]
pub async fn apply_user_action(
    id: String,
    tab_id: String,
    database: String,
    action: UserAction,
    state: State<'_, AppState>,
) -> Result<usize, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    refuse_read_only(&params)?;
    let (statements, _) = users::user_script(params.engine, &action)?;
    tab.with_session(&params, Some(database), async |session| {
        for statement in &statements {
            session.execute(statement, 0).await?;
        }
        Ok(statements.len())
    })
    .await
    .map_err(|error| users::scrub(error, &action))
}
