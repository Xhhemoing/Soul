//! Soul is not a medical product, and the vocabulary check says so at runtime.
//!
//! `xtask denylist-audit` already keeps the forbidden words out of the crate
//! sources, but that only covers text a developer typed. Anything a model
//! returns, or a template assembles at runtime, has to be checked when it is
//! produced. Both halves read the same file — `fixtures/denylist/diagnostic_terms.txt`
//! is the single source of truth, embedded here at compile time so the check
//! works in a packaged build with no fixtures directory next to it.
//!
//! Matching follows `crates/xtask/src/denylist.rs`: ASCII terms match on word
//! boundaries so `underscore` is not a hit for the rating word inside it, and
//! non-ASCII terms match as substrings because there are no word boundaries to
//! lean on.

use std::sync::OnceLock;

/// The vocabulary, verbatim. Same bytes `xtask` reads.
pub const DENYLIST_SOURCE: &str = include_str!("../../../fixtures/denylist/diagnostic_terms.txt");

/// The sentence every inference-bearing surface has to carry.
pub const WORKING_HYPOTHESIS_NOTICE: &str = "工作假设，非临床结论";

fn terms() -> &'static [String] {
    static TERMS: OnceLock<Vec<String>> = OnceLock::new();
    TERMS.get_or_init(|| {
        DENYLIST_SOURCE
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_lowercase)
            .collect()
    })
}

/// How many terms the embedded list holds. Used by a test to catch an empty
/// or truncated include.
pub fn term_count() -> usize {
    terms().len()
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("output carries `{term}`, which this product must not say")]
pub struct NonClinicalViolation {
    pub term: String,
}

/// Every denied term present in `text`, lowercased.
pub fn denied_terms_in(text: &str) -> Vec<String> {
    let haystack = text.to_lowercase();
    let mut hits: Vec<String> = terms()
        .iter()
        .filter(|term| contains_term(&haystack, term))
        .cloned()
        .collect();
    hits.sort();
    hits.dedup();
    hits
}

/// Refuse text that makes a medical claim.
///
/// Named as an assertion because that is how callers should treat it: a
/// summary that trips this is a bug in the generator, not something to filter
/// and ship anyway.
pub fn assert_non_clinical(text: &str) -> Result<(), NonClinicalViolation> {
    match denied_terms_in(text).into_iter().next() {
        Some(term) => Err(NonClinicalViolation { term }),
        None => Ok(()),
    }
}

pub fn is_non_clinical(text: &str) -> bool {
    assert_non_clinical(text).is_ok()
}

fn contains_term(haystack_lowercase: &str, term_lowercase: &str) -> bool {
    if !term_lowercase.is_ascii() {
        return haystack_lowercase.contains(term_lowercase);
    }
    let mut from = 0usize;
    while let Some(offset) = haystack_lowercase[from..].find(term_lowercase) {
        let start = from + offset;
        let end = start + term_lowercase.len();
        let before_ok = haystack_lowercase[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word_char(c));
        let after_ok = haystack_lowercase[end..]
            .chars()
            .next()
            .is_none_or(|c| !is_word_char(c));
        if before_ok && after_ok {
            return true;
        }
        from = start + term_lowercase.len().max(1);
    }
    false
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
