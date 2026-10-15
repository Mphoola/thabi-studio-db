//! Query tabs: each editor tab runs on its own database session, so
//! `BEGIN`, `SET`, `USE` and temp tables stay with the tab that ran them.
//!
//! Stopping a query:
//! - PostgreSQL and MySQL are cancelled on the server from a second, short
//!   session (`pg_cancel_backend` / `KILL QUERY`), keeping the tab's session.
//! - SQL Server (no cancel in Tiberius), or a server that doesn't respond to
//!   the cancel within a few seconds: the tab's session is dropped, which
//!   closes its connection and makes the server stop. The next run opens a
//!   fresh session and the UI says the session was reset.
//!
//! Queries are never retried automatically.

use crate::{
    database::{CancelTarget, Cell, ConnectParams, Engine, QueryResult, Session, cancel_backend},
    errors::AppError,
    results::{StoredResult, into_first_page},
};
use serde_json::json;
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// How long a server-side cancel may take before the session is dropped.
const CANCEL_GRACE: Duration = Duration::from_secs(5);
/// Results kept per tab for paging, copying and export. Older ones are
/// dropped so memory stays bounded.
const KEPT_RESULTS: usize = 8;

#[derive(Default)]
pub struct QueryTab {
    session: Mutex<Option<Session>>,
    /// Database the tab's session opened in.
    database: StdMutex<Option<String>>,
    cancel_target: StdMutex<Option<CancelTarget>>,
    /// Cancelled to abandon the running statement and drop the session.
    abort: StdMutex<CancellationToken>,
    cancel_requested: AtomicBool,
    running: AtomicBool,
    results: StdMutex<VecDeque<(String, Arc<StoredResult>)>>,
    next_result: std::sync::atomic::AtomicU64,
}

fn query_running() -> AppError {
    AppError::new(
        "query_running",
        "A query is already running in this tab. Wait for it or cancel it.",
    )
}

fn cancelled(session_reset: bool) -> AppError {
    let message = if session_reset {
        "Query cancelled. This tab's session was reset, so temporary tables and open transactions are gone."
    } else {
        "Query cancelled."
    };
    AppError::new("query_cancelled", message).with_data(json!({ "sessionReset": session_reset }))
}

fn timed_out(seconds: u64, session_reset: bool) -> AppError {
    let mut message = format!("The query ran longer than {seconds} s and was stopped.");
    if session_reset {
        message.push_str(" This tab's session was reset.");
    }
    AppError::new("query_timed_out", message).with_data(json!({ "sessionReset": session_reset }))
}

enum Stop {
    None,
    Timeout,
}

impl QueryTab {
    /// Keeps the full rows of a result and returns its first page.
    fn keep(&self, result: QueryResult) -> QueryResult {
        if result.columns.is_empty() {
            return result;
        }
        let id = format!("r{}", self.next_result.fetch_add(1, Ordering::Relaxed) + 1);
        let (first, stored) = into_first_page(result, id.clone());
        let mut results = self.results.lock().expect("tab lock");
        results.push_back((id, Arc::new(stored)));
        while results.len() > KEPT_RESULTS {
            results.pop_front();
        }
        first
    }

    /// A result kept by this tab, for paging, copying or export.
    pub fn result(&self, id: &str) -> Result<Arc<StoredResult>, AppError> {
        self.results
            .lock()
            .expect("tab lock")
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, result)| result.clone())
            .ok_or_else(|| {
                AppError::new(
                    "result_expired",
                    "This result is no longer kept. Run the query again.",
                )
            })
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Runs one statement. `params` are the connection's; `database` is the
    /// tab's chosen database (a new session opens if it changed).
    pub async fn run(
        &self,
        params: &ConnectParams,
        database: Option<String>,
        sql: &str,
        max_rows: usize,
        timeout: Option<Duration>,
    ) -> Result<QueryResult, AppError> {
        let mut guard = self.session.try_lock().map_err(|_| query_running())?;
        self.open_session(&mut guard, params, database).await?;

        let token = CancellationToken::new();
        *self.abort.lock().expect("tab lock") = token.clone();
        self.cancel_requested.store(false, Ordering::SeqCst);
        self.running.store(true, Ordering::SeqCst);

        let session = guard.as_mut().expect("opened above");
        let outcome = {
            let exec = session.execute(sql, max_rows);
            tokio::pin!(exec);
            let sleep = async {
                match timeout {
                    Some(limit) => tokio::time::sleep(limit).await,
                    None => std::future::pending().await,
                }
            };
            tokio::select! {
                result = &mut exec => Ok((result, Stop::None)),
                _ = token.cancelled() => Err(()),
                _ = sleep => {
                    // Ask the server to stop, then give it a moment.
                    self.request_server_cancel(params);
                    tokio::select! {
                        result = &mut exec => Ok((result, Stop::Timeout)),
                        _ = tokio::time::sleep(CANCEL_GRACE) => Err(()),
                        _ = token.cancelled() => Err(()),
                    }
                }
            }
        };
        self.running.store(false, Ordering::SeqCst);

        let seconds = timeout.map(|t| t.as_secs()).unwrap_or_default();
        match outcome {
            // The statement was abandoned: drop the session so the server
            // stops it, and start fresh next time.
            Err(()) => {
                *guard = None;
                *self.cancel_target.lock().expect("tab lock") = None;
                if self.cancel_requested.load(Ordering::SeqCst) {
                    Err(cancelled(true))
                } else {
                    Err(timed_out(seconds, true))
                }
            }
            Ok((result, Stop::Timeout)) => match result {
                Err(_) => Err(timed_out(seconds, false)),
                // It finished just as the limit was reached.
                Ok(result) => Ok(self.keep(result)),
            },
            Ok((result, Stop::None)) => match result {
                Err(_) if self.cancel_requested.load(Ordering::SeqCst) => Err(cancelled(false)),
                Ok(result) => {
                    // `USE db` moved this session to another database; remember
                    // it so the next run doesn't open a new session.
                    if let Some(database) = use_target(sql) {
                        *self.database.lock().expect("tab lock") = Some(database);
                    }
                    Ok(self.keep(result))
                }
                other => other,
            },
        }
    }

    /// Opens the tab's session, or a new one if `database` changed.
    async fn open_session(
        &self,
        guard: &mut Option<Session>,
        params: &ConnectParams,
        database: Option<String>,
    ) -> Result<(), AppError> {
        let wanted = database
            .filter(|d| !d.is_empty())
            .or_else(|| params.database.clone());
        let same_database = *self.database.lock().expect("tab lock") == wanted;
        if guard.is_none() || !same_database {
            if let Some(old) = guard.take() {
                let _ = old.close().await;
            }
            let mut tab_params = params.clone();
            tab_params.database = wanted.clone();
            let session = Session::connect(&tab_params).await?;
            *self.cancel_target.lock().expect("tab lock") = session.cancel_target();
            *self.database.lock().expect("tab lock") = wanted;
            *guard = Some(session);
        }
        Ok(())
    }

    /// Runs staged row edits in one transaction. Every statement must change
    /// exactly one row; otherwise everything is rolled back and nothing is
    /// saved. If the rollback itself fails, the session is dropped, which
    /// makes the server discard the transaction. Never retried.
    pub async fn apply(
        &self,
        params: &ConnectParams,
        database: Option<String>,
        statements: &[String],
    ) -> Result<usize, AppError> {
        let mut guard = self.session.try_lock().map_err(|_| query_running())?;
        self.open_session(&mut guard, params, database).await?;
        let engine = params.engine;
        let (begin, commit, rollback) = match engine {
            Engine::Postgres => ("BEGIN", "COMMIT", "ROLLBACK"),
            Engine::MySql => ("START TRANSACTION", "COMMIT", "ROLLBACK"),
            Engine::SqlServer => (
                "BEGIN TRANSACTION",
                "COMMIT TRANSACTION",
                "ROLLBACK TRANSACTION",
            ),
        };
        self.running.store(true, Ordering::SeqCst);
        let session = guard.as_mut().expect("opened above");
        let outcome: Result<usize, (bool, AppError)> = async {
            session.execute(begin, 1).await.map_err(|e| (false, e))?;
            for (index, sql) in statements.iter().enumerate() {
                let count = match engine {
                    // Simple queries on SQL Server don't report rows affected.
                    Engine::SqlServer => session
                        .execute(&format!("{sql};\nSELECT @@ROWCOUNT"), 1)
                        .await
                        .map(|r| match r.rows.first().and_then(|row| row.first()) {
                            Some(Cell::Value(n)) => n.parse::<u64>().ok(),
                            _ => None,
                        }),
                    _ => session.execute(sql, 1).await.map(|r| r.rows_affected),
                };
                match count {
                    Ok(Some(1)) => {}
                    Ok(count) => {
                        return Err((true, not_one_row(index, statements.len(), count)));
                    }
                    Err(error) => {
                        return Err((true, change_failed(index, statements.len(), error)));
                    }
                }
            }
            session.execute(commit, 1).await.map_err(|e| (false, e))?;
            Ok(statements.len())
        }
        .await;
        let result = match outcome {
            Ok(count) => Ok(count),
            Err((roll_back, error)) => {
                if roll_back && session.execute(rollback, 1).await.is_ok() {
                    Err(error)
                } else {
                    *guard = None;
                    Err(error)
                }
            }
        };
        self.running.store(false, Ordering::SeqCst);
        result
    }

    fn request_server_cancel(&self, params: &ConnectParams) {
        let target = *self.cancel_target.lock().expect("tab lock");
        let database = self.database.lock().expect("tab lock").clone();
        if let Some(target) = target {
            let mut params = params.clone();
            params.database = database;
            tauri::async_runtime::spawn(async move {
                let _ = cancel_backend(&params, target).await;
            });
        }
    }

    /// Stops the running statement, if any. Returns false when nothing runs.
    pub fn cancel(&self, params: &ConnectParams) -> bool {
        if !self.is_running() {
            return false;
        }
        self.cancel_requested.store(true, Ordering::SeqCst);
        let token = self.abort.lock().expect("tab lock").clone();
        if self.cancel_target.lock().expect("tab lock").is_some() {
            self.request_server_cancel(params);
            // If the server doesn't stop it soon, drop the session.
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(CANCEL_GRACE).await;
                token.cancel();
            });
        } else {
            token.cancel();
        }
        true
    }

    /// Stops anything running and closes the session.
    pub async fn close(&self) {
        self.abort.lock().expect("tab lock").cancel();
        let session = self.session.lock().await.take();
        if let Some(session) = session {
            let _ = session.close().await;
        }
    }
}

pub type QueryTabHandle = Arc<QueryTab>;

fn not_one_row(index: usize, total: usize, count: Option<u64>) -> AppError {
    let found = match count {
        Some(0) => "found no matching row".to_owned(),
        Some(n) => format!("would change {n} rows"),
        None => "couldn't confirm how many rows it changed".to_owned(),
    };
    AppError::new(
        "edit_conflict",
        format!(
            "Change {} of {total} {found}, so nothing was saved. The row may have changed or been deleted; reload and try again.",
            index + 1
        ),
    )
}

fn change_failed(index: usize, total: usize, mut error: AppError) -> AppError {
    error.message = format!(
        "Change {} of {total} failed, so nothing was saved. {}",
        index + 1,
        error.message
    );
    error
}

/// The database named by a `USE name` statement, without quotes.
fn use_target(sql: &str) -> Option<String> {
    let rest = sql.trim().strip_prefix(|c: char| c == 'u' || c == 'U')?;
    let rest = rest.strip_prefix(['s', 'S'])?.strip_prefix(['e', 'E'])?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let name = rest.trim().trim_end_matches(';').trim();
    if name.is_empty() || (name.contains(char::is_whitespace) && !name.starts_with(['`', '[', '"']))
    {
        return None;
    }
    let unquoted = name
        .strip_prefix('`')
        .and_then(|n| n.strip_suffix('`'))
        .or_else(|| name.strip_prefix('[').and_then(|n| n.strip_suffix(']')))
        .or_else(|| name.strip_prefix('"').and_then(|n| n.strip_suffix('"')))
        .unwrap_or(name);
    Some(unquoted.to_owned())
}

#[cfg(test)]
mod tests {
    use super::use_target;

    #[test]
    fn reads_use_statements() {
        assert_eq!(use_target("USE shop"), Some("shop".into()));
        assert_eq!(use_target("  use `my db`;"), Some("my db".into()));
        assert_eq!(use_target("USE [Sales Ü]"), Some("Sales Ü".into()));
        assert_eq!(use_target("SELECT 1"), None);
        assert_eq!(use_target("user_defined()"), None);
    }
}
