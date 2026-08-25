use soul_tie_round3_probes::{t4, t4d, Band, Direction, Interaction, Venue, DAY_SECONDS};

const NOW: i64 = 21_000 * DAY_SECONDS;
const F_OLD_LAST: i64 = 1_560_124_800; // 2019-06-10T00:00:00Z
const AS_OF_2026: i64 = 1_787_529_600; // 2026-08-24T00:00:00Z

fn interaction(age_days: u64, direction: Direction, venue: Venue) -> Interaction {
    Interaction {
        occurred_at: NOW - (age_days as i64 * DAY_SECONDS),
        direction,
        venue,
    }
}

fn strong_direct_with_latest_age(age_days: u64) -> Vec<Interaction> {
    (0..12)
        .map(|index| {
            interaction(
                age_days + (index % 3) as u64,
                if index % 2 == 0 {
                    Direction::Incoming
                } else {
                    Direction::Outgoing
                },
                Venue::Direct,
            )
        })
        .collect()
}

fn lilei_12() -> Vec<Interaction> {
    (0..12)
        .map(|index| {
            interaction(
                3 + (index % 6) as u64,
                if index % 2 == 0 {
                    Direction::Incoming
                } else {
                    Direction::Outgoing
                },
                Venue::Direct,
            )
        })
        .collect()
}

fn f_heavy() -> Vec<Interaction> {
    let mut events: Vec<_> = (0..100)
        .map(|index| {
            interaction(
                (index % 50) as u64,
                if index % 2 == 0 {
                    Direction::Incoming
                } else {
                    Direction::Outgoing
                },
                Venue::Group,
            )
        })
        .collect();
    events.push(interaction(0, Direction::Incoming, Venue::Direct));
    events.push(interaction(1, Direction::Outgoing, Venue::Direct));
    events
}

fn group_only() -> Vec<Interaction> {
    (0..50)
        .map(|index| {
            interaction(
                (index % 10) as u64,
                if index % 2 == 0 {
                    Direction::Incoming
                } else {
                    Direction::Outgoing
                },
                Venue::Group,
            )
        })
        .collect()
}

fn f_old() -> Vec<Interaction> {
    (0..20)
        .map(|index| Interaction {
            occurred_at: F_OLD_LAST - (index % 10) as i64 * DAY_SECONDS,
            direction: if index % 2 == 0 {
                Direction::Incoming
            } else {
                Direction::Outgoing
            },
            venue: Venue::Direct,
        })
        .collect()
}

fn shuffled(mut events: Vec<Interaction>, seed: u64) -> Vec<Interaction> {
    let mut state = seed;
    for upper in (1..events.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        events.swap(upper, (state as usize) % (upper + 1));
    }
    events
}

#[test]
fn boundary_179_keeps_t4_and_t4d_strong() {
    let events = strong_direct_with_latest_age(179);
    let a = t4(&events, Some(NOW));
    let b = t4d(&events, Some(NOW));

    assert_eq!(a.age_days, Some(179));
    assert_eq!(b.age_days, Some(179));
    assert_eq!(a.band, Band::Strong);
    assert_eq!(b.band, Band::Strong);
}

#[test]
fn boundary_180_demotes_t4_and_t4d() {
    let events = strong_direct_with_latest_age(180);
    let a = t4(&events, Some(NOW));
    let b = t4d(&events, Some(NOW));

    assert_eq!(a.age_days, Some(180));
    assert_eq!(b.age_days, Some(180));
    assert_eq!(a.band, Band::Moderate);
    assert_eq!(b.band, Band::Moderate);
}

#[test]
fn boundary_359_still_demotes_only_one_band() {
    let events = strong_direct_with_latest_age(359);

    assert_eq!(t4(&events, Some(NOW)).band, Band::Moderate);
    assert_eq!(t4d(&events, Some(NOW)).band, Band::Moderate);
}

#[test]
fn boundary_360_forces_t4_and_t4d_weak() {
    let events = strong_direct_with_latest_age(360);

    assert_eq!(t4(&events, Some(NOW)).band, Band::Weak);
    assert_eq!(t4d(&events, Some(NOW)).band, Band::Weak);
}

#[test]
fn f_heavy_group_fanout_plus_two_directs_separates_t4d() {
    let events = f_heavy();
    let original = t4(&events, Some(NOW));
    let defended = t4d(&events, Some(NOW));

    assert_eq!(original.band, Band::Strong);
    assert_eq!(original.total_count, 102);
    assert_eq!(defended.direct_count, 2);
    assert_ne!(defended.band, Band::Strong);
}

#[test]
fn lilei_is_strong_in_t4_and_t4d() {
    let events = lilei_12();
    let original = t4(&events, Some(NOW));
    let defended = t4d(&events, Some(NOW));

    assert_eq!(original.direct_count, 12);
    assert_eq!(original.direct_active_days, 6);
    assert_eq!(original.band, Band::Strong);
    assert_eq!(defended.band, Band::Strong);
}

#[test]
fn group_only_is_never_strong_in_either_candidate() {
    let events = group_only();

    assert_eq!(t4(&events, Some(NOW)).band, Band::Moderate);
    assert_eq!(t4d(&events, Some(NOW)).band, Band::Weak);
}

#[test]
fn dormant_2019_is_weak_with_the_explicit_2026_as_of() {
    let events = f_old();
    let original = t4(&events, Some(AS_OF_2026));
    let defended = t4d(&events, Some(AS_OF_2026));

    assert_eq!(original.age_days, Some(2_632));
    assert_eq!(defended.age_days, Some(2_632));
    assert_eq!(original.band, Band::Weak);
    assert_eq!(defended.band, Band::Weak);
}

#[test]
fn scoring_is_shuffle_invariant() {
    let events = f_heavy();
    let original_t4 = t4(&events, Some(NOW));
    let original_t4d = t4d(&events, Some(NOW));

    for seed in 0..64 {
        let permutation = shuffled(events.clone(), seed);
        assert_eq!(t4(&permutation, Some(NOW)), original_t4);
        assert_eq!(t4d(&permutation, Some(NOW)), original_t4d);
    }
}

#[test]
fn interaction_record_is_exhaustively_metadata_only() {
    let sample = interaction(0, Direction::Incoming, Venue::Direct);

    // This pattern stops compiling if the public record gains another field.
    let Interaction {
        occurred_at,
        direction,
        venue,
    } = sample;
    assert_eq!(occurred_at, NOW);
    assert_eq!(direction, Direction::Incoming);
    assert_eq!(venue, Venue::Direct);
}

#[test]
fn public_output_has_only_neutral_band_words() {
    let fixtures = [lilei_12(), group_only(), f_old()];
    for events in &fixtures {
        for band in [
            t4(events, Some(AS_OF_2026)).band,
            t4d(events, Some(AS_OF_2026)).band,
        ] {
            assert!(matches!(band.as_str(), "weak" | "moderate" | "strong"));
        }
    }
}

#[test]
fn t4d_strong_never_has_fewer_than_ten_direct_events() {
    for direct_count in 0..10 {
        for group_count in [0, 1, 12, 100] {
            let mut events: Vec<_> = (0..direct_count)
                .map(|index| {
                    interaction(
                        (index % 3) as u64,
                        if index % 2 == 0 {
                            Direction::Incoming
                        } else {
                            Direction::Outgoing
                        },
                        Venue::Direct,
                    )
                })
                .collect();
            events.extend((0..group_count).map(|index| {
                interaction(
                    (index % 5) as u64,
                    if index % 2 == 0 {
                        Direction::Incoming
                    } else {
                        Direction::Outgoing
                    },
                    Venue::Group,
                )
            }));

            let result = t4d(&events, Some(NOW));
            assert!(result.direct_count < 10);
            assert_ne!(
                result.band,
                Band::Strong,
                "direct_count={direct_count}, group_count={group_count}"
            );
        }
    }
}

#[test]
fn trap_peer_local_fixture_max_wrongly_revives_f_old() {
    let events = f_old();

    // Wrong: omitting the rebuild-wide 2026 cutoff makes this peer's own
    // latest 2019 event become its cutoff, erasing 2,632 days of dormancy.
    let peer_local_fallback = t4(&events, None);
    assert_eq!(peer_local_fallback.resolved_as_of, Some(F_OLD_LAST));
    assert_eq!(peer_local_fallback.age_days, Some(0));
    assert_eq!(peer_local_fallback.band, Band::Strong);

    // Correct: every peer receives the same caller-supplied rebuild cutoff.
    assert_eq!(t4(&events, Some(AS_OF_2026)).band, Band::Weak);
}
