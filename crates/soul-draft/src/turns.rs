//! Pasted text becoming turns, and the audit entries the injection scan owes.
//!
//! There is one way into this crate and it takes [`UntrustedText`]. A function
//! that took a `String` would be a function someone could hand a prompt to; a
//! function that took the paste and returned an instruction would be worse.
//! What comes out is [`Turn`] values for `soul-policy`'s redactor, and nothing
//! downstream reads the prose again except that redactor.
//!
//! The scan in [`injection_audit`] is not a filter. `soul_policy::injection`
//! says so in its own module docs and this crate obeys it: a hit adds an audit
//! entry and changes nothing else — not the turns, not the route, not whether
//! the draft happens.

use uuid::Uuid;

use soul_policy::audit::AuditContent;
use soul_policy::injection::{scan, InjectionSignal, UntrustedText};
use soul_policy::redactor::Turn;
use soul_policy::ReasonCode;
use soul_schema::audit::AuditAction;
use soul_schema::common::SealedSubject;

/// One message the user pasted into the draft box.
///
/// The identifier is minted here rather than when the turn is assembled, so a
/// caller can name exactly one turn in an `ExemptionRequest` before drafting
/// and get the turn it meant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PastedTurn {
    turn_id: Uuid,
    subject: SealedSubject,
    body: UntrustedText,
}

impl PastedTurn {
    pub fn new(subject: SealedSubject, body: UntrustedText) -> PastedTurn {
        PastedTurn {
            turn_id: Uuid::now_v7(),
            subject,
            body,
        }
    }

    /// A paste whose owner the user did not state.
    ///
    /// `Mixed`, never `Owner`: `Turn::is_third_party` counts `Mixed` as
    /// somebody else's, which is the reading that keeps the prose behind a
    /// placeholder. Drafting does not reclassify a turn afterwards.
    pub fn unattributed(body: UntrustedText) -> PastedTurn {
        PastedTurn::new(SealedSubject::Mixed, body)
    }

    pub fn turn_id(&self) -> Uuid {
        self.turn_id
    }

    pub fn subject(&self) -> SealedSubject {
        self.subject
    }

    pub fn body(&self) -> &UntrustedText {
        &self.body
    }

    pub fn into_turn(self) -> Turn {
        Turn {
            turn_id: self.turn_id,
            subject: self.subject,
            body: self.body,
        }
    }
}

/// Assemble the conversation the redactor will work on.
pub fn turns_from(pasted: Vec<PastedTurn>) -> Vec<Turn> {
    pasted.into_iter().map(PastedTurn::into_turn).collect()
}

/// What each turn's content tried, for the audit trail.
///
/// Turns with nothing to report are left out, so an empty result means the
/// scan found nothing rather than that it did not run.
pub fn injection_signals(turns: &[Turn]) -> Vec<(Uuid, Vec<InjectionSignal>)> {
    turns
        .iter()
        .filter_map(|turn| match scan(&turn.body) {
            signals if signals.is_empty() => None,
            signals => Some((turn.turn_id, signals)),
        })
        .collect()
}

/// One `injection.blocked` entry per turn that carried markers.
///
/// "Blocked" describes the authority the content asked for and did not get,
/// not a turn that was dropped: the turn still goes through the same redaction
/// as every other one. The entry carries the turn id and a reason code, never
/// the markers and never the prose.
pub fn injection_audit(turns: &[Turn]) -> Vec<AuditContent> {
    injection_signals(turns)
        .into_iter()
        .map(|(turn_id, _)| {
            AuditContent::denied(
                AuditAction::InjectionBlocked,
                ReasonCode::InjectionMarkersFound,
            )
            .about(&[turn_id])
        })
        .collect()
}
