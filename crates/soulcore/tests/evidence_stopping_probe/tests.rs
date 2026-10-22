use super::{adapter::Harness, artifacts, fixtures, policy, ranking, semantic};

fn needs() -> policy::Needs {
    policy::parse_needs(br#"{"task_id":"t0","needs":[{"need_id":"n0","description":"first fact"},{"need_id":"n1","description":"second fact"}]}"#, "t0").unwrap()
}

#[test]
fn and_or_uses_earliest_complete_group_and_retains_cap_uncertainty() {
    let case = fixtures::cases().remove(1).input;
    let rank: Vec<_> = case
        .candidates
        .iter()
        .map(|entry| entry.id.clone())
        .collect();
    let needs = needs();
    let raw = serde_json::json!({"case_id":case.case_id,"supports":[
        {"need_id":"n0","groups":[[rank[0],rank[2]],[rank[4]]]},
        {"need_id":"n1","groups":[[rank[1]]]}
    ]});
    let support = policy::parse_support(&serde_json::to_vec(&raw).unwrap(), &case, &needs).unwrap();
    let decision = policy::decide(&case, &needs, &support, &rank).unwrap();
    assert_eq!(decision.requested_k, 3);
    assert!(decision.all_needs_satisfied);
    assert!(
        decision.needs[0].triggering_group.as_ref().unwrap()
            == &vec![rank[0].clone(), rank[2].clone()]
    );
    assert_eq!(decision.needs[1].first_satisfied_at, Some(2));
    let mut alternative_rank = rank.clone();
    alternative_rank.swap(0, 4);
    let alternative = policy::decide(&case, &needs, &support, &alternative_rank).unwrap();
    assert_eq!(alternative.requested_k, 2);
    let mut impossible = support.clone();
    impossible.supports[0].groups.clear();
    let capped = policy::decide(&case, &needs, &impossible, &rank).unwrap();
    assert_eq!(capped.requested_k, 8);
    assert_eq!(capped.reason, "budget_cap");
    assert!(!capped.all_needs_satisfied);
    assert!(capped.needs[0].first_satisfied_at.is_none());
    let mut bad_rank = rank.clone();
    bad_rank[13] = rank[12].clone();
    assert!(policy::decide(&case, &needs, &support, &bad_rank).is_err());
    let encoded = semantic::encode_ordered(
        std::slice::from_ref(&case),
        [(case.case_id.clone(), rank)].into_iter().collect(),
    )
    .unwrap();
    assert!(semantic::parse(&encoded, std::slice::from_ref(&case)).is_ok());
}

#[test]
fn machine_parser_rejects_empty_needs_bad_ids_missing_rows_and_invalid_groups() {
    for raw in [
        r#"{"task_id":"t0","needs":[]}"#,
        r#"{"task_id":"t0","needs":[{"need_id":"n0","description":" "}]}"#,
        r#"{"task_id":"t0","needs":[{"need_id":"n0","description":"a"},{"need_id":"n0","description":"b"}]}"#,
        r#"{"task_id":"wrong","needs":[{"need_id":"n0","description":"a"}]}"#,
        r#"{"task_id":"t0","needs":[{"need_id":"n0","description":"a","extra":0}]}"#,
    ] {
        assert!(policy::parse_needs(raw.as_bytes(), "t0").is_err());
    }
    let long = serde_json::json!({"task_id":"t0","needs":[{"need_id":"n0","description":"字".repeat(241)}]});
    assert!(policy::parse_needs(&serde_json::to_vec(&long).unwrap(), "t0").is_err());
    assert!(policy::parse_needs(&vec![b' '; policy::MAX_PREDICTION_BYTES + 1], "t0").is_err());
    let case = fixtures::cases().remove(1).input;
    let needs = needs();
    let id = &case.candidates[0].id;
    let valid = serde_json::json!({"case_id":case.case_id,"supports":[{"need_id":"n0","groups":[[id]]},{"need_id":"n1","groups":[]}]});
    assert!(policy::parse_support(&serde_json::to_vec(&valid).unwrap(), &case, &needs).is_ok());
    for mode in [
        "missing",
        "duplicate",
        "empty_group",
        "unknown_source",
        "unknown_need",
        "unknown_field",
        "duplicate_group",
    ] {
        let mut bad = valid.clone();
        match mode {
            "missing" => {
                bad["supports"].as_array_mut().unwrap().pop();
            }
            "duplicate" => bad["supports"][0]["groups"][0] = serde_json::json!([id, id]),
            "empty_group" => bad["supports"][0]["groups"][0] = serde_json::json!([]),
            "unknown_source" => bad["supports"][0]["groups"][0] = serde_json::json!(["outside"]),
            "unknown_need" => bad["supports"][0]["need_id"] = "unknown".into(),
            "unknown_field" => bad["extra"] = true.into(),
            _ => bad["supports"][0]["groups"] = serde_json::json!([[id], [id]]),
        }
        assert!(policy::parse_support(&serde_json::to_vec(&bad).unwrap(), &case, &needs).is_err());
    }
}

#[test]
fn exported_shapes_do_not_expose_gold_or_rank_and_fixture_truth_remains_separate() {
    for case in fixtures::cases() {
        let task = artifacts::task_bundle(case.task_index, &case.input.task);
        let value = serde_json::to_value(task).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert!(value.get("candidates").is_none() && value.get("facts").is_none());
        let mapping =
            serde_json::to_value(artifacts::mapping_bundle(&case.input, &needs())).unwrap();
        assert_eq!(mapping.as_object().unwrap().len(), 3);
        assert!(mapping.get("rank").is_none() && mapping.get("facts").is_none());
        assert!(case
            .facts
            .iter()
            .all(|fact| !fact.description.is_empty() && !fact.reason.is_empty()));
    }
}

#[test]
fn prefix_checks_full_tail_and_limits_and_old_observe_still_submits_all() {
    let case = fixtures::cases().remove(1);
    let mut harness = Harness::new(&case.input);
    let snapshot = harness.snapshot().unwrap();
    let rank = ranking::rank(&case.input, true, true);
    let old = harness.observe(&snapshot, &rank, true).unwrap();
    assert_eq!(old.metrics["submitted_candidate_count"], 14);
    assert_eq!(old.selected.len(), 8);
    for k in [0, 9, 15] {
        assert!(harness.observe_prefix(&snapshot, &rank, k, true).is_err());
    }
    let mut bad = rank.clone();
    bad[13] = bad[12].clone();
    assert!(harness.observe_prefix(&snapshot, &bad, 2, true).is_err());
    let mut short = case.input.clone();
    short.candidates.truncate(2);
    let mut harness = Harness::new(&short);
    let snapshot = harness.snapshot().unwrap();
    let rank = short
        .candidates
        .iter()
        .map(|c| c.id.clone())
        .collect::<Vec<_>>();
    assert!(harness.observe_prefix(&snapshot, &rank, 3, true).is_err());
}

#[test]
fn prefix_revalidates_goal_tail_and_hidden_alias_before_consuming() {
    use soul_store_api::{ForgetOps, ForgetUnit};
    use soulcore::commands::session::{
        AssistanceAction, AssistanceActionRequest, EditAssistanceGoal,
    };
    let case = fixtures::cases().remove(1);
    for change in [
        "goal_edit",
        "goal_cancel",
        "goal_forget",
        "tail",
        "alias_edit",
        "alias_forget",
    ] {
        let mut harness = Harness::new_with_aliases(&case.input, &case.aliases);
        let snapshot = harness.snapshot().unwrap();
        let rank = ranking::rank(&case.input, true, true);
        let item = &harness.goal.item;
        match change {
            "goal_edit" => {
                harness
                    .session
                    .edit_assistance_goal(EditAssistanceGoal {
                        item_id: item.item_id,
                        expected_revision: item.revision,
                        title: "changed".into(),
                        summary: "changed intent".into(),
                        deadline_at: None,
                    })
                    .unwrap();
            }
            "goal_cancel" => {
                harness
                    .session
                    .act_assistance_item(AssistanceActionRequest {
                        item_id: item.item_id,
                        expected_revision: item.revision,
                        action: AssistanceAction::Cancel,
                    })
                    .unwrap();
            }
            _ => {
                let id = match change {
                    "goal_forget" => harness.goal_memory_id(),
                    "tail" => {
                        harness.ids[case
                            .input
                            .candidates
                            .iter()
                            .position(|c| c.id == rank[13])
                            .unwrap()]
                    }
                    _ => harness.alias_ids[0],
                };
                let shared = harness.session.store().unwrap();
                let mut store = shared.lock().unwrap();
                if change.ends_with("forget") {
                    store.execute_forget(ForgetUnit::Memory(id)).unwrap();
                } else {
                    soul_memory::update(
                        &mut *store,
                        id,
                        &soul_memory::MemoryEdit {
                            title: None,
                            summary: Some("synthetic change".into()),
                            memory_type: None,
                        },
                        1_800_000_000,
                    )
                    .unwrap();
                }
            }
        }
        assert!(harness.observe_prefix(&snapshot, &rank, 2, true).is_err());
    }
}

#[test]
fn prefix_owner_permission_stays_local_and_email_redaction_preserves_facts() {
    let case = fixtures::cases().remove(7);
    let mut harness = Harness::new_with_aliases(&case.input, &case.aliases);
    let snapshot = harness.snapshot().unwrap();
    let email = case
        .input
        .candidates
        .iter()
        .find(|c| c.summary.contains("zhaoning@example.invalid"))
        .unwrap();
    let mut rank = vec![email.id.clone()];
    rank.extend(
        case.input
            .candidates
            .iter()
            .filter(|c| c.id != email.id)
            .map(|c| c.id.clone()),
    );
    for allow in [false, true] {
        let observation = harness.observe_prefix(&snapshot, &rank, 2, allow).unwrap();
        assert_eq!(observation.selected.len(), 2);
        assert_eq!(observation.retained.len(), 2);
        assert_eq!(
            observation.metrics["owner_excerpts_explicitly_allowed"],
            allow
        );
        assert_eq!(observation.metrics["network_requests"], 0);
    }
    let id = harness.ids[case
        .input
        .candidates
        .iter()
        .position(|c| c.id == email.id)
        .unwrap()];
    let context = harness
        .session
        .prepare_memory_context(soulcore::commands::session::MemoryContextRequest {
            candidate_ids: vec![id],
            query: None,
            memory_type: Some(soul_schema::memory::MemoryType::Commitment),
            time_specific: false,
        })
        .unwrap();
    let plan = harness
        .session
        .prepare_draft_with_memory(
            "合成消息等待答复",
            &context.context_id.unwrap().to_string(),
            &[id],
            false,
        )
        .unwrap();
    assert!(!plan
        .prepared
        .body_preview
        .contains("zhaoning@example.invalid"));
    let (_, data) = plan
        .prepared
        .body_preview
        .split_once("本次明确提供的记忆参考（保存的记录，未经事实或当前性核验）：")
        .unwrap();
    assert!(data.contains("[账号已占位]") && data.contains("她没有承诺接任"));
}
