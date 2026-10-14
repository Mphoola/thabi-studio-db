//! Tauri commands exposed to the frontend. Keep this surface small: every
//! command added here must also be reviewed against the capabilities in
//! `capabilities/`. Commands are async; SQLite and Keychain work runs on a
//! blocking thread through `AppState::blocking`.

pub mod app;
pub mod connection_profiles;
pub mod connections;
pub mod settings;
