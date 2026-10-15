//! The native menu bar. Custom items carry the frontend command id (see
//! src/composables/useAppCommands.js) and are forwarded to the webview as a
//! `menu-command` event. Standard Edit items stay native so copy, paste and
//! undo work everywhere.
//!
//! macOS gets the usual app menu (About, Settings, Services, Hide, Quit).
//! Windows and Linux put Settings and Quit in File and About in Help, as
//! apps there do. Everywhere, Cmd/Ctrl+W closes the current tab, not the
//! window.

use tauri::{
    AppHandle, Emitter, Runtime,
    menu::{Menu, MenuBuilder, MenuItem, MenuItemBuilder, Submenu, SubmenuBuilder},
};

fn item<R: Runtime>(
    app: &AppHandle<R>,
    id: &str,
    text: &str,
    accelerator: Option<&str>,
) -> tauri::Result<MenuItem<R>> {
    let mut builder = MenuItemBuilder::with_id(id, text);
    if let Some(accelerator) = accelerator {
        builder = builder.accelerator(accelerator);
    }
    builder.build(app)
}

fn about<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<MenuItem<R>> {
    item(app, "app.about", "About Thabi Studio", None)
}

fn settings<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<MenuItem<R>> {
    item(app, "app.settings", "Settings…", Some("CmdOrCtrl+,"))
}

fn file_items<'a, R: Runtime>(
    app: &'a AppHandle<R>,
    builder: SubmenuBuilder<'a, R, AppHandle<R>>,
) -> tauri::Result<SubmenuBuilder<'a, R, AppHandle<R>>> {
    Ok(builder
        .item(&item(
            app,
            "connections.new",
            "New Connection…",
            Some("CmdOrCtrl+N"),
        )?)
        .item(&item(
            app,
            "connections.switch",
            "Switch Connection…",
            Some("CmdOrCtrl+Shift+K"),
        )?)
        .item(&item(
            app,
            "tabs.newQuery",
            "New Query Tab",
            Some("CmdOrCtrl+T"),
        )?)
        .separator()
        .item(&item(app, "tabs.close", "Close Tab", Some("CmdOrCtrl+W"))?))
}

#[cfg(target_os = "macos")]
fn app_and_file<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Vec<Submenu<R>>> {
    let app_menu = SubmenuBuilder::new(app, "Thabi Studio")
        .item(&about(app)?)
        .separator()
        .item(&settings(app)?)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;
    let file = file_items(app, SubmenuBuilder::new(app, "File"))?.build()?;
    Ok(vec![app_menu, file])
}

#[cfg(not(target_os = "macos"))]
fn app_and_file<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Vec<Submenu<R>>> {
    let file = file_items(app, SubmenuBuilder::new(app, "File"))?
        .separator()
        .item(&settings(app)?)
        .separator()
        .quit()
        .build()?;
    Ok(vec![file])
}

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let mut menus = app_and_file(app)?;
    menus.push(
        SubmenuBuilder::new(app, "Edit")
            .undo()
            .redo()
            .separator()
            .cut()
            .copy()
            .paste()
            .select_all()
            .build()?,
    );
    menus.push(
        SubmenuBuilder::new(app, "View")
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
            .build()?,
    );
    menus.push(
        SubmenuBuilder::new(app, "Query")
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
            .build()?,
    );
    #[cfg(target_os = "macos")]
    menus.push(
        SubmenuBuilder::new(app, "Window")
            .minimize()
            .maximize()
            .build()?,
    );
    let mut help = SubmenuBuilder::new(app, "Help").item(&item(
        app,
        "app.shortcuts",
        "Keyboard Shortcuts",
        Some("CmdOrCtrl+/"),
    )?);
    if cfg!(not(target_os = "macos")) {
        help = help.separator().item(&about(app)?);
    }
    menus.push(help.build()?);

    let refs: Vec<&dyn tauri::menu::IsMenuItem<R>> = menus
        .iter()
        .map(|menu| menu as &dyn tauri::menu::IsMenuItem<R>)
        .collect();
    MenuBuilder::new(app).items(&refs).build()
}

/// Forwards custom menu items to the frontend command registry.
pub fn handle_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let _ = app.emit("menu-command", id);
}
