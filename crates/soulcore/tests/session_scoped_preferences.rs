//! Synthetic SQLCipher sessions exercise actual next-task behavior and lifecycle.
use soul_schema::{
    common::SchemaVersion,
    contact::{ContactClass, SoulContact},
    memory::ForgetState,
};
use soul_store_api::{GraphStore, MemoryStore};
use soulcore::commands::{
    memory::MemoryChange,
    session::{PreferenceOccasion as Occasion, Session, VoiceRegister as Register},
};
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, Session, Uuid, Uuid) {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    session.set_voice("register", "formal").unwrap();
    let x = Uuid::now_v7();
    let y = Uuid::now_v7();
    {
        let shared = session.store().unwrap();
        let mut store = shared.lock().unwrap();
        for id in [x, y] {
            store
                .put_contact(SoulContact {
                    schema_version: SchemaVersion,
                    contact_id: id,
                    contact_class: ContactClass::ThirdParty,
                    display_label_ref: None,
                    identifiers: None,
                    forget_state: ForgetState::Active,
                })
                .unwrap();
        }
    }
    (dir, session, x, y)
}

fn save(
    session: &mut Session,
    x: Uuid,
    register: Register,
) -> soulcore::commands::session::ScopedRegisterReceipt {
    let result = session
        .draft_scoped_reply(
            "synthetic private paste",
            &x.to_string(),
            Occasion::Personal,
        )
        .unwrap();
    let proposal = session
        .propose_scoped_register(&result.result_id.to_string(), 1, register)
        .unwrap();
    assert!(proposal.notice.contains("更改或遗忘"));
    session
        .confirm_scoped_register(
            &proposal.proposal_id.to_string(),
            proposal.expected_revision,
        )
        .unwrap()
}

#[test]
fn scoped_session_exact_next_task_and_restart_keep_global_unchanged() {
    let (dir, mut session, x, y) = fixture();
    let before = session.profile().unwrap();
    for (register, greeting) in [
        (Register::Casual, "嗨"),
        (Register::Plain, "你好"),
        (Register::Formal, "您好"),
    ] {
        let receipt = save(&mut session, x, register);
        let result = session
            .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
            .unwrap();
        assert!(
            result.draft.text.starts_with(greeting),
            "{}",
            result.draft.text
        );
        assert_eq!(
            result.applied_preference.unwrap().memory_id,
            receipt.memory_id
        );
        for (person, occasion) in [(y, Occasion::Personal), (x, Occasion::Work)] {
            let other = session
                .draft_scoped_reply("synthetic", &person.to_string(), occasion)
                .unwrap();
            assert!(other.applied_preference.is_none());
            assert!(other.draft.text.starts_with("您好"));
        }
        assert_eq!(session.profile().unwrap(), before);
    }
    save(&mut session, x, Register::Casual);
    assert!(session
        .draft_pasted("synthetic")
        .unwrap()
        .text
        .starts_with("您好"));
    drop(session);
    let mut session = Session::open(dir.path());
    assert!(session
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap()
        .draft
        .text
        .starts_with("嗨"));
}

#[test]
fn scoped_session_cancel_replacement_stale_revision_and_replay_do_not_write() {
    let (_dir, mut session, x, _) = fixture();
    let result = session
        .draft_scoped_reply(
            "private body never saved",
            &x.to_string(),
            Occasion::Personal,
        )
        .unwrap();
    let proposal = session
        .propose_scoped_register(&result.result_id.to_string(), 2, Register::Casual)
        .unwrap();
    assert!(session
        .discard_scoped_register_proposal(&proposal.proposal_id.to_string())
        .unwrap());
    assert!(session
        .confirm_scoped_register(&proposal.proposal_id.to_string(), None)
        .is_err());
    assert!(session.memories().unwrap().memories.is_empty());
    let proposal = session
        .propose_scoped_register(&result.result_id.to_string(), 3, Register::Casual)
        .unwrap();
    session.draft_pasted("another task").unwrap();
    assert!(session
        .confirm_scoped_register(&proposal.proposal_id.to_string(), None)
        .is_err());
    let receipt = save(&mut session, x, Register::Casual);
    assert!(session
        .confirm_scoped_register(&proposal.proposal_id.to_string(), None)
        .is_err());
    let result = session
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap();
    let proposal = session
        .propose_scoped_register(&result.result_id.to_string(), 4, Register::Plain)
        .unwrap();
    assert!(session
        .confirm_scoped_register(&proposal.proposal_id.to_string(), Some(Uuid::now_v7()))
        .is_err());
    assert_eq!(session.memories().unwrap().memories.len(), 1);
    let memory = session.memory(&receipt.memory_id.to_string()).unwrap();
    assert!(!memory.summary.contains("private body"));
    let audit = serde_json::to_string(&session.audit().unwrap()).unwrap();
    assert!(!audit.contains("private body"));
}

#[test]
fn scoped_session_ordinary_edit_and_forget_revoke_old_result() {
    let (_dir, mut session, x, _) = fixture();
    let saved = save(&mut session, x, Register::Casual);
    let result = session
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap();
    let proposal = session
        .propose_scoped_register(&result.result_id.to_string(), 1, Register::Plain)
        .unwrap();
    session
        .edit_memory(
            &saved.memory_id.to_string(),
            &MemoryChange {
                summary: Some("ordinary edit".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(session
        .confirm_scoped_register(
            &proposal.proposal_id.to_string(),
            proposal.expected_revision
        )
        .is_err());
    assert!(session
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap()
        .applied_preference
        .is_none());
    let replaced = save(&mut session, x, Register::Casual);
    assert_eq!(saved.memory_id, replaced.memory_id);
    let result = session
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap();
    let shared = session.store().unwrap();
    {
        use soul_store_api::forget::{ForgetOps, ForgetUnit};
        let mut store = shared.lock().unwrap();
        store
            .execute_forget(ForgetUnit::Memory(saved.memory_id))
            .unwrap();
        assert_eq!(
            store.get_memory(saved.memory_id).unwrap().forget_state,
            ForgetState::Forgotten
        );
    }
    assert!(session
        .propose_scoped_register(&result.result_id.to_string(), 1, Register::Plain)
        .is_err());
    assert!(session
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap()
        .applied_preference
        .is_none());
    let fresh = save(&mut session, x, Register::Casual);
    assert_ne!(fresh.memory_id, saved.memory_id);
}

#[test]
fn scoped_session_missing_profile_invalid_person_and_foreign_result_refuse() {
    let (dir, mut session, x, _) = fixture();
    let first = session
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap();
    let next = session
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap();
    assert!(session
        .propose_scoped_register(&first.result_id.to_string(), 1, Register::Casual)
        .is_err());
    let proposal = session
        .propose_scoped_register(&next.result_id.to_string(), 1, Register::Casual)
        .unwrap();
    session.prepare_draft("synthetic", None).ok();
    assert!(session
        .confirm_scoped_register(&proposal.proposal_id.to_string(), None)
        .is_err());
    for class in [
        ContactClass::Owner,
        ContactClass::Group,
        ContactClass::Service,
    ] {
        let shared = session.store().unwrap();
        let mut store = shared.lock().unwrap();
        let mut person = store.get_contact(x).unwrap();
        person.contact_class = class;
        store.put_contact(person).unwrap();
        drop(store);
        assert!(session
            .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
            .is_err());
        assert!(!session
            .scoped_register_recipients()
            .unwrap()
            .iter()
            .any(|row| row.person_id == x));
    }
    drop(session);
    let mut reopened = Session::open(dir.path());
    assert!(reopened
        .confirm_scoped_register(&proposal.proposal_id.to_string(), None)
        .is_err());
    let empty = tempfile::tempdir().unwrap();
    let mut blank = Session::open(empty.path());
    assert!(blank
        .draft_scoped_reply("synthetic", &x.to_string(), Occasion::Personal)
        .unwrap_err()
        .explanation
        .contains("档案"));
}

#[test]
fn scoped_session_configured_endpoint_still_has_zero_requests() {
    let (_dir, mut session, x, _) = fixture();
    let endpoint = soul_testkit::mock_llm::MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    save(&mut session, x, Register::Casual);
    let result = session
        .draft_scoped_reply(
            "synthetic private paste",
            &x.to_string(),
            Occasion::Personal,
        )
        .unwrap();
    assert_eq!(
        result.draft.source,
        soulcore::commands::draft::DraftSource::ToneTemplate
    );
    assert_eq!(endpoint.request_count(), 0);
}

fn status_json(session: &Session, person: Uuid, occasion: Occasion) -> serde_json::Value {
    serde_json::to_value(
        session
            .scoped_register_status(&person.to_string(), occasion)
            .unwrap(),
    )
    .unwrap()
}

fn fault_connection(directory: &std::path::Path) -> rusqlite::Connection {
    use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
    use soulcore::commands::session::KEY_BLOB_FILE_NAME;
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(directory.join(KEY_BLOB_FILE_NAME)))
    } else {
        Box::new(TestKeyProvider::in_dir(directory))
    };
    let conn =
        rusqlite::Connection::open(soulcore::commands::store::database_path(directory)).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    conn
}

#[test]
fn scoped_status_exact_current_state_and_plain_edit_inactivation() {
    let (_dir, mut session, x, y) = fixture();
    let absent = status_json(&session, x, Occasion::Personal);
    assert_eq!(absent["state"], "absent");
    assert!(absent["preference"].is_null());
    assert!(absent["memory_id"].is_null());
    let saved = save(&mut session, x, Register::Casual);
    let active = status_json(&session, x, Occasion::Personal);
    assert_eq!(active["state"], "active");
    assert_eq!(active["person_id"], x.to_string());
    assert_eq!(active["occasion"], "personal");
    assert_eq!(active["preference"]["register"], "casual");
    assert_eq!(active["preference"]["revision"], saved.revision.to_string());
    assert_eq!(active["memory_id"], saved.memory_id.to_string());
    for (person, occasion) in [(y, Occasion::Personal), (x, Occasion::Work)] {
        assert_eq!(status_json(&session, person, occasion)["state"], "absent");
    }
    session
        .edit_memory(
            &saved.memory_id.to_string(),
            &MemoryChange {
                summary: Some("synthetic edited confirmation is not the current rule".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let inactive = status_json(&session, x, Occasion::Personal);
    assert_eq!(inactive["state"], "inactive");
    assert!(inactive["preference"].is_null());
    assert_eq!(inactive["memory_id"], saved.memory_id.to_string());
    assert!(!inactive
        .to_string()
        .contains("synthetic edited confirmation"));
}

#[test]
fn scoped_status_memory_and_key_forget_have_proof_and_survive_restart() {
    use soul_store_api::forget::{ForgetOps, ForgetUnit};
    for by_key in [false, true] {
        let (dir, mut session, x, _) = fixture();
        let saved = save(&mut session, x, Register::Casual);
        {
            let shared = session.store().unwrap();
            let mut store = shared.lock().unwrap();
            let memory = store.get_memory(saved.memory_id).unwrap();
            let unit = if by_key {
                ForgetUnit::ContentKey(memory.content_key_id)
            } else {
                ForgetUnit::Memory(saved.memory_id)
            };
            store.execute_forget(unit).unwrap();
        }
        let forgotten = status_json(&session, x, Occasion::Personal);
        assert_eq!(forgotten["state"], "forgotten");
        assert!(forgotten["preference"].is_null());
        assert_eq!(forgotten["memory_id"], saved.memory_id.to_string());
        drop(session);
        let reopened = Session::open(dir.path());
        assert_eq!(
            status_json(&reopened, x, Occasion::Personal)["state"],
            "forgotten"
        );
    }
}

#[test]
fn scoped_status_active_broken_blob_is_unavailable_and_sql_failure_is_error() {
    let (dir, mut session, x, _) = fixture();
    let saved = save(&mut session, x, Register::Casual);
    let conn = fault_connection(dir.path());
    conn.execute(
        "DELETE FROM sealed_blobs WHERE row_id=?1",
        [saved.memory_id.to_string()],
    )
    .unwrap();
    let damaged = status_json(&session, x, Occasion::Personal);
    assert_eq!(damaged["state"], "unavailable");
    assert!(damaged["preference"].is_null());
    conn.execute_batch(
        "ALTER TABLE scoped_register_preferences RENAME TO synthetic_missing_scoped_register;",
    )
    .unwrap();
    let error = session
        .scoped_register_status(&x.to_string(), Occasion::Personal)
        .unwrap_err();
    assert!(!error.explanation.contains("synthetic_missing"));
    assert!(!error.explanation.contains("SELECT"));
    assert!(!error.explanation.contains("no such table"));
}

#[test]
fn scoped_status_reads_and_refusals_do_not_consume_proposal_or_write_audit() {
    let (dir, mut session, x, y) = fixture();
    let saved = save(&mut session, x, Register::Casual);
    let result = session
        .draft_scoped_reply("synthetic current task", &x.to_string(), Occasion::Personal)
        .unwrap();
    let proposal = session
        .propose_scoped_register(&result.result_id.to_string(), 12, Register::Plain)
        .unwrap();
    let before_audit = serde_json::to_value(session.audit().unwrap()).unwrap();
    assert_eq!(
        status_json(&session, x, Occasion::Personal)["state"],
        "active"
    );
    assert_eq!(status_json(&session, y, Occasion::Work)["state"], "absent");
    assert!(session
        .scoped_register_status("invalid uuid", Occasion::Personal)
        .is_err());
    assert!(session
        .scoped_register_status(&Uuid::now_v7().to_string(), Occasion::Personal)
        .is_err());
    let conn = fault_connection(dir.path());
    conn.execute_batch(
        "ALTER TABLE scoped_register_preferences RENAME TO synthetic_hidden_scoped_register;",
    )
    .unwrap();
    assert!(session
        .scoped_register_status(&x.to_string(), Occasion::Personal)
        .is_err());
    conn.execute_batch(
        "ALTER TABLE synthetic_hidden_scoped_register RENAME TO scoped_register_preferences;",
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(session.audit().unwrap()).unwrap(),
        before_audit
    );
    let after = session
        .confirm_scoped_register(
            &proposal.proposal_id.to_string(),
            proposal.expected_revision,
        )
        .unwrap();
    assert_eq!(after.memory_id, saved.memory_id);
    assert_eq!(after.register, Register::Plain);
}

#[test]
fn scoped_status_missing_owner_and_forgotten_contact_are_refused() {
    use soul_store_api::forget::{ForgetOps, ForgetUnit};
    let empty = tempfile::tempdir().unwrap();
    let blank = Session::open(empty.path());
    assert!(blank
        .scoped_register_status(&Uuid::now_v7().to_string(), Occasion::Personal)
        .is_err());
    let (_dir, mut session, x, _) = fixture();
    save(&mut session, x, Register::Casual);
    {
        let shared = session.store().unwrap();
        shared
            .lock()
            .unwrap()
            .execute_forget(ForgetUnit::Contact(x))
            .unwrap();
    }
    assert!(session
        .scoped_register_status(&x.to_string(), Occasion::Personal)
        .is_err());
}

#[test]
fn scoped_status_inconsistent_tombstone_missing_key_and_key_query_failure_are_not_forget() {
    let (_dir, mut session, x, _) = fixture();
    let saved = save(&mut session, x, Register::Casual);
    {
        let shared = session.store().unwrap();
        let mut store = shared.lock().unwrap();
        let mut memory = store.get_memory(saved.memory_id).unwrap();
        memory.forget_state = ForgetState::Forgotten;
        store.put_memory(memory).unwrap();
    }
    // A flag without the irreversible key-destruction ledger is not proof.
    assert_eq!(
        status_json(&session, x, Occasion::Personal)["state"],
        "unavailable"
    );

    let (dir, mut session, x, _) = fixture();
    let saved = save(&mut session, x, Register::Casual);
    let conn = fault_connection(dir.path());
    conn.execute("DELETE FROM content_keys WHERE content_key_id=(SELECT content_key_id FROM memories WHERE memory_id=?1)", [saved.memory_id.to_string()]).unwrap();
    assert_eq!(
        status_json(&session, x, Occasion::Personal)["state"],
        "unavailable"
    );
    conn.execute_batch("ALTER TABLE content_keys RENAME TO synthetic_missing_content_keys;")
        .unwrap();
    assert!(session
        .scoped_register_status(&x.to_string(), Occasion::Personal)
        .is_err());
}
