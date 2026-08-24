//! WP09's file page, from the core's side: the preview it shows, and the four
//! ways it refuses.
//!
//! `fileplan_commands.rs` already proves the scan and the plan — the action
//! check, the audit entries, the approval hash, and that no file name reaches
//! the chain. What is checked here is the layer the WebView calls: that the
//! roots come from the session rather than from the caller, that a refusal
//! names the directories the user did authorize and never the one they asked
//! about, and that the value crossing the IPC says `written_to_disk: false`
//! because there is nothing in this build that could make it say otherwise.

use std::path::{Path, PathBuf};

use soul_policy::ReasonCode;
use soul_schema::audit::{AuditAction, AuditDecision, SoulAuditEntry};
use soul_store_api::AuditLog;
use soulcore::commands::collect::share;
use soulcore::commands::fileplan::{
    fileplan_view, GROUP_ACTION_LABEL, MOVE_ACTION_LABEL, PLAN_PREVIEW_ONLY_EXPLANATION,
    RENAME_ACTION_LABEL,
};
use soulcore::commands::shell::{Session, ViewRefusedReason, NO_STORE_FOR_VIEW_EXPLANATION};
use soulcore::commands::store::{open_test_store, StoreSlot};

const SEED: &str = "soulcore fileplan view";

/// A name a file might really have, and a person it might really be about.
const OWNER_NAME: &str = "王小明";
const THIRD_PARTY_NAME: &str = "李雷";
const THIRD_PARTY_BODY: &str = "周五的场地我已经订好了，你直接过来就行";

/// The tree every test scans, built the way `fileplan_commands.rs` builds one:
/// two siblings that want grouping, a loose file that wants moving, and an
/// upper-case extension that wants renaming, so every action label is exercised.
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
    _store_dir: tempfile::TempDir,
    _tree_dir: tempfile::TempDir,
    slot: StoreSlot,
    session: Session,
    authorized: PathBuf,
    outside: PathBuf,
}

/// A real database, a real tree, and a session with exactly one directory
/// authorised — which is the state the settings page leaves behind.
fn fixture() -> Fixture {
    let store_dir = tempfile::tempdir().expect("a directory for the store");
    let tree_dir = tempfile::tempdir().expect("a directory for the tree");
    let authorized = tree_dir.path().join("a");
    let outside = tree_dir.path().join("b");
    std::fs::create_dir_all(&authorized).expect("create the authorized directory");
    std::fs::create_dir_all(&outside).expect("create the unauthorized directory");
    std::fs::write(outside.join("private.txt"), THIRD_PARTY_BODY).expect("create a private file");
    build_tree(&authorized);

    let slot = StoreSlot::default();
    let store = open_test_store(store_dir.path(), SEED).expect("open the store");
    assert!(slot.install(share(store)), "an empty slot accepts a handle");

    let session = Session::new();
    session
        .authorize_root(authorized.to_str().expect("a utf-8 temporary path"))
        .expect("the directory exists, so the user may authorise it");

    Fixture {
        _store_dir: store_dir,
        _tree_dir: tree_dir,
        slot,
        session,
        authorized,
        outside,
    }
}

impl Fixture {
    fn chain(&self) -> Vec<SoulAuditEntry> {
        self.slot
            .lock()
            .expect("the slot is filled")
            .list_audit()
            .expect("read the audit chain")
    }
}

/// The mock runtime's answer, in the same words the draft page gets.
#[test]
fn a_plan_without_a_store_is_refused_in_the_core_s_own_words() {
    let refused = fileplan_view(&StoreSlot::default(), &Session::new(), "/tmp".to_owned())
        .expect_err("there is no database to record the request in");

    assert_eq!(refused.reason, ViewRefusedReason::NoStoreOpened);
    assert_eq!(refused.message, NO_STORE_FOR_VIEW_EXPLANATION);
    assert!(
        refused.code.is_none(),
        "no gate decided this; none was open"
    );
    assert_eq!(refused.to_string(), refused.message);

    let value = serde_json::to_value(&refused).expect("serialize");
    assert_eq!(value["reason"], serde_json::json!("no_store_opened"));
    assert_eq!(value["code"], serde_json::Value::Null);
    assert_eq!(value.as_object().expect("an object").len(), 3);
}

/// The one success the page is for: a tree that has something to suggest, and
/// a preview that says out loud it changed nothing.
#[test]
fn an_authorised_directory_produces_a_preview_that_touches_nothing() {
    let fixture = fixture();

    let view = fileplan_view(
        &fixture.slot,
        &fixture.session,
        fixture.authorized.display().to_string(),
    )
    .expect("the directory the user authorised can be scanned");

    let value = serde_json::to_value(&view).expect("serialize");
    assert_eq!(value["written_to_disk"], serde_json::json!(false));
    assert_eq!(
        value["notice"],
        serde_json::json!(PLAN_PREVIEW_ONLY_EXPLANATION),
        "the page renders this sentence verbatim; it must come from here",
    );
    assert_eq!(value["file_count"], serde_json::json!(5));
    assert_eq!(value["dir_count"], serde_json::json!(1), "inbox");
    assert_eq!(value["skipped_escaping_links"], serde_json::json!(0));

    let entries = value["entries"].as_array().expect("a list of suggestions");
    assert!(
        !entries.is_empty(),
        "a tree with two loose spreadsheets in it has something to suggest",
    );
    assert_eq!(
        value["entry_count"],
        serde_json::json!(entries.len()),
        "the count and the list are two readings of one plan",
    );

    // The three counts partition the list, and each entry's label is the word
    // this crate chose for its action rather than one the WebView invented.
    let mut counted = [0usize; 3];
    for entry in entries {
        let action = entry["action"].as_str().expect("an action");
        let label = entry["action_label"].as_str().expect("a label");
        match action {
            "group" => {
                counted[0] += 1;
                assert_eq!(label, GROUP_ACTION_LABEL);
            }
            "move" => {
                counted[1] += 1;
                assert_eq!(label, MOVE_ACTION_LABEL);
            }
            "rename" => {
                counted[2] += 1;
                assert_eq!(label, RENAME_ACTION_LABEL);
            }
            other => panic!("the plan proposed something with no label: {other}"),
        }
        assert!(!entry["source_rel"].as_str().expect("a source").is_empty());
        assert!(
            entry["target_rel"].as_str().is_some(),
            "every suggestion this build makes says where it would put the file",
        );
    }
    assert_eq!(value["group_count"], serde_json::json!(counted[0]));
    assert_eq!(value["move_count"], serde_json::json!(counted[1]));
    assert_eq!(value["rename_count"], serde_json::json!(counted[2]));
    assert_eq!(
        counted.iter().sum::<usize>(),
        entries.len(),
        "the three counts have to add up to the list, or one of them is wrong",
    );

    // A scan and a plan: two requests, two entries, both allowed.
    let chain = fixture.chain();
    assert_eq!(chain.len(), 2, "one scan, one plan, and nothing else");
    for entry in &chain {
        assert_eq!(entry.action, AuditAction::FilePlan);
        assert_eq!(entry.decision, AuditDecision::Allowed);
    }
}

/// The refusal matrix a page can actually reach, and the property that makes
/// each refusal safe to show: the path that was asked about is not in it.
#[test]
fn a_directory_outside_the_authorised_roots_is_refused_without_being_named() {
    let fixture = fixture();
    let outside = fixture.outside.display().to_string();

    for target in [
        outside.clone(),
        fixture.outside.join("private.txt").display().to_string(),
        fixture
            .authorized
            .join("..")
            .join("b")
            .display()
            .to_string(),
    ] {
        let refused = match fileplan_view(&fixture.slot, &fixture.session, target.clone()) {
            Ok(view) => panic!("{target} was scanned: {view:?}"),
            Err(refused) => refused,
        };
        assert_eq!(refused.reason, ViewRefusedReason::Refused, "for {target}");
        assert_eq!(
            refused.code.as_deref(),
            Some(ReasonCode::PathNotAuthorized.as_str()),
            "for {target}",
        );
        assert!(
            !refused.message.contains(&outside),
            "the refusal repeated the path it refused, which answers the question it was \
             refusing to answer: {refused}",
        );
        assert!(
            refused.message.contains(
                &fixture
                    .authorized
                    .canonicalize()
                    .expect("the authorised root resolves")
                    .display()
                    .to_string()
            ),
            "a refusal has to say what the user may look at instead: {refused}",
        );
    }

    // One request in, one refusal recorded, every time.
    let chain = fixture.chain();
    assert_eq!(chain.len(), 3);
    for entry in &chain {
        assert_eq!(entry.decision, AuditDecision::Denied);
        assert_eq!(
            entry.reason_code.as_deref(),
            Some(ReasonCode::PathNotAuthorized.as_str()),
        );
    }
}

/// The shipped default authorises nothing, and nothing has to be a refusal
/// rather than a scan of whatever was asked for.
#[test]
fn a_session_with_no_authorised_root_scans_nothing_at_all() {
    let fixture = fixture();
    let fresh = Session::new();
    assert!(fresh.authorized_roots().is_empty());

    let refused = fileplan_view(
        &fixture.slot,
        &fresh,
        fixture.authorized.display().to_string(),
    )
    .expect_err("a directory nobody authorised is a directory nobody may read");

    assert_eq!(refused.reason, ViewRefusedReason::Refused);
    assert_eq!(
        refused.code.as_deref(),
        Some(ReasonCode::PathNotAuthorized.as_str()),
    );
    assert!(
        !refused
            .message
            .contains(&fixture.authorized.display().to_string()),
        "an empty root list still must not confirm a path: {refused}",
    );
}

/// The field set is the contract with `apps/desktop/src/core.ts`, so it is
/// written down here rather than left to whoever reads the struct next.
#[test]
fn a_file_plan_view_carries_these_fields_and_no_others() {
    let fixture = fixture();
    let view = fileplan_view(
        &fixture.slot,
        &fixture.session,
        fixture.authorized.display().to_string(),
    )
    .expect("a preview the page can show");

    let value = serde_json::to_value(&view).expect("serialize");
    let mut fields: Vec<&str> = value
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    fields.sort_unstable();
    assert_eq!(
        fields,
        vec![
            "dir_count",
            "entries",
            "entry_count",
            "file_count",
            "group_count",
            "move_count",
            "notice",
            "rename_count",
            "scan_id",
            "skipped_escaping_links",
            "written_to_disk",
        ],
    );

    let mut entry_fields: Vec<&str> = value["entries"][0]
        .as_object()
        .expect("a suggestion is an object")
        .keys()
        .map(String::as_str)
        .collect();
    entry_fields.sort_unstable();
    assert_eq!(
        entry_fields,
        vec!["action", "action_label", "source_rel", "target_rel"],
    );
}
