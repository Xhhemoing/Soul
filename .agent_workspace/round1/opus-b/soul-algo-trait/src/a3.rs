//! **A3 — lexicon trait inference from message text. Rejected, and rejected in
//! code.**
//!
//! A3 is the candidate that reads what someone wrote and moves a trait axis
//! from the words in it (LIWC-style category counts, sentiment lexicons, and
//! their descendants). It is in this crate as a function that always refuses,
//! not as a paragraph in a design document, because a rejected candidate that
//! exists only in prose gets re-implemented by the next person who has the
//! idea. Here, the refusal has a test.
//!
//! # Why it is refused for v0.1
//!
//! - **It needs message bodies.** PRODUCT_LOCK puts third-party text at
//!   `local_only` and requires the redactor on anything leaving the machine.
//!   A0/A1/A2 never touch a message body, so there is nothing to leak; A3
//!   would make the body a load-bearing input to the profile.
//! - **It cannot be checked by the user.** 「算法必须可向用户解释」: a person
//!   can recount their questionnaire answers or count messages on an edge, but
//!   nobody can audit "your word choice leans this way" without being shown
//!   the words and the weights — and the weights are a score, which D22 rules
//!   out.
//! - **It clinicalises by default.** Published lexicons carry categories named
//!   after symptoms. D5 says this product is not clinical, and an algorithm
//!   whose safety depends on filtering its own vocabulary at the output is the
//!   wrong shape.
//! - **It has no baseline to beat.** A0 and A1 already produce corrigible,
//!   citable axes from evidence the user handed over on purpose. C8 requires a
//!   new mechanism to beat the baseline; A3 buys reach at the cost of every
//!   other criterion.
//!
//! If a later version revisits this, it needs explicit consent for text
//! analysis, an explanation that names the observation rather than a weight,
//! and a lock that behaves exactly as in A0. Until then this function returns
//! `Err`, and [`A3_ALGORITHM_ID`] is registered as rejected so nothing can
//! quietly stamp a state with it.

use crate::types::{AxisId, AxisState};

/// The identifier A3 would have used. Registered so it can be recognised and
/// refused, never stamped on a state.
pub const A3_ALGORITHM_ID: &str = "a3.lexicon_text_inference.rejected";

/// The one reason A3 ever returns.
pub const A3_REFUSAL_REASON: &str = "v0.1_forbids_text_trait_inference";

/// Why a text-derived trait inference was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A3Refused {
    /// Always [`A3_REFUSAL_REASON`]. A stable string so a caller can branch on
    /// it and an audit row can record it without holding any message text.
    pub reason: &'static str,
}

impl A3Refused {
    /// The refusal. There is no other constructor and no other value.
    pub fn new() -> Self {
        A3Refused {
            reason: A3_REFUSAL_REASON,
        }
    }
}

impl Default for A3Refused {
    fn default() -> Self {
        A3Refused::new()
    }
}

impl core::fmt::Display for A3Refused {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{}", self.reason)
    }
}

impl std::error::Error for A3Refused {}

/// Infer a trait axis from message text. Always refuses.
///
/// The parameters exist so the refusal sits on the same signature a working
/// implementation would have needed: this is the call site that must fail, not
/// a placeholder to be filled in. The text is neither read, copied, hashed nor
/// logged — the function borrows it and returns.
pub fn a3_from_message_text(
    _axis: AxisId,
    _message_text: &str,
    _evidence_ids: &[u64],
) -> Result<AxisState, A3Refused> {
    Err(A3Refused::new())
}

/// The batch form, refused for the same reason. Present so that "just do it in
/// bulk" is not an unguarded path.
pub fn a3_from_messages(
    _axis: AxisId,
    _messages: &[&str],
    _evidence_ids: &[u64],
) -> Result<AxisState, A3Refused> {
    Err(A3Refused::new())
}

/// Whether an algorithm id belongs to a candidate that was rejected. A write
/// boundary can call this before storing a state.
pub fn is_rejected_algorithm(algorithm_id: &str) -> bool {
    algorithm_id == A3_ALGORITHM_ID
}
