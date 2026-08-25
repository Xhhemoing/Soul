//! Reading what the endpoint sent back — as data, and only as data.
//!
//! DECISIONS D25 says external content is never instruction, and a model's
//! response is external content: the user's endpoint is theirs, but the bytes
//! coming out of it were shaped by whatever went in, injected material
//! included. So this module parses, checks, and returns a string. It has no
//! notion of a tool call, no branch that acts on one, and nothing downstream
//! takes an action because of what a reply says.
//!
//! [`ModelReply::signals`] exists for the audit trail alone. It records what
//! the returned text *tried* to do so `injection.blocked` can say something
//! true, and nothing reads it back. A scanner that were load-bearing would be
//! a filter, and a filter can be evaded.
//!
//! [`is_grounded_in`] is the one check here that decides anything, and what it
//! decides is smaller than it looks: whether a reply has anything to do with
//! the material Soul sent it. It is not a proof of faithfulness and it is not
//! a safety filter. Deciding whether a sentence *follows from* a set of counts
//! is not something string comparison can do, and a check that claimed to
//! would license exactly the label this module exists to avoid.
//!
//! So the caller gets two things it can rely on and no more: a reply that
//! states a figure the material does not is refused, and a reply about
//! something else entirely is refused. A caller that shows the answer on one
//! labelled line gets a third — [`read_grounded_in`] also refuses an answer
//! that carries a line break, because a label introduces the line it sits on
//! and nothing underneath it. Everything that passes is still the endpoint's
//! own prose, still carries no evidence, and still has to be labelled as such
//! wherever it is shown.

use std::collections::BTreeSet;

use serde_json::Value;

use soul_policy::clinical::{assert_non_clinical, NonClinicalViolation};
use soul_policy::injection::{self, InjectionSignal, UntrustedText};

/// Model text that has been read as data and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelReply {
    pub text: String,
    /// What the reply attempted. Recorded for the audit entry, never obeyed.
    pub signals: Vec<InjectionSignal>,
}

/// Why a reply cannot be shown to the user.
///
/// None of these carry the endpoint's own words. A hostile endpoint controls
/// what it says, and a defect that quoted it would carry that text into a log.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplyDefect {
    #[error("the endpoint did not answer with JSON")]
    NotJson,
    #[error("the answer has no assistant message in it")]
    NoMessage,
    #[error("the assistant message is empty")]
    Empty,
    #[error("the answer carries vocabulary this product must not use: {0}")]
    Clinical(#[from] NonClinicalViolation),
    #[error("the answer states a figure the material does not, or is about something else")]
    Ungrounded,
    #[error("the answer is more than one line, and it is shown on one labelled line")]
    NotOneLine,
}

/// Pull the assistant's text out of an OpenAI-compatible response.
///
/// Both spellings are accepted because "OpenAI-compatible" is a family rather
/// than a specification: `choices[].message.content` is the chat shape and
/// `choices[].text` is the completion shape, and a user pointing Soul at their
/// own server should not have to care which one it speaks.
pub fn read(raw: &str) -> Result<ModelReply, ReplyDefect> {
    let value: Value = serde_json::from_str(raw).map_err(|_| ReplyDefect::NotJson)?;
    let choice = value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or(ReplyDefect::NoMessage)?;

    let text = choice
        .pointer("/message/content")
        .or_else(|| choice.get("text"))
        .and_then(Value::as_str)
        .ok_or(ReplyDefect::NoMessage)?
        .trim()
        .to_owned();

    if text.is_empty() {
        return Err(ReplyDefect::Empty);
    }
    assert_non_clinical(&text)?;

    let signals = injection::scan(&UntrustedText::new(&text));
    Ok(ModelReply { text, signals })
}

/// [`read`], for a request that handed the endpoint material to work from.
///
/// `material` is the body that went out — the bytes the endpoint saw, not a
/// second rendering of them. A reply that states a figure the material does
/// not, or that shares almost none of its subject, is refused with
/// [`ReplyDefect::Ungrounded`], so a caller that asked for 「把这些计数改写一遍」
/// is not left holding prose about something else with nothing to do but show
/// it.
///
/// An answer carrying a line break is refused too, and that rule is here
/// rather than in [`read`] because it is about where the answer ends up. A
/// draft is prose the user copies into a messaging app, where a paragraph
/// break is an ordinary thing to want. An answer read against material Soul
/// supplied is shown beside Soul's own lines under one label that names who
/// wrote it — and a label introduces the line it sits on, not the ones below.
/// [`read`] trims the ends, which is not the same thing: a break in the middle
/// survives trimming and puts the rest of the answer on a line of the screen
/// wearing no attribution at all.
pub fn read_grounded_in(raw: &str, material: &str) -> Result<ModelReply, ReplyDefect> {
    let reply = read(raw)?;
    if reply.text.contains(starts_a_new_line) {
        return Err(ReplyDefect::NotOneLine);
    }
    if !is_grounded_in(&reply.text, material) {
        return Err(ReplyDefect::Ungrounded);
    }
    Ok(reply)
}

/// Characters that begin a new line where an answer is displayed.
///
/// Line feed and carriage return are the two an endpoint actually sends. The
/// rest are the other Unicode mandatory breaks, and they are here because
/// `str::lines` splits on none of them: a rule written as "count the lines"
/// would call such an answer one line while an element that preserves breaks
/// renders it as several.
fn starts_a_new_line(character: char) -> bool {
    matches!(
        character,
        '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

/// Whether `text` is about `material`, and quotes no figure it does not have.
///
/// Two questions. The first is exact and the second is not, and the difference
/// between them is the reason this function's name says 是不是有关 rather than
/// 是不是忠实:
///
/// * **does it state a figure the material does not?** Every run of numerals —
///   Arabic, full-width or Chinese — in `text` has to occur in `material`. A
///   people summary is counts, so a number that was not in the counts is a
///   number the endpoint supplied, and no amount of hedging on the screen
///   makes that acceptable. This half is a decision, not an estimate;
/// * **is it about the same thing at all?** `text` and `material` have to
///   share at least [`SHARED_PAIRS`] distinct adjacent character pairs.
///   这个人最喜欢榴莲 shares 这个 and 个人 with a people summary and nothing
///   else, which is what an answer that ignored the material looks like.
///
/// What the second half cannot do is tell a rewrite from an interpretation.
/// 你们最近往来比较稳定 shares 你们 / 最近 / 往来 with the counts and adds
/// 稳定, which is a claim the counts do not make; it passes. That is a known
/// limit and not a gap to be closed by raising the number — the sentences a
/// user would want kept and the ones they would want dropped are not far
/// enough apart in characters for any threshold to separate them. What keeps
/// the screen honest is the label the caller puts on what it shows, and the
/// fact that the points and their evidence never came from here.
pub fn is_grounded_in(text: &str, material: &str) -> bool {
    if figures(text)
        .difference(&figures(material))
        .next()
        .is_some()
    {
        return false;
    }

    let known = bigrams(material);
    bigrams(text).intersection(&known).count() >= SHARED_PAIRS
}

/// How much of the material a reply has to be talking about.
///
/// Three pairs rather than one, because two of anything happen by accident in
/// a language where 这个 and 个人 are ordinary words.
const SHARED_PAIRS: usize = 3;

/// Characters that state a quantity. Arabic and full-width digits, and the
/// Chinese numerals a model writing 六次往来 would reach for.
fn is_figure(character: char) -> bool {
    character.is_ascii_digit()
        || ('０'..='９').contains(&character)
        || "〇零一二两三四五六七八九十百千万亿".contains(character)
}

/// Every maximal run of figure characters, as a set.
fn figures(text: &str) -> BTreeSet<String> {
    let mut runs = BTreeSet::new();
    let mut current = String::new();
    for character in text.chars() {
        match is_figure(character) {
            true => current.push(character),
            false => {
                if !current.is_empty() {
                    runs.insert(std::mem::take(&mut current));
                }
            }
        }
    }
    if !current.is_empty() {
        runs.insert(current);
    }
    runs
}

/// Adjacent pairs of word characters.
///
/// Pairs are taken inside a run of word characters rather than across the
/// punctuation between them, so two sentences that happen to sit next to each
/// other do not invent a pair neither of them contains.
fn bigrams(text: &str) -> BTreeSet<(char, char)> {
    let mut pairs = BTreeSet::new();
    let mut previous: Option<char> = None;
    for character in text.chars() {
        match character.is_alphanumeric() {
            true => {
                if let Some(left) = previous {
                    pairs.insert((left, character));
                }
                previous = Some(character);
            }
            false => previous = None,
        }
    }
    pairs
}
