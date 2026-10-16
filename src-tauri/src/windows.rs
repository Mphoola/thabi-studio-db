//! Extra workspace windows (File > New Window). Each window has its own tabs
//! and selected connection; live connections stay shared in Rust, and every
//! query or table tab still has its own session, keyed by a tab id that is
//! unique across windows.

use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder};

static NEXT: AtomicU32 = AtomicU32::new(1);

/// Opens another workspace window, labelled `workspace-N` (the capability
/// in capabilities/default.json covers `workspace-*`).
pub fn open_workspace<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let label = loop {
        let label = format!("workspace-{}", NEXT.fetch_add(1, Ordering::Relaxed));
        if app.get_webview_window(&label).is_none() {
            break label;
        }
    };
    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title("Thabi Studio")
        .inner_size(1200.0, 780.0)
        .min_inner_size(860.0, 560.0);
    // Same chrome as the main window on macOS (tauri.macos.conf.json).
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(14.0, 19.0));
    builder.build().map(|_| ())
}

/// Opens a new workspace window from the frontend (the command palette).
#[tauri::command]
pub async fn open_workspace_window(app: AppHandle) -> Result<(), crate::errors::AppError> {
    open_workspace(&app).map_err(|e| {
        crate::errors::AppError::new("window", "A new window couldn't be opened.")
            .with_detail(Some(e.to_string()))
    })
}
