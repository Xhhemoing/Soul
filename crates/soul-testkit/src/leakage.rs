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

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

/// The `≥8` in "no eight-scalar run of third-party prose may leave the box".
pub const DEFAULT_MIN_NGRAM: usize = 8;

/// Normalize to NFC so composed and decomposed spellings compare equal.
pub fn normalize(text: &str) -> String {
    text.nfc().collect()
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
pub struct LeakageChecker {
    bodies: Vec<Body>,
    identifiers: Vec<CorpusEntry>,
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
            self.identifiers.push(CorpusEntry {
                id: id.into(),
                text: normalized,
                note: None,
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

        for entry in &self.identifiers {
            if haystack.contains(&entry.text) {
                findings.insert(LeakageFinding {
                    kind: LeakageKind::KnownIdentifier,
                    source_id: entry.id.clone(),
                    matched: entry.text.clone(),
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
