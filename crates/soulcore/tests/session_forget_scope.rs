//! Product-path revalidation happens before the irreversible store operation.
use soul_store_api::MemoryStore;
use soulcore::commands::memory::{ForgetConfirmation, MemoryChange, NewMemory};
use soulcore::commands::session::Session;
use uuid::Uuid;

fn write(session: &mut Session) -> String {
    session
        .write_memory(&NewMemory {
            memory_type: "episodic".into(),
            title: "synthetic scope test".into(),
            summary: "synthetic private prose".into(),
        })
        .expect("write")
        .memory_id
}

#[test]
fn changed_scope_refuses_before_destroying_any_key_and_fresh_preview_can_commit() {
    let dir = tempfile::tempdir().expect("synthetic session");
    let mut session = Session::open(dir.path());
    let memory_id = write(&mut session);
    let quoted = session.preview_forget(&memory_id).expect("preview");
    let confirmation = ForgetConfirmation {
        preview_id: quoted.preview_id,
        memory_id: memory_id.clone(),
    };
    let dependent_id = Uuid::now_v7();
    {
        let shared = session.store().expect("one store");
        let mut store = shared.lock().expect("store guard");
        let mut dependent = store
            .get_memory(memory_id.parse().expect("UUID"))
            .expect("row");
        dependent.memory_id = dependent_id;
        store
            .put_memory(dependent)
            .expect("scope grew after preview");
    }

    for _ in 0..2 {
        let refusal = session
            .forget_memory(&confirmation)
            .expect_err("stale scope");
        assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
        assert!(
            session.memory(&memory_id).is_ok(),
            "original key must survive"
        );
        assert!(
            session.memory(&dependent_id.to_string()).is_ok(),
            "dependent must survive"
        );
    }
    let audit = session
        .audit()
        .expect("refusal must not deadlock the store mutex");
    assert!(audit.verified);
    assert_eq!(
        audit
            .entries
            .iter()
            .filter(|e| e.action == "hitl.deny")
            .count(),
        2
    );
    assert!(!audit.entries.iter().any(|e| e.action == "forget.execute"));
    let encoded = serde_json::to_string(&audit).expect("audit JSON");
    assert!(!encoded.contains("synthetic private prose"));

    let fresh = session.preview_forget(&memory_id).expect("fresh scope");
    assert_eq!(fresh.memories_affected, 2);
    let receipt = session
        .forget_memory(&ForgetConfirmation {
            preview_id: fresh.preview_id,
            memory_id: memory_id.clone(),
        })
        .expect("confirmed current scope");
    assert!(receipt.logical_committed);
    assert!(receipt.matched_preview);
    assert!(session.memory(&memory_id).is_err());
    assert!(session.memory(&dependent_id.to_string()).is_err());
    drop(session);
    let reopened = Session::open(dir.path());
    assert!(
        reopened.memory(&memory_id).is_err(),
        "destruction survives restart"
    );
}

#[test]
fn editing_after_preview_requires_a_new_quote_and_preserves_the_edited_content() {
    let dir = tempfile::tempdir().expect("synthetic session");
    let mut session = Session::open(dir.path());
    let memory_id = write(&mut session);
    let quoted = session.preview_forget(&memory_id).expect("preview");
    session
        .edit_memory(
            &memory_id,
            &MemoryChange {
                summary: Some("a newer synthetic memory".into()),
                ..MemoryChange::default()
            },
        )
        .expect("edit after preview");
    let refusal = session
        .forget_memory(&ForgetConfirmation {
            preview_id: quoted.preview_id,
            memory_id: memory_id.clone(),
        })
        .expect_err("new content was not quoted");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    assert_eq!(
        session.memory(&memory_id).expect("still readable").summary,
        "a newer synthetic memory"
    );
}
