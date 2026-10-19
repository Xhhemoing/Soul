//! U5 opt-in, exact ephemeral previews and transactional invalidation.
use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
use soul_store_api::forget::ForgetOutcomeOps;
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_store_api::{ForgetUnit, GraphStore, MemoryStore, ProfileStore};
use soulcore::commands::memory::ForgetConfirmation;
use soulcore::commands::session::*;
use soulcore::commands::store::database_path;
use std::time::{Duration, Instant};

fn setup() -> (tempfile::TempDir, Session, AssistanceItemRow) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Session::open(dir.path());
    let i = s
        .create_assistance_goal(CreateAssistanceGoal {
            title: "synthetic opening goal".into(),
            summary: "SYNTHETIC_GOAL_IS_NOT_AN_INBOUND_MESSAGE".into(),
            deadline_at: None,
            source_observation: None,
        })
        .unwrap()
        .item;
    (dir, s, i)
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
fn run(s: &mut Session) {
    for _ in 0..20 {
        let Some(work) = s.begin_assistance_work().unwrap() else {
            return;
        };
        s.complete_assistance_work(work.execute()).unwrap();
    }
    panic!("local work failed to settle")
}
fn enable(s: &mut Session, i: &AssistanceItemRow, enabled: bool) -> AssistanceLocalPreparation {
    let prior = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    s.set_assistance_local_preparation(SetAssistanceLocalPreparation {
        item_id: i.item_id,
        expected_revision: i.revision,
        expected_preference_revision: prior.preference_revision,
        enabled,
    })
    .unwrap()
}
fn request(meta: &AssistanceLocalPreparation) -> PreviewAssistanceLocalTemplate {
    PreviewAssistanceLocalTemplate {
        item_id: meta.item_id,
        expected_revision: meta.item_revision,
        expected_preference_revision: meta.preference_revision.unwrap(),
        template_id: meta.template_id.unwrap(),
    }
}
fn ready(s: &mut Session, i: &AssistanceItemRow) -> AssistanceLocalPreparation {
    s.set_voice("register", "plain").unwrap();
    enable(s, i, true);
    run(s);
    let meta = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert_eq!(meta.state, AssistanceLocalPreparationState::Ready);
    meta
}
fn preview(
    s: &mut Session,
    meta: &AssistanceLocalPreparation,
) -> Result<AssistanceLocalTemplatePreview, SessionRefusal> {
    let work = s.begin_assistance_local_template_preview(request(meta))?;
    s.complete_assistance_local_template_preview(work.execute())
}
fn observe(s: &mut Session) -> AssistanceItemRow {
    let text =
        soul_testkit::fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap();
    let work = s
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let p = s.complete_import_prepare(work.execute()).unwrap();
    let work = s
        .begin_import(
            &p.operation.operation_id.to_string(),
            &p.operation.preview_id.to_string(),
        )
        .unwrap();
    s.complete_import(work.execute()).unwrap();
    let bundle = s.local_insights().unwrap().remove(0);
    s.link_assistance_observation(LinkAssistanceObservation {
        bundle_id: bundle.bundle_id,
        contact_id: bundle.summaries[0].contact_id.parse().unwrap(),
    })
    .unwrap()
    .item
}

#[test]
fn default_off_and_strict_owner_profile_prepare_only_an_opening_skeleton_without_egress() {
    let (dir, mut s, i) = setup();
    run(&mut s);
    let off = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert!(!off.enabled);
    assert!(off.template_id.is_none());
    assert!(off.preference_revision.is_none());
    enable(&mut s, &i, true);
    run(&mut s);
    let failed = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert_eq!(failed.state, AssistanceLocalPreparationState::Failed);
    assert_eq!(
        failed.failure_code.as_deref(),
        Some("ASSISTANCE_LOCAL_PROFILE_UNAVAILABLE")
    );
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string())
            .unwrap()
            .state,
        AssistanceWorkState::Ready
    );
    s.set_voice("register", "formal").unwrap();
    run(&mut s);
    let meta = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    let p = preview(&mut s, &meta).unwrap();
    assert!(p.body.contains(soul_draft::BODY_SLOT));
    assert!(p.body.starts_with("您好"));
    assert!(!p.body.contains("SYNTHETIC_GOAL"));
    assert_eq!(p.kind, LocalTemplateKind::OpeningV1);
    let conn = raw(dir.path());
    let stored: String = conn
        .query_row(
            "SELECT source_doc FROM assistance_local_templates",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!stored.contains("SYNTHETIC_GOAL"));
    assert!(!stored.contains(soul_draft::BODY_SLOT));
    assert_eq!(s.assistance_item(&i.item_id.to_string()).unwrap().item, i);
    assert!(!s
        .audit()
        .unwrap()
        .entries
        .iter()
        .any(|e| e.action == "egress.request"));
}

#[test]
fn preference_aba_rejects_old_inflight_templates_without_changing_base_work_or_new_pending() {
    let (_dir, mut s, i) = setup();
    s.set_voice("register", "plain").unwrap();
    let a = enable(&mut s, &i, true);
    let shared = s.store().unwrap();
    let old = s.begin_assistance_work().unwrap().unwrap();
    let off = shared
        .lock()
        .unwrap()
        .set_local_preparation_preference(
            i.item_id,
            i.revision,
            a.preference_revision,
            false,
            OWNER_PROFILE_ID,
        )
        .unwrap();
    let b = shared
        .lock()
        .unwrap()
        .set_local_preparation_preference(
            i.item_id,
            i.revision,
            off.revision,
            true,
            OWNER_PROFILE_ID,
        )
        .unwrap();
    assert_ne!(a.preference_revision, b.revision);
    let out = s.complete_assistance_work(old.execute()).unwrap();
    assert_eq!(out.status, AssistanceWorkDisposition::Ready);
    let current = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert_eq!(current.state, AssistanceLocalPreparationState::Pending);
    assert!(current.template_id.is_none());
    run(&mut s);
    let ready = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    let work = s
        .begin_assistance_local_template_preview(request(&ready))
        .unwrap();
    let done = work.execute();
    let off = shared
        .lock()
        .unwrap()
        .set_local_preparation_preference(
            i.item_id,
            i.revision,
            ready.preference_revision,
            false,
            OWNER_PROFILE_ID,
        )
        .unwrap();
    shared
        .lock()
        .unwrap()
        .set_local_preparation_preference(
            i.item_id,
            i.revision,
            off.revision,
            true,
            OWNER_PROFILE_ID,
        )
        .unwrap();
    assert!(s.complete_assistance_local_template_preview(done).is_err());
    assert_eq!(s.assistance_item(&i.item_id.to_string()).unwrap().item, i);
    run(&mut s);
    let new = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert_ne!(ready.template_id, new.template_id);
    assert!(preview(&mut s, &new).is_ok());
}

#[test]
fn opt_out_preserves_a_newer_business_revision_while_an_old_claim_is_running() {
    let (_dir, mut s, i) = setup();
    s.set_voice("register", "plain").unwrap();
    let pref = enable(&mut s, &i, true);
    let shared = s.store().unwrap();
    let old = s.begin_assistance_work().unwrap().unwrap();
    let AssistanceSource::ManualGoal { memory_id, .. } = i.source else {
        panic!("manual")
    };
    let memory = shared.lock().unwrap().get_memory(memory_id).unwrap();
    shared.lock().unwrap().put_memory(memory).unwrap();
    let edited = shared.lock().unwrap().assistance_item(i.item_id).unwrap();
    shared
        .lock()
        .unwrap()
        .set_local_preparation_preference(
            i.item_id,
            edited.revision,
            pref.preference_revision,
            false,
            OWNER_PROFILE_ID,
        )
        .unwrap();
    assert_eq!(
        s.complete_assistance_work(old.execute()).unwrap().status,
        AssistanceWorkDisposition::Superseded
    );
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string())
            .unwrap()
            .pending_revision,
        Some(edited.revision)
    );
    run(&mut s);
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string())
            .unwrap()
            .state,
        AssistanceWorkState::Ready
    );
    let meta = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert_eq!(meta.state, AssistanceLocalPreparationState::Off);
    assert!(meta.template_id.is_none());
}

#[test]
fn a_replaced_exact_claim_cannot_publish_its_old_template() {
    let (dir, mut s, i) = setup();
    s.set_voice("register", "plain").unwrap();
    enable(&mut s, &i, true);
    let work = s.begin_assistance_work().unwrap().unwrap();
    let conn = raw(dir.path());
    conn.execute(
        "UPDATE assistance_work SET claim_id=?2 WHERE item_id=?1",
        rusqlite::params![
            i.item_id.to_string(),
            AssistanceNotificationId::now_v7().to_string()
        ],
    )
    .unwrap();
    assert!(s.complete_assistance_work(work.execute()).is_err());
    assert_eq!(
        conn.query_row("SELECT count(*) FROM assistance_local_templates", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn profile_four_enum_changes_are_atomic_but_axes_locks_and_other_owners_do_not_revoke_templates() {
    let (dir, mut s, i) = setup();
    let meta = ready(&mut s, &i);
    let shared = s.store().unwrap();
    let profile = shared
        .lock()
        .unwrap()
        .get_profile(OWNER_PROFILE_ID)
        .unwrap();
    let work = s.assistance_item_work(&i.item_id.to_string()).unwrap();
    let conn = raw(dir.path());
    let audits = s.audit().unwrap().entries.len();
    conn.execute_batch("CREATE TRIGGER fail_preparation_pending BEFORE UPDATE ON assistance_work BEGIN SELECT RAISE(ABORT,'synthetic profile rollback'); END").unwrap();
    assert!(s.set_voice("warmth", "warm").is_err());
    let mut changed = profile.clone();
    changed.voice["warmth"] = serde_json::json!("warm");
    assert!(shared.lock().unwrap().put_profile(changed).is_err());
    assert_eq!(
        shared
            .lock()
            .unwrap()
            .get_profile(OWNER_PROFILE_ID)
            .unwrap(),
        profile
    );
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string()).unwrap(),
        work
    );
    assert_eq!(
        s.assistance_local_preparation(&i.item_id.to_string())
            .unwrap()
            .template_id,
        meta.template_id
    );
    assert_eq!(s.audit().unwrap().entries.len(), audits);
    conn.execute_batch("DROP TRIGGER fail_preparation_pending")
        .unwrap();
    let mut lock_only = profile.clone();
    lock_only.voice["user_set"] = serde_json::json!(["register", "warmth"]);
    lock_only.trait_axes[0].locked_by_user = Some(true);
    shared
        .lock()
        .unwrap()
        .put_profile(lock_only.clone())
        .unwrap();
    assert_eq!(
        s.assistance_local_preparation(&i.item_id.to_string())
            .unwrap()
            .template_id,
        meta.template_id
    );
    lock_only.profile_id = AssistanceNotificationId::now_v7();
    lock_only.voice["warmth"] = serde_json::json!("warm");
    shared.lock().unwrap().put_profile(lock_only).unwrap();
    assert_eq!(
        s.assistance_local_preparation(&i.item_id.to_string())
            .unwrap()
            .template_id,
        meta.template_id
    );
    s.set_voice("warmth", "warm").unwrap();
    assert!(s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap()
        .template_id
        .is_none());
    run(&mut s);
    s.set_voice("warmth", "even").unwrap();
    run(&mut s);
    let refreshed = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert_ne!(refreshed.template_id, meta.template_id);
    assert_eq!(s.assistance_item(&i.item_id.to_string()).unwrap().item, i);
    assert!(preview(&mut s, &refreshed).is_ok());
}

#[test]
fn preview_read_fault_is_recoverable_but_a_real_mismatch_commits_refresh_before_refusal() {
    let (dir, mut s, i) = setup();
    let meta = ready(&mut s, &i);
    let conn = raw(dir.path());
    let doc: String = conn
        .query_row(
            "SELECT doc FROM profiles WHERE profile_id=?1",
            [OWNER_PROFILE_ID.to_string()],
            |r| r.get(0),
        )
        .unwrap();
    let AssistanceSource::ManualGoal { memory_id, .. } = i.source else {
        panic!("manual")
    };
    let shared = s.store().unwrap();
    let blob = shared
        .lock()
        .unwrap()
        .get_memory(memory_id)
        .unwrap()
        .title_ref
        .unwrap()
        .blob_id
        .to_string();
    conn.execute_batch(
        "CREATE TEMP TABLE saved_preview_blob AS SELECT * FROM sealed_blobs WHERE 0",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO saved_preview_blob SELECT * FROM sealed_blobs WHERE blob_id=?1",
        [&blob],
    )
    .unwrap();
    conn.execute("DELETE FROM sealed_blobs WHERE blob_id=?1", [&blob])
        .unwrap();
    assert!(preview(&mut s, &meta).is_err());
    assert_eq!(
        s.assistance_local_preparation(&i.item_id.to_string())
            .unwrap()
            .template_id,
        meta.template_id
    );
    assert_eq!(
        shared.lock().unwrap().assistance_item(i.item_id).unwrap(),
        i
    );
    conn.execute_batch("INSERT INTO sealed_blobs SELECT * FROM saved_preview_blob")
        .unwrap();
    assert!(preview(&mut s, &meta).is_ok());
    conn.execute(
        "UPDATE profiles SET doc='{' WHERE profile_id=?1",
        [OWNER_PROFILE_ID.to_string()],
    )
    .unwrap();
    assert!(preview(&mut s, &meta).is_err());
    assert_eq!(s.assistance_item(&i.item_id.to_string()).unwrap().item, i);
    assert_eq!(
        s.assistance_local_preparation(&i.item_id.to_string())
            .unwrap()
            .template_id,
        meta.template_id
    );
    conn.execute(
        "UPDATE profiles SET doc=?2 WHERE profile_id=?1",
        rusqlite::params![OWNER_PROFILE_ID.to_string(), doc.clone()],
    )
    .unwrap();
    assert!(preview(&mut s, &meta).is_ok());
    let mut altered: serde_json::Value = serde_json::from_str(&doc).unwrap();
    altered["voice"]["warmth"] = serde_json::json!("warm");
    conn.execute(
        "UPDATE profiles SET doc=?2 WHERE profile_id=?1",
        rusqlite::params![OWNER_PROFILE_ID.to_string(), altered.to_string()],
    )
    .unwrap();
    assert_eq!(
        preview(&mut s, &meta).unwrap_err().reason_code,
        "ASSISTANCE_LOCAL_PREPARATION_CHANGED"
    );
    let next = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert_eq!(next.state, AssistanceLocalPreparationState::Pending);
    assert!(next.template_id.is_none());
    assert_eq!(s.assistance_item(&i.item_id.to_string()).unwrap().item, i);
    run(&mut s);
    let refreshed = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert!(preview(&mut s, &refreshed).is_ok());
}

#[test]
fn owned_preview_preserves_held_memory_and_busy_controls_are_immediately_reachable() {
    let (_dir, mut s, i) = setup();
    let meta = ready(&mut s, &i);
    let context = s
        .prepare_assistance_context(AssistanceItemRequest {
            item_id: i.item_id,
            expected_revision: i.revision,
        })
        .unwrap();
    let AssistanceContextView::Memory { memory, .. } = context else {
        panic!("memory")
    };
    let shared = s.store().unwrap();
    let work = s
        .begin_assistance_local_template_preview(request(&meta))
        .unwrap();
    let guard = shared.lock().unwrap();
    let start = Instant::now();
    assert!(s.memories().is_err());
    s.stop_assistance_reminder_scheduler();
    s.revoke_collect_consent().unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(guard);
    assert!(s
        .complete_assistance_local_template_preview(work.execute())
        .is_ok());
    s.draft_pasted_with_memory(
        "synthetic explicit paste",
        &memory.context_id.unwrap().to_string(),
    )
    .unwrap();
    assert!(s.begin_assistance_work().unwrap().is_none());
    let dropped = s
        .begin_assistance_local_template_preview(request(&meta))
        .unwrap();
    drop(dropped);
    let next = s
        .begin_assistance_local_template_preview(request(&meta))
        .unwrap();
    assert!(s
        .complete_assistance_local_template_preview(next.execute())
        .is_ok());
}

#[test]
fn forgetting_compares_exact_template_and_preference_rows_but_bad_auxiliary_data_cannot_save_the_ck(
) {
    let (dir, mut s, i) = setup();
    let _meta = ready(&mut s, &i);
    let AssistanceSource::ManualGoal { memory_id, .. } = i.source else {
        panic!("manual")
    };
    let conn = raw(dir.path());
    let quote = s.preview_forget(&memory_id.to_string()).unwrap();
    conn.execute(
        "UPDATE assistance_local_templates SET template_id=?1",
        [AssistanceNotificationId::now_v7().to_string()],
    )
    .unwrap();
    assert!(s
        .forget_memory(&ForgetConfirmation {
            preview_id: quote.preview_id,
            memory_id: memory_id.to_string()
        })
        .is_err());
    let quote = s.preview_forget(&memory_id.to_string()).unwrap();
    conn.execute(
        "UPDATE assistance_preparation_preferences SET revision=?1",
        [AssistanceNotificationId::now_v7().to_string()],
    )
    .unwrap();
    assert!(s
        .forget_memory(&ForgetConfirmation {
            preview_id: quote.preview_id,
            memory_id: memory_id.to_string()
        })
        .is_err());
    conn.execute_batch("UPDATE assistance_local_templates SET template_id='bad-template',source_doc='{',preference_revision='bad-pref'; UPDATE assistance_preparation_preferences SET revision='bad-pref',owner_profile_id='bad-owner'").unwrap();
    let quote = s.preview_forget(&memory_id.to_string()).unwrap();
    assert!(
        s.forget_memory(&ForgetConfirmation {
            preview_id: quote.preview_id,
            memory_id: memory_id.to_string()
        })
        .unwrap()
        .logical_committed
    );
    assert!(s.memory(&memory_id.to_string()).is_err());
    assert_eq!(
        conn.query_row("SELECT count(*) FROM assistance_local_templates", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM assistance_preparation_preferences",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn source_writes_revoke_only_derived_templates_atomically_and_late_preview_is_refused() {
    let (dir, mut s, _independent) = setup();
    let observation = observe(&mut s);
    let item = s
        .create_assistance_goal(CreateAssistanceGoal {
            title: "synthetic sourced goal".into(),
            summary: "synthetic goal".into(),
            deadline_at: None,
            source_observation: Some(AssistanceItemRequest {
                item_id: observation.item_id,
                expected_revision: observation.revision,
            }),
        })
        .unwrap()
        .item;
    let meta = ready(&mut s, &item);
    let shared = s.store().unwrap();
    let AssistanceSource::Observation { contact_id, .. } = observation.source else {
        panic!("observation")
    };
    let contact = shared.lock().unwrap().get_contact(contact_id).unwrap();
    let conn = raw(dir.path());
    let mut altered = contact.clone();
    altered.contact_class = soul_schema::contact::ContactClass::Service;
    conn.execute_batch("CREATE TRIGGER fail_source_pending BEFORE UPDATE ON assistance_work BEGIN SELECT RAISE(ABORT,'synthetic source rollback'); END").unwrap();
    assert!(shared.lock().unwrap().put_contact(altered).is_err());
    assert_eq!(
        shared.lock().unwrap().get_contact(contact_id).unwrap(),
        contact
    );
    assert_eq!(
        s.assistance_local_preparation(&item.item_id.to_string())
            .unwrap()
            .template_id,
        meta.template_id
    );
    conn.execute_batch("DROP TRIGGER fail_source_pending")
        .unwrap();
    let work = s
        .begin_assistance_local_template_preview(request(&meta))
        .unwrap();
    let completed = work.execute();
    shared.lock().unwrap().put_contact(contact).unwrap();
    assert!(s
        .complete_assistance_local_template_preview(completed)
        .is_err());
    let next = s
        .assistance_local_preparation(&item.item_id.to_string())
        .unwrap();
    assert!(next.template_id.is_none());
    assert_eq!(next.state, AssistanceLocalPreparationState::Pending);
    assert_eq!(
        s.assistance_item(&item.item_id.to_string()).unwrap().item,
        item
    );
    run(&mut s);
    let refreshed = s
        .assistance_local_preparation(&item.item_id.to_string())
        .unwrap();
    assert!(preview(&mut s, &refreshed).is_ok());
}

#[test]
fn forgetting_a_source_keeps_independent_goal_opt_in_and_own_memory_until_explicit_rebind() {
    let (_dir, mut s, _independent) = setup();
    let observation = observe(&mut s);
    let item = s
        .create_assistance_goal(CreateAssistanceGoal {
            title: "synthetic sourced goal".into(),
            summary: "synthetic goal".into(),
            deadline_at: None,
            source_observation: Some(AssistanceItemRequest {
                item_id: observation.item_id,
                expected_revision: observation.revision,
            }),
        })
        .unwrap()
        .item;
    let meta = ready(&mut s, &item);
    let shared = s.store().unwrap();
    let AssistanceSource::Observation { contact_id, .. } = observation.source else {
        panic!("observation")
    };
    let AssistanceSource::ManualGoal { memory_id, .. } = item.source else {
        panic!("manual")
    };
    shared
        .lock()
        .unwrap()
        .execute_forget_outcome(ForgetUnit::Contact(contact_id))
        .unwrap();
    let state = s
        .assistance_local_preparation(&item.item_id.to_string())
        .unwrap();
    assert!(state.enabled);
    assert_eq!(state.preference_revision, meta.preference_revision);
    assert!(state.template_id.is_none());
    assert!(s.memory(&memory_id.to_string()).is_ok());
    let current = s.assistance_item(&item.item_id.to_string()).unwrap().item;
    assert_eq!(current.validity, AssistanceValidity::Stale);
    let restored = s
        .bind_assistance_goal_source(BindAssistanceGoalSource {
            item_id: current.item_id,
            expected_revision: current.revision,
            source_observation: None,
        })
        .unwrap()
        .item;
    run(&mut s);
    let refreshed = s
        .assistance_local_preparation(&restored.item_id.to_string())
        .unwrap();
    assert_eq!(refreshed.state, AssistanceLocalPreparationState::Ready);
    assert!(preview(&mut s, &refreshed).is_ok());
}

#[test]
fn schema_six_upgrade_adds_empty_opt_in_tables_without_requeueing_or_enabling_existing_goals() {
    let (dir, mut s, i) = setup();
    run(&mut s);
    let work = s.assistance_item_work(&i.item_id.to_string()).unwrap();
    let reminder = s
        .set_assistance_reminder(SetAssistanceReminder {
            item_id: i.item_id,
            expected_revision: i.revision,
            expected_schedule_revision: None,
            remind_at: Some("2030-01-01T00:02:00Z".into()),
        })
        .unwrap();
    drop(s);
    let conn = raw(dir.path());
    conn.execute_batch("DROP TABLE assistance_local_templates; DROP TABLE assistance_preparation_preferences; UPDATE meta SET value='6' WHERE key='schema_version'").unwrap();
    drop(conn);
    let s = Session::open(dir.path());
    let meta = s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap();
    assert!(!meta.enabled);
    assert!(meta.template_id.is_none());
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string()).unwrap(),
        work
    );
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .schedule_revision,
        reminder.schedule_revision
    );
    assert_eq!(s.assistance_item(&i.item_id.to_string()).unwrap().item, i);
    drop(s);
    let s = Session::open(dir.path());
    assert_eq!(
        s.assistance_item_work(&i.item_id.to_string()).unwrap(),
        work
    );
    assert!(s
        .assistance_local_preparation(&i.item_id.to_string())
        .unwrap()
        .preference_revision
        .is_none());
    let conn = raw(dir.path());
    assert_eq!(
        conn.query_row(
            "SELECT value FROM meta WHERE key='schema_version'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        "7"
    );
}
