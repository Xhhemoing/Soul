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

    let response = get_ipc_response(
        &webview,
        InvokeRequest {
            cmd: command.to_owned(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url,
            body: body.into(),
            headers: Default::default(),
            invoke_key: INVOKE_KEY.to_owned(),
        },
    );

    match response {
        Ok(value) => Ok(value.deserialize::<Value>().expect("a JSON body")),
        Err(value) => Err(value),
    }
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

/// Nothing but the application's own page may reach the commands. The WebView
/// loads no remote page, so this is a second lock on a door that is already
/// shut — but it is the lock that would matter if the first one ever opened.
#[test]
fn a_call_claiming_a_different_origin_is_refused() {
    let refusal = invoke_from(
        "config_snapshot",
        json!({}),
        Some("https://example.invalid/page"),
    )
    .expect_err("a foreign origin gets nothing");
    assert!(
        refusal.to_string().contains("not allowed"),
        "unexpected refusal: {refusal}",
    );
}
