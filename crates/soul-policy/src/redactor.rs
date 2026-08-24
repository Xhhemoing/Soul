//! Third-party prose never leaves the machine by default.
//!
//! PRODUCT_LOCK is specific about the shape of this: prose belonging to
//! someone other than the user is replaced by a placeholder before a request
//! body is built; names and account handles are replaced too, whatever their
//! length; a single request may carry the original after the user confirms
//! twice; and that permission is not remembered, so the next draft is
//! placeheld again. The research track has no exemption at all.
//!
//! Three design consequences worth stating, because they are what make the
//! promise testable rather than aspirational:
//!
//! * [`RedactedBody`] has private fields and no public constructor outside
//!   this module, and it is the only thing `soul-egress` will send. A caller
//!   cannot assemble a request body out of raw strings and skip this file.
//! * [`OneShotExemption`] is consumed by value and carries the identifier of
//!   the one turn it covers, so it cannot be reused, stored, or widened. The
//!   redactor holds no state about it, which is what "not remembered" means:
//!   there is nowhere for it to be remembered.
//! * [`redact_for_research`] takes no exemption parameter. The absence is the
//!   enforcement.

use std::collections::BTreeSet;

use unicode_normalization::UnicodeNormalization;
use uuid::Uuid;

use soul_schema::common::SealedSubject;

use crate::injection::UntrustedText;

/// What replaces a third-party body.
pub const THIRD_PARTY_PLACEHOLDER: &str = "[第三人正文已占位]";
/// What replaces a name or display label.
pub const NAME_PLACEHOLDER: &str = "[姓名已占位]";
/// What replaces an account handle, address or number.
pub const ACCOUNT_PLACEHOLDER: &str = "[账号已占位]";

/// One turn of the conversation being drafted against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    /// Stable identity of this turn, so an exemption can name exactly one.
    pub turn_id: Uuid,
    pub subject: SealedSubject,
    /// The prose. External by construction: it came from an import, a paste
    /// or a file name, and is never treated as instruction.
    pub body: UntrustedText,
}

impl Turn {
    pub fn new(turn_id: Uuid, subject: SealedSubject, body: impl Into<String>) -> Turn {
        Turn {
            turn_id,
            subject,
            body: UntrustedText::new(body),
        }
    }

    /// `mixed` is handled as third-party: PRODUCT_LOCK says so explicitly,
    /// because the part that is not the user's is the part that matters.
    pub fn is_third_party(&self) -> bool {
        matches!(
            self.subject,
            SealedSubject::ThirdParty | SealedSubject::Mixed
        )
    }
}

/// Names, handles, addresses and numbers that must never appear verbatim.
///
/// Supplied by the caller from the contact graph. Pattern matching catches the
/// shapes nobody registered; this catches the ones that look like ordinary
/// words, such as a two-character Chinese name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnownIdentifiers {
    names: BTreeSet<String>,
    accounts: BTreeSet<String>,
}

impl KnownIdentifiers {
    pub fn new() -> KnownIdentifiers {
        KnownIdentifiers::default()
    }

    pub fn with_name(mut self, name: &str) -> KnownIdentifiers {
        self.add_name(name);
        self
    }

    pub fn with_account(mut self, account: &str) -> KnownIdentifiers {
        self.add_account(account);
        self
    }

    pub fn add_name(&mut self, name: &str) -> &mut Self {
        let normalized = normalize(name);
        if !normalized.is_empty() {
            self.names.insert(normalized);
        }
        self
    }

    pub fn add_account(&mut self, account: &str) -> &mut Self {
        let normalized = normalize(account);
        if !normalized.is_empty() {
            self.accounts.insert(normalized);
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty() && self.accounts.is_empty()
    }
}

/// NFC, so a decomposed spelling cannot slip past a composed corpus. The
/// leakage checker in `soul-testkit` normalizes the same way.
fn normalize(text: &str) -> String {
    text.nfc().collect()
}

/// A request body that has been through the redactor.
///
/// Fields are private and there is no public constructor: `soul-egress`
/// depends on this crate, so the only way it can obtain one of these is from
/// [`Redactor::redact_for_e1`] or [`Redactor::redact_for_e1_with_exemption`].
/// That is the type-level half of "the request body must go through the
/// redactor first".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedBody {
    text: String,
    third_party_turns: usize,
    placeheld_turns: usize,
    /// The one turn the user confirmed twice for, if any.
    exempted_turn: Option<Uuid>,
}

impl RedactedBody {
    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn into_string(self) -> String {
        self.text
    }

    /// How many turns belonged to someone else.
    pub fn third_party_turns(&self) -> usize {
        self.third_party_turns
    }

    /// How many of those were replaced by a placeholder.
    pub fn placeheld_turns(&self) -> usize {
        self.placeheld_turns
    }

    /// `true` when this body carries one original third-party turn because the
    /// user confirmed it twice. The audit records the fact, never the prose.
    pub fn carries_exempted_original(&self) -> bool {
        self.exempted_turn.is_some()
    }

    pub fn exempted_turn(&self) -> Option<Uuid> {
        self.exempted_turn
    }
}

/// Step one of the two-step confirmation.
///
/// Holding this proves the user was asked. It is not permission yet, and it
/// cannot be turned into permission without [`ExemptionRequest::confirm`].
#[derive(Debug)]
pub struct ExemptionRequest {
    turn_id: Uuid,
}

impl ExemptionRequest {
    /// Ask to include one turn's original prose in the next request.
    pub fn for_turn(turn_id: Uuid) -> ExemptionRequest {
        ExemptionRequest { turn_id }
    }

    pub fn turn_id(&self) -> Uuid {
        self.turn_id
    }

    /// Step two. `confirmed` is the user's second answer; anything but `true`
    /// yields no exemption, and the caller falls back to placeholders.
    pub fn confirm(self, confirmed: bool) -> Option<OneShotExemption> {
        match confirmed {
            true => Some(OneShotExemption {
                turn_id: self.turn_id,
            }),
            false => None,
        }
    }
}

/// Permission to include exactly one turn's original prose, exactly once.
///
/// Consumed by value by the redactor and never stored anywhere, so there is no
/// object left to replay and no state to make the next draft behave
/// differently. Deliberately not `Clone` and not `Copy`.
#[derive(Debug)]
pub struct OneShotExemption {
    turn_id: Uuid,
}

impl OneShotExemption {
    pub fn turn_id(&self) -> Uuid {
        self.turn_id
    }
}

/// Turns a conversation into a request body.
///
/// Stateless with respect to exemptions on purpose; see the module docs.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    identifiers: KnownIdentifiers,
}

impl Redactor {
    pub fn new(identifiers: KnownIdentifiers) -> Redactor {
        Redactor { identifiers }
    }

    pub fn identifiers(&self) -> &KnownIdentifiers {
        &self.identifiers
    }

    /// The default path: every third-party body becomes a placeholder, and
    /// identifiers are placeheld everywhere, including inside the user's own
    /// turns.
    pub fn redact_for_e1(&self, turns: &[Turn]) -> RedactedBody {
        self.build(turns, None)
    }

    /// The exempted path: the one turn named by `exemption` keeps its original
    /// prose. Identifiers are still placeheld, and every other third-party
    /// turn is still a placeholder — the exemption is for one turn, not for
    /// the request.
    pub fn redact_for_e1_with_exemption(
        &self,
        turns: &[Turn],
        exemption: OneShotExemption,
    ) -> RedactedBody {
        self.build(turns, Some(exemption.turn_id))
    }

    /// The research path. There is no exemption parameter and no overload that
    /// takes one: PRODUCT_LOCK gives the research track no way to include
    /// third-party prose, so the function signature is the enforcement.
    ///
    /// Third-party turns are dropped rather than placeheld, because the
    /// research preview counts third-party rows and the count must be zero.
    pub fn redact_for_research(&self, turns: &[Turn]) -> RedactedBody {
        let mut lines = Vec::new();
        let mut third_party_turns = 0usize;
        for turn in turns {
            if turn.is_third_party() {
                third_party_turns += 1;
                continue;
            }
            lines.push(self.scrub_identifiers(turn.body.as_str()));
        }
        RedactedBody {
            text: lines.join("\n"),
            third_party_turns,
            placeheld_turns: third_party_turns,
            exempted_turn: None,
        }
    }

    /// Placehold identifiers in a single string, for a label or a file name
    /// that is not part of a conversation.
    pub fn scrub_identifiers(&self, text: &str) -> String {
        let mut out = normalize(text);
        for name in &self.identifiers.names {
            out = out.replace(name.as_str(), NAME_PLACEHOLDER);
        }
        for account in &self.identifiers.accounts {
            out = out.replace(account.as_str(), ACCOUNT_PLACEHOLDER);
        }
        scrub_identifier_shapes(&out)
    }

    fn build(&self, turns: &[Turn], exempted: Option<Uuid>) -> RedactedBody {
        let mut lines = Vec::with_capacity(turns.len());
        let mut third_party_turns = 0usize;
        let mut placeheld_turns = 0usize;
        let mut honoured: Option<Uuid> = None;

        for turn in turns {
            if !turn.is_third_party() {
                lines.push(self.scrub_identifiers(turn.body.as_str()));
                continue;
            }
            third_party_turns += 1;
            if exempted == Some(turn.turn_id) {
                honoured = Some(turn.turn_id);
                // Identifiers stay placeheld even here: the user confirmed to
                // send one message, not to publish a phone number.
                lines.push(self.scrub_identifiers(turn.body.as_str()));
                continue;
            }
            placeheld_turns += 1;
            lines.push(THIRD_PARTY_PLACEHOLDER.to_owned());
        }

        RedactedBody {
            text: lines.join("\n"),
            third_party_turns,
            placeheld_turns,
            exempted_turn: honoured,
        }
    }
}

/// Replace the identifier shapes nobody had to register: e-mail addresses,
/// `@handles`, and digit runs long enough to be a phone number.
///
/// Registered identifiers are handled before this; the shapes are the safety
/// net for a contact the graph never learned about.
fn scrub_identifier_shapes(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let scalars: Vec<char> = text.chars().collect();
    let mut index = 0usize;

    while index < scalars.len() {
        let c = scalars[index];

        if c == '@' {
            let end = run_end(&scalars, index + 1, is_handle_char);
            if end > index + 1 {
                out.push_str(ACCOUNT_PLACEHOLDER);
                index = end;
                continue;
            }
        }

        if is_email_local_char(c) {
            let local_end = run_end(&scalars, index, is_email_local_char);
            if scalars.get(local_end) == Some(&'@') {
                let domain_end = run_end(&scalars, local_end + 1, is_domain_char);
                let domain = &scalars[local_end + 1..domain_end];
                if domain.contains(&'.') && domain.last().is_some_and(|c| c.is_alphanumeric()) {
                    out.push_str(ACCOUNT_PLACEHOLDER);
                    index = domain_end;
                    continue;
                }
            }
        }

        if c.is_ascii_digit() {
            let end = run_end(&scalars, index, |c| c.is_ascii_digit());
            // Seven digits is the shortest thing that is plausibly a phone
            // number rather than a date, a count or a port.
            if end - index >= 7 {
                out.push_str(ACCOUNT_PLACEHOLDER);
                index = end;
                continue;
            }
        }

        out.push(c);
        index += 1;
    }

    out
}

fn run_end(scalars: &[char], from: usize, mut accept: impl FnMut(char) -> bool) -> usize {
    let mut end = from;
    while end < scalars.len() && accept(scalars[end]) {
        end += 1;
    }
    end
}

fn is_handle_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-'
}

fn is_email_local_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

fn is_domain_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '.' || c == '-'
}
