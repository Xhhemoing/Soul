//! Round 3 reference implementation of the Soul trait-axis and
//! personnel-summary algorithm family. Pure functions over plain data, so every
//! claim below can be checked without a store, a clock, a network or a
//! platform.
//!
//! # What is retained, and what is only present
//!
//! `R2-SYNTHESIS.md` retains exactly two algorithms for v0.1: one tie-strength
//! algorithm (in the sibling package) and **A0**. This package holds keeper #2
//! and everything around it.
//!
//! | Id | Module | Status |
//! |---|---|---|
//! | **A0** | [`a0`] | **Retained — keeper #2.** Questionnaire + correction lock, intake path patched. Frozen by Round 3. |
//! | **A2** | [`a2`] | **Retained as A0's renderer, not as an algorithm.** Turns a [`a2::TieScore`] into citable sentences and defines no bands. Frozen by Round 3. |
//! | A1 | [`a1`] | **Not retained.** An aggregation rule A0 could adopt later, tested and inert. Nothing in the frozen path calls it. |
//! | A3 | [`a3`] | **Rejected.** Lexicon inference from message text. Refuses, in code, with a test. |
//!
//! A2 is on the retained side of the line because dropping it would leave the
//! personnel summary with no implementation at all, and it is *not* counted as
//! one of the two because it makes no decision: given a [`a2::TieScore`] its
//! output is a pure function of the band somebody else chose. Counting a
//! renderer as an algorithm to make the list add up to two would be arithmetic,
//! not selection.
//!
//! # What Round 3 changed
//!
//! - **A0 and A2 are frozen.** A0's behaviour, its identifier and its default
//!   write mode are unchanged from Round 2 and pinned by
//!   `tests/frozen_defaults.rs`. A2 is unchanged except for the additive field
//!   pair below.
//! - **A1's default independence is now
//!   [`a1::A1Independence::TwoKindsAcrossDays`]**, so
//!   [`a1::A1_EMPTY_ON_V01`] is `true`: on a v0.1 install A1 provably cannot
//!   reach `Strong`. Round 2's `(kind, utc_day)` reading survives as the named
//!   alternative [`a1::A1Independence::KindAndDay`], which is not reachable
//!   without naming it. Answering the same questionnaire on three different
//!   days is no longer three independent observations.
//! - **[`a2::TieScore`] gained [`a2::TieScore::direct_count`] and
//!   [`a2::TieScore::group_count`]**, both optional. When a tie algorithm
//!   splits its tally by venue — as T4D would — A2 says what the split is; when
//!   it does not, A2 says nothing. A2 still has no threshold of its own.
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
//!   still returned so the user can see the machine disagrees. On the retained
//!   path a correction is also the *only* way to reach [`Band::Strong`].
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
    IntakeAnswer, IntakeReport, IntakeSkip, WriteMode, A0_ALGORITHM_ID, A0_DEFAULT_WRITE_MODE,
};
pub use a1::{
    a1_all_axes, a1_axis_state, a1_axis_state_with, a1_can_fire_on_v01_data, A1Independence,
    A1Outcome, A1Reason, EvidenceGroup, IndependenceKey, A1_ALGORITHM_ID, A1_DEFAULT_INDEPENDENCE,
    A1_EMPTY_ON_V01, A1_GROUPS_FOR_STRONG,
};
pub use a2::{
    a2_render, PersonnelSummary, SummaryBullet, TieScore, A2_ALGORITHM_ID, A2_STATEMENT_KEYS,
    DORMANT_AFTER_DAYS,
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
