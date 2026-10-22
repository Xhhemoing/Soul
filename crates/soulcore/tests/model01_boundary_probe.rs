//! Finite synthetic observations for MODEL-01 design, not a personal model.
//! Run explicitly; no user data, real model, quality score, or production change.

use serde_json::{json, Value};
use soul_memory::{ExclusionReason, SelectionOutcome};
use soul_policy::e1::{DRAFTING_INSTRUCTION, QUOTE_CLOSE, QUOTE_OPEN};
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::memory::NewMemory;
use soulcore::commands::session::{MemoryContextRequest, Session};
use uuid::Uuid;

fn memory(session: &mut Session, title: &str, summary: &str) -> Uuid {
    let saved = session
        .write_memory(&NewMemory {
            memory_type: "preference".into(),
            title: title.into(),
            summary: summary.into(),
        })
        .expect("synthetic memory must be saved");
    Uuid::parse_str(&saved.memory_id).expect("saved memory must have an identity")
}

fn scope(ids: &[Uuid]) -> MemoryContextRequest {
    MemoryContextRequest {
        candidate_ids: ids.to_vec(),
        query: None,
        memory_type: None,
        time_specific: false,
    }
}

fn keyword_counterevidence() -> Value {
    let dir = tempfile::tempdir().expect("temporary synthetic store");
    let mut session = Session::open(dir.path());
    let open = memory(&mut session, "合成场所甲记录一", "合成场所甲当前开放。");
    let closed = memory(&mut session, "合成场所甲记录二", "合成场所甲已经关闭。");
    let outside = memory(
        &mut session,
        "范围外合成记录",
        "合成场所甲开放时间另有说明。",
    );

    let all = session
        .prepare_memory_context(scope(&[open, closed]))
        .expect("explicit synthetic scope");
    assert!(all.context_id.is_some());
    assert_eq!(all.preview.outcome, SelectionOutcome::Context);
    assert_eq!(all.preview.selected.len(), 2);
    assert!(all.preview.selected[0].memory_id == open);
    assert!(all.preview.selected[1].memory_id == closed);
    assert!(all.preview.selected.iter().all(|entry| {
        entry.citation.memory_id == entry.memory_id && entry.memory_id != outside
    }));
    assert!(all.preview.excluded.is_empty());
    assert!(!all.preview.notice.is_empty());

    let mut request = scope(&[open, closed]);
    request.query = Some("开放".into());
    let filtered = session
        .prepare_memory_context(request)
        .expect("keyword selection from the same scope");
    assert!(filtered.context_id.is_some());
    assert_eq!(filtered.preview.outcome, SelectionOutcome::Context);
    assert_eq!(filtered.preview.selected.len(), 1);
    assert!(filtered.preview.selected[0].memory_id == open);
    assert!(filtered.preview.selected[0].citation.memory_id == open);
    assert_eq!(filtered.preview.excluded.len(), 1);
    assert!(filtered.preview.excluded[0].memory_id == closed);
    assert_eq!(
        filtered.preview.excluded[0].reason,
        ExclusionReason::KeywordMismatch
    );
    assert!(filtered
        .preview
        .selected
        .iter()
        .all(|entry| entry.memory_id != outside));

    let mut timed_request = scope(&[open, closed]);
    timed_request.time_specific = true;
    let timed = session
        .prepare_memory_context(timed_request)
        .expect("unsupported fact time is represented explicitly");
    assert_eq!(timed.preview.outcome, SelectionOutcome::Clarify);
    assert!(timed.context_id.is_none());
    assert!(timed.preview.selected.is_empty());

    // The contradiction is fixture knowledge, not an inferred runtime label.
    // These observations cannot determine which statement is actually true.
    json!({
        "scenario": "keyword_counterevidence",
        "candidate_count": 2,
        "unfiltered_selected": all.preview.selected.len(),
        "filtered_selected": filtered.preview.selected.len(),
        "filtered_excluded": filtered.preview.excluded.len(),
        "exclusion_reason": "keyword_mismatch",
        "out_of_scope_selected": false,
        "fact_time_outcome": "clarify",
        "fact_time_context_available": timed.context_id.is_some(),
        "observation": "lexical_filter_can_remove_fixture_counterevidence",
        "semantic_conflict_detection_measured": false,
        "truth_or_quality_measured": false
    })
}

fn profile_and_memory_material() -> Value {
    let dir = tempfile::tempdir().expect("temporary synthetic store");
    let endpoint = MockLlm::start().expect("controlled loopback mock");
    let mut session = Session::open(dir.path());
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("explicit loopback endpoint");
    let pinned = session
        .set_voice("register", "formal")
        .expect("user explicitly pins formal register");
    let register = pinned
        .voice
        .fields
        .iter()
        .find(|field| field.field == "register")
        .expect("register is a supported voice field");
    assert!(register.value == "formal" && register.locked_by_user);

    const CASUAL_NOTE: &str = "合成记忆建议：这次请用随意口吻。";
    let id = memory(&mut session, "合成语气记录", CASUAL_NOTE);
    let context = session
        .prepare_memory_context(scope(&[id]))
        .expect("synthetic owner memory selection")
        .context_id
        .expect("readable owner memory has a local context");
    let prepared = session
        .prepare_draft_with_memory("合成安排请求", &context.to_string(), &[id], false)
        .expect("separately select the owner excerpt for this exact request");
    assert!(prepared.prepared.retained_memory_ids == [id]);
    assert!(prepared.prepared.endpoint_owner_ids == [id]);
    assert_eq!(endpoint.request_count(), 0);

    let approval = prepared.prepared.plan.approval();
    let completed = session
        .begin_draft(&approval)
        .expect("explicit exact-plan approval")
        .execute();
    session
        .complete_draft(completed)
        .expect("accept the controlled endpoint completion");
    let requests = endpoint.requests();
    assert_eq!(requests.len(), 1);
    let body: Value =
        serde_json::from_str(&requests[0].body).expect("actual loopback request is a JSON object");
    let messages = body["messages"]
        .as_array()
        .expect("actual request has messages");
    assert_eq!(messages.len(), 2);
    assert!(messages[0]["role"] == "system");
    assert!(messages[0]["content"] == DRAFTING_INSTRUCTION);
    assert!(messages[1]["role"] == "user");
    let material = messages[1]["content"]
        .as_str()
        .expect("quoted material is text");
    assert!(material.starts_with(QUOTE_OPEN) && material.ends_with(QUOTE_CLOSE));
    let (profile_material, memory_material) = material
        .split_once("本次明确提供的记忆参考（保存的记录，未经事实或当前性核验）：")
        .expect("memory remains a separately labelled data section");
    assert!(profile_material.contains("【本机档案，供起草参考】"));
    assert!(profile_material.contains("语气：正式"));
    assert!(!profile_material.contains(CASUAL_NOTE));
    assert!(memory_material.contains(CASUAL_NOTE));
    let after = session
        .profile()
        .expect("read persisted profile after draft");
    assert!(after.voice == pinned.voice);
    assert!(session.begin_draft(&approval).is_err());
    assert_eq!(endpoint.request_count(), 1);

    // Both the profile and memory are quoted data, not two system instructions.
    // A fixed mock response says nothing about a real model's obedience.
    json!({
        "scenario": "profile_and_memory_material",
        "approved_requests": requests.len(),
        "profile_register": "formal",
        "profile_lock_preserved": true,
        "profile_material_and_memory_separate": true,
        "memory_in_instruction_position": false,
        "replayed_approval_requests": 0,
        "observation": "conflicting_memory_does_not_rewrite_profile_or_system_instruction",
        "model_obedience_measured": false,
        "truth_or_quality_measured": false
    })
}

#[test]
#[ignore = "explicit finite synthetic MODEL-01 design probe; no real model or user evaluation"]
fn report() {
    let observations = [keyword_counterevidence(), profile_and_memory_material()];
    // Emit only fixed labels, counts, and booleans, never the input or request.
    println!(
        "{}",
        json!({
            "synthetic": true,
            "human_evaluation": false,
            "production_model_implemented": false,
            "observations": observations
        })
    );
}
