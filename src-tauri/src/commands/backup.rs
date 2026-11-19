//! Backup and restore with the engines' own tools (Phase 15).

use crate::backup::{self, Job, JobResult, ToolStatus};
use crate::commands::connections::refuse_read_only;
use crate::{errors::AppError, state::AppState};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn backup_tools(id: String, state: State<'_, AppState>) -> Result<ToolStatus, AppError> {
    let engine = state.connections.params(&id).await?.engine;
    tauri::async_runtime::spawn_blocking(move || backup::status(engine))
        .await
        .map_err(|_| AppError::background_task())
}

async fn with_job<F>(
    state: &AppState,
    job_id: String,
    work: impl FnOnce(Arc<Job>) -> F,
) -> Result<JobResult, AppError>
where
    F: std::future::Future<Output = Result<JobResult, AppError>>,
{
    let job = Arc::new(Job::default());
    state
        .jobs
        .lock()
        .map_err(|_| AppError::background_task())?
        .insert(job_id.clone(), job.clone());
    let result = work(job).await;
    if let Ok(mut jobs) = state.jobs.lock() {
        jobs.remove(&job_id);
    }
    result
}

/// Backs a database up to `path` (on this computer, or on the server for
/// SQL Server). Reading only, so allowed on a read-only connection.
#[tauri::command]
pub async fn backup_database(
    id: String,
    job_id: String,
    database: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<JobResult, AppError> {
    let params = state.connections.params(&id).await?;
    with_job(&state, job_id, async |job| {
        backup::backup(&params, &database, PathBuf::from(&path), &job).await
    })
    .await
}

/// Restores a backup into a database, replacing what is there.
#[tauri::command]
pub async fn restore_database(
    id: String,
    job_id: String,
    database: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<JobResult, AppError> {
    let params = state.connections.params(&id).await?;
    refuse_read_only(&params)?;
    with_job(&state, job_id, async |job| {
        backup::restore(&params, &database, PathBuf::from(&path), &job).await
    })
    .await
}

/// The log so far of a running backup or restore.
#[tauri::command]
pub fn backup_log(job_id: String, state: State<'_, AppState>) -> Vec<String> {
    state
        .jobs
        .lock()
        .ok()
        .and_then(|jobs| jobs.get(&job_id).map(|job| job.log()))
        .unwrap_or_default()
}

#[tauri::command]
pub fn cancel_backup(job_id: String, state: State<'_, AppState>) {
    if let Ok(jobs) = state.jobs.lock()
        && let Some(job) = jobs.get(&job_id)
    {
        job.cancel.cancel();
    }
}
