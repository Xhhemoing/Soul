//! Test-only task-aware retrieval adapter and finite offline semantic replay.
#[path = "task_retrieval_probe/adapter.rs"]
mod adapter;
#[path = "task_retrieval_probe/cards.rs"]
mod cards;
#[path = "task_retrieval_probe/ranking.rs"]
mod ranking;
#[path = "task_retrieval_probe/replay.rs"]
mod replay;
#[path = "task_retrieval_probe/semantic.rs"]
mod semantic;
#[path = "task_retrieval_probe/types.rs"]
mod types;

use soul_store_api::{ForgetOps, ForgetUnit};
use soulcore::commands::session::{AssistanceAction, AssistanceActionRequest, EditAssistanceGoal};
use types::{Candidate, InputCase, TaskInput};

fn synthetic_case() -> InputCase {
    InputCase {
        case_id: "parser_case".into(),
        task: TaskInput {
            scope: "合成对象事项".into(),
            intent: "选择蓝色封面校对".into(),
            constraints: "不提红色印刷采购".into(),
        },
        candidates: (0..32)
            .map(|index| Candidate {
                id: format!("r{index:02}"),
                title: "合成对象事项".into(),
                summary: format!("合成记录内容{index}"),
            })
            .collect(),
    }
}

#[test]
fn full_task_changes_lexical_order_while_scope_ablation_stays_identical() {
    let mut first = synthetic_case();
    first.task.intent = "蓝色封面校对".into();
    first.task.constraints = "核对蓝色封面校对备注".into();
    first.candidates[0].summary = "蓝色封面校对备注".into();
    first.candidates[1].summary = "红色印刷采购报价".into();
    let mut second = first.clone();
    second.task.intent = "红色印刷采购".into();
    second.task.constraints = "核对红色印刷采购报价".into();
    assert!(ranking::rank(&first, false) == ranking::rank(&second, false));
    let a = ranking::rank(&first, true);
    let b = ranking::rank(&second, true);
    assert!(a != b);
    assert!(a[0] == first.candidates[0].id);
    assert!(b[0] == first.candidates[1].id);
    assert_eq!(a.len(), 32);
    assert_eq!(b.len(), 32);
}

#[test]
fn semantic_parser_accepts_only_exact_complete_bounded_permutations() {
    let case = synthetic_case();
    let valid = serde_json::json!([{ "case_id": case.case_id, "ranked_ids": case.candidates.iter().map(|entry| &entry.id).collect::<Vec<_>>() }]);
    let expected = [case];
    assert!(semantic::parse(&serde_json::to_vec(&valid).unwrap(), &expected).is_ok());
    let mut bad = valid.clone();
    bad[0]["ranked_ids"][0] = "unknown_id".into();
    assert!(semantic::parse(&serde_json::to_vec(&bad).unwrap(), &expected).is_err());
    let mut bad = valid.clone();
    bad[0]["ranked_ids"][0] = bad[0]["ranked_ids"][1].clone();
    assert!(semantic::parse(&serde_json::to_vec(&bad).unwrap(), &expected).is_err());
    let mut bad = valid.clone();
    bad[0]["ranked_ids"].as_array_mut().unwrap().pop();
    assert!(semantic::parse(&serde_json::to_vec(&bad).unwrap(), &expected).is_err());
    let mut bad = valid.clone();
    bad[0]["unexpected"] = true.into();
    assert!(semantic::parse(&serde_json::to_vec(&bad).unwrap(), &expected).is_err());
    let mut bad = valid.clone();
    bad[0]["case_id"] = "unknown_case".into();
    assert!(semantic::parse(&serde_json::to_vec(&bad).unwrap(), &expected).is_err());
    assert!(semantic::parse(b"not json", &expected).is_err());
    assert!(semantic::parse(&vec![b' '; semantic::MAX_RESPONSE_BYTES + 1], &expected).is_err());
    assert!(semantic::parse(b"[]", &expected).is_err());
    let mut second = expected[0].clone();
    second.case_id = "second_case".into();
    let duplicated = serde_json::json!([valid[0].clone(), valid[0].clone()]);
    assert!(semantic::parse(
        &serde_json::to_vec(&duplicated).unwrap(),
        &[expected[0].clone(), second]
    )
    .is_err());
    let mut oversized = expected[0].clone();
    oversized.candidates[0].summary = "x".repeat(types::MAX_CASE_BYTES);
    assert!(types::case_bytes(&oversized).is_err());
}

#[test]
fn task_is_saved_and_read_back_from_current_manual_goal() {
    let case = synthetic_case();
    let harness = adapter::Harness::new(&case);
    let current = harness
        .session
        .assistance_item(&harness.goal.item.item_id.to_string())
        .unwrap();
    let stored: TaskInput = serde_json::from_str(&current.goal.unwrap().summary).unwrap();
    assert!(stored == case.task);
    let snapshot = harness
        .snapshot()
        .expect("capture current task and all candidates");
    assert!(harness.validate(&snapshot).is_ok());
}

#[test]
fn semantic_return_window_rejects_goal_edit_cancel_forget_and_any_candidate_change() {
    for change in ["edit", "cancel", "forget", "candidate"] {
        let mut harness = adapter::Harness::new(&synthetic_case());
        let snapshot = harness
            .snapshot()
            .expect("snapshot before delayed semantic result");
        let item = &harness.goal.item;
        match change {
            "edit" => {
                harness
                    .session
                    .edit_assistance_goal(EditAssistanceGoal {
                        item_id: item.item_id,
                        expected_revision: item.revision,
                        title: "合成对象事项".into(),
                        summary: "修改后的意图".into(),
                        deadline_at: None,
                    })
                    .unwrap();
            }
            "cancel" => {
                harness
                    .session
                    .act_assistance_item(AssistanceActionRequest {
                        item_id: item.item_id,
                        expected_revision: item.revision,
                        action: AssistanceAction::Cancel,
                    })
                    .unwrap();
            }
            "forget" => {
                harness
                    .session
                    .store()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .execute_forget(ForgetUnit::Memory(harness.goal_memory_id()))
                    .unwrap();
            }
            "candidate" => {
                let shared = harness.session.store().unwrap();
                soul_memory::update(
                    &mut *shared.lock().unwrap(),
                    harness.ids[31],
                    &soul_memory::MemoryEdit {
                        title: None,
                        summary: Some("池尾记录发生变化".into()),
                        memory_type: None,
                    },
                    1_800_000_000,
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        assert!(harness.validate(&snapshot).is_err());
    }
}

#[test]
fn fixture_truth_is_separate_and_orders_change_only_candidate_order() {
    use std::collections::BTreeSet;
    let forward = cards::cards(false);
    let reverse = cards::cards(true);
    assert_eq!(forward.len(), 12);
    assert_eq!(
        forward
            .iter()
            .map(|card| &card.input.case_id)
            .collect::<BTreeSet<_>>()
            .len(),
        12
    );
    for (index, (card, reversed)) in forward.iter().zip(&reverse).enumerate() {
        assert!(card.input.task == reversed.input.task);
        assert!(card
            .input
            .candidates
            .iter()
            .rev()
            .eq(&reversed.input.candidates));
        let ids: BTreeSet<_> = card
            .input
            .candidates
            .iter()
            .map(|entry| entry.id.as_str())
            .collect();
        assert_eq!(ids.len(), 32);
        assert!((2..=3).contains(&card.gold.required.len()));
        assert!(card
            .gold
            .required
            .iter()
            .all(|entry| ids.contains(entry.id.as_str()) && !entry.necessity.is_empty()));
        let categories: Vec<_> = card
            .gold
            .required
            .iter()
            .chain(&card.gold.supporting)
            .chain(&card.gold.not_applicable)
            .collect();
        assert_eq!(categories.len(), 32);
        assert_eq!(
            categories
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<BTreeSet<_>>(),
            ids
        );
        assert!(categories.iter().all(|entry| !entry.necessity.is_empty()));
        assert!(card.gold.other_task_required.iter().all(|id| !card
            .gold
            .required
            .iter()
            .any(|entry| &entry.id == id)));
        assert!(types::case_bytes(&card.input).is_ok());
        if index % 2 == 0 {
            assert!(card.input.candidates == forward[index + 1].input.candidates);
            assert!(card.input.task.scope == forward[index + 1].input.task.scope);
            assert!(card.input.task.intent != forward[index + 1].input.task.intent);
        }
    }
}

#[test]
fn retained_export_requires_exact_task_content_and_candidate_order_without_gold() {
    let expected = vec![synthetic_case()];
    let bytes = replay::encode_inputs(expected.clone());
    assert!(replay::retained_inputs_match(&bytes, &expected));
    let mut altered = expected.clone();
    altered[0].task.intent.push_str("变化");
    assert!(!replay::retained_inputs_match(&bytes, &altered));
    let mut altered = expected.clone();
    altered[0].candidates.swap(0, 1);
    assert!(!replay::retained_inputs_match(&bytes, &altered));
    let mut altered = expected.clone();
    altered[0].candidates[31].summary.push_str("变化");
    assert!(!replay::retained_inputs_match(&bytes, &altered));
    let encoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mut keys: Vec<_> = encoded["cases"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert!(keys == ["candidates", "case_id", "task"]);
}

#[test]
fn local_task_and_candidate_prose_receive_no_implicit_endpoint_permission() {
    let mut harness = adapter::Harness::new(&synthetic_case());
    let snapshot = harness.snapshot().unwrap();
    let ranked = ranking::rank(&snapshot.input, true);
    let result = harness.observe(&snapshot, &ranked, false).unwrap();
    assert_eq!(result.selected.len(), 8);
    assert!(result.metrics["owner_excerpts_explicitly_allowed"] == false);
    assert!(result.metrics["network_requests"] == 0);
}

#[test]
#[ignore = "export fixed synthetic inputs to the explicitly selected external directory"]
fn export_inputs() {
    replay::export();
}

#[test]
#[ignore = "replay retained external rankings; creates no model calls"]
fn replay_predictions() {
    replay::replay();
}

#[test]
fn partition_response_composition_preserves_original_case_order_and_exact_ranks() {
    let first = synthetic_case();
    let mut second = first.clone();
    second.case_id = "another_case".into();
    let cases = [first, second];
    let ranks: Vec<_> = cases[0]
        .candidates
        .iter()
        .rev()
        .map(|entry| entry.id.clone())
        .collect();
    // Opposite to source case order; lexical-map iteration would also be wrong.
    let response = serde_json::json!([
        { "case_id": cases[1].case_id, "ranked_ids": ranks },
        { "case_id": cases[0].case_id, "ranked_ids": ranks }
    ]);
    let parsed = semantic::parse(&serde_json::to_vec(&response).unwrap(), &cases).unwrap();
    let composed = semantic::encode_ordered(&cases, parsed).unwrap();
    let actual: serde_json::Value = serde_json::from_slice(&composed).unwrap();
    assert!(actual[0]["case_id"] == cases[0].case_id);
    assert!(actual[1]["case_id"] == cases[1].case_id);
    assert!(actual[0]["ranked_ids"] == response[1]["ranked_ids"]);
    assert!(actual[1]["ranked_ids"] == response[0]["ranked_ids"]);
    assert!(semantic::parse(&composed, &cases).is_ok());
}

#[test]
#[ignore = "strictly compose retained partition responses into new derived group files"]
fn compose_partitioned_responses() {
    replay::compose_partitions();
}
