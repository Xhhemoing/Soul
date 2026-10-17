use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::memory::{MemoryChange, NewMemory};
use soulcore::commands::session::{MemoryContextRequest, Session};
use uuid::Uuid;

fn memory(session: &mut Session) -> Uuid {
    let value = session
        .write_memory(&NewMemory {
            memory_type: "preference".into(),
            title: "synthetic owner preference".into(),
            summary: "synthetic local note".into(),
        })
        .unwrap();
    Uuid::parse_str(&value.memory_id).unwrap()
}
fn request(id: Uuid) -> MemoryContextRequest {
    MemoryContextRequest {
        candidate_ids: vec![id],
        query: None,
        memory_type: None,
        time_specific: false,
    }
}

#[test]
fn clarified_or_empty_selection_cannot_mint_a_memory_grant_and_local_use_consumes_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let id = memory(&mut session);
    let mut timed = request(id);
    timed.time_specific = true;
    let clarified = session.prepare_memory_context(timed).unwrap();
    assert!(clarified.context_id.is_none());
    assert_eq!(
        clarified.preview.outcome,
        soul_memory::SelectionOutcome::Clarify
    );
    let selected = session.prepare_memory_context(request(id)).unwrap();
    let context = selected.context_id.unwrap().to_string();
    let plain = session.draft_pasted("synthetic paste").unwrap();
    let drafted = session
        .draft_pasted_with_memory("synthetic paste", &context)
        .unwrap();
    assert_eq!(drafted.draft.text, plain.text);
    assert_eq!(drafted.context.selected[0].citation.memory_id, id);
    assert!(session.draft_pasted_with_memory("next", &context).is_err());
}

#[test]
fn owner_ids_cannot_escape_selection_and_an_old_approval_cannot_consume_a_new_context() {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = MockLlm::start().unwrap();
    let mut session = Session::open(dir.path());
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let id = memory(&mut session);
    let first = session
        .prepare_memory_context(request(id))
        .unwrap()
        .context_id
        .unwrap()
        .to_string();
    assert!(session
        .prepare_draft_with_memory("synthetic", &first, &[Uuid::now_v7()], false)
        .is_err());
    let plan = session
        .prepare_draft_with_memory("synthetic", &first, &[id], false)
        .unwrap();
    assert_eq!(plan.prepared.retained_memory_ids, vec![id]);
    assert_eq!(plan.prepared.endpoint_owner_ids, vec![id]);
    assert!(plan.prepared.body_preview.contains("synthetic local note"));
    let second = session
        .prepare_memory_context(request(id))
        .unwrap()
        .context_id
        .unwrap()
        .to_string();
    assert!(session.begin_draft(&plan.prepared.plan.approval()).is_err());
    assert_eq!(endpoint.request_count(), 0);
    assert!(session.draft_pasted_with_memory("new", &second).is_ok());
}

#[test]
fn endpoint_completion_revalidates_actual_snapshot_even_for_direct_store_edits() {
    let dir = tempfile::tempdir().unwrap();
    let endpoint = MockLlm::start().unwrap();
    let mut session = Session::open(dir.path());
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let id = memory(&mut session);
    let context = session
        .prepare_memory_context(request(id))
        .unwrap()
        .context_id
        .unwrap()
        .to_string();
    let plan = session
        .prepare_draft_with_memory("synthetic", &context, &[], false)
        .unwrap();
    assert!(!plan.prepared.body_preview.contains("synthetic local note"));
    let completed = session
        .begin_draft(&plan.prepared.plan.approval())
        .unwrap()
        .execute();
    let store = session.store().unwrap();
    soul_memory::update(
        &mut *store.lock().unwrap(),
        id,
        &soul_memory::MemoryEdit {
            title: None,
            summary: Some("changed direct store".into()),
            memory_type: None,
        },
        1_800_000_000,
    )
    .unwrap();
    assert!(session.complete_draft(completed).is_err());
    assert_eq!(endpoint.request_count(), 1);
    let audit = session.audit().unwrap();
    assert!(audit
        .entries
        .iter()
        .any(|entry| entry.action == "egress.request"));
    assert!(!audit
        .entries
        .iter()
        .any(|entry| entry.action == "draft.create"));
}

#[test]
fn ordinary_edits_and_forget_destroy_the_local_context_instead_of_reclassifying_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let id = memory(&mut session);
    let context = session
        .prepare_memory_context(request(id))
        .unwrap()
        .context_id
        .unwrap()
        .to_string();
    session
        .edit_memory(
            &id.to_string(),
            &MemoryChange {
                title: None,
                summary: Some("changed".into()),
                memory_type: None,
            },
        )
        .unwrap();
    assert!(session
        .draft_pasted_with_memory("synthetic", &context)
        .is_err());
    let selected = session
        .prepare_memory_context(request(id))
        .unwrap()
        .context_id
        .unwrap()
        .to_string();
    let preview = session.preview_forget(&id.to_string()).unwrap();
    session
        .forget_memory(&soulcore::commands::memory::ForgetConfirmation {
            preview_id: preview.preview_id,
            memory_id: id.to_string(),
        })
        .unwrap();
    assert!(session
        .draft_pasted_with_memory("synthetic", &selected)
        .is_err());
    assert!(session
        .prepare_memory_context(request(id))
        .unwrap()
        .context_id
        .is_none());
}
