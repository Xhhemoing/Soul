//! Recognising the two timestamp shapes the v0.1 importers meet.
//!
//! Neither importer converts between calendars or zones, so there is no date
//! library here — only enough checking to tell a real instant from a field
//! somebody filled in by hand. `soul-import-v1` promises RFC 3339 and Telegram
//! writes a zoneless civil timestamp beside a Unix second.

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
}
