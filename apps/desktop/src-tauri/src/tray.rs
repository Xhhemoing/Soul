//! The tray icon. AC-01's visible half.
//!
//! Two menu entries — bring the window back, and quit — and a left click that
//! does the first of them without opening a menu, because that is what a left
//! click on a notification-area icon has always meant. When the tray is there,
//! closing the window hides it instead of exiting (see `lib.rs`), so quitting
//! has to be reachable from here or the process becomes hard to stop — which
//! is not the impression a program that reads your life should give. When the
//! tray is not there, the window closes for real; `install_or_report` says
//! which of the two this session got.
//!
//! The reveal is the same three calls wherever it is asked for: the menu entry,
//! the left click, and a second launch that found the name taken all end at
//! [`reveal_main_window`] or, from another process, at the `ShowWindow` pair in
//! `instance.rs`.
//!
//! Building the tray is compiled on every platform and only *works* where
//! there is a desktop session. Linux CI compiles this file and stops there;
//! whether the icon really appears in the Windows 11 notification area is on
//! the author's manual checklist, because no runner has a tray to look at.

use std::error::Error;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};

/// Whether this session has a tray. Read by the window-close handler.
#[derive(Debug, Clone, Copy)]
pub struct TrayState {
    pub installed: bool,
}

pub const TRAY_ID: &str = "soul-tray";
pub const MENU_OPEN: &str = "tray.open";
pub const MENU_QUIT: &str = "tray.quit";
pub const MAIN_WINDOW: &str = "main";

/// Install the tray, or say so and carry on.
///
/// Failing to start over a missing notification area would be the wrong trade:
/// on Windows 11 there is always one, and on a desktop that has none the user
/// is better served by a window they can close.
pub fn install_or_report<R: Runtime>(app: &AppHandle<R>) -> TrayState {
    match install(app) {
        Ok(()) => TrayState { installed: true },
        Err(error) => {
            eprintln!(
                "soul: no tray icon on this desktop ({error}); \
                 closing the window will quit instead of hiding"
            );
            TrayState { installed: false }
        }
    }
}

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
        // Windows puts the context menu on the right button and the
        // application itself on the left one. Tauri's default puts the menu on
        // both, which turns the gesture a user reaches for first into one more
        // thing to read.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            MENU_OPEN => reveal_main_window(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        // The same reveal as 打开 Soul, on the gesture the notification area
        // has always meant it by. `Up` is the end of the click; acting on
        // `Down` would raise the window out from under a user who was on their
        // way to a drag. Linux emits no tray events at all, which is one more
        // reason the menu entry stays.
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal_main_window(tray.app_handle());
            }
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
