use super::{adapter::Harness, policy, replay, selector, semantic, types::*};

fn case(count: usize) -> InputCase {
    InputCase {
        case_id: "synthetic-selection".into(),
        task: TaskInput {
            scope: "合成任务".into(),
            intent: "确认当前合成安排".into(),
            constraints: "仅使用合成来源".into(),
        },
        candidates: (0..count)
            .map(|index| Candidate {
                id: format!("c{index:02}"),
                title: format!("合成来源{index}"),
                summary: format!("合成安排记录{index}，用于离线选择测试。"),
            })
            .collect(),
    }
}

fn rank(case: &InputCase) -> Vec<String> {
    case.candidates
        .iter()
        .map(|entry| entry.id.clone())
        .collect()
}

fn predictions(
    case: &InputCase,
    groups: Vec<Vec<Vec<usize>>>,
) -> (policy::Needs, policy::SupportMap) {
    let needs = policy::Needs {
        task_id: "t0".into(),
        needs: groups
            .iter()
            .enumerate()
            .map(|(index, _)| policy::Need {
                need_id: format!("n{index}"),
                description: format!("合成需求{index}"),
            })
            .collect(),
    };
    let support = policy::SupportMap {
        case_id: case.case_id.clone(),
        supports: groups
            .into_iter()
            .enumerate()
            .map(|(index, groups)| policy::Support {
                need_id: format!("n{index}"),
                groups: groups
                    .into_iter()
                    .map(|group| {
                        group
                            .into_iter()
                            .map(|i| case.candidates[i].id.clone())
                            .collect()
                    })
                    .collect(),
            })
            .collect(),
    };
    (needs, support)
}

#[test]
fn and_groups_are_whole_and_or_minimizes_new_sources_before_rank() {
    let case = case(12);
    let rank = rank(&case);
    let (needs, support) = predictions(&case, vec![vec![vec![0, 9]], vec![vec![1], vec![9, 0]]]);
    let plan = selector::select(&case, &needs, &support, &rank, false).unwrap();
    assert!(plan.available);
    assert_eq!(plan.requested_ids, vec![rank[0].clone(), rank[9].clone()]);
    assert_eq!(plan.chosen_groups.len(), 2);
    assert_eq!(
        plan.chosen_groups[1].source_ids,
        plan.chosen_groups[0].source_ids
    );
    assert_eq!(&plan.full_rank[..2], plan.requested_ids.as_slice());
    assert_eq!(plan.full_rank.len(), rank.len());
    assert!(!selector::groups_retained(&plan.chosen_groups, &rank[..1]));
    assert!(selector::groups_retained(
        &plan.chosen_groups,
        &plan.requested_ids
    ));
}

#[test]
fn or_ties_prefer_earliest_latest_rank_then_canonical_ids() {
    let case = case(12);
    let rank = rank(&case);
    let (needs, support) = predictions(
        &case,
        vec![vec![vec![5], vec![2]], vec![vec![1, 3], vec![0, 3]]],
    );
    let plan = selector::select(&case, &needs, &support, &rank, false).unwrap();
    assert_eq!(plan.chosen_groups[0].source_ids, vec![rank[2].clone()]);
    assert_eq!(
        plan.chosen_groups[1].source_ids,
        vec![rank[0].clone(), rank[3].clone()]
    );
}

#[test]
fn support_at_tenth_position_is_promoted_without_losing_full_tail() {
    let case = case(12);
    let rank = rank(&case);
    let (needs, support) = predictions(&case, vec![vec![vec![9]]]);
    let prefix = policy::decide(&case, &needs, &support, &rank).unwrap();
    assert!(!prefix.all_needs_satisfied);
    let plan = selector::select(&case, &needs, &support, &rank, false).unwrap();
    assert!(plan.available);
    assert_eq!(plan.requested_ids, vec![rank[9].clone()]);
    assert_eq!(plan.full_rank[0], rank[9]);
    let tail: Vec<_> = rank.iter().filter(|id| **id != rank[9]).cloned().collect();
    assert_eq!(&plan.full_rank[1..], tail.as_slice());
    let encoded = semantic::encode_ordered(
        std::slice::from_ref(&case),
        [(case.case_id.clone(), plan.full_rank.clone())]
            .into_iter()
            .collect(),
    )
    .unwrap();
    assert_eq!(
        semantic::parse(&encoded, std::slice::from_ref(&case)).unwrap()[&case.case_id],
        plan.full_rank
    );
}

#[test]
fn invalid_rank_tail_and_invalid_or_missing_support_are_rejected() {
    let case = case(12);
    let rank = rank(&case);
    let (needs, support) = predictions(&case, vec![vec![vec![0]]]);
    let mut bad_rank = rank.clone();
    bad_rank[11] = rank[10].clone();
    assert_eq!(
        selector::select(&case, &needs, &support, &bad_rank, false).err(),
        Some("rank_not_permutation")
    );
    let mut bad = support.clone();
    bad.supports[0].groups[0].push("outside".into());
    assert_eq!(
        selector::select(&case, &needs, &bad, &rank, false).err(),
        Some("support_group_ids")
    );
    bad = support.clone();
    bad.supports.clear();
    assert_eq!(
        selector::select(&case, &needs, &bad, &rank, false).err(),
        Some("support_need_set")
    );
    bad = support;
    bad.supports[0].groups = vec![vec![]];
    assert_eq!(
        selector::select(&case, &needs, &bad, &rank, false).err(),
        Some("support_group_ids")
    );
}

#[test]
fn missing_machine_support_is_unavailable_without_asserting_fact_absence() {
    let case = case(12);
    let rank = rank(&case);
    let (needs, support) = predictions(&case, vec![vec![vec![0]], vec![]]);
    let plan = selector::select(&case, &needs, &support, &rank, false).unwrap();
    assert!(!plan.available);
    assert_eq!(plan.reason, "mapping_unsatisfied");
    assert_eq!(plan.unsatisfied_need_ids, vec!["n1"]);
    assert_eq!(plan.requested_ids, vec![rank[0].clone()]);
    let value = serde_json::to_value(plan).unwrap();
    assert!(value.get("missing_facts").is_none());
    assert!(value.get("fact_coverage").is_none());
}

#[test]
fn eight_source_boundary_refuses_whole_nine_source_group_without_truncation() {
    let case = case(12);
    let rank = rank(&case);
    let (needs, support) = predictions(&case, vec![vec![(0..8).collect()]]);
    assert!(
        selector::select(&case, &needs, &support, &rank, false)
            .unwrap()
            .available
    );
    let (needs, support) = predictions(&case, vec![vec![(0..9).collect()]]);
    let plan = selector::select(&case, &needs, &support, &rank, false).unwrap();
    assert!(!plan.available);
    assert_eq!(plan.reason, "selection_budget");
    assert_eq!(plan.requested_ids.len(), 9);
    assert_eq!(plan.chosen_groups[0].source_ids.len(), 9);
    assert_eq!(plan.full_rank, rank);
}

#[test]
fn initial_four_union_deduplicates_and_obeys_same_budget() {
    let case = case(12);
    let rank = rank(&case);
    let (needs, support) = predictions(&case, vec![vec![vec![0, 9]]]);
    let plan = selector::select(&case, &needs, &support, &rank, true).unwrap();
    assert!(plan.available);
    assert_eq!(
        plan.requested_ids,
        vec![
            rank[0].clone(),
            rank[1].clone(),
            rank[2].clone(),
            rank[3].clone(),
            rank[9].clone()
        ]
    );
    let (needs, support) = predictions(&case, vec![vec![(4..9).collect()]]);
    let plan = selector::select(&case, &needs, &support, &rank, true).unwrap();
    assert!(!plan.available);
    assert_eq!(plan.reason, "selection_budget");
    assert_eq!(plan.requested_ids.len(), 9);
}

#[test]
fn real_session_preparation_keeps_promoted_and_group_without_network() {
    let case = case(12);
    let rank = rank(&case);
    let (needs, support) = predictions(&case, vec![vec![vec![0, 9]]]);
    let plan = selector::select(&case, &needs, &support, &rank, false).unwrap();
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    let observed = harness
        .observe_prefix(&snapshot, &plan.full_rank, plan.requested_ids.len(), true)
        .unwrap();
    assert_eq!(observed.selected.len(), 2);
    assert_eq!(observed.retained.len(), 2);
    assert!(selector::groups_retained(
        &plan.chosen_groups,
        &observed.retained
    ));
    assert_eq!(observed.metrics["network_requests"], 0);
    assert!(observed.metrics["encoded_bytes"].as_u64().unwrap() > 0);
    harness.validate(&snapshot).unwrap();
}

fn observation_row() -> serde_json::Value {
    serde_json::json!({"available":false,"reason":null,"attempted_prepare":false,
        "selected_ids":null,"retained_ids":null,"encoded_bytes":null,"preparation":null,
        "group_retained_complete":null,"fact_coverage":null})
}

#[test]
fn real_context_budget_loss_is_recorded_and_following_condition_still_runs() {
    let mut case = case(3);
    for candidate in &mut case.candidates {
        candidate.summary = format!("synthetic-{}:{}", candidate.id, "x".repeat(7_000));
    }
    case_bytes(&case).unwrap();
    let rank = rank(&case);
    let (needs, support) = predictions(&case, vec![vec![vec![0, 1, 2]]]);
    let selection = selector::select(&case, &needs, &support, &rank, false).unwrap();
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    let mut row = observation_row();
    let observed = replay::observe_row(&mut harness, &snapshot, &selection, &mut row)
        .expect("budget losses remain observable");
    assert!(observed.selected.len() < selection.requested_ids.len());
    assert_eq!(row["available"], false);
    assert_eq!(row["reason"], "serialized_selection_loss");
    assert_eq!(row["attempted_prepare"], true);
    assert_eq!(row["group_retained_complete"], false);
    assert!(row["requested_to_selected_loss"].as_u64().unwrap() > 0);
    assert_eq!(
        row["selected_ids"],
        serde_json::to_value(&observed.selected).unwrap()
    );
    assert_eq!(
        row["retained_ids"],
        serde_json::to_value(&observed.retained).unwrap()
    );
    assert_eq!(row["network_requests"], 0);
    assert!(row["fact_coverage"].is_null());
    harness.validate(&snapshot).unwrap();

    let (needs, support) = predictions(&case, vec![vec![vec![0]]]);
    let selection = selector::select(&case, &needs, &support, &rank, false).unwrap();
    let mut next = observation_row();
    let observed = replay::observe_row(&mut harness, &snapshot, &selection, &mut next)
        .expect("following normal condition prepares");
    assert_eq!(next["available"], true);
    assert_eq!(observed.retained, selection.requested_ids);
    assert_eq!(next["network_requests"], 0);
    harness.validate(&snapshot).unwrap();
}

#[test]
fn recoverable_observation_error_records_null_metrics_then_continues() {
    let case = case(2);
    let rank = rank(&case);
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    let selection = selector::Selection {
        available: true,
        reason: "fixed_prefix",
        full_rank: rank.clone(),
        requested_ids: Vec::new(),
        chosen_groups: Vec::new(),
        unsatisfied_need_ids: Vec::new(),
    };
    let mut row = observation_row();
    assert!(replay::observe_row(&mut harness, &snapshot, &selection, &mut row).is_none());
    assert_eq!(row["available"], false);
    assert_eq!(row["reason"], "prefix_count");
    assert_eq!(row["attempted_prepare"], true);
    for field in [
        "selected_ids",
        "retained_ids",
        "encoded_bytes",
        "preparation",
        "group_retained_complete",
        "fact_coverage",
    ] {
        assert!(row[field].is_null());
    }
    harness.validate(&snapshot).unwrap();
    let selection = selector::Selection {
        requested_ids: rank[..1].to_vec(),
        ..selection
    };
    let mut next = observation_row();
    assert!(replay::observe_row(&mut harness, &snapshot, &selection, &mut next).is_some());
    assert_eq!(next["available"], true);
    assert!(next["group_retained_complete"].is_null());
    assert_eq!(next["network_requests"], 0);
    harness.validate(&snapshot).unwrap();
}
