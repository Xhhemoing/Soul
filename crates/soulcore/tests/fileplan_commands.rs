//! WP11's command surface against a real encrypted store: who may ask for a
//! scan, what the chain records about it, and what an approval is worth once
//! the plan has been edited.
//!
//! `soul-fileplan` already proves that an unauthorized path is refused. What
//! is checked here is the wiring around it — that the action check runs, that
//! both outcomes leave an audit entry, and that the entry carries a count and
//! an identifier and not one character of anybody's file name.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use soul_fileplan::{AuthorizedRoots, ScanReport};
use soul_policy::hitl::{
    check_action, ActionKind, ActionRequest, CapabilityScope, HitlDenial, PlanHash, RequestOrigin,
    TokenIssuer,
};
use soul_policy::ReasonCode;
use soul_schema::audit::{AuditAction, AuditDecision, SoulAuditEntry};
use soul_store_api::AuditLog;
use soul_testkit::leakage::LeakageChecker;
use soulcore::commands::fileplan::{plan_files, scan_directory, FilePlanRefusal};
use soulcore::commands::policy::PolicySession;
use soulcore::commands::store::{open_test_store, SqlCipherStore};

const SEED: &str = "soulcore fileplan commands";
const NOW_MS: u64 = 1_700_000_000_000;
const AT_UNIX: i64 = 1_700_000_000;

/// A name a file might really have, and a person it might really be about.
const OWNER_NAME: &str = "王小明";
const THIRD_PARTY_NAME: &str = "李雷";
const THIRD_PARTY_BODY: &str = "周五的场地我已经订好了，你直接过来就行";

/// The tree every test scans. Named after people on purpose: a file name is
/// the easiest way for somebody's name to end up somewhere it should not.
fn build_tree(root: &Path) -> Vec<String> {
    let files = [
        (format!("{OWNER_NAME}-会议记录.txt"), "会议记录".to_owned()),
        ("预算.csv".to_owned(), "item,amount\nsample,1\n".to_owned()),
        ("支出.csv".to_owned(), "item,amount\nsample,2\n".to_owned()),
        ("NOTES.MD".to_owned(), "# untidy extension\n".to_owned()),
        (
            format!("inbox/{THIRD_PARTY_NAME}的邮件.txt"),
            THIRD_PARTY_BODY.to_owned(),
        ),
    ];
    std::fs::create_dir_all(root.join("inbox")).expect("create the inbox directory");
    let mut names = Vec::new();
    for (relative, contents) in &files {
        std::fs::write(root.join(relative), contents).expect("create a file");
        names.push(
            Path::new(relative)
                .file_name()
                .expect("every fixture path has a name")
                .to_string_lossy()
                .into_owned(),
        );
    }
    names
}

#[derive(Debug)]
struct Fixture {
    store_dir: tempfile::TempDir,
    tree_dir: tempfile::TempDir,
    authorized: PathBuf,
    outside: PathBuf,
    names: Vec<String>,
}

fn fixture() -> (Fixture, SqlCipherStore, PolicySession) {
    let store_dir = tempfile::tempdir().expect("a directory for the store");
    let tree_dir = tempfile::tempdir().expect("a directory for the tree");
    let authorized = tree_dir.path().join("a");
    let outside = tree_dir.path().join("b");
    std::fs::create_dir_all(&authorized).expect("create the authorized directory");
    std::fs::create_dir_all(&outside).expect("create the unauthorized directory");
    std::fs::write(outside.join("private.txt"), THIRD_PARTY_BODY).expect("create a private file");
    let names = build_tree(&authorized);

    let store = open_test_store(store_dir.path(), SEED).expect("open the store");
    (
        Fixture {
            store_dir,
            tree_dir,
            authorized,
            outside,
            names,
        },
        store,
        PolicySession::closed(),
    )
}

impl Fixture {
    fn roots(&self) -> AuthorizedRoots {
        AuthorizedRoots::canonicalized(&[self.authorized.clone()]).expect("the root exists")
    }
}

fn scan(fixture: &Fixture, store: &mut SqlCipherStore, session: &mut PolicySession) -> ScanReport {
    scan_directory(
        session,
        store,
        &fixture.roots(),
        &fixture.authorized,
        RequestOrigin::User,
        NOW_MS,
        AT_UNIX,
    )
    .expect("the authorized directory can be scanned")
}

fn chain(store: &SqlCipherStore) -> Vec<SoulAuditEntry> {
    store.list_audit().expect("read the audit chain")
}

#[test]
fn a_scan_through_the_session_leaves_a_counted_audit_entry() {
    let (fixture, mut store, mut session) = fixture();

    let report = scan(&fixture, &mut store, &mut session);
    assert_eq!(report.file_count(), fixture.names.len());

    let entries = chain(&store);
    assert_eq!(entries.len(), 1, "one request, one entry");
    let entry = &entries[0];
    assert_eq!(entry.action, AuditAction::FilePlan);
    assert_eq!(entry.decision, AuditDecision::Allowed);
    assert_eq!(
        entry.reason_code.as_deref(),
        Some(ReasonCode::Routine.as_str())
    );

    // A uuid and a count. Nothing else is set, because nothing else about a
    // scan can be recorded without recording where somebody's files are.
    assert_eq!(entry.subject_refs.as_deref(), Some(&[report.scan_id()][..]));
    let counts = entry.counts.expect("the entry counts what it saw");
    assert_eq!(counts.items, Some(report.file_count() as u64));
    assert_eq!(counts.bytes, None);
    assert!(entry.plan_hash.is_none(), "a scan has no plan to approve");
    assert!(entry.capability_token_id.is_none(), "and needs no token");

    // A second scan of the same directory is a second entry with its own
    // identifier. The identifier names the scan and not the place: the
    // directory has a stable name too, and it is a digest of its path, which
    // is the one thing that must not reach the chain.
    let second = scan(&fixture, &mut store, &mut session);
    assert_ne!(second.scan_id(), report.scan_id());
    assert_eq!(second.root_fingerprint(), report.root_fingerprint());
    let entries = chain(&store);
    assert_eq!(entries.len(), 2);
    let serialized = serde_json::to_string(&entries).expect("serialize the chain");
    assert!(
        !serialized.contains(report.root_fingerprint()),
        "the chain must not carry a value that confirms a guess about a path",
    );
}

#[test]
fn a_refused_path_is_audited_as_path_not_authorized() {
    let (fixture, mut store, mut session) = fixture();

    let targets = [
        fixture.outside.clone(),
        fixture.outside.join("private.txt"),
        fixture.authorized.join("..").join("b"),
    ];
    for target in &targets {
        let refusal = scan_directory(
            &mut session,
            &mut store,
            &fixture.roots(),
            target,
            RequestOrigin::User,
            NOW_MS,
            AT_UNIX,
        )
        .expect_err("nothing outside the root may be scanned");
        assert_eq!(refusal.reason_code(), ReasonCode::PathNotAuthorized);
        assert!(matches!(refusal, FilePlanRefusal::Path(_)));
    }

    let entries = chain(&store);
    assert_eq!(entries.len(), targets.len(), "requests in, refusals out");
    for entry in &entries {
        assert_eq!(entry.action, AuditAction::FilePlan);
        assert_eq!(entry.decision, AuditDecision::Denied);
        assert_eq!(
            entry.reason_code.as_deref(),
            Some(ReasonCode::PathNotAuthorized.as_str()),
        );
        assert_eq!(entry.counts.and_then(|counts| counts.items), Some(1));
        assert!(
            entry.subject_refs.is_none(),
            "there is no directory to name"
        );
    }
}

#[test]
fn external_content_cannot_request_a_scan_or_plan() {
    let (fixture, mut store, mut session) = fixture();
    let report = scan(&fixture, &mut store, &mut session);
    let allowed_entries = chain(&store).len();

    let refusal = scan_directory(
        &mut session,
        &mut store,
        &fixture.roots(),
        &fixture.authorized,
        RequestOrigin::ExternalContent,
        NOW_MS,
        AT_UNIX,
    )
    .expect_err("a scan asked for by scanned content is not a scan the user asked for");
    assert_eq!(
        refusal.reason_code(),
        ReasonCode::ExternalContentNotAuthority,
    );

    let refusal = plan_files(
        &mut session,
        &mut store,
        &report,
        None,
        RequestOrigin::ExternalContent,
        NOW_MS,
        AT_UNIX,
    )
    .expect_err("nor a plan");
    assert!(matches!(
        refusal,
        FilePlanRefusal::Hitl(HitlDenial::ExternalContentNotAuthority),
    ));

    let entries = chain(&store);
    assert_eq!(entries.len(), allowed_entries + 2);
    for entry in &entries[allowed_entries..] {
        assert_eq!(entry.decision, AuditDecision::Denied);
        assert_eq!(
            entry.reason_code.as_deref(),
            Some(ReasonCode::ExternalContentNotAuthority.as_str()),
        );
    }
}

#[test]
fn an_edited_plan_fails_the_approved_hash() {
    let (fixture, mut store, mut session) = fixture();
    let report = scan(&fixture, &mut store, &mut session);

    // The control: the plan the user was shown, approved as it stands.
    let preview = soul_fileplan::plan(&report);
    let honest = preview.to_plan_json();
    let approved = plan_files(
        &mut session,
        &mut store,
        &report,
        Some(&PlanHash::of(&honest)),
        RequestOrigin::User,
        NOW_MS,
        AT_UNIX,
    )
    .expect("the plan is the one that was approved");
    assert_eq!(approved.to_plan_json(), honest);
    assert!(!approved.written_to_disk());
    assert!(!approved.is_empty(), "the tree has something to suggest");

    // A target changed after approval.
    let mut retargeted = honest.clone();
    retargeted["entries"][0]["target_rel"] = serde_json::json!("somewhere-else/the-same-file.txt");
    assert_ne!(retargeted, honest);

    // The order changed after approval, and nothing else.
    let mut reordered = honest.clone();
    let list = reordered["entries"]
        .as_array_mut()
        .expect("the plan lists its entries");
    list.reverse();
    assert_ne!(reordered, honest);

    for edited in [retargeted, reordered] {
        let refusal = plan_files(
            &mut session,
            &mut store,
            &report,
            Some(&PlanHash::of(&edited)),
            RequestOrigin::User,
            NOW_MS,
            AT_UNIX,
        )
        .expect_err("an edited plan is not the plan that was approved");
        assert_eq!(refusal.reason_code(), ReasonCode::PlanHashMismatch);
        assert!(matches!(
            refusal,
            FilePlanRefusal::Hitl(HitlDenial::PlanHashMismatch { .. }),
        ));
    }

    let entries = chain(&store);
    let denied: Vec<&SoulAuditEntry> = entries
        .iter()
        .filter(|entry| entry.decision == AuditDecision::Denied)
        .collect();
    assert_eq!(denied.len(), 2);
    let allowed: Vec<&SoulAuditEntry> = entries
        .iter()
        .filter(|entry| {
            entry.action == AuditAction::FilePlan
                && entry.decision == AuditDecision::Allowed
                && entry.plan_hash.is_some()
        })
        .collect();
    assert_eq!(allowed.len(), 1, "only the approved plan was recorded");
    assert_eq!(
        allowed[0].plan_hash.as_ref().map(|hash| hash.as_str()),
        Some(PlanHash::of(&honest).as_str()),
    );
}

#[test]
fn a_file_write_token_buys_no_execution() {
    let (fixture, mut store, mut session) = fixture();
    let report = scan(&fixture, &mut store, &mut session);
    let preview = soul_fileplan::plan(&report);
    let plan_hash = PlanHash::of(&preview.to_plan_json());

    let refused = session
        .issue_token(CapabilityScope::FileWrite, plan_hash.clone(), NOW_MS)
        .expect_err("v0.1 has no write implementation to hand a token to");
    assert_eq!(refused.reason, ReasonCode::WriteNotImplemented);

    // And there was never a ledger to consult: neither action needs a token,
    // so the whole path runs with an issuer that stays empty.
    assert!(!ActionKind::ScanDirectory.needs_capability_token());
    assert!(!ActionKind::PlanFiles.needs_capability_token());

    let mut issuer = TokenIssuer::new();
    check_action(
        &mut issuer,
        &ActionRequest::new(ActionKind::ScanDirectory.as_str(), RequestOrigin::User),
        NOW_MS,
    )
    .expect("a scan needs no token");
    let approved = check_action(
        &mut issuer,
        &ActionRequest::new(ActionKind::PlanFiles.as_str(), RequestOrigin::User)
            .with_plan(preview.to_plan_json())
            .approved_as(plan_hash),
        NOW_MS,
    )
    .expect("a plan needs no token either");
    assert!(approved.spent_token.is_none());
    assert_eq!(issuer.issued_count(), 0);

    // There is no way to carry the plan out even now: the preview is the
    // whole of what this build produces, and it says so.
    assert!(!preview.written_to_disk());
}

#[test]
fn the_audit_chain_carries_no_file_name() {
    let (fixture, mut store, mut session) = fixture();

    let report = scan(&fixture, &mut store, &mut session);
    let preview = plan_files(
        &mut session,
        &mut store,
        &report,
        None,
        RequestOrigin::User,
        NOW_MS,
        AT_UNIX,
    )
    .expect("a plan the user asked for");
    scan_directory(
        &mut session,
        &mut store,
        &fixture.roots(),
        &fixture.outside,
        RequestOrigin::User,
        NOW_MS,
        AT_UNIX,
    )
    .expect_err("and one refusal, so the chain has both shapes in it");

    store
        .verify_audit_chain()
        .expect("the chain links up before anything is read out of it");
    let entries = chain(&store);
    assert_eq!(entries.len(), 3);
    let serialized = serde_json::to_string(&entries).expect("serialize the whole chain");

    // The corpus is everything the scan saw, plus the people the names are
    // about. A single one of these surviving into the chain is a leak: the
    // chain outlives the rows it names, and forgetting cannot reach it.
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("their-message", THIRD_PARTY_BODY);
    checker.add_known_identifier("owner-name", OWNER_NAME);
    checker.add_known_identifier("their-name", THIRD_PARTY_NAME);
    for name in &fixture.names {
        checker.add_known_identifier(format!("file-name:{name}"), name);
    }
    checker.assert_clean("the audit chain", &serialized);

    // The reverse: the preview a user is shown does contain their file
    // names, because it is about their own disk. It is the chain that may
    // not, and the two come from the same scan.
    let rendered = preview.render();
    let shown: BTreeSet<bool> = fixture
        .names
        .iter()
        .map(|name| rendered.contains(name))
        .collect();
    assert!(
        shown.contains(&true),
        "a preview that named none of the files would be no preview at all",
    );
    assert!(
        !fixture.names.is_empty() && !preview.is_empty(),
        "and there has to be something in it to check",
    );

    // The store directory and the scanned tree are different places; nothing
    // in this test wrote to the tree.
    assert!(fixture.store_dir.path().exists());
    assert!(fixture.tree_dir.path().exists());
}
