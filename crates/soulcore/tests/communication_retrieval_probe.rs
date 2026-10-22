//! Finite synthetic retrieval ordering diagnostic; no production ranking changes.
#[path = "communication_retrieval_probe/cards.rs"]
mod cards;
#[path = "communication_retrieval_probe/observe.rs"]
mod observe;
#[path = "communication_retrieval_probe/strategy.rs"]
mod strategy;

#[test]
#[ignore = "finite synthetic retrieval comparison; temporary stores and unused loopback only"]
fn report() {
    let mut observations = Vec::new();
    let mut fixture_store_count = 0;
    for scene in &cards::SCENES {
        for variant in ["lexical_control", "paraphrase", "revision"] {
            for pool_size in [3, 8, 16, 32] {
                let initial = cards::card(scene, variant, pool_size, "front");
                let mut fixture = observe::Fixture::new(&initial);
                fixture_store_count += 1;
                for position in ["front", "middle", "back"] {
                    observations
                        .push(fixture.observe(&cards::card(scene, variant, pool_size, position)));
                }
            }
        }
    }
    assert_eq!(observations.len(), 108);
    assert_eq!(fixture_store_count, 36);
    let condition_count: usize = observations
        .iter()
        .map(|card| card["conditions"].as_array().unwrap().len())
        .sum();
    assert_eq!(condition_count, 432);
    for mode in ["experimental_soft_rank", "experimental_interleave"] {
        assert!(
            observations
                .iter()
                .any(|card| card["conditions"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|condition| {
                        condition["mode"] == mode && condition["required_selected"] == 0
                    })),
            "a lexical ordering must retain its constructed failure cases"
        );
    }
    println!(
        "{}",
        serde_json::json!({
            "synthetic": true,
            "card_count": observations.len(),
            "condition_count": condition_count,
            "fixture_store_count": fixture_store_count,
            "independent_statistical_samples": false,
            "noise_byte_scope": "selected_original_title_and_summary_utf8_bytes",
            "noise_bytes_are_request_bytes": false,
            "token_usage": null,
            "monetary_cost": null,
            "observations": observations,
            "generation_quality": null,
            "human_evaluation": false,
            "model_semantics_measured": false,
            "candidate_read_performance_measured": false,
            "timing_interpretation": "single_observation_not_performance_comparison",
            "ranking_timing_scope": "in_memory_fixture_ordering_only",
            "selection_timing_scope": "session_prepare_memory_context",
            "preparation_timing_scope": "session_prepare_draft_with_memory",
            "fixture_creation_in_timing": false
        })
    );
}
