//! Deterministic fixtures, shared by the unit tests and the comparison suite.
//!
//! Every fixture returns its own `now`, so no test reads the wall clock and no
//! test goes red because a year passed. Timestamps are built from
//! [`DAY`]-sized offsets off a fixed epoch anchor rather than from a date
//! library, which keeps the crate dependency-free and keeps the arithmetic
//! visible.
//!
//! The module is public rather than `#[cfg(test)]` so the other Round 1 slots
//! can run their benchmarks and adversarial probes against exactly the same
//! evidence.

use crate::types::{Interaction, SECONDS_PER_DAY};

/// One UTC day in seconds, spelled shortly because fixtures are full of it.
pub const DAY: i64 = SECONDS_PER_DAY;

/// 2026-01-01T00:00:00Z. The anchor most fixtures count from.
pub const T_2026: i64 = 1_767_225_600;
/// 2019-06-01T00:00:00Z, for the dormant-tie fixture.
pub const T_2019: i64 = 1_559_347_200;
/// 2026-08-24T00:00:00Z, "now" for the fixtures that need a present.
pub const NOW_2026_08_24: i64 = 1_787_529_600;

/// A one-to-one exchange.
pub fn direct(
    peer_id: u64,
    outgoing: bool,
    occurred_at_unix: i64,
    conversation_id: u64,
) -> Interaction {
    Interaction {
        peer_id,
        outgoing,
        occurred_at_unix,
        venue_direct: true,
        conversation_id,
    }
}

/// An exchange with other people in the room.
pub fn group(
    peer_id: u64,
    outgoing: bool,
    occurred_at_unix: i64,
    conversation_id: u64,
) -> Interaction {
    Interaction {
        peer_id,
        outgoing,
        occurred_at_unix,
        venue_direct: false,
        conversation_id,
    }
}

/// Nothing has been imported. Returns `(log, now)`.
pub fn empty() -> (Vec<Interaction>, i64) {
    (Vec::new(), NOW_2026_08_24)
}

/// One message, from them, never answered.
pub fn single_inbound() -> (Vec<Interaction>, i64) {
    (
        vec![direct(1, false, NOW_2026_08_24 - DAY, 1)],
        NOW_2026_08_24,
    )
}

/// The Goal 1 acceptance fixture: 12 reciprocal one-to-one messages spread
/// over 6 days, ending three days before `now`.
///
/// This is the tie the shipped rule must call Strong. If a candidate disagrees
/// with T0 here it is not a refinement of the baseline, it is a replacement,
/// and it owes an argument.
pub fn lilei_12_over_6_days() -> (Vec<Interaction>, i64) {
    let start = NOW_2026_08_24 - DAY * 9;
    let mut log = Vec::new();
    for day in 0..6i64 {
        // Two exchanges a day, one each way, at 09:00 and 21:00 UTC.
        log.push(direct(1, true, start + DAY * day + 9 * 3_600, 1));
        log.push(direct(1, false, start + DAY * day + 21 * 3_600, 1));
    }
    (log, NOW_2026_08_24)
}

/// Twenty reciprocal messages inside one afternoon.
///
/// One conversation, not a habit. The span gate exists for this case, so any
/// candidate that calls it Strong has lost the property Goal 1 wrote a comment
/// about.
pub fn twenty_in_one_afternoon() -> (Vec<Interaction>, i64) {
    let start = NOW_2026_08_24 - DAY + 13 * 3_600;
    let log = (0..20i64)
        .map(|i| direct(2, i % 2 == 0, start + i * 120, 2))
        .collect();
    (log, NOW_2026_08_24)
}

/// Fifty reciprocal messages, all of them in the same group chat, spread over
/// fifty days.
///
/// A colleague in a project channel. Frequent, recent, multi-day, and by
/// Granovetter's definition still a weak tie.
pub fn group_only_50() -> (Vec<Interaction>, i64) {
    let start = NOW_2026_08_24 - DAY * 50;
    let log = (0..50i64)
        .map(|i| group(3, i % 2 == 0, start + DAY * i + 10 * 3_600, 3))
        .collect();
    (log, NOW_2026_08_24)
}

/// A close friendship that stopped in 2019, scored in 2026.
///
/// Twenty reciprocal one-to-one messages over ten days, seven years ago.
pub fn dormant_since_2019() -> (Vec<Interaction>, i64) {
    let log = (0..20i64)
        .map(|i| direct(4, i % 2 == 0, T_2019 + DAY * (i / 2) + (i % 2) * 3_600, 4))
        .collect();
    (log, NOW_2026_08_24)
}

/// A hundred messages sent, none ever answered.
///
/// The shape of a broadcast channel, a bot, or someone who is not talking to
/// the user. Nothing in this family may call it anything but Weak.
pub fn one_sided_100_outbound() -> (Vec<Interaction>, i64) {
    let start = NOW_2026_08_24 - DAY * 100;
    let log = (0..100i64)
        .map(|i| direct(5, true, start + DAY * i + 8 * 3_600, 5))
        .collect();
    (log, NOW_2026_08_24)
}

/// Every fixture above, with the peer id each one is about.
pub fn all() -> Vec<(&'static str, u64, Vec<Interaction>, i64)> {
    let (empty_log, now) = empty();
    let (inbound, _) = single_inbound();
    let (lilei, _) = lilei_12_over_6_days();
    let (afternoon, _) = twenty_in_one_afternoon();
    let (group_chat, _) = group_only_50();
    let (dormant, _) = dormant_since_2019();
    let (one_sided, _) = one_sided_100_outbound();
    vec![
        ("empty", 1, empty_log, now),
        ("single_inbound", 1, inbound, now),
        ("lilei_12_over_6_days", 1, lilei, now),
        ("twenty_in_one_afternoon", 2, afternoon, now),
        ("group_only_50", 3, group_chat, now),
        ("dormant_since_2019", 4, dormant, now),
        ("one_sided_100_outbound", 5, one_sided, now),
    ]
}
