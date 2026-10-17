mod app_lock;
mod app_recovery;
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

type InvokeHandler<R> = Box<dyn Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync>;

/// Wraps the command handler with the app lock: while Thabi Studio is locked,
/// only the lock screen's own commands run (see `app_lock`).
fn guarded<R: tauri::Runtime>(
    handler: impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static,
) -> InvokeHandler<R> {
    Box::new(move |invoke| {
        let blocked = !app_lock::allowed_while_locked(invoke.message.command())
            && invoke
                .message
                .webview()
                .state::<state::AppState>()
                .app_lock
                .is_locked();
        if blocked {
            invoke.resolver.reject(app_lock::locked());
            return true;
        }
        handler(invoke)
    })
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
        .invoke_handler(guarded(tauri::generate_handler![
            commands::app::app_info,
            commands::app_lock::app_lock_status,
            commands::app_lock::unlock_app,
            commands::app_lock::lock_app,
            commands::app_lock::set_app_password,
            commands::app_lock::remove_app_password,
            commands::app_lock::reset_app_password,
            commands::app_recovery::recovery_options,
            commands::app_recovery::recover_with_code,
            commands::app_recovery::recover_with_answers,
            commands::app_recovery::regenerate_recovery_codes,
            commands::app_recovery::set_security_questions,
            commands::app_recovery::remove_security_questions,
            commands::app_recovery::save_recovery_codes,
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
            commands::connections::schema_diagram,
            commands::connections::describe_object,
            commands::connections::open_object_data,
            commands::connections::fetch_table_page,
            commands::connections::count_table_rows,
            commands::connections::copy_table,
            commands::connections::export_sql,
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
            commands::connections::preview_table_ddl,
            commands::connections::apply_table_ddl,
            commands::connections::preview_table_action,
            commands::connections::apply_table_action,
            commands::library::rename_saved_folder,
            commands::library::save_workspace_tabs,
            commands::library::load_workspace_tabs,
            commands::settings::get_app_settings,
            commands::settings::set_app_setting,
            commands::settings::import_legacy_preferences
        ]))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::test::{INVOKE_KEY, get_ipc_response, mock_builder, mock_context, noop_assets};
    use tauri::webview::InvokeRequest;

    fn request(cmd: &str) -> InvokeRequest {
        InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            // The app's own origin, which differs on Windows.
            url: if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            }
            .parse()
            .unwrap(),
            body: tauri::ipc::InvokeBody::default(),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.into(),
        }
    }

    #[test]
    fn a_locked_app_refuses_commands_before_they_run() {
        let app = mock_builder()
            .invoke_handler(guarded(tauri::generate_handler![
                commands::settings::get_app_settings,
                commands::library::list_history,
            ]))
            .build(mock_context(noop_assets()))
            .unwrap();
        let mut database = rusqlite::Connection::open_in_memory().unwrap();
        storage::migrate(&mut database).unwrap();
        app.manage(state::AppState::new(database));
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        // Locked from startup until the credential store has been read.
        let refused = get_ipc_response(&webview, request("list_history")).unwrap_err();
        assert_eq!(refused["kind"], "locked");
        assert!(get_ipc_response(&webview, request("get_app_settings")).is_ok());

        let state = app.state::<state::AppState>();
        state.app_lock.load(true);
        assert!(get_ipc_response(&webview, request("list_history")).is_err());
        state.app_lock.unlock();
        assert!(get_ipc_response(&webview, request("list_history")).is_ok());
        state.app_lock.lock();
        let refused = get_ipc_response(&webview, request("list_history")).unwrap_err();
        assert_eq!(refused["kind"], "locked");
    }
}
