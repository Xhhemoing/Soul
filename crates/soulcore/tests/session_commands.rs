//! WP13, second slice: the one open store, and the two answers that survive a
//! restart.
//!
//! The interesting assertions here are about the second launch. Everything the
//! shell showed in WP09 came out of `Config::default()`, so "nothing is
//! switched on" was true by construction and meant very little. Now there is a
//! file on disk, and the question AC-02 actually asks — *can a restart reopen
//! collection, the cloud or a model endpoint?* — has a place where the answer
//! could go wrong. Several tests below reopen the directory and look.

use std::path::{Path, PathBuf};

use soulcore::commands::draft::Approval;
use soulcore::commands::session::{
    read_stored_config, Session, StoredConfig, CONFIG_FILE_NAME, KEY_BLOB_FILE_NAME,
};
use soulcore::commands::shell::WizardAnswers;

const PASTED: &str = "周五那个方案你还改吗？我这边可以等到下午三点。";

fn acknowledged() -> WizardAnswers {
    WizardAnswers {
        acknowledged_defaults_are_off: true,
    }
}

/// A scratch directory, canonical so the file-plan screening — which refuses
/// symlinked paths on purpose — sees the same spelling twice.
fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

fn a_folder_worth_tidying(root: &Path) -> String {
    std::fs::write(root.join("budget.csv"), "a,b\n1,2\n").expect("write");
    std::fs::write(root.join("photo.jpg"), "jpeg-ish").expect("write");
    std::fs::write(root.join("mystery.qqq"), "?").expect("write");
    root.to_string_lossy().into_owned()
}

fn config_text(directory: &Path) -> String {
    std::fs::read_to_string(directory.join(CONFIG_FILE_NAME)).expect("the configuration file")
}

/// A first launch: the store opens, the wizard has not been through, and
/// nothing has been written down yet.
#[test]
fn a_first_launch_opens_the_store_and_writes_nothing_of_its_own() {
    let (keep, directory) = scratch();
    let session = Session::open(&directory);
    let status = session.status();

    assert!(!status.wizard_completed, "nobody has finished it yet");
    assert!(status.store_opened, "{}", status.store_notice);
    assert!(status.config_problem.is_none());
    assert!(
        !directory.join(CONFIG_FILE_NAME).exists(),
        "opening a session must not write a configuration nobody asked for",
    );
    assert!(
        directory.join("soul.db").exists(),
        "the store is the one thing opening a session does create",
    );

    let snapshot = session.snapshot();
    assert!(snapshot.fully_closed);
    assert_eq!(snapshot.authorized_root_count, 0);
    drop(keep);
}

/// Both callers of `store()` hold the same connection. Two would be two
/// write-ahead logs — WP07 leftover 8, which is why this type has no second
/// entry point that opens one.
#[test]
fn the_session_hands_out_one_handle_to_one_store() {
    let (keep, directory) = scratch();
    let session = Session::open(&directory);

    let first = session.store().expect("the store opened");
    let second = session.store().expect("the store opened");
    assert!(
        std::sync::Arc::ptr_eq(&first, &second),
        "two calls produced two stores",
    );

    // And the source says so: the shipped path goes through `open_store` with
    // a platform provider, exactly once, and never through the test one.
    const SOURCE: &str = include_str!("../src/commands/session.rs");
    assert_eq!(
        SOURCE.matches("store_commands::open_store(").count(),
        1,
        "the session opens a store in more than one place",
    );
    assert!(
        !SOURCE.contains("open_test_store"),
        "test key material must not be the shipped default",
    );
    drop(keep);
}

/// The wizard is the one thing a first launch asks, and the second launch must
/// not ask it again.
#[test]
fn finishing_the_wizard_survives_a_restart() {
    let (keep, directory) = scratch();

    let mut first = Session::open(&directory);
    let snapshot = first
        .complete_wizard(&acknowledged())
        .expect("the wizard finishes");
    assert!(snapshot.fully_closed);
    assert!(first.status().wizard_completed);
    drop(first);

    let second = Session::open(&directory);
    let status = second.status();
    assert!(status.wizard_completed, "the wizard was asked twice");
    assert!(status.config_problem.is_none());

    // AC-02 on the second launch: still nothing on.
    let snapshot = second.snapshot();
    assert!(snapshot.fully_closed);
    assert!(snapshot.open_capabilities.is_empty());
    assert!(!snapshot.collect_enabled);
    assert!(!snapshot.cloud.enabled);
    assert!(!snapshot.llm_endpoint_configured);
    drop(keep);
}

/// A wizard that did not finish leaves nothing behind, so the next launch asks
/// again rather than skipping a question the user never answered.
#[test]
fn an_unacknowledged_wizard_writes_nothing_down() {
    let (keep, directory) = scratch();

    let mut session = Session::open(&directory);
    session
        .complete_wizard(&WizardAnswers::default())
        .expect_err("an unacknowledged wizard does not finish");
    assert!(!directory.join(CONFIG_FILE_NAME).exists());
    drop(session);

    assert!(!Session::open(&directory).status().wizard_completed);
    drop(keep);
}

/// AC-02, in the form the file makes possible to check: there is no field in
/// it that could say a capability is on.
#[test]
fn nothing_a_configuration_file_can_say_switches_a_capability_on() {
    let (keep, directory) = scratch();

    let mut session = Session::open(&directory);
    session
        .complete_wizard(&acknowledged())
        .expect("the wizard finishes");
    drop(session);

    let written = config_text(&directory);
    for capability in ["collect_enabled", "cloud_enabled", "llm_endpoint"] {
        assert!(
            !written.contains(capability),
            "the file this build writes names `{capability}`: {written}",
        );
    }

    // A file that names one anyway does not load: `StoredConfig` denies
    // unknown fields, so the session reports the problem and runs closed
    // rather than reading a capability out of it.
    std::fs::write(
        directory.join(CONFIG_FILE_NAME),
        r#"{"wizard_completed": true, "collect_enabled": true, "cloud_enabled": true}"#,
    )
    .expect("write");

    let session = Session::open(&directory);
    let status = session.status();
    assert!(
        status.config_problem.is_some(),
        "a file this build cannot read must be reported, not guessed at",
    );
    assert!(!status.wizard_completed, "and nothing in it may be trusted");
    assert!(session.snapshot().fully_closed);
    drop(keep);
}

/// WP11 leftover 8: the authorization list now lives somewhere, so a restart
/// no longer means authorizing again.
#[test]
fn an_authorized_directory_comes_back_after_a_restart() {
    let (keep_data, directory) = scratch();
    let (keep_folder, folder) = scratch();
    let shown = a_folder_worth_tidying(&folder);

    let mut session = Session::open(&directory);
    let files = session.authorize(&shown).expect("the user authorizes it");
    assert_eq!(files.roots.len(), 1);
    assert!(files.unavailable_roots.is_empty());
    assert!(!files.executable_in_this_version);
    assert!(files.read_only_notice.contains("v0.1.1"));
    drop(session);

    let stored: StoredConfig = read_stored_config(&directory).expect("the file reads back");
    assert_eq!(stored.authorized_roots.len(), 1);

    let session = Session::open(&directory);
    assert_eq!(session.files().roots.len(), 1);
    assert_eq!(session.snapshot().authorized_root_count, 1);
    drop((keep_data, keep_folder));
}

/// A directory that has gone since it was authorized is reported, not quietly
/// dropped: the user believes Soul can read it.
#[test]
fn a_directory_that_has_gone_is_reported_rather_than_dropped() {
    let (keep_data, directory) = scratch();
    let (keep_folder, folder) = scratch();
    let stays = a_folder_worth_tidying(&folder);
    let goes = folder.join("subfolder");
    std::fs::create_dir(&goes).expect("create");

    let mut session = Session::open(&directory);
    session.authorize(&stays).expect("authorize");
    session
        .authorize(&goes.to_string_lossy())
        .expect("authorize");
    drop(session);
    std::fs::remove_dir(&goes).expect("remove");

    let files = Session::open(&directory).files();
    assert_eq!(files.roots.len(), 1);
    assert_eq!(files.unavailable_roots.len(), 1);
    assert!(!files.unavailable_roots[0].explanation.is_empty());
    drop((keep_data, keep_folder));
}

/// The file-plan screen's whole state: a plan the user can read, and no way to
/// carry it out.
#[test]
fn the_plan_the_shell_receives_is_a_preview_and_only_that() {
    let (keep_data, directory) = scratch();
    let (keep_folder, folder) = scratch();
    let shown = a_folder_worth_tidying(&folder);

    let mut session = Session::open(&directory);
    let refusal = session
        .preview(&shown)
        .expect_err("nothing is authorized yet");
    assert_eq!(refusal.reason_code, "CONSENT_MISSING");

    session.authorize(&shown).expect("authorize");
    let preview = session.preview(&shown).expect("a plan");

    assert!(!preview.executable_in_this_version);
    assert!(preview.disk_unchanged);
    assert_eq!(preview.moves.len(), 2, "{:?}", preview.moves);
    assert_eq!(preview.left_alone.len(), 1);
    assert!(!preview.plan_hash.is_empty());
    assert!(preview.read_only_notice.contains("只读"));

    // Twice over the same unchanged directory is the same plan.
    let again = session.preview(&shown).expect("a plan");
    assert_eq!(again.plan_hash, preview.plan_hash);
    drop((keep_data, keep_folder));
}

/// AC-17 through the session: no endpoint, no request body, a draft anyway.
#[test]
fn the_local_drafting_path_needs_nothing_configured() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let draft = session.draft_pasted(PASTED).expect("a draft");
    assert_eq!(
        draft.source,
        soulcore::commands::draft::DraftSource::ToneTemplate
    );
    assert!(!draft.text.is_empty());
    assert_eq!(draft.third_party_turns, 0, "no body was built at all");
    drop(keep);
}

/// The endpoint path, one screen at a time: prepare describes, generate is
/// the only thing that could act, and an approval that does not match sends
/// nothing.
#[test]
fn an_approval_that_does_not_match_the_plan_reaches_no_endpoint() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let plan = session.prepare_draft(PASTED, None).expect("a plan to read");
    assert_eq!(plan.third_party_turns, 1);
    assert_eq!(plan.placeheld_turns, 1);
    assert!(!plan.carries_exempted_original);
    assert!(!plan.plan_hash.is_empty());
    assert!(!plan.preparation_id.is_empty());
    assert!(plan.notice.contains("不会替你发送"));

    let wrong = Approval {
        preparation_id: plan.preparation_id.clone(),
        plan_hash: "00".repeat(32),
    };
    let refusal = session
        .generate_draft(&wrong)
        .expect_err("a plan hash that was never on screen");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");

    // The prepared body was taken by value on the way to that refusal, so the
    // right approval now has nothing to approve. One preparation, one chance.
    let refusal = session
        .generate_draft(&plan.approval())
        .expect_err("there is nothing prepared any more");
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    drop(keep);
}

/// With no endpoint configured the approved request is refused at the last
/// gate rather than quietly turning into a local draft: AC-11's refusal is
/// meant to be visible.
#[test]
fn approving_a_plan_with_no_endpoint_configured_is_refused_and_sends_nothing() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let plan = session.prepare_draft(PASTED, None).expect("a plan to read");
    let refusal = session
        .generate_draft(&plan.approval())
        .expect_err("there is nowhere to send it");
    assert_eq!(refusal.reason_code, "E1_NOT_CONFIGURED");
    assert!(!session.discard_draft(), "the preparation was spent");
    drop(keep);
}

/// A user who reads the plan and says no leaves nothing behind.
#[test]
fn discarding_a_prepared_request_throws_it_away() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    session.prepare_draft(PASTED, None).expect("a plan");
    assert!(session.discard_draft());
    assert!(!session.discard_draft(), "there was only one");
    drop(keep);
}

/// An empty store is an empty graph, not an error and not a mock-up.
#[test]
fn a_store_with_nobody_in_it_produces_an_empty_graph() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    let people = session.people().expect("an empty graph is still a graph");
    assert!(people.people.is_empty());
    assert!(people.ties.is_empty());
    assert_eq!(people.notice, "工作假设，非临床结论");
    assert!(people.third_party_data_is_local_only);

    let refusal = session
        .person_summary(&uuid::Uuid::now_v7().to_string())
        .expect_err("nobody is in the store");
    assert!(!refusal.explanation.is_empty());

    let refusal = session
        .person_summary("not-a-uuid")
        .expect_err("that is not an identifier");
    assert!(refusal.explanation.contains("编号"));
    drop(keep);
}

/// The key material this machine used is named rather than assumed.
///
/// On the platform Soul ships to that is DPAPI, and since the provider stopped
/// being a skeleton the store opens there: the KEK is protected under the
/// logged-in user in `keys.dpapi`, and the notice on screen says so instead of
/// explaining why there is nothing to show. Anywhere else the seed file beside
/// the database is what opened it, and the notice has to admit that.
#[test]
fn the_session_says_which_key_material_opened_the_store() {
    let (keep, directory) = scratch();
    let status = Session::open(&directory).status();

    if cfg!(windows) {
        assert_eq!(status.key_protection, "dpapi");
        assert!(status.store_opened, "{}", status.store_notice);
        assert!(
            status.store_notice.contains("密钥"),
            "a protected build has to say what protects it: {}",
            status.store_notice,
        );
        assert!(
            directory.join(KEY_BLOB_FILE_NAME).is_file(),
            "the store opened without a DPAPI-protected KEK beside it",
        );
    } else {
        assert_eq!(status.key_protection, "developer_key_file");
        assert!(status.store_opened);
        assert!(
            status.store_notice.contains("开发构建"),
            "a build with no platform key protection has to say so",
        );
        assert!(
            !directory.join(KEY_BLOB_FILE_NAME).exists(),
            "there is no DPAPI on this platform, so nothing may write its blob",
        );
    }
    drop(keep);
}

/// The other half of the same promise, and the one Linux CI is here to check:
/// the Windows provider is not what opened this store.
///
/// `key_protection` is a label the session writes; this looks at what is
/// actually on disk. `TestKeyProvider::in_dir` leaves `soul-test-keys.bin`
/// and `DpapiKeyProvider` leaves `keys.dpapi`, so exactly one of them is
/// present on any machine and it says which provider ran.
#[test]
fn nothing_but_the_developer_seed_file_opens_the_store_off_windows() {
    if cfg!(windows) {
        return;
    }
    let (keep, directory) = scratch();
    let session = Session::open(&directory);

    assert!(session.status().store_opened);
    assert!(
        directory.join("soul-test-keys.bin").is_file(),
        "the developer seed file is what opens the store on this platform",
    );
    assert!(
        !directory.join(KEY_BLOB_FILE_NAME).exists(),
        "a DPAPI key blob off Windows means the platform branch went the wrong way",
    );
    drop(keep);
}
