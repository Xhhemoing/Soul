use super::{adapter::Harness, data::*, diagnosis, replay, scoring, types::*};

fn input() -> Input {
    Input {
        case_id: "synthetic-example".into(),
        task: TaskInput {
            scope: "合成交接任务".into(),
            intent: "确认当前交接地点".into(),
            constraints: "合成材料，不补充未知事实".into(),
        },
        information_options: (0..3)
            .map(|i| InformationOption {
                id: format!("r{i}"),
                description: format!("合成信息项{i}"),
            })
            .collect(),
        candidates: (0..4)
            .map(|i| Candidate {
                id: format!("c{i}"),
                title: format!("合成来源{i}"),
                summary: format!("synthetic-{i}，当前安排的合成来源。"),
            })
            .collect(),
        unavailable_ids: vec!["c3".into()],
        source_metadata: Vec::new(),
    }
}

fn with_metadata() -> Input {
    let mut input = input();
    input.candidates[3].summary = "[来源不可读取]".into();
    input.source_metadata = input
        .candidates
        .iter()
        .map(|candidate| SourceMetadata {
            source_id: candidate.id.clone(),
            subject: "合成甲组".into(),
            scope: "合成交接任务".into(),
            valid_time: "2030-03-02 本次交接".into(),
            topics: vec!["交接地点".into()],
            supersedes: Vec::new(),
        })
        .collect();
    input
}

#[test]
fn metadata_stays_attached_to_the_exact_source_in_preparation() {
    let input = with_metadata();
    diagnosis::validate_input(&input).unwrap();
    let case = input.case();
    assert!(case.candidates[0].summary.contains("2030-03-02"));
    assert!(case.candidates[0].summary.contains("\"source_id\":\"c0\""));
    assert!(case.candidates[3].summary.starts_with("[来源不可读取]"));
    let pred = parse(&input, &prediction(Status::Supported)).unwrap();
    let rank: Vec<_> = input.candidates.iter().map(|c| c.id.clone()).collect();
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    let result = replay::execute(
        &input,
        Ok(&pred),
        Ok(&rank),
        "predicted_required",
        &mut harness,
        &snapshot,
    );
    assert!(result.available);
    assert_eq!(result.retained_ids.unwrap(), vec!["c0"]);
    assert_eq!(result.network_requests, 0);
    harness.validate(&snapshot).unwrap();
}

#[test]
fn metadata_rejects_wrong_source_links_and_readability_claims() {
    let input = with_metadata();
    let mut duplicate = input.clone();
    duplicate.source_metadata[1].source_id = "c0".into();
    assert_eq!(
        diagnosis::validate_input(&duplicate),
        Err("source_metadata_ids")
    );
    let mut dangling = input.clone();
    dangling.source_metadata[0].supersedes = vec!["absent".into()];
    assert_eq!(
        diagnosis::validate_input(&dangling),
        Err("source_metadata_shape")
    );
    let mut self_link = input.clone();
    self_link.source_metadata[0].supersedes = vec!["c0".into()];
    assert_eq!(
        diagnosis::validate_input(&self_link),
        Err("source_metadata_shape")
    );
    let mut readable = input;
    readable.candidates[3].summary = "should not reveal an unavailable body".into();
    assert_eq!(
        diagnosis::validate_input(&readable),
        Err("source_readability_mismatch")
    );
}

#[test]
fn original_input_projection_and_serialization_remain_unchanged() {
    let original = input();
    assert!(original.case().candidates == original.candidates);
    assert!(serde_json::to_value(&original)
        .unwrap()
        .get("source_metadata")
        .is_none());
}

fn prediction(status: Status) -> Prediction {
    Prediction {
        case_id: "synthetic-example".into(),
        required_ids: vec!["r0".into()],
        assessments: vec![Assessment {
            need_id: "r0".into(),
            status,
            support_groups: if status == Status::Supported {
                vec![vec!["c0".into()]]
            } else {
                vec![]
            },
            conflict_ids: if status == Status::Conflict {
                vec!["c0".into(), "c1".into()]
            } else {
                vec![]
            },
        }],
    }
}

fn parse(input: &Input, prediction: &Prediction) -> Result<Prediction, &'static str> {
    diagnosis::parse(&serde_json::to_vec(prediction).unwrap(), input)
}

#[test]
fn nonrequired_options_without_assessments_do_not_block_ready() {
    let input = input();
    let pred = parse(&input, &prediction(Status::Supported)).unwrap();
    assert_eq!(diagnosis::action(&pred), Action::Ready);
    let all = diagnosis::all_options(&input, &pred);
    assert_eq!(diagnosis::action(&all), Action::Unknown);
    assert_eq!(all.assessments[1].status, Status::Unknown);
    assert_eq!(all.assessments[2].status, Status::Unknown);
}

#[test]
fn actual_optional_support_allows_all_options_to_be_ready_with_required_subset() {
    let input = input();
    let mut pred = prediction(Status::Supported);
    for index in 1..3 {
        pred.assessments.push(Assessment {
            need_id: format!("r{index}"),
            status: Status::Supported,
            support_groups: vec![vec![format!("c{index}")]],
            conflict_ids: Vec::new(),
        });
    }
    let pred = parse(&input, &pred).unwrap();
    let all = diagnosis::all_options(&input, &pred);
    assert_eq!(diagnosis::action(&pred), Action::Ready);
    assert_eq!(diagnosis::action(&all), Action::Ready);
    let rank: Vec<_> = input.candidates.iter().map(|c| c.id.clone()).collect();
    let mut harness = Harness::new(&input.case());
    let snapshot = harness.snapshot().unwrap();
    let predicted = replay::execute(
        &input,
        Ok(&pred),
        Ok(&rank),
        "predicted_required",
        &mut harness,
        &snapshot,
    );
    assert!(predicted.available);
    assert_eq!(predicted.action, Action::Ready);
    assert_eq!(predicted.retained_ids.unwrap(), vec!["c0"]);
    let broad = replay::execute(
        &input,
        Ok(&pred),
        Ok(&rank),
        "all_options",
        &mut harness,
        &snapshot,
    );
    assert!(broad.available);
    assert_eq!(broad.action, Action::Ready);
    assert_eq!(broad.retained_ids.unwrap().len(), 3);
    assert_eq!(broad.network_requests, 0);
}

#[test]
fn assessed_optional_missing_does_not_block_predicted_required() {
    let input = input();
    let mut pred = prediction(Status::Supported);
    pred.assessments.push(Assessment {
        need_id: "r1".into(),
        status: Status::Missing,
        support_groups: Vec::new(),
        conflict_ids: Vec::new(),
    });
    pred.assessments.push(Assessment {
        need_id: "r2".into(),
        status: Status::Supported,
        support_groups: vec![vec!["c2".into()]],
        conflict_ids: Vec::new(),
    });
    let pred = parse(&input, &pred).unwrap();
    assert_eq!(diagnosis::action(&pred), Action::Ready);
    assert_eq!(
        diagnosis::action(&diagnosis::all_options(&input, &pred)),
        Action::ClarifyMissing
    );
    let rank: Vec<_> = input.candidates.iter().map(|c| c.id.clone()).collect();
    let selected = diagnosis::selection(&input, &pred, &rank).unwrap();
    assert_eq!(selected.requested_ids, vec!["c0"]);
    assert_eq!(selected.chosen_groups.len(), 1);
}

#[test]
fn required_missing_clarifies_and_unknown_does_not_claim_missing() {
    let input = input();
    let missing = parse(&input, &prediction(Status::Missing)).unwrap();
    assert_eq!(diagnosis::action(&missing), Action::ClarifyMissing);
    let unknown = parse(&input, &prediction(Status::Unknown)).unwrap();
    assert_eq!(diagnosis::action(&unknown), Action::Unknown);
    let mut invalid = prediction(Status::Supported);
    invalid.assessments[0].support_groups = vec![vec!["outside".into()]];
    assert_eq!(parse(&input, &invalid).err(), Some("support_source_ids"));
    invalid.assessments[0].support_groups = vec![vec!["c3".into()]];
    assert_eq!(parse(&input, &invalid).err(), Some("support_source_ids"));
}

#[test]
fn conflict_retains_both_sides_and_unknown_has_priority() {
    let input = input();
    let mut pred = parse(&input, &prediction(Status::Conflict)).unwrap();
    assert_eq!(diagnosis::action(&pred), Action::ClarifyConflict);
    assert_eq!(pred.assessments[0].conflict_ids, vec!["c0", "c1"]);
    pred.required_ids.push("r1".into());
    pred.assessments.push(Assessment {
        need_id: "r1".into(),
        status: Status::Unknown,
        support_groups: vec![],
        conflict_ids: vec![],
    });
    assert_eq!(
        diagnosis::action(&parse(&input, &pred).unwrap()),
        Action::Unknown
    );
}

#[test]
fn malformed_incomplete_extra_and_contradictory_predictions_are_invalid() {
    let input = input();
    for mode in 0..8 {
        let mut pred = prediction(Status::Supported);
        match mode {
            0 => pred.assessments.clear(),
            1 => pred.required_ids.push("r0".into()),
            2 => pred.assessments.push(pred.assessments[0].clone()),
            3 => pred.assessments[0].need_id = "r1".into(),
            4 => pred.assessments[0].support_groups.clear(),
            5 => pred.assessments[0].support_groups = vec![vec!["c0".into(), "c0".into()]],
            6 => pred.assessments[0].status = Status::Missing,
            _ => pred.assessments[0].conflict_ids = vec!["c0".into(), "c1".into()],
        }
        assert!(parse(&input, &pred).is_err(), "mode {mode}");
    }
    let mut unknown = serde_json::to_value(prediction(Status::Supported)).unwrap();
    unknown["extra"] = true.into();
    assert!(diagnosis::parse(&serde_json::to_vec(&unknown).unwrap(), &input).is_err());
    assert!(diagnosis::parse(b"{", &input).is_err());
    let mut conflict = prediction(Status::Conflict);
    conflict.assessments[0].conflict_ids.pop();
    assert!(parse(&input, &conflict).is_err());
}

#[test]
fn and_or_selection_reuses_complete_groups_and_validates_the_entire_rank() {
    let input = input();
    let mut pred = prediction(Status::Supported);
    pred.assessments[0].support_groups = vec![vec!["c0".into(), "c2".into()], vec!["c1".into()]];
    let rank: Vec<String> = vec!["c0", "c2", "c1", "c3"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let chosen = diagnosis::selection(&input, &parse(&input, &pred).unwrap(), &rank).unwrap();
    assert_eq!(chosen.requested_ids, vec!["c1"]);
    assert_eq!(chosen.full_rank, vec!["c1", "c0", "c2", "c3"]);
    pred.assessments[0].support_groups.pop();
    let chosen = diagnosis::selection(&input, &parse(&input, &pred).unwrap(), &rank).unwrap();
    assert_eq!(chosen.requested_ids, vec!["c0", "c2"]);
    let mut bad_rank = rank;
    bad_rank[3] = "c2".into();
    assert!(diagnosis::validate_rank(&input, &bad_rank).is_err());
}

#[test]
fn real_replay_avoids_unavailable_sources_and_rejects_changed_snapshots() {
    let input = input();
    let pred = parse(&input, &prediction(Status::Supported)).unwrap();
    let rank: Vec<_> = input.candidates.iter().map(|c| c.id.clone()).collect();
    let case = input.case();
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    let ready = replay::execute(
        &input,
        Ok(&pred),
        Ok(&rank),
        "predicted_required",
        &mut harness,
        &snapshot,
    );
    assert_eq!(ready.action, Action::Ready);
    assert!(ready.available);
    assert_eq!(ready.retained_ids.as_ref().unwrap(), &vec!["c0".to_owned()]);
    assert_eq!(ready.network_requests, 0);
    let fixed = replay::execute(
        &input,
        Err("prediction_missing"),
        Ok(&rank),
        "fixed4",
        &mut harness,
        &snapshot,
    );
    assert!(fixed.available);
    assert!(!fixed.selected_ids.as_ref().unwrap().contains(&"c3".into()));
    let memory_id = harness.ids[0].to_string();
    let forget = harness.session.preview_forget(&memory_id).unwrap();
    harness
        .session
        .forget_memory(&soulcore::commands::memory::ForgetConfirmation {
            preview_id: forget.preview_id,
            memory_id,
        })
        .unwrap();
    let changed = replay::execute(
        &input,
        Ok(&pred),
        Ok(&rank),
        "predicted_required",
        &mut harness,
        &snapshot,
    );
    assert!(!changed.available);
    assert_eq!(changed.reason, "snapshot_changed");
    assert_eq!(changed.action, Action::Unknown);
    assert_eq!(changed.network_requests, 0);
}

#[test]
fn invalid_predictions_remain_in_accuracy_denominators() {
    let input = input();
    let gold = Gold {
        required_ids: vec!["r0".into()],
        assessments: prediction(Status::Supported).assessments,
        action: Action::Ready,
        rationale: "合成真值".into(),
        conflict_groups: Default::default(),
    };
    let case = input.case();
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    let rank: Vec<_> = input.candidates.iter().map(|c| c.id.clone()).collect();
    let failed = replay::execute(
        &input,
        Err("prediction_shape"),
        Ok(&rank),
        "predicted_required",
        &mut harness,
        &snapshot,
    );
    let score = scoring::score(&gold, None, &failed);
    assert_eq!(score["case_denominator"], 1);
    assert_eq!(score["required_fn"], 1);
    assert_eq!(score["action_correct"], false);
    assert_eq!(score["action_denominator"], 1);
    assert!(!failed.available);
    assert_eq!(failed.action, Action::Unknown);
}

#[test]
fn diagnostic_clarifications_do_not_prepare_context_or_depend_on_ranking() {
    let input = input();
    let case = input.case();
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    for status in [Status::Missing, Status::Conflict, Status::Unknown] {
        let pred = parse(&input, &prediction(status)).unwrap();
        let result = replay::execute(
            &input,
            Ok(&pred),
            Err("ranking_shape"),
            "predicted_required",
            &mut harness,
            &snapshot,
        );
        assert!(result.available);
        assert!(!result.attempted_prepare);
        assert!(result.preparation.is_none());
        assert!(result.selected_ids.is_none());
        assert_eq!(result.network_requests, 0);
    }
    let pred = parse(&input, &prediction(Status::Supported)).unwrap();
    let result = replay::execute(
        &input,
        Ok(&pred),
        Err("ranking_shape"),
        "predicted_required",
        &mut harness,
        &snapshot,
    );
    assert!(!result.available);
    assert_eq!(result.reason, "ranking_shape");
    assert_eq!(result.action, Action::Unknown);
    assert_eq!(result.predicted_action, Some(Action::Ready));
}

#[test]
fn no_required_information_is_ready_without_context_or_ranking_dependency() {
    let input = input();
    let mut pred = prediction(Status::Supported);
    pred.required_ids.clear();
    pred.assessments.clear();
    let pred = parse(&input, &pred).unwrap();
    let case = input.case();
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    let result = replay::execute(
        &input,
        Ok(&pred),
        Err("ranking_shape"),
        "predicted_required",
        &mut harness,
        &snapshot,
    );
    assert!(result.available);
    assert_eq!(result.action, Action::Ready);
    assert!(!result.attempted_prepare);
    assert!(result.retained_ids.as_ref().unwrap().is_empty());
    assert_eq!(result.network_requests, 0);
}

#[test]
fn input_only_serialization_excludes_evaluation_metadata_and_truth() {
    let value = serde_json::to_value(input()).unwrap();
    for name in [
        "gold",
        "required_ids",
        "assessments",
        "family_id",
        "split",
        "variant",
        "rationale",
    ] {
        assert!(value.get(name).is_none());
    }
    assert_eq!(value.as_object().unwrap().len(), 5);
    let mut injected = value;
    injected["gold"] = serde_json::json!({"required_ids":["r0"]});
    assert!(serde_json::from_value::<Input>(injected).is_err());
}

#[test]
fn either_complete_conflict_path_is_accepted_without_requiring_the_union() {
    let input = input();
    let mut truth = prediction(Status::Conflict);
    truth.assessments[0].conflict_ids = vec!["c0".into(), "c1".into(), "c2".into()];
    let mut gold = Gold {
        required_ids: vec!["r0".into()],
        assessments: truth.assessments,
        action: Action::ClarifyConflict,
        rationale: "c0 conflicts with either independent alternative c1 or c2".into(),
        conflict_groups: [(
            "r0".into(),
            vec![
                vec!["c0".into(), "c1".into()],
                vec!["c0".into(), "c2".into()],
            ],
        )]
        .into_iter()
        .collect(),
    };
    let mut result = replay::Outcome::unavailable("synthetic_conflict_scoring");
    result.available = true;
    result.action = Action::ClarifyConflict;
    result.predicted_action = Some(Action::ClarifyConflict);
    result.required_ids = Some(vec!["r0".into()]);
    for ids in [vec!["c0", "c1"], vec!["c0", "c2"]] {
        let mut pred = prediction(Status::Conflict);
        pred.assessments[0].conflict_ids = ids.into_iter().map(str::to_owned).collect();
        let pred = parse(&input, &pred).unwrap();
        result.assessments = Some(pred.assessments.clone());
        assert_eq!(
            scoring::score(&gold, Some(&pred), &result)["diagnosis_correct"],
            true
        );
    }
    let mut incomplete = prediction(Status::Conflict);
    incomplete.assessments[0].conflict_ids = vec!["c1".into(), "c2".into()];
    result.assessments = Some(incomplete.assessments.clone());
    assert_eq!(
        scoring::score(&gold, Some(&incomplete), &result)["diagnosis_correct"],
        false
    );
    gold.conflict_groups.clear();
    let strict = prediction(Status::Conflict);
    result.assessments = Some(strict.assessments.clone());
    assert_eq!(
        scoring::score(&gold, Some(&strict), &result)["diagnosis_correct"],
        false
    );
}

#[test]
fn provided_ranking_is_validated_in_full_and_invalid_file_never_falls_back() {
    let input = input();
    let directory = tempfile::tempdir().unwrap();
    let (initial, source) = super::files::rank(directory.path(), &input);
    assert!(initial.is_ok());
    assert_eq!(source, "bm25_groups_mmr");
    let path = directory
        .path()
        .join(format!("ranking-{}.json", input.case_id));
    let raw = serde_json::json!({"case_id":input.case_id,"ranked_ids":["c2","c0","c1","c3"]});
    std::fs::write(&path, serde_json::to_vec(&raw).unwrap()).unwrap();
    let (valid, source) = super::files::rank(directory.path(), &input);
    assert_eq!(valid.unwrap(), vec!["c2", "c0", "c1", "c3"]);
    assert_eq!(source, "provided_ranking");
    let mut bad = raw;
    bad["ranked_ids"][3] = "c1".into();
    std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
    let (invalid, source) = super::files::rank(directory.path(), &input);
    assert!(invalid.is_err());
    assert_eq!(source, "provided_ranking");
}

#[test]
fn real_budget_loss_changes_runtime_action_to_unknown_and_preserves_actual_sources() {
    let mut input = input();
    for candidate in &mut input.candidates {
        candidate.summary = format!("synthetic-{}:{}", candidate.id, "x".repeat(7_000));
    }
    let mut pred = prediction(Status::Supported);
    pred.assessments[0].support_groups = vec![vec!["c0".into(), "c1".into(), "c2".into()]];
    let pred = parse(&input, &pred).unwrap();
    let rank: Vec<_> = input.candidates.iter().map(|c| c.id.clone()).collect();
    let case = input.case();
    let mut harness = Harness::new(&case);
    let snapshot = harness.snapshot().unwrap();
    let result = replay::execute(
        &input,
        Ok(&pred),
        Ok(&rank),
        "predicted_required",
        &mut harness,
        &snapshot,
    );
    assert!(!result.available);
    assert_eq!(result.action, Action::Unknown);
    assert_eq!(result.predicted_action, Some(Action::Ready));
    assert_eq!(result.reason, "serialized_selection_loss");
    assert_eq!(result.group_retained_complete, Some(false));
    assert!(result.selected_ids.as_ref().unwrap().len() < 3);
    assert!(result
        .retained_ids
        .as_ref()
        .unwrap()
        .iter()
        .all(|id| !input.unavailable_ids.contains(id)));
    assert_eq!(result.network_requests, 0);
    harness.validate(&snapshot).unwrap();
}

#[test]
fn wrong_and_group_is_not_scored_as_sufficient_from_one_member() {
    let mut gold_prediction = prediction(Status::Supported);
    gold_prediction.assessments[0].support_groups = vec![vec!["c0".into(), "c1".into()]];
    let gold = Gold {
        required_ids: vec!["r0".into()],
        assessments: gold_prediction.assessments,
        action: Action::Ready,
        rationale: "two independent facts".into(),
        conflict_groups: Default::default(),
    };
    let mut result = replay::Outcome::unavailable("synthetic_scoring_example");
    result.available = true;
    result.action = Action::Ready;
    result.predicted_action = Some(Action::Ready);
    result.required_ids = Some(vec!["r0".into()]);
    result.assessments = Some(prediction(Status::Supported).assessments);
    result.retained_ids = Some(vec!["c0".into()]);
    let score = scoring::score(&gold, Some(&prediction(Status::Supported)), &result);
    assert_eq!(score["wrong_sufficient"], true);
    assert_eq!(score["all_required_fact_groups_retained"], false);
    assert_eq!(score["diagnosis_correct"], false);
    result.diagnosis_method = false;
    let baseline = scoring::score(&gold, None, &result);
    assert_eq!(baseline["diagnosis_denominator"], 0);
    assert!(baseline["required_tp"].is_null());
    assert!(baseline["diagnosis_correct"].is_null());
}
