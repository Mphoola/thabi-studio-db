mod commands;
mod connections;
mod credentials;
mod errors;
mod state;
mod storage;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;

            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let database = rusqlite::Connection::open(app_data_dir.join("thabi-studio.sqlite3"))?;
            storage::migrate(&database)
                .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;
            app.manage(state::AppState::new(database));
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
            commands::connections::test_connection_profile,
            commands::connections::connect_connection_profile,
            commands::connections::disconnect_connection_profile,
            commands::connections::list_connected_profiles,
            commands::settings::get_app_settings,
            commands::settings::set_app_setting,
            commands::settings::import_legacy_preferences
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
