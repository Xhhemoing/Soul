use soul_tie_ablation_probes::{
    explain, score, score_t3, score_t3r, score_t4, Algorithm, Band, Direction, Interaction, Venue,
};

const DAY: i64 = 86_400;
const PEER: u64 = 42;
const ALL: [Algorithm; 3] = [Algorithm::T3, Algorithm::T3R, Algorithm::T4];

fn interaction(
    evidence_id: u64,
    direction: Direction,
    venue: Venue,
    occurred_at_unix: i64,
) -> Interaction {
    Interaction {
        evidence_id,
        conversation_id: 7,
        peer_id: PEER,
        direction,
        venue,
        occurred_at_unix,
    }
}

fn reciprocal_direction(index: u64) -> Direction {
    if index % 2 == 0 {
        Direction::Outgoing
    } else {
        Direction::Incoming
    }
}

fn lilei_direct(base_unix: i64) -> Vec<Interaction> {
    let mut events = Vec::new();
    for day in 0..6u64 {
        events.push(interaction(
            day * 2,
            Direction::Outgoing,
            Venue::Direct,
            base_unix + day as i64 * DAY + 60,
        ));
        events.push(interaction(
            day * 2 + 1,
            Direction::Incoming,
            Venue::Direct,
            base_unix + day as i64 * DAY + 120,
        ));
    }
    events
}

fn group_only_50(base_unix: i64) -> Vec<Interaction> {
    (0..50u64)
        .map(|index| {
            interaction(
                index,
                reciprocal_direction(index),
                Venue::Group,
                base_unix + (index / 5) as i64 * DAY + (index % 5) as i64 * 60,
            )
        })
        .collect()
}

fn afternoon_burst(base_unix: i64) -> Vec<Interaction> {
    (0..20u64)
        .map(|index| {
            interaction(
                index,
                reciprocal_direction(index),
                Venue::Direct,
                base_unix + 13 * 3_600 + index as i64 * 60,
            )
        })
        .collect()
}

fn fixture_as_of(events: &[Interaction]) -> i64 {
    events
        .iter()
        .map(|event| event.occurred_at_unix)
        .max()
        .expect("non-empty fixture")
}

#[test]
fn group_only_50_over_10_days_is_never_strong() {
    let events = group_only_50(1_700_000_000);
    let as_of = fixture_as_of(&events);

    for algorithm in ALL {
        let result = score(algorithm, PEER, &events, as_of).expect("tie result");
        assert!(!result.any_direct);
        assert_ne!(
            result.band,
            Band::Strong,
            "{algorithm:?} must cap group-only traffic below Strong"
        );
        assert_eq!(result.band, Band::Moderate);
    }
}

#[test]
fn dormant_2019_vs_2026_is_the_recency_ablation_contrast() {
    let events = lilei_direct(1_546_300_800);
    let as_of_2026 = 1_767_225_600;

    let t3 = score_t3(PEER, &events, as_of_2026).expect("T3 result");
    let t3r = score_t3r(PEER, &events, as_of_2026).expect("T3R result");
    let t4 = score_t4(PEER, &events, as_of_2026).expect("T4 result");

    assert_eq!(
        t3.band,
        Band::Strong,
        "the no-recency T3 arm is intentionally Strong"
    );
    assert_ne!(t3r.band, Band::Strong);
    assert_ne!(t4.band, Band::Strong);
    assert_eq!(t3r.band, Band::Weak);
    assert_eq!(t4.band, Band::Weak);
}

#[test]
fn one_afternoon_burst_is_never_strong() {
    let events = afternoon_burst(1_700_006_400);
    let as_of = fixture_as_of(&events);

    for algorithm in ALL {
        let result = score(algorithm, PEER, &events, as_of).expect("tie result");
        assert_eq!(result.active_day_count, 1);
        assert_ne!(result.band, Band::Strong);
        assert_eq!(result.band, Band::Moderate);
    }
}

#[test]
fn lilei_12_over_6_days_direct_is_strong_in_all_arms() {
    let events = lilei_direct(1_700_000_000);
    let as_of = fixture_as_of(&events);

    for algorithm in ALL {
        let result = score(algorithm, PEER, &events, as_of).expect("tie result");
        assert_eq!(result.band, Band::Strong, "{algorithm:?}");
        assert_eq!(result.interaction_count, 12);
        assert_eq!(result.active_day_count, 6);
        assert!(result.any_direct);
    }
}

#[test]
fn input_order_shuffle_is_invariant_for_every_arm() {
    let ordered = lilei_direct(1_700_000_000);
    let mut shuffled = ordered.clone();
    shuffled.reverse();
    shuffled.rotate_left(5);
    let as_of = fixture_as_of(&ordered);

    for algorithm in ALL {
        let expected = score(algorithm, PEER, &ordered, as_of);
        let actual = score(algorithm, PEER, &shuffled, as_of);
        assert_eq!(actual, expected, "{algorithm:?} changed after shuffling");
    }
}

#[test]
fn active_days_use_utc_not_a_local_timezone() {
    // These are 2024-03-10 07:30Z and 08:30Z. They straddle midnight in
    // US Pacific time around the DST transition but share one UTC date.
    let events = vec![
        interaction(1, Direction::Outgoing, Venue::Direct, 1_710_055_800),
        interaction(2, Direction::Incoming, Venue::Direct, 1_710_059_400),
    ];

    for algorithm in ALL {
        let result = score(algorithm, PEER, &events, 1_710_059_400).expect("tie result");
        assert_eq!(result.active_day_count, 1, "{algorithm:?}");
        assert_eq!(result.band, Band::Weak);
    }
}

#[test]
fn negative_unix_instants_do_not_panic() {
    let events = lilei_direct(-6 * DAY);

    for algorithm in ALL {
        let result = score(algorithm, PEER, &events, 0).expect("tie result");
        assert_eq!(result.band, Band::Strong, "{algorithm:?}");
        assert_eq!(result.active_day_count, 6);
    }

    let extreme = vec![
        interaction(100, Direction::Outgoing, Venue::Direct, i64::MIN),
        interaction(
            101,
            Direction::Incoming,
            Venue::Direct,
            i64::MIN.saturating_add(DAY),
        ),
    ];
    for algorithm in ALL {
        assert!(score(algorithm, PEER, &extreme, i64::MAX).is_some());
    }
}

#[test]
fn as_of_before_events_clamps_age_to_zero_without_panicking() {
    let events = lilei_direct(1_700_000_000);

    for algorithm in ALL {
        let result = score(algorithm, PEER, &events, 1).expect("tie result");
        assert_eq!(result.last_contact_age_days, 0, "{algorithm:?}");
        assert_eq!(result.band, Band::Strong, "{algorithm:?}");
        assert_eq!(result.weighted_interaction_milli, 12_000);
    }
}

#[test]
fn interaction_source_grep_rejects_message_body_fields() {
    let source = include_str!("../src/lib.rs");
    let fields = source
        .split_once("pub struct Interaction {")
        .expect("Interaction declaration")
        .1
        .split_once("\n}")
        .expect("Interaction closing brace")
        .0;
    let forbidden = ["body", "text", "content", "message", "payload", "prose"];

    for line in fields.lines() {
        let code = line.split("//").next().unwrap_or("").trim();
        let Some(field_name) = code
            .strip_prefix("pub ")
            .and_then(|declaration| declaration.split_once(':'))
            .map(|(name, _)| name.trim().to_ascii_lowercase())
        else {
            continue;
        };
        assert!(
            !forbidden.iter().any(|word| field_name.contains(word)),
            "Interaction exposes forbidden field {field_name:?}"
        );
    }
}

#[test]
fn every_explanation_obeys_the_language_denylist() {
    let fixtures = [
        vec![interaction(
            1,
            Direction::Outgoing,
            Venue::Group,
            1_700_000_000,
        )],
        afternoon_burst(1_700_006_400),
        lilei_direct(1_700_000_000),
    ];
    let forbidden = [
        "诊断",
        "抑郁",
        "焦虑",
        "障碍",
        "人格",
        "量表",
        "百分位",
        "score",
        "percentile",
    ];

    for events in fixtures {
        let as_of = fixture_as_of(&events);
        for algorithm in ALL {
            let result = score(algorithm, PEER, &events, as_of).expect("tie result");
            let rendered = explain(algorithm, &result).to_lowercase();
            for denied in forbidden {
                assert!(
                    !rendered.contains(denied),
                    "{algorithm:?} explanation contains {denied:?}: {rendered}"
                );
            }
        }
    }
}

#[test]
fn t3r_uses_exact_integer_milli_buckets() {
    let as_of = 400 * DAY;
    let ages = [0, 90, 180, 359, 360];
    let events: Vec<_> = ages
        .into_iter()
        .enumerate()
        .map(|(index, age_days)| {
            interaction(
                index as u64,
                reciprocal_direction(index as u64),
                Venue::Direct,
                as_of - age_days * DAY,
            )
        })
        .collect();
    let result = score_t3r(PEER, &events, as_of).expect("T3R result");
    let integer_milli: u64 = result.weighted_interaction_milli;

    assert_eq!(integer_milli, 1_000 + 500 + 250 + 250);
    assert_eq!(result.observed_interaction_count, 5);
    assert_eq!(result.interaction_count, 4);

    let source = include_str!("../src/lib.rs");
    assert!(!source.contains("f32"));
    assert!(!source.contains("f64"));
    assert!(!source.contains("NAN"));
    assert!(!source.contains("INFINITY"));
}

#[test]
fn t4_integer_recency_boundaries_match_the_written_rule() {
    let events = lilei_direct(1_700_000_000);
    let latest = fixture_as_of(&events);

    assert_eq!(
        score_t4(PEER, &events, latest + 180 * DAY)
            .expect("180-day result")
            .band,
        Band::Strong
    );
    assert_eq!(
        score_t4(PEER, &events, latest + 180 * DAY + 1)
            .expect("post-180-day result")
            .band,
        Band::Moderate
    );
    assert_eq!(
        score_t4(PEER, &events, latest + 360 * DAY)
            .expect("360-day result")
            .band,
        Band::Weak
    );
}
