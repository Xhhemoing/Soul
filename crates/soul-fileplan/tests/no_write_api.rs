//! The promise this crate makes is about what it does *not* do, and no
//! amount of running it can demonstrate that. So the sources are read back.
//!
//! Two decisions worth stating, because both have an obvious wrong version:
//!
//! * the file list is built at run time by walking `src/`, not written down
//!   with `include_str!`. A named list stops covering the crate the moment
//!   somebody adds a module, and it fails open — quietly, and in the
//!   direction of not checking;
//! * the vocabulary is the vocabulary of *writing*, not of the filesystem.
//!   `crates/soul-store/tests/research_preview.rs` forbids `PathBuf` and
//!   `tempfile`, which is right for a module that has no business naming a
//!   path at all and wrong here: a directory scanner cannot be written
//!   without `Path`, `read_dir` and `metadata`. Copying that list would have
//!   produced a red test and, eventually, a weakened one.
//!
//! Only `src/` is read. Tests write files — this very file's neighbours
//! create trees to scan — and a rule that covered them would have to be
//! turned off to be satisfied.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Ways to write, create, remove or rearrange something on disk.
///
/// Notes on the shapes chosen, since a few are deliberately not the bare
/// name: `create_dir` also catches `create_dir_all`; `fs::rename` and
/// `fs::copy` are qualified because a bare `rename` would hit
/// `PlanAction::Rename` and a bare `copy` would hit every `Copy` bound; and
/// the link entries name the three creation calls, because a bare `symlink`
/// appears in the comments and identifiers of the code that refuses to
/// follow one.
const WAYS_TO_WRITE: &[&str] = &[
    "fs::write",
    "File::create",
    "File::options",
    "OpenOptions",
    "create_dir",
    "remove_file",
    "remove_dir",
    "fs::rename",
    "fs::copy",
    "hard_link",
    "os::unix::fs::symlink",
    "symlink_file",
    "symlink_dir",
    "set_permissions",
    "set_len",
    ".truncate(",
    ".append(",
    ".create_new(",
    "BufWriter",
    "io::Write",
    "write!",
    "writeln!",
    "tempfile",
];

/// Capability machinery. Neither action in WP11 needs a token, so a mention
/// of one here means the wrong path was taken.
const TOKEN_MACHINERY: &[&str] = &[
    "TokenIssuer",
    "CapabilityToken",
    "CapabilityScope",
    "FileWrite",
    "issue_token",
];

/// Function names that would mean this crate had learned to act.
const ACTING_VERBS: &[&str] = &["execute", "apply", "undo"];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `src/`, as (path relative to `src/`, text).
fn sources() -> Vec<(String, String)> {
    let root = src_dir();
    let mut found: Vec<(String, String)> = Vec::new();
    for step in WalkDir::new(&root).follow_links(false) {
        let step = step.expect("read the source tree");
        if step.file_type().is_dir() || step.path().extension() != Some("rs".as_ref()) {
            continue;
        }
        let name = step
            .path()
            .strip_prefix(&root)
            .expect("walkdir stays under src/")
            .to_string_lossy()
            .into_owned();
        let text = std::fs::read_to_string(step.path()).expect("read a source file");
        found.push((name, text));
    }
    found.sort();
    found
}

/// The names of every function declared in `text`.
fn function_names(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for (index, _) in text.match_indices("fn ") {
        let rest = &text[index + 3..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.insert(name);
        }
    }
    names
}

#[test]
fn production_source_has_no_write_surface() {
    for (name, text) in sources() {
        for needle in WAYS_TO_WRITE {
            assert!(
                !text.contains(needle),
                "src/{name} mentions `{needle}`; WP11 previews and stops, so nothing in this \
                 crate may write, create, remove or move a file",
            );
        }
    }
}

#[test]
fn the_scanner_saw_the_real_modules() {
    let sources = sources();
    let names: BTreeSet<&str> = sources.iter().map(|(name, _)| name.as_str()).collect();

    // The anchor. Without it, a file that moved would leave the assertions
    // above passing over an empty list.
    assert!(
        sources.len() >= 4,
        "expected the crate's modules, found {names:?}",
    );
    for expected in ["lib.rs", "authorize.rs", "scan.rs", "plan.rs", "error.rs"] {
        assert!(names.contains(expected), "src/{expected} was not read");
    }

    let functions: BTreeSet<String> = sources
        .iter()
        .flat_map(|(_, text)| function_names(text))
        .collect();
    for expected in [
        "scan",
        "plan",
        "canonicalized",
        "authorize",
        "written_to_disk",
    ] {
        assert!(
            functions.contains(expected),
            "`fn {expected}` was not among the functions read; the scanner is not looking at \
             this crate",
        );
    }
}

#[test]
fn the_scanner_recognises_a_synthetic_write_call() {
    // The control. If the check above cannot fail, it is not a check.
    let synthetic = "fn tidy(path: &Path) { std::fs::write(path, b\"\").unwrap(); }";
    let caught: Vec<&&str> = WAYS_TO_WRITE
        .iter()
        .filter(|needle| synthetic.contains(**needle))
        .collect();
    assert_eq!(
        caught,
        vec![&"fs::write"],
        "the vocabulary must catch a plain write and nothing else in this line",
    );

    let also_synthetic = "let file = File::create(path)?; let mut out = BufWriter::new(file);";
    assert!(WAYS_TO_WRITE
        .iter()
        .any(|needle| also_synthetic.contains(needle)));
}

#[test]
fn no_execute_or_apply_entry_point_exists() {
    // A compile-time fact, recorded here: there is no `execute`, `apply` or
    // `undo` in this crate's public surface, because carrying a plan out is
    // v0.1.1. Nothing calls into this crate expecting one, and nothing here
    // could be persuaded to grow one quietly.
    for (name, text) in sources() {
        for function in function_names(&text) {
            let lowered = function.to_lowercase();
            for verb in ACTING_VERBS {
                assert!(
                    !lowered.contains(verb),
                    "src/{name} declares `fn {function}`; v0.1 previews and stops",
                );
            }
        }
    }
}

#[test]
fn no_token_machinery_appears_in_the_source() {
    for (name, text) in sources() {
        for needle in TOKEN_MACHINERY {
            assert!(
                !text.contains(needle),
                "src/{name} mentions `{needle}`; neither `scan.directory` nor `plan.files` \
                 needs a capability token, so reaching for one means the wrong gate was used",
            );
        }
    }
}
