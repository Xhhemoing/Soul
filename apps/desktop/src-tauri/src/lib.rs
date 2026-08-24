//! Soul's desktop shell.
//!
//! Everything this crate does is hosting: it opens one window on local files,
//! puts an icon in the tray, opens Soul's store once, and forwards commands to
//! `soulcore`. There is no business logic here and there is no network client
//! anywhere in its dependency graph except the one `soul-egress` owns, which
//! only moves when the user has configured their own endpoint and pressed
//! something.
//!
//! What the shell promises, and where each promise is checked:
//!
//! | promise | where |
//! |---|---|
//! | runs as the invoking user, no elevation | `windows/soul.exe.manifest`, `tests/shell_is_local_only.rs` |
//! | no automatic updates | `tauri.conf.json`, `tests/no_egress_path.rs` |
//! | the WebView loads only what shipped with it | the CSP in `tauri.conf.json` |
//! | a tray entry exists | `tray.rs`; the icon itself is author-manual |
//! | one store handle for the process | [`run`] opens it, `tests/one_store.rs` |

#![forbid(unsafe_code)]

use tauri::Manager;

pub mod commands;
pub mod tray;

/// State and command registration, kept separate from the window and the tray.
///
/// The session is a parameter rather than something built here, and that is
/// the whole of "one store handle per process": there is one `Session`, the
/// caller made it, and a second call to `configure` would need a second one
/// handed to it deliberately. `tests/ipc_roundtrip.rs` passes a session on a
/// scratch directory and gets the commands the shipped binary registers,
/// rather than a second list that happens to look the same.
pub fn configure<R: tauri::Runtime>(
    builder: tauri::Builder<R>,
    session: commands::SessionState,
) -> tauri::Builder<R> {
    builder
        .manage(session)
        .invoke_handler(tauri::generate_handler![
            commands::config_snapshot,
            commands::session_status,
            commands::complete_wizard,
            commands::cloud_toggle,
            commands::files_view,
            commands::authorize_directory,
            commands::preview_plan,
            commands::people_graph,
            commands::person_summary,
            commands::draft_reply,
            commands::draft_notices,
            commands::prepare_draft,
            commands::generate_draft,
            commands::discard_draft
        ])
}

pub fn run() {
    // The one place a database is opened. `soulcore` works out where this
    // machine keeps Soul's data — `%LOCALAPPDATA%\Soul` on the platform this
    // ships to — and a session that could not be built is a Soul that has
    // nowhere to keep anything, which is worth stopping for rather than
    // starting an interface that would refuse every route.
    let session = soulcore::commands::session::Session::open_platform_directory()
        .expect("Soul could not work out where to keep its data");

    configure(
        tauri::Builder::default(),
        commands::SessionState::new(session),
    )
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
