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
    database::{CancelTarget, ConnectParams, QueryResult, Session, cancel_backend},
    errors::AppError,
};
use serde_json::json;
use std::{
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
                Ok(result) => Ok(result),
            },
            Ok((result, Stop::None)) => match result {
                Err(_) if self.cancel_requested.load(Ordering::SeqCst) => Err(cancelled(false)),
                other => other,
            },
        }
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
