//! AC-18, the half that is a percentage: everything about the directory nobody
//! authorized is refused, however it is spelled.
//!
//! "One hundred percent" is a claim about the spellings nobody thought of, so
//! the corpus here is built by taking each way one path can be made to name
//! another and applying it to the unauthorized directory: parent segments,
//! symbolic links, mixed separators, case, UNC and verbatim prefixes, device
//! names, trailing dots, alternate data streams. Every candidate is refused,
//! and the count is asserted so that a corpus which quietly shrank would fail
//! rather than pass more easily.
//!
//! Several of the spellings are Windows-only ways of naming a file. They are
//! refused on Linux too, which is the point: `\\?\C:\Bravo` is a string that
//! reaches a device on the platform Soul ships to and a relative name on the
//! platform its tests run on, and the screen works on the string.
//!
//! Two controls keep this from being a test of a function that says no to
//! everything. `paths_under_the_authorized_root_are_still_accepted` asserts the
//! other side, and `authorized_scan.rs` asserts that a real plan comes out.

mod common;

use common::{walk, Tree};

use soul_fileplan::{Authorization, PathMatching, Refusal, ScanLimits};
use soul_policy::hitl::{RequestOrigin, TokenIssuer};
use soul_policy::ReasonCode;

const NOW_MS: u64 = 1_787_529_600_000;

fn authorized(tree: &Tree) -> Authorization {
    let mut authorization = Authorization::new();
    authorization
        .authorize(&tree.alpha())
        .expect("authorize Alpha");
    authorization
}

/// Every spelling that must not be read, with the trick it uses.
fn refusable(tree: &Tree) -> Vec<(&'static str, String)> {
    let alpha = tree.alpha();
    let bravo = tree.bravo();
    let base = tree.base().to_string_lossy().into_owned();

    let corpus = vec![
        ("the unauthorized directory itself", bravo.clone()),
        ("a file in it", format!("{bravo}/secret.txt")),
        ("a file below it", format!("{bravo}/nested/deep.txt")),
        ("its parent", base.clone()),
        (
            "out and back in with ..",
            format!("{alpha}/../Bravo/secret.txt"),
        ),
        ("out twice", format!("{alpha}/sub/../../Bravo/secret.txt")),
        ("a bare .. at the end", format!("{alpha}/..")),
        ("a . segment on the way", format!("{alpha}/./../Bravo")),
        (
            "backslashes instead of slashes",
            format!("{alpha}\\..\\Bravo\\secret.txt"),
        ),
        ("a verbatim prefix", format!("\\\\?\\{bravo}")),
        ("a device prefix", format!("\\\\.\\{bravo}")),
        ("a UNC share", r"\\server\share\secret.txt".to_owned()),
        (
            "a UNC share with forward slashes",
            "//server/share/secret.txt".to_owned(),
        ),
        ("a drive-relative path", r"\Bravo\secret.txt".to_owned()),
        (
            "a drive-current-directory path",
            r"C:Bravo\secret.txt".to_owned(),
        ),
        ("a relative path", "Bravo/secret.txt".to_owned()),
        ("nothing at all", String::new()),
        ("a double separator", format!("{alpha}//taken.png")),
        (
            "a trailing dot, which Windows strips",
            format!("{bravo}/secret.txt."),
        ),
        (
            "a trailing space, which Windows strips",
            format!("{bravo}/secret.txt "),
        ),
        (
            "an alternate data stream",
            format!("{alpha}/taken.png:hidden"),
        ),
        ("a reserved device name", format!("{alpha}/NUL")),
        (
            "a reserved device name with a suffix",
            format!("{alpha}/con.txt"),
        ),
        ("a control character", format!("{alpha}/photo\u{7}.jpg")),
        (
            "the authorized root's name as a prefix of another",
            format!("{alpha}Extra/secret.txt"),
        ),
    ];

    #[cfg(unix)]
    let corpus = {
        let mut corpus = corpus;
        corpus.push(("a sibling that was never named", tree.lower_alpha()));
        corpus.push((
            "a file in that sibling",
            format!("{}/decoy.txt", tree.lower_alpha()),
        ));
        corpus.push((
            "a link out of the authorized root",
            format!("{alpha}/escape/secret.txt"),
        ));
        corpus.push(("the link itself", format!("{alpha}/escape")));
        corpus.push((
            "a link that stays inside it",
            format!("{alpha}/loopback/inner.md"),
        ));
        corpus
    };

    corpus
}

#[test]
fn every_way_of_naming_the_unauthorized_directory_is_refused() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let corpus = refusable(&tree);

    assert!(
        corpus.len() >= 25,
        "the corpus shrank to {} entries",
        corpus.len(),
    );

    let mut accepted = Vec::new();
    for (trick, candidate) in &corpus {
        match authorization.resolve(candidate) {
            Err(refusal) => assert!(
                refusal.is_authorization_refusal(),
                "{trick}: `{candidate}` was refused for the wrong reason: {refusal}",
            ),
            Ok(resolution) => accepted.push(format!(
                "{trick}: `{candidate}` resolved to {}",
                resolution.canonical().display(),
            )),
        }
    }
    assert!(
        accepted.is_empty(),
        "an unauthorized path was accepted: {accepted:#?}",
    );
}

/// The same corpus, through the door the desktop shell uses. `resolve` is the
/// decision, but a preview that reached the disk before consulting it would
/// still be a leak.
#[test]
fn the_preview_refuses_the_same_corpus_and_leaves_the_disk_alone() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let mut issuer = TokenIssuer::new();

    let before = walk(tree.base());
    for (trick, candidate) in refusable(&tree) {
        let outcome = soul_fileplan::preview(
            &authorization,
            &mut issuer,
            &candidate,
            RequestOrigin::User,
            ScanLimits::default(),
            NOW_MS,
        );
        let Err(refusal) = outcome else {
            panic!("{trick}: `{candidate}` produced a preview");
        };
        assert!(refusal.is_authorization_refusal(), "{trick}: {refusal}");
        assert_eq!(refusal.reason_code(), ReasonCode::ConsentMissing);
    }
    assert_eq!(
        before,
        walk(tree.base()),
        "a refused preview changed something on disk",
    );
    assert_eq!(issuer.issued_count(), 0);
}

/// The control. Without it, a `resolve` that returned an error unconditionally
/// would pass everything above.
#[test]
fn paths_under_the_authorized_root_are_still_accepted() {
    let tree = Tree::build();
    let authorization = authorized(&tree);

    for candidate in [
        tree.alpha(),
        format!("{}/", tree.alpha()),
        format!("{}/photo.jpg", tree.alpha()),
        format!("{}/笔记.txt", tree.alpha()),
        format!("{}/sub", tree.alpha()),
        format!("{}/sub/inner.md", tree.alpha()),
        format!("{}/图片/already.png", tree.alpha()),
        // Not there yet, and still inside: this is what a destination is.
        format!("{}/文档/report.pdf", tree.alpha()),
    ] {
        authorization
            .resolve(&candidate)
            .unwrap_or_else(|refusal| panic!("`{candidate}` should be readable: {refusal}"));
    }
}

/// Before anything is authorized, everything is refused — including the
/// directory that is about to be authorized. Collection defaults to off in
/// WP07 for the same reason.
#[test]
fn an_authorization_that_names_nothing_refuses_everything() {
    let tree = Tree::build();
    let empty = Authorization::new();
    assert!(empty.is_empty());

    for candidate in [tree.alpha(), tree.bravo(), "/".to_owned()] {
        assert_eq!(empty.resolve(&candidate), Err(Refusal::NothingAuthorized));
    }
}

/// Case is a platform question, so it is answered per platform rather than
/// guessed at. Both answers are checked here, on whichever machine is running.
///
/// Folding case makes containment match *more* often, which is why the default
/// is the platform's real behaviour: applying NTFS's rule to a case-sensitive
/// filesystem would put a directory nobody authorized inside a root.
#[test]
fn case_is_decided_by_the_filesystem_rather_than_assumed() {
    let tree = Tree::build();

    // Unix: a third directory named `alpha`. Windows: the same directory as
    // `Alpha`, probed through the other spelling, at a file that is actually
    // there (`photo.jpg`). NTFS cannot hold both spellings at once.
    let case_probe = if cfg!(windows) {
        format!("{}/photo.jpg", tree.lower_alpha())
    } else {
        format!("{}/decoy.txt", tree.lower_alpha())
    };

    let mut exact = Authorization::with_matching(PathMatching::Exact);
    exact.authorize(&tree.alpha()).expect("authorize Alpha");
    assert!(
        exact.resolve(&case_probe).is_err(),
        "on a case-sensitive filesystem `alpha` is a third directory, not `Alpha`",
    );

    let mut folded = Authorization::with_matching(PathMatching::CaseFolded);
    folded.authorize(&tree.alpha()).expect("authorize Alpha");
    // Under NTFS's rule the two spellings are one directory, so this one is
    // accepted — and that is correct, because on Windows it is the same place.
    assert!(folded.resolve(&case_probe).is_ok());

    // What neither rule may do is let the unauthorized directory in, in any
    // case, which is the part AC-18 is actually about.
    for matching in [PathMatching::Exact, PathMatching::CaseFolded] {
        let mut authorization = Authorization::with_matching(matching);
        authorization
            .authorize(&tree.alpha())
            .expect("authorize Alpha");
        for candidate in [
            tree.bravo(),
            format!("{}/secret.txt", tree.bravo()),
            tree.bravo().to_uppercase(),
            tree.bravo().to_lowercase(),
            format!("{}/SECRET.TXT", tree.bravo()),
        ] {
            assert!(
                authorization.resolve(&candidate).is_err(),
                "{matching:?} accepted `{candidate}`",
            );
        }
    }

    assert_eq!(
        PathMatching::for_this_platform(),
        match cfg!(windows) {
            true => PathMatching::CaseFolded,
            false => PathMatching::Exact,
        },
    );
}

/// Authorizing is itself gated. A file, a link, or something that is not there
/// cannot become a root.
#[test]
fn only_a_real_directory_can_become_a_root() {
    let tree = Tree::build();
    let mut authorization = Authorization::new();

    let file = format!("{}/photo.jpg", tree.alpha());
    assert!(matches!(
        authorization.authorize(&file),
        Err(Refusal::NotADirectory { .. }),
    ));

    let missing = format!("{}/nowhere", tree.alpha());
    assert!(matches!(
        authorization.authorize(&missing),
        Err(Refusal::Unreadable { .. }),
    ));

    assert!(matches!(
        authorization.authorize("Alpha"),
        Err(Refusal::Malformed(_)),
    ));

    #[cfg(unix)]
    {
        // Authorizing a link would mean consenting to a name rather than to a
        // place: repoint the link and the authorization follows it.
        assert!(matches!(
            authorization.authorize(&format!("{}/escape", tree.alpha())),
            Err(Refusal::SymlinkNotFollowed { .. }),
        ));
    }

    assert!(
        authorization.is_empty(),
        "a refused authorization must not leave a root behind",
    );
    authorization
        .authorize(&tree.alpha())
        .expect("the real thing");
    assert_eq!(authorization.roots().len(), 1);
    // Authorizing the same place twice is one root, not two.
    authorization.authorize(&tree.alpha()).expect("again");
    assert_eq!(authorization.roots().len(), 1);
}

/// External content cannot ask for a directory to be scanned, whatever
/// directory it names. This fires before the path is looked at, so a file name
/// that contains an authorized path is still not authority.
#[test]
fn a_request_that_came_out_of_a_file_cannot_scan_anything() {
    let tree = Tree::build();
    let authorization = authorized(&tree);
    let mut issuer = TokenIssuer::new();

    for candidate in [tree.alpha(), tree.bravo()] {
        let refusal = soul_fileplan::preview(
            &authorization,
            &mut issuer,
            &candidate,
            RequestOrigin::ExternalContent,
            ScanLimits::default(),
            NOW_MS,
        )
        .expect_err("external content is never authority");
        assert_eq!(
            refusal.reason_code(),
            ReasonCode::ExternalContentNotAuthority,
        );
    }
}

/// The two containment checks are independent, and the second one is not
/// redundant.
///
/// A link out of the root is inside it by every lexical measure — the string
/// starts with the root, and there is no `..` anywhere — and outside it once
/// the operating system has resolved the name. Without the canonical check the
/// only thing standing between a scan and the rest of the disk would be the
/// symlink walk, and one net is not two.
#[cfg(unix)]
#[test]
fn the_lexical_and_canonical_checks_disagree_about_a_link_on_purpose() {
    let tree = Tree::build();
    let mut authorization = Authorization::with_matching(PathMatching::Exact);
    let root = authorization
        .authorize(&tree.alpha())
        .expect("authorize Alpha");

    let through_the_link = soul_fileplan::screen(&format!("{}/escape/secret.txt", tree.alpha()))
        .expect("the spelling itself is fine");
    assert!(
        PathMatching::Exact.contains_screened(root.requested(), &through_the_link),
        "lexically the link is inside the root, which is why the second check exists",
    );

    let resolved =
        std::fs::canonicalize(through_the_link.to_path_buf()).expect("the link resolves");
    assert!(
        !PathMatching::Exact.contains_canonical(root.canonical(), &resolved),
        "canonically it is somewhere else entirely",
    );
}

/// A refusal is auditable, and the audit entry says nothing about where the
/// user was pointed. The chain records that a file plan was denied and why.
#[test]
fn a_refusal_audits_without_naming_the_path() {
    let tree = Tree::build();
    let authorization = authorized(&tree);

    let candidate = format!("{}/secret.txt", tree.bravo());
    let refusal = authorization.resolve(&candidate).expect_err("refused");
    let entry = refusal
        .audit()
        .into_entry(uuid::Uuid::from_u128(1), 1_787_529_600)
        .expect("an audit entry with no prose in it");

    let serialized = serde_json::to_string(&entry).expect("serialize");
    assert!(serialized.contains("CONSENT_MISSING"));
    for needle in [
        "secret",
        "Bravo",
        "Alpha",
        tree.base().to_string_lossy().as_ref(),
    ] {
        assert!(
            !serialized.contains(needle),
            "the audit entry repeated `{needle}`: {serialized}",
        );
    }
}
