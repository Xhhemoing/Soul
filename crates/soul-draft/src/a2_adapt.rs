//! The narrow boundary between a stored [`TieEdge`] and the frozen A2
//! renderer.
//!
//! D40: the people summary says what `soul_algo_trait::a2_render` says. This
//! crate holds no second wording for a band, no second dormancy threshold and
//! no second opinion about what the counts mean — it hands over the numbers the
//! graph already persisted and turns the bullets that come back into something
//! [`crate::analysis`] can attach evidence to.
//!
//! Two rules are implemented here rather than left to the caller:
//!
//! * **The venue split is read, never re-derived.** [`venue_split`] reads the
//!   persisted per-venue tallies and hands them over only when the edge was
//!   written by a rebuild that produced them (`as_of_utc.is_some()`). A real
//!   zero is a zero; a legacy row that never had the fields says nothing at all
//!   rather than claiming zero. Walking the evidence to count venues here would
//!   make this crate the author of a number the band was not decided on
//!   (COPY_ZH §7, DECISIONS D33).
//! * **No clock.** Both instants are read off the edge. `as_of_utc` is the one
//!   store-wide instant the rebuild scored against; a row written before that
//!   field existed falls back to its own last contact, which makes the recency
//!   sentence read "today" rather than inventing a gap.
//!
//! This is also where the frozen crate's vocabulary stops, exactly as
//! `soul-graph`'s tie adapter does for the tie rule: the renderer's input type
//! is spelled in one of the words the diagnostic-term audit denies, so it is
//! named here once — under this file's exemption — and the rest of the crate
//! works in [`RenderedBullet`].

use soul_algo_trait::a2::{a2_render, TieScore};
use soul_algo_trait::types::Band;
use soul_graph::model::{TieEdge, TieStrength, TieType};
use soul_schema::common::SupportedBand;
use uuid::Uuid;

/// The statement A2 uses to say which band the graph filed an edge under.
///
/// Named because GC-9a takes exactly this bullet out on an edge the user has
/// corrected; `the_filing_key_is_one_a2_emits` keeps the constant honest.
pub const FILED_BAND_KEY: &str = "personnel.tie.filed_band";

/// One rendered sentence, with the evidence rows it rests on back in the
/// identifiers this crate uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedBullet {
    /// The renderer's stable key for this kind of statement.
    pub statement_key: String,
    /// The sentence, as the frozen renderer wrote it.
    pub text_zh: String,
    /// Never empty: A2 emits no bullet without evidence behind it.
    pub evidence_ids: Vec<Uuid>,
}

/// Render one edge.
///
/// `counted` is the evidence the summary is allowed to cite, in the order it
/// will be cited; `last_contact` is the row holding the most recent exchange,
/// when the caller could identify one. Both are UUIDs, and the renderer works
/// in dense `u64`s, so they are interned by position for this call and mapped
/// back on the way out. The interned values are scratch and are never stored.
pub fn bullets_for(
    edge: &TieEdge,
    counted: &[Uuid],
    last_contact: Option<Uuid>,
) -> Vec<RenderedBullet> {
    let interned: Vec<u64> = (0..counted.len()).map(|index| index as u64).collect();
    let last_contact_evidence_id = last_contact.and_then(|row| {
        counted
            .iter()
            .position(|cited| *cited == row)
            .map(|index| index as u64)
    });

    let strength = &edge.tie_strength;
    let last_contact_unix = parse_rfc3339(strength.last_contact_utc.as_str());
    let as_of_unix = as_of_unix(strength)
        .or(last_contact_unix)
        .unwrap_or_default();
    let (direct_count, group_count) = match venue_split(strength) {
        Some((direct, group)) => (Some(direct), Some(group)),
        None => (None, None),
    };

    let reading = TieScore {
        band: band_of(strength.band),
        interaction_count: count(strength.interaction_count),
        outgoing: count(strength.outgoing_count),
        incoming: count(strength.incoming_count),
        active_day_count: count(strength.active_day_count),
        conversation_count: count(strength.conversation_count),
        any_direct: any_direct(edge),
        direct_count,
        group_count,
        last_contact_unix,
        as_of_unix,
        evidence_ids: interned,
        last_contact_evidence_id,
    };

    a2_render(&reading)
        .bullets
        .into_iter()
        .map(|bullet| RenderedBullet {
            statement_key: bullet.statement_key,
            text_zh: bullet.text_zh,
            evidence_ids: bullet
                .evidence_ids
                .iter()
                .filter_map(|id| counted.get(*id as usize).copied())
                .collect(),
        })
        .collect()
}

/// Whether the user has overruled the band on this edge.
///
/// The edge's own answer, not a second definition of one: `locked_by_user` and
/// `user_band` are written together by `soul_graph::correct_tie` and cleared
/// together by `release_tie`, and [`TieStrength::is_locked_by_user`] is the one
/// place that says what the pair means.
pub fn is_locked_by_user(strength: &TieStrength) -> bool {
    strength.is_locked_by_user()
}

/// The venue split the rebuild persisted, when it persisted one.
///
/// Both halves or neither: half a split invites the reader to subtract, and
/// subtracting would make this crate the author of a number nobody gave it.
///
/// Gated on `as_of_utc`, because the rebuild that scored the edge wrote the
/// per-venue tallies in the same object: `as_of` present means the split is a
/// measurement, and four zeroes really are four zeroes. On a row written before
/// the frozen rule landed the fields are serde defaults, and A2's own rule is
/// that a split it was not handed is a sentence it does not say. Counting
/// venues out of the evidence here instead would be the second opinion D33
/// rules out.
fn venue_split(strength: &TieStrength) -> Option<(u32, u32)> {
    strength.as_of_utc.as_ref()?;
    Some((
        count(strength.direct_out_count + strength.direct_in_count),
        count(strength.group_out_count + strength.group_in_count),
    ))
}

/// The one store-wide instant the rebuild scored this edge against.
///
/// A row without one falls back to its own last contact in [`bullets_for`],
/// which makes the recency sentence read "today" — the honest reading of a row
/// that never recorded what it was scored against, and the reason nothing here
/// reaches for a wall clock instead.
fn as_of_unix(strength: &TieStrength) -> Option<i64> {
    parse_rfc3339(strength.as_of_utc.as_ref()?.as_str())
}

/// Whether anything on this edge has ever been one to one.
///
/// The observed shape, because that is the field the rebuild sets from the
/// rule's own reading of the venues.
fn any_direct(edge: &TieEdge) -> bool {
    edge.types.contains(&TieType::Direct)
}

fn band_of(band: SupportedBand) -> Band {
    match band {
        SupportedBand::Weak => Band::Weak,
        SupportedBand::Moderate => Band::Moderate,
        SupportedBand::Strong => Band::Strong,
    }
}

/// A tally, in the width the renderer takes. Saturating rather than wrapping:
/// four billion messages with one person is not a number this product will see,
/// and if it ever does, an overstated count is a worse answer than a capped
/// one.
fn count(tally: u64) -> u32 {
    u32::try_from(tally).unwrap_or(u32::MAX)
}

/// The RFC 3339 subset the graph contract writes, as whole Unix seconds.
///
/// `None` rather than a sentinel for anything else: an unparseable instant
/// means the recency sentence does not appear, which is the honest outcome for
/// a row whose time nobody can read.
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
            match sign {
                b'+' => magnitude,
                _ => -magnitude,
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
    use super::*;

    #[test]
    fn the_filing_key_is_one_a2_emits() {
        assert!(soul_algo_trait::a2::A2_STATEMENT_KEYS.contains(&FILED_BAND_KEY));
    }

    #[test]
    fn instants_the_graph_writes_parse_to_whole_seconds() {
        assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_rfc3339("2026-08-24T00:00:00Z"), Some(1_787_529_600));
        // 11:00 in a +08:00 zone is 03:00 UTC, not 11:00 with a note attached.
        assert_eq!(
            parse_rfc3339("2026-08-24T11:00:00+08:00"),
            Some(1_787_540_400),
        );
        assert_eq!(
            parse_rfc3339("2026-08-24T02:00:00.999Z"),
            Some(1_787_536_800),
        );
        assert_eq!(parse_rfc3339("2000-02-29T00:00:00Z"), Some(951_782_400));
    }

    #[test]
    fn an_instant_nobody_can_read_is_none_rather_than_a_guess() {
        for unreadable in [
            "",
            "2026-03-09",
            "2026-02-31T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-03-09T24:00:00Z",
            "2026-03-09T00:00:00",
            "2026-03-09T00:00:00+0800",
        ] {
            assert_eq!(parse_rfc3339(unreadable), None, "{unreadable}");
        }
    }
}
