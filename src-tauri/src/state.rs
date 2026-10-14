use crate::connections::ConnectionService;
use crate::credentials::{CredentialStore, KeychainCredentialStore};
use crate::errors::AppError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub database: Arc<Mutex<Connection>>,
    pub credentials: Arc<dyn CredentialStore>,
    pub connections: ConnectionService,
}

impl AppState {
    pub fn new(database: Connection) -> Self {
        Self {
            database: Arc::new(Mutex::new(database)),
            credentials: Arc::new(KeychainCredentialStore),
            connections: ConnectionService::default(),
        }
    }

    /// Runs SQLite and Keychain work on a blocking thread. Keychain calls can
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
