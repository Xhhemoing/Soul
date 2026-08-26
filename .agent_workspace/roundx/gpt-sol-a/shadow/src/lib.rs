#![forbid(unsafe_code)]

//! Independent, dependency-free implementation of the frozen T4D rule.
//! This crate intentionally shares no code or types with `soul-algo-tie`.

use std::collections::BTreeSet;

pub const SECONDS_PER_DAY: i64 = 86_400;
pub const MODERATE_MIN_INTERACTIONS: u64 = 3;
pub const STRONG_MIN_INTERACTIONS: u64 = 10;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;
pub const DEMOTE_ONE_BAND_DAYS: i64 = 180;
pub const FORCE_WEAK_DAYS: i64 = 360;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    Weak,
    Moderate,
    Strong,
}

impl Band {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Weak => "Weak",
            Self::Moderate => "Moderate",
            Self::Strong => "Strong",
        }
    }

    const fn demote_once(self) -> Self {
        match self {
            Self::Strong => Self::Moderate,
            Self::Moderate | Self::Weak => Self::Weak,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub outgoing: bool,
    pub occurred_at_unix: i64,
    pub direct: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub band: Band,
    pub band_before_recency: Band,
    pub direct_count: u64,
    pub group_count: u64,
    pub direct_active_days: u64,
    pub silent_days: i64,
}

/// Apply T4D to one peer's observations at a caller-supplied store-wide time.
pub fn score(events: &[Event], as_of_unix: i64) -> Outcome {
    let mut direct_out = 0u64;
    let mut direct_in = 0u64;
    let mut group_count = 0u64;
    let mut direct_days = BTreeSet::new();
    let mut last_contact: Option<i64> = None;

    for event in events {
        last_contact = Some(match last_contact {
            Some(previous) => previous.max(event.occurred_at_unix),
            None => event.occurred_at_unix,
        });

        if event.direct {
            if event.outgoing {
                direct_out += 1;
            } else {
                direct_in += 1;
            }
            direct_days.insert(event.occurred_at_unix.div_euclid(SECONDS_PER_DAY));
        } else {
            group_count += 1;
        }
    }

    let direct_count = direct_out + direct_in;
    let direct_active_days = direct_days.len() as u64;
    let reciprocal = direct_out > 0 && direct_in > 0;
    let band_before_recency = if reciprocal
        && direct_count >= STRONG_MIN_INTERACTIONS
        && direct_active_days >= STRONG_MIN_ACTIVE_DAYS
    {
        Band::Strong
    } else if reciprocal && direct_count >= MODERATE_MIN_INTERACTIONS {
        Band::Moderate
    } else {
        Band::Weak
    };

    // Empty observations are Weak without entering timestamp arithmetic.
    let silent_days = last_contact.map_or(0, |last| {
        as_of_unix
            .saturating_sub(last)
            .max(0)
            .div_euclid(SECONDS_PER_DAY)
    });
    let band = if silent_days >= FORCE_WEAK_DAYS {
        Band::Weak
    } else if silent_days >= DEMOTE_ONE_BAND_DAYS {
        band_before_recency.demote_once()
    } else {
        band_before_recency
    };

    Outcome {
        band,
        band_before_recency,
        direct_count,
        group_count,
        direct_active_days,
        silent_days,
    }
}

pub mod fixtures {
    use super::{Band, Event, SECONDS_PER_DAY};

    pub const AS_OF: i64 = 1_787_580_000; // 2026-08-24T14:00:00Z
    const MIDNIGHT: i64 = AS_OF - 14 * 3_600;

    #[derive(Clone, Debug)]
    pub struct Fixture {
        pub id: &'static str,
        pub description: &'static str,
        pub expected: Band,
        pub events: Vec<Event>,
        pub as_of: i64,
    }

    fn at(days_ago: i64, hour: i64, minute: i64) -> i64 {
        MIDNIGHT - days_ago * SECONDS_PER_DAY + hour * 3_600 + minute * 60
    }

    fn run(count: usize, active_days: usize, newest_days_ago: i64, direct: bool) -> Vec<Event> {
        (0..count)
            .map(|index| {
                let day = index * active_days / count;
                let days_ago = newest_days_ago + (active_days - 1 - day) as i64;
                Event {
                    outgoing: index % 2 == 0,
                    occurred_at_unix: at(days_ago, 9, (index % 30) as i64),
                    direct,
                }
            })
            .collect()
    }

    fn fixture(
        id: &'static str,
        description: &'static str,
        expected: Band,
        events: Vec<Event>,
    ) -> Fixture {
        Fixture {
            id,
            description,
            expected,
            events,
            as_of: AS_OF,
        }
    }

    fn strong_history(id: &'static str, silent_days: i64, expected: Band) -> Fixture {
        fixture(
            id,
            "12 reciprocal direct events / 6 UTC days",
            expected,
            run(12, 6, silent_days, true),
        )
    }

    pub fn all() -> Vec<Fixture> {
        let mut heavy = run(100, 50, 0, false);
        heavy.push(Event {
            outgoing: true,
            occurred_at_unix: at(0, 11, 0),
            direct: true,
        });
        heavy.push(Event {
            outgoing: false,
            occurred_at_unix: at(0, 12, 0),
            direct: true,
        });

        vec![
            strong_history("F_LILEI", 3, Band::Strong),
            fixture(
                "F_HEAVY",
                "100 reciprocal group events / 50 days + one direct each way",
                Band::Weak,
                heavy,
            ),
            fixture(
                "F_GROUP",
                "50 reciprocal group-only events / 10 days",
                Band::Weak,
                run(50, 10, 0, false),
            ),
            strong_history("F_OLD", 2_800, Band::Weak),
            fixture(
                "F_BURST",
                "20 reciprocal direct events / 1 UTC day",
                Band::Moderate,
                run(20, 1, 0, true),
            ),
            strong_history("179", 179, Band::Strong),
            strong_history("180", 180, Band::Moderate),
            strong_history("359", 359, Band::Moderate),
            strong_history("360", 360, Band::Weak),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::{fixtures, score, Band, Event};

    #[test]
    fn all_requested_fixtures_match_the_decision() {
        for fixture in fixtures::all() {
            let actual = score(&fixture.events, fixture.as_of);
            assert_eq!(actual.band, fixture.expected, "{}", fixture.id);
        }
    }

    #[test]
    fn group_activity_is_not_a_band_input_but_is_the_recency_clock() {
        let as_of = fixtures::AS_OF;
        let mut events = fixtures::all()
            .into_iter()
            .find(|fixture| fixture.id == "360")
            .unwrap()
            .events;
        assert_eq!(score(&events, as_of).band, Band::Weak);
        events.push(Event {
            outgoing: true,
            occurred_at_unix: as_of - 60,
            direct: false,
        });
        let result = score(&events, as_of);
        assert_eq!(result.band, Band::Strong);
        assert_eq!(result.group_count, 1);
        assert_eq!(result.silent_days, 0);
    }

    #[test]
    fn empty_input_is_weak_without_timestamp_arithmetic() {
        let result = score(&[], i64::MIN);
        assert_eq!(result.band, Band::Weak);
        assert_eq!(result.silent_days, 0);
    }
}
