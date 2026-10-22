//! Explicit, finite material diagnostics; not a model or human-effectiveness test.
#[path = "communication_material_probe/cards.rs"]
mod cards;
#[path = "communication_material_probe/context.rs"]
mod context;
#[path = "communication_material_probe/selection.rs"]
mod selection;
#[path = "communication_material_probe/support.rs"]
mod support;

#[test]
#[ignore = "finite synthetic communication material diagnostic; only temporary stores and loopback"]
fn report() {
    let selection = cards::cards()
        .iter()
        .map(selection::observe)
        .collect::<Vec<_>>();
    let observed = cards::SCENES
        .iter()
        .map(context::observe)
        .collect::<Vec<_>>();
    assert!(observed.windows(2).all(|pair| pair[0].1 == pair[1].1));
    let context = observed
        .into_iter()
        .map(|(record, _)| record)
        .collect::<Vec<_>>();
    assert_eq!(selection.len(), 15);
    assert_eq!(context.len(), 3);
    let prepared_selection_conditions = selection
        .iter()
        .flat_map(|card| card["selection_conditions"].as_array().expect("conditions"))
        .filter(|condition| !condition["preparation"].is_null())
        .count();
    println!(
        "{}",
        serde_json::json!({
            "synthetic": true,
            "human_evaluation": false,
            "model_semantics_measured": false,
            "selection_cards": selection,
            "prepared_selection_conditions": prepared_selection_conditions,
            "context_pairs": context,
            "same_voice_different_goals_local_templates_equal": true,
            "generation_quality": null,
            "user_minutes": null,
            "monetary_cost": null,
            "timing_interpretation": "single_observation_not_performance_comparison"
        })
    );
}
