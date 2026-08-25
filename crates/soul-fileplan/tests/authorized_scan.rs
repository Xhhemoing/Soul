//! AC-18, the half that must work: an authorized directory is read, the plan
//! that comes back describes what is really there, and the tree is byte for
//! byte what it was.
//!
//! The tree is built from `fixtures/fileplan/sample_tree.json` in a temporary
//! directory. No `ScanEntry` is constructed by hand anywhere in this file —
//! the type has private fields and no public constructor, so an observation
//! in these assertions is an observation of a real disk.

mod common;

use std::collections::BTreeSet;

use soul_fileplan::{EntryKind, PlanAction};
use soul_policy::hitl::PlanHash;

/// How many times a read-only operation is repeated before the tree is
/// checked again. Once could miss a lazy write; three is cheap.
const REPEATS: usize = 3;

#[test]
fn authorized_root_scans_real_entries() {
    let space = common::workspace();
    let files = common::build_sample_tree(&space.authorized);

    let report =
        soul_fileplan::scan(&space.roots(), &space.authorized).expect("an authorized scan");

    let on_disk = common::paths_in(&common::snapshot(&space.authorized));
    let scanned: BTreeSet<String> = report
        .entries()
        .iter()
        .map(|entry| common::encode(entry.rel_path()))
        .collect();

    assert!(!scanned.is_empty(), "the fixture tree is not empty");
    assert_eq!(
        scanned, on_disk,
        "the scan must report the tree that is there, no more and no less",
    );
    assert_eq!(report.file_count(), files);
    assert!(report.dir_count() > 0, "the fixture has subdirectories");
    assert_eq!(
        report.skipped_escaping_links(),
        0,
        "there are no links here"
    );

    // Sizes are read, not assumed: an entry that reported a constant would
    // disagree with the file it names, and a blanket "every file is nonempty"
    // would lock the zero-byte fixture out.
    let described = common::sample_tree();
    for entry in report.entries() {
        let full = space.authorized.join(entry.rel_path());
        let metadata = std::fs::metadata(&full).expect("the scanned path exists");
        match entry.kind() {
            EntryKind::File => {
                assert!(metadata.is_file());
                assert_eq!(entry.bytes(), metadata.len(), "{:?}", entry.rel_path());
                let rel = common::encode(entry.rel_path());
                if let Some(sample) = described
                    .entries
                    .iter()
                    .find(|sample| sample.relative_path == rel)
                {
                    let expected = sample.content_utf8.as_deref().unwrap_or_default().len() as u64;
                    assert_eq!(entry.bytes(), expected, "{rel}");
                }
            }
            EntryKind::Dir => {
                assert!(metadata.is_dir());
                assert_eq!(entry.bytes(), 0);
            }
        }
    }

    // Names are relative to the scanned directory, so nothing carries the
    // temporary parent — or, on a real machine, the user's home — with it.
    for entry in report.entries() {
        assert!(
            entry.rel_path().is_relative(),
            "an absolute path in a scan entry would end up in every plan",
        );
    }
}

#[test]
fn scan_and_preview_leave_the_tree_byte_for_byte_unchanged() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);
    common::write_file(
        &space.authorized.join("large.bin.txt"),
        &"x".repeat(1 << 20),
    );

    // Everything the setup writes is written before the snapshot is taken;
    // otherwise the test measures its own tail.
    let before = common::snapshot(&space.authorized);
    assert!(!before.is_empty());

    let mut last = None;
    for _ in 0..REPEATS {
        let report =
            soul_fileplan::scan(&space.roots(), &space.authorized).expect("an authorized scan");
        let preview = soul_fileplan::plan(&report);
        last = Some(preview);
    }
    let after = common::snapshot(&space.authorized);

    assert_eq!(
        before, after,
        "a scan and a preview must not create, remove, grow or alter any file",
    );

    // The reverse: this passes trivially if the preview is empty, so it must
    // not be, and every file it names must be one of the files in the
    // snapshot that was just compared.
    let preview = last.expect("the loop ran");
    assert!(
        !preview.is_empty(),
        "the fixture tree has something to tidy"
    );
    let known = common::paths_in(&after);
    for entry in preview.entries() {
        assert!(
            known.contains(&common::encode(entry.source_rel())),
            "{:?} is not one of the files that are really there",
            entry.source_rel(),
        );
    }
}

#[test]
fn preview_is_derived_from_the_real_scan() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);

    let report =
        soul_fileplan::scan(&space.roots(), &space.authorized).expect("an authorized scan");
    let preview = soul_fileplan::plan(&report);

    let files: BTreeSet<String> = report
        .entries()
        .iter()
        .filter(|entry| entry.is_file())
        .map(|entry| common::encode(entry.rel_path()))
        .collect();

    assert!(!preview.is_empty());
    assert_eq!(preview.len(), preview.entries().len());
    for entry in preview.entries() {
        assert!(
            files.contains(&common::encode(entry.source_rel())),
            "every suggestion is about a file the scan saw",
        );
        assert!(
            entry.target_rel().is_some(),
            "each of the three suggestions proposes somewhere to put the file",
        );
    }
    assert!(
        preview.entries().len() <= files.len(),
        "a suggestion is per file at most, so the plan cannot invent entries",
    );

    // The JSON that gets hashed says exactly what the entries say. This is
    // the assertion that keeps the approval honest: a hash taken over a plan
    // that had dropped the target would approve "rename this file" without
    // approving what to, and would still look like a hash.
    let json = preview.to_plan_json();
    let encoded = json["entries"]
        .as_array()
        .expect("the plan lists its entries");
    assert_eq!(encoded.len(), preview.entries().len());
    for (value, entry) in encoded.iter().zip(preview.entries()) {
        assert_eq!(
            value["source_rel"],
            serde_json::json!(common::encode(entry.source_rel())),
        );
        assert_eq!(value["action"], serde_json::json!(entry.action().as_str()));
        assert_eq!(
            value["target_rel"],
            serde_json::json!(entry.target_rel().map(common::encode)),
        );
    }
    assert_eq!(json["entry_count"], serde_json::json!(preview.len()));
    assert_eq!(json["root"], serde_json::json!(preview.root_fingerprint()));

    // The fixture tree is shaped to produce one of each, which a plan that
    // ignored the tree could not manage.
    for action in PlanAction::ALL {
        assert!(
            preview.count_of(*action) > 0,
            "the fixture should produce a `{}` suggestion",
            action.as_str(),
        );
    }
    assert_eq!(
        preview.count_of(PlanAction::Group)
            + preview.count_of(PlanAction::Move)
            + preview.count_of(PlanAction::Rename),
        preview.len(),
        "every entry is counted exactly once",
    );

    // An empty directory has nothing to suggest. A constant plan would still
    // have something to say about it.
    let empty = space.authorized.join("inbox").join("empty");
    std::fs::create_dir_all(&empty).expect("create an empty directory");
    let empty_report = soul_fileplan::scan(&space.roots(), &empty).expect("an authorized scan");
    assert!(soul_fileplan::plan(&empty_report).is_empty());

    // The rendering and the hashed JSON come from the same entries, so a
    // display layer cannot show one plan while another is approved.
    let rendered = preview.render();
    for entry in preview.entries() {
        assert!(rendered.contains(&common::encode(entry.source_rel())));
    }
}

#[test]
fn the_same_tree_scanned_twice_yields_the_same_plan_hash() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);

    let first_report = soul_fileplan::scan(&space.roots(), &space.authorized).expect("scan once");
    let first = soul_fileplan::plan(&first_report);
    let second = soul_fileplan::plan(
        &soul_fileplan::scan(&space.roots(), &space.authorized).expect("scan again"),
    );

    // Sorted, not merely repeatable. Two runs of one process see one
    // directory order, so a plan that inherited the walk's order would agree
    // with itself here and disagree with the same tree on another machine.
    // The invariant has to be asserted directly.
    let scanned: Vec<&std::path::Path> = first_report
        .entries()
        .iter()
        .map(|entry| entry.rel_path())
        .collect();
    let mut expected = scanned.clone();
    expected.sort();
    assert_eq!(scanned, expected, "the scan orders its entries");
    let mut planned = first.entries().to_vec();
    planned.sort();
    assert_eq!(first.entries(), planned, "and so does the plan");

    assert_eq!(
        PlanHash::of(&first.to_plan_json()),
        PlanHash::of(&second.to_plan_json()),
        "directory order is not a promise any filesystem makes; if it reached the plan, an \
         unchanged tree would fail its own approval",
    );
    assert_eq!(first.entries(), second.entries());

    // The directory is named the same way both times, and the two scans are
    // still two scans. The scan's own identifier is deliberately outside the
    // hash for exactly this reason.
    assert_eq!(first.root_fingerprint(), second.root_fingerprint());
    assert_ne!(first.scan_id(), second.scan_id());
    assert!(!first
        .to_plan_json()
        .to_string()
        .contains(&first.scan_id().to_string()));

    // A different directory is a different plan, even when the two hold the
    // same files, because the root is part of what is approved.
    let elsewhere = space.parent.path().join("c");
    std::fs::create_dir_all(&elsewhere).expect("create another directory");
    common::build_sample_tree(&elsewhere);
    let other_roots = common::roots(&[&space.authorized, &elsewhere]);
    let other = soul_fileplan::plan(
        &soul_fileplan::scan(&other_roots, &elsewhere).expect("the copy is authorized too"),
    );
    assert_eq!(other.entries(), first.entries(), "the same tree, copied");
    assert_ne!(
        PlanHash::of(&other.to_plan_json()),
        PlanHash::of(&first.to_plan_json()),
        "an approval is about one directory",
    );
}

#[test]
fn changing_a_real_file_changes_the_plan_hash() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);

    let before = PlanHash::of(
        &soul_fileplan::plan(
            &soul_fileplan::scan(&space.roots(), &space.authorized).expect("scan"),
        )
        .to_plan_json(),
    );

    common::write_file(
        &space.authorized.join("one-more.txt"),
        "another loose file\n",
    );

    let after = PlanHash::of(
        &soul_fileplan::plan(
            &soul_fileplan::scan(&space.roots(), &space.authorized).expect("scan"),
        )
        .to_plan_json(),
    );

    assert_ne!(
        before, after,
        "a plan that did not change when the tree did would be a constant",
    );
}

#[test]
fn a_preview_can_never_claim_a_disk_write() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);

    let populated =
        soul_fileplan::plan(&soul_fileplan::scan(&space.roots(), &space.authorized).expect("scan"));
    let empty_dir = space.authorized.join("reference").join("nothing-here");
    std::fs::create_dir_all(&empty_dir).expect("create an empty directory");
    let empty =
        soul_fileplan::plan(&soul_fileplan::scan(&space.roots(), &empty_dir).expect("scan"));

    // Not a field, not an argument: `plan` is the only constructor and it
    // takes a scan report. There is nowhere for a caller to pass `true`, and
    // no setter to reach afterwards — the compiler enforces that, and these
    // assertions record the value it produces.
    assert!(!populated.written_to_disk());
    assert!(!empty.written_to_disk());
    assert_eq!(
        populated.to_plan_json()["written_to_disk"],
        serde_json::json!(false),
    );
    assert!(populated.render().contains("Nothing is carried out"));
}
