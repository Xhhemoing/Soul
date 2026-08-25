//! Loads the corpora under `fixtures/` at the repository root.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::leakage::LeakageFixture;

/// The repository root, derived from this crate's manifest location.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/soul-testkit sits two levels below the repository root")
        .to_path_buf()
}

pub fn fixtures_dir() -> PathBuf {
    repo_root().join("fixtures")
}

/// Absolute path of a fixture named relative to `fixtures/`.
pub fn path(relative: &str) -> PathBuf {
    let mut full = fixtures_dir();
    for segment in relative.split('/') {
        full.push(segment);
    }
    full
}

pub fn read_text(relative: &str) -> Result<String> {
    let full = path(relative);
    std::fs::read_to_string(&full).with_context(|| format!("reading fixture {}", full.display()))
}

pub fn read_json<T: DeserializeOwned>(relative: &str) -> Result<T> {
    let text = read_text(relative)?;
    serde_json::from_str(&text).with_context(|| format!("parsing fixture {relative}"))
}

/// One JSON value per non-empty line, as `soul-import-v1` is shaped.
pub fn read_jsonl(relative: &str) -> Result<Vec<Value>> {
    let text = read_text(relative)?;
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .with_context(|| format!("{relative} line {} is not JSON", index + 1))?;
        out.push(value);
    }
    Ok(out)
}

/// Lines of `fixtures/import/soul-import-v1/*.jsonl` exactly as written,
/// including any that are not valid JSON. Import tests need the raw bytes to
/// prove a malformed line fails readably instead of being skipped.
pub fn read_lines(relative: &str) -> Result<Vec<String>> {
    Ok(read_text(relative)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect())
}

pub fn leakage_fixture() -> Result<LeakageFixture> {
    read_json("leakage/third_party_unicode.json")
}

/// The single source of truth for forbidden diagnostic vocabulary.
/// Blank lines and `#` comments are stripped.
pub fn denylist_terms() -> Result<Vec<String>> {
    Ok(read_text("denylist/diagnostic_terms.txt")?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect())
}
