//! Every threshold in the family, in one place.
//!
//! Acceptance gate G3 in fable-a's `ACCEPTANCE.md` asks that the band
//! constants live in exactly one module so two call sites cannot drift apart.
//! This is that module: `t0`, `t3`, `t3r` and `t4` all read from here and
//! define no numbers of their own.
//!
//! ## Alignment with Goal 1 (Round 2 change)
//!
//! Round 1's T3 used `count >= 8` for the Strong gate, which was a guess made
//! before the candidates had to be compared side by side. fable-a's frozen
//! `CANDIDATE_SPEC` uses Goal 1's `count >= 10`. Round 2 aligns every
//! surviving candidate on the shipped constants — 3 / 10 / 3 — so that any
//! difference the ablation finds is caused by a rule, not by a threshold that
//! happens to differ. See `REPORT.md` §"常量对齐".

use crate::types::Band;

/// Reciprocal contact from here on is more than an exchanged greeting.
///
/// Goal 1 `graph_build.rs::MODERATE_MIN_INTERACTIONS`.
pub const MODERATE_MIN_INTERACTIONS: u64 = 3;

/// A strong tie has to be frequent.
///
/// Goal 1 `graph_build.rs::STRONG_MIN_INTERACTIONS`.
pub const STRONG_MIN_INTERACTIONS: u64 = 10;

/// A strong tie has to be spread over several days: twenty messages in one
/// afternoon is one conversation, not a habit.
///
/// Goal 1 `graph_build.rs::STRONG_MIN_ACTIVE_DAYS`.
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;

/// A tie that was never one to one cannot be Strong.
///
/// Granovetter's argument, spelled as a gate rather than as a weight: sitting
/// in the same two-hundred-person group as somebody for a year is the textbook
/// weak tie. T0 does not have this gate, which is why it can be flooded.
pub const STRONG_REQUIRES_DIRECT: bool = true;

/// The highest band a tie that was never one to one can reach.
///
/// `Moderate`, not `Weak`: a colleague you are in a project group with every
/// day is not a stranger, and telling the user they are would make the graph
/// look wrong. Round 1 opus-a §3.1 argued this; Round 2 keeps it and pins the
/// strict reading (`Weak`) in a test so flipping it stays a one-line change.
pub const GROUP_ONLY_CEILING: Band = Band::Moderate;

// ---------------------------------------------------------------------------
// T3R — bucketed decay
// ---------------------------------------------------------------------------

/// Quarter-interaction units per whole interaction.
///
/// The decay weights are 1, 1/2, 1/4 and 0. Multiplying everything by four
/// makes them 4, 2, 1 and 0, so the whole rule is integer arithmetic and there
/// is no float anywhere in the crate. fable-a's spec calls these "millicounts";
/// the name is kept for cross-slot consistency even though the unit is a
/// quarter, not a thousandth.
pub const MILLI_PER_INTERACTION: u64 = 4;

/// Lower edge, in whole days of age, of each decay bucket.
///
/// `[0, 90) [90, 180) [180, 360) [360, ∞)`.
pub const BUCKET_LOWER_EDGES_DAYS: [i64; 4] = [0, 90, 180, 360];

/// The weight of each bucket, in [`MILLI_PER_INTERACTION`] units.
///
/// Half-life 90 days, hard cut-off at 360. 90 rather than 30 because a monthly
/// half-life would demote a close friend over one long business trip; 360
/// rather than "never" because otherwise ten thousand messages from 2019 can
/// still carry a Strong band today.
pub const BUCKET_WEIGHTS_MILLI: [u64; 4] = [4, 2, 1, 0];

/// Age at which an interaction stops counting towards the band entirely.
///
/// It still shows in the raw counts: the evidence is not deleted, it just no
/// longer supports a claim about the present.
pub const DECAY_CUTOFF_DAYS: i64 = BUCKET_LOWER_EDGES_DAYS[3];

// ---------------------------------------------------------------------------
// T4 — integer demotion
// ---------------------------------------------------------------------------

/// Silence from here on costs one band.
pub const DEMOTE_AFTER_SILENT_DAYS: i64 = 180;

/// Silence from here on caps the tie at Weak, whatever the history says.
pub const WEAK_AFTER_SILENT_DAYS: i64 = 360;

/// When an explanation should volunteer that the tie has gone quiet.
///
/// Same number as [`DEMOTE_AFTER_SILENT_DAYS`], stated separately because it
/// is a rendering decision (fable-a's A2 "最近半年没有往来" sentence), not a
/// banding one.
pub const DORMANT_NOTICE_DAYS: i64 = 180;

/// The decay weight, in quarter units, of something `age_days` old.
///
/// The table above, read as a step function. Ages are clamped at zero before
/// they get here, so a timestamp from a clock that runs fast is worth today's
/// weight and never more.
pub fn bucket_weight_milli(age_days: i64) -> u64 {
    BUCKET_WEIGHTS_MILLI[bucket_of(age_days)]
}

/// Which bucket `age_days` falls in, as an index into the two tables.
pub fn bucket_of(age_days: i64) -> usize {
    let mut bucket = 0;
    let mut i = 0;
    while i < BUCKET_LOWER_EDGES_DAYS.len() {
        if age_days >= BUCKET_LOWER_EDGES_DAYS[i] {
            bucket = i;
        }
        i += 1;
    }
    bucket
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_constants_are_unchanged() {
        // Goal 1 `graph_build.rs`. If this test fails, the port stopped being a
        // port and REPORT.md owes an argument.
        assert_eq!(MODERATE_MIN_INTERACTIONS, 3);
        assert_eq!(STRONG_MIN_INTERACTIONS, 10);
        assert_eq!(STRONG_MIN_ACTIVE_DAYS, 3);
    }

    #[test]
    fn round_one_t3s_count_of_eight_is_gone() {
        // Documented alignment: Round 1 opus-a shipped T3 with a Strong gate of
        // 8, fable-a froze 10. Round 2 uses 10 everywhere.
        assert_ne!(STRONG_MIN_INTERACTIONS, 8);
    }

    #[test]
    fn buckets_step_down_at_the_documented_edges() {
        assert_eq!(bucket_weight_milli(0), 4);
        assert_eq!(bucket_weight_milli(89), 4);
        assert_eq!(bucket_weight_milli(90), 2);
        assert_eq!(bucket_weight_milli(179), 2);
        assert_eq!(bucket_weight_milli(180), 1);
        assert_eq!(bucket_weight_milli(359), 1);
        assert_eq!(bucket_weight_milli(360), 0);
        assert_eq!(bucket_weight_milli(100_000), 0);
    }

    #[test]
    fn the_weights_are_one_a_half_a_quarter_and_nothing() {
        // Read back as fractions of a whole interaction, which is what the
        // Chinese explanation promises the user.
        assert_eq!(BUCKET_WEIGHTS_MILLI[0], MILLI_PER_INTERACTION);
        assert_eq!(BUCKET_WEIGHTS_MILLI[1] * 2, MILLI_PER_INTERACTION);
        assert_eq!(BUCKET_WEIGHTS_MILLI[2] * 4, MILLI_PER_INTERACTION);
        assert_eq!(BUCKET_WEIGHTS_MILLI[3], 0);
    }
}
