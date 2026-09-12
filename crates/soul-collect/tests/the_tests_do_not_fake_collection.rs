//! The acceptance tests are only worth their assertions if the rows they count
//! came from the collector.
//!
//! An event appended by a test would satisfy "at least one event during the on
//! period" without a collector existing at all, and a test that seals its own
//! body would satisfy "the application name is not on disk in the clear"
//! without the collector ever sealing anything. So the rule is that nothing in
//! this directory writes to the store, and the rule is checked rather than
//! remembered: every test file is read back and searched for the write half of
//! the storage boundary.
//!
//! Reads are what tests are for and are not restricted.

use std::path::{Path, PathBuf};

/// The write half of `soul-store-api`, as it appears at a call site.
const WRITES: &[&str] = &[
    ".append_event(",
    ".put_memory(",
    ".put_event(",
    ".put_evidence(",
    ".put_inference(",
    ".put_contact(",
    ".put_relationship(",
    ".put_profile(",
    ".seal(",
    ".append_audit(",
    ".execute_forget(",
];

fn test_files() -> Vec<(PathBuf, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let this_file = Path::new(file!())
        .file_name()
        .expect("this file has a name")
        .to_owned();

    let mut files = Vec::new();
    let mut pending = vec![dir];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read the tests directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "rs")
                && path.file_name() != Some(this_file.as_os_str())
            {
                let text = std::fs::read_to_string(&path).expect("read a test file");
                files.push((path, text));
            }
        }
    }
    assert!(
        files.len() >= 3,
        "the scan found {} test files, which is fewer than this crate has",
        files.len(),
    );
    files
}

#[test]
fn no_test_in_this_crate_writes_to_the_store_itself() {
    let mut found = Vec::new();
    for (path, text) in test_files() {
        for (number, line) in text.lines().enumerate() {
            for write in WRITES {
                if line.contains(write) {
                    found.push(format!("{}:{}: {write}", path.display(), number + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "an acceptance test that writes its own rows proves nothing about the collector: {found:#?}",
    );
}

#[test]
fn the_scan_recognises_a_write_when_it_sees_one() {
    // The control for the test above, which would otherwise pass just as
    // happily against a search that matches nothing.
    let forged = r#"    store.append_event(event).expect("a hand-written foreground event");"#;
    assert!(WRITES.iter().any(|write| forged.contains(write)));
    assert!(!WRITES
        .iter()
        .any(|write| "    let bytes = store.open(sealed).expect(\"read\");".contains(write)));
}
