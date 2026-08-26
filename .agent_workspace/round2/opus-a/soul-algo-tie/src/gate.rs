//! The Granovetter gate, written once and used by every survivor.
//!
//! T3 and T3R differ in *what they count*, not in *how they judge*. Sharing
//! one body makes that structural rather than aspirational: if the gate ever
//! changes, it changes for both, and the ablation keeps measuring the measure
//! instead of accidentally measuring a divergence in the rule.
//!
//! ```text
//! Strong    if reciprocal ∧ any_direct ∧ count ≥ strong_min_count
//!                          ∧ days ≥ strong_min_days
//! Moderate  if reciprocal ∧ count ≥ moderate_min_count
//! Weak      otherwise
//! then: group-only ties are capped at GROUP_ONLY_CEILING
//! ```
//!
//! Two invariants fall out and are pinned by tests rather than by comment:
//! a one-sided tie can never leave Weak, and a tie that was never one to one
//! can never reach Strong.

use crate::constants::{GROUP_ONLY_CEILING, STRONG_REQUIRES_DIRECT};
use crate::types::Band;

/// What the rule observed, in whatever unit the caller counts in.
///
/// T3 fills `count` and `days` with raw tallies; T3R fills them with
/// quarter-interaction units. The gate does not care which, as long as the
/// thresholds are in the same unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observed {
    pub reciprocal: bool,
    pub any_direct: bool,
    pub group_only: bool,
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

/// The band, before any recency adjustment.
pub fn band(observed: Observed, thresholds: Thresholds) -> Band {
    let strong = observed.reciprocal
        && (observed.any_direct || !STRONG_REQUIRES_DIRECT)
        && observed.count >= thresholds.strong_min_count
        && observed.days >= thresholds.strong_min_days;

    let uncapped = if strong {
        Band::Strong
    } else if observed.reciprocal && observed.count >= thresholds.moderate_min_count {
        Band::Moderate
    } else {
        Band::Weak
    };

    // Redundant while STRONG_REQUIRES_DIRECT holds — a group-only tie has no
    // direct exchange, so it cannot have passed the Strong branch. Kept
    // because the ceiling is the invariant the product cares about, and it
    // should survive somebody switching the flag off.
    if observed.group_only {
        uncapped.capped_at(GROUP_ONLY_CEILING)
    } else {
        uncapped
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

    fn observed(reciprocal: bool, any_direct: bool, count: u64, days: u64) -> Observed {
        Observed {
            reciprocal,
            any_direct,
            group_only: !any_direct && count > 0,
            count,
            days,
        }
    }

    #[test]
    fn one_sided_can_never_leave_weak() {
        for count in [0, 3, 10, 1_000_000] {
            for days in [0, 1, 3, 10_000] {
                assert_eq!(band(observed(false, true, count, days), BARS), Band::Weak);
            }
        }
    }

    #[test]
    fn group_only_can_never_reach_strong() {
        for count in [3, 10, 1_000_000] {
            for days in [1, 3, 10_000] {
                let got = band(observed(true, false, count, days), BARS);
                assert_ne!(got, Band::Strong, "count {count}, days {days}");
                assert_eq!(got, GROUP_ONLY_CEILING);
            }
        }
    }

    #[test]
    fn the_strong_corner_is_exact() {
        assert_eq!(band(observed(true, true, 10, 3), BARS), Band::Strong);
        assert_eq!(band(observed(true, true, 9, 3), BARS), Band::Moderate);
        assert_eq!(band(observed(true, true, 10, 2), BARS), Band::Moderate);
    }

    #[test]
    fn the_moderate_corner_is_exact() {
        assert_eq!(band(observed(true, true, 3, 1), BARS), Band::Moderate);
        assert_eq!(band(observed(true, true, 2, 1), BARS), Band::Weak);
    }
}
