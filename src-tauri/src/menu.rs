//! The native macOS menu bar. Custom items carry the frontend command id
//! (see src/composables/useAppCommands.js) and are forwarded to the webview
//! as a `menu-command` event. Standard Edit items stay native so copy,
//! paste and undo work everywhere.
//!
//! The default Tauri menu binds ⌘W to "Close Window"; here ⌘W closes the
//! current tab instead, as in other tabbed Mac apps.

use tauri::{
    AppHandle, Emitter, Runtime,
    menu::{Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
};

fn item<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    text: &str,
    accelerator: Option<&str>,
) -> tauri::Result<tauri::menu::MenuItem<R>> {
    let mut builder = MenuItemBuilder::with_id(id, text);
    if let Some(accelerator) = accelerator {
        builder = builder.accelerator(accelerator);
    }
    builder.build(app)
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let app_menu = SubmenuBuilder::new(app, "Thabi Studio")
        .item(&item(app, "app.about", "About Thabi Studio", None)?)
        .separator()
        .item(&item(
            app,
            "app.settings",
            "Settings…",
            Some("CmdOrCtrl+,"),
        )?)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let file = SubmenuBuilder::new(app, "File")
        .item(&item(
            app,
            "connections.new",
            "New Connection…",
            Some("CmdOrCtrl+N"),
        )?)
        .item(&item(
            app,
            "tabs.newQuery",
            "New Query Tab",
            Some("CmdOrCtrl+T"),
        )?)
        .separator()
        .item(&item(app, "tabs.close", "Close Tab", Some("CmdOrCtrl+W"))?)
        .build()?;
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;
    let view = SubmenuBuilder::new(app, "View")
        .item(&item(
            app,
            "app.commandPalette",
            "Open Anything",
            Some("CmdOrCtrl+P"),
        )?)
        .separator()
        .item(&item(
            app,
            "view.toggleSidebar",
            "Toggle Sidebar",
            Some("CmdOrCtrl+1"),
        )?)
        .item(&item(
            app,
            "explorer.refresh",
            "Refresh Explorer",
            Some("CmdOrCtrl+R"),
        )?)
        .item(&item(app, "view.connections", "Show Connections", None)?)
        .separator()
        .fullscreen()
        .build()?;
    let query = SubmenuBuilder::new(app, "Query")
        .item(&item(
            app,
            "query.run",
            "Run Statement",
            Some("CmdOrCtrl+Enter"),
        )?)
        .item(&item(
            app,
            "query.runAll",
            "Run All",
            Some("CmdOrCtrl+Shift+Enter"),
        )?)
        .item(&item(
            app,
            "query.cancel",
            "Cancel Query",
            Some("CmdOrCtrl+."),
        )?)
        .build()?;
    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .build()?;
    let help = SubmenuBuilder::new(app, "Help")
        .item(&item(
            app,
            "app.shortcuts",
            "Keyboard Shortcuts",
            Some("CmdOrCtrl+/"),
        )?)
        .build()?;

    MenuBuilder::new(app)
        .items(&[&app_menu, &file, &edit, &view, &query, &window, &help])
        .build()
}

/// Forwards custom menu items to the frontend command registry.
pub fn handle_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let _ = app.emit("menu-command", id);
}
