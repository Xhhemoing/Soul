//! Explicit ignored synthetic entry. No human evaluation is implied.
use super::{
    presentation::*,
    records::*,
    synthetic_cards::{CardId, CARDS},
};

/// Explicit preset observations for a control-flow exercise, never inferred
/// from generated prose or counted as a human score.
pub(super) fn scripted_feedback(card: CardId) -> SimulatedFeedback {
    let task = card.card().task;
    let side = |index| SimulatedSideFeedback {
        completed: true,
        first_pass: Measured::Value(false),
        edit_seconds: if card == CardId::S3 && index == 0 {
            Measured::Missing(MissingReason::NotMeasured)
        } else {
            Measured::Value(10)
        },
        pause_seconds: Measured::Value(0),
        help_count: Measured::Value(0),
        unsupported_claim: if task == TaskType::T1 {
            Measured::NotApplicable
        } else {
            Measured::Value(false)
        },
        fact_errors: if task == TaskType::T3 {
            Measured::Value(0)
        } else {
            Measured::NotApplicable
        },
        safety_stop: false,
        t1_slot_filled: task == TaskType::T1,
        missing_reason: if card == CardId::S3 && index == 0 {
            Some(MissingReason::NotMeasured)
        } else {
            None
        },
    };
    SimulatedFeedback {
        preference: Preference::Tie,
        blind_status: BlindStatus::Intact,
        sides: [side(0), side(1)],
    }
}

pub(super) fn run(mode: &str) -> Result<serde_json::Value, &'static str> {
    if !matches!(mode, "generate_only" | "scripted_feedback") {
        return Err("invalid_mode");
    }
    let mut simulation = Simulation::new();
    simulation
        .generate_all(11, SimulatedApproval::ApproveExact)
        .map_err(|_| "generation")?;
    for card in CARDS {
        simulation
            .present(card, 12, |view| {
                // Synthetic renderer consumes the borrowed shape only; no GUI,
                // terminal body printing, extra metadata, or claims of human sight.
                std::hint::black_box((
                    view.card,
                    view.instruction,
                    view.left_text,
                    view.right_text,
                ));
            })
            .map_err(|_| "presentation")?;
        if mode == "scripted_feedback" {
            simulation
                .submit(card, 13, scripted_feedback(card))
                .map_err(|_| "feedback")?;
            simulation.reveal(card).map_err(|_| "reveal")?;
        }
    }
    let mut report = simulation.operator_summary().map_err(|_| "metadata")?;
    report["mode"] = serde_json::json!(mode);
    report["feedback_origin"] = serde_json::json!(if mode == "scripted_feedback" {
        "preset_simulated"
    } else {
        "none"
    });
    if mode == "scripted_feedback" {
        report["metadata"] = simulation.revealed_metadata().map_err(|_| "metadata")?;
    }
    Ok(report)
}

#[test]
#[ignore = "explicit synthetic exercise mode required"]
fn exercise() -> Result<(), ()> {
    let result = match std::env::var("SOUL_VAL01_EXERCISE_MODE") {
        Ok(mode) => run(&mode),
        Err(_) => Err("mode_not_specified"),
    };
    let output = match &result {
        Ok(report) => report.clone(),
        Err(category) => serde_json::json!({"synthetic": true, "error": {"category": category}}),
    };
    let json = serde_json::to_string_pretty(&output).map_err(|_| ())?;
    println!("VAL01_EXERCISE_BEGIN\n{json}\nVAL01_EXERCISE_END");
    result.map(|_| ()).map_err(|_| ())
}
