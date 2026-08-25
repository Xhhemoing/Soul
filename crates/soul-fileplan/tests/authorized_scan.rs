//! AC-18, the half that has to work: scanning the authorized directory
//! produces a read-only plan preview, and the directory is untouched.
//!
//! The refusals in `unauthorized_paths.rs` would all pass against a build that
//! refused everything, so this file is what makes that one mean something. It
//! asserts the plan by naming the moves it expects, not by counting them: a
//! plan that proposed nothing would satisfy "no file was written" perfectly.

mod common;

use common::{walk, Tree};

use soul_fileplan::{Authorization, FileKind, LeaveReason, ScanLimits};
use soul_policy::hitl::{RequestOrigin, TokenIssuer};

const NOW_MS: u64 = 1_787_529_600_000;

fn authorized(tree: &Tree) -> Authorization {
    let mut authorization = Authorization::new();
    authorization
        .authorize(&tree.alpha())
        .expect("authorize Alpha");
    authorization
}

#[test]
fn an_authorized_directory_previews_a_plan_and_the_disk_does_not_move() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let mut issuer = TokenIssuer::new();

    let before = walk(tree.base());
    let preview = soul_fileplan::preview(
        &authorization,
        &mut issuer,
        &tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("a preview of the authorized root");
    let after = walk(tree.base());

    // Three independent statements of the same promise: the crate's own
    // before/after snapshot, an outside walk of the authorized directory, and
    // an outside walk of everything beside it.
    assert!(
        preview.disk_unchanged(),
        "the scan's own snapshots disagree: {:?} then {:?}",
        preview.scan().snapshot_before(),
        preview.scan().snapshot_after(),
    );
    assert_eq!(
        preview.scan().snapshot_before().hash(),
        preview.scan().snapshot_after().hash(),
    );
    assert_eq!(before, after, "the tree changed while it was being scanned");

    let moves: Vec<(&str, &str)> = preview
        .plan()
        .moves()
        .iter()
        .map(|proposed| (proposed.from_relative(), proposed.to_relative()))
        .collect();
    assert_eq!(
        moves,
        vec![
            ("budget.csv", "表格/budget.csv"),
            ("photo.jpg", "图片/photo.jpg"),
            ("report.pdf", "文档/report.pdf"),
            ("笔记.txt", "文档/笔记.txt"),
        ],
    );
}

/// Every file the plan does not move is listed with a reason. A preview that
/// silently dropped what it had no opinion about would leave the user unable to
/// tell "considered and left alone" from "never looked at".
#[test]
fn everything_the_plan_leaves_alone_says_why() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let mut issuer = TokenIssuer::new();

    let preview = soul_fileplan::preview(
        &authorization,
        &mut issuer,
        &tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("a preview");

    let left: Vec<(&str, LeaveReason)> = preview
        .plan()
        .left_alone()
        .iter()
        .map(|left| (left.relative(), left.reason()))
        .collect();
    assert_eq!(
        left,
        vec![
            ("Makefile", LeaveReason::UnrecognisedKind),
            ("mystery.qqq", LeaveReason::UnrecognisedKind),
            ("sub", LeaveReason::Directory),
            ("sub/inner.md", LeaveReason::Nested),
            ("taken.png", LeaveReason::DestinationTaken),
            ("图片", LeaveReason::Directory),
            ("图片/already.png", LeaveReason::AlreadySorted),
            ("图片/taken.png", LeaveReason::AlreadySorted),
        ],
    );

    for left in preview.plan().left_alone() {
        assert!(!left.reason().explanation().is_empty());
    }
}

/// Every destination the plan names is a place inside the root the user
/// authorized. Checked through the same resolver a real path goes through, on
/// paths that do not exist yet, because that is what a destination is.
#[test]
fn no_proposed_destination_leaves_the_authorized_root() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let mut issuer = TokenIssuer::new();

    let preview = soul_fileplan::preview(
        &authorization,
        &mut issuer,
        &tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("a preview");

    assert!(!preview.plan().moves().is_empty());
    for proposed in preview.plan().moves() {
        let destination = format!("{}/{}", tree.alpha(), proposed.to_relative());
        let resolution = authorization
            .resolve(&destination)
            .expect("a destination inside the root");
        assert!(
            !resolution.exists(),
            "{} already exists, so it should not have been proposed",
            proposed.to_relative(),
        );
        assert_eq!(resolution.relative(), proposed.to_relative());
    }
}

/// The scan sees names and sizes. It does not open files, and it does not
/// descend through the two links in the tree.
#[test]
fn the_scan_reads_directory_entries_and_stops_there() {
    let tree = Tree::build();
    let authorization = authorized(&tree);

    let scan = soul_fileplan::scan::scan(&authorization, &tree.alpha(), ScanLimits::default())
        .expect("a scan");

    let relatives: Vec<&str> = scan.entries().iter().map(|e| e.relative()).collect();
    assert_eq!(
        relatives,
        vec![
            "Makefile",
            "budget.csv",
            "mystery.qqq",
            "photo.jpg",
            "report.pdf",
            "sub",
            "sub/inner.md",
            "taken.png",
            "图片",
            "图片/already.png",
            "图片/taken.png",
            "笔记.txt",
        ],
    );

    let csv = scan
        .files()
        .find(|entry| entry.relative() == "budget.csv")
        .expect("the spreadsheet");
    assert_eq!(csv.kind(), FileKind::Sheet);
    assert_eq!(csv.extension(), Some("csv"));
    assert_eq!(csv.size_bytes(), "a,b\n1,2\n".len() as u64);
    assert!(csv.modified_unix_seconds().is_some());

    #[cfg(unix)]
    {
        use soul_fileplan::SkipReason;
        let skipped: Vec<(&str, SkipReason)> = scan
            .skipped()
            .iter()
            .map(|entry| (entry.shown.as_str(), entry.reason))
            .collect();
        assert_eq!(
            skipped,
            vec![
                ("escape", SkipReason::Symlink),
                ("loopback", SkipReason::Symlink),
            ],
            "v0.1 does not follow a link, inward or outward",
        );
        assert!(
            !relatives.iter().any(|r| r.starts_with("loopback")),
            "the link that stays inside the root is still not followed",
        );
    }
}

/// The same directory scanned twice produces the same hash; a file appearing
/// produces a different one. The second half is AC-19's middle case set up.
#[test]
fn the_plan_hash_is_stable_until_the_directory_is_not() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let mut issuer = TokenIssuer::new();

    let first = soul_fileplan::preview(
        &authorization,
        &mut issuer,
        &tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("first preview");
    let second = soul_fileplan::preview(
        &authorization,
        &mut issuer,
        &tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("second preview");
    assert_eq!(first.plan_hash(), second.plan_hash());
    assert_eq!(first.plan(), second.plan());

    std::fs::write(
        std::path::Path::new(&tree.alpha()).join("holiday.jpeg"),
        "another photo",
    )
    .expect("the user saves a file");

    let third = soul_fileplan::preview(
        &authorization,
        &mut issuer,
        &tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("third preview");
    assert_ne!(first.plan_hash(), third.plan_hash());
    assert_eq!(third.plan().moves().len(), first.plan().moves().len() + 1);
}

/// A subdirectory of an authorized root is authorized too, and scanning it
/// gives a plan about that subdirectory rather than about its parent.
#[test]
fn a_directory_below_the_root_can_be_scanned_on_its_own() {
    let tree = Tree::build();
    let authorization = authorized(&tree);

    let scan = soul_fileplan::scan::scan(
        &authorization,
        &tree.path_of("Alpha/sub"),
        ScanLimits::default(),
    )
    .expect("a scan of a subdirectory");

    let relatives: Vec<&str> = scan.entries().iter().map(|e| e.relative()).collect();
    assert_eq!(relatives, vec!["inner.md"]);

    let plan = soul_fileplan::plan::build(&scan);
    assert_eq!(plan.moves().len(), 1);
    assert_eq!(plan.moves()[0].to_relative(), "文档/inner.md");
}

/// The depth limit stops the walk and says so, rather than showing less
/// without mentioning it.
#[test]
fn a_limit_that_bites_is_reported_instead_of_hidden() {
    let tree = Tree::build();
    let authorization = authorized(&tree);

    let scan = soul_fileplan::scan::scan(
        &authorization,
        &tree.alpha(),
        ScanLimits {
            max_depth: 1,
            max_entries: 1_000,
        },
    )
    .expect("a shallow scan");

    assert!(scan.truncated());
    assert!(scan.entries().iter().all(|entry| entry.depth() == 1));
    assert!(soul_fileplan::plan::build(&scan).truncated());
}

/// The plan says, in the value its hash is taken over, that this build will not
/// carry it out. A build that started executing plans would hash differently
/// from the one the user approved a plan in.
#[test]
fn the_plan_states_in_its_own_hash_that_it_will_not_be_executed() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let mut issuer = TokenIssuer::new();

    let preview = soul_fileplan::preview(
        &authorization,
        &mut issuer,
        &tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("a preview");

    assert!(!preview.plan().executable_in_this_version());
    let json = preview.plan().to_json();
    assert_eq!(json["executable_in_this_version"], serde_json::json!(false));
    assert_eq!(json["action"], serde_json::json!("plan.files"));

    let mut edited = json.clone();
    edited["executable_in_this_version"] = serde_json::json!(true);
    assert_ne!(
        soul_policy::hitl::PlanHash::of(&edited),
        *preview.plan_hash(),
    );
}

/// Previewing needs no capability token, so previewing cannot spend one. This
/// is the file-plan side of WP08's promise that v0.1 issues and consumes no
/// file-write token.
#[test]
fn a_preview_never_touches_the_token_ledger() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let mut issuer = TokenIssuer::new();

    for _ in 0..3 {
        soul_fileplan::preview(
            &authorization,
            &mut issuer,
            &tree.alpha(),
            RequestOrigin::User,
            ScanLimits::default(),
            NOW_MS,
        )
        .expect("a preview");
    }
    assert_eq!(issuer.issued_count(), 0);

    for kind in soul_fileplan::FILEPLAN_ACTIONS {
        assert!(
            !kind.needs_capability_token(),
            "{} would make a preview spend a token",
            kind.as_str(),
        );
    }
}
