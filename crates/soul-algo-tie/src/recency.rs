//! The one recency step, shared by both rules.
//!
//! ```text
//! silence = max(0, as_of - last_contact_in_any_venue) / 86400
//!
//! if silence >= 360 -> Weak
//! else if silence >= 180 -> one band lower
//! else -> unchanged
//! ```
//!
//! Frozen by R2-SYNTHESIS §冻结边界, including the closed intervals: a tie
//! silent for exactly 180 days is demoted, and one silent for exactly 360 days
//! is Weak. Round 2's gpt-sol-b spelling with `>` is void.
//!
//! Two properties that matter more than the arithmetic:
//!
//! * **It only ever goes down.** Silence cannot promote anybody, so a rule's
//!   counts stage remains an upper bound on what it can say.
//! * **The clock is the newest exchange in any venue**, in both rules. T4D
//!   refuses to *band* on group traffic, but it still accepts a group message
//!   as evidence that the person has not vanished from the user's life. That
//!   asymmetry is deliberate and is the Round 3 brief's instruction; the risk
//!   it carries is written up in `REPORT.md` §任一场地的近因.

use crate::constants::{DEMOTE_AFTER_SILENT_DAYS, WEAK_AFTER_SILENT_DAYS};
use crate::types::Band;

/// The band after silence, given the band the counts stage produced.
pub const fn demote(band: Band, silent_days: i64) -> Band {
    if silent_days >= WEAK_AFTER_SILENT_DAYS {
        Band::Weak
    } else if silent_days >= DEMOTE_AFTER_SILENT_DAYS {
        band.demoted_once()
    } else {
        band
    }
}

/// Which threshold, if any, the silence has crossed. Used by the explanations
/// so they can name the number the user is being held to.
pub const fn crossed_threshold(silent_days: i64) -> Option<i64> {
    if silent_days >= WEAK_AFTER_SILENT_DAYS {
        Some(WEAK_AFTER_SILENT_DAYS)
    } else if silent_days >= DEMOTE_AFTER_SILENT_DAYS {
        Some(DEMOTE_AFTER_SILENT_DAYS)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_edges_are_closed_and_land_on_whole_days() {
        assert_eq!(demote(Band::Strong, 0), Band::Strong);
        assert_eq!(demote(Band::Strong, 179), Band::Strong);
        assert_eq!(demote(Band::Strong, 180), Band::Moderate);
        assert_eq!(demote(Band::Strong, 359), Band::Moderate);
        assert_eq!(demote(Band::Strong, 360), Band::Weak);
        assert_eq!(demote(Band::Strong, 10_000), Band::Weak);
    }

    #[test]
    fn a_moderate_tie_loses_its_last_rung_at_the_first_threshold() {
        assert_eq!(demote(Band::Moderate, 179), Band::Moderate);
        assert_eq!(demote(Band::Moderate, 180), Band::Weak);
        assert_eq!(demote(Band::Moderate, 360), Band::Weak);
        assert_eq!(demote(Band::Weak, 10_000), Band::Weak);
    }

    #[test]
    fn silence_never_promotes() {
        for band in [Band::Weak, Band::Moderate, Band::Strong] {
            for silence in [0, 1, 179, 180, 359, 360, 100_000] {
                assert!(demote(band, silence).rank() <= band.rank());
            }
        }
    }

    #[test]
    fn the_named_threshold_matches_the_step_that_fired() {
        assert_eq!(crossed_threshold(0), None);
        assert_eq!(crossed_threshold(179), None);
        assert_eq!(crossed_threshold(180), Some(180));
        assert_eq!(crossed_threshold(359), Some(180));
        assert_eq!(crossed_threshold(360), Some(360));
    }
}
