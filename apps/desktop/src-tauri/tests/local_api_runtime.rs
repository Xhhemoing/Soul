use serde_json::{json, Value};
use soul_desktop::local_api::{LaunchMode, SyntheticLaunch};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
use tauri::webview::InvokeRequest;
use tauri::{Manager, WebviewWindowBuilder};

#[test]
fn disabled_configuration_never_selects_a_directory_or_starts_a_server() {
    assert!(matches!(
        LaunchMode::parse(None, None).unwrap(),
        LaunchMode::Disabled
    ));
    assert!(LaunchMode::parse(Some("synthetic"), None).is_err());
    assert!(LaunchMode::parse(Some("on"), Some("relative")).is_err());
    assert!(LaunchMode::parse(Some("synthetic"), Some("relative")).is_err());
}

#[test]
fn synthetic_webview_profile_replaces_both_configuration_override_sources() {
    let root = tempfile::tempdir().unwrap();
    let launch = SyntheticLaunch::create(root.path()).unwrap();
    let mut windows = vec![
        tauri::utils::config::WindowConfig::default(),
        tauri::utils::config::WindowConfig::default(),
    ];
    let inherited_profile = root.path().join("synthetic-existing-profile");
    for window in &mut windows {
        window.data_directory = Some(inherited_profile.clone());
    }
    let mut writes = Vec::new();
    launch.configure_webview_profile(&mut windows, |key, value| {
        writes.push((key.to_owned(), value.to_owned()));
    });
    assert!(windows
        .iter()
        .all(|window| window.data_directory.as_deref() == Some(launch.profile_directory())));
    assert_eq!(
        writes,
        vec![(
            std::ffi::OsString::from("WEBVIEW2_USER_DATA_FOLDER"),
            launch.profile_directory().as_os_str().to_owned()
        )]
    );
    assert_ne!(launch.profile_directory(), inherited_profile);
    assert!(launch.profile_directory().is_dir());
    // The setter was captured locally: no process environment, existing
    // WebView profile, native window or registered override was accessed.
}

#[test]
fn production_ipc_and_http_read_the_same_live_session_and_then_release_it() {
    let root = tempfile::tempdir().unwrap();
    let launch = SyntheticLaunch::create(root.path()).unwrap();
    let data = launch.data_directory().to_owned();
    let (state, host) = launch.start().unwrap();
    let document: Value =
        serde_json::from_slice(&std::fs::read(host.credentials_path()).unwrap()).unwrap();
    let endpoint = document["endpoint"].as_str().unwrap();
    let authority = endpoint.strip_prefix("http://").unwrap();
    let app = soul_desktop::configure(mock_builder(), state)
        .build(tauri::generate_context!())
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let invoke = |cmd: &str, body: Value| -> Result<Value, Value> {
        get_ipc_response(
            &webview,
            InvokeRequest {
                cmd: cmd.to_owned(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: app.config().build.dev_url.clone().unwrap(),
                body: body.into(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            },
        )
        .map(|response| response.deserialize().unwrap())
    };
    let memory = invoke("create_memory", json!({"memory": {
        "memory_type": "preference", "title": "synthetic-api", "summary": "first synthetic summary"
    }})).unwrap();
    let memory_id = memory["memory_id"].as_str().unwrap();
    let read = |path: &str| -> Value {
        let mut stream = TcpStream::connect(authority).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {}\r\n\r\n",
            document["token"].as_str().unwrap()
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        serde_json::from_str(response.split("\r\n\r\n").nth(1).unwrap()).unwrap()
    };
    assert_eq!(
        read(&format!("/v1/memories/{memory_id}"))["summary"],
        "first synthetic summary"
    );
    invoke(
        "update_memory",
        json!({"memoryId": memory_id, "change": {
            "title": "synthetic-api-updated", "summary": "second synthetic summary"
        }}),
    )
    .unwrap();
    assert_eq!(
        read(&format!("/v1/memories/{memory_id}"))["summary"],
        "second synthetic summary"
    );
    assert_eq!(
        read("/v1/status"),
        invoke("runtime_status", json!({})).unwrap()
    );
    assert_eq!(
        read("/v1/people?offset=0&limit=1"),
        invoke(
            "runtime_people",
            json!({"request": {"offset": 0, "limit": 1}})
        )
        .unwrap()
    );
    assert!(invoke("grant_collect_consent", json!({})).is_err());
    assert!(invoke(
        "set_user_endpoint",
        json!({"url": "https://example.invalid"})
    )
    .is_err());
    let busy = soulcore::commands::session::Session::open(&data);
    assert!(!busy.status().store_opened);
    drop(busy);
    host.request_stop();
    host.shutdown();
    assert!(TcpStream::connect(authority).is_err());
    #[allow(deprecated)]
    drop(
        webview
            .unmanage::<soul_desktop::commands::SessionState>()
            .unwrap(),
    );
    drop(host);
    drop(webview);
    drop(app);
    let reopened = soulcore::commands::session::Session::open(data);
    assert!(reopened.status().store_opened);
    let persisted = reopened.memory(memory_id).unwrap();
    assert_eq!(persisted.title, "synthetic-api-updated");
    assert_eq!(persisted.summary, "second synthetic summary");
}

#[test]
fn enabled_host_uses_a_fresh_run_and_removes_credentials_on_shutdown() {
    let root = tempfile::tempdir().unwrap();
    let existing = root.path().join("data");
    std::fs::create_dir(&existing).unwrap();
    std::fs::write(existing.join("sentinel"), b"synthetic-existing-not-read").unwrap();
    let launch = SyntheticLaunch::create(root.path()).unwrap();
    assert_ne!(launch.data_directory(), existing);
    let data = launch.data_directory().to_owned();
    let (state, host) = launch.start().unwrap();
    assert!(state.runtime().status().store_opened);
    let credentials = host.credentials_path().to_owned();
    assert!(credentials.is_file());
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&credentials).unwrap()).unwrap();
    assert!(document["endpoint"]
        .as_str()
        .unwrap()
        .starts_with("http://127.0.0.1:"));
    assert!(document["token"].as_str().unwrap().len() >= 64);
    assert!(existing.join("sentinel").is_file());
    host.request_stop();
    host.shutdown();
    assert!(!credentials.exists());
    drop(host);
    drop(state);
    assert!(
        soulcore::commands::session::Session::open(data)
            .status()
            .store_opened
    );
}

struct N3Shell {
    app: tauri::App<tauri::test::MockRuntime>,
    webview: tauri::WebviewWindow<tauri::test::MockRuntime>,
    host: soul_desktop::local_api::LocalApiHost,
    authority: String,
    token: String,
    relay: Option<soul_desktop::runtime_events::RuntimeEventRelay>,
}

impl N3Shell {
    fn new(root: &std::path::Path) -> (Self, std::path::PathBuf) {
        let launch = SyntheticLaunch::create(root).unwrap();
        let data = launch.data_directory().to_owned();
        let (state, host) = launch.start().unwrap();
        let credentials: Value =
            serde_json::from_slice(&std::fs::read(host.credentials_path()).unwrap()).unwrap();
        let app = soul_desktop::configure(mock_builder(), state)
            .build(tauri::generate_context!())
            .unwrap();
        let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        (
            Self {
                app,
                webview,
                host,
                authority: credentials["endpoint"]
                    .as_str()
                    .unwrap()
                    .strip_prefix("http://")
                    .unwrap()
                    .to_owned(),
                token: credentials["token"].as_str().unwrap().to_owned(),
                relay: None,
            },
            data,
        )
    }
    fn invoke(&self, cmd: &str, body: Value) -> Result<Value, Value> {
        get_ipc_response(
            &self.webview,
            InvokeRequest {
                cmd: cmd.to_owned(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: self.app.config().build.dev_url.clone().unwrap(),
                body: body.into(),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_owned(),
            },
        )
        .map(|response| response.deserialize().unwrap())
    }
    fn http(&self, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
        let body = body.map(|body| body.to_string()).unwrap_or_default();
        let mut stream = TcpStream::connect(&self.authority).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(8)))
            .unwrap();
        write!(stream, "{method} {path} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", self.authority, self.token, body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        (
            response.split_whitespace().nth(1).unwrap().parse().unwrap(),
            serde_json::from_str(response.split("\r\n\r\n").nth(1).unwrap()).unwrap(),
        )
    }
}

impl Drop for N3Shell {
    fn drop(&mut self) {
        self.host.request_stop();
        if let Some(relay) = &self.relay {
            relay.request_stop();
        }
        self.host.shutdown();
        if let Some(relay) = self.relay.take() {
            relay.shutdown();
        }
        #[allow(deprecated)]
        drop(
            self.webview
                .unmanage::<soul_desktop::commands::SessionState>(),
        );
    }
}

#[test]
fn native_runtime_relay_emits_http_started_settled_and_drops_without_retaining_session() {
    use tauri::Listener;
    let root = tempfile::tempdir().unwrap();
    let (mut shell, data) = N3Shell::new(root.path());
    let (sent, received) = std::sync::mpsc::channel::<soul_types::RuntimeEvent>();
    let listener = shell
        .app
        .listen(soul_desktop::runtime_events::EVENT_NAME, move |event| {
            sent.send(serde_json::from_str(event.payload()).unwrap())
                .unwrap();
        });
    let runtime = shell
        .webview
        .state::<soul_desktop::commands::SessionState>()
        .runtime();
    shell.relay = Some(
        soul_desktop::runtime_events::RuntimeEventRelay::start(
            &runtime,
            shell.app.handle().clone(),
        )
        .unwrap(),
    );
    let prepared = shell.http(
        "POST",
        "/v1/imports/prepare",
        Some(json!({"fixture_id":"basic"})),
    );
    assert_eq!(prepared.0, 200);
    let first = received.recv_timeout(Duration::from_secs(5)).unwrap();
    let last = received.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(first.phase, soul_types::OperationPhase::Started);
    assert_eq!(last.phase, soul_types::OperationPhase::Settled);
    assert_eq!(first.operation_id, last.operation_id);
    assert_eq!(first.caller.transport, soul_types::CallerTransport::Http);
    assert_eq!(first.caller.webview_label, None);
    assert_eq!(first.correlation_id, None);
    assert_eq!(last.outcome, Some(soul_types::OperationOutcome::Success));
    assert_eq!(last.sequence, first.sequence + 1);
    assert_eq!(
        shell.invoke("runtime_event_snapshot", json!({})).unwrap()["sequence"],
        last.sequence
    );
    let failed = shell.http(
        "PATCH",
        "/v1/memories/00000000-0000-0000-0000-000000000000",
        Some(json!({"summary":"SYNTHETIC_N3_REJECTED_BODY"})),
    );
    assert_ne!(failed.0, 200);
    let failed_start = received.recv_timeout(Duration::from_secs(5)).unwrap();
    let failed_finish = received.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(failed_start.phase, soul_types::OperationPhase::Started);
    assert_eq!(failed_finish.phase, soul_types::OperationPhase::Settled);
    assert_eq!(failed_start.operation_id, failed_finish.operation_id);
    assert_eq!(
        failed_finish.outcome,
        Some(soul_types::OperationOutcome::Refused)
    );
    assert_eq!(failed_start.sequence, last.sequence + 1);
    let payload = serde_json::to_string(&last).unwrap();
    for private in [
        "SYNTHETIC_N3_PRIVATE_BODY",
        "summary",
        "token",
        "preview_id",
    ] {
        assert!(!payload.contains(private));
    }
    shell.app.unlisten(listener);
    // Exercise the RAII path, without a separate explicit relay shutdown.
    drop(shell.relay.take());
    let snapshot = runtime.event_snapshot();
    assert!(snapshot.active_operations.is_empty());
    drop(runtime);
    drop(shell);
    assert!(
        soulcore::commands::session::Session::open(data)
            .status()
            .store_opened
    );
}

#[test]
fn n3_reverse_http_preference_and_ipc_forget_keep_the_original_owners() {
    let root = tempfile::tempdir().unwrap();
    let (shell, data) = N3Shell::new(root.path());
    let prepared = shell
        .invoke(
            "prepare_synthetic_import",
            json!({"request":{"fixture_id":"basic"}}),
        )
        .unwrap();
    let operation = prepared["operation"]["operation_id"].as_str().unwrap();
    let preview = prepared["operation"]["preview_id"].as_str().unwrap();
    let denied = shell.http(
        "POST",
        &format!("/v1/imports/{operation}/confirm"),
        Some(json!({"preview_id":preview})),
    );
    assert_eq!(denied.0, 403);
    assert_eq!(denied.1["code"], "caller_mismatch");
    shell
        .invoke(
            "confirm_import",
            json!({"operationId":operation,"previewId":preview}),
        )
        .unwrap();
    // Scoped drafts require the same explicit profile choice as the UI flow.
    shell
        .invoke("set_voice", json!({"field":"register","option":"formal"}))
        .unwrap();
    let people = shell
        .invoke("scoped_register_recipients", json!({}))
        .unwrap();
    let person = people[0]["person_id"].as_str().unwrap();
    let draft = shell.http(
        "POST",
        "/v1/scoped/draft",
        Some(json!({"pasted":"SYNTHETIC_HTTP_SCOPED_BODY","person_id":person,"occasion":"work"})),
    );
    assert_eq!(draft.0, 200);
    let proposal = shell.http(
        "POST",
        "/v1/scoped/propose",
        Some(json!({"result_id":draft.1["result_id"],"edit_revision":1,"register":"plain"})),
    );
    assert_eq!(proposal.0, 200);
    let denied = shell.invoke("confirm_scoped_register", json!({"proposalId":proposal.1["proposal_id"],"expectedRevision":proposal.1["expected_revision"]})).unwrap_err();
    assert_eq!(denied["reason_code"], "RUNTIME_CALLER_MISMATCH");
    let saved = shell.http("POST", "/v1/scoped/confirm", Some(json!({"proposal_id":proposal.1["proposal_id"],"expected_revision":proposal.1["expected_revision"]})));
    assert_eq!(saved.0, 200);
    let memory = saved.1["memory_id"].as_str().unwrap().to_owned();
    let status = shell.http(
        "GET",
        &format!("/v1/scoped/status?person_id={person}&occasion=work"),
        None,
    );
    assert_eq!(status.0, 200);
    assert_eq!(status.1["state"], "active");
    let quote = shell
        .invoke("preview_forget", json!({"memoryId":memory}))
        .unwrap();
    let confirmation = json!({"preview_id":quote["preview_id"],"memory_id":memory});
    let denied = shell.http(
        "POST",
        &format!("/v1/memories/{memory}/forget-confirm"),
        Some(confirmation.clone()),
    );
    assert_eq!(denied.0, 403);
    assert_eq!(denied.1["code"], "caller_mismatch");
    let receipt = shell
        .invoke("forget_memory", json!({"confirmation":confirmation}))
        .unwrap();
    assert_eq!(receipt["logical_committed"], true);
    assert_eq!(
        shell
            .http("GET", &format!("/v1/memories/{memory}/state"), None)
            .1["forget_state"],
        "forgotten"
    );
    drop(shell);
    let reopened = soulcore::commands::session::Session::open(data);
    assert!(reopened.status().store_opened);
    assert!(reopened.memory(&memory).is_err());
}

#[test]
fn n3_http_and_production_ipc_preserve_caller_once_edit_forget_and_restart() {
    let root = tempfile::tempdir().unwrap();
    let (shell, data) = N3Shell::new(root.path());
    let (status, prepared) = shell.http(
        "POST",
        "/v1/imports/prepare",
        Some(json!({"fixture_id":"basic"})),
    );
    assert_eq!(status, 200, "HTTP must enter the shared import facade");
    let operation = prepared["operation"]["operation_id"].as_str().unwrap();
    let preview = prepared["operation"]["preview_id"].as_str().unwrap();
    let foreign = shell
        .invoke(
            "confirm_import",
            json!({"operationId":operation,"previewId":preview}),
        )
        .unwrap_err();
    assert_eq!(foreign["reason_code"], "RUNTIME_CALLER_MISMATCH");
    let (status, committed) = shell.http(
        "POST",
        &format!("/v1/imports/{operation}/confirm"),
        Some(json!({"preview_id":preview})),
    );
    assert_eq!(status, 200);
    assert_eq!(committed["state"], "committed");
    assert_eq!(
        shell
            .http("GET", &format!("/v1/imports/{operation}"), None)
            .1,
        committed
    );
    // Import creates contacts, while the user's voice choice creates the profile.
    shell
        .invoke("set_voice", json!({"field":"register","option":"formal"}))
        .unwrap();
    let recipients = shell
        .invoke("scoped_register_recipients", json!({}))
        .unwrap();
    let person = recipients[0]["person_id"].as_str().unwrap().to_owned();
    let draft = shell
        .invoke(
            "draft_scoped_reply",
            json!({"pasted":"SYNTHETIC_N3_PRIVATE_BODY","personId":person,"occasion":"personal"}),
        )
        .unwrap();
    let proposal = shell
        .invoke(
            "propose_scoped_register",
            json!({"resultId":draft["result_id"],"editRevision":1,"register":"casual"}),
        )
        .unwrap();
    let confirmation = json!({"proposal_id":proposal["proposal_id"],"expected_revision":proposal["expected_revision"]});
    let denied = shell.http("POST", "/v1/scoped/confirm", Some(confirmation));
    assert_eq!(denied.0, 403);
    assert_eq!(denied.1["code"], "caller_mismatch");
    let saved = shell.invoke("confirm_scoped_register", json!({"proposalId":proposal["proposal_id"],"expectedRevision":proposal["expected_revision"]})).unwrap();
    assert!(shell.invoke("confirm_scoped_register", json!({"proposalId":proposal["proposal_id"],"expectedRevision":proposal["expected_revision"]})).is_err());
    let memory = saved["memory_id"].as_str().unwrap().to_owned();
    let edit = shell.http(
        "PATCH",
        &format!("/v1/memories/{memory}"),
        Some(json!({"title":"SYNTHETIC_N3_EDIT_TITLE","summary":"SYNTHETIC_N3_EDIT_BODY"})),
    );
    assert_eq!(edit.0, 200);
    assert_eq!(
        shell
            .invoke(
                "scoped_register_status",
                json!({"personId":person,"occasion":"personal"})
            )
            .unwrap()["state"],
        "inactive"
    );
    let quote = shell.http(
        "POST",
        &format!("/v1/memories/{memory}/forget-preview"),
        Some(json!({})),
    );
    assert_eq!(quote.0, 200);
    let forget = json!({"preview_id":quote.1["preview_id"],"memory_id":memory});
    assert_eq!(
        shell
            .invoke("forget_memory", json!({"confirmation":forget}))
            .unwrap_err()["reason_code"],
        "RUNTIME_CALLER_MISMATCH"
    );
    let receipt = shell.http(
        "POST",
        &format!("/v1/memories/{memory}/forget-confirm"),
        Some(forget.clone()),
    );
    assert_eq!(receipt.0, 200);
    assert_eq!(receipt.1["logical_committed"], true);
    assert_ne!(
        shell
            .http(
                "POST",
                &format!("/v1/memories/{memory}/forget-confirm"),
                Some(forget)
            )
            .0,
        200
    );
    assert_eq!(
        shell
            .http("GET", &format!("/v1/memories/{memory}/state"), None)
            .1["forget_state"],
        "forgotten"
    );
    let audit = shell.invoke("audit_chain", json!({})).unwrap();
    assert_eq!(audit["verified"], true);
    for private in [
        "SYNTHETIC_N3_PRIVATE_BODY",
        "SYNTHETIC_N3_EDIT_BODY",
        "SYNTHETIC_N3_EDIT_TITLE",
    ] {
        assert!(!audit.to_string().contains(private));
    }
    let authority = shell.authority.clone();
    drop(shell);
    assert!(TcpStream::connect(authority).is_err());
    let reopened = soulcore::commands::session::Session::open(&data);
    assert!(reopened.status().store_opened);
    assert!(reopened.memory(&memory).is_err());
    let row = reopened
        .memories()
        .unwrap()
        .memories
        .into_iter()
        .find(|row| row.memory_id == memory)
        .unwrap();
    assert_eq!(row.forget_state, "forgotten");
    assert_eq!(
        reopened
            .scoped_register_status(
                &person,
                soulcore::commands::session::PreferenceOccasion::Personal
            )
            .unwrap()
            .state,
        soulcore::commands::session::ScopedRegisterStatusState::Forgotten
    );
}
