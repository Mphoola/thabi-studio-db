//! Tauri commands exposed to the frontend. Keep this surface small: every
//! command added here must also be reviewed against the capabilities in
//! `capabilities/`. Commands are async; SQLite and credential-store work runs on a
//! blocking thread through `AppState::blocking`.

pub mod app;
pub mod app_lock;
pub mod app_recovery;
pub mod connection_profiles;
pub mod connections;
pub mod library;
pub mod settings;
