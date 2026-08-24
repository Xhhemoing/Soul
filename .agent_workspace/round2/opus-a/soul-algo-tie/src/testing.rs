//! Deterministic fixtures, shared by the unit tests, the ablation suite and
//! the other Round 2 slots.
//!
//! Every fixture carries its own `as_of`, so no test reads the wall clock and
//! no test goes red because a year passed. Timestamps are built from whole-day
//! offsets off a fixed anchor rather than from a date library, which keeps the
//! crate dependency-free and keeps the arithmetic visible.
//!
//! ## The `as_of` used here
//!
//! [`AS_OF_2026_08_24`] is 2026-08-24T14:00:00Z, the instant this round's
//! brief names. It is one of the two legal choices — the other being
//! `max(occurred_at)` over the whole store, which `tests/as_of_discipline.rs`
//! also exercises through [`store`]. Nothing in the crate ever reads a clock.
//!
//! All fixture events are placed with [`at`], which puts them at a whole hour
//! of a whole UTC day counted back from the anchor's midnight. For any hour up
//! to 14:00 this makes `age_days` exactly equal to the fixture's `days_ago`
//! argument, so the expected buckets can be read straight off the fixture.

use crate::types::{Interaction, SECONDS_PER_DAY};

/// One UTC day in seconds, spelled shortly because fixtures are full of it.
pub const DAY: i64 = SECONDS_PER_DAY;

/// 2026-08-24T14:00:00Z. The `as_of` this round's brief names.
///
/// Derived, not looked up: 2026-01-01T00:00:00Z is 1 767 225 600, the 235 days
/// from 1 January to 24 August 2026 (a common year) add 20 304 000 seconds,
/// and 14 hours add 50 400. Pinned in `tests/as_of_discipline.rs`.
pub const AS_OF_2026_08_24: i64 = 1_787_580_000;

/// 2026-08-24T00:00:00Z, midnight of the anchor day.
pub const TODAY_MIDNIGHT: i64 = 1_787_529_600;

/// 2019-06-01T00:00:00Z, for the dormant-tie fixture.
pub const T_2019: i64 = 1_559_347_200;

/// `hour` o'clock UTC, `days_ago` whole days before the anchor's midnight.
///
/// For `hour <= 14` the resulting instant is exactly `days_ago` whole days old
/// as of [`AS_OF_2026_08_24`].
pub const fn at(days_ago: i64, hour: i64) -> i64 {
    TODAY_MIDNIGHT - days_ago * DAY + hour * 3_600
}

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

/// One named case: the evidence, who it is about, and when it is being judged.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub name: &'static str,
    /// What the tie is, in the words the ablation table uses.
    pub summary: &'static str,
    pub peer_id: u64,
    pub log: Vec<Interaction>,
    pub as_of: i64,
}

impl Fixture {
    fn new(
        name: &'static str,
        summary: &'static str,
        peer_id: u64,
        log: Vec<Interaction>,
    ) -> Fixture {
        Fixture {
            name,
            summary,
            peer_id,
            log,
            as_of: AS_OF_2026_08_24,
        }
    }
}

/// `count` exchanges a day, alternating direction, on `days` consecutive days
/// ending `newest_days_ago` days before the anchor.
fn run(
    peer_id: u64,
    venue_direct: bool,
    newest_days_ago: i64,
    days: i64,
    per_day: i64,
    conversation_id: u64,
) -> Vec<Interaction> {
    let mut log = Vec::new();
    for day in 0..days {
        let days_ago = newest_days_ago + (days - 1 - day);
        for slot in 0..per_day {
            let outgoing = (day * per_day + slot) % 2 == 0;
            let at_unix = at(days_ago, 9 + 2 * slot);
            log.push(Interaction {
                peer_id,
                outgoing,
                occurred_at_unix: at_unix,
                venue_direct,
                conversation_id,
            });
        }
    }
    log
}

/// Nothing has been imported about this person.
pub fn empty() -> Fixture {
    Fixture::new("empty", "没有任何往来记录", 99, Vec::new())
}

/// One message, from them, never answered.
pub fn single_inbound() -> Fixture {
    Fixture::new(
        "single_inbound",
        "对方发来 1 条，从未回应",
        11,
        vec![direct(11, false, at(1, 9), 11)],
    )
}

/// The Goal 1 acceptance fixture: 12 reciprocal one-to-one messages spread
/// over 6 days, ending three days before `as_of`.
///
/// This is the tie the shipped rule must call Strong. Any candidate that
/// disagrees here is not a refinement of the baseline, it is a replacement,
/// and it owes an argument.
pub fn lilei_12() -> Fixture {
    Fixture::new(
        "lilei_12",
        "私聊互惠 12 次 / 6 天 / 3 天前",
        1,
        run(1, true, 3, 6, 2, 1),
    )
}

/// Twenty reciprocal messages inside one afternoon.
///
/// One conversation, not a habit. The span gate exists for this case.
pub fn afternoon_20() -> Fixture {
    let log = (0..20i64)
        .map(|i| direct(2, i % 2 == 0, at(1, 13) + i * 120, 2))
        .collect();
    Fixture::new("afternoon_20", "私聊互惠 20 次挤在一个下午", 2, log)
}

/// Fifty reciprocal messages, all in the same group chat, over fifty days.
///
/// A colleague in a project channel: frequent, recent, multi-day, and by
/// Granovetter's definition still a weak tie. T0 calls it Strong.
pub fn group_only_50() -> Fixture {
    let log = (0..50i64)
        .map(|i| group(3, i % 2 == 0, at(50 - i, 10), 3))
        .collect();
    Fixture::new("group_only_50", "群聊互惠 50 次 / 50 天 / 从未私聊", 3, log)
}

/// A close friendship that stopped in 2019, judged in 2026.
///
/// Twenty reciprocal one-to-one messages over ten days, seven years ago. This
/// is F04, the headline ablation case.
pub fn dormant_2019() -> Fixture {
    let log = (0..20i64)
        .map(|i| direct(4, i % 2 == 0, T_2019 + DAY * (i / 2) + (i % 2) * 3_600, 4))
        .collect();
    Fixture::new(
        "dormant_2019",
        "私聊互惠 20 次 / 10 天，全部发生在 2019 年",
        4,
        log,
    )
}

/// A hundred messages sent, none ever answered.
pub fn one_sided_100() -> Fixture {
    let log = (0..100i64)
        .map(|i| direct(5, true, at(100 - i, 8), 5))
        .collect();
    Fixture::new("one_sided_100", "单向发出 100 次，对方从未回应", 5, log)
}

/// Strong by every count, last heard from 200 days ago.
///
/// The second net-win case: T3 has no way to notice, and the demotion rule
/// does.
pub fn quiet_200_days() -> Fixture {
    Fixture::new(
        "quiet_200_days",
        "私聊互惠 12 次 / 6 天，最近一次 200 天前",
        6,
        run(6, true, 200, 6, 2, 6),
    )
}

/// A long private history that went quiet, then restarted four days ago.
///
/// Thirty exchanges around 200 days old plus two this week. The decayed count
/// lands at 9.5 effective interactions — half a message under the bar — while
/// the demotion rule sees somebody the user spoke to this week.
pub fn revived_after_gap() -> Fixture {
    let mut log = run(7, true, 191, 10, 3, 7);
    log.push(direct(7, true, at(5, 9), 7));
    log.push(direct(7, false, at(4, 13), 7));
    Fixture::new(
        "revived_after_gap",
        "私聊互惠 30 次 / 200 天前，另加最近 2 次",
        7,
        log,
    )
}

/// A project-channel colleague from last spring: group-only and quiet.
pub fn group_only_quiet_200() -> Fixture {
    Fixture::new(
        "group_only_quiet_200",
        "群聊互惠 20 次 / 10 天，最近一次 200 天前",
        8,
        run(8, false, 200, 10, 2, 8),
    )
}

/// Two exchanges a week for eight weeks. The steady friendship (F05).
pub fn steady_16_over_8_weeks() -> Fixture {
    let mut log = Vec::new();
    for week in 0..8i64 {
        let days_ago = 56 - week * 7;
        log.push(direct(9, true, at(days_ago, 9), 9));
        log.push(direct(9, false, at(days_ago, 13), 9));
    }
    Fixture::new(
        "steady_16_over_8_weeks",
        "私聊互惠 16 次 / 8 周 / 每周 2 次",
        9,
        log,
    )
}

/// A thousand messages in one day: a flood, or an import that lost its dates.
pub fn flood_1000_in_one_day() -> Fixture {
    let log = (0..1_000i64)
        .map(|i| direct(10, i % 2 == 0, at(1, 0) + i * 50, 10))
        .collect();
    Fixture::new("flood_1000_in_one_day", "私聊互惠 1000 次挤在一天", 10, log)
}

/// Strong by count, silent for exactly the demotion threshold.
pub fn quiet_180_days() -> Fixture {
    Fixture::new(
        "quiet_180_days",
        "私聊互惠 12 次 / 6 天，最近一次正好 180 天前",
        12,
        run(12, true, 180, 6, 2, 12),
    )
}

/// Strong by count, silent for one day less than the demotion threshold.
pub fn quiet_179_days() -> Fixture {
    Fixture::new(
        "quiet_179_days",
        "私聊互惠 12 次 / 6 天，最近一次 179 天前",
        13,
        run(13, true, 179, 6, 2, 13),
    )
}

/// Strong by count, silent for exactly the cut-off.
pub fn dormant_360_days() -> Fixture {
    Fixture::new(
        "dormant_360_days",
        "私聊互惠 12 次 / 6 天，最近一次正好 360 天前",
        14,
        run(14, true, 360, 6, 2, 14),
    )
}

/// Strong by count, silent for one day less than the cut-off.
pub fn dormant_359_days() -> Fixture {
    Fixture::new(
        "dormant_359_days",
        "私聊互惠 12 次 / 6 天，最近一次 359 天前",
        15,
        run(15, true, 359, 6, 2, 15),
    )
}

/// Every fixture, in table order.
pub fn all() -> Vec<Fixture> {
    vec![
        empty(),
        single_inbound(),
        lilei_12(),
        afternoon_20(),
        group_only_50(),
        one_sided_100(),
        flood_1000_in_one_day(),
        steady_16_over_8_weeks(),
        quiet_179_days(),
        quiet_180_days(),
        quiet_200_days(),
        revived_after_gap(),
        group_only_quiet_200(),
        dormant_359_days(),
        dormant_360_days(),
        dormant_2019(),
    ]
}

/// Every fixture's evidence in one log, the way a real store holds it.
///
/// Peer ids are distinct across fixtures on purpose, so the concatenation is
/// still one row per observation and `as_of = max(occurred_at)` over the whole
/// store is well defined.
pub fn store() -> Vec<Interaction> {
    all().into_iter().flat_map(|fixture| fixture.log).collect()
}
