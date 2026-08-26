//! Round 2 reference implementations of the Soul trait-axis and
//! personnel-summary algorithm family.
//!
//! Four candidates, all pure functions over plain data, so they can be
//! compared against each other without a store, a clock, a network or a
//! platform:
//!
//! | Id | Module | What it is |
//! |---|---|---|
//! | A0 | [`a0`] | Questionnaire + correction lock. The Goal 1 baseline, with the intake path patched. |
//! | A1 | [`a1`] | A0 plus a band upgrade when *independent* evidence agrees. |
//! | A2 | [`a2`] | Renders a [`a2::TieScore`] into citable sentences. Defines no bands. |
//! | A3 | [`a3`] | Lexicon inference from message text. **Refuses. Rejected.** |
//!
//! # What Round 2 changed
//!
//! - **A0's lock now holds on the intake path.** Goal 1's `intake` calls
//!   `place_axis` without asking whether the user has corrected the axis, so
//!   re-running the questionnaire moves a locked axis. [`a0::apply_intake`]
//!   consults the lock, skips those answers, and reports them.
//! - **A0 gained a non-clobbering write mode.** Last-write-wins stays the
//!   default and the regression contract; [`a0::WriteMode::NoDowngrade`] is the
//!   opt-in that stops a `Weak` inference erasing a `Moderate` answer's
//!   citation.
//! - **A1 counts independent groups, not rows.** The key is
//!   `(kind, utc_day)`, per `CANDIDATE_SPEC.md`. Five re-fills in one afternoon
//!   are one group.
//! - **A2 lost its thresholds.** It reads the band off the score it is handed
//!   and renders; it has no opinion about what a count means.
//!
//! # What holds for all of them
//!
//! - **Evidence or nothing.** Every state and every bullet cites the rows
//!   behind it; nothing is claimed from an empty set.
//! - **Forgettable.** Forgotten rows are dropped before aggregation, so
//!   replaying the log after a forget is how the derived state comes back
//!   down. No algorithm here keeps a running total that a forget could not
//!   reach.
//! - **A correction wins.** Only the user locks an axis, and once locked
//!   nothing the machine or the questionnaire says moves it. Refused rows are
//!   still returned so the user can see the machine disagrees.
//! - **No score.** Direction plus band, never a number on a scale (D22).
//! - **No clinical claim.** Every user-facing string passes
//!   [`denylist::assert_publishable`] (D5).
//! - **No message text.** A0/A1/A2 never take one as input, and A3 refuses to.
//! - **No clock.** `as_of` and every timestamp are parameters, iteration order
//!   is input order, and there is no hashing or randomness, so a test result is
//!   a fact rather than a probability.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod a0;
pub mod a1;
pub mod a2;
pub mod a3;
pub mod denylist;
pub mod fixtures;
pub mod types;

pub use a0::{
    a0_all_axes, a0_all_axes_with, a0_axis_state, a0_axis_state_with, apply_intake, IgnoredAnswer,
    IntakeAnswer, IntakeReport, IntakeSkip, WriteMode, A0_ALGORITHM_ID,
};
pub use a1::{
    a1_all_axes, a1_axis_state, a1_axis_state_with, A1Independence, A1Outcome, A1Reason,
    EvidenceGroup, IndependenceKey, A1_ALGORITHM_ID, A1_GROUPS_FOR_STRONG,
};
pub use a2::{
    a2_render, PersonnelSummary, SummaryBullet, TieScore, A2_ALGORITHM_ID, DORMANT_AFTER_DAYS,
};
pub use a3::{a3_from_message_text, a3_from_messages, A3Refused, A3_ALGORITHM_ID};
pub use types::{
    utc_day, ApplyResult, AxisId, AxisOutcome, AxisState, Band, EvidenceKind, EvidenceRef,
    Position, ShadowInference,
};

/// Every fixed user-facing string the axis vocabulary can produce: labels,
/// both poles, and the sentence each position reads as.
///
/// The screening tests walk this list, so a new axis or a reworded pole is
/// checked the moment it is added rather than the next time somebody
/// remembers to look.
pub fn axis_vocabulary() -> Vec<String> {
    let mut strings = Vec::new();
    for axis in AxisId::ALL {
        strings.push(axis.label().to_owned());
        strings.push(axis.leans_low().to_owned());
        strings.push(axis.leans_high().to_owned());
        for position in [
            Position::LeansLow,
            Position::Mixed,
            Position::LeansHigh,
            Position::Unknown,
        ] {
            strings.push(axis.describe(position));
            strings.push(axis.statement_key(position));
        }
    }
    strings
}
