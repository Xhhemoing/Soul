//! The narrow boundary between stored graph observations and the pure T4D
//! rule.
//!
//! The numeric identifiers in [`soul_algo_tie::Interaction`] are scratch
//! values, not persisted identities. A fresh [`InteractionInterner`] is
//! therefore created for each rebuild and assigns dense `u64`s while retaining
//! each complete UUID and conversation reference as the lookup key.
//!
//! This module is also where the frozen crate's vocabulary stops. The
//! diagnostic-term audit keeps rating words out of the product surface, and
//! the frozen entry point is spelled in one of them, so the call and the type
//! are named here once — under this file's exemption — and the rest of the
//! graph works in [`TieReading`].

use std::collections::BTreeMap;

use soul_algo_tie::Interaction;
use uuid::Uuid;

use crate::interaction::{Direction, InteractionRef, Venue};

/// What the frozen rule made of one peer: the band, and every count behind it.
pub type TieReading = soul_algo_tie::TieScore;

/// Ask the frozen rule about one peer, as of one store-wide instant.
///
/// The free entry point rather than the rule by name, so rolling the decision
/// back stays a one-line change inside the frozen crate rather than an edit
/// here.
pub fn tie_reading(peer_id: u64, rows: &[Interaction], as_of_unix: i64) -> TieReading {
    soul_algo_tie::score(peer_id, rows, as_of_unix)
}

/// Failure to turn one stored observation into scorer input.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AdaptError {
    #[error("interaction timestamp is not valid RFC 3339: {timestamp}")]
    InvalidTimestamp { timestamp: String },
}

/// Rebuild-local dense identifiers for peers and conversations.
///
/// Peer UUIDs are keys in a map in their full 128-bit form. The assigned
/// `u64`s are sequential scratch values; they are never made by truncating or
/// hashing a UUID and must never be persisted.
#[derive(Debug, Default)]
pub struct InteractionInterner {
    peer_ids: BTreeMap<Uuid, u64>,
    conversation_ids: BTreeMap<String, u64>,
}

impl InteractionInterner {
    /// Convert one graph observation into the content-free scorer shape.
    pub fn adapt(&mut self, source: &InteractionRef) -> Result<Interaction, AdaptError> {
        Ok(Interaction {
            peer_id: intern(&mut self.peer_ids, source.peer_contact_id),
            outgoing: source.direction == Direction::Outgoing,
            occurred_at_unix: rfc3339_to_unix(source.occurred_at.as_str())?,
            venue_direct: source.venue == Venue::Direct,
            conversation_id: intern(
                &mut self.conversation_ids,
                source.conversation_ref.as_str().to_owned(),
            ),
        })
    }

    /// Number of distinct peers interned during this rebuild.
    pub fn peer_count(&self) -> usize {
        self.peer_ids.len()
    }

    /// Number of distinct conversations interned during this rebuild.
    pub fn conversation_count(&self) -> usize {
        self.conversation_ids.len()
    }
}

fn intern<K: Ord>(ids: &mut BTreeMap<K, u64>, key: K) -> u64 {
    let next = u64::try_from(ids.len()).expect("a map cannot contain more than u64::MAX entries");
    *ids.entry(key).or_insert(next)
}

fn rfc3339_to_unix(timestamp: &str) -> Result<i64, AdaptError> {
    parse_rfc3339(timestamp).ok_or_else(|| AdaptError::InvalidTimestamp {
        timestamp: timestamp.to_owned(),
    })
}

/// Parse the RFC 3339 subset accepted by the graph contract, including
/// fractional seconds and numeric UTC offsets. Fractions are discarded because
/// the scorer's clock is whole Unix seconds.
fn parse_rfc3339(timestamp: &str) -> Option<i64> {
    let bytes = timestamp.as_bytes();
    if bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || !matches!(bytes.get(10), Some(b'T' | b't'))
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return None;
    }

    let year = digits(bytes, 0, 4)?;
    let month = digits(bytes, 5, 2)?;
    let day = digits(bytes, 8, 2)?;
    let hour = digits(bytes, 11, 2)?;
    let minute = digits(bytes, 14, 2)?;
    let second = digits(bytes, 17, 2)?;

    if !(1..=12).contains(&month)
        || !(1..=days_in_month(year, month)).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }

    let mut zone_start = 19;
    if bytes.get(zone_start) == Some(&b'.') {
        zone_start += 1;
        let fraction_start = zone_start;
        while bytes.get(zone_start).is_some_and(u8::is_ascii_digit) {
            zone_start += 1;
        }
        if zone_start == fraction_start {
            return None;
        }
    }

    let offset_seconds = match bytes.get(zone_start) {
        Some(b'Z' | b'z') if zone_start + 1 == bytes.len() => 0,
        Some(sign @ (b'+' | b'-')) if zone_start + 6 == bytes.len() => {
            if bytes.get(zone_start + 3) != Some(&b':') {
                return None;
            }
            let offset_hour = digits(bytes, zone_start + 1, 2)?;
            let offset_minute = digits(bytes, zone_start + 4, 2)?;
            if offset_hour > 23 || offset_minute > 59 {
                return None;
            }
            let magnitude = i64::from(offset_hour * 3_600 + offset_minute * 60);
            if *sign == b'+' {
                magnitude
            } else {
                -magnitude
            }
        }
        _ => return None,
    };

    let days = days_from_civil(i64::from(year), month, day);
    Some(days * 86_400 + i64::from(hour * 3_600 + minute * 60 + second) - offset_seconds)
}

fn digits(bytes: &[u8], start: usize, len: usize) -> Option<u32> {
    bytes
        .get(start..start + len)?
        .iter()
        .try_fold(0, |value, byte| {
            byte.is_ascii_digit()
                .then(|| value * 10 + u32::from(byte - b'0'))
        })
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Howard Hinnant's `days_from_civil`, shifted to the Unix epoch.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let shifted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use soul_algo_tie::as_of_max;
    use soul_schema::common::{Sha256Hex, Timestamp};

    use super::*;
    use crate::interaction::RefKind;

    fn observation(
        peer_contact_id: Uuid,
        conversation: char,
        occurred_at: &str,
        direction: Direction,
        venue: Venue,
    ) -> InteractionRef {
        InteractionRef {
            ref_kind: RefKind::GraphInteraction,
            event_id: Uuid::nil(),
            self_contact_id: Uuid::nil(),
            peer_contact_id,
            conversation_ref: Sha256Hex::new(conversation.to_string().repeat(64)),
            direction,
            occurred_at: Timestamp::new(occurred_at),
            venue,
        }
    }

    #[test]
    fn interned_ids_are_dense_unique_and_use_the_whole_uuid() {
        let peers = [
            Uuid::from_u128(0xaaaaaaaaaaaaaaaa_bbbbbbbbbbbbbbbb),
            Uuid::from_u128(0xaaaaaaaaaaaaaaaa_cccccccccccccccc),
            Uuid::from_u128(0xdddddddddddddddd_bbbbbbbbbbbbbbbb),
        ];
        let sources = [
            observation(
                peers[0],
                'a',
                "2026-08-24T00:00:00Z",
                Direction::Outgoing,
                Venue::Direct,
            ),
            observation(
                peers[1],
                'a',
                "2026-08-24T00:00:01Z",
                Direction::Incoming,
                Venue::Group,
            ),
            observation(
                peers[0],
                'b',
                "2026-08-24T00:00:02Z",
                Direction::Incoming,
                Venue::Direct,
            ),
            observation(
                peers[2],
                'b',
                "2026-08-24T00:00:03Z",
                Direction::Outgoing,
                Venue::Group,
            ),
            observation(
                peers[0],
                'a',
                "2026-08-24T00:00:04Z",
                Direction::Outgoing,
                Venue::Direct,
            ),
        ];

        let mut interner = InteractionInterner::default();
        let adapted: Vec<_> = sources
            .iter()
            .map(|source| interner.adapt(source).unwrap())
            .collect();

        assert_eq!(
            adapted.iter().map(|row| row.peer_id).collect::<Vec<_>>(),
            [0, 1, 0, 2, 0]
        );
        assert_eq!(
            adapted
                .iter()
                .map(|row| row.conversation_id)
                .collect::<Vec<_>>(),
            [0, 0, 1, 1, 0]
        );
        assert_eq!(interner.peer_count(), 3);
        assert_eq!(interner.conversation_count(), 2);
    }

    #[test]
    fn adapted_vector_has_field_fidelity_and_one_store_wide_as_of_max() {
        let peer = Uuid::from_u128(1);
        let sources = [
            observation(
                peer,
                'a',
                "2026-08-24T11:00:00+08:00",
                Direction::Outgoing,
                Venue::Direct,
            ),
            observation(
                peer,
                'b',
                "2026-08-24T02:00:00.999Z",
                Direction::Incoming,
                Venue::Group,
            ),
        ];
        let mut interner = InteractionInterner::default();
        let adapted: Vec<_> = sources
            .iter()
            .map(|source| interner.adapt(source).unwrap())
            .collect();

        assert!(adapted[0].outgoing);
        assert!(adapted[0].venue_direct);
        assert!(!adapted[1].outgoing);
        assert!(!adapted[1].venue_direct);
        assert_eq!(adapted[0].occurred_at_unix, 1_787_540_400);
        assert_eq!(adapted[1].occurred_at_unix, 1_787_536_800);
        assert_eq!(as_of_max(&adapted), Some(1_787_540_400));
    }
}
