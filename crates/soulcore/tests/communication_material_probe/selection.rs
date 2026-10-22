use std::collections::BTreeSet;
use std::time::Instant;

use serde_json::{json, Value};
use soul_memory::{ExclusionReason, SelectionOutcome};
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::session::{MemoryContextView, Session};
use uuid::Uuid;

use super::cards::Card;
use super::support;

pub fn observe(card: &Card<'_>) -> Value {
    let (_dir, mut session) = support::setup();
    let endpoint = MockLlm::start().expect("controlled unused loopback endpoint");
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("explicit endpoint for preparation only");
    // Positions and expectations are authored in cards.rs, not by the selector.
    let required: BTreeSet<_> = card.required.iter().copied().collect();
    let irrelevant: BTreeSet<_> = card.irrelevant.iter().copied().collect();
    assert_eq!(required.len(), card.required.len());
    assert!(required.is_disjoint(&irrelevant));
    assert!(required.iter().chain(&irrelevant).all(|i| *i < 3));
    assert!(card.counter.map_or(true, |i| required.contains(&i)));
    assert!(!card.task_requirement.is_empty());
    assert_eq!(
        card.missing_fact_label.is_some(),
        card.variant == "missing_material"
    );
    let ids: Vec<_> = card
        .notes
        .iter()
        .map(|note| support::memory(&mut session, "合成资料", note))
        .collect();
    let manual_ids: Vec<_> = card.required.iter().map(|i| ids[*i]).collect();
    let mut results = Vec::new();
    for (mode, candidates, query) in [
        ("manual_same_pool", manual_ids.as_slice(), None),
        ("all_without_query", ids.as_slice(), None),
        ("all_with_query", ids.as_slice(), Some(card.scene.keyword)),
    ] {
        let started = Instant::now();
        let result = session
            .prepare_memory_context(support::request(candidates, query, card.time_specific))
            .expect("bounded synthetic selection");
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        let selected: BTreeSet<_> = result
            .preview
            .selected
            .iter()
            .map(|entry| {
                let i = ids
                    .iter()
                    .position(|id| *id == entry.memory_id)
                    .expect("same pool");
                assert!(entry.citation.memory_id == ids[i]);
                assert!(entry.summary == card.notes[i]);
                i
            })
            .collect();
        assert_eq!(selected.len(), result.preview.selected.len());
        let expected: BTreeSet<_> = if card.time_specific {
            BTreeSet::new()
        } else {
            match mode {
                "manual_same_pool" => required.clone(),
                "all_without_query" => [0, 1, 2].into_iter().collect(),
                "all_with_query" => card.expected_query_indices.iter().copied().collect(),
                _ => unreachable!(),
            }
        };
        assert!(
            selected == expected,
            "hand-authored selection observation changed"
        );
        let expected_outcome = if card.time_specific {
            SelectionOutcome::Clarify
        } else if expected.is_empty() {
            SelectionOutcome::Abstain
        } else {
            SelectionOutcome::Context
        };
        assert_eq!(result.preview.outcome, expected_outcome);
        assert_eq!(result.context_id.is_some(), !selected.is_empty());
        if card.time_specific {
            assert!(result
                .preview
                .excluded
                .iter()
                .all(|entry| { entry.reason == ExclusionReason::TemporalScopeUnavailable }));
        } else if query.is_some() {
            assert!(result
                .preview
                .excluded
                .iter()
                .all(|entry| entry.reason == ExclusionReason::KeywordMismatch));
        }
        let required_found =
            (!required.is_empty()).then(|| selected.intersection(&required).count());
        let preparation = prepare_selected(&mut session, &endpoint, &result, card);
        results.push(json!({
            "mode": mode,
            "candidate_count": candidates.len(),
            "selected_count": selected.len(),
            "required_available_count": required.len(),
            "required_found": required_found,
            "required_coverage": required_found.map(|found| found as f64 / required.len() as f64),
            "counterevidence_found": card.counter.map(|i| selected.contains(&i)),
            "irrelevant_selected": selected.intersection(&irrelevant).count(),
            "outcome": result.preview.outcome,
            "context_available": result.context_id.is_some(),
            "selection_ms": elapsed_ms,
            "preparation": preparation
        }));
    }
    json!({
        "scene": card.scene.label,
        "variant": card.variant,
        "unavailable_required_facts": card.unavailable_facts,
        "task_requirement": card.missing_fact_label.unwrap_or(if card.time_specific {
            "fact_time_capability_only"
        } else {
            "scene_goal"
        }),
        "missing_fact_label": card.missing_fact_label,
        "clarification_scope": card.missing_fact_label.map(|_| "only_named_missing_fact"),
        "determined_task_parts_may_continue": card.missing_fact_label.map(|_| true),
        "task_requirements_are_fixture_annotations_not_selector_inputs": true,
        "fact_time_capability_probe": card.time_specific,
        "semantic_fact_time_reasoning_measured": false,
        "fact_time_flag_identical_across_conditions": true,
        "truth_labels": "hand_authored_fixture_only",
        "selection_conditions": results,
        "semantic_sufficiency_detection_measured": false,
        "preparation_network_requests": endpoint.request_count()
    })
}

fn prepare_selected(
    session: &mut Session,
    endpoint: &MockLlm,
    context: &MemoryContextView,
    card: &Card<'_>,
) -> Value {
    assert_eq!(endpoint.request_count(), 0);
    let Some(context_id) = context.context_id else {
        return Value::Null;
    };
    let owner_ids: Vec<_> = context
        .preview
        .selected
        .iter()
        .map(|entry| entry.memory_id)
        .collect();
    let expected_ids: BTreeSet<Uuid> = owner_ids.iter().copied().collect();
    let started = Instant::now();
    let plan = session
        .prepare_draft_with_memory(card.scene.paste, &context_id.to_string(), &owner_ids, false)
        .expect("explicit selected owner excerpts, preparation only");
    let prepare_ms = started.elapsed().as_secs_f64() * 1000.0;
    let retained: BTreeSet<_> = plan.prepared.retained_memory_ids.iter().copied().collect();
    let permitted: BTreeSet<_> = plan.prepared.endpoint_owner_ids.iter().copied().collect();
    assert!(retained == expected_ids);
    assert!(permitted == expected_ids);
    assert_eq!(retained.len(), plan.prepared.retained_memory_ids.len());
    let (profile_section, memory_section) = plan
        .prepared
        .body_preview
        .split_once("本次明确提供的记忆参考（保存的记录，未经事实或当前性核验）：")
        .expect("prepared owner material section");
    let all_selected_present = context.preview.selected.iter().all(|entry| {
        !profile_section.contains(&entry.summary) && memory_section.contains(&entry.summary)
    });
    assert!(all_selected_present);
    let mandatory_selected: Vec<_> = card
        .required
        .iter()
        .filter(|i| {
            context
                .preview
                .selected
                .iter()
                .any(|entry| entry.summary == card.notes[**i])
        })
        .collect();
    let mandatory_present = mandatory_selected
        .iter()
        .filter(|i| memory_section.contains(card.notes[***i]))
        .count();
    assert_eq!(mandatory_present, mandatory_selected.len());
    let selected_material_truncated = retained != expected_ids || !all_selected_present;
    assert!(!selected_material_truncated);
    assert!(!plan.prepared.body_preview.contains(card.scene.paste));
    // No approval and no execute: the next prepare replaces this preparation.
    assert_eq!(endpoint.request_count(), 0);
    json!({
        "selected_count": owner_ids.len(),
        "retained_count": retained.len(),
        "retained_identity_set_matches_selection": retained == expected_ids,
        "explicit_owner_permission_matches_selection": permitted == expected_ids,
        "all_selected_summaries_in_memory_data_section": all_selected_present,
        "selected_material_truncated": selected_material_truncated,
        "mandatory_selected_count": mandatory_selected.len(),
        "mandatory_selected_present_count": mandatory_present,
        "mandatory_selected_fully_present": (!mandatory_selected.is_empty())
            .then_some(mandatory_present == mandatory_selected.len()),
        "prepare_ms": prepare_ms,
        "encoded_bytes": plan.prepared.encoded_bytes,
        "approved": false,
        "executed": false,
        "network_requests": endpoint.request_count(),
        "budget_boundary_tested": false
    })
}
