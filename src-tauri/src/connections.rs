//! Live connections, owned by Rust and keyed by profile id.
//!
//! Each open connection is exactly one database session (see
//! `database::Session`). States are explicit: an entry is `Connecting`,
//! `Connected` or `Closing`; no entry means disconnected. Errors are returned
//! to the caller rather than stored.

use crate::{
    database::{Capabilities, ConnectParams, ServerInfo, Session},
    errors::AppError,
    queries::{QueryTab, QueryTabHandle},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

/// Close a connection nobody has used for this long.
pub const IDLE_DISCONNECT_AFTER: Duration = Duration::from_secs(60 * 60);
/// Check that quiet connections are still alive this often.
pub const HEALTH_CHECK_AFTER: Duration = Duration::from_secs(5 * 60);
/// How long disconnect waits for a running statement before giving up.
const CLOSE_WAIT: Duration = Duration::from_secs(15);

pub type SessionHandle = Arc<Mutex<Option<Session>>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Connecting,
    Connected,
    Closing,
}

struct Entry {
    generation: u64,
    state: ConnectionState,
    session: Option<SessionHandle>,
    server: Option<ServerInfo>,
    capabilities: Option<Capabilities>,
    last_used: Instant,
    /// Kept in memory only, to open query-tab sessions and cancel queries.
    params: Option<ConnectParams>,
    /// One session per open query tab, keyed by tab id.
    tabs: HashMap<String, QueryTabHandle>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub profile_id: String,
    /// Changes every time the profile connects, so stale replies can be told apart.
    pub connection_id: String,
    pub state: ConnectionState,
    pub server: Option<ServerInfo>,
    pub capabilities: Option<Capabilities>,
}

#[derive(Default)]
struct Inner {
    entries: Mutex<HashMap<String, Entry>>,
    next_generation: AtomicU64,
}

#[derive(Clone, Default)]
pub struct ConnectionService {
    inner: Arc<Inner>,
}

fn status(profile_id: &str, entry: &Entry) -> ConnectionStatus {
    ConnectionStatus {
        profile_id: profile_id.to_owned(),
        connection_id: format!("{profile_id}:{}", entry.generation),
        state: entry.state,
        server: entry.server.clone(),
        capabilities: entry.capabilities,
    }
}

/// Removes a `Connecting` entry if the connect call is dropped midway
/// (for example when the window closes), so the profile never stays stuck.
struct Reservation {
    inner: Arc<Inner>,
    profile_id: String,
    generation: u64,
    armed: bool,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let inner = self.inner.clone();
        let profile_id = std::mem::take(&mut self.profile_id);
        let generation = self.generation;
        tauri::async_runtime::spawn(async move {
            let mut entries = inner.entries.lock().await;
            if entries.get(&profile_id).is_some_and(|e| {
                e.generation == generation && e.state == ConnectionState::Connecting
            }) {
                entries.remove(&profile_id);
            }
        });
    }
}

impl ConnectionService {
    pub async fn connect(
        &self,
        profile_id: &str,
        params: &ConnectParams,
    ) -> Result<ConnectionStatus, AppError> {
        let generation = {
            let mut entries = self.inner.entries.lock().await;
            match entries.get(profile_id) {
                Some(entry) if entry.state == ConnectionState::Connected => {
                    return Ok(status(profile_id, entry));
                }
                Some(_) => return Err(AppError::connection_in_progress()),
                None => {}
            }
            let generation = self.inner.next_generation.fetch_add(1, Ordering::Relaxed) + 1;
            entries.insert(
                profile_id.to_owned(),
                Entry {
                    generation,
                    state: ConnectionState::Connecting,
                    session: None,
                    server: None,
                    capabilities: None,
                    last_used: Instant::now(),
                    params: None,
                    tabs: HashMap::new(),
                },
            );
            generation
        };
        let mut reservation = Reservation {
            inner: self.inner.clone(),
            profile_id: profile_id.to_owned(),
            generation,
            armed: true,
        };

        let opened = async {
            let mut session = Session::connect(params).await?;
            session.ping().await?;
            Ok::<_, AppError>(session)
        }
        .await;

        let mut entries = self.inner.entries.lock().await;
        reservation.armed = false;
        let still_wanted = entries
            .get(profile_id)
            .is_some_and(|e| e.generation == generation && e.state == ConnectionState::Connecting);
        match opened {
            Ok(session) if still_wanted => {
                let entry = entries.get_mut(profile_id).expect("checked above");
                entry.server = Some(session.server_info().clone());
                entry.capabilities = Some(session.capabilities());
                entry.session = Some(Arc::new(Mutex::new(Some(session))));
                entry.state = ConnectionState::Connected;
                entry.last_used = Instant::now();
                entry.params = Some(params.clone());
                Ok(status(profile_id, entry))
            }
            Ok(session) => {
                // Disconnect was requested while connecting.
                drop(entries);
                let _ = session.close().await;
                Err(AppError::connection_cancelled())
            }
            Err(error) => {
                if still_wanted {
                    entries.remove(profile_id);
                }
                Err(error)
            }
        }
    }

    /// Closes the connection if there is one. Always removes the entry, so
    /// nothing is left behind even if the server doesn't answer.
    pub async fn disconnect(&self, profile_id: &str) -> Result<(), AppError> {
        let (generation, session, tabs) = {
            let mut entries = self.inner.entries.lock().await;
            let Some(entry) = entries.get_mut(profile_id) else {
                return Ok(());
            };
            match entry.state {
                ConnectionState::Connecting => {
                    // The connect call sees the entry gone and closes what it opened.
                    entries.remove(profile_id);
                    return Ok(());
                }
                ConnectionState::Closing => return Ok(()),
                ConnectionState::Connected => {
                    entry.state = ConnectionState::Closing;
                    let tabs: Vec<_> = entry.tabs.drain().map(|(_, tab)| tab).collect();
                    (entry.generation, entry.session.clone(), tabs)
                }
            }
        };
        for tab in tabs {
            tab.close().await;
        }

        let result = match session {
            Some(handle) => match tokio::time::timeout(CLOSE_WAIT, handle.lock()).await {
                Ok(mut guard) => match guard.take() {
                    Some(session) => session.close().await,
                    None => Ok(()),
                },
                // A statement is still running. Dropping our handle lets the
                // session close when that statement finishes.
                Err(_) => Err(AppError::connection_close_failed()),
            },
            None => Ok(()),
        };

        let mut entries = self.inner.entries.lock().await;
        if entries
            .get(profile_id)
            .is_some_and(|e| e.generation == generation)
        {
            entries.remove(profile_id);
        }
        result
    }

    /// The session for a connected profile. Lock it to run a statement; the
    /// lock keeps statements on one session in order.
    pub async fn session(&self, profile_id: &str) -> Result<SessionHandle, AppError> {
        let mut entries = self.inner.entries.lock().await;
        match entries.get_mut(profile_id) {
            Some(entry) if entry.state == ConnectionState::Connected => {
                entry.last_used = Instant::now();
                entry.session.clone().ok_or_else(AppError::not_connected)
            }
            Some(entry) if entry.state == ConnectionState::Connecting => {
                Err(AppError::connection_in_progress())
            }
            _ => Err(AppError::not_connected()),
        }
    }

    /// The query tab for `tab_id`, created on first use, with the
    /// connection's parameters for opening its session.
    pub async fn query_tab(
        &self,
        profile_id: &str,
        tab_id: &str,
    ) -> Result<(QueryTabHandle, ConnectParams), AppError> {
        let mut entries = self.inner.entries.lock().await;
        match entries.get_mut(profile_id) {
            Some(entry) if entry.state == ConnectionState::Connected => {
                entry.last_used = Instant::now();
                let params = entry.params.clone().ok_or_else(AppError::not_connected)?;
                let tab = entry
                    .tabs
                    .entry(tab_id.to_owned())
                    .or_insert_with(|| Arc::new(QueryTab::default()))
                    .clone();
                Ok((tab, params))
            }
            Some(entry) if entry.state == ConnectionState::Connecting => {
                Err(AppError::connection_in_progress())
            }
            _ => Err(AppError::not_connected()),
        }
    }

    /// Existing query tab, without creating one.
    pub async fn existing_query_tab(
        &self,
        profile_id: &str,
        tab_id: &str,
    ) -> Option<(QueryTabHandle, ConnectParams)> {
        let entries = self.inner.entries.lock().await;
        let entry = entries.get(profile_id)?;
        Some((entry.tabs.get(tab_id)?.clone(), entry.params.clone()?))
    }

    pub async fn close_query_tab(&self, profile_id: &str, tab_id: &str) {
        let tab = {
            let mut entries = self.inner.entries.lock().await;
            entries
                .get_mut(profile_id)
                .and_then(|entry| entry.tabs.remove(tab_id))
        };
        if let Some(tab) = tab {
            tab.close().await;
        }
    }

    pub async fn statuses(&self) -> Vec<ConnectionStatus> {
        let entries = self.inner.entries.lock().await;
        let mut list: Vec<_> = entries.iter().map(|(id, e)| status(id, e)).collect();
        list.sort_by(|a, b| a.profile_id.cmp(&b.profile_id));
        list
    }

    pub async fn disconnect_all(&self) {
        let ids: Vec<String> = self.inner.entries.lock().await.keys().cloned().collect();
        for id in ids {
            let _ = self.disconnect(&id).await;
        }
    }

    /// One pass of upkeep: closes idle connections and checks quiet ones.
    /// Returns the profiles that were closed, with the reason.
    pub async fn maintain(&self) -> Vec<(String, &'static str)> {
        let candidates: Vec<(String, Duration, Option<SessionHandle>)> = {
            let entries = self.inner.entries.lock().await;
            entries
                .iter()
                .filter(|(_, e)| e.state == ConnectionState::Connected)
                .filter(|(_, e)| !e.tabs.values().any(|tab| tab.is_running()))
                .map(|(id, e)| (id.clone(), e.last_used.elapsed(), e.session.clone()))
                .collect()
        };
        let mut closed = Vec::new();
        for (profile_id, idle, handle) in candidates {
            if idle >= IDLE_DISCONNECT_AFTER {
                let _ = self.disconnect(&profile_id).await;
                closed.push((profile_id, "idle"));
                continue;
            }
            if idle < HEALTH_CHECK_AFTER {
                continue;
            }
            let Some(handle) = handle else { continue };
            // Skip sessions that are busy running a statement.
            let Ok(mut guard) = handle.try_lock() else {
                continue;
            };
            let alive = match guard.as_mut() {
                Some(session) => session.ping().await.is_ok(),
                None => false,
            };
            drop(guard);
            if !alive {
                let _ = self.disconnect(&profile_id).await;
                closed.push((profile_id, "lost"));
            }
        }
        closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{Engine, TlsMode};

    fn unreachable_params() -> ConnectParams {
        ConnectParams {
            engine: Engine::Postgres,
            // Reserved for documentation (RFC 6761); never resolves.
            host: "thabi-studio-test.invalid".into(),
            port: 5432,
            database: None,
            username: "user".into(),
            password: None,
            tls: TlsMode::Verify,
            ca_path: None,
            ssh: None,
            connect_to: None,
        }
    }

    #[tokio::test]
    async fn failed_connect_leaves_no_entry_behind() {
        let service = ConnectionService::default();
        let error = service
            .connect("profile", &unreachable_params())
            .await
            .unwrap_err();
        assert_eq!(error.kind, "host_not_found");
        assert!(service.statuses().await.is_empty());
    }

    #[tokio::test]
    async fn disconnecting_an_unknown_profile_is_a_no_op() {
        let service = ConnectionService::default();
        service.disconnect("nothing").await.unwrap();
        assert!(matches!(
            service.session("nothing").await,
            Err(AppError {
                kind: "not_connected",
                ..
            })
        ));
    }

    #[tokio::test]
    async fn disconnect_during_connect_cancels_it() {
        let service = ConnectionService::default();
        let connecting = {
            let service = service.clone();
            tokio::spawn(async move { service.connect("p", &unreachable_params()).await })
        };
        // Give connect a moment to register, then cancel.
        tokio::time::sleep(Duration::from_millis(1)).await;
        service.disconnect("p").await.unwrap();
        let _ = connecting.await.unwrap();
        assert!(service.statuses().await.is_empty());
    }
}
