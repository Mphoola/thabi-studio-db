use crate::credentials::{CredentialStore, KeychainCredentialStore};
use rusqlite::Connection;
use std::sync::Mutex;

pub struct AppState {
    pub database: Mutex<Connection>,
    pub credentials: Box<dyn CredentialStore>,
}

impl AppState {
    pub fn new(database: Connection) -> Self {
        Self {
            database: Mutex::new(database),
            credentials: Box::new(KeychainCredentialStore),
        }
    }
}
