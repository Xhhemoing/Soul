//! Round 1 reference implementations of the Soul trait-axis and
//! personnel-summary algorithm family.
//!
//! Four candidates, all pure functions over plain data, so they can be
//! compared against each other without a store, a clock, a network or a
//! platform:
//!
//! | Id | Module | What it is |
//! |---|---|---|
//! | A0 | [`a0`] | Questionnaire + correction lock. The Goal 1 baseline. |
//! | A1 | [`a1`] | A0 plus a band upgrade when independent evidence agrees. |
//! | A2 | [`a2`] | Statistical personnel summary; the no-key WP10 path. |
//! | A3 | [`a3`] | Lexicon inference from message text. **Refuses. Rejected.** |
//!
//! # What holds for all of them
//!
//! - **Evidence or nothing.** Every state and every bullet cites the rows
//!   behind it; nothing is claimed from an empty set.
//! - **Forgettable.** Forgotten rows are dropped before aggregation, so
//!   replaying the log after a forget is how the derived state comes back
//!   down. No algorithm here keeps a running total that a forget could not
//!   reach.
//! - **A correction wins.** Only the user locks an axis, and once locked no
//!   inference moves it. Refused inferences are still returned so the user can
//!   see the machine disagrees.
//! - **No score.** Direction plus band, never a number on a scale (D22).
//! - **No clinical claim.** Every user-facing string passes
//!   [`denylist::assert_publishable`] (D5).
//! - **No message text.** A0/A1/A2 never take one as input, and A3 refuses to.
//! - **Deterministic.** Times are parameters, iteration order is input order,
//!   and there is no hashing or randomness, so a test result is a fact rather
//!   than a probability.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod a0;
pub mod a1;
pub mod a2;
pub mod a3;
pub mod denylist;
pub mod fixtures;
pub mod types;

pub use a0::{a0_all_axes, a0_axis_state, A0_ALGORITHM_ID};
pub use a1::{
    a1_all_axes, a1_axis_state, a1_axis_state_with, DisagreementPolicy, A1_ALGORITHM_ID,
    A1_INDEPENDENT_FOR_STRONG,
};
pub use a2::{a2_personnel_summary, PeerStats, PersonnelSummary, SummaryBullet, A2_ALGORITHM_ID};
pub use a3::{a3_from_message_text, a3_from_messages, A3Refused, A3_ALGORITHM_ID};
pub use types::{
    ApplyResult, AxisId, AxisOutcome, AxisState, Band, EvidenceRef, Position, ShadowInference,
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
