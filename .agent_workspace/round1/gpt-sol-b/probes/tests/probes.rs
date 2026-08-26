use std::collections::BTreeSet;

use soul_boundary_probes::{
    apply_axis_inference, correct_axis, explain_t0, score_t0, AxisPosition, AxisState, AxisUpdate,
    Band, Direction, Interaction, Venue,
};

fn interaction(
    evidence_id: u64,
    conversation_id: u64,
    peer_id: u64,
    direction: Direction,
    venue: Venue,
    occurred_at_unix: i64,
) -> Interaction {
    Interaction {
        evidence_id,
        conversation_id,
        peer_id,
        direction,
        venue,
        occurred_at_unix,
    }
}

fn no_forgotten_evidence() -> BTreeSet<u64> {
    BTreeSet::new()
}

#[test]
fn p_no_text_interaction_is_metadata_only() {
    // Compile-time-style source probe: include_str! embeds our own source in
    // this test binary, then this grep checks only Interaction's field names.
    let source = include_str!("../src/lib.rs");
    let marker = "pub struct Interaction {";
    let fields = source
        .split_once(marker)
        .expect("Interaction declaration")
        .1
        .split_once("\n}")
        .expect("Interaction closing brace")
        .0;
    let forbidden = ["body", "text", "content"];

    for line in fields.lines() {
        let code = line.split("//").next().unwrap_or("").trim();
        let Some(field) = code
            .strip_prefix("pub ")
            .and_then(|declaration| declaration.split_once(':'))
            .map(|(field, _)| field.trim().to_ascii_lowercase())
        else {
            continue;
        };
        assert!(
            !forbidden.iter().any(|word| field.contains(word)),
            "Interaction exposes forbidden prose field {field:?}"
        );
    }
}

#[test]
fn p_no_name_explanation_needs_only_peer_id() {
    let events = [interaction(
        1,
        7,
        42,
        Direction::Outgoing,
        Venue::Direct,
        1_700_000_000,
    )];
    let result = score_t0(42, &events, &no_forgotten_evidence(), 1_800_000_000).unwrap();
    let explanation = explain_t0(&result);

    assert!(explanation.contains("42"));
    for third_party_name in ["张三", "Alice", "Bob"] {
        assert!(!explanation.contains(third_party_name));
    }
}

#[test]
fn p_no_clinical_explanations_obey_denylist() {
    let fixtures = [
        (Band::Weak, 1u64),
        (Band::Moderate, 2u64),
        (Band::Strong, 3u64),
    ];
    let denylist = [
        "抑郁",
        "焦虑",
        "障碍",
        "诊断",
        "人格",
        "量表",
        "百分位",
        "score",
        "percentile",
    ];

    for (band, peer_id) in fixtures {
        let result = soul_boundary_probes::TieResult {
            peer_id,
            band,
            interaction_count: 10,
            outgoing_count: 5,
            incoming_count: 5,
            conversation_count: 1,
            active_day_count: 3,
            first_contact_unix: 0,
            last_contact_unix: 172_800,
            has_direct: true,
            evidence_ids: vec![1],
        };
        let explanation = explain_t0(&result).to_lowercase();
        for forbidden in denylist {
            assert!(
                !explanation.contains(forbidden),
                "explanation contains denylisted term {forbidden:?}: {explanation}"
            );
        }
    }
}

#[test]
fn p_dst_uses_utc_day_floor_not_us_pacific_dates() {
    // 2024-03-10 07:30Z is 2024-03-09 23:30 PST; 08:30Z is
    // 2024-03-10 00:30 PST. Pacific dates split around the DST transition,
    // while both instants are on the same UTC date.
    let events = [
        interaction(1, 1, 9, Direction::Outgoing, Venue::Direct, 1_710_055_800),
        interaction(2, 1, 9, Direction::Incoming, Venue::Direct, 1_710_059_400),
    ];
    let result = score_t0(9, &events, &no_forgotten_evidence(), 1_800_000_000).unwrap();

    assert_eq!(result.active_day_count, 1);
    assert_eq!(result.band, Band::Weak);
}

#[test]
fn p_leap_february_29_counts_as_one_active_day() {
    let events = [
        interaction(1, 1, 9, Direction::Outgoing, Venue::Direct, 1_709_165_100),
        interaction(2, 1, 9, Direction::Incoming, Venue::Direct, 1_709_250_900),
    ];
    let result = score_t0(9, &events, &no_forgotten_evidence(), 1_800_000_000).unwrap();

    assert_eq!(result.active_day_count, 1);
}

#[test]
fn p_order_shuffled_and_sorted_inputs_have_identical_results() {
    let mut sorted = Vec::new();
    for id in 0..10u64 {
        sorted.push(interaction(
            id,
            id % 2,
            77,
            if id % 2 == 0 {
                Direction::Outgoing
            } else {
                Direction::Incoming
            },
            Venue::Direct,
            (id / 4) as i64 * 86_400,
        ));
    }
    let mut shuffled = sorted.clone();
    shuffled.reverse();
    shuffled.rotate_left(3);

    let expected = score_t0(77, &sorted, &no_forgotten_evidence(), 9_999).unwrap();
    let actual = score_t0(77, &shuffled, &no_forgotten_evidence(), 9_999).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(actual.band, Band::Strong);
}

#[test]
fn p_self_reserved_owner_id_zero_is_scoreable_in_algo_layer() {
    let events = [interaction(1, 0, 0, Direction::Outgoing, Venue::Direct, 0)];
    let result = score_t0(0, &events, &no_forgotten_evidence(), 0).unwrap();

    assert_eq!(result.peer_id, 0);
    assert_eq!(result.band, Band::Weak);
}

#[test]
fn p_u64_max_identifiers_are_scoreable() {
    let events = [
        interaction(1, u64::MAX, u64::MAX, Direction::Outgoing, Venue::Direct, 0),
        interaction(2, u64::MAX, u64::MAX, Direction::Incoming, Venue::Direct, 1),
    ];
    let result = score_t0(u64::MAX, &events, &no_forgotten_evidence(), i64::MAX).unwrap();

    assert_eq!(result.peer_id, u64::MAX);
    assert_eq!(result.conversation_count, 1);
}

#[test]
fn p_negative_time_uses_euclidean_utc_floor_without_panicking() {
    // Defined behavior: -1 and -86_399 are both on UTC day -1
    // (1969-12-31), and pre-epoch evidence otherwise scores normally.
    let events = [
        interaction(1, 1, 5, Direction::Outgoing, Venue::Direct, -1),
        interaction(2, 1, 5, Direction::Incoming, Venue::Direct, -86_399),
    ];
    let result = score_t0(5, &events, &no_forgotten_evidence(), 0).unwrap();

    assert_eq!(result.active_day_count, 1);
    assert_eq!(result.first_contact_unix, -86_399);
    assert_eq!(result.last_contact_unix, -1);
}

#[test]
fn p_now_before_event_is_ignored_and_does_not_panic() {
    let events = [
        interaction(1, 1, 5, Direction::Outgoing, Venue::Direct, 2_000),
        interaction(2, 1, 5, Direction::Incoming, Venue::Direct, 2_001),
        interaction(3, 1, 5, Direction::Outgoing, Venue::Direct, 2_002),
    ];
    let skewed = score_t0(5, &events, &no_forgotten_evidence(), 1).unwrap();
    let later = score_t0(5, &events, &no_forgotten_evidence(), 9_999).unwrap();

    assert_eq!(skewed, later);
    assert_eq!(skewed.band, Band::Moderate);
}

#[test]
fn p_all_group_strong_attempt_should_not_claim_strong_tie() {
    let mut events = Vec::new();
    for day in 0..10u64 {
        for n in 0..20u64 {
            let id = day * 20 + n;
            events.push(interaction(
                id,
                88,
                99,
                if n % 2 == 0 {
                    Direction::Outgoing
                } else {
                    Direction::Incoming
                },
                Venue::Group,
                1_700_000_000 + day as i64 * 86_400 + n as i64,
            ));
        }
    }
    let result = score_t0(99, &events, &no_forgotten_evidence(), 1_800_000_000).unwrap();

    assert!(!result.has_direct);
    assert_ne!(
        result.band,
        Band::Strong,
        "PRODUCT RISK: naive T0 promotes group-only reciprocal spam to Strong"
    );
}

#[test]
fn p_lock_later_conflicting_evidence_cannot_override_correction() {
    let mut state = AxisState::default();
    correct_axis(&mut state, AxisPosition::LeansLow, 100);
    let corrected = state.clone();

    let update = apply_axis_inference(
        &mut state,
        AxisPosition::LeansHigh,
        Band::Strong,
        &[200],
        &no_forgotten_evidence(),
    );

    assert_eq!(update, AxisUpdate::RefusedAxisLocked);
    assert_eq!(state, corrected);
}

#[test]
fn p_forget_forgotten_evidence_cannot_support_the_old_band() {
    let mut events = Vec::new();
    for id in 1..=10u64 {
        let day = match id {
            1 | 2 => 0,
            3 => 1,
            _ => 2,
        };
        events.push(interaction(
            id,
            1,
            55,
            if id % 2 == 0 {
                Direction::Incoming
            } else {
                Direction::Outgoing
            },
            Venue::Direct,
            day * 86_400,
        ));
    }
    let baseline = score_t0(55, &events, &no_forgotten_evidence(), 999_999).unwrap();
    assert_eq!(baseline.band, Band::Strong);

    let forgotten: BTreeSet<u64> = (4..=10).collect();
    let after_forgetting = score_t0(55, &events, &forgotten, 999_999).unwrap();
    assert_eq!(after_forgetting.interaction_count, 3);
    assert_eq!(after_forgetting.band, Band::Moderate);
    assert!(after_forgetting.evidence_ids.iter().all(|id| *id <= 3));
}

#[test]
fn duplicate_import_with_new_evidence_ids_can_upgrade_t0() {
    let once = vec![
        interaction(1, 90, 91, Direction::Outgoing, Venue::Direct, 0),
        interaction(2, 90, 91, Direction::Incoming, Venue::Direct, 1),
    ];
    let mut imported_twice = once.clone();
    imported_twice.push(interaction(
        3,
        90,
        91,
        Direction::Outgoing,
        Venue::Direct,
        0,
    ));
    imported_twice.push(interaction(
        4,
        90,
        91,
        Direction::Incoming,
        Venue::Direct,
        1,
    ));

    assert_eq!(
        score_t0(91, &once, &no_forgotten_evidence(), 2)
            .unwrap()
            .band,
        Band::Weak
    );
    assert_eq!(
        score_t0(91, &imported_twice, &no_forgotten_evidence(), 2)
            .unwrap()
            .band,
        Band::Moderate
    );
}

#[test]
fn forgotten_axis_evidence_cannot_apply_a0_inference() {
    let mut state = AxisState::default();
    let forgotten = BTreeSet::from([300]);

    let update = apply_axis_inference(
        &mut state,
        AxisPosition::Mixed,
        Band::Moderate,
        &[300],
        &forgotten,
    );

    assert_eq!(update, AxisUpdate::RefusedNoLiveEvidence);
    assert_eq!(state, AxisState::default());
}
