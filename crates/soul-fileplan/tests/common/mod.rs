//! Building the sample tree, and taking a snapshot strong enough to prove
//! nothing touched it.
//!
//! The tree is described by `fixtures/fileplan/sample_tree.json` and created
//! here, in a temporary directory, for the reasons that fixture's README
//! gives: a repository cannot carry a symlink or a file called
//! `; rm -rf ~ .txt` without becoming awkward to check out, and a tree that
//! is created per test cannot be left dirty by the previous one.

// Each test binary uses a different subset of these helpers.
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use soul_fileplan::AuthorizedRoots;
use soul_policy::hitl::PlanHash;
use walkdir::WalkDir;

/// `fixtures/fileplan/sample_tree.json`.
#[derive(Debug, Deserialize)]
pub struct SampleTree {
    pub authorized_root: String,
    pub entries: Vec<SampleEntry>,
}

#[derive(Debug, Deserialize)]
pub struct SampleEntry {
    pub relative_path: String,
    pub kind: String,
    #[serde(default)]
    pub content_utf8: Option<String>,
}

pub fn sample_tree() -> SampleTree {
    soul_testkit::fixtures::read_json("fileplan/sample_tree.json")
        .expect("the fileplan sample tree fixture parses")
}

/// An authorized directory and an unauthorized one, side by side.
///
/// Siblings on purpose: sharing a parent is what makes `A/../B` and a link
/// from `A` into `B` easy to write, which are the two ways out of a root that
/// a prefix check alone would miss.
#[derive(Debug)]
pub struct Workspace {
    pub parent: tempfile::TempDir,
    pub authorized: PathBuf,
    pub outside: PathBuf,
}

impl Workspace {
    /// The roots for a scan: the authorized directory, and nothing else.
    pub fn roots(&self) -> AuthorizedRoots {
        roots(&[&self.authorized])
    }
}

pub fn workspace() -> Workspace {
    let parent = tempfile::tempdir().expect("a temporary parent directory");
    let authorized = parent.path().join("a");
    let outside = parent.path().join("b");
    std::fs::create_dir_all(&authorized).expect("create the authorized directory");
    std::fs::create_dir_all(&outside).expect("create the unauthorized directory");
    Workspace {
        parent,
        authorized,
        outside,
    }
}

pub fn roots(paths: &[&Path]) -> AuthorizedRoots {
    let owned: Vec<PathBuf> = paths.iter().map(|path| path.to_path_buf()).collect();
    AuthorizedRoots::canonicalized(&owned).expect("the roots exist and are directories")
}

/// Create the fixture tree under `root`. Returns how many files it made.
pub fn build_sample_tree(root: &Path) -> usize {
    let tree = sample_tree();
    let mut files = 0usize;
    for entry in &tree.entries {
        let path = root.join(&entry.relative_path);
        match entry.kind.as_str() {
            "directory" => std::fs::create_dir_all(&path).expect("create a fixture directory"),
            "file" => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).expect("create a fixture parent");
                }
                let contents = entry.content_utf8.clone().unwrap_or_default();
                std::fs::write(&path, contents).expect("create a fixture file");
                files += 1;
            }
            other => panic!("the fixture describes a `{other}` and this builder makes two kinds"),
        }
    }
    files
}

pub fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create a parent directory");
    }
    std::fs::write(path, contents).expect("write a test file");
}

/// One entry of a tree snapshot.
///
/// Path, kind, byte length and a digest of the contents. Deliberately no
/// timestamp: reading a directory is allowed to touch atime, and a mtime
/// comparison would fail for reasons that have nothing to do with anything
/// being written. A changed byte changes the digest, which is the evidence
/// that matters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Shot {
    pub rel_path: String,
    pub kind: &'static str,
    pub bytes: u64,
    pub digest: String,
}

/// Every file and directory under `root`, recursively, with contents hashed.
///
/// Links are recorded as links and never followed, so a snapshot of an
/// authorized tree does not quietly include whatever a link points at.
pub fn snapshot(root: &Path) -> Vec<Shot> {
    let mut shots: Vec<Shot> = Vec::new();
    for step in WalkDir::new(root).follow_links(false).min_depth(1) {
        let step = step.expect("read the tree");
        let rel_path = encode(
            step.path()
                .strip_prefix(root)
                .expect("walkdir stays under the root it was given"),
        );
        let metadata = step
            .path()
            .symlink_metadata()
            .expect("stat an entry without following it");

        let shot = if metadata.is_symlink() {
            let target = std::fs::read_link(step.path()).expect("read a link");
            Shot {
                rel_path,
                kind: "symlink",
                bytes: 0,
                digest: digest(target.to_string_lossy().as_bytes()),
            }
        } else if metadata.is_dir() {
            Shot {
                rel_path,
                kind: "dir",
                bytes: 0,
                digest: digest(&[]),
            }
        } else {
            let contents = std::fs::read(step.path()).expect("read a file");
            Shot {
                rel_path,
                kind: "file",
                bytes: metadata.len(),
                digest: digest(&contents),
            }
        };
        shots.push(shot);
    }
    shots.sort();
    shots
}

/// SHA-256 of some bytes, by way of the hash this workspace already has.
///
/// `PlanHash::of` is the SHA-256 in `soul-policy`; feeding it the bytes as a
/// JSON value is a deterministic encoding of them, which is all a snapshot
/// digest needs. It saves this crate a dependency it would only ever use in
/// a test.
pub fn digest(bytes: &[u8]) -> String {
    PlanHash::of(&serde_json::json!({ "bytes": bytes }))
        .as_str()
        .to_owned()
}

/// A relative path as one string, components joined with `/`.
pub fn encode(rel_path: &Path) -> String {
    rel_path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join("/")
}

/// The relative paths a snapshot covers.
pub fn paths_in(shots: &[Shot]) -> BTreeSet<String> {
    shots.iter().map(|shot| shot.rel_path.clone()).collect()
}
