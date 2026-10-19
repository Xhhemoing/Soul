//! Synthetic real Session/SQLCipher regressions for assistance identity and CAS.
use soul_schema::memory::MemoryType;
use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
use soul_store_api::forget::ForgetOutcomeOps;
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_store_api::{ForgetUnit, MemoryStore};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::memory::{ForgetConfirmation, MemoryChange, NewMemory};
use soulcore::commands::session::*;
use soulcore::commands::store::database_path;
use uuid::Uuid;

fn setup() -> (tempfile::TempDir, Session) {
    let dir = tempfile::tempdir().unwrap();
    let session = Session::open(dir.path());
    assert!(session.status().store_opened);
    (dir, session)
}
fn goal(session: &mut Session, source: Option<AssistanceItemRequest>) -> AssistanceItemDetail {
    session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "Synthetic explicit commitment".into(),
            summary: "Finish the synthetic draft and report completion".into(),
            deadline_at: Some("2026-11-20T16:00:00+08:00".into()),
            source_observation: source,
        })
        .unwrap()
}
fn req(item: &AssistanceItemRow) -> AssistanceItemRequest {
    AssistanceItemRequest {
        item_id: item.item_id,
        expected_revision: item.revision,
    }
}
fn memory_id(item: &AssistanceItemRow) -> Uuid {
    match item.source {
        AssistanceSource::ManualGoal { memory_id, .. } => memory_id,
        _ => panic!("goal"),
    }
}
fn observe(session: &mut Session) -> AssistanceItemDetail {
    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap();
    let work = session
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let prepared = session.complete_import_prepare(work.execute()).unwrap();
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    session.complete_import(work.execute()).unwrap();
    let bundle = session.local_insights().unwrap().remove(0);
    session
        .link_assistance_observation(LinkAssistanceObservation {
            bundle_id: bundle.bundle_id,
            contact_id: Uuid::parse_str(&bundle.summaries[0].contact_id).unwrap(),
        })
        .unwrap()
}
fn raw(dir: &std::path::Path) -> rusqlite::Connection {
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(dir.join(KEY_BLOB_FILE_NAME)))
    } else {
        Box::new(TestKeyProvider::in_dir(dir))
    };
    let conn = rusqlite::Connection::open(database_path(dir)).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    conn
}

#[test]
fn goal_and_memory_creation_roll_back_together_and_survive_reopen_without_prose_in_list() {
    let (dir, mut session) = setup();
    let conn = raw(dir.path());
    conn.execute_batch("CREATE TRIGGER fail_goal_insert BEFORE INSERT ON assistance_items BEGIN SELECT RAISE(ABORT,'synthetic item failure'); END;").unwrap();
    assert!(session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "synthetic".into(),
            summary: "synthetic detail".into(),
            deadline_at: None,
            source_observation: None
        })
        .is_err());
    for table in [
        "memories",
        "content_keys",
        "sealed_blobs",
        "assistance_items",
    ] {
        let count: i64 = conn
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    conn.execute_batch("DROP TRIGGER fail_goal_insert").unwrap();
    let item = goal(&mut session, None);
    let id = item.item.item_id.to_string();
    let revision = item.item.revision;
    let list = serde_json::to_string(&session.assistance_items().unwrap()).unwrap();
    assert!(!list.contains("Synthetic explicit"));
    assert!(!list.contains("report completion"));
    drop(conn);
    drop(session);
    let reopened = Session::open(dir.path());
    let item = reopened.assistance_item(&id).unwrap();
    assert_eq!(item.item.revision, revision);
    assert_eq!(item.goal.unwrap().title, "Synthetic explicit commitment");
}

#[test]
fn schema_three_gains_empty_assistance_metadata_without_changing_existing_memory() {
    let (dir, mut session) = setup();
    let memory = session
        .write_memory(&NewMemory {
            memory_type: "commitment".into(),
            title: "Existing synthetic memory".into(),
            summary: "Existing synthetic commitment".into(),
        })
        .unwrap();
    drop(session);
    let conn = raw(dir.path());
    conn.execute_batch(
        "DROP TABLE assistance_items; UPDATE meta SET value='3' WHERE key='schema_version'",
    )
    .unwrap();
    drop(conn);
    let reopened = Session::open(dir.path());
    assert!(reopened.status().store_opened);
    assert!(reopened.assistance_items().unwrap().items.is_empty());
    assert_eq!(reopened.memory(&memory.memory_id).unwrap(), memory);
    let conn = raw(dir.path());
    let version: String = conn
        .query_row(
            "SELECT value FROM meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(version, "4");
}

#[test]
fn absolute_deadline_only_and_terminal_actions_never_reopen() {
    let (_dir, mut session) = setup();
    for at in [
        "tomorrow",
        "2026-11-20",
        "2026-11-20T16:00:00",
        "2026-11-20T16:00:00-00:00",
        "2026-02-30T16:00:00+08:00",
    ] {
        assert!(session
            .create_assistance_goal(CreateAssistanceGoal {
                title: "synthetic".into(),
                summary: "synthetic detail".into(),
                deadline_at: Some(at.into()),
                source_observation: None
            })
            .is_err());
    }
    let a = goal(&mut session, None);
    let b = goal(&mut session, None);
    let done = session
        .act_assistance_item(AssistanceActionRequest {
            item_id: a.item.item_id,
            expected_revision: a.item.revision,
            action: AssistanceAction::CompleteSelfReported,
        })
        .unwrap();
    assert_eq!(done.item.handling, AssistanceHandling::ResolvedReported);
    assert!(session.prepare_assistance_context(req(&a.item)).is_err());
    assert!(session
        .edit_assistance_goal(EditAssistanceGoal {
            item_id: a.item.item_id,
            expected_revision: a.item.revision,
            title: "late".into(),
            summary: "late".into(),
            deadline_at: None
        })
        .is_err());
    assert!(session
        .bind_assistance_goal_source(BindAssistanceGoalSource {
            item_id: done.item.item_id,
            expected_revision: done.item.revision,
            source_observation: None
        })
        .is_err());
    let cancelled = session
        .act_assistance_item(AssistanceActionRequest {
            item_id: b.item.item_id,
            expected_revision: b.item.revision,
            action: AssistanceAction::Cancel,
        })
        .unwrap();
    assert_eq!(cancelled.item.handling, AssistanceHandling::Cancelled);
    assert_eq!(
        session
            .assistance_item(&a.item.item_id.to_string())
            .unwrap()
            .item
            .handling,
        AssistanceHandling::ResolvedReported
    );
}

#[test]
fn ordinary_and_direct_memory_edits_refresh_revision_and_cannot_change_goal_type() {
    let (_dir, mut session) = setup();
    let item = goal(&mut session, None);
    let memory = memory_id(&item.item);
    let context = session.prepare_assistance_context(req(&item.item)).unwrap();
    let AssistanceContextView::Memory {
        memory: context, ..
    } = context
    else {
        panic!("memory context")
    };
    let shared = session.store().unwrap();
    {
        let mut store = shared.lock().unwrap();
        let mut row = store.get_memory(memory).unwrap();
        row.memory_type = MemoryType::Semantic;
        assert!(store.put_memory(row).is_err());
        let row = store.get_memory(memory).unwrap();
        store.put_memory(row).unwrap();
    }
    assert!(session
        .draft_pasted_with_memory("synthetic paste", &context.context_id.unwrap().to_string())
        .is_err());
    let edited = session
        .edit_assistance_goal(EditAssistanceGoal {
            item_id: item.item.item_id,
            expected_revision: session
                .assistance_item(&item.item.item_id.to_string())
                .unwrap()
                .item
                .revision,
            title: "Changed synthetic goal".into(),
            summary: "Changed synthetic summary".into(),
            deadline_at: None,
        })
        .unwrap();
    assert_eq!(
        edited.goal.as_ref().unwrap().title,
        "Changed synthetic goal"
    );
    assert_eq!(edited.item.deadline_at, None);
    assert_eq!(
        edited.goal.unwrap().content_key_id,
        item.goal.unwrap().content_key_id
    );
    assert!(session
        .edit_memory(
            &memory.to_string(),
            &MemoryChange {
                memory_type: Some("preference".into()),
                ..Default::default()
            }
        )
        .is_err());
}

#[test]
fn failed_item_revision_write_rolls_back_ordinary_and_direct_memory_edits() {
    let (dir, mut session) = setup();
    let item = goal(&mut session, None);
    let memory = memory_id(&item.item);
    let conn = raw(dir.path());
    let shared = session.store().unwrap();
    let original = shared.lock().unwrap().get_memory(memory).unwrap();
    let blobs = shared.lock().unwrap().blob_count().unwrap();
    conn.execute_batch("CREATE TRIGGER fail_item_revision BEFORE UPDATE ON assistance_items BEGIN SELECT RAISE(ABORT,'synthetic revision failure'); END").unwrap();
    assert!(session
        .edit_memory(
            &memory.to_string(),
            &MemoryChange {
                title: Some("must roll back".into()),
                ..Default::default()
            }
        )
        .is_err());
    {
        let mut store = shared.lock().unwrap();
        assert_eq!(store.get_memory(memory).unwrap(), original);
        assert_eq!(store.blob_count().unwrap(), blobs);
        assert_eq!(store.assistance_item(item.item.item_id).unwrap(), item.item);
        let mut changed = original.clone();
        changed.third_party_content_present = Some(true);
        assert!(store.put_memory(changed).is_err());
        assert_eq!(store.get_memory(memory).unwrap(), original);
        assert_eq!(store.assistance_item(item.item.item_id).unwrap(), item.item);
    }
    conn.execute_batch("DROP TRIGGER fail_item_revision")
        .unwrap();
    assert_eq!(
        session.memory(&memory.to_string()).unwrap(),
        item.goal.unwrap()
    );
}

#[test]
fn source_forget_stales_goal_but_keeps_its_memory_and_clear_source_restores_current() {
    let (_dir, mut session) = setup();
    let observation = observe(&mut session);
    let sourced = goal(&mut session, Some(req(&observation.item)));
    let independent = goal(&mut session, None);
    let context = session
        .prepare_assistance_context(req(&sourced.item))
        .unwrap();
    assert!(matches!(context, AssistanceContextView::Insight { .. }));
    let AssistanceSource::Observation { contact_id, .. } = observation.item.source else {
        panic!("observation")
    };
    {
        let shared = session.store().unwrap();
        let mut store = shared.lock().unwrap();
        let outcome = store
            .execute_forget_outcome(ForgetUnit::Contact(contact_id))
            .unwrap();
        assert!(outcome.receipt.impact.assistance_items_invalidated >= 2);
    }
    let after = session
        .assistance_item(&sourced.item.item_id.to_string())
        .unwrap();
    assert_eq!(after.item.validity, AssistanceValidity::Stale);
    assert_eq!(after.item.handling, AssistanceHandling::Open);
    assert!(after.goal.is_some());
    assert!(session
        .memory(&memory_id(&sourced.item).to_string())
        .is_ok());
    assert_eq!(
        session
            .assistance_item(&independent.item.item_id.to_string())
            .unwrap()
            .item
            .revision,
        independent.item.revision
    );
    assert!(session
        .prepare_assistance_context(req(&sourced.item))
        .is_err());
    let cleared = session
        .bind_assistance_goal_source(BindAssistanceGoalSource {
            item_id: after.item.item_id,
            expected_revision: after.item.revision,
            source_observation: None,
        })
        .unwrap();
    assert_eq!(cleared.item.validity, AssistanceValidity::Current);
    assert!(matches!(
        session
            .prepare_assistance_context(req(&cleared.item))
            .unwrap(),
        AssistanceContextView::Memory { .. }
    ));
}

#[test]
fn observation_identity_and_goal_fixed_source_do_not_follow_recompute_or_termination() {
    let (_dir, mut session) = setup();
    let observation = observe(&mut session);
    let sourced = goal(&mut session, Some(req(&observation.item)));
    let AssistanceSource::Observation {
        bundle_id,
        contact_id,
        ..
    } = observation.item.source
    else {
        panic!("observation")
    };
    let recomputed = session
        .recompute_local_insight(&bundle_id.to_string())
        .unwrap();
    let duplicate = session
        .link_assistance_observation(LinkAssistanceObservation {
            bundle_id: recomputed.bundle_id,
            contact_id,
        })
        .unwrap();
    assert_eq!(duplicate.item, observation.item);
    let refreshed = session
        .refresh_assistance_observation(RefreshAssistanceObservation {
            item_id: observation.item.item_id,
            expected_revision: observation.item.revision,
            bundle_id: recomputed.bundle_id,
        })
        .unwrap();
    assert_eq!(refreshed.item.item_id, observation.item.item_id);
    assert_ne!(refreshed.item.revision, observation.item.revision);
    let stale = session
        .assistance_item(&sourced.item.item_id.to_string())
        .unwrap();
    assert_eq!(stale.item.validity, AssistanceValidity::Stale);
    let AssistanceSource::ManualGoal {
        observation: Some(fixed),
        ..
    } = stale.item.source
    else {
        panic!("fixed source")
    };
    assert_eq!(fixed.bundle_id, bundle_id);
    let cancelled = session
        .act_assistance_item(AssistanceActionRequest {
            item_id: refreshed.item.item_id,
            expected_revision: refreshed.item.revision,
            action: AssistanceAction::Cancel,
        })
        .unwrap();
    let duplicate = session
        .link_assistance_observation(LinkAssistanceObservation {
            bundle_id: recomputed.bundle_id,
            contact_id,
        })
        .unwrap();
    assert_eq!(duplicate.item.handling, AssistanceHandling::Cancelled);
    assert_eq!(duplicate.item.revision, cancelled.item.revision);
}

#[test]
fn stale_observation_revision_cannot_create_or_rebind_a_goal() {
    let (_dir, mut session) = setup();
    let observation = observe(&mut session);
    let target = goal(&mut session, None);
    let AssistanceSource::Observation { bundle_id, .. } = observation.item.source else {
        panic!("observation")
    };
    let fresh = session
        .recompute_local_insight(&bundle_id.to_string())
        .unwrap();
    session
        .refresh_assistance_observation(RefreshAssistanceObservation {
            item_id: observation.item.item_id,
            expected_revision: observation.item.revision,
            bundle_id: fresh.bundle_id,
        })
        .unwrap();
    let shared = session.store().unwrap();
    let count = shared.lock().unwrap().list_memories().unwrap().len();
    assert!(session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "must not persist".into(),
            summary: "must not persist".into(),
            deadline_at: None,
            source_observation: Some(req(&observation.item))
        })
        .is_err());
    assert_eq!(shared.lock().unwrap().list_memories().unwrap().len(), count);
    assert!(session
        .bind_assistance_goal_source(BindAssistanceGoalSource {
            item_id: target.item.item_id,
            expected_revision: target.item.revision,
            source_observation: Some(req(&observation.item))
        })
        .is_err());
    assert_eq!(
        session
            .assistance_item(&target.item.item_id.to_string())
            .unwrap()
            .item,
        target.item
    );
    assert_eq!(
        session
            .memory(&memory_id(&target.item).to_string())
            .unwrap(),
        target.goal.unwrap()
    );
}

#[test]
fn same_count_item_revision_change_rejects_old_forget_quote_and_goal_forget_destroys_memory() {
    let (_dir, mut session) = setup();
    let item = goal(&mut session, None);
    let memory = memory_id(&item.item);
    let preview = session.preview_forget(&memory.to_string()).unwrap();
    assert_eq!(preview.assistance_items_invalidated, 1);
    {
        let shared = session.store().unwrap();
        let mut store = shared.lock().unwrap();
        let mut row = store.assistance_item(item.item.item_id).unwrap();
        let old = row.revision;
        row.revision = Uuid::now_v7();
        store.replace_assistance_item(&row, old).unwrap();
    }
    assert!(session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: memory.to_string()
        })
        .is_err());
    assert!(session.memory(&memory.to_string()).is_ok());
    let preview = session.preview_forget(&memory.to_string()).unwrap();
    session
        .forget_memory(&ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: memory.to_string(),
        })
        .unwrap();
    assert!(session.memory(&memory.to_string()).is_err());
    let row = session
        .assistance_item(&item.item.item_id.to_string())
        .unwrap();
    assert_eq!(row.item.validity, AssistanceValidity::Forgotten);
    assert!(row.goal.is_none());
}

#[test]
fn changed_item_identity_and_damaged_auxiliary_fields_cannot_bypass_quote_or_block_destruction() {
    let (dir, mut session) = setup();
    let item = goal(&mut session, None);
    let memory = memory_id(&item.item);
    let conn = raw(dir.path());
    let quote = session.preview_forget(&memory.to_string()).unwrap();
    conn.execute(
        "UPDATE assistance_items SET item_id=?2 WHERE item_id=?1",
        rusqlite::params![item.item.item_id.to_string(), Uuid::now_v7().to_string()],
    )
    .unwrap();
    assert!(session
        .forget_memory(&ForgetConfirmation {
            preview_id: quote.preview_id,
            memory_id: memory.to_string()
        })
        .is_err());
    // Damage auxiliary UUID text only; the cryptographic unit and SQL row are
    // still precisely identifiable without deserializing that auxiliary model.
    conn.execute_batch(
        "UPDATE assistance_items SET item_id='damaged-id',revision='damaged-revision'",
    )
    .unwrap();
    let quote = session.preview_forget(&memory.to_string()).unwrap();
    assert_eq!(quote.assistance_items_invalidated, 1);
    let receipt = session
        .forget_memory(&ForgetConfirmation {
            preview_id: quote.preview_id,
            memory_id: memory.to_string(),
        })
        .unwrap();
    assert!(receipt.logical_committed);
    assert!(session.memory(&memory.to_string()).is_err());
    let state: String = conn
        .query_row("SELECT validity FROM assistance_items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(state, "forgotten");
}

#[test]
fn sourced_goal_own_blob_failure_refuses_without_permanent_metadata_change() {
    let (dir, mut session) = setup();
    let observation = observe(&mut session);
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let item = goal(&mut session, Some(req(&observation.item)));
    let context = session.prepare_assistance_context(req(&item.item)).unwrap();
    let AssistanceContextView::Insight { insight, .. } = context else {
        panic!("insight")
    };
    let plan = session
        .prepare_draft_with_insight(
            "synthetic paste",
            &insight.context_id.to_string(),
            false,
            false,
        )
        .unwrap();
    let conn = raw(dir.path());
    let memory = memory_id(&item.item);
    let shared = session.store().unwrap();
    let row = shared.lock().unwrap().get_memory(memory).unwrap();
    let blob = row.title_ref.unwrap().blob_id.to_string();
    // Temporarily remove only this ciphertext, then restore its exact row.
    conn.execute_batch("CREATE TEMP TABLE saved_blob AS SELECT * FROM sealed_blobs WHERE 0")
        .unwrap();
    conn.execute(
        "INSERT INTO saved_blob SELECT * FROM sealed_blobs WHERE blob_id=?1",
        [&blob],
    )
    .unwrap();
    conn.execute("DELETE FROM sealed_blobs WHERE blob_id=?1", [&blob])
        .unwrap();
    assert!(session
        .assistance_item(&item.item.item_id.to_string())
        .is_err());
    assert!(session.prepare_assistance_context(req(&item.item)).is_err());
    assert!(session
        .local_draft_with_insight("synthetic paste", &insight.context_id.to_string())
        .is_err());
    assert!(session.begin_draft(&plan.prepared.plan.approval()).is_err());
    assert_eq!(endpoint.request_count(), 0);
    assert_eq!(
        shared
            .lock()
            .unwrap()
            .assistance_item(item.item.item_id)
            .unwrap(),
        item.item
    );
    conn.execute_batch("INSERT INTO sealed_blobs SELECT * FROM saved_blob")
        .unwrap();
    assert_eq!(
        session
            .assistance_item(&item.item.item_id.to_string())
            .unwrap()
            .item,
        item.item
    );
    let AssistanceContextView::Insight { insight, .. } =
        session.prepare_assistance_context(req(&item.item)).unwrap()
    else {
        panic!("insight")
    };
    let plan = session
        .prepare_draft_with_insight(
            "synthetic paste",
            &insight.context_id.to_string(),
            false,
            false,
        )
        .unwrap();
    let work = session.begin_draft(&plan.prepared.plan.approval()).unwrap();
    conn.execute("DELETE FROM sealed_blobs WHERE blob_id=?1", [&blob])
        .unwrap();
    assert!(session.complete_draft(work.execute()).is_err());
    assert_eq!(endpoint.request_count(), 1);
    assert_eq!(
        shared
            .lock()
            .unwrap()
            .assistance_item(item.item.item_id)
            .unwrap(),
        item.item
    );
    conn.execute_batch("INSERT INTO sealed_blobs SELECT * FROM saved_blob")
        .unwrap();
}

#[test]
fn cancellation_after_dispatch_keeps_slot_until_worker_finishes_and_audits_request() {
    let (_dir, mut session) = setup();
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let item = goal(&mut session, None);
    let AssistanceContextView::Memory { memory, .. } =
        session.prepare_assistance_context(req(&item.item)).unwrap()
    else {
        panic!("memory")
    };
    let plan = session
        .prepare_draft_with_memory(
            "synthetic paste",
            &memory.context_id.unwrap().to_string(),
            &[],
            false,
        )
        .unwrap();
    let work = session.begin_draft(&plan.prepared.plan.approval()).unwrap();
    session
        .act_assistance_item(AssistanceActionRequest {
            item_id: item.item.item_id,
            expected_revision: item.item.revision,
            action: AssistanceAction::Cancel,
        })
        .unwrap();
    assert!(session
        .prepare_draft("another synthetic paste", None)
        .is_err());
    assert!(session.complete_draft(work.execute()).is_err());
    assert_eq!(endpoint.request_count(), 1);
    let audit = session.audit().unwrap();
    assert!(audit.entries.iter().any(|e| e.action == "egress.request"));
    assert!(!audit.entries.iter().any(|e| e.action == "draft.create"));
    assert!(session.prepare_draft("new synthetic paste", None).is_ok());
}
