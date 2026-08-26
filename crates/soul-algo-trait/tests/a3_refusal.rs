//! A3 refuses. Round 1 killed it; this is the test that keeps it dead.

use soul_algo_trait::a3::{
    a3_from_message_text, a3_from_messages, is_rejected_algorithm, A3Refused, A3_ALGORITHM_ID,
    A3_REFUSAL_REASON,
};
use soul_algo_trait::types::AxisId;
use soul_algo_trait::{A0_ALGORITHM_ID, A1_ALGORITHM_ID, A2_ALGORITHM_ID};

#[test]
fn text_inference_always_refuses() {
    for text in [
        "",
        "今天开了一整天会，累。",
        "I would rather stay in tonight.",
        "焦虑",
    ] {
        for axis in AxisId::ALL {
            assert_eq!(
                a3_from_message_text(axis, text, &[1, 2, 3]),
                Err(A3Refused::new())
            );
        }
    }
}

#[test]
fn the_batch_form_refuses_too() {
    let messages = ["一", "二", "三"];
    assert_eq!(
        a3_from_messages(AxisId::SocialEnergy, &messages, &[1]),
        Err(A3Refused::new())
    );
}

#[test]
fn plenty_of_evidence_does_not_buy_a_text_inference() {
    let ids: Vec<u64> = (0..1_000).collect();
    assert!(a3_from_message_text(AxisId::Curiosity, "很多字", &ids).is_err());
}

#[test]
fn the_refusal_reason_is_stable_and_holds_no_text() {
    let refused = A3Refused::default();
    assert_eq!(refused.reason, A3_REFUSAL_REASON);
    assert_eq!(refused.to_string(), "v0.1_forbids_text_trait_inference");
}

#[test]
fn the_a3_identifier_is_registered_as_rejected() {
    assert!(is_rejected_algorithm(A3_ALGORITHM_ID));
    for live in [A0_ALGORITHM_ID, A1_ALGORITHM_ID, A2_ALGORITHM_ID] {
        assert!(!is_rejected_algorithm(live), "{live}");
    }
}

#[test]
fn no_live_algorithm_stamps_the_rejected_identifier() {
    use soul_algo_trait::a0::a0_all_axes;
    use soul_algo_trait::a1::a1_all_axes;
    use soul_algo_trait::fixtures::axis_evidence_matrix;

    for (name, log) in axis_evidence_matrix() {
        for outcome in a0_all_axes(&log) {
            assert!(!is_rejected_algorithm(outcome.state.algorithm_id), "{name}");
        }
        for outcome in a1_all_axes(&log) {
            assert!(!is_rejected_algorithm(outcome.state.algorithm_id), "{name}");
        }
    }
}
