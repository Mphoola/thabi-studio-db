//! Importing a file into a table (Phase 15): preview it, run it on a tab's
//! own session, follow its progress, and stop it.

use crate::commands::connections::refuse_read_only;
use crate::database::import::{
    self, ColumnMapping, ImportOptions, ImportProgress, ImportSummary, ImportTarget, Preview,
    ReadOptions,
};
use crate::{errors::AppError, state::AppState};
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tauri::State;

/// The first rows of the file and its columns, before anything is sent.
#[tauri::command]
pub async fn preview_import(
    id: String,
    path: String,
    options: ReadOptions,
    state: State<'_, AppState>,
) -> Result<Preview, AppError> {
    let engine = state.connections.params(&id).await?.engine;
    tauri::async_runtime::spawn_blocking(move || import::preview(&path, &options, engine))
        .await
        .map_err(|_| AppError::background_task())?
}

/// Imports the file into the table on the tab's own session.
#[tauri::command]
pub async fn import_rows(
    id: String,
    tab_id: String,
    target: ImportTarget,
    mappings: Vec<ColumnMapping>,
    options: ImportOptions,
    path: String,
    state: State<'_, AppState>,
) -> Result<ImportSummary, AppError> {
    let (tab, params) = state.connections.query_tab(&id, &tab_id).await?;
    refuse_read_only(&params)?;
    let progress = Arc::new(ImportProgress::default());
    state
        .imports
        .lock()
        .map_err(|_| AppError::background_task())?
        .insert(tab_id.clone(), progress.clone());
    let result = tab
        .with_session(&params, Some(target.database.clone()), async |session| {
            import::run(session, &target, &mappings, &options, &path, &progress).await
        })
        .await;
    if let Ok(mut running) = state.imports.lock() {
        running.remove(&tab_id);
    }
    result
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportStatus {
    pub rows: u64,
    pub rejected: u64,
    pub bytes: u64,
    pub total_bytes: u64,
}

/// How far a running import has got, or None when none is running.
#[tauri::command]
pub fn import_progress(tab_id: String, state: State<'_, AppState>) -> Option<ImportStatus> {
    let running = state.imports.lock().ok()?;
    let progress = running.get(&tab_id)?;
    Some(ImportStatus {
        rows: progress.rows.load(Ordering::Relaxed),
        rejected: progress.rejected.load(Ordering::Relaxed),
        bytes: progress.bytes.load(Ordering::Relaxed),
        total_bytes: progress.total_bytes.load(Ordering::Relaxed),
    })
}

/// Stops after the batch in flight; everything is rolled back.
#[tauri::command]
pub fn cancel_import(tab_id: String, state: State<'_, AppState>) {
    if let Ok(running) = state.imports.lock()
        && let Some(progress) = running.get(&tab_id)
    {
        progress.cancel.store(true, Ordering::SeqCst);
    }
}
