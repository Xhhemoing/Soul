//! Detects third-party prose surviving into text that is about to leave the
//! machine (an E1 request body, a research preview, a log line).
//!
//! Three rules, all applied after Unicode NFC normalization so that a
//! decomposed `café` cannot slip past a composed one:
//!
//! 1. any run of [`DEFAULT_MIN_NGRAM`] or more Unicode scalars taken from a
//!    third-party body is a leak — this is the `≥8` rule PRODUCT_LOCK names;
//! 2. a known name, handle, phone number or address is a leak at *any* length,
//!    because "李雷" is two scalars and still identifies someone;
//! 3. normalization applies to both sides, so NFD input is caught by an NFC
//!    corpus and the reverse.
//!
//! Rule 1 has a deliberate floor. A third-party reply shorter than the n-gram
//! threshold, such as `好的没问题`, is not caught by it; that is exactly why
//! rule 2 exists and why [`LeakageChecker::with_min_ngram`] is public.
//!
//! Rule 2 has a floor of its own that NFC does not lift. A registered number
//! is one string, and the same number reaches an endpoint written however the
//! keyboard wrote it: grouped, dashed with whichever dash, or typed in
//! fullwidth digits. NFC folds none of those together, so a checker that only
//! compared normalized substrings reported clean on a body carrying the very
//! number the corpus registered. [`digit_skeleton`] closes that, on the same
//! terms `soul_policy`'s phone shape uses and deliberately not by NFKC — see
//! the note beside `KnownIdentifiers::add_account` in `redactor.rs`.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// The `≥8` in "no eight-scalar run of third-party prose may leave the box".
pub const DEFAULT_MIN_NGRAM: usize = 8;

/// Normalize to NFC so composed and decomposed spellings compare equal.
pub fn normalize(text: &str) -> String {
    text.nfc().collect()
}

/// Seven digits, the same floor `soul_policy`'s `MIN_PHONE_DIGITS` draws: the
/// shortest thing that is plausibly a number somebody can be reached on rather
/// than a date, a count or a port.
const MIN_DIGITS_FOR_A_NUMBER: usize = 7;

/// What separates one digit group from the next in the skeleton below, and
/// what a candidate's prose collapses to. Any scalar outside the digits and
/// their separators would do; NUL is used because no fixture contains one, so
/// a digit needle can never accidentally span it.
const BREAK: char = '\u{0}';

/// A digit of a number, in either width a keyboard produces.
///
/// U+FF10–U+FF19 are what a Chinese IME in fullwidth mode gives, and
/// `１３８００１３８０００` is the same number as `13800138000` to every reader
/// and every phone.
fn is_phone_digit(c: char) -> bool {
    c.is_ascii_digit() || ('\u{FF10}'..='\u{FF19}').contains(&c)
}

/// The characters a written-out number is grouped by. Kept in step with
/// `soul_policy::redactor::is_group_separator`, including the exclusion of the
/// comma: a comma groups digits (`45,000`) but is also how a Chinese sentence
/// separates its clauses, so joining across one would let a budget and a room
/// number add up to somebody's phone number.
fn is_group_separator(c: char) -> bool {
    matches!(
        c,
        ' ' | '\u{3000}'
            | '.'
            | '-'
            | '\u{2010}'
            | '\u{2011}'
            | '\u{2012}'
            | '\u{2013}'
            | '\u{2014}'
            | '\u{2212}'
            | '\u{FF0D}'
    )
}

/// The digits of `text`, with every grouping the number could have been
/// written with removed and everything else collapsed to a [`BREAK`].
///
/// `138 0013 8000`, `138–0013–8000` and `１３８００１３８０００` all skeletonize
/// to `13800138000`, so one registered spelling covers the others. What is
/// deliberately *not* joined is what the redactor does not join either: two
/// separators in a row, a comma, or any prose between the groups. That is what
/// keeps `下午 3 点，第 2 会议室，预算 45000` — seven digits in three groups
/// that never touch — from skeletonizing into a phone number.
fn digit_skeleton(text: &str) -> String {
    let scalars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(scalars.len() + 1);

    for (index, &c) in scalars.iter().enumerate() {
        if is_phone_digit(c) {
            let ascii = char::from_digit(
                u32::from(c)
                    - if c.is_ascii_digit() {
                        u32::from('0')
                    } else {
                        0xFF10
                    },
                10,
            )
            .expect("a phone digit is 0..=9");
            out.push(ascii);
            continue;
        }

        let joins_two_groups = is_group_separator(c)
            && out.ends_with(|previous: char| previous.is_ascii_digit())
            && scalars.get(index + 1).copied().is_some_and(is_phone_digit);
        if joins_two_groups {
            continue;
        }

        if !out.ends_with(BREAK) {
            out.push(BREAK);
        }
    }

    out
}

/// The digit needle for a registered identifier, if it is a number at all.
///
/// A whole identifier that skeletonizes to one unbroken run of at least
/// [`MIN_DIGITS_FOR_A_NUMBER`] digits is a number, and is matched by its
/// digits. Everything else — a name, a handle, an address, a short room
/// number — keeps being matched as the literal string it is, so `@wang_xiao2`
/// cannot start matching anything with a `2` in it.
fn number_needle(text: &str) -> Option<String> {
    let skeleton = digit_skeleton(text);
    let digits = skeleton.trim_matches(BREAK);
    (digits.len() >= MIN_DIGITS_FOR_A_NUMBER && digits.chars().all(|c| c.is_ascii_digit()))
        .then(|| digits.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeakageKind {
    /// A long enough run of third-party prose.
    ThirdPartyNgram,
    /// A name, handle, phone number or address, at any length.
    KnownIdentifier,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LeakageFinding {
    pub kind: LeakageKind,
    /// Which corpus entry matched.
    pub source_id: String,
    /// The offending text, already normalized.
    pub matched: String,
}

/// One third-party body or known identifier, as stored in a fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusEntry {
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// What a fixture expects the checker to say about one candidate string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeakageCase {
    pub id: String,
    pub text: String,
    /// `true` when the candidate must be reported as leaking.
    pub leaks: bool,
    #[serde(default)]
    pub expect_kinds: Vec<LeakageKind>,
    #[serde(default)]
    pub note: Option<String>,
}

/// `fixtures/leakage/third_party_unicode.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeakageFixture {
    #[serde(default)]
    pub note: Option<String>,
    pub third_party_bodies: Vec<CorpusEntry>,
    pub known_identifiers: Vec<CorpusEntry>,
    pub cases: Vec<LeakageCase>,
}

#[derive(Debug, Clone)]
struct Body {
    id: String,
    scalars: Vec<char>,
}

#[derive(Debug, Clone)]
struct Identifier {
    id: String,
    /// The registered spelling, normalized.
    text: String,
    /// Its digits, when the identifier is a number. See [`number_needle`].
    number: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LeakageChecker {
    bodies: Vec<Body>,
    identifiers: Vec<Identifier>,
    min_ngram: usize,
}

impl Default for LeakageChecker {
    fn default() -> Self {
        LeakageChecker::new()
    }
}

impl LeakageChecker {
    pub fn new() -> Self {
        LeakageChecker {
            bodies: Vec::new(),
            identifiers: Vec::new(),
            min_ngram: DEFAULT_MIN_NGRAM,
        }
    }

    /// Lower the threshold to catch short replies that the `≥8` rule misses.
    pub fn with_min_ngram(mut self, min_ngram: usize) -> Self {
        assert!(
            min_ngram >= 1,
            "an n-gram threshold of zero matches everything"
        );
        self.min_ngram = min_ngram;
        self
    }

    pub fn min_ngram(&self) -> usize {
        self.min_ngram
    }

    pub fn add_third_party_body(&mut self, id: impl Into<String>, text: &str) -> &mut Self {
        self.bodies.push(Body {
            id: id.into(),
            scalars: normalize(text).chars().collect(),
        });
        self
    }

    pub fn add_known_identifier(&mut self, id: impl Into<String>, text: &str) -> &mut Self {
        let normalized = normalize(text);
        if !normalized.is_empty() {
            self.identifiers.push(Identifier {
                id: id.into(),
                number: number_needle(&normalized),
                text: normalized,
            });
        }
        self
    }

    /// Build a checker from a fixture's corpus. The fixture's `cases` are the
    /// test expectations and are not loaded into the corpus.
    pub fn from_fixture(fixture: &LeakageFixture) -> Self {
        let mut checker = LeakageChecker::new();
        for entry in &fixture.third_party_bodies {
            checker.add_third_party_body(entry.id.clone(), &entry.text);
        }
        for entry in &fixture.known_identifiers {
            checker.add_known_identifier(entry.id.clone(), &entry.text);
        }
        checker
    }

    /// Every leak found in `candidate`, sorted and deduplicated.
    pub fn inspect(&self, candidate: &str) -> Vec<LeakageFinding> {
        let haystack = normalize(candidate);
        let mut findings = BTreeSet::new();

        // Built once, and only when some identifier is a number to look for.
        let skeleton = self
            .identifiers
            .iter()
            .any(|entry| entry.number.is_some())
            .then(|| digit_skeleton(&haystack));

        for entry in &self.identifiers {
            // The registered spelling first, so an identifier that matches as
            // written is reported as written and a number is reported once.
            if haystack.contains(&entry.text) {
                findings.insert(LeakageFinding {
                    kind: LeakageKind::KnownIdentifier,
                    source_id: entry.id.clone(),
                    matched: entry.text.clone(),
                });
                continue;
            }

            let Some((number, skeleton)) = entry.number.as_ref().zip(skeleton.as_ref()) else {
                continue;
            };
            if skeleton.contains(number) {
                findings.insert(LeakageFinding {
                    kind: LeakageKind::KnownIdentifier,
                    source_id: entry.id.clone(),
                    matched: number.clone(),
                });
            }
        }

        for body in &self.bodies {
            if body.scalars.len() < self.min_ngram {
                continue;
            }
            for window in body.scalars.windows(self.min_ngram) {
                let run: String = window.iter().collect();
                if haystack.contains(&run) {
                    findings.insert(LeakageFinding {
                        kind: LeakageKind::ThirdPartyNgram,
                        source_id: body.id.clone(),
                        matched: run,
                    });
                    break;
                }
            }
        }

        findings.into_iter().collect()
    }

    pub fn is_clean(&self, candidate: &str) -> bool {
        self.inspect(candidate).is_empty()
    }

    /// Panics with the offending runs listed, so a failure says what leaked.
    pub fn assert_clean(&self, context: &str, candidate: &str) {
        let findings = self.inspect(candidate);
        assert!(
            findings.is_empty(),
            "{context} leaked third-party content: {findings:#?}",
        );
    }
}
