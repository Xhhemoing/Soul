//! Synthetic damaged evidence must refuse an import, without a partial graph.
use soul_store_api::import::{ImportOperationState, ImportSourceKind, LegacyImportChoice};
use soul_store_api::types::EventFilter;
use soul_store_api::{EventStore, GraphStore, ProfileStore};
use soulcore::commands::session::Session;

fn export(prefix: &str) -> String {
    let mut lines = vec![
        serde_json::json!({"type":"header", "format":"soul-import-v1",
        "version":1, "exported_at":"2024-03-02T00:00:00Z"})
        .to_string(),
    ];
    for (n, scope, sender) in [(0, "self", "owner"), (1, "third_party", "peer")] {
        lines.push(
            serde_json::json!({"type":"message", "id":format!("{prefix}-{n}"),
            "occurred_at":"2024-03-01T00:00:00Z", "sender_scope":scope,
            "sender_id":sender, "conversation_id":"synthetic-conversation",
            "text":"SECRET_SYNTHETIC_BODY"})
            .to_string(),
        );
    }
    lines.join("\n")
}

#[test]
fn damaged_time_rolls_back_persistent_import_but_records_rejection() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let work = session
        .begin_import_prepare(
            export("first"),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let first = session.complete_import_prepare(work.execute()).unwrap();
    let work = session
        .begin_import(
            &first.operation.operation_id.to_string(),
            &first.operation.preview_id.to_string(),
        )
        .unwrap();
    session.complete_import(work.execute()).unwrap();
    let work = session
        .begin_import_prepare(
            export("second"),
            ImportSourceKind::SoulImportV1,
            Some(first.operation.namespace_id),
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let next = session.complete_import_prepare(work.execute()).unwrap();
    let shared = session.store().unwrap();
    {
        let mut store = shared.lock().unwrap();
        let mut row = store
            .list_evidence()
            .unwrap()
            .into_iter()
            .find(soul_graph::interaction::is_interaction)
            .unwrap();
        row.source_refs[0]["occurred_at"] = serde_json::Value::Null;
        store.put_evidence(row).unwrap();
    }
    let snapshot = || {
        let store = shared.lock().unwrap();
        (
            store.list_events(&EventFilter::all()).unwrap(),
            store.list_contacts().unwrap(),
            store.list_evidence().unwrap(),
            store.list_relationships().unwrap(),
            store.list_inferences().unwrap(),
            store.blob_count().unwrap(),
            store.content_key_count().unwrap(),
            store.audit_links().unwrap(),
        )
    };
    let before = snapshot();
    let work = session
        .begin_import(
            &next.operation.operation_id.to_string(),
            &next.operation.preview_id.to_string(),
        )
        .unwrap();
    let refusal = session
        .complete_import(work.execute())
        .expect_err("damaged tagged source");
    assert!(!refusal.explanation.contains("SECRET"));
    assert_eq!(snapshot(), before);
    let recorded = session
        .import_operation(&next.operation.operation_id.to_string())
        .unwrap()
        .operation;
    assert_eq!(recorded.state, ImportOperationState::Rejected);
    assert!(recorded.receipt.is_none());
}

#[test]
fn damaged_existing_time_rolls_back_formal_session_import() {
    for value in [
        None,
        Some(serde_json::Value::Null),
        Some(serde_json::json!(4)),
        Some(serde_json::json!("SECRET_INVALID_TIME")),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::open(dir.path());
        session.commit_soul_import_v1(&export("before")).unwrap();
        let shared = session.store().unwrap();
        {
            let mut store = shared.lock().unwrap();
            let mut row = store
                .list_evidence()
                .unwrap()
                .into_iter()
                .find(soul_graph::interaction::is_interaction)
                .unwrap();
            match value {
                Some(value) => {
                    row.source_refs[0]["occurred_at"] = value;
                }
                None => {
                    row.source_refs[0]
                        .as_object_mut()
                        .unwrap()
                        .remove("occurred_at");
                }
            }
            store.put_evidence(row).unwrap();
        }
        let snapshot = || {
            let store = shared.lock().unwrap();
            (
                store.list_events(&EventFilter::all()).unwrap(),
                store.list_contacts().unwrap(),
                store.list_evidence().unwrap(),
                store.list_relationships().unwrap(),
                store.list_inferences().unwrap(),
                store.blob_count().unwrap(),
                store.content_key_count().unwrap(),
            )
        };
        let before = snapshot();
        let audit_before = session.audit().unwrap().entries.len();
        let refusal = session
            .commit_soul_import_v1(&export("after"))
            .expect_err("damaged tagged time");
        assert!(refusal.explanation.contains("回滚"));
        assert!(!refusal.explanation.contains("SECRET"));
        assert_eq!(
            snapshot(),
            before,
            "all durable domain rows must remain unchanged"
        );
        assert_eq!(session.audit().unwrap().entries.len(), audit_before);
    }
}
