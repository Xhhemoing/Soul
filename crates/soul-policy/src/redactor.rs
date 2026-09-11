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

    /// Both the label as it was stored and the same label with its whitespace
    /// taken out.
    ///
    /// An export writes a name the way its own format joins one — Telegram
    /// puts a space between `first_name` and `last_name`, so `李` and `雷`
    /// arrive as `李 雷` — and the ordinary spelling of that name has no space
    /// in it. One `replace` over the stored spelling misses the other, which
    /// on the E1 path means a paste that writes the person's name the way a
    /// person writes it reaches the endpoint unplaceheld.
    ///
    /// Only the whole label is stripped, never its parts. Registering `李` on
    /// its own would placehold 李先生 and every other ordinary use of the
    /// character out of the user's own prose, which is a different and worse
    /// failure than the one being fixed.
    pub fn add_name(&mut self, name: &str) -> &mut Self {
        let normalized = normalize(name);
        if normalized.is_empty() {
            return self;
        }
        let unspaced: String = normalized.chars().filter(|c| !c.is_whitespace()).collect();
        if !unspaced.is_empty() {
            self.names.insert(unspaced);
        }
        self.names.insert(normalized);
        self
    }

    /// Registered as written, in one width only.
    ///
    /// A contact card holds `13800138000` and a paste may spell the same
    /// number `１３８００１３８０００`, which is not this string and is not
    /// replaced by name. What covers it is [`phone_shape_end`], which reads a
    /// digit in [either width](is_phone_digit): a registered number is at
    /// least seven digits, so the shape reaches it whichever way it was typed.
    ///
    /// Folding the widths here as well would be the tighter fix, and it is a
    /// separate change: it belongs with the same fold on the leakage checker's
    /// corpus side, and unlike the shape net it can only be got right by
    /// deciding what an account that is not a phone number — a handle, an
    /// address — should fold to.
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
/// `@handles`, and digits — run together or grouped — in enough quantity to be
/// a phone number. See [`phone_shape_end`] for what counts as grouped.
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

        if is_phone_digit(c) {
            if let Some(end) = phone_shape_end(&scalars, index) {
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

/// Seven digits is the shortest thing that is plausibly a phone number rather
/// than a date, a count or a port.
const MIN_PHONE_DIGITS: usize = 7;

/// Where the phone-number shape starting at `start` ends, if there is one.
///
/// A person writing a number down for another person to read groups it:
/// `138 0013 8000` and `138-0013-8000` are the same number as `13800138000`,
/// and a rule that counted only unbroken runs saw none of them, because every
/// group on its own is shorter than a number. So the groups are counted
/// instead of the run. A single [separator](is_group_separator) joins one
/// group to the next, and it is the total that has to reach
/// [`MIN_PHONE_DIGITS`].
///
/// A digit is [either width](is_phone_digit), because the layout the number
/// was typed on is not something the person being protected chose.
///
/// Nothing else joins. Two separators in a row, a comma, a colon or a Han
/// character ends the run, which is what leaves 下午 3 点，第 2 会议室，预算
/// 45000 the sentence the user wrote: three groups that never touch.
///
/// What is replaced is the whole grouped run, separators included, rather than
/// one group at a time. Stopping at the first seven digits would leave ` 8000`
/// standing beside the placeholder, which is the last four digits of the
/// number on the wire.
///
/// # The false positives this buys
///
/// `2026-08-25` is eight digits in three groups, so it is placeheld. It is a
/// date and nobody can be reached on it, and a paste that quotes one loses it
/// to a placeholder.
///
/// The widenings above each add one of the same kind. A year range,
/// `2019–2026`, is eight digits in two groups joined by an en-dash, which is
/// the punctuation a range is written with far more often than a number is.
/// `２０２６－０８－２５` is that same ISO date, typed on an IME, and
/// `２０２６．０８．２５` is it again with the dot that IME gives: a
/// fullwidth-dotted digit run totalling seven digits or more is placeheld, on
/// exactly the terms the ASCII-dotted `2026.08.25` already was.
///
/// The trade is deliberate and it is the same one-directional trade
/// [`KnownIdentifiers::add_name`] makes: a placeholder too many is something
/// the user can see and work around, and a number on the wire is not.
/// Excusing the `\d{4}-\d{2}-\d{2}` shape by name would be a few lines, and it
/// would excuse every number that happens to be punctuated 4-2-2 along with
/// the dates; excusing the en-dash would put `138–0013–8000` back on the wire.
/// Letting a number through because of how it was written is the wrong
/// direction to be wrong in.
fn phone_shape_end(scalars: &[char], start: usize) -> Option<usize> {
    let mut index = start;
    let mut digits = 0usize;
    let mut end = start;

    loop {
        let group_end = run_end(scalars, index, is_phone_digit);
        if group_end == index {
            break;
        }
        digits += group_end - index;
        end = group_end;
        let separated = scalars
            .get(group_end)
            .is_some_and(|c| is_group_separator(*c))
            && scalars
                .get(group_end + 1)
                .copied()
                .is_some_and(is_phone_digit);
        if !separated {
            break;
        }
        index = group_end + 1;
    }

    (digits >= MIN_PHONE_DIGITS).then_some(end)
}

/// A digit of a phone number, in either of the two widths a keyboard produces.
///
/// U+FF10–U+FF19 are what a Chinese IME in fullwidth mode gives, and
/// `１３８００１３８０００` is the same number as `13800138000` to every reader
/// and every phone. `char::is_ascii_digit` is the whole reason it was not: the
/// phone shape was written against ASCII, so a number typed on the layout most
/// of this product's users have in front of them was not a number to it.
///
/// The two widths are folded here rather than by normalizing the text, because
/// NFKC would also fold the fullwidth punctuation around them and hand the
/// endpoint a message the user never wrote — and the message is the thing two
/// confirmation screens were about.
fn is_phone_digit(c: char) -> bool {
    c.is_ascii_digit() || ('\u{FF10}'..='\u{FF19}').contains(&c)
}

/// The characters a written-out number is grouped by, and no others.
///
/// A comma groups digits too (`45,000`), and it is left out on purpose: it is
/// also how a Chinese sentence separates its clauses, so joining across one
/// would let a budget and a room number add up to a phone number.
///
/// The dash is whichever dash the keyboard produced. An IME on a Chinese
/// layout gives U+FF0D, a document that has been through an autocorrect gives
/// U+2013 or U+2014, a spreadsheet gives U+2212, and a typographer's hyphen is
/// U+2010 or U+2011. They are one dash to the person reading the number, and a
/// rule that knew only the ASCII one placeheld `138-0013-8000` and left
/// `138–0013–8000` on the wire. Worse, it left `138-0013–8000` half done: the
/// run stopped at the dash it did not know, and `8000` — the last four digits
/// — stood beside the placeholder, which is the failure the whole-run
/// replacement above exists to prevent.
///
/// The dot is likewise whichever width it was typed in. The same IME in
/// fullwidth mode gives U+FF0E for the same key the ASCII dot is on, so
/// `１３８．００１３．８０００` is a number with neither an ASCII digit nor an
/// ASCII separator anywhere in it, and `138.0013．8000` is the half-corrected
/// line that left the tail standing. The ASCII dot was already accepted here;
/// U+FF0E is the same separator in the other width.
fn is_group_separator(c: char) -> bool {
    is_label_space(c)
        || matches!(c, '.' | '\u{FF0E}')
        || matches!(
            c,
            '-' | '\u{2010}'
                | '\u{2011}'
                | '\u{2012}'
                | '\u{2013}'
                | '\u{2014}'
                | '\u{2212}'
                | '\u{FF0D}'
        )
}

fn is_label_space(c: char) -> bool {
    c == ' ' || c == '\u{3000}'
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
