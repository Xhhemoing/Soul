use soul_algo_tie::{Band, Interaction, TieAlgorithm, SECONDS_PER_DAY, T4D};

const PEER: u64 = 7;
const AS_OF: i64 = 20_000 * SECONDS_PER_DAY;

fn row(outgoing: bool, occurred_at_unix: i64, venue_direct: bool) -> Interaction {
    Interaction {
        peer_id: PEER,
        outgoing,
        occurred_at_unix,
        venue_direct,
        conversation_id: if venue_direct { 1 } else { 2 },
    }
}

fn direct(outgoing: bool, days_ago: i64) -> Interaction {
    row(outgoing, AS_OF - days_ago * SECONDS_PER_DAY, true)
}

fn group(outgoing: bool, days_ago: i64) -> Interaction {
    row(outgoing, AS_OF - days_ago * SECONDS_PER_DAY, false)
}

/// Twelve reciprocal direct exchanges spread over six UTC days, with the
/// newest exchange exactly `latest_days_ago` whole days before `AS_OF`.
fn lilei(latest_days_ago: i64) -> Vec<Interaction> {
    (0..12)
        .map(|index| direct(index % 2 == 0, latest_days_ago + index % 6))
        .collect()
}

fn heavy_group_plus_two_directs() -> Vec<Interaction> {
    let mut log: Vec<_> = (0..100)
        .map(|index| group(index % 2 == 0, index % 50))
        .collect();
    log.push(direct(true, 0));
    log.push(direct(false, 1));
    log
}

fn shuffled(mut log: Vec<Interaction>, seed: u64) -> Vec<Interaction> {
    let mut state = seed;
    for upper in (1..log.len()).rev() {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let other = (state % (upper as u64 + 1)) as usize;
        log.swap(upper, other);
    }
    log
}

#[test]
fn lilei_is_strong_through_the_public_t4d_api() {
    let score = T4D::score(PEER, &lilei(3), AS_OF);

    assert_eq!(score.algorithm_id, "T4D");
    assert_eq!(score.direct_count(), 12);
    assert_eq!(score.direct_active_day_count, 6);
    assert_eq!(score.band, Band::Strong);
}

#[test]
fn heavy_group_traffic_plus_two_directs_is_not_strong() {
    let score = T4D::score(PEER, &heavy_group_plus_two_directs(), AS_OF);

    assert_eq!(score.interaction_count, 102);
    assert_eq!(score.group_count(), 100);
    assert_eq!(score.direct_count(), 2);
    assert!(score.is_direct_reciprocal());
    assert_ne!(score.band, Band::Strong);
}

#[test]
fn group_only_traffic_is_not_strong() {
    let log: Vec<_> = (0..100)
        .map(|index| group(index % 2 == 0, index % 20))
        .collect();
    let score = T4D::score(PEER, &log, AS_OF);

    assert_eq!(score.direct_count(), 0);
    assert_eq!(score.group_count(), 100);
    assert!(score.is_group_only());
    assert_eq!(score.band, Band::Weak);
}

#[test]
fn recency_intervals_are_closed_at_180_and_360_days() {
    assert_eq!(T4D::score(PEER, &lilei(179), AS_OF).band, Band::Strong);
    assert_eq!(T4D::score(PEER, &lilei(180), AS_OF).band, Band::Moderate);
    assert_eq!(T4D::score(PEER, &lilei(359), AS_OF).band, Band::Moderate);
    assert_eq!(T4D::score(PEER, &lilei(360), AS_OF).band, Band::Weak);
}

#[test]
fn scoring_is_invariant_under_shuffle() {
    let log = heavy_group_plus_two_directs();
    let expected = T4D::score(PEER, &log, AS_OF);

    for seed in 0..64 {
        let permutation = shuffled(log.clone(), seed);
        assert_eq!(
            T4D::score(PEER, &permutation, AS_OF),
            expected,
            "seed {seed}"
        );
    }
}

#[test]
fn i64_min_timestamp_does_not_panic() {
    let log = vec![row(true, i64::MIN, true), row(false, i64::MIN + 1, true)];

    let result = std::panic::catch_unwind(|| T4D::score(PEER, &log, i64::MAX));
    let score = result.expect("T4D::score panicked on an i64::MIN timestamp");

    assert_eq!(score.first_contact_unix, i64::MIN);
    assert_eq!(score.silent_days, i64::MAX / SECONDS_PER_DAY);
    assert_eq!(score.band, Band::Weak);
}
