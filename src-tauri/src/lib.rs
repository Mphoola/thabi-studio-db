mod commands;
mod connections;
mod credentials;
pub mod database;
mod errors;
pub mod filter_presets;
mod library;
mod menu;
mod profiles;
pub mod queries;
pub mod results;
mod settings;
pub mod ssh;
mod state;
mod storage;
mod windows;

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
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            credentials::init_platform_store()?;
            app.set_menu(menu::build(app.handle())?)?;
            app.on_menu_event(|app, event| menu::handle_event(app, event.id().as_ref()));
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
            commands::connection_profiles::export_connection_profiles,
            commands::connection_profiles::import_connection_profiles,
            commands::connection_profiles::update_connection_profile,
            commands::connection_profiles::duplicate_connection_profile,
            commands::connection_profiles::delete_connection_profile,
            commands::connection_profiles::select_connection_profile,
            commands::connection_profiles::reorder_connection_profiles,
            commands::connections::test_connection_input,
            commands::connections::connect_connection_profile,
            commands::connections::disconnect_connection_profile,
            commands::connections::list_connections,
            commands::connections::list_databases,
            commands::connections::list_schemas,
            commands::connections::list_objects,
            commands::connections::describe_object,
            commands::connections::open_object_data,
            commands::connections::fetch_table_page,
            commands::connections::count_table_rows,
            commands::connections::copy_table,
            commands::connections::export_table,
            commands::connections::preview_row_changes,
            commands::connections::apply_row_changes,
            commands::connections::lookup_reference,
            commands::connections::list_referencing,
            commands::connections::preview_filter,
            commands::connections::completion_schema,
            commands::connections::explain_statement,
            commands::connections::query_progress,
            commands::connections::read_sql_file,
            commands::connections::run_sql_file,
            commands::connections::sql_file_progress,
            windows::open_workspace_window,
            commands::connections::run_statement,
            commands::connections::cancel_statement,
            commands::connections::close_query_tab,
            commands::connections::fetch_rows,
            commands::connections::copy_rows,
            commands::connections::export_csv,
            commands::connections::export_result,
            commands::connections::trust_ssh_host_key,
            commands::connections::resolve_ssh_alias,
            commands::connections::forget_ssh_host_keys,
            commands::library::list_history,
            commands::library::delete_history_entry,
            commands::library::clear_history,
            commands::library::list_saved_queries,
            commands::library::save_query,
            commands::library::delete_saved_query,
            commands::library::list_filter_presets,
            commands::library::save_filter_preset,
            commands::library::delete_filter_preset,
            commands::library::rename_saved_folder,
            commands::library::save_workspace_tabs,
            commands::library::load_workspace_tabs,
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
