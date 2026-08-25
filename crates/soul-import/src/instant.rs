//! Recognising the two timestamp shapes the v0.1 importers meet, and putting
//! the zoned one into UTC.
//!
//! There is no date library here: `soul-import-v1` promises RFC 3339 and
//! Telegram writes a zoneless civil timestamp beside a Unix second, so what is
//! needed is enough checking to tell a real instant from a field somebody
//! filled in by hand, plus the one conversion in [`to_utc`]. Telegram never
//! reaches that conversion — its instant is built from `date_unixtime` and is
//! already in UTC.

/// `YYYY-MM-DDTHH:MM:SS`, with the components in range and no zone.
///
/// This is what Telegram's `date` looks like. It is not enough to build an
/// instant from — that is what `date_unixtime` is for — but a `date` that is
/// not even this shape says the file is damaged. A day the month does not
/// have is damage too: `2026-02-31` is a field somebody generated, not a
/// moment anything happened at.
pub fn is_civil_datetime(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 19 {
        return false;
    }
    for (index, expected) in b"nnnn-nn-nnTnn:nn:nn".iter().enumerate() {
        let ok = match expected {
            b'n' => bytes[index].is_ascii_digit(),
            other => bytes[index] == *other,
        };
        if !ok {
            return false;
        }
    }
    let number = |from: usize, to: usize| text[from..to].parse::<u32>().unwrap_or(u32::MAX);
    let (year, month, day) = (number(0, 4), number(5, 7), number(8, 10));
    let (hour, minute, second) = (number(11, 13), number(14, 16), number(17, 19));
    (1..=days_in_month(year, month)).contains(&day) && hour < 24 && minute < 60 && second < 60
}

/// How long a month is, by the Gregorian leap rule. Zero for a month that does
/// not exist, so a day is never in range for one.
fn days_in_month(year: u32, month: u32) -> u32 {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    }
}

/// A civil timestamp followed by optional fractional seconds and a zone.
///
/// The same thing `date-time` means in the frozen contract, checked here so a
/// rejected line can say which field is not a timestamp. The schema remains
/// the authority on whether the line is accepted.
pub fn is_date_time(text: &str) -> bool {
    let (civil, rest) = match text.char_indices().nth(19) {
        Some((index, _)) => text.split_at(index),
        None => (text, ""),
    };
    if !is_civil_datetime(civil) {
        return false;
    }

    let zone = match rest.strip_prefix('.') {
        None => rest,
        Some(after_dot) => {
            let digits = after_dot.chars().take_while(char::is_ascii_digit).count();
            if digits == 0 {
                return false;
            }
            &after_dot[digits..]
        }
    };

    if zone == "Z" || zone == "z" {
        return true;
    }
    let Some(offset) = zone.strip_prefix(['+', '-']) else {
        return false;
    };
    let bytes = offset.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return false;
    }
    let digits = [0usize, 1, 3, 4]
        .iter()
        .all(|index| bytes[*index].is_ascii_digit());
    digits
        && offset[..2].parse::<u32>().is_ok_and(|hours| hours < 24)
        && offset[3..].parse::<u32>().is_ok_and(|minutes| minutes < 60)
}

/// The same instant, written in UTC with a `Z`, or `None` when it cannot be.
///
/// `date-time` lets a file write `2026-08-20T23:00:00+08:00`, and the graph
/// compares stored instants as strings — RFC 3339 in UTC sorts
/// lexicographically, which is the whole reason every writer normalizes before
/// storing. An offset left in place breaks that: `23:00+08:00` is 15:00Z and
/// therefore *earlier* than `16:00Z`, and a string comparison says the
/// opposite. So the offset is applied here, before the value is staged, rather
/// than anywhere downstream.
///
/// `None` for an instant that is not the shape [`is_date_time`] accepts, and
/// for one so close to either end of the calendar that applying its offset
/// leaves a year that will not fit in four digits — five digits past 9999, a
/// sign before year zero. Those are refused rather than stored, for the reason
/// `telegram::representable_instant` gives: one unreadable row makes every
/// later rebuild fail on data nobody can edit.
///
/// Fractional seconds are dropped. The scorer's clock is whole seconds and the
/// contract's other writers render whole seconds, so keeping them would make
/// one importer's output sort differently from everyone else's.
pub fn to_utc(text: &str) -> Option<String> {
    if !is_date_time(text) {
        return None;
    }
    // `is_date_time` has already established that the first nineteen bytes are
    // the ASCII civil form, so these slices are on character boundaries.
    let number = |from: usize, to: usize| text[from..to].parse::<i64>().ok();
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (hour, minute, second) = (number(11, 13)?, number(14, 16)?, number(17, 19)?);

    let zone = &text[19..];
    let zone = match zone.strip_prefix('.') {
        None => zone,
        Some(after_dot) => {
            let digits = after_dot.chars().take_while(char::is_ascii_digit).count();
            &after_dot[digits..]
        }
    };
    let offset_seconds = match zone.as_bytes().first()? {
        b'Z' | b'z' => 0,
        sign => {
            let magnitude =
                zone[1..3].parse::<i64>().ok()? * 3_600 + zone[4..6].parse::<i64>().ok()? * 60;
            match sign {
                b'+' => magnitude,
                _ => -magnitude,
            }
        }
    };

    let unix_seconds =
        days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second
            - offset_seconds;
    let rendered = soul_policy::clock::rfc3339_utc(unix_seconds);
    // `rfc3339_utc` renders any `i64`, including years of five digits and
    // years before the common era. Only the twenty-character form is an
    // instant the rest of Soul parses and sorts.
    match rendered.len() == 20 && is_date_time(&rendered) {
        true => Some(rendered),
        false => None,
    }
}

/// Howard Hinnant's `days_from_civil`, shifted to the Unix epoch.
///
/// The inverse of the `civil_from_days` in `soul_policy::clock`, which is what
/// renders the result of [`to_utc`].
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_the_month_does_not_have_is_not_an_instant() {
        assert!(!is_civil_datetime("2026-02-31T00:00:00"));
        assert!(!is_civil_datetime("2026-04-31T00:00:00"));
        assert!(is_civil_datetime("2026-04-30T00:00:00"));
        assert!(is_civil_datetime("2026-01-31T23:59:59"));
        assert!(!is_civil_datetime("2026-13-01T00:00:00"));
        assert!(!is_civil_datetime("2026-01-00T00:00:00"));
    }

    #[test]
    fn february_the_twenty_ninth_only_exists_in_a_leap_year() {
        assert!(is_civil_datetime("2024-02-29T12:00:00"));
        assert!(!is_civil_datetime("2025-02-29T12:00:00"));
        assert!(!is_civil_datetime("2026-02-29T12:00:00"));
        // The hundred-year rule and the exception to it.
        assert!(!is_civil_datetime("1900-02-29T12:00:00"));
        assert!(is_civil_datetime("2000-02-29T12:00:00"));
    }

    #[test]
    fn the_shape_and_the_time_of_day_are_still_checked() {
        assert!(!is_civil_datetime("2026-04-30T00:00:00Z"));
        assert!(!is_civil_datetime("2026-04-30 00:00:00"));
        assert!(!is_civil_datetime("2026-04-30T24:00:00"));
        assert!(!is_civil_datetime("2026-04-30T00:60:00"));
        assert!(!is_civil_datetime("2026-04-30T00:00:60"));
    }

    #[test]
    fn the_calendar_reaches_the_zoned_form_too() {
        assert!(!is_date_time("2026-02-31T00:00:00Z"));
        assert!(!is_date_time("2025-02-29T00:00:00+08:00"));
        assert!(is_date_time("2024-02-29T00:00:00Z"));
        assert!(is_date_time("2024-02-29T00:00:00.500+08:00"));
    }

    /// The case the graph gets wrong when an offset survives into the store:
    /// as strings `23:00+08:00` beats `16:00Z`, and as instants it loses.
    #[test]
    fn an_offset_is_applied_rather_than_carried() {
        assert_eq!(
            to_utc("2026-08-20T23:00:00+08:00").as_deref(),
            Some("2026-08-20T15:00:00Z"),
        );
        assert!("2026-08-20T23:00:00+08:00" > "2026-08-20T16:00:00Z");
        assert!(
            to_utc("2026-08-20T23:00:00+08:00").expect("utc").as_str() < "2026-08-20T16:00:00Z"
        );
    }

    #[test]
    fn a_negative_offset_and_a_date_line_crossing_both_land_on_the_right_day() {
        assert_eq!(
            to_utc("2026-08-20T20:30:00-05:30").as_deref(),
            Some("2026-08-21T02:00:00Z"),
        );
        assert_eq!(
            to_utc("2026-01-01T07:00:00+08:00").as_deref(),
            Some("2025-12-31T23:00:00Z"),
        );
        // The leap day the hundred-year rule keeps, and the one it drops.
        assert_eq!(
            to_utc("2000-02-29T23:00:00+01:00").as_deref(),
            Some("2000-02-29T22:00:00Z"),
        );
        assert_eq!(
            to_utc("2024-03-01T00:30:00+01:00").as_deref(),
            Some("2024-02-29T23:30:00Z"),
        );
    }

    #[test]
    fn an_instant_already_in_utc_comes_back_unchanged_and_loses_its_fraction() {
        assert_eq!(
            to_utc("2026-08-24T08:00:00Z").as_deref(),
            Some("2026-08-24T08:00:00Z"),
        );
        assert_eq!(
            to_utc("2026-08-24T08:00:00z").as_deref(),
            Some("2026-08-24T08:00:00Z"),
        );
        assert_eq!(
            to_utc("2026-08-24T08:00:00.750Z").as_deref(),
            Some("2026-08-24T08:00:00Z"),
        );
    }

    #[test]
    fn a_field_that_is_not_an_instant_converts_to_nothing() {
        assert_eq!(to_utc("2026-02-31T00:00:00Z"), None);
        assert_eq!(to_utc("2026-08-24 08:00:00Z"), None);
        assert_eq!(to_utc("2026-08-24T08:00:00"), None);
        assert_eq!(to_utc(""), None);
    }

    /// Years the rest of Soul cannot read back are refused rather than
    /// rendered with five digits or a sign.
    #[test]
    fn an_offset_that_walks_off_the_calendar_is_refused() {
        assert_eq!(to_utc("9999-12-31T23:00:00-05:00"), None);
        assert_eq!(to_utc("0000-01-01T00:30:00+08:00"), None);
        // A hair inside the same edges still converts.
        assert_eq!(
            to_utc("9999-12-31T23:00:00+05:00").as_deref(),
            Some("9999-12-31T18:00:00Z"),
        );
        assert_eq!(
            to_utc("0001-01-01T09:00:00+08:00").as_deref(),
            Some("0001-01-01T01:00:00Z"),
        );
    }

    /// Every hour of a day, through the offsets a file may carry, keeps its
    /// order once converted — which is the property the graph relies on.
    #[test]
    fn conversion_preserves_the_order_of_the_instants_it_is_given() {
        let mut rendered: Vec<String> = Vec::new();
        for (civil, zone) in [
            ("2026-08-20T23:00:00", "+08:00"),
            ("2026-08-20T16:00:00", "Z"),
            ("2026-08-20T09:00:00", "-06:00"),
            ("2026-08-21T00:59:00", "+09:30"),
        ] {
            rendered.push(to_utc(&format!("{civil}{zone}")).expect("a real instant"));
        }
        assert_eq!(
            rendered,
            vec![
                "2026-08-20T15:00:00Z",
                "2026-08-20T16:00:00Z",
                "2026-08-20T15:00:00Z",
                "2026-08-20T15:29:00Z",
            ],
        );
        assert_eq!(
            rendered.iter().max().map(String::as_str),
            Some("2026-08-20T16:00:00Z"),
            "the true latest instant is the one that sorts last",
        );
    }
}
