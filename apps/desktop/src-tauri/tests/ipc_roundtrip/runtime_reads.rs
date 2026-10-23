//! The first local-API reads use the same Session as the shipped IPC writes.

use super::*;

#[test]
fn runtime_reads_observe_ipc_writes_and_hide_storage_internals() {
    let directory = tempfile::tempdir().unwrap();
    let shell = Shell::on(directory.path());
    let status = shell.invoke("runtime_status", json!({})).unwrap();
    let runtime = shell
        .webview
        .state::<soul_desktop::commands::SessionState>()
        .runtime();
    assert_eq!(status, serde_json::to_value(runtime.status()).unwrap());
    assert_eq!(status["store_opened"], true);
    assert_eq!(status["wizard_completed"], false);
    assert_eq!(status["config_problem"], false);
    assert!(status.get("store_notice").is_none());

    let written = shell
        .invoke(
            "create_memory",
            json!({"memory": {
                "memory_type": "episodic", "title": "Synthetic runtime memory",
                "summary": "Synthetic exact-read content."
            }}),
        )
        .unwrap();
    let read = shell
        .invoke(
            "runtime_memory",
            json!({"request": {"memory_id": written["memory_id"]}}),
        )
        .unwrap();
    assert_eq!(read["memory_id"], written["memory_id"]);
    assert_eq!(read["title"], "Synthetic runtime memory");
    assert_eq!(read["summary"], "Synthetic exact-read content.");
    assert_eq!(read["third_party_content_present"], false);
    assert!(read.get("content_key_id").is_none());
    assert_eq!(
        read,
        serde_json::to_value(
            runtime
                .memory(soul_types::MemoryIdRequest {
                    memory_id: written["memory_id"].as_str().unwrap().into(),
                })
                .unwrap()
        )
        .unwrap()
    );

    let page = shell
        .invoke("runtime_people", json!({"request": {}}))
        .unwrap();
    assert_eq!(page["limit"], 50);
    assert_eq!(page["offset"], 0);
    assert_eq!(page["people"], json!([]));
    assert_eq!(page["total"], 0);
    assert_eq!(page["has_more"], false);
    assert!(page.get("ties").is_none());
    assert!(page.get("evidence").is_none());
    assert_eq!(
        page,
        serde_json::to_value(
            runtime
                .people(soul_types::PeoplePageRequest::default())
                .unwrap()
        )
        .unwrap()
    );
}

#[test]
fn synthetic_mode_refuses_real_data_capabilities_before_side_effects() {
    let directory = tempfile::tempdir().unwrap();
    let session = Session::open(directory.path());
    assert!(session.status().store_opened);
    let runtime = soul_runtime::Runtime::new_synthetic(session);
    let state = soul_desktop::commands::SessionState::from_runtime(runtime.clone());
    let app = soul_desktop::configure(mock_builder(), state)
        .build(tauri::generate_context!())
        .unwrap();
    let webview = WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let shell = Shell {
        directory: directory.path().to_owned(),
        url: app_url(&app),
        webview,
    };
    let not_read = directory.path().join("SYNTHETIC_NOT_AUTHORIZED");
    let not_written = directory.path().join("SYNTHETIC_NOT_WRITTEN.zip");
    let requests = [
        ("grant_collect_consent", json!({})),
        ("set_user_endpoint", json!({"url": "http://127.0.0.1:1"})),
        ("authorize_directory", json!({"path": not_read})),
        ("preview_plan", json!({"path": not_read})),
        (
            "prepare_import",
            json!({"request": {"text": "SYNTHETIC_NOT_IMPORTED", "source": "soul-import-v1", "namespace_id": null, "legacy_choice": "reject"}}),
        ),
        (
            "preview_soul_import_v1",
            json!({"text": "SYNTHETIC_NOT_IMPORTED"}),
        ),
        (
            "preview_telegram",
            json!({"text": "SYNTHETIC_NOT_IMPORTED"}),
        ),
        (
            "commit_soul_import_v1",
            json!({"text": "SYNTHETIC_NOT_IMPORTED"}),
        ),
        ("commit_telegram", json!({"text": "SYNTHETIC_NOT_IMPORTED"})),
        (
            "prepare_research_save",
            json!({"request": {"scope": {"from_day": "2026-01-01", "to_day": "2026-12-31"}, "destination": not_written}}),
        ),
        ("confirm_research_save", json!({"requestId": "synthetic"})),
        ("person_summary", json!({"contactId": "synthetic"})),
        (
            "generate_draft",
            json!({"approval": {"preparation_id": "synthetic", "plan_hash": "synthetic"}}),
        ),
    ];
    for (command, request) in requests {
        let error = shell.invoke(command, request).unwrap_err();
        assert_eq!(
            error["reason_code"], "SYNTHETIC_MODE_RESTRICTED",
            "{command}: {error}"
        );
    }
    // N3 accepts confirmation of a built-in fixture prepared by this caller;
    // an unknown approval still cannot create an import or grant raw-text input.
    let absent = shell
        .invoke(
            "confirm_import",
            json!({"operationId": "018f00a0-0000-7000-8000-000000000001", "previewId": "018f00a0-0000-7000-8000-000000000002"}),
        )
        .unwrap_err();
    assert_eq!(absent["reason_code"], "RUNTIME_CONFIRMATION_UNAVAILABLE");
    assert!(!not_written.exists());
    assert!(!not_read.exists());
    assert!(!runtime.held_session().collect_status().consent_granted);
    assert!(!runtime.held_session().collect_status().collector_running);
    assert!(runtime.held_session().snapshot().fully_closed);
    assert!(runtime
        .held_session()
        .memories()
        .unwrap()
        .memories
        .is_empty());
    assert!(runtime
        .held_session()
        .recent_import_operations()
        .unwrap()
        .is_empty());
    assert!(shell.invoke("clear_user_endpoint", json!({})).is_ok());
    assert!(shell.invoke("revoke_collect_consent", json!({})).is_ok());

    let written = shell.invoke("create_memory", json!({"memory": {
        "memory_type": "episodic", "title": "Synthetic permitted mutation", "summary": "Synthetic content only."
    }})).unwrap();
    let read = runtime
        .memory(soul_types::MemoryIdRequest {
            memory_id: written["memory_id"].as_str().unwrap().into(),
        })
        .unwrap();
    assert_eq!(read.title, "Synthetic permitted mutation");
}

#[test]
fn runtime_read_refusals_are_bounded_and_do_not_echo_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let shell = Shell::on(directory.path());
    for request in [
        json!({"limit": 0}),
        json!({"limit": 101}),
        json!({"offset": 1_000_001}),
    ] {
        let error = shell
            .invoke("runtime_people", json!({"request": request}))
            .unwrap_err();
        assert_eq!(error["code"], "invalid_request");
        assert!(error["message"].is_string());
    }
    let error = shell
        .invoke(
            "runtime_memory",
            json!({"request": {"memory_id": "C:/SYNTHETIC_PRIVATE_PATH/not-an-id"}}),
        )
        .unwrap_err();
    assert_eq!(error["code"], "invalid_request");
    assert!(!error.to_string().contains("SYNTHETIC_PRIVATE_PATH"));
    let unknown = shell
        .invoke(
            "runtime_memory",
            json!({"request": {"memory_id": "018f00a0-0000-7000-8000-000000000001"}}),
        )
        .unwrap_err();
    assert_eq!(unknown["code"], "read_failed");
    assert!(unknown.get("explanation").is_none());
    assert!(!unknown
        .to_string()
        .contains(&directory.path().to_string_lossy().to_string()));
}
