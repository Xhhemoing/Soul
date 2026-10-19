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
//! | one Soul per signed-in user | `instance.rs`, `tests/shell_is_local_only.rs` |

// The single-instance and notification Windows bindings declare Win32 entry
// points: on Windows the root denies unsafe code rather than forbidding
// it, so those modules can allow it back for themselves, and everywhere else the
// stronger `forbid` still holds. Same arrangement, and same reason, as
// `crates/soul-collect` and `crates/soul-win-dpapi`.
#![cfg_attr(not(windows), forbid(unsafe_code))]
#![cfg_attr(windows, deny(unsafe_code))]

use tauri::Manager;

pub mod assistance_runtime;
pub mod commands;
pub mod instance;
pub mod notification_runtime;
pub mod notification_transport;
pub mod tray;

/// State and command registration, kept separate from the window and the tray.
///
/// The session is a parameter rather than something built here, and that is
/// the whole of "one store handle per process": there is one `Session`, the
/// caller made it, and a second call to `configure` would need a second one
/// handed to it deliberately. `tests/ipc_roundtrip.rs` passes a session on a
/// scratch directory and gets the commands the shipped binary registers,
/// rather than a second list that happens to look the same.
///
/// The single-instance claim is not made here for the same reason: it is a
/// name in the logon session, and a test that took it would be reaching out of
/// its own process to stop the author's Soul from starting. [`run`] claims it;
/// `configure` registers commands.
pub fn configure<R: tauri::Runtime>(
    builder: tauri::Builder<R>,
    session: commands::SessionState,
) -> tauri::Builder<R> {
    builder
        .manage(session)
        .invoke_handler(tauri::generate_handler![
            commands::assistance_reminder,
            commands::set_assistance_reminder,
            commands::assistance_feedback,
            commands::set_assistance_feedback,
            commands::assistance_category_preferences,
            commands::set_assistance_category_preference,
            commands::retry_assistance_notification_withdrawal,
            commands::assistance_notification_runtime_status,
            commands::assistance_work_status,
            commands::assistance_item_work,
            commands::retry_assistance_work,
            commands::assistance_items,
            commands::assistance_item,
            commands::create_assistance_goal,
            commands::edit_assistance_goal,
            commands::link_assistance_observation,
            commands::refresh_assistance_observation,
            commands::bind_assistance_goal_source,
            commands::act_assistance_item,
            commands::prepare_assistance_context,
            commands::local_insights,
            commands::ignore_local_insight,
            commands::recompute_local_insight,
            commands::import_recovery_state,
            commands::prepare_memory_context,
            commands::prepare_insight_context,
            commands::evidence_sources,
            commands::read_evidence_event,
            commands::local_draft_with_insight,
            commands::prepare_draft_with_insight,
            commands::draft_reply_with_memory,
            commands::prepare_draft_with_memory,
            commands::list_import_namespaces,
            commands::recent_import_operations,
            commands::import_operation,
            commands::discard_import,
            commands::prepare_import,
            commands::confirm_import,
            commands::config_snapshot,
            commands::session_status,
            commands::complete_wizard,
            commands::cloud_toggle,
            commands::files_view,
            commands::authorize_directory,
            commands::preview_plan,
            commands::people_graph,
            commands::person_summary,
            commands::correct_tie,
            commands::release_tie,
            commands::draft_reply,
            commands::draft_notices,
            commands::prepare_draft,
            commands::generate_draft,
            commands::discard_draft,
            commands::set_user_endpoint,
            commands::clear_user_endpoint,
            commands::preview_soul_import_v1,
            commands::preview_telegram,
            commands::commit_soul_import_v1,
            commands::commit_telegram,
            commands::questionnaire,
            commands::answer_questionnaire,
            commands::profile_screen,
            commands::correct_axis,
            commands::set_voice,
            commands::memory_list,
            commands::memory_detail,
            commands::create_memory,
            commands::update_memory,
            commands::preview_forget,
            commands::forget_memory,
            commands::retry_forget_cleanup,
            commands::research_preview,
            commands::audit_chain,
            commands::collect_status,
            commands::grant_collect_consent,
            commands::revoke_collect_consent
        ])
}

pub fn run() {
    // Before anything else, and before a database in particular. Closing the
    // window leaves Soul in the tray, so the second launch is the ordinary way
    // back in: the user clicks Soul in the Start menu again. That process must
    // not reach the store — two handles on one SQLCipher file are two
    // write-ahead logs — so it claims the name first, hands the window back to
    // the Soul that already has it, and leaves. Exiting `run` here returns
    // through `main` with status 0: nothing failed, this launch was answered
    // by the process that was already running.
    let _instance = match instance::claim() {
        instance::Claim::Ours(guard) => guard,
        instance::Claim::AlreadyRunning => {
            if !instance::reveal_running_instance() {
                // The first Soul is still starting and has no window yet.
                // Leaving is still right: it will finish, and a second store
                // is not the price of a window that is one second away.
                eprintln!("soul: Soul is already running in this session");
            }
            return;
        }
    };

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
        app.manage(assistance_runtime::AssistanceRuntime::start(
            app.state::<commands::SessionState>().shared(),
        ));
        let handle = app.handle().clone();
        app.manage(notification_runtime::NotificationRuntime::start(
            app.state::<commands::SessionState>().shared(),
            move |item| {
                if let Some(window) = handle.get_webview_window("main") {
                    // `item` is a typed UUID, never source text or an external URL.
                    let _ = window.eval(&format!(
                        "window.location.hash = '#/assistance?item={item}'"
                    ));
                }
                tray::reveal_main_window(&handle);
            },
        ));
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
    .build(tauri::generate_context!())
    .expect("Soul could not start")
    .run(|app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            // Signal every owner before waiting for any worker or collector.
            app.state::<assistance_runtime::AssistanceRuntime>()
                .request_stop();
            app.state::<notification_runtime::NotificationRuntime>()
                .request_stop();
            app.state::<assistance_runtime::AssistanceRuntime>()
                .shutdown();
            app.state::<notification_runtime::NotificationRuntime>()
                .shutdown();
        }
    });
}
