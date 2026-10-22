use super::{
    presentation::*,
    records::*,
    synthetic_cards::{CardId, CARDS},
};

pub(super) fn t1_feedback() -> SimulatedFeedback {
    let side = || SimulatedSideFeedback {
        completed: true,
        first_pass: Measured::Value(false),
        edit_seconds: Measured::Value(10),
        pause_seconds: Measured::Value(0),
        help_count: Measured::Value(0),
        unsupported_claim: Measured::NotApplicable,
        fact_errors: Measured::NotApplicable,
        safety_stop: false,
        t1_slot_filled: true,
        missing_reason: None,
    };
    SimulatedFeedback {
        preference: Preference::Tie,
        blind_status: BlindStatus::Intact,
        sides: [side(), side()],
    }
}

#[test]
fn five_cards_generate_real_unrated_candidates_without_simulated_feedback() {
    let mut simulation = Simulation::new();
    simulation
        .generate_all(11, SimulatedApproval::ApproveExact)
        .expect("synthetic generation");
    for card in CARDS {
        simulation
            .present(card, 12, |view| {
                assert!(!view.card.is_empty() && !view.instruction.is_empty());
                assert!(!view.left_text.is_empty() && !view.right_text.is_empty());
            })
            .expect("synthetic presentation");
    }
    let report = simulation.operator_summary().expect("synthetic report");
    assert_eq!(report["summary"]["funnel"]["assigned"], 5);
    assert_eq!(report["summary"]["generated_candidates"], 10);
    assert_eq!(report["summary"]["funnel"]["rated"], 0);
    assert_eq!(report["summary"]["funnel"]["completed"], 0);
    assert_eq!(report["counts"]["http_requests"], 8);
}

#[test]
fn blind_view_requires_presentation_then_one_feedback_before_reveal() {
    let mut simulation = Simulation::new();
    assert!(simulation.reveal(CardId::S1).is_err());
    simulation
        .generate_all(11, SimulatedApproval::ApproveExact)
        .expect("synthetic generation");
    assert!(simulation.submit(CardId::S1, 12, t1_feedback()).is_err());
    assert!(simulation.reveal(CardId::S1).is_err());
    simulation
        .present(CardId::S1, 12, |view| {
            assert!(!view.left_text.is_empty() && !view.right_text.is_empty());
        })
        .expect("blind view");
    simulation
        .submit(CardId::S1, 13, t1_feedback())
        .expect("explicit simulated feedback");
    assert!(simulation.reveal(CardId::S1).is_ok());
    assert!(simulation.submit(CardId::S1, 14, t1_feedback()).is_err());
}

fn silent_view(view: BlindPairView<'_>) {
    // Exhaustive destructuring documents the entire callback input surface.
    let BlindPairView {
        card,
        instruction,
        left_text,
        right_text,
    } = view;
    assert!(!card.is_empty() && !instruction.is_empty());
    assert!(!left_text.is_empty() && !right_text.is_empty());
}

#[test]
fn explicit_denial_sends_zero_requests_and_keeps_all_registered_denominators() {
    let mut simulation = Simulation::new();
    assert!(simulation
        .generate(CardId::S2, 11, SimulatedApproval::Deny)
        .is_err());
    assert!(simulation
        .present(CardId::S2, 12, |_| panic!("denied view"))
        .is_err());
    let report = simulation.operator_summary().unwrap();
    assert_eq!(report["counts"]["http_requests"], 0);
    assert_eq!(report["summary"]["funnel"]["assigned"], 5);
    assert_eq!(report["summary"]["funnel"]["started"], 1);
    assert_eq!(report["summary"]["generated_candidates"], 0);
    assert_eq!(report["summary"]["funnel"]["completed"], 0);
    assert!(simulation
        .generate(CardId::S2, 12, SimulatedApproval::ApproveExact)
        .is_err());
    assert_eq!(
        simulation.operator_summary().unwrap()["counts"]["http_requests"],
        0
    );
}

#[test]
fn generation_admission_expiry_and_withdrawal_do_not_send_or_drop_slots() {
    for withdrawn in [false, true] {
        let mut simulation = Simulation::new();
        if withdrawn {
            simulation.withdraw();
        }
        assert!(simulation
            .generate_all(
                if withdrawn { 11 } else { 100 },
                SimulatedApproval::ApproveExact
            )
            .is_err());
        let report = simulation.operator_summary().unwrap();
        assert_eq!(report["counts"]["http_requests"], 0);
        assert_eq!(report["summary"]["funnel"]["assigned"], 5);
        assert_eq!(report["summary"]["funnel"]["started"], 0);
        assert_eq!(report["summary"]["generated_candidates"], 0);
        // Moving the supplied synthetic time backward cannot reactivate views.
        assert!(simulation
            .generate(CardId::S2, 11, SimulatedApproval::ApproveExact)
            .is_err());
    }
}

#[test]
fn failed_b_never_falls_back_or_positional_maps_a_single_local_draft() {
    let mut simulation = Simulation::with_invalid_b();
    assert!(simulation
        .generate(CardId::S1, 11, SimulatedApproval::ApproveExact)
        .is_err());
    assert!(simulation
        .present(CardId::S1, 12, |_| panic!("partial local view"))
        .is_err());
    let report = simulation.operator_summary().unwrap();
    assert_eq!(report["counts"]["local_draft_calls"], 1);
    assert_eq!(report["summary"]["generated_candidates"], 1);
    assert_eq!(report["summary"]["funnel"]["generated"], 0);
    assert!(simulation.reveal(CardId::S1).is_err());
    assert!(simulation
        .generate(CardId::S3, 11, SimulatedApproval::ApproveExact)
        .is_err());
    let report = simulation.operator_summary().unwrap();
    assert_eq!(report["counts"]["http_requests"], 1);
    assert_eq!(report["summary"]["generated_candidates"], 2);
    assert!(simulation
        .present(CardId::S3, 12, |_| panic!("partial endpoint view"))
        .is_err());
}

#[test]
fn invalid_feedback_is_atomic_and_does_not_enable_reveal_or_fake_t1_completion() {
    let mut simulation = Simulation::new();
    simulation
        .generate(CardId::S1, 11, SimulatedApproval::ApproveExact)
        .unwrap();
    simulation.present(CardId::S1, 12, silent_view).unwrap();
    let before = simulation.operator_summary().unwrap();
    let mut invalid = t1_feedback();
    invalid.sides[0].t1_slot_filled = false;
    assert!(simulation.submit(CardId::S1, 13, invalid).is_err());
    assert!(simulation.operator_summary().unwrap() == before);
    assert!(simulation.reveal(CardId::S1).is_err());
    let mut invalid = t1_feedback();
    invalid.sides[0].help_count = Measured::Missing(MissingReason::NotRated);
    assert!(simulation.submit(CardId::S1, 13, invalid).is_err());
    assert!(simulation.operator_summary().unwrap() == before);
    simulation.submit(CardId::S1, 13, t1_feedback()).unwrap();
    assert!(simulation
        .present(CardId::S1, 14, |_| panic!("scored output reused"))
        .is_err());
    assert!(simulation.submit(CardId::S1, 14, t1_feedback()).is_err());
    assert!(simulation.reveal(CardId::S1).is_ok());
    assert!(simulation.reveal(CardId::S1).is_err());
}

#[test]
fn filling_t1_body_slot_cannot_relabel_original_draft_as_first_pass_accepted() {
    let mut simulation = Simulation::new();
    simulation
        .generate(CardId::S1, 11, SimulatedApproval::ApproveExact)
        .unwrap();
    simulation.present(CardId::S1, 12, silent_view).unwrap();
    let before = simulation.operator_summary().unwrap();
    let mut feedback = t1_feedback();
    assert!(feedback.sides[0].t1_slot_filled);
    feedback.sides[0].first_pass = Measured::Value(true);
    assert!(simulation.submit(CardId::S1, 13, feedback).is_err());
    assert!(simulation.operator_summary().unwrap() == before);
    assert!(simulation.reveal(CardId::S1).is_err());
}

#[test]
fn task_specific_factual_requirements_cannot_be_bypassed_by_simulated_feedback() {
    for card in [CardId::S2, CardId::S3] {
        let mut simulation = Simulation::new();
        simulation
            .generate(card, 11, SimulatedApproval::ApproveExact)
            .unwrap();
        simulation.present(card, 12, silent_view).unwrap();
        let before = simulation.operator_summary().unwrap();
        let mut feedback = super::synthetic_entry::scripted_feedback(card);
        if card == CardId::S2 {
            feedback.sides[0].unsupported_claim = Measured::Value(true);
        } else {
            feedback.sides[0].fact_errors = Measured::Missing(MissingReason::NotRated);
        }
        assert!(simulation.submit(card, 13, feedback).is_err());
        assert!(simulation.operator_summary().unwrap() == before);
        assert!(simulation.reveal(card).is_err());
    }
}

#[test]
fn late_or_withdrawn_feedback_invalidates_text_and_cannot_be_replayed() {
    for withdrawn in [false, true] {
        let mut simulation = Simulation::new();
        simulation
            .generate(CardId::S1, 11, SimulatedApproval::ApproveExact)
            .unwrap();
        simulation.present(CardId::S1, 12, silent_view).unwrap();
        if withdrawn {
            simulation.withdraw();
        }
        assert!(simulation
            .submit(CardId::S1, if withdrawn { 13 } else { 100 }, t1_feedback())
            .is_err());
        assert!(simulation
            .present(CardId::S1, 12, |_| panic!("invalidated text reused"))
            .is_err());
        assert!(simulation.submit(CardId::S1, 13, t1_feedback()).is_err());
        assert!(simulation.reveal(CardId::S1).is_err());
        assert_eq!(
            simulation.operator_summary().unwrap()["summary"]["funnel"]["completed"],
            0
        );
    }
}

#[test]
fn generated_text_is_not_presented_after_deadline_or_withdrawal() {
    for withdrawn in [false, true] {
        let mut simulation = Simulation::new();
        simulation
            .generate(CardId::S2, 11, SimulatedApproval::ApproveExact)
            .unwrap();
        if withdrawn {
            simulation.withdraw();
        }
        assert!(simulation
            .present(CardId::S2, if withdrawn { 12 } else { 100 }, |_| panic!(
                "closed run presented text"
            ))
            .is_err());
        assert!(simulation
            .present(CardId::S2, 12, |_| panic!("invalidated text reactivated"))
            .is_err());
        assert_eq!(
            simulation.operator_summary().unwrap()["counts"]["presentations"],
            0
        );
    }
}

#[test]
fn reversed_sides_bind_feedback_to_registered_variants_only_after_reveal() {
    let mut simulation = Simulation::new();
    simulation
        .generate_all(11, SimulatedApproval::ApproveExact)
        .unwrap();
    assert!(simulation.revealed_metadata().is_err());
    for card in CARDS {
        simulation.present(card, 12, silent_view).unwrap();
        let mut feedback = super::synthetic_entry::scripted_feedback(card);
        if card == CardId::S3 {
            feedback.sides[0].fact_errors = Measured::Value(2);
            feedback.sides[1].fact_errors = Measured::Value(0);
            feedback.sides[1].first_pass = Measured::Value(true);
        }
        simulation.submit(card, 13, feedback).unwrap();
        let mapping = simulation.reveal(card).unwrap();
        if card == CardId::S3 {
            assert_eq!(mapping, [Variant::C, Variant::B]);
        }
    }
    let metadata = simulation.revealed_metadata().unwrap();
    let rows = metadata["candidates"].as_array().unwrap();
    let c = rows
        .iter()
        .find(|row| row["pair_id"] == "synthetic-S3" && row["requested_variant"] == "c")
        .unwrap();
    let b = rows
        .iter()
        .find(|row| row["pair_id"] == "synthetic-S3" && row["requested_variant"] == "b")
        .unwrap();
    assert_eq!(c["required_fact_error_count"]["value"], 2);
    assert_eq!(b["required_fact_error_count"]["value"], 0);
    assert_eq!(b["first_pass_accepted"]["value"], true);
}

#[test]
fn safety_feedback_pauses_same_path_outputs_and_preserves_other_path() {
    let mut simulation = Simulation::new();
    simulation
        .generate_all(11, SimulatedApproval::ApproveExact)
        .unwrap();
    simulation.present(CardId::S3, 12, silent_view).unwrap();
    let mut feedback = super::synthetic_entry::scripted_feedback(CardId::S3);
    let side = &mut feedback.sides[0];
    side.completed = false;
    side.safety_stop = true;
    side.edit_seconds = Measured::Missing(MissingReason::NotCompleted);
    side.pause_seconds = Measured::Missing(MissingReason::NotCompleted);
    side.missing_reason = Some(MissingReason::NotCompleted);
    simulation.submit(CardId::S3, 13, feedback).unwrap();
    assert!(simulation
        .present(CardId::S4, 14, |_| panic!("paused path text"))
        .is_err());
    assert!(simulation.reveal(CardId::S3).is_err());
    simulation.present(CardId::S2, 14, silent_view).unwrap();
    let report = simulation.operator_summary().unwrap();
    assert_eq!(report["summary"]["funnel"]["assigned"], 5);
    assert_eq!(report["summary"]["funnel"]["completed"], 0);
}

#[test]
fn explicit_scripted_entry_keeps_missing_timing_and_separates_metadata_from_generate_only() {
    let generated = super::synthetic_entry::run("generate_only").unwrap();
    assert_eq!(generated["synthetic"], true);
    assert_eq!(generated["human_evaluation"], false);
    assert!(generated.get("metadata").is_none());
    assert_eq!(generated["counts"]["presentations"], 5);
    assert_eq!(generated["counts"]["feedback_records"], 0);
    let scripted = super::synthetic_entry::run("scripted_feedback").unwrap();
    assert_eq!(scripted["feedback_origin"], "preset_simulated");
    assert_eq!(scripted["summary"]["funnel"]["completed"], 5);
    assert_eq!(scripted["counts"]["reveals"], 5);
    assert_eq!(scripted["counts"]["http_requests"], 8);
    let pairs = scripted["summary"]["pairs"].as_array().unwrap();
    assert_eq!(
        pairs
            .iter()
            .map(|p| p["summary"]["edit_missing_pairs"].as_u64().unwrap())
            .sum::<u64>(),
        1
    );
    assert_eq!(
        scripted["metadata"]["candidates"].as_array().unwrap().len(),
        10
    );
    for report in [&generated, &scripted] {
        let output = report.to_string();
        for id in CARDS {
            let card = id.card();
            assert!(!output.contains(card.paste));
            if !card.reply.is_empty() {
                assert!(!output.contains(card.reply));
            }
        }
    }
    assert!(super::synthetic_entry::run("human").is_err());
}
