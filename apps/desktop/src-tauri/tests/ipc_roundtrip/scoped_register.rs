//! Explicit local scoped preference through the shipped command registry.
use super::*;

#[test]
fn scoped_register_wire_save_next_task_cancel_forget_and_restart() {
    let shell = Shell::on(scratch());
    shell
        .import(
            "soul-import-v1",
            &fixture("import/soul-import-v1/valid_basic.jsonl"),
        )
        .unwrap();
    shell
        .invoke(
            "set_voice",
            json!({"field": "register", "option": "formal"}),
        )
        .unwrap();
    let people = shell
        .invoke("scoped_register_recipients", json!({}))
        .unwrap();
    let person = &people[0]["person_id"];
    assert!(person.is_string());
    let request = json!({"pasted": "SYNTHETIC_SCOPED_PRIVATE_BODY", "personId": person, "occasion": "personal"});
    let first = shell.invoke("draft_scoped_reply", request.clone()).unwrap();
    assert!(first["draft"]["text"].as_str().unwrap().starts_with("您好"));
    let proposal = shell
        .invoke(
            "propose_scoped_register",
            json!({"resultId": first["result_id"], "editRevision": 2, "register": "casual"}),
        )
        .unwrap();
    let confirm = json!({"proposalId": proposal["proposal_id"], "expectedRevision": proposal["expected_revision"]});
    let saved = shell
        .invoke("confirm_scoped_register", confirm.clone())
        .unwrap();
    assert!(shell.invoke("confirm_scoped_register", confirm).is_err());
    let next = shell.invoke("draft_scoped_reply", request.clone()).unwrap();
    assert_eq!(next["applied_preference"]["memory_id"], saved["memory_id"]);
    assert!(next["draft"]["text"].as_str().unwrap().starts_with("嗨"));
    assert_eq!(next["draft"]["source"], "tone_template");
    assert_eq!(next["draft"]["delivery"], false);
    let proposal = shell
        .invoke(
            "propose_scoped_register",
            json!({"resultId": next["result_id"], "editRevision": 3, "register": "formal"}),
        )
        .unwrap();
    assert_eq!(
        shell
            .invoke(
                "discard_scoped_register_proposal",
                json!({"proposalId": proposal["proposal_id"]})
            )
            .unwrap(),
        true
    );
    assert!(shell.invoke("confirm_scoped_register", json!({"proposalId": proposal["proposal_id"], "expectedRevision": proposal["expected_revision"]})).is_err());
    let shell = shell.restart();
    assert!(
        shell.invoke("draft_scoped_reply", request.clone()).unwrap()["applied_preference"]
            .is_object()
    );
    let quote = shell
        .invoke("preview_forget", json!({"memoryId": saved["memory_id"]}))
        .unwrap();
    shell.invoke("forget_memory", json!({"confirmation": {"preview_id": quote["preview_id"], "memory_id": saved["memory_id"]}})).unwrap();
    let shell = shell.restart();
    assert!(shell.invoke("draft_scoped_reply", request).unwrap()["applied_preference"].is_null());
}

#[test]
fn scoped_register_wire_requires_explicit_valid_values_and_current_result() {
    let shell = Shell::on(scratch());
    for body in [
        json!({}),
        json!({"pasted": "synthetic", "personId": "bad", "occasion": "unknown"}),
        json!({"pasted": "synthetic", "occasion": "personal"}),
    ] {
        assert!(shell.invoke("draft_scoped_reply", body).is_err());
    }
    assert!(shell
        .invoke(
            "propose_scoped_register",
            json!({"resultId": "not-a-result", "editRevision": 0, "register": "casual"})
        )
        .is_err());
    assert!(shell
        .invoke(
            "propose_scoped_register",
            json!({"resultId": "not-a-result", "editRevision": 0, "register": "unknown"})
        )
        .is_err());
}

#[test]
fn scoped_register_status_wire_is_read_only_and_tracks_edit_and_forget() {
    let shell = Shell::on(scratch());
    shell
        .import(
            "soul-import-v1",
            &fixture("import/soul-import-v1/valid_basic.jsonl"),
        )
        .unwrap();
    shell
        .invoke(
            "set_voice",
            json!({"field": "register", "option": "formal"}),
        )
        .unwrap();
    let people = shell
        .invoke("scoped_register_recipients", json!({}))
        .unwrap();
    let person = &people[0]["person_id"];
    let scope = json!({"personId": person, "occasion": "personal"});
    let status = shell
        .invoke("scoped_register_status", scope.clone())
        .unwrap();
    assert_eq!(status["state"], "absent");
    assert!(status["preference"].is_null());
    let draft = shell
        .invoke(
            "draft_scoped_reply",
            json!({"pasted": "SYNTHETIC_STATUS_BODY", "personId": person, "occasion": "personal"}),
        )
        .unwrap();
    let proposal = shell
        .invoke(
            "propose_scoped_register",
            json!({"resultId": draft["result_id"], "editRevision": 0, "register": "casual"}),
        )
        .unwrap();
    let audit = shell.invoke("audit_chain", json!({})).unwrap();
    shell
        .invoke("scoped_register_status", scope.clone())
        .unwrap();
    assert_eq!(shell.invoke("audit_chain", json!({})).unwrap(), audit);
    let saved = shell
        .invoke(
            "confirm_scoped_register",
            json!({"proposalId": proposal["proposal_id"], "expectedRevision": null}),
        )
        .unwrap();
    let status = shell
        .invoke("scoped_register_status", scope.clone())
        .unwrap();
    assert_eq!(status["state"], "active");
    assert_eq!(status["preference"]["register"], "casual");
    assert_eq!(status["memory_id"], saved["memory_id"]);
    assert!(!status.to_string().contains("SYNTHETIC_STATUS_BODY"));
    shell.invoke("update_memory", json!({"memoryId": saved["memory_id"], "change": {"summary": "SYNTHETIC_ORDINARY_EDIT"}})).unwrap();
    let status = shell
        .invoke("scoped_register_status", scope.clone())
        .unwrap();
    assert_eq!(status["state"], "inactive");
    assert!(status["preference"].is_null());
    let preview = shell
        .invoke("preview_forget", json!({"memoryId": saved["memory_id"]}))
        .unwrap();
    shell.invoke("forget_memory", json!({"confirmation": {"memory_id": saved["memory_id"], "preview_id": preview["preview_id"]}})).unwrap();
    let status = shell
        .invoke("scoped_register_status", scope.clone())
        .unwrap();
    assert_eq!(status["state"], "forgotten");
    assert!(status["preference"].is_null());
    assert_eq!(
        shell
            .invoke(
                "scoped_register_status",
                json!({"personId": person, "occasion": "work"})
            )
            .unwrap()["state"],
        "absent"
    );
    assert!(shell
        .invoke(
            "scoped_register_status",
            json!({"personId": person, "occasion": "unknown"})
        )
        .is_err());
}
