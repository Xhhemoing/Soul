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

pub mod commands;
pub mod tray;

pub fn run() {
    tauri::Builder::default()
        .manage(commands::SessionConfig::default())
        .invoke_handler(tauri::generate_handler![
            commands::config_snapshot,
            commands::complete_wizard,
            commands::cloud_toggle
        ])
        .setup(|app| {
            tray::install(app.handle())?;
            Ok(())
        })
        // Closing the window puts Soul in the tray rather than ending it. The
        // tray menu is where quitting lives, so the user always has one.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("Soul could not start");
}
