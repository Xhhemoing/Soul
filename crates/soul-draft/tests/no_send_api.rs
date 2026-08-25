//! The crate that drafts has no way to put a draft anywhere.
//!
//! Three layers guard "never sends" and this is the second: the dependency
//! graph has no HTTP client (`cargo deny` and `xtask e0-audit`), the wire tests
//! in `soulcore` prove only generation requests leave, and this reads the
//! sources back so that an API nobody calls yet still fails. A method named
//! `send` on a draft type is a red line whether or not a caller exists.
//!
//! The file list is built with `read_dir` at run time rather than written down
//! here: a new module must not be able to hide by not being on a list.

use std::path::{Path, PathBuf};

/// Spellings of "this thing puts text somewhere else".
///
/// `fn send` rather than `send`, because a doc comment is allowed to say the
/// product does not send anything — that sentence is the promise, not a
/// breach of it.
const FORBIDDEN: &[&str] = &[
    "fn send",
    "fn submit",
    "smtp",
    "sendmail",
    "deliver",
    "transmit",
];

/// Proof the scanner read this crate and not an empty directory.
const EXPECTED: &[&str] = &["fn template_draft", "fn summarize", "struct DraftOutcome"];

fn source_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `src/`, recursively.
fn sources(dir: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("the crate has a src directory") {
        let path = entry.expect("a readable directory entry").path();
        if path.is_dir() {
            found.extend(sources(&path));
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a readable source file");
        found.push((path, text));
    }
    found.sort_by(|left, right| left.0.cmp(&right.0));
    found
}

fn hits(text: &str) -> Vec<&'static str> {
    let lowered = text.to_lowercase();
    FORBIDDEN
        .iter()
        .filter(|needle| lowered.contains(**needle))
        .copied()
        .collect()
}

#[test]
fn the_crate_has_no_send_surface() {
    let sources = sources(&source_dir());
    assert!(
        sources.len() >= 5,
        "only {} source files were scanned; the walk is wrong",
        sources.len(),
    );

    for (path, text) in &sources {
        let found = hits(text);
        assert!(
            found.is_empty(),
            "{} names {found:?}; this crate drafts and nothing else",
            path.display(),
        );
    }
}

/// The reverse control: the scanner read the right crate.
#[test]
fn the_scanner_sees_the_draft_functions() {
    let all: String = sources(&source_dir())
        .into_iter()
        .map(|(_, text)| text)
        .collect::<Vec<_>>()
        .join("\n");

    for expected in EXPECTED {
        assert!(
            all.contains(expected),
            "`{expected}` was not found, so the scan was pointed at the wrong files",
        );
    }
}

/// The other control: the scanner is capable of failing.
#[test]
fn the_scanner_catches_a_synthetic_send() {
    let synthetic = "impl DraftOutcome {\n    pub fn send(&self) -> bool { true }\n}\n";
    assert_eq!(hits(synthetic), vec!["fn send"]);

    let over_smtp = "// hands the draft to an SMTP relay\n";
    assert_eq!(hits(over_smtp), vec!["smtp"]);
}
