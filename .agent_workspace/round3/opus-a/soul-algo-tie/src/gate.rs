//! The Granovetter ladder, written once and used by both rules.
//!
//! ```text
//! Strong    if reciprocal ∧ count >= strong_min_count ∧ days >= strong_min_days
//! Moderate  if reciprocal ∧ count >= moderate_min_count
//! Weak      otherwise
//! ```
//!
//! Deliberately venue-blind. T4 and T4D disagree about venue, and they
//! disagree in *what they feed this function*, not in how the ladder is
//! climbed:
//!
//! * **T4** passes every venue's counts and then applies a ceiling, so a
//!   group-only tie stops at [`crate::constants::GROUP_ONLY_CEILING`].
//! * **T4D** passes the one-to-one counts and one-to-one reciprocity, so group
//!   traffic never reaches the ladder at all.
//!
//! Sharing one body makes the comparison honest: if the ladder ever changes it
//! changes for both, and the ablation keeps measuring the venue decision
//! instead of accidentally measuring a divergence in the rung heights.
//!
//! One invariant falls out and is pinned by a test rather than by a comment: a
//! tie that only one side ever contributed to can never leave Weak, whatever
//! the volume.

use crate::types::Band;

/// What the rule counted, in whichever venue the rule decided to look at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observed {
    /// Both sides contributed, in the venue being counted.
    pub reciprocal: bool,
    pub count: u64,
    pub days: u64,
}

/// The bars, in the same unit as [`Observed`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Thresholds {
    pub strong_min_count: u64,
    pub strong_min_days: u64,
    pub moderate_min_count: u64,
}

/// The band, before any recency adjustment and before any venue ceiling.
pub const fn band(observed: Observed, thresholds: Thresholds) -> Band {
    if !observed.reciprocal {
        return Band::Weak;
    }
    if observed.count >= thresholds.strong_min_count && observed.days >= thresholds.strong_min_days
    {
        Band::Strong
    } else if observed.count >= thresholds.moderate_min_count {
        Band::Moderate
    } else {
        Band::Weak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BARS: Thresholds = Thresholds {
        strong_min_count: 10,
        strong_min_days: 3,
        moderate_min_count: 3,
    };

    const fn observed(reciprocal: bool, count: u64, days: u64) -> Observed {
        Observed {
            reciprocal,
            count,
            days,
        }
    }

    #[test]
    fn one_sided_can_never_leave_weak() {
        for count in [0, 3, 10, 1_000_000] {
            for days in [0, 1, 3, 10_000] {
                assert_eq!(band(observed(false, count, days), BARS), Band::Weak);
            }
        }
    }

    #[test]
    fn the_strong_corner_is_exact() {
        assert_eq!(band(observed(true, 10, 3), BARS), Band::Strong);
        assert_eq!(band(observed(true, 9, 3), BARS), Band::Moderate);
        assert_eq!(band(observed(true, 10, 2), BARS), Band::Moderate);
    }

    #[test]
    fn the_moderate_corner_is_exact() {
        assert_eq!(band(observed(true, 3, 1), BARS), Band::Moderate);
        assert_eq!(band(observed(true, 2, 1), BARS), Band::Weak);
        assert_eq!(band(observed(true, 2, 10_000), BARS), Band::Weak);
    }

    #[test]
    fn the_ladder_is_monotone_in_both_quantities() {
        // Relied on by `t4d.rs`: T4D counts a subset of T4's rows, so if the
        // ladder were not monotone, "T4D never says more than T4" would not
        // follow from the subset relation.
        for count in 0..14u64 {
            for days in 0..5u64 {
                let here = band(observed(true, count, days), BARS).rank();
                assert!(here <= band(observed(true, count + 1, days), BARS).rank());
                assert!(here <= band(observed(true, count, days + 1), BARS).rank());
            }
        }
    }
}
