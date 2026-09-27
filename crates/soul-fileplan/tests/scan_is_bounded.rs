//! What the scan does when the directory is bigger than the preview is.
//!
//! `ScanLimits` exists because a preview that takes ten minutes is a hang, and
//! the preview runs with the session locked: whatever the walk collects, the
//! rest of the application waits behind. The depth half of that promise was
//! already tested in `authorized_scan.rs`. This file is the entry half, and it
//! is written against a directory large enough that "bounded" and "eventually
//! finishes" are different statements.
//!
//! The load-bearing assertion is not that the limit is reported. It is that
//! the same limit over a directory four times the size collects exactly the
//! same amount — including the skipped list, which is otherwise the unbounded
//! list under another name.

use std::path::{Path, PathBuf};

use soul_fileplan::{Authorization, DirectorySnapshot, ScanLimits, SkipReason};

/// Comfortably more entries than any limit used below, small enough that
/// creating it twice per test is cheap.
const CROWD: usize = 400;

struct Folder {
    directory: tempfile::TempDir,
    root: PathBuf,
}

impl Folder {
    /// One flat directory holding `count` files, and nothing else.
    fn flat(count: usize) -> Folder {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let base = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
        let root = base.join("Alpha");
        std::fs::create_dir_all(&root).expect("create the folder");
        for index in 0..count {
            std::fs::write(root.join(format!("file-{index:06}.txt")), "x")
                .expect("write a fixture file");
        }
        Folder { directory, root }
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn raw(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }

    /// Keeps the temporary directory alive for as long as the folder is.
    fn keep(&self) -> &tempfile::TempDir {
        &self.directory
    }

    fn authorization(&self) -> Authorization {
        let mut authorization = Authorization::new();
        authorization.authorize(&self.raw()).expect("authorize");
        authorization
    }
}

fn limits(max_entries: usize) -> ScanLimits {
    ScanLimits {
        max_depth: 8,
        max_entries,
    }
}

/// The snapshot is the half that had no entry bound at all: it hashes names
/// the plan walk skips, so it has to stop on its own.
#[test]
fn a_snapshot_stops_at_the_entry_limit_and_says_so() {
    let folder = Folder::flat(CROWD);
    let _keep = folder.keep();

    let complete = DirectorySnapshot::of(folder.path(), limits(CROWD * 4));
    assert_eq!(complete.entries(), CROWD);
    assert!(!complete.truncated());

    let bounded = DirectorySnapshot::of(folder.path(), limits(25));
    assert_eq!(
        bounded.entries(),
        25,
        "the snapshot hashed more entries than the limit allowed",
    );
    assert!(
        bounded.truncated(),
        "a snapshot that saw 25 of {CROWD} entries has to say so",
    );
}

/// A partial view of a directory is a prefix of the complete one. If the two
/// hashed the same, a plan hash taken over the snapshot would claim knowledge
/// of a directory the scan never finished reading.
#[test]
fn a_truncated_snapshot_does_not_hash_like_a_complete_one() {
    let folder = Folder::flat(CROWD);
    let _keep = folder.keep();

    let complete = DirectorySnapshot::of(folder.path(), limits(CROWD));
    let bounded = DirectorySnapshot::of(folder.path(), limits(CROWD - 1));

    assert!(!complete.truncated());
    assert!(bounded.truncated());
    assert_ne!(complete.hash(), bounded.hash());
    assert_ne!(complete, bounded);
}

/// Nothing changed on disk, so the before and after snapshots agree — the
/// read-only proof survives truncation, it just covers less.
#[test]
fn two_truncated_snapshots_of_an_unchanged_directory_still_agree() {
    let folder = Folder::flat(CROWD);
    let _keep = folder.keep();

    let before = DirectorySnapshot::of(folder.path(), limits(30));
    let after = DirectorySnapshot::of(folder.path(), limits(30));

    assert!(before.truncated());
    assert_eq!(before, after);
    assert_eq!(before.hash(), after.hash());
}

/// The whole point: a directory nobody could preview does not become a
/// directory nobody can use. Entries, skips and both snapshots stay inside the
/// limit, and the scan says it was cut short.
#[test]
fn a_huge_directory_is_scanned_within_its_limits() {
    let folder = Folder::flat(CROWD);
    let _keep = folder.keep();
    let authorization = folder.authorization();

    let scan = soul_fileplan::scan::scan(&authorization, &folder.raw(), limits(20))
        .expect("a bounded scan");

    assert_eq!(scan.entries().len(), 20);
    assert!(scan.truncated());
    assert_eq!(
        scan.skipped()
            .iter()
            .filter(|entry| entry.reason == SkipReason::EntryLimit)
            .count(),
        1,
        "the limit is reported once, not once per entry behind it",
    );
    assert!(scan.snapshot_before().entries() <= 20);
    assert!(scan.snapshot_after().entries() <= 20);
    assert!(
        scan.disk_unchanged(),
        "the scan wrote nothing, limit or no limit: {:?} then {:?}",
        scan.snapshot_before(),
        scan.snapshot_after(),
    );
    assert!(soul_fileplan::plan::build(&scan).truncated());
}

/// A fake directory four times the size, scanned under the same limit, costs
/// the same. Were any list in the walk still growing with the directory rather
/// than with the limit, this is where it would show.
#[test]
fn a_directory_four_times_the_size_does_not_collect_four_times_as_much() {
    let small = Folder::flat(CROWD);
    let _keep_small = small.keep();
    let large = Folder::flat(CROWD * 4);
    let _keep_large = large.keep();

    let limits = limits(20);
    let small_scan = soul_fileplan::scan::scan(&small.authorization(), &small.raw(), limits)
        .expect("a scan of the smaller directory");
    let large_scan = soul_fileplan::scan::scan(&large.authorization(), &large.raw(), limits)
        .expect("a scan of the larger directory");

    assert_eq!(small_scan.entries().len(), large_scan.entries().len());
    assert_eq!(small_scan.skipped().len(), large_scan.skipped().len());
    assert_eq!(
        small_scan.snapshot_before().entries(),
        large_scan.snapshot_before().entries(),
    );
    assert!(small_scan.truncated() && large_scan.truncated());
}

/// A limit of zero is a limit. The walk collects nothing and admits it, rather
/// than treating "no room" as "nothing there".
#[test]
fn a_limit_of_zero_collects_nothing_and_admits_it() {
    let folder = Folder::flat(4);
    let _keep = folder.keep();

    let snapshot = DirectorySnapshot::of(folder.path(), limits(0));
    assert_eq!(snapshot.entries(), 0);
    assert!(snapshot.truncated());

    let scan = soul_fileplan::scan::scan(&folder.authorization(), &folder.raw(), limits(0))
        .expect("a scan with no room in it");
    assert!(scan.entries().is_empty());
    assert!(scan.truncated());
    assert_ne!(
        snapshot.hash(),
        DirectorySnapshot::of(folder.path(), limits(64)).hash(),
        "an empty view of four files is not the same as four files",
    );
}

#[test]
fn skipped_names_consume_the_entry_budget() {
    let folder = Folder::flat(0);
    for index in 0..80 {
        std::fs::write(
            folder.path().join(format!("skip-\u{0085}-{index:03}.txt")),
            "x",
        )
        .expect("write a name the planner must skip");
    }
    let scan = soul_fileplan::scan::scan(&folder.authorization(), &folder.raw(), limits(5))
        .expect("a bounded scan");
    assert!(scan.entries().is_empty());
    assert_eq!(
        scan.skipped()
            .iter()
            .filter(|entry| entry.reason == SkipReason::UnplannableName)
            .count(),
        5,
        "skipped names must cost the same budget as accepted names",
    );
    assert_eq!(
        scan.skipped()
            .iter()
            .filter(|entry| entry.reason == SkipReason::EntryLimit)
            .count(),
        1
    );
    assert!(scan.truncated());
    assert!(scan.disk_unchanged());
}

#[test]
fn a_snapshot_obeys_the_same_depth_boundary_as_the_scan() {
    let folder = Folder::flat(0);
    std::fs::create_dir_all(folder.path().join("nested/deeper")).expect("nested directories");
    std::fs::write(
        folder.path().join("nested/hidden.txt"),
        "outside the preview depth",
    )
    .expect("nested fixture");
    let limits = ScanLimits {
        max_depth: 1,
        max_entries: 50,
    };
    let scan = soul_fileplan::scan::scan(&folder.authorization(), &folder.raw(), limits)
        .expect("a shallow scan");
    assert_eq!(scan.entries().len(), 1);
    assert_eq!(
        scan.snapshot_before().entries(),
        1,
        "the snapshot must not enter nested"
    );
    assert!(
        scan.snapshot_before().truncated(),
        "the depth bound leaves a partial snapshot"
    );
    assert_eq!(scan.snapshot_before(), scan.snapshot_after());
}

#[test]
fn zero_depth_never_reads_children() {
    let folder = Folder::flat(4);
    let limits = ScanLimits {
        max_depth: 0,
        max_entries: 50,
    };
    let scan = soul_fileplan::scan::scan(&folder.authorization(), &folder.raw(), limits)
        .expect("zero-depth preview");
    assert!(scan.entries().is_empty());
    assert_eq!(scan.snapshot_before().entries(), 0);
    assert_eq!(scan.snapshot_after().entries(), 0);
    assert!(scan.truncated());
    assert_eq!(scan.skipped().len(), 1);
    assert_eq!(scan.skipped()[0].reason, SkipReason::DepthLimit);
}
