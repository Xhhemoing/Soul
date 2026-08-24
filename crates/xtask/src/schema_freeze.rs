//! `schema-freeze`: pins the frozen contracts by digest.
//!
//! DECISIONS puts the eleven schema documents behind an approval gate. A lock
//! file makes an unreviewed edit show up as a CI failure rather than as a
//! surprise three work packages later.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SCHEMA_DIR: &str = "docs/schemas";
pub const LOCK_FILE: &str = "docs/schemas/schemas.lock.json";
/// The frozen set is exactly eleven documents; a twelfth needs approval too.
pub const EXPECTED_SCHEMA_COUNT: usize = 11;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaLock {
    /// Why this file exists, for whoever hits the failure first.
    pub note: String,
    pub algorithm: String,
    /// File name relative to `docs/schemas/` mapped to a lowercase hex digest.
    pub schemas: BTreeMap<String, String>,
}

pub const LOCK_NOTE: &str = "SHA-256 of every document in docs/schemas/. Regenerate with `cargo run -p xtask -- schema-freeze --write` and only in a change that has approval to touch the frozen contracts. `--check` runs in CI.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockDrift {
    Added(String),
    Removed(String),
    Changed {
        file: String,
        locked: String,
        actual: String,
    },
}

impl fmt::Display for LockDrift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LockDrift::Added(file) => write!(f, "{file} is present but not in the lock"),
            LockDrift::Removed(file) => write!(f, "{file} is in the lock but missing from disk"),
            LockDrift::Changed {
                file,
                locked,
                actual,
            } => write!(f, "{file} changed: locked {locked}, on disk {actual}"),
        }
    }
}

/// Hex SHA-256 of the file's bytes, so line endings are part of the identity.
pub fn digest_file(path: &Path) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading {} to hash", path.display()))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

/// Digest every `*.json` document in `docs/schemas/`, excluding the lock file.
pub fn digest_schema_dir(repo_root: &Path) -> Result<BTreeMap<String, String>> {
    let dir = repo_root.join(SCHEMA_DIR);
    let lock_name = Path::new(LOCK_FILE)
        .file_name()
        .expect("LOCK_FILE has a file name");

    let mut digests = BTreeMap::new();
    let entries = std::fs::read_dir(&dir).with_context(|| format!("listing {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading an entry of {}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = path
            .file_name()
            .expect("a file has a name")
            .to_string_lossy()
            .into_owned();
        if path.file_name() == Some(lock_name) {
            continue;
        }
        digests.insert(name, digest_file(&path)?);
    }
    Ok(digests)
}

pub fn read_lock(repo_root: &Path) -> Result<SchemaLock> {
    let path = repo_root.join(LOCK_FILE);
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// Rewrite the lock from what is on disk. Requires approval to touch the
/// frozen contracts; the audit cannot tell an approved edit from a slip.
pub fn write_lock(repo_root: &Path) -> Result<PathBuf> {
    let schemas = digest_schema_dir(repo_root)?;
    if schemas.len() != EXPECTED_SCHEMA_COUNT {
        bail!(
            "expected {EXPECTED_SCHEMA_COUNT} schema documents, found {}: {:?}",
            schemas.len(),
            schemas.keys().collect::<Vec<_>>(),
        );
    }
    let lock = SchemaLock {
        note: LOCK_NOTE.to_owned(),
        algorithm: "sha256".to_owned(),
        schemas,
    };
    let path = repo_root.join(LOCK_FILE);
    let mut text = serde_json::to_string_pretty(&lock).context("serializing the lock")?;
    text.push('\n');
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

/// Compare the lock against the documents on disk.
pub fn check_lock(repo_root: &Path) -> Result<Vec<LockDrift>> {
    let lock = read_lock(repo_root)?;
    let actual = digest_schema_dir(repo_root)?;
    let mut drift = Vec::new();

    if actual.len() != EXPECTED_SCHEMA_COUNT {
        drift.push(LockDrift::Changed {
            file: SCHEMA_DIR.to_owned(),
            locked: format!("{EXPECTED_SCHEMA_COUNT} documents"),
            actual: format!("{} documents", actual.len()),
        });
    }

    for (file, digest) in &actual {
        match lock.schemas.get(file) {
            None => drift.push(LockDrift::Added(file.clone())),
            Some(locked) if locked != digest => drift.push(LockDrift::Changed {
                file: file.clone(),
                locked: locked.clone(),
                actual: digest.clone(),
            }),
            Some(_) => {}
        }
    }
    for file in lock.schemas.keys() {
        if !actual.contains_key(file) {
            drift.push(LockDrift::Removed(file.clone()));
        }
    }

    Ok(drift)
}
