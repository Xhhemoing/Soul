//! The tray icon. AC-01's visible half.
//!
//! Two entries: bring the window back, and quit. Closing the window hides it
//! instead of exiting (see `lib.rs`), so quitting has to be reachable from
//! here or the process becomes hard to stop — which is not the impression a
//! program that reads your life should give.
//!
//! Building the tray is compiled on every platform and only *works* where
//! there is a desktop session. Linux CI compiles this file and stops there;
//! whether the icon really appears in the Windows 11 notification area is on
//! the author's manual checklist, because no runner has a tray to look at.

use std::error::Error;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime};

pub const TRAY_ID: &str = "soul-tray";
pub const MENU_OPEN: &str = "tray.open";
pub const MENU_QUIT: &str = "tray.quit";
pub const MAIN_WINDOW: &str = "main";

pub fn install<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn Error>> {
    let open = MenuItem::with_id(app, MENU_OPEN, "打开 Soul", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出 Soul", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or("the bundle has no default window icon, so the tray would be invisible")?;

    TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Soul")
        .icon(icon)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_OPEN => reveal_main_window(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;

    Ok(())
}

/// Bring the main window back from wherever the user put it.
pub fn reveal_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}
