//! AC-02 and AC-22 over the real IPC.
//!
//! `soulcore` tests the decisions and vitest tests the screen; between them
//! sits the part neither can see — command registration, the camelCase to
//! snake_case argument conversion Tauri does, and the JSON the WebView
//! actually receives. Tauri's mock runtime has no window and no WebView, so
//! this runs on a CI host with no display while still going through
//! `invoke_handler`.
//!
//! The registration under test is `soul_desktop::configure`, which is the same
//! function the shipped binary calls. A second list here would test itself.

use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::WebviewWindowBuilder;

/// Call a command the way the WebView would, and return what it answered.
///
/// The context is the real one — `generate_context!` reads `tauri.conf.json`
/// and `capabilities/`, so the access-control list under test is the one that
/// ships. A synthetic context would allow or refuse commands on its own terms.
fn app_url<R: tauri::Runtime>(app: &tauri::App<R>) -> tauri::Url {
    app.config()
        .build
        .dev_url
        .clone()
        .expect("the configuration names a development server")
}

fn invoke(command: &str, body: Value) -> Result<Value, Value> {
    invoke_from(command, body, None)
}

/// As [`invoke`], with the option to claim a different origin.
fn invoke_from(command: &str, body: Value, origin: Option<&str>) -> Result<Value, Value> {
    let mut answers = invoke_all(&[(command, body)], origin);
    answers.pop().expect("one call, one answer")
}

/// Several calls against one shell, in order.
///
/// State a command writes has to be visible to the next command, and building
/// a fresh application per call would hide exactly that: authorising a
/// directory and then reading the list back would pass against two unrelated
/// sessions and mean nothing.
fn invoke_all(calls: &[(&str, Value)], origin: Option<&str>) -> Vec<Result<Value, Value>> {
    let app = soul_desktop::configure(mock_builder())
        .build(tauri::generate_context!())
        .expect("the mock application builds");
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .expect("a mock webview");
    let url = match origin {
        Some(url) => url.parse().expect("a url"),
        None => app_url(&app),
    };

    calls
        .iter()
        .map(|(command, body)| {
            let response = get_ipc_response(
                &webview,
                InvokeRequest {
                    cmd: (*command).to_owned(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: url.clone(),
                    body: body.clone().into(),
                    headers: Default::default(),
                    invoke_key: INVOKE_KEY.to_owned(),
                },
            );

            match response {
                Ok(value) => Ok(value.deserialize::<Value>().expect("a JSON body")),
                Err(value) => Err(value),
            }
        })
        .collect()
}

/// A directory of this test's own, outside the source tree.
///
/// `tempfile` is not a dependency here and adding one to the shell's manifest
/// to get a temporary directory would put a crate in the graph that
/// `tests/no_egress_path.rs` then has to walk. Six lines of `std` is cheaper.
fn scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("soul-ipc-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The snapshot the shell renders on launch has nothing switched on.
#[test]
fn the_configuration_that_crosses_the_ipc_is_fully_closed() {
    let snapshot = invoke("config_snapshot", json!({})).expect("the command answers");

    assert_eq!(snapshot["collect_enabled"], json!(false));
    assert_eq!(snapshot["llm_endpoint_configured"], json!(false));
    assert_eq!(snapshot["authorized_root_count"], json!(0));
    assert_eq!(snapshot["fully_closed"], json!(true));
    assert_eq!(snapshot["open_capabilities"], json!([]));
    assert_eq!(snapshot["cloud"]["state"], json!("not_yet_available"));
    assert_eq!(snapshot["cloud"]["label"], json!("尚未启用"));
}

/// AC-02, end to end: the wizard's answer is a closed configuration.
#[test]
fn a_finished_wizard_answers_with_everything_off() {
    let snapshot = invoke(
        "complete_wizard",
        json!({ "answers": { "acknowledged_defaults_are_off": true } }),
    )
    .expect("the wizard finishes");

    assert_eq!(snapshot["fully_closed"], json!(true));
    assert_eq!(snapshot["cloud"]["enabled"], json!(false));
}

/// The refusal reaches the WebView as an error rather than as a configuration
/// that quietly claims to be finished.
#[test]
fn an_unacknowledged_wizard_comes_back_as_an_error() {
    let refusal = invoke(
        "complete_wizard",
        json!({ "answers": { "acknowledged_defaults_are_off": false } }),
    )
    .expect_err("the wizard refuses");

    assert_eq!(refusal["reason"], json!("not_acknowledged"));
}

/// AC-22, end to end: asking for the cloud gets the same notice back.
#[test]
fn pressing_the_cloud_switch_returns_the_same_notice() {
    let asked_on = invoke("cloud_toggle", json!({ "requestedOn": true })).expect("an answer");
    let asked_off = invoke("cloud_toggle", json!({ "requestedOn": false })).expect("an answer");

    assert_eq!(asked_on, asked_off);
    assert_eq!(asked_on["enabled"], json!(false));
    assert_eq!(asked_on["state"], json!("not_yet_available"));
    assert_eq!(asked_on["label"], json!("尚未启用"));
    assert_eq!(asked_on["performs_network_request"], json!(false));
}

/// `requestedOn` is what `core.ts` sends. Tauri converts it to `requested_on`;
/// if that ever stopped being true the switch would silently receive `false`
/// and still look correct, so the conversion is asserted rather than assumed.
#[test]
fn the_webview_spelling_of_the_argument_is_the_one_that_arrives() {
    invoke("cloud_toggle", json!({ "requestedOn": true }))
        .expect("core.ts sends requestedOn, and it has to reach requested_on");

    assert!(
        invoke("cloud_toggle", json!({})).is_err(),
        "a command that tolerates a missing argument would hide a renamed one",
    );
}

/// A command the shell does not have must not resolve to something.
#[test]
fn an_unknown_command_is_refused() {
    assert!(invoke("execute_file_plan", json!({})).is_err());
}

/// The snapshot the shell renders on launch says nothing about a key that has
/// not landed. `configure` opens no database — that happens in `setup`, which
/// the mock runtime never reaches — and the honest answer here is to say so.
#[test]
fn a_shell_with_no_database_does_not_describe_one() {
    let snapshot = invoke("config_snapshot", json!({})).expect("the command answers");

    assert_eq!(snapshot["kek_protected"], json!(false));
    assert_eq!(
        snapshot["key_protection"],
        json!(soulcore::commands::shell::NO_STORE_OPENED_EXPLANATION),
    );
}

/// Authorising a real directory reaches the configuration the next command
/// reads, which is the whole reason the session is behind a lock.
#[test]
fn authorising_a_directory_is_visible_to_the_next_command() {
    let dir = scratch_dir("authorised");
    let canonical = dir.canonicalize().expect("a canonical path");

    let answers = invoke_all(
        &[
            ("authorized_roots", json!({})),
            (
                "authorize_root",
                json!({ "path": dir.display().to_string() }),
            ),
            ("authorized_roots", json!({})),
            ("config_snapshot", json!({})),
        ],
        None,
    );

    assert_eq!(answers[0].as_ref().expect("a list"), &json!([]));

    let snapshot = answers[1].as_ref().expect("the directory is authorised");
    assert_eq!(snapshot["authorized_root_count"], json!(1));
    assert_eq!(snapshot["fully_closed"], json!(false));
    assert_eq!(snapshot["open_capabilities"], json!(["authorized_roots"]));

    assert_eq!(
        answers[2].as_ref().expect("a list"),
        &json!([canonical.display().to_string()]),
        "the list has to read back the resolved path, not the string the user typed",
    );
    assert_eq!(
        answers[3].as_ref().expect("a snapshot")["authorized_root_count"],
        json!(1),
        "a later command must see the same session, not a fresh default",
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A path that is not a directory comes back as a refusal the user can read,
/// with the reason the core decided on rather than one the WebView guessed.
#[test]
fn a_path_that_is_not_a_directory_comes_back_as_a_readable_refusal() {
    let dir = scratch_dir("refusals");
    let file = dir.join("notes.txt");
    std::fs::write(&file, b"not a directory").expect("write the file");
    let missing = dir.join("no-such-directory");

    let cases = [
        (missing.display().to_string(), "not_found"),
        (file.display().to_string(), "not_a_directory"),
        (String::new(), "empty"),
    ];

    for (path, reason) in cases {
        let refusal = invoke("authorize_root", json!({ "path": path }))
            .expect_err("the core refuses this path");
        assert_eq!(refusal["reason"], json!(reason), "for {path:?}");
        assert!(
            refusal["message"]
                .as_str()
                .is_some_and(|message| !message.trim().is_empty()),
            "a refusal has to arrive as a sentence: {refusal}",
        );
    }

    let after = invoke("config_snapshot", json!({})).expect("a snapshot");
    assert_eq!(after["authorized_root_count"], json!(0));

    let _ = std::fs::remove_dir_all(&dir);
}

/// `path` is what `core.ts` sends. Tauri hands it to the command's `path`
/// argument; a command that tolerated its absence would hide a rename.
#[test]
fn the_authorisation_command_needs_the_path_it_is_given() {
    assert!(invoke("authorize_root", json!({})).is_err());
}

/// Mock runtime never reaches `setup`, so the store slot is empty. A view that
/// needs the database has to come back as the core's own sentence, not as a
/// panic inside the command.
#[test]
fn a_view_without_a_store_is_refused_in_the_core_s_own_words() {
    for (command, body) in [
        ("draft_view", json!({ "pasted": ["一段话"] })),
        ("fileplan_view", json!({ "target": "/tmp" })),
    ] {
        let refusal = invoke(command, body).expect_err("the mock runtime opens no store");
        assert_eq!(refusal["reason"], json!("no_store_opened"), "for {command}");
        assert_eq!(
            refusal["message"],
            json!(soulcore::commands::shell::NO_STORE_FOR_VIEW_EXPLANATION),
            "for {command}: {refusal}",
        );
    }
}

/// `pasted` and `target` are what `core.ts` sends. A command that tolerated a
/// missing argument, or accepted a renamed one, would hide the typo.
#[test]
fn the_view_commands_need_the_arguments_they_are_given() {
    for (command, body) in [
        ("draft_view", json!({})),
        ("draft_view", json!({ "paste": ["一段话"] })),
        ("fileplan_view", json!({})),
        ("fileplan_view", json!({ "path": "/tmp" })),
    ] {
        let refusal = invoke(command, body).expect_err("the argument name is part of the contract");
        assert_ne!(
            refusal.get("reason"),
            Some(&json!("no_store_opened")),
            "a missing or renamed argument must fail at the IPC, not as a view refusal: \
             {command} → {refusal}",
        );
    }
}

/// Nothing but the application's own page may reach the commands. The WebView
/// loads no remote page, so this is a second lock on a door that is already
/// shut — but it is the lock that would matter if the first one ever opened.
#[test]
fn a_call_claiming_a_different_origin_is_refused() {
    let dir = scratch_dir("foreign-origin");

    for (command, body) in [
        ("config_snapshot", json!({})),
        ("authorized_roots", json!({})),
        (
            "authorize_root",
            json!({ "path": dir.display().to_string() }),
        ),
        ("draft_view", json!({ "pasted": ["一段话"] })),
        ("fileplan_view", json!({ "target": "/tmp" })),
    ] {
        let refusal = invoke_from(command, body, Some("https://example.invalid/page"))
            .expect_err("a foreign origin gets nothing");
        assert!(
            refusal.to_string().contains("not allowed"),
            "unexpected refusal for {command}: {refusal}",
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
