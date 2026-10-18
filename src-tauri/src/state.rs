use crate::app_lock::AppLock;
use crate::connections::ConnectionService;
use crate::credentials::{CredentialStore, OsCredentialStore};
use crate::errors::AppError;
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub database: Arc<Mutex<Connection>>,
    pub credentials: Arc<dyn CredentialStore>,
    pub connections: ConnectionService,
    pub app_lock: AppLock,
    /// One flag per connection running a dump, so an export of a real database
    /// — which is minutes, not seconds — can be stopped.
    pub exports: Arc<Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>>,
}

impl AppState {
    pub fn new(database: Connection) -> Self {
        Self {
            database: Arc::new(Mutex::new(database)),
            credentials: Arc::new(OsCredentialStore),
            connections: ConnectionService::default(),
            app_lock: AppLock::default(),
            exports: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Runs SQLite and credential-store work on a blocking thread. Those calls can
    /// be slow or show a macOS prompt, so they must never run on the main
    /// thread or an async worker.
    pub async fn blocking<T, F>(&self, work: F) -> Result<T, AppError>
    where
        T: Send + 'static,
        F: FnOnce(&mut Connection, &dyn CredentialStore) -> Result<T, AppError> + Send + 'static,
    {
        let database = self.database.clone();
        let credentials = self.credentials.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut connection = database.lock().map_err(|_| AppError::storage())?;
            work(&mut connection, &*credentials)
        })
        .await
        .map_err(|_| AppError::background_task())?
    }
}

impl AppState {
    /// A cancel flag for a dump on this connection, replacing any run before.
    pub fn start_export(&self, connection: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut exports) = self.exports.lock() {
            exports.insert(connection.to_owned(), flag.clone());
        }
        flag
    }

    /// Asks a running dump to stop. False when nothing is running, which is
    /// what the button says when it is pressed with nothing to stop.
    pub fn cancel_export(&self, connection: &str) -> bool {
        self.exports
            .lock()
            .ok()
            .and_then(|exports| exports.get(connection).cloned())
            .map(|flag| {
                flag.store(true, Ordering::SeqCst);
                true
            })
            .unwrap_or(false)
    }

    pub fn finish_export(&self, connection: &str) {
        if let Ok(mut exports) = self.exports.lock() {
            exports.remove(connection);
        }
    }
}
