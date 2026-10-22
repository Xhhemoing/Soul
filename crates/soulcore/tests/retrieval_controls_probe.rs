//! Test-only retrieval controls; synthetic stores and unused loopback endpoints.
#[path = "task_retrieval_probe/adapter.rs"]
mod adapter;
#[path = "retrieval_controls_probe/fixtures.rs"]
mod fixtures;
#[path = "retrieval_controls_probe/ranking.rs"]
mod ranking;
#[path = "task_retrieval_probe/ranking.rs"]
mod reference_rank;
#[path = "retrieval_controls_probe/replay.rs"]
mod replay;
#[path = "task_retrieval_probe/semantic.rs"]
mod semantic;
#[path = "task_retrieval_probe/types.rs"]
mod types;

use std::collections::BTreeSet;
use types::{Candidate, InputCase, TaskInput};

#[test]
fn repeated_bodies_do_not_consume_the_distinct_material_budget() {
    let case = InputCase {
        case_id: "duplicate_budget_control".into(),
        task: TaskInput {
            scope: "合成场地预约".into(),
            intent: "合成场地预约".into(),
            constraints: "合成场地预约".into(),
        },
        candidates: (0..32)
            .map(|index| Candidate {
                id: format!("r{index:02}"),
                title: "合成场地预约".into(),
                summary: if index < 19 {
                    "合成场地预约".into()
                } else {
                    format!("合成场地预约，补充材料第{index}项：窗口和设备的独立记录。")
                },
            })
            .collect(),
    };
    types::case_bytes(&case).expect("bounded synthetic input");
    let mut harness = adapter::Harness::new(&case);
    let snapshot = harness.snapshot().expect("current synthetic snapshots");
    let baseline = reference_rank::rank(&snapshot.input, true);
    let baseline_bodies: BTreeSet<_> = baseline[..8]
        .iter()
        .map(|id| {
            let entry = case
                .candidates
                .iter()
                .find(|entry| &entry.id == id)
                .unwrap();
            (&entry.title, &entry.summary)
        })
        .collect();
    assert_eq!(
        baseline_bodies.len(),
        1,
        "observed_reference_counterexample"
    );
    let ranked = ranking::rank(&snapshot.input, true, false);
    let observation = harness
        .observe(&snapshot, &ranked, true)
        .expect("real selection and preparation");
    assert_eq!(observation.selected.len(), 8);
    assert_eq!(observation.retained.len(), 8);
    assert_eq!(observation.metrics["network_requests"], 0);
    let selected_bodies: BTreeSet<_> = observation
        .selected
        .iter()
        .map(|id| {
            let candidate = case.candidates.iter().find(|row| &row.id == id).unwrap();
            (&candidate.title, &candidate.summary)
        })
        .collect();
    assert_eq!(selected_bodies.len(), 8, "distinct_material_budget");
}

#[test]
fn dynamic_permutations_remain_strict_at_fourteen_and_thirty_two() {
    for case in fixtures::cases().into_iter().take(3) {
        types::case_bytes(&case.input).unwrap();
        let ranked = ranking::rank(&case.input, true, true);
        let predictions = [(case.input.case_id.clone(), ranked)].into_iter().collect();
        let bytes =
            semantic::encode_ordered(std::slice::from_ref(&case.input), predictions).unwrap();
        assert!(semantic::parse(&bytes, std::slice::from_ref(&case.input)).is_ok());
        let valid: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for corruption in ["duplicate", "missing", "unknown", "extra_field"] {
            let mut bad = valid.clone();
            match corruption {
                "duplicate" => bad[0]["ranked_ids"][0] = bad[0]["ranked_ids"][1].clone(),
                "missing" => {
                    bad[0]["ranked_ids"].as_array_mut().unwrap().pop();
                }
                "unknown" => bad[0]["ranked_ids"][0] = "outside".into(),
                _ => bad[0]["extra"] = true.into(),
            }
            assert!(semantic::parse(
                &serde_json::to_vec(&bad).unwrap(),
                std::slice::from_ref(&case.input)
            )
            .is_err());
        }
        let mut invalid_input = case.input.clone();
        invalid_input.candidates[0].id = invalid_input.candidates[1].id.clone();
        assert!(types::case_bytes(&invalid_input).is_err());
    }
}

#[test]
fn fixtures_preserve_facts_and_sources_without_duplicate_credit() {
    let cases = fixtures::cases();
    assert_eq!(cases.len(), 18);
    for trio in cases.chunks_exact(3) {
        assert_eq!(trio[0].input.candidates.len(), 32);
        assert_eq!(trio[1].input.candidates.len(), 14);
        assert_eq!(trio[1].aliases.len(), 18);
        assert_eq!(trio[2].input.candidates.len(), 32);
        for case in trio {
            types::case_bytes(&case.input).unwrap();
            assert!(case.input.task == trio[0].input.task);
            assert_eq!(case.facts.len(), trio[0].facts.len());
            let ids: BTreeSet<_> = case
                .input
                .candidates
                .iter()
                .chain(&case.aliases)
                .map(|c| &c.id)
                .collect();
            assert_eq!(ids.len(), 32);
            let mut classified = BTreeSet::new();
            for fact in &case.facts {
                assert!(!fact.description.is_empty() && !fact.reason.is_empty());
                assert!(fact.acceptable_source_ids.iter().all(|id| ids.contains(id)));
                classified.extend(fact.acceptable_source_ids.iter());
                let repeated = vec![fact.acceptable_source_ids[0].clone(); 8];
                assert_eq!(
                    fixtures::fact_coverage(std::slice::from_ref(fact), &repeated),
                    1
                );
            }
            for (id, reason) in case.supporting.iter().chain(&case.inapplicable) {
                assert!(!reason.is_empty());
                assert!(classified.insert(id));
            }
            assert!(classified == ids);
        }
        for base in &trio[1].input.candidates {
            for case in [&trio[0], &trio[2]] {
                assert!(case
                    .input
                    .candidates
                    .iter()
                    .any(|candidate| candidate == base));
            }
        }
    }
}

#[test]
fn grouping_and_mmr_preserve_all_ids_and_change_distinct_content_order() {
    let mut changed = false;
    for case in fixtures::cases() {
        let grouped = ranking::rank(&case.input, true, false);
        let mmr = ranking::rank(&case.input, true, true);
        let expected: BTreeSet<_> = case.input.candidates.iter().map(|c| &c.id).collect();
        assert!(grouped.iter().collect::<BTreeSet<_>>() == expected);
        assert!(mmr.iter().collect::<BTreeSet<_>>() == expected);
        let group_count = ranking::groups(&case.input).len();
        for ranked in [&grouped, &mmr] {
            let bodies: BTreeSet<_> = ranked[..group_count]
                .iter()
                .map(|id| {
                    let candidate = case.input.candidates.iter().find(|c| &c.id == id).unwrap();
                    (&candidate.title, &candidate.summary)
                })
                .collect();
            assert_eq!(bodies.len(), group_count);
        }
        changed |= grouped[..group_count] != mmr[..group_count];
    }
    assert!(changed, "diversity_changes_nonduplicate_order");
}

#[test]
fn compact_alias_changes_and_forgetting_invalidate_the_ranking_window() {
    use soul_store_api::{ForgetOps, ForgetUnit};
    let case = fixtures::cases().remove(1);
    for forget in [false, true] {
        let mut harness = adapter::Harness::new_with_aliases(&case.input, &case.aliases);
        let snapshot = harness.snapshot().unwrap();
        let alias = harness.alias_ids[0];
        let store = harness.session.store().unwrap();
        let mut store = store.lock().unwrap();
        if forget {
            store.execute_forget(ForgetUnit::Memory(alias)).unwrap();
        } else {
            soul_memory::update(
                &mut *store,
                alias,
                &soul_memory::MemoryEdit {
                    title: None,
                    summary: Some("changed_synthetic_summary".into()),
                    memory_type: None,
                },
                1_800_000_000,
            )
            .unwrap();
        }
        drop(store);
        assert!(harness.validate(&snapshot).is_err());
        let ranked = ranking::rank(&case.input, true, false);
        assert!(harness.observe(&snapshot, &ranked, true).is_err());
    }
}

#[test]
fn compact_aliases_do_not_expand_default_or_selected_owner_permission() {
    let case = fixtures::cases().remove(1);
    let mut harness = adapter::Harness::new_with_aliases(&case.input, &case.aliases);
    let snapshot = harness.snapshot().unwrap();
    let ranked = ranking::rank(&snapshot.input, true, false);
    for allow_owner in [false, true] {
        let observed = harness.observe(&snapshot, &ranked, allow_owner).unwrap();
        assert_eq!(
            observed.metrics["owner_excerpts_explicitly_allowed"],
            allow_owner
        );
        assert!(observed.selected.iter().all(|id| case
            .input
            .candidates
            .iter()
            .any(|c| &c.id == id)));
    }
}

#[test]
#[ignore = "explicit synthetic input export"]
fn export_inputs() {
    replay::export();
}

#[test]
fn owner_email_source_is_retained_with_identifier_redaction() {
    let case = fixtures::cases().remove(7);
    let email_source = case
        .input
        .candidates
        .iter()
        .find(|entry| entry.summary.contains("zhaoning@example.invalid"))
        .expect("synthetic approved contact source");
    let mut ranked = vec![email_source.id.clone()];
    ranked.extend(
        case.input
            .candidates
            .iter()
            .filter(|entry| entry.id != email_source.id)
            .map(|entry| entry.id.clone()),
    );
    let mut harness = adapter::Harness::new_with_aliases(&case.input, &case.aliases);
    let snapshot = harness.snapshot().unwrap();
    let observation = harness
        .observe(&snapshot, &ranked, true)
        .expect("redacted source remains valid material");
    assert!(observation.selected.contains(&email_source.id));
    assert!(observation.retained.contains(&email_source.id));
    assert_eq!(observation.metrics["network_requests"], 0);
    let source_index = case
        .input
        .candidates
        .iter()
        .position(|entry| entry.id == email_source.id)
        .unwrap();
    let source_id = harness.ids[source_index];
    let context = harness
        .session
        .prepare_memory_context(soulcore::commands::session::MemoryContextRequest {
            candidate_ids: vec![source_id],
            query: None,
            memory_type: Some(soul_schema::memory::MemoryType::Commitment),
            time_specific: false,
        })
        .unwrap();
    let prepared = harness
        .session
        .prepare_draft_with_memory(
            "合成消息等待答复",
            &context.context_id.unwrap().to_string(),
            &[source_id],
            false,
        )
        .unwrap();
    assert!(prepared.prepared.retained_memory_ids == vec![source_id]);
    let body = &prepared.prepared.body_preview;
    assert!(!body.contains("zhaoning@example.invalid"));
    let (_, data) = body
        .split_once("本次明确提供的记忆参考（保存的记录，未经事实或当前性核验）：")
        .unwrap();
    assert!(data.contains("[账号已占位]"));
    assert!(data.contains("同意我把她的活动邮箱"));
    assert!(data.contains("她没有承诺接任"));
}

#[test]
#[ignore = "explicit retained semantic response replay"]
fn replay_predictions() {
    replay::run();
}
