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

    /// Registers the label as it was written, and — when it is written the way
    /// an export writes one — the same name with its spaces taken out.
    ///
    /// This is where the spelling an import produces meets the spelling
    /// everybody else uses. A Telegram export writes a display name as `李 雷`,
    /// that is the string `soul-import` seals, and `soulcore` registers what it
    /// reads back. Chinese prose does not space its characters, so the person
    /// who is called that is written `李雷` in a paste, in a message, and in
    /// the one turn a user confirmed twice for. [`Redactor::scrub_identifiers`]
    /// is a `String::replace` over this set, so registering only the export's
    /// spelling left the ordinary one reaching the endpoint out of an imported
    /// Soul — 「姓名与账号两种情况下都占位」 has no such condition in it.
    ///
    /// The fold is exactly the shape [`scrub_spaced_label_shapes`] reads, held
    /// to the whole label: two to four groups of one or two Han characters,
    /// single spaces between them, six characters at most. `Wang Xiao` is not
    /// folded, because `WangXiao` is not a spelling anybody writes; a script
    /// that spaces its words carries no signal in the spacing.
    ///
    /// # What the fold costs
    ///
    /// A folded name is replaced wherever those characters stand, including in
    /// prose the user wrote themselves, and two Han characters are sometimes an
    /// ordinary word. [`NOT_A_NAME`] is consulted, but it is short and was
    /// written for a different rule, so a label whose folded form is an
    /// everyday word that is not on it — a nickname stored as `明 天`, folding
    /// to `明天` — costs that word a placeholder. The trade is deliberate and
    /// it is one-directional: the spaced label is a person this Soul imported,
    /// a placeholder too many is something the user can see and work around,
    /// and a name on the wire is not.
    pub fn add_name(&mut self, name: &str) -> &mut Self {
        let normalized = normalize(name);
        if normalized.is_empty() {
            return self;
        }
        if let Some(folded) = fold_spaced_label(&normalized) {
            self.names.insert(folded);
        }
        self.names.insert(normalized);
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
    ///
    /// 正文 exemption is not 姓名 exemption, and the exempted turn is held to a
    /// stricter identifier rule than the rest of the body for that reason: it
    /// is the only prose that leaves verbatim, so it is the only place a
    /// display label nobody registered could travel. See
    /// [`scrub_spaced_label_shapes`].
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

    /// What an exempted turn goes through: everything
    /// [`Redactor::scrub_identifiers`] does, and then the two name shapes that
    /// only bite when prose travels verbatim.
    ///
    /// 正文 exemption is not 姓名 exemption. Everywhere else a third-party turn
    /// is a placeholder, so the only unregistered name that can reach an
    /// endpoint is one inside the single turn the user confirmed twice for —
    /// and on a Soul that has imported nothing, *every* name is unregistered.
    ///
    /// The two shapes are the spelling ([`scrub_spaced_label_shapes`]) and the
    /// position ([`scrub_attributed_name_shapes`]). Neither guesses: what is
    /// left over after both is written down at
    /// [`scrub_attributed_name_shapes`], because a promise with a hole in it
    /// should say where the hole is.
    fn scrub_exempted_original(&self, text: &str) -> String {
        let identified = self.scrub_identifiers(text);
        scrub_attributed_name_shapes(&scrub_spaced_label_shapes(&identified))
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
                // send one message, not to publish a phone number or somebody's
                // name.
                lines.push(self.scrub_exempted_original(turn.body.as_str()));
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

/// The identifier shape the scrub above cannot see: a display label written
/// with spaces between its characters.
///
/// `李 雷` is how a Telegram export spells a person, and it is two ordinary
/// characters and a space — no digits, no `@`, no address, nothing
/// [`scrub_identifier_shapes`] can recognize. Registered labels are replaced
/// by name before this runs, so what is left for this to catch is the person
/// the contact graph never learned about; on a Soul that has imported nothing
/// that is every person there is, and the exempted turn is the one place their
/// name would travel verbatim.
///
/// The rule is the shape and nothing cleverer. Two to four groups of one or
/// two Han characters, separated by single spaces, six characters at most:
/// Chinese prose does not space its characters, so the spacing is the signal.
/// A name written the ordinary way — `李雷` inside a sentence — has no
/// spelling to recognize; what it can have is a position, which is
/// [`scrub_attributed_name_shapes`]'s half of the job.
///
/// The three numbers below are module-scope because
/// [`KnownIdentifiers::add_name`] folds a registered label by the same ones. A
/// fold and a shape that drifted apart would leave a spelling in the
/// identifier set that this cannot recognize, or the other way round.
fn scrub_spaced_label_shapes(text: &str) -> String {
    let scalars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0usize;

    while index < scalars.len() {
        // Only the head of a run of Han characters can begin a label. Anywhere
        // else and the "group" would be the tail of an ordinary word.
        let at_head =
            is_han(scalars[index]) && !index.checked_sub(1).is_some_and(|i| is_han(scalars[i]));
        let label_end = match at_head {
            true => spaced_label_end(&scalars, index, MAX_GROUP, MAX_GROUPS, MAX_CHARS),
            false => None,
        };
        match label_end {
            Some(end) => {
                out.push_str(NAME_PLACEHOLDER);
                index = end;
            }
            None => {
                out.push(scalars[index]);
                index += 1;
            }
        }
    }

    out
}

/// A given name is one or two characters; a surname likewise.
const MAX_GROUP: usize = 2;
/// `欧阳 娜娜` is four, and three groups of two is the generous end.
const MAX_CHARS: usize = 6;
const MAX_GROUPS: usize = 4;

/// A registered label with its spaces taken out, when the label is exactly the
/// shape [`scrub_spaced_label_shapes`] reads.
///
/// The whole string has to be the shape, which is the difference between this
/// and the scrub: there the shape is looked for inside prose, here the label
/// either is one or is not. Two groups are the minimum, so a label with no
/// space in it cannot reach the fold, and neither can `Wang Xiao`, whose
/// groups are not Han. A folded form that is an everyday word in
/// [`NOT_A_NAME`] is left unregistered rather than costing that word a
/// placeholder everywhere it appears.
fn fold_spaced_label(label: &str) -> Option<String> {
    let scalars: Vec<char> = label.chars().collect();
    if spaced_label_end(&scalars, 0, MAX_GROUP, MAX_GROUPS, MAX_CHARS) != Some(scalars.len()) {
        return None;
    }
    let folded: String = scalars
        .iter()
        .copied()
        .filter(|c| !is_label_space(*c))
        .collect();
    (!NOT_A_NAME.contains(&folded.as_str())).then_some(folded)
}

/// Where the spaced label starting at `start` ends, if there is one.
///
/// The whole run has to qualify. A spaced run that is longer than a name could
/// be is left alone rather than truncated to the first few characters: half a
/// placeholder in the middle of a sentence would be a worse answer than the
/// sentence, and a run that long is not the shape being described.
fn spaced_label_end(
    scalars: &[char],
    start: usize,
    max_group: usize,
    max_groups: usize,
    max_chars: usize,
) -> Option<usize> {
    let mut index = start;
    let mut groups = 0usize;
    let mut characters = 0usize;
    let mut end = start;

    loop {
        let group_end = run_end(scalars, index, is_han);
        let length = group_end - index;
        if length == 0 || length > max_group {
            break;
        }
        groups += 1;
        characters += length;
        end = group_end;
        if groups > max_groups || characters > max_chars {
            return None;
        }
        let separated = scalars.get(group_end).is_some_and(|c| is_label_space(*c))
            && scalars.get(group_end + 1).is_some_and(|c| is_han(*c));
        if !separated {
            break;
        }
        index = group_end + 1;
    }

    (groups >= 2).then_some(end)
}

/// The other shape a name has: not how it is spelled, but where it stands.
///
/// `李雷说周五的场地他已经订好了` is how a person writes a Chinese name — no
/// space, no digits, nothing [`scrub_spaced_label_shapes`] can see — and
/// `Wang Xiao said Friday's venue is booked` is the same sentence in a script
/// that spaces every word, so the spacing carries no signal there at all.
/// Both were leaving an exempted turn intact until this ran, on a Soul whose
/// contact graph is empty, which is the state a Soul is installed in.
///
/// What both have in common is the position: the name stands immediately in
/// front of a verb of saying, because a chat log is a list of who said what.
/// So the rule is two signals that have to agree, and it stays narrow on
/// purpose — the user confirmed twice to send *this message*, and a body full
/// of placeholders is not the message they confirmed:
///
/// * Han: the head of a run of Han characters, beginning with a surname from
///   [`HAN_SURNAMES`] (or a compound one from [`HAN_COMPOUND_SURNAMES`]), two
///   or three characters long, and not one of the everyday words in
///   [`NOT_A_NAME`] that happen to start with a surname character.
/// * Latin: two or three capitalized words in a row, which is what a display
///   label looks like and what an ordinary English clause does not.
///
/// and then, in either script, a verb of saying right after it.
///
/// # What this still does not catch
///
/// This is a shape, not a name detector, and the promise the user reads
/// (「姓名与账号两种情况下都占位」) is unconditional. The gap between the two
/// is narrow but real, and lives entirely inside the one turn a user confirmed
/// twice for on a Soul that has imported nobody:
///
/// * a name anywhere but in front of a verb of saying — `方案下周交给李雷` —
///   because nothing distinguishes those characters from ordinary words;
/// * a name whose surname is not in the list, or that is written the way
///   friends write one (`小王`, `老李`);
/// * a second name later in the same Han run, since only the head of a run is
///   examined and the run does not end at a name.
///
/// Closing those needs either the contact graph ([`KnownIdentifiers`], which
/// `soulcore` fills from the contact rows) or a step the user sees before the
/// bytes leave. The graph covers all three for anybody it holds, position and
/// surname and run alike, and it does so in the spelling a person writes as
/// well as the spaced one an export seals, because
/// [`KnownIdentifiers::add_name`] folds the spaces out of a label shaped like
/// one. What is left over is a person the graph never learned about, which on
/// a Soul that has imported nothing is everybody; `soulcore`'s `session_e1`
/// pins that in a test rather than leaving it to be rediscovered.
fn scrub_attributed_name_shapes(text: &str) -> String {
    let scalars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut index = 0usize;

    while index < scalars.len() {
        let current = scalars[index];
        let previous = index.checked_sub(1).map(|i| scalars[i]);
        // Only at the head of a word or of a Han run: anywhere else the
        // "name" would be the tail of an ordinary word.
        let name_end = match current {
            c if is_han(c) && !previous.is_some_and(is_han) => han_name_end(&scalars, index),
            c if c.is_uppercase() && !previous.is_some_and(char::is_alphanumeric) => {
                latin_label_end(&scalars, index)
            }
            _ => None,
        };
        match name_end {
            Some(end) => {
                out.push_str(NAME_PLACEHOLDER);
                index = end;
            }
            None => {
                out.push(current);
                index += 1;
            }
        }
    }

    out
}

/// Where the Han name starting at `start` ends, if a verb of saying follows it.
///
/// The shortest candidate wins, because the alternative is redacting a
/// character of the message the user confirmed on the chance that it belonged
/// to the name.
fn han_name_end(scalars: &[char], start: usize) -> Option<usize> {
    let run = run_end(scalars, start, is_han);
    let lengths: &[usize] = if HAN_COMPOUND_SURNAMES
        .iter()
        .any(|surname| matches_at(scalars, start, surname))
    {
        &[3, 4]
    } else if HAN_SURNAMES.contains(&scalars[start]) {
        &[2, 3]
    } else {
        return None;
    };

    for &length in lengths {
        let end = start + length;
        if end > run {
            break;
        }
        let candidate: String = scalars[start..end].iter().collect();
        if NOT_A_NAME.contains(&candidate.as_str()) {
            continue;
        }
        if speech_marker_follows(scalars, end) {
            return Some(end);
        }
    }
    None
}

/// Where the Latin display label starting at `start` ends, if a verb of saying
/// follows it.
///
/// Two capitalized words are the signal. One is not: the account owner's own
/// name is one word in the fixtures and in most sentences, PRODUCT_LOCK's
/// placeholder is for 第三人姓名, and a rule that ate a single capitalized word
/// would redact the user out of their own draft.
fn latin_label_end(scalars: &[char], start: usize) -> Option<usize> {
    /// `Mary Jo Wang` is three; four is a sentence.
    const MAX_WORDS: usize = 3;

    let mut index = start;
    let mut words = 0usize;
    let mut end = start;

    while words < MAX_WORDS {
        match capitalized_word_end(scalars, index) {
            Some(word_end) => {
                words += 1;
                end = word_end;
                index = word_end;
            }
            None => break,
        }
        let separated =
            scalars.get(index) == Some(&' ') && capitalized_word_end(scalars, index + 1).is_some();
        if !separated {
            break;
        }
        index += 1;
    }

    (words >= 2 && speech_marker_follows(scalars, end)).then_some(end)
}

/// One capitalized word: an initial capital and at least one lowercase letter.
///
/// `SYSTEM` is not a word a person is called, and `W` is an initial rather
/// than a name.
fn capitalized_word_end(scalars: &[char], start: usize) -> Option<usize> {
    let first = *scalars.get(start)?;
    if !first.is_alphabetic() || !first.is_uppercase() {
        return None;
    }
    let end = run_end(scalars, start + 1, |c| {
        c.is_alphabetic() && c.is_lowercase()
    });
    let ends_cleanly = !scalars.get(end).is_some_and(|c| c.is_alphanumeric());
    (end > start + 1 && ends_cleanly).then_some(end)
}

/// Whether a verb of saying begins at `at`, allowing the one space a label is
/// separated from its verb by.
fn speech_marker_follows(scalars: &[char], at: usize) -> bool {
    let after_space = match scalars.get(at) {
        Some(c) if is_label_space(*c) => at + 1,
        _ => at,
    };

    if HAN_SPEECH_MARKERS
        .iter()
        .any(|marker| matches_at(scalars, after_space, marker))
    {
        return true;
    }

    // A Latin verb has to be a whole word, and it has to have been separated
    // from the label by the space above: `Wangsaid` is one word.
    after_space > at
        && LATIN_SPEECH_MARKERS.iter().any(|marker| {
            matches_at(scalars, after_space, marker)
                && !scalars
                    .get(after_space + marker.chars().count())
                    .is_some_and(|c| c.is_alphanumeric())
        })
}

fn matches_at(scalars: &[char], at: usize, word: &str) -> bool {
    word.chars()
        .enumerate()
        .all(|(offset, c)| scalars.get(at + offset) == Some(&c))
}

/// Verbs of saying, in the script a paste on this machine arrives in.
///
/// Short enough to read: every entry is a verb that takes a person in front of
/// it, and the ones that are also ordinary words in other positions (`回`,
/// `称`, `道`) are left out rather than paid for in redacted prose.
const HAN_SPEECH_MARKERS: &[&str] = &[
    "说", "说道", "讲", "问", "表示", "告诉", "提到", "回复", "写道", "提醒", "通知", "发来",
];

const LATIN_SPEECH_MARKERS: &[&str] = &[
    "said",
    "says",
    "wrote",
    "writes",
    "asked",
    "asks",
    "told",
    "tells",
    "mentioned",
    "replied",
    "replies",
    "texted",
    "messaged",
];

/// The single-character surnames, which is the part of a Chinese name that
/// comes from a closed list. Roughly the hundred commonest, which is most of
/// the people there are; a surname outside it is [`KnownIdentifiers`]'s job.
const HAN_SURNAMES: &[char] = &[
    '王', '李', '张', '刘', '陈', '杨', '黄', '赵', '吴', '周', '徐', '孙', '马', '朱', '胡', '郭',
    '何', '高', '林', '罗', '郑', '梁', '谢', '宋', '唐', '许', '韩', '冯', '邓', '曹', '彭', '曾',
    '肖', '田', '董', '袁', '潘', '于', '蒋', '蔡', '余', '杜', '叶', '程', '苏', '魏', '吕', '丁',
    '任', '沈', '姚', '卢', '姜', '崔', '钟', '谭', '陆', '汪', '范', '金', '石', '廖', '贾', '夏',
    '韦', '付', '方', '白', '邹', '孟', '熊', '秦', '邱', '江', '尹', '薛', '闫', '段', '雷', '侯',
    '龙', '史', '陶', '黎', '贺', '顾', '毛', '郝', '龚', '邵', '万', '钱', '严', '覃', '武', '戴',
    '莫', '孔', '向', '汤',
];

const HAN_COMPOUND_SURNAMES: &[&str] = &[
    "欧阳", "司马", "上官", "诸葛", "皇甫", "慕容", "东方", "尉迟", "令狐", "端木", "独孤", "长孙",
];

/// Everyday words that begin with a surname character and can stand in front
/// of a verb of saying.
///
/// 「于是说」 is "and so said", not a person called 于是, and placeholding it
/// would take away part of the message the exemption was for. The list is
/// short because the surname anchor already excludes 我们/大家/老板/对方 and
/// the rest of what usually precedes 说.
const NOT_A_NAME: &[&str] = &[
    "于是", "马上", "方才", "白天", "高兴", "向来", "万一", "严重", "石头", "毛病",
];

/// Han, which is the script the spaced-label shape is about. A label in a
/// script that spaces its words anyway — `Wang Xiao` — has no spelling to tell
/// it apart from a sentence; what it can have is a position, which is
/// [`scrub_attributed_name_shapes`].
fn is_han(c: char) -> bool {
    matches!(c,
        '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{20000}'..='\u{2A6DF}')
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
