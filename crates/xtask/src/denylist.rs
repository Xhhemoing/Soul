//! `denylist-audit`: keeps clinical vocabulary and numeric ratings out of the
//! product surface.
//!
//! PRODUCT_LOCK rules out diagnostic claims, and DECISIONS D22 rules out
//! scales and numeric ratings on trait axes. Both are easy to reintroduce by
//! accident — a field called `score`, a string that says "symptom" — so the
//! vocabulary lives in one file, `fixtures/denylist/diagnostic_terms.txt`, and
//! CI reads it.
//!
//! Matching is deliberately word-aware for ASCII so that `underscore` is not a
//! hit for `score`, and plain substring for CJK where there are no word
//! boundaries to lean on.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Path of the single source of truth, relative to the repository root.
pub const DENYLIST_PATH: &str = "fixtures/denylist/diagnostic_terms.txt";

/// Trees that may name the forbidden words: the denylist itself, the audit
/// that implements it, the corpora, tests, and algorithm prototypes whose
/// frozen API types predate this product-surface audit.
pub const EXEMPT_PATH_SEGMENTS: &[&str] = &["fixtures", "tests", "target", "node_modules", ".git"];
pub const EXEMPT_CRATES: &[&str] = &["xtask", "soul-algo-tie", "soul-algo-trait"];

/// Single files that have to say a forbidden word to reach an exempt crate.
///
/// The frozen tie rule's entry point is spelled in one of the denied words, so
/// a product crate that calls it must name it somewhere. Naming it in one
/// adapter file, listed here, is what keeps the audit over everything else in
/// that crate — the stored field names, the statement keys and the sentences a
/// user reads all stay covered. Exempting a whole crate for one call would
/// not.
pub const EXEMPT_FILES: &[&str] = &["crates/soul-graph/src/t4d_adapt.rs"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitContext {
    StringLiteral,
    Identifier,
}

impl fmt::Display for HitContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HitContext::StringLiteral => f.write_str("string literal"),
            HitContext::Identifier => f.write_str("identifier"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DenylistHit {
    pub file: PathBuf,
    pub line: usize,
    pub term: String,
    pub context: HitContext,
    pub found_in: String,
}

impl fmt::Display for DenylistHit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: `{}` in {} `{}`",
            self.file.display(),
            self.line,
            self.term,
            self.context,
            self.found_in,
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DenylistReport {
    pub hits: Vec<DenylistHit>,
    pub terms_loaded: usize,
    pub files_scanned: usize,
}

impl DenylistReport {
    pub fn is_clean(&self) -> bool {
        self.hits.is_empty()
    }
}

impl fmt::Display for DenylistReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "denylist-audit: {} term(s) from {DENYLIST_PATH}, {} file(s) scanned",
            self.terms_loaded, self.files_scanned,
        )?;
        for hit in &self.hits {
            writeln!(f, "  {hit}")?;
        }
        if self.is_clean() {
            write!(f, "  clean")?;
        }
        Ok(())
    }
}

/// Read the denylist, dropping blank lines and `#` comments.
pub fn load_terms(repo_root: &Path) -> Result<Vec<String>> {
    let path = repo_root.join(DENYLIST_PATH);
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("reading the denylist at {}", path.display()))?;
    Ok(parse_terms(&text))
}

pub fn parse_terms(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_lowercase())
        .collect()
}

/// Scan every non-exempt Rust source file under `crates/`.
pub fn audit(repo_root: &Path) -> Result<DenylistReport> {
    let terms = load_terms(repo_root)?;
    let mut report = DenylistReport {
        terms_loaded: terms.len(),
        ..DenylistReport::default()
    };

    let crates_dir = repo_root.join("crates");
    if !crates_dir.is_dir() {
        return Ok(report);
    }

    for entry in walkdir::WalkDir::new(&crates_dir) {
        let entry = entry.with_context(|| format!("walking {}", crates_dir.display()))?;
        let path = entry.path();
        if !entry.file_type().is_file() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if is_exempt(path) {
            continue;
        }
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        report.files_scanned += 1;
        report.hits.extend(scan_source(path, &text, &terms));
    }

    report.hits.sort_by(|a, b| {
        (a.file.clone(), a.line, a.term.clone()).cmp(&(b.file.clone(), b.line, b.term.clone()))
    });
    Ok(report)
}

pub fn is_exempt(path: &Path) -> bool {
    if is_exempt_file(path) {
        return true;
    }
    let mut components = path.components().map(|c| c.as_os_str().to_string_lossy());
    let mut previous_was_crates = false;
    for component in components.by_ref() {
        if EXEMPT_PATH_SEGMENTS.contains(&component.as_ref()) {
            return true;
        }
        if previous_was_crates && EXEMPT_CRATES.contains(&component.as_ref()) {
            return true;
        }
        previous_was_crates = component == "crates";
    }
    false
}

/// Whether this is one of the named adapter files, wherever the repository is
/// checked out.
fn is_exempt_file(path: &Path) -> bool {
    let normalized = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join("/");
    EXEMPT_FILES
        .iter()
        .any(|exempt| normalized == *exempt || normalized.ends_with(&format!("/{exempt}")))
}

/// Find denied vocabulary in one Rust source file.
///
/// Comments are ignored on purpose: this checks what the product says and what
/// its API is called, not what a developer wrote to themselves.
pub fn scan_source(file: &Path, text: &str, terms: &[String]) -> Vec<DenylistHit> {
    let mut hits = Vec::new();
    for piece in tokenize(text) {
        match piece.kind {
            PieceKind::StringLiteral => {
                let haystack = piece.text.to_lowercase();
                for term in terms {
                    if contains_term(&haystack, term) {
                        hits.push(DenylistHit {
                            file: file.to_path_buf(),
                            line: piece.line,
                            term: term.clone(),
                            context: HitContext::StringLiteral,
                            found_in: truncate(&piece.text),
                        });
                    }
                }
            }
            PieceKind::Identifier => {
                let words = split_identifier(&piece.text);
                for term in terms {
                    let matched = if term.is_ascii() {
                        identifier_matches(&words, term)
                    } else {
                        piece.text.to_lowercase().contains(term)
                    };
                    if matched {
                        hits.push(DenylistHit {
                            file: file.to_path_buf(),
                            line: piece.line,
                            term: term.clone(),
                            context: HitContext::Identifier,
                            found_in: piece.text.clone(),
                        });
                    }
                }
            }
        }
    }
    hits
}

/// ASCII terms match on word boundaries; CJK terms match as substrings.
pub fn contains_term(haystack_lowercase: &str, term_lowercase: &str) -> bool {
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

/// `EvidenceBand` and `evidence_band` both become `["evidence", "band"]`.
pub fn split_identifier(identifier: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut previous: Option<char> = None;
    for c in identifier.chars() {
        if c == '_' || c == '-' || c.is_whitespace() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            previous = None;
            continue;
        }
        let boundary = matches!(previous, Some(p) if p.is_lowercase() && c.is_uppercase())
            || matches!(previous, Some(p) if p.is_ascii_digit() != c.is_ascii_digit());
        if boundary && !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
        current.push(c.to_ascii_lowercase());
        previous = Some(c);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

/// A multi-word term must appear as a contiguous run of identifier words.
fn identifier_matches(words: &[String], term: &str) -> bool {
    let needle: Vec<&str> = term
        .split([' ', '-', '_'])
        .filter(|w| !w.is_empty())
        .collect();
    if needle.is_empty() || needle.len() > words.len() {
        return false;
    }
    words
        .windows(needle.len())
        .any(|window| window.iter().zip(&needle).all(|(w, n)| w == n))
}

fn truncate(text: &str) -> String {
    const LIMIT: usize = 80;
    if text.chars().count() <= LIMIT {
        return text.to_owned();
    }
    let head: String = text.chars().take(LIMIT).collect();
    format!("{head}…")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PieceKind {
    StringLiteral,
    Identifier,
}

#[derive(Debug, Clone)]
struct Piece {
    kind: PieceKind,
    text: String,
    line: usize,
}

/// Split Rust source into string literals and identifiers, skipping comments.
///
/// This is a lexer good enough for an audit, not a parser. It understands line
/// and block comments, ordinary and raw strings, and character literals.
fn tokenize(text: &str) -> Vec<Piece> {
    let chars: Vec<char> = text.chars().collect();
    let mut pieces = Vec::new();
    let mut index = 0usize;
    let mut line = 1usize;

    let bump = |c: char, line: &mut usize| {
        if c == '\n' {
            *line += 1;
        }
    };

    while index < chars.len() {
        let c = chars[index];

        if c == '/' && chars.get(index + 1) == Some(&'/') {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }

        if c == '/' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            let mut depth = 1usize;
            while index < chars.len() && depth > 0 {
                if chars[index] == '/' && chars.get(index + 1) == Some(&'*') {
                    depth += 1;
                    index += 2;
                    continue;
                }
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    depth -= 1;
                    index += 2;
                    continue;
                }
                bump(chars[index], &mut line);
                index += 1;
            }
            continue;
        }

        // Raw string: r"…", r#"…"#, br#"…"#
        if c == 'r' || (c == 'b' && chars.get(index + 1) == Some(&'r')) {
            let mut probe = index + if c == 'b' { 2 } else { 1 };
            let mut hashes = 0usize;
            while chars.get(probe) == Some(&'#') {
                hashes += 1;
                probe += 1;
            }
            if chars.get(probe) == Some(&'"') {
                let start_line = line;
                probe += 1;
                let mut body = String::new();
                loop {
                    if probe >= chars.len() {
                        break;
                    }
                    if chars[probe] == '"' {
                        let closing = (1..=hashes).all(|k| chars.get(probe + k) == Some(&'#'));
                        if closing {
                            probe += hashes + 1;
                            break;
                        }
                    }
                    bump(chars[probe], &mut line);
                    body.push(chars[probe]);
                    probe += 1;
                }
                pieces.push(Piece {
                    kind: PieceKind::StringLiteral,
                    text: body,
                    line: start_line,
                });
                index = probe;
                continue;
            }
        }

        if c == '"' {
            let start_line = line;
            index += 1;
            let mut body = String::new();
            while index < chars.len() && chars[index] != '"' {
                if chars[index] == '\\' {
                    index += 1;
                    if index < chars.len() {
                        bump(chars[index], &mut line);
                        body.push(chars[index]);
                        index += 1;
                    }
                    continue;
                }
                bump(chars[index], &mut line);
                body.push(chars[index]);
                index += 1;
            }
            index += 1;
            pieces.push(Piece {
                kind: PieceKind::StringLiteral,
                text: body,
                line: start_line,
            });
            continue;
        }

        // Character literal, distinguished from a lifetime by the closing quote.
        if c == '\'' {
            let mut probe = index + 1;
            if chars.get(probe) == Some(&'\\') {
                probe += 2;
            } else {
                probe += 1;
            }
            if chars.get(probe) == Some(&'\'') {
                index = probe + 1;
                continue;
            }
            index += 1;
            continue;
        }

        if c.is_alphabetic() || c == '_' || !c.is_ascii() {
            let start_line = line;
            let mut word = String::new();
            while index < chars.len() {
                let ch = chars[index];
                if ch.is_alphanumeric() || ch == '_' || !ch.is_ascii() {
                    word.push(ch);
                    index += 1;
                } else {
                    break;
                }
            }
            pieces.push(Piece {
                kind: PieceKind::Identifier,
                text: word,
                line: start_line,
            });
            continue;
        }

        bump(c, &mut line);
        index += 1;
    }

    pieces
}
