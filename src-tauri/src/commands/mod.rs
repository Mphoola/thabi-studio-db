//! Tauri commands exposed to the frontend. Keep this surface small: every
//! command added here must also be reviewed against the capabilities in
//! `capabilities/`. Commands are async; SQLite and credential-store work runs on a
//! blocking thread through `AppState::blocking`.

pub mod admin;
pub mod app;
pub mod app_lock;
pub mod app_recovery;
pub mod backup;
pub mod connection_profiles;
pub mod connections;
pub mod files;
pub mod import;
pub mod library;
pub mod settings;
pub mod users;
