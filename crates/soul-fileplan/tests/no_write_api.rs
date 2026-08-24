//! There is no write API in this crate, and this is where that stops being a
//! promise and starts being a check.
//!
//! Three layers, because each catches something the others do not:
//!
//! 1. **Source.** Every `src/*.rs` file is read back and searched for the ways
//!    Rust changes a file: creating, truncating, renaming, removing, linking,
//!    setting permissions, spawning a process. A call that is never reached by
//!    a test is still found by a search.
//! 2. **Dependencies.** The manifest is read and checked for the crates that
//!    could write on this one's behalf — `soul-store` above all, because
//!    turning a scan into stored file metadata is the *other* thing
//!    PRODUCT_LOCK defers to v0.1.1.
//! 3. **Runtime.** A preview runs against a real tree and the tree is compared
//!    entry for entry, byte for byte, timestamp for timestamp, afterwards.
//!
//! Layer 1 is only worth having if the search would find a write, so
//! `the_search_recognises_a_write_when_it_sees_one` feeds it lines that
//! contain one. Without that control this file would pass just as happily
//! against a list of needles that match nothing.

mod common;

use std::path::{Path, PathBuf};

use common::{walk, Tree};

use soul_fileplan::{Authorization, ScanLimits};
use soul_policy::hitl::{RequestOrigin, TokenIssuer};

/// The ways a Rust program changes something on disk, or hands the job to
/// something that will.
const WRITES: &[&str] = &[
    "fs::write",
    "fs::create_dir",
    "create_dir_all",
    "fs::remove_file",
    "fs::remove_dir",
    "fs::rename",
    "fs::copy",
    "fs::hard_link",
    "fs::set_permissions",
    "fs::soft_link",
    // Spelled with the paren so that `fs::symlink_metadata`, which is how
    // every link in this crate is *recognised*, does not match.
    "fs::symlink(",
    "symlink_file(",
    "symlink_dir(",
    "File::create",
    "File::options",
    "OpenOptions",
    "set_len(",
    "write_all",
    "BufWriter",
    "std::io::Write",
    "io::Write",
    "process::Command",
    "set_modified",
    "set_times",
    // The store's write half. This crate does not depend on it; if it ever
    // did, a scan could become collected file metadata, which is v0.1.1.
    "append_event",
    "append_audit",
    "put_memory",
    "put_event",
    ".seal(",
    "execute_forget",
];

fn source_files() -> Vec<(PathBuf, String)> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    let mut pending = vec![directory];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).expect("read the source directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().is_some_and(|extension| extension == "rs") {
                let text = std::fs::read_to_string(&path).expect("read a source file");
                files.push((path, text));
            }
        }
    }
    assert!(
        files.len() >= 8,
        "the scan found {} source files, which is fewer than this crate has",
        files.len(),
    );
    files
}

#[test]
fn no_source_file_in_this_crate_can_change_anything_on_disk() {
    let mut found = Vec::new();
    for (path, text) in source_files() {
        for (number, line) in text.lines().enumerate() {
            // Documentation has to be able to name what it promises not to do.
            let code = line.split("//").next().unwrap_or(line);
            for write in WRITES {
                if code.contains(write) {
                    found.push(format!("{}:{}: {write}", path.display(), number + 1));
                }
            }
        }
    }
    assert!(
        found.is_empty(),
        "v0.1 has no file-write path, and AC-27 is v0.1.1: {found:#?}",
    );
}

#[test]
fn the_search_recognises_a_write_when_it_sees_one() {
    for forged in [
        r#"    std::fs::rename(&from, &to).expect("carry out the plan");"#,
        r#"    std::fs::create_dir_all(&destination)?;"#,
        r#"    let mut file = File::create(path)?;"#,
        r#"    store.append_event(event)?;"#,
        r#"    std::process::Command::new("cmd.exe").spawn()?;"#,
    ] {
        assert!(
            WRITES.iter().any(|write| forged.contains(write)),
            "the search would have missed `{forged}`",
        );
    }

    // And does not fire on the read half, which is all this crate does.
    for read in [
        r#"    let metadata = std::fs::symlink_metadata(entry.path())?;"#,
        r#"    for entry in std::fs::read_dir(&directory)? {"#,
        r#"    let canonical = std::fs::canonicalize(path)?;"#,
    ] {
        assert!(
            !WRITES.iter().any(|write| read.contains(write)),
            "the search fires on a read: `{read}`",
        );
    }
}

/// The manifest, because a dependency can write on a crate's behalf.
#[test]
fn this_crate_depends_on_nothing_that_could_write_for_it() {
    const MANIFEST: &str = include_str!("../Cargo.toml");

    // Comments stripped first: the manifest explains why the storage crate is
    // absent, and naming it in order to rule it out is not depending on it.
    let normal: String = MANIFEST
        .split("[dev-dependencies]")
        .next()
        .expect("the manifest has a dependency section")
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in ["soul-store", "soul-collect", "soul-import", "tempfile"] {
        assert!(
            !normal.contains(forbidden),
            "`{forbidden}` is a normal dependency of soul-fileplan",
        );
    }
    assert!(
        normal.contains("soul-policy"),
        "the permission crate should still be here",
    );
}

/// The runtime half. A preview over a real tree, and the tree afterwards.
#[test]
fn a_preview_leaves_the_tree_exactly_as_it_found_it() {
    let tree = Tree::build();
    let mut authorization = Authorization::new();
    authorization
        .authorize(&tree.alpha())
        .expect("authorize Alpha");
    let mut issuer = TokenIssuer::new();

    let before = walk(tree.base());
    for _ in 0..5 {
        soul_fileplan::preview(
            &authorization,
            &mut issuer,
            &tree.alpha(),
            RequestOrigin::User,
            ScanLimits::default(),
            1_787_529_600_000,
        )
        .expect("a preview");
    }
    let after = walk(tree.base());

    assert_eq!(before, after);
    assert!(
        !before.is_empty(),
        "an empty tree would make the comparison above vacuous",
    );
}

/// The public API has no method that acts on a plan. Reading the surface back
/// is cruder than a type check, and it is the check that keeps working when
/// somebody adds a free function next year.
#[test]
fn nothing_in_the_public_surface_offers_to_carry_a_plan_out() {
    let acting = [
        "fn execute",
        "fn apply",
        "fn perform",
        "fn commit",
        "fn undo",
    ];
    for (path, text) in source_files() {
        for verb in acting {
            assert!(
                !text.contains(verb),
                "{} declares `{verb}`, and v0.1 carries nothing out",
                path.display(),
            );
        }
    }
    // `refuse_execution` is the only thing named after execution, and it
    // returns a refusal rather than a `Result` with a success side.
    let execute =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/execute.rs"))
            .expect("read the execution module");
    assert!(execute.contains("pub fn refuse_execution"));
    assert!(
        !execute.contains("Result<"),
        "a Result would give a future caller an Ok arm to fill in",
    );
}
