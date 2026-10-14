mod commands;
mod connections;
mod credentials;
pub mod database;
mod errors;
mod profiles;
mod settings;
mod state;
mod storage;

pub use errors::AppError;

use tauri::{Emitter, Manager};

/// How often idle and dropped connections are checked.
const MAINTENANCE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionClosed {
    profile_id: String,
    reason: &'static str,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            credentials::init_platform_store()?;
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let mut database =
                rusqlite::Connection::open(app_data_dir.join("thabi-studio.sqlite3"))?;
            storage::migrate(&mut database)?;
            let state = state::AppState::new(database);
            let connections = state.connections.clone();
            app.manage(state);

            // Close connections idle for an hour and notice dropped ones.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(MAINTENANCE_INTERVAL);
                loop {
                    interval.tick().await;
                    for (profile_id, reason) in connections.maintain().await {
                        let _ = handle
                            .emit("connection-closed", ConnectionClosed { profile_id, reason });
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::connection_profiles::list_connection_profiles,
            commands::connection_profiles::create_connection_profile,
            commands::connection_profiles::update_connection_profile,
            commands::connection_profiles::duplicate_connection_profile,
            commands::connection_profiles::delete_connection_profile,
            commands::connection_profiles::select_connection_profile,
            commands::connections::test_connection_input,
            commands::connections::connect_connection_profile,
            commands::connections::disconnect_connection_profile,
            commands::connections::list_connections,
            commands::connections::list_databases,
            commands::connections::run_query,
            commands::settings::get_app_settings,
            commands::settings::set_app_setting,
            commands::settings::import_legacy_preferences
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // Close database sessions cleanly when the app quits.
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app.state::<state::AppState>();
                tauri::async_runtime::block_on(state.connections.disconnect_all());
            }
        });
}
