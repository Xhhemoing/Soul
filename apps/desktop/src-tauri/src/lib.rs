//! Soul's desktop shell.
//!
//! Everything this crate does is hosting: it opens one window on local files,
//! puts an icon in the tray, and forwards three commands to `soulcore`. There
//! is no business logic here and there is no network client anywhere in its
//! dependency graph except the one `soul-egress` owns, which only moves when
//! the user has configured their own endpoint and pressed something.
//!
//! What the shell promises, and where each promise is checked:
//!
//! | promise | where |
//! |---|---|
//! | runs as the invoking user, no elevation | `windows/soul.exe.manifest`, `tests/shell_is_local_only.rs` |
//! | no automatic updates | `tauri.conf.json`, `tests/no_egress_path.rs` |
//! | the WebView loads only what shipped with it | the CSP in `tauri.conf.json` |
//! | a tray entry exists | `tray.rs`; the icon itself is author-manual |

#![forbid(unsafe_code)]

use tauri::Manager;

pub mod commands;
pub mod tray;

/// State and command registration, kept separate from the window and the tray.
///
/// `tests/ipc_roundtrip.rs` builds on this with Tauri's mock runtime, so the
/// commands it calls are the ones the shipped binary registers rather than a
/// second list that happens to look the same.
pub fn configure<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(commands::SessionConfig::default())
        .invoke_handler(tauri::generate_handler![
            commands::config_snapshot,
            commands::complete_wizard,
            commands::cloud_toggle
        ])
}

pub fn run() {
    configure(tauri::Builder::default())
        .setup(|app| {
            app.manage(tray::install_or_report(app.handle()));
            Ok(())
        })
        // Closing the window puts Soul in the tray rather than ending it —
        // but only when there is a tray to put it in. Without one the tray
        // menu's 退出 is unreachable, and a window that will not close and
        // cannot be quit is worse than no tray at all.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.state::<tray::TrayState>().installed {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Soul could not start");
}
