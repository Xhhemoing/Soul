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
pub mod local_api;
pub mod notification_runtime;
pub mod notification_transport;
pub mod runtime_events;
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
            commands::assistance_local_preparation,
            commands::set_assistance_local_preparation,
            commands::preview_assistance_local_template,
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
            commands::prepare_task_context,
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
            commands::runtime_status,
            commands::runtime_people,
            commands::runtime_memory,
            commands::runtime_event_snapshot,
            commands::prepare_synthetic_import,
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
            commands::scoped_register_recipients,
            commands::scoped_register_status,
            commands::draft_scoped_reply,
            commands::propose_scoped_register,
            commands::confirm_scoped_register,
            commands::discard_scoped_register_proposal,
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
            commands::prepare_research_save,
            commands::confirm_research_save,
            commands::cancel_research_save,
            commands::audit_chain,
            commands::collect_status,
            commands::grant_collect_consent,
            commands::revoke_collect_consent
        ])
}

pub fn run() {
    let launch =
        local_api::LaunchMode::from_environment().expect("Soul local API configuration refused");
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
            if matches!(launch, local_api::LaunchMode::Synthetic(_)) {
                eprintln!("soul: synthetic API launch refused while Soul is running");
                return;
            }
            if !instance::reveal_running_instance() {
                // The first Soul is still starting and has no window yet.
                // Leaving is still right: it will finish, and a second store
                // is not the price of a window that is one second away.
                eprintln!("soul: Soul is already running in this session");
            }
            return;
        }
    };

    let mut context = tauri::generate_context!();
    let (state, api_host) = match launch {
        local_api::LaunchMode::Disabled => {
            // The ordinary product launch keeps its original platform path.
            let session = soulcore::commands::session::Session::open_platform_directory()
                .expect("Soul could not work out where to keep its data");
            (commands::SessionState::new(session), None)
        }
        local_api::LaunchMode::Synthetic(root) => {
            let launch =
                local_api::SyntheticLaunch::create(&root).expect("Soul synthetic launch refused");
            // Pin the explicit config and WebView2's higher-precedence process
            // override before starting workers or constructing any WebView.
            launch.configure_webview_profile(
                &mut context.config_mut().app.windows,
                |key, value| {
                    std::env::set_var(key, value);
                },
            );
            let (state, host) = launch.start().expect("Soul synthetic API startup refused");
            (state, Some(host))
        }
    };

    // Both launch branches provide their already-created sole Session owner.
    let application = configure(tauri::Builder::default(), state)
        .setup(move |app| {
            app.manage(tray::install_or_report(app.handle()));
            if let Some(host) = api_host {
                app.manage(host);
                return Ok(());
            }
            app.manage(assistance_runtime::AssistanceRuntime::start(
                app.state::<commands::SessionState>().shared(),
            ));
            let handle = app.handle().clone();
            app.manage(notification_runtime::NotificationRuntime::start(
                app.state::<commands::SessionState>().shared(),
                move |item| {
                    if let Some(window) = handle.get_webview_window("main") {
                        // `item` is a typed UUID, never source text or an external URL.
                        let _ = window
                            .eval(format!("window.location.hash = '#/assistance?item={item}'"));
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
        .build(context)
        .expect("Soul could not start");
    // The relay is externally owned so its AppHandle cannot keep itself alive
    // through managed state. Notifications may be missed before subscription;
    // UI listen-first snapshots/focus reconciliation remain authoritative.
    let relay = runtime_events::RuntimeEventRelay::start(
        &application.state::<commands::SessionState>().runtime(),
        application.handle().clone(),
    )
    .expect("Soul runtime event relay could not start");
    application.run(move |app, event| {
        if matches!(event, tauri::RunEvent::Exit) {
            // Signal every owner before waiting for any worker or collector.
            if let Some(owner) = app.try_state::<assistance_runtime::AssistanceRuntime>() {
                owner.request_stop();
            }
            if let Some(owner) = app.try_state::<notification_runtime::NotificationRuntime>() {
                owner.request_stop();
            }
            if let Some(owner) = app.try_state::<local_api::LocalApiHost>() {
                owner.request_stop();
            }
            relay.request_stop();
            if let Some(owner) = app.try_state::<assistance_runtime::AssistanceRuntime>() {
                owner.shutdown();
            }
            if let Some(owner) = app.try_state::<notification_runtime::NotificationRuntime>() {
                owner.shutdown();
            }
            if let Some(owner) = app.try_state::<local_api::LocalApiHost>() {
                owner.shutdown();
            }
            relay.shutdown();
        }
    });
}
