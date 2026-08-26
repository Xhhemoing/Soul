//! Every threshold both rules use, in one place.
//!
//! Acceptance gate G3 in fable-a's `ACCEPTANCE.md` asks that the band
//! constants live in exactly one module so two call sites cannot drift apart.
//! This is that module: `t4`, `t4d` and the test-only oracle all read from
//! here and define no numbers of their own.
//!
//! The numbers are frozen by R2-SYNTHESIS §冻结边界 and are not Round 3's to
//! move:
//!
//! * counts ladder 3 / 10 / 3, i.e. Goal 1's `graph_build.rs`;
//! * demotion at 180 whole days and the floor at 360, both **closed**
//!   intervals (`>=`, not `>`);
//! * a tie that was never one to one cannot be Strong.
//!
//! What Round 3 changes is not a number, it is *which rows are counted*: T4
//! counts every venue, T4D counts only the one-to-one ones. See `t4d.rs`.

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

/// The highest band T4 lets a tie that was never one to one reach.
///
/// `Moderate`, not `Weak`: T4's argument (Round 1 opus-a §3.1) is that a
/// colleague you are in a project group with every day is not a stranger.
///
/// **T4D has no such constant**, and that is the substantive difference
/// between the two rules rather than an oversight: T4D bands on one-to-one
/// counts, so a group-only tie has a direct count of zero, fails the
/// reciprocity gate on that count, and lands on Weak with no ceiling needed.
/// R2-SYNTHESIS §潜在边界风险 2 asked for exactly that. The cost is priced in
/// `REPORT.md` §群聊代价.
pub const GROUP_ONLY_CEILING: Band = Band::Moderate;

/// Silence from here on costs one band.
pub const DEMOTE_AFTER_SILENT_DAYS: i64 = 180;

/// Silence from here on caps the tie at Weak, whatever the history says.
pub const WEAK_AFTER_SILENT_DAYS: i64 = 360;

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
    fn the_frozen_recency_edges_are_closed_intervals_at_180_and_360() {
        // R2-SYNTHESIS §冻结边界: «>= 180 降一档；>= 360 → Weak。闭区间».
        assert_eq!(DEMOTE_AFTER_SILENT_DAYS, 180);
        assert_eq!(WEAK_AFTER_SILENT_DAYS, 360);
        assert_eq!(WEAK_AFTER_SILENT_DAYS - DEMOTE_AFTER_SILENT_DAYS, 180);
    }

    #[test]
    fn round_one_t3s_count_of_eight_is_gone() {
        // Documented alignment: Round 1 opus-a shipped a Strong gate of 8,
        // fable-a froze 10, Round 2 aligned on 10 everywhere.
        assert_ne!(STRONG_MIN_INTERACTIONS, 8);
    }
}
