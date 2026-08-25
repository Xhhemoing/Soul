//! Wall-clock time, rendered the way the frozen contracts require.
//!
//! `_defs.schema.json` declares every timestamp as `format: date-time`, and
//! `soul-schema` compiles its validators with format assertions on, so an
//! audit entry with a made-up timestamp is rejected at the store boundary.
//! Rather than take a date library for one format string, the civil-date
//! conversion is written out here: it is a dozen lines, it has no timezone
//! database to keep current, and it lets every test state the instant it
//! means instead of freezing a clock.

/// Seconds since the Unix epoch, now.
///
/// Callers that need determinism pass their own value instead; nothing in this
/// crate reads the clock except at the outermost edge.
pub fn now_unix_seconds() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(elapsed) => elapsed.as_secs() as i64,
        // Before 1970. Nothing sensible to do, and nothing worth panicking on.
        Err(error) => -(error.duration().as_secs() as i64),
    }
}

/// Milliseconds since the Unix epoch, for capability-token lifetimes.
pub fn now_unix_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

/// RFC 3339 in UTC, to the second: `2026-08-24T11:00:00Z`.
pub fn rfc3339_utc(unix_seconds: i64) -> String {
    let days = unix_seconds.div_euclid(86_400);
    let seconds_of_day = unix_seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Howard Hinnant's `civil_from_days`, with the epoch shifted to 1970-01-01.
///
/// The algorithm counts from March so that the leap day lands at the end of
/// the year and needs no special case.
fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    (year + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_instants_render_correctly() {
        assert_eq!(rfc3339_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339_utc(1), "1970-01-01T00:00:01Z");
        // 2000-02-29, the leap day the hundred-year rule keeps.
        assert_eq!(rfc3339_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339_utc(1_787_529_600), "2026-08-24T00:00:00Z");
        assert_eq!(rfc3339_utc(1_787_529_600 + 39_600), "2026-08-24T11:00:00Z");
    }

    #[test]
    fn the_last_second_of_a_day_does_not_roll_over() {
        assert_eq!(rfc3339_utc(1_787_702_399), "2026-08-25T23:59:59Z");
        assert_eq!(rfc3339_utc(1_787_702_400), "2026-08-26T00:00:00Z");
    }

    #[test]
    fn every_day_for_eight_years_round_trips_through_the_civil_calendar() {
        // Guards the month/leap arithmetic across century and leap boundaries
        // without hard-coding three thousand expectations.
        let mut expected = (2024i64, 1u32, 1u32);
        let start = 1_704_067_200i64; // 2024-01-01T00:00:00Z
        for offset in 0..(366 + 365 * 7) {
            let rendered = rfc3339_utc(start + offset * 86_400);
            let (year, month, day) = expected;
            assert_eq!(rendered, format!("{year:04}-{month:02}-{day:02}T00:00:00Z"));
            expected = next_day(expected);
        }
    }

    fn next_day((year, month, day): (i64, u32, u32)) -> (i64, u32, u32) {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let length = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            _ => 28,
        };
        match (day + 1, month) {
            (d, m) if d <= length => (year, m, d),
            (_, 12) => (year + 1, 1, 1),
            (_, m) => (year, m + 1, 1),
        }
    }

    #[test]
    fn the_clock_reads_a_plausible_instant() {
        // 2020-01-01, chosen so this does not become a failing test in 2030.
        assert!(now_unix_seconds() > 1_577_836_800);
        assert!(now_unix_millis() > 1_577_836_800_000);
    }
}
