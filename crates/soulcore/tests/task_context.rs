//! Real Session and encrypted temporary-store task context regression.
use soulcore::commands::memory::NewMemory;
use soulcore::commands::session::{Session, TaskContextRequest, TaskRequirementStatus, TaskStatus};
use uuid::Uuid;

fn memory(session: &mut Session, summary: &str) -> Uuid {
    let row = session
        .write_memory(&NewMemory {
            memory_type: "preference".into(),
            title: "synthetic task material".into(),
            summary: summary.into(),
        })
        .unwrap();
    Uuid::parse_str(&row.memory_id).unwrap()
}
fn request(ids: &[Uuid], task: &str) -> TaskContextRequest {
    TaskContextRequest {
        candidate_ids: ids.to_vec(),
        task: task.into(),
    }
}

#[test]
fn explicit_fields_match_without_question_and_local_use_stays_single_use() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let id = memory(&mut session, "时间：九点\n地点：北门");
    let view = session
        .prepare_task_context(request(&[id], "周五会议\n需要确认：时间、地点"))
        .unwrap();
    assert_eq!(view.assessment.status, TaskStatus::Matched);
    assert!(view.assessment.question.is_none());
    assert_eq!(view.assessment.requirements.len(), 2);
    assert!(view
        .assessment
        .requirements
        .iter()
        .all(|r| r.status == TaskRequirementStatus::Matched && r.source_ids == vec![id]));
    let context = view.memory.context_id.unwrap().to_string();
    assert!(session
        .draft_pasted_with_memory("synthetic", &context)
        .is_ok());
    assert!(session
        .draft_pasted_with_memory("synthetic", &context)
        .is_err());
}

#[test]
fn a_tenth_candidate_can_supply_the_one_bounded_supplement() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let mut ids: Vec<_> = (0..9)
        .map(|_| memory(&mut session, "备注：合成材料"))
        .collect();
    let support = memory(&mut session, "时间：九点");
    ids.push(support);
    let view = session
        .prepare_task_context(request(&ids, "需要确认：时间"))
        .unwrap();
    assert_eq!(view.assessment.status, TaskStatus::Matched);
    assert_eq!(view.assessment.supplemented_ids, vec![support]);
    assert!(view
        .memory
        .preview
        .selected
        .iter()
        .any(|row| row.memory_id == support));
    assert!(view.memory.preview.selected.len() <= 8);
}

#[test]
fn explicit_missing_field_asks_exactly_one_question_and_mints_no_context() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let id = memory(&mut session, "时间：九点");
    let view = session
        .prepare_task_context(request(&[id], "需要确认：时间、地点"))
        .unwrap();
    assert_eq!(view.assessment.status, TaskStatus::Clarify);
    assert_eq!(
        view.assessment.requirements[1].status,
        TaskRequirementStatus::Missing
    );
    assert!(view.assessment.question.as_ref().unwrap().contains("地点"));
    assert!(view.memory.context_id.is_none());
}

#[test]
fn unequal_literal_values_are_a_conflict_instead_of_a_sufficiency_claim() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let a = memory(&mut session, "时间：九点");
    let b = memory(&mut session, "时间：十点");
    let view = session
        .prepare_task_context(request(&[a, b], "需要确认：时间"))
        .unwrap();
    assert_eq!(view.assessment.status, TaskStatus::Clarify);
    assert_eq!(
        view.assessment.requirements[0].status,
        TaskRequirementStatus::Conflict
    );
    assert_eq!(view.assessment.requirements[0].source_ids, vec![a, b]);
    assert!(view.memory.context_id.is_none());
}

#[test]
fn narrative_or_unparsed_required_material_is_unknown_without_question() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let id = memory(&mut session, "时间可能是九点，也可能更晚");
    for task in ["帮我确认见面安排", "需要确认：时间"] {
        let view = session.prepare_task_context(request(&[id], task)).unwrap();
        assert_eq!(view.assessment.status, TaskStatus::Unknown);
        assert!(view.assessment.question.is_none());
        assert!(view.memory.context_id.is_none());
    }
}

#[test]
fn too_many_requirements_is_budget_limited_without_a_question() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let id = memory(&mut session, "甲：值");
    let view = session
        .prepare_task_context(request(
            &[id],
            "需要确认：甲、乙、丙、丁、戊、己、庚、辛、壬",
        ))
        .unwrap();
    assert_eq!(view.assessment.status, TaskStatus::BudgetLimited);
    assert!(view.assessment.question.is_none());
    assert!(view.memory.context_id.is_none());
}

#[test]
fn pool_is_explicit_and_a_new_unknown_task_invalidates_old_context() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let id = memory(&mut session, "时间：九点");
    let outside = memory(&mut session, "时间：十点");
    let view = session
        .prepare_task_context(request(&[id], "需要确认：时间"))
        .unwrap();
    assert_eq!(view.assessment.requirements[0].source_ids, vec![id]);
    assert!(!view.assessment.requirements[0]
        .source_ids
        .contains(&outside));
    let old = view.memory.context_id.unwrap().to_string();
    let unknown = session
        .prepare_task_context(request(&[id], "帮我安排见面"))
        .unwrap();
    assert_eq!(unknown.assessment.status, TaskStatus::Unknown);
    assert!(session.draft_pasted_with_memory("synthetic", &old).is_err());
    assert!(session
        .prepare_task_context(request(&[Uuid::now_v7()], "需要确认：时间"))
        .is_err());
}

fn direct_edit(session: &Session, id: Uuid) {
    soul_memory::update(
        &mut *session.store().unwrap().lock().unwrap(),
        id,
        &soul_memory::MemoryEdit {
            title: None,
            summary: Some("备注：更改后的合成材料".into()),
            memory_type: None,
        },
        1_800_000_000,
    )
    .unwrap();
}

#[test]
fn every_pool_snapshot_is_revalidated_at_prepare_dispatch_and_completion() {
    use soul_testkit::mock_llm::MockLlm;
    for stage in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let mut session = Session::open(dir.path());
        let endpoint = MockLlm::start().unwrap();
        session.set_user_endpoint(&endpoint.base_url()).unwrap();
        let support = memory(&mut session, "时间：九点");
        let mut ids = vec![support];
        ids.extend((0..5).map(|_| memory(&mut session, "备注：合成材料")));
        let unused = ids[5];
        let view = session
            .prepare_task_context(request(&ids, "需要确认：时间"))
            .unwrap();
        assert_eq!(view.assessment.status, TaskStatus::Matched);
        assert!(!view
            .memory
            .preview
            .selected
            .iter()
            .any(|row| row.memory_id == unused));
        let context = view.memory.context_id.unwrap().to_string();
        if stage == 0 {
            direct_edit(&session, unused);
            assert!(session
                .prepare_draft_with_memory("synthetic", &context, &[], false)
                .is_err());
        } else {
            let plan = session
                .prepare_draft_with_memory("synthetic", &context, &[], false)
                .unwrap();
            if stage == 1 {
                direct_edit(&session, unused);
                assert!(session.begin_draft(&plan.prepared.plan.approval()).is_err());
            } else {
                let completed = session
                    .begin_draft(&plan.prepared.plan.approval())
                    .unwrap()
                    .execute();
                direct_edit(&session, unused);
                assert!(session.complete_draft(completed).is_err());
            }
        }
        assert_eq!(endpoint.request_count(), usize::from(stage == 2));
        assert!(!session
            .audit()
            .unwrap()
            .entries
            .iter()
            .any(|row| row.action == "draft.create"));
    }
}

#[test]
fn final_http_budget_cannot_silently_drop_a_required_support() {
    use soul_testkit::mock_llm::MockLlm;
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    let a = memory(&mut session, &format!("甲：{}", "x".repeat(6_000)));
    let b = memory(&mut session, &format!("乙：{}", "y".repeat(6_000)));
    let view = session
        .prepare_task_context(request(&[a, b], "需要确认：甲、乙"))
        .unwrap();
    assert_eq!(view.assessment.status, TaskStatus::Matched);
    let context = view.memory.context_id.unwrap().to_string();
    let refusal = session
        .prepare_draft_with_memory(&"z".repeat(58_000), &context, &[a, b], true)
        .unwrap_err();
    assert_eq!(refusal.reason_code, "MEMORY_CONTEXT_CHANGED");
    assert_eq!(endpoint.request_count(), 0);
}

#[test]
fn partial_or_conditional_pool_never_turns_uncertainty_into_a_missing_question() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let clear = memory(&mut session, "时间：九点");
    for summary in [
        "地点可能在北门",
        "地点：如果晴天则北门",
        "地点：",
        "地点：北门或南门",
    ] {
        let uncertain = memory(&mut session, summary);
        let view = session
            .prepare_task_context(request(&[clear, uncertain], "需要确认：时间、地点"))
            .unwrap();
        assert_eq!(view.assessment.status, TaskStatus::Unknown, "{summary}");
        assert!(view.assessment.question.is_none());
        assert!(view.memory.context_id.is_none());
    }
}

#[test]
fn unsupported_task_constraints_and_multiple_instructions_are_unknown() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let id = memory(&mut session, "时间：九点");
    for task in [
        "不需要地点\n需要确认：地点",
        "如果晴天\n需要确认：地点",
        "仅时间\n需要确认：时间、地点",
        "需要确认：时间\n需要确认：地点",
    ] {
        let view = session.prepare_task_context(request(&[id], task)).unwrap();
        assert_eq!(view.assessment.status, TaskStatus::Unknown, "{task}");
        assert!(view.assessment.question.is_none());
        assert!(view.memory.context_id.is_none());
    }
    let oversized = vec![id; 33];
    assert!(session
        .prepare_task_context(request(&oversized, "需要确认：时间"))
        .is_err());
}

#[test]
fn repeated_equal_values_need_one_retained_support_and_keep_all_diagnostic_sources() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let ids: Vec<_> = (0..10)
        .map(|_| memory(&mut session, "时间：九点"))
        .collect();
    let view = session
        .prepare_task_context(request(&ids, "需要确认：时间"))
        .unwrap();
    assert_eq!(view.assessment.status, TaskStatus::Matched);
    assert_eq!(view.assessment.requirements[0].source_ids, ids);
    assert_eq!(view.memory.preview.selected.len(), 4);
    assert!(view.assessment.supplemented_ids.is_empty());
    assert!(view.memory.context_id.is_some());
}
