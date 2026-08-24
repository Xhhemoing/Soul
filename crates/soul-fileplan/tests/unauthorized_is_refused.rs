//! AC-18, the half that must not work.
//!
//! Every way out of an authorized root gets its own case, and each one is
//! asserted rather than sampled: a refusal matrix with a gap in it is a
//! refusal matrix that says nothing about the gap. The count is checked too —
//! requests in, refusals out, with no partial results — because "mostly
//! refused" is the failure mode this is here to catch.

mod common;

use std::path::{Path, PathBuf};

use soul_fileplan::{AuthorizedRoots, FilePlanError};
use soul_policy::ReasonCode;

/// Prose that lives outside the authorized root and must never surface.
const SECRET: &str = "这是 B 目录里的私人内容，扫描 A 时不该出现";
const SECRET_FILE: &str = "private-notes.txt";

/// Refuse, with the one reason code that means it, and nothing else.
fn assert_refused(roots: &AuthorizedRoots, target: &Path) -> FilePlanError {
    let error = soul_fileplan::scan(roots, target)
        .err()
        .unwrap_or_else(|| panic!("{} must be refused", target.display()));
    assert!(
        matches!(error, FilePlanError::PathNotAuthorized { .. }),
        "expected a refusal, got {error:?}",
    );
    assert_eq!(error.reason_code(), ReasonCode::PathNotAuthorized);
    error
}

#[test]
fn empty_roots_refuse_everything() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);

    let roots = AuthorizedRoots::canonicalized(&[]).expect("an empty list of roots is legal");
    assert!(roots.is_empty());
    assert_eq!(roots.len(), 0);

    // Including the directory that would have been authorized had anyone
    // authorized it. The shipped default is "no directory", and that has to
    // be a refusal rather than a pass with nothing to check against.
    let targets = [
        space.authorized.clone(),
        space.authorized.join("inbox"),
        space.outside.clone(),
        space.parent.path().to_path_buf(),
    ];
    let mut refused = 0usize;
    for target in &targets {
        assert_refused(&roots, target);
        refused += 1;
    }
    assert_eq!(refused, targets.len(), "no request may get through");
}

#[test]
fn a_path_outside_every_root_is_refused() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);
    common::write_file(&space.outside.join(SECRET_FILE), SECRET);

    let roots = space.roots();
    let targets = [
        space.outside.clone(),
        space.outside.join(SECRET_FILE),
        space.parent.path().to_path_buf(),
    ];
    let mut refused = 0usize;
    for target in &targets {
        assert_refused(&roots, target);
        refused += 1;
    }
    assert_eq!(refused, targets.len(), "requests in, refusals out");
}

#[test]
fn dotdot_traversal_out_of_the_root_is_refused() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);
    common::write_file(&space.outside.join(SECRET_FILE), SECRET);

    let roots = space.roots();

    // Textually inside the root, and outside it once resolved. This is the
    // case that decides the order of the algorithm: canonicalize first, then
    // compare, because the reverse lets this through.
    assert_refused(&roots, &space.authorized.join("..").join("b"));
    assert_refused(
        &roots,
        &space
            .authorized
            .join("inbox")
            .join("..")
            .join("..")
            .join("b"),
    );
    assert_refused(
        &roots,
        &space.authorized.join("..").join("b").join(SECRET_FILE),
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_escaping_the_root_is_refused() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);
    common::write_file(&space.outside.join(SECRET_FILE), SECRET);

    let link = space.authorized.join("outside");
    std::os::unix::fs::symlink(&space.outside, &link).expect("create the escaping link");
    let roots = space.roots();

    // Naming the link as the thing to scan refuses the whole request: the
    // link resolves to B, and B is not authorized.
    assert_refused(&roots, &link);

    // Scanning A walks past it. The link is counted, and nothing under B —
    // no path, no name, no byte count — reaches the report or the plan.
    let report = soul_fileplan::scan(&roots, &space.authorized).expect("A itself is authorized");
    assert_eq!(report.skipped_escaping_links(), 1);
    let preview = soul_fileplan::plan(&report);

    let seen = format!("{report:?} {preview:?} {}", preview.render());
    for needle in [SECRET, SECRET_FILE, "outside"] {
        assert!(
            !seen.contains(needle),
            "`{needle}` came back from a directory that was never authorized",
        );
    }
    assert!(
        report
            .entries()
            .iter()
            .all(|entry| entry.rel_path() != Path::new("outside")),
        "the link itself is not an entry either",
    );

    // A link that resolves to nothing cannot be shown to stay inside, so it
    // is skipped for the same reason rather than followed hopefully.
    std::os::unix::fs::symlink(
        space.outside.join("does-not-exist"),
        space.authorized.join("dangling"),
    )
    .expect("create a dangling link");
    let report = soul_fileplan::scan(&roots, &space.authorized).expect("A is still authorized");
    assert_eq!(report.skipped_escaping_links(), 2);
}

#[cfg(not(unix))]
#[test]
#[ignore = "creating a link or a junction needs a privileged call on Windows; this case is on \
            the manual checklist in .agent_workspace/dev-sota/reports/ST-02.md"]
fn a_symlink_escaping_the_root_is_refused() {}

#[test]
fn a_parent_directory_of_the_root_is_refused() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);

    let roots = space.roots();
    assert_refused(&roots, space.parent.path());

    // And the parent of that, for as far up as there is one to name.
    if let Some(grandparent) = space.parent.path().parent() {
        assert_refused(&roots, grandparent);
    }
}

#[test]
fn a_sibling_sharing_the_root_prefix_is_refused() {
    let parent = tempfile::tempdir().expect("a temporary parent directory");
    let data = parent.path().join("data");
    let authorized = data.join("a");
    let sibling = data.join("ab");
    std::fs::create_dir_all(&authorized).expect("create the authorized directory");
    std::fs::create_dir_all(&sibling).expect("create its neighbour");
    common::write_file(&sibling.join(SECRET_FILE), SECRET);

    let roots = common::roots(&[&authorized]);

    // `/data/ab` starts with the string `/data/a` and is not inside it. The
    // comparison is per path component for exactly this reason.
    assert_refused(&roots, &sibling);
    assert_refused(&roots, &sibling.join(SECRET_FILE));
}

#[test]
fn a_case_or_nfd_variant_outside_the_root_is_refused() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);

    // The same word, composed and decomposed. On Linux these are two
    // different directory names; only one of them exists.
    let composed = space.outside.join("caf\u{e9}");
    let decomposed = space.outside.join("cafe\u{301}");
    std::fs::create_dir_all(&composed).expect("create the composed spelling");

    let roots = space.roots();
    assert_refused(&roots, &composed);
    assert_refused(&roots, &decomposed);

    // A differently cased spelling of the authorized root itself. On this
    // host it does not resolve, so it is refused: the decision is made on
    // the bytes canonicalization returns, not on a case-folded comparison
    // this crate invented. On a case-insensitive filesystem the same request
    // would canonicalize to the very same directory and be allowed, which is
    // the right answer there — it is the same directory.
    #[cfg(target_os = "linux")]
    {
        let shouted = space.parent.path().join("A");
        assert_refused(&roots, &shouted);
    }
}

#[test]
fn refusal_names_no_requested_path() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);
    common::write_file(&space.outside.join(SECRET_FILE), SECRET);

    let roots = space.roots();
    let target = space.outside.join(SECRET_FILE);
    let error = assert_refused(&roots, &target);

    let shown = format!("{error} {error:?}");
    let authorized = roots.roots()[0].display().to_string();
    assert!(
        shown.contains(&authorized),
        "the message has to say which directories are authorized: {shown}",
    );

    let refused = target.canonicalize().expect("the file is really there");
    for needle in [
        refused.display().to_string(),
        space
            .outside
            .canonicalize()
            .expect("B is really there")
            .display()
            .to_string(),
        SECRET_FILE.to_owned(),
        SECRET.to_owned(),
    ] {
        assert!(
            !shown.contains(&needle),
            "a refusal that repeats `{needle}` back confirms it exists",
        );
    }
}

#[test]
fn the_refused_tree_is_untouched() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);
    common::build_sample_tree(&space.outside);
    common::write_file(&space.outside.join(SECRET_FILE), SECRET);

    let roots = space.roots();
    let before_outside = common::snapshot(&space.outside);
    let before_authorized = common::snapshot(&space.authorized);
    assert!(!before_outside.is_empty());

    let targets: Vec<PathBuf> = vec![
        space.outside.clone(),
        space.outside.join(SECRET_FILE),
        space.outside.join("inbox"),
        space.authorized.join("..").join("b"),
        space.parent.path().to_path_buf(),
    ];
    let mut refused = 0usize;
    for target in &targets {
        assert_refused(&roots, target);
        refused += 1;
    }
    assert_eq!(refused, targets.len());

    assert_eq!(
        before_outside,
        common::snapshot(&space.outside),
        "being refused must not have been preceded by a look, or a write",
    );
    assert_eq!(before_authorized, common::snapshot(&space.authorized));
}

#[test]
fn an_authorized_request_still_passes() {
    let space = common::workspace();
    common::build_sample_tree(&space.authorized);
    common::write_file(&space.outside.join(SECRET_FILE), SECRET);

    let roots = space.roots();
    let report = soul_fileplan::scan(&roots, &space.authorized).expect("A is authorized");
    assert!(
        !report.is_empty(),
        "a matrix of refusals proves nothing if everything is refused"
    );

    let nested = soul_fileplan::scan(&roots, &space.authorized.join("inbox"))
        .expect("a directory inside an authorized root is authorized too");
    assert!(!nested.is_empty());
}
