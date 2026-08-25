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
/// not even this shape says the file is damaged.
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
    let (month, day) = (number(5, 7), number(8, 10));
    let (hour, minute, second) = (number(11, 13), number(14, 16), number(17, 19));
    (1..=12).contains(&month) && (1..=31).contains(&day) && hour < 24 && minute < 60 && second < 60
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
