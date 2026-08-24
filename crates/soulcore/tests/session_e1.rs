//! The user's own endpoint, proven through the object the desktop shell holds.
//!
//! `soul-policy` already proves that an origin is compared exactly and that a
//! guard with nothing configured refuses everything, and
//! `tests/endpoint_is_user_supplied.rs` proves that writing an address into a
//! [`Config`](soulcore::Config) contacts nobody. What none of them can show is
//! that a running Soul is wired to any of it: until the session had a set and a
//! clear, `PolicySession::with_user_endpoint` was a constructor the product
//! never called, and 语言模型端点 was 未填写 for the life of every installation.
//! That is the gap this file closes — the same guard, the same drafting pair,
//! reached the way `set_user_endpoint` is reached from the 设置 page.
//!
//! The other half is about the file on disk. AC-02 says a restart reopens
//! nothing, and for the endpoint that is a claim about `config.json` having
//! nowhere to write an address down: the tests reopen the directory and read
//! the bytes.
//!
//! The mock endpoint is here to be *not* contacted. It is a real loopback
//! server, so `request_count() == 0` is a statement about sockets rather than
//! about intent.

use std::path::{Path, PathBuf};

use soul_draft::brief::BRIEF_HEADING;
use soul_policy::redactor::{ACCOUNT_PLACEHOLDER, THIRD_PARTY_PLACEHOLDER};
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::draft::Approval;
use soulcore::commands::session::{
    read_stored_config, Session, SessionRefusal, StoredConfig, CONFIG_FILE_NAME,
    ENDPOINT_UNPARSABLE_NOTICE,
};
use soulcore::commands::shell::{WizardAnswers, LLM_ENDPOINT_SESSION_ONLY_NOTICE};

fn scratch() -> (tempfile::TempDir, PathBuf) {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let canonical = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
    (directory, canonical)
}

fn finish_the_wizard(session: &mut Session) {
    session
        .complete_wizard(&WizardAnswers {
            acknowledged_defaults_are_off: true,
        })
        .expect("the wizard hands back a closed configuration");
}

fn config_text(directory: &Path) -> String {
    let bytes = std::fs::read(directory.join(CONFIG_FILE_NAME)).expect("the configuration file");
    String::from_utf8(bytes).expect("the configuration file is utf-8")
}

/// Prepare a generation and approve it, which is the only path that opens a
/// socket. Returns whatever the session answered.
fn draft_through_the_endpoint(session: &mut Session) -> Result<(), SessionRefusal> {
    let plan = session
        .prepare_draft("周五的场地我已经订好了，你直接过来就行", None)
        .expect("a paste can always be described");
    session.generate_draft(&Approval {
        preparation_id: plan.preparation_id,
        plan_hash: plan.plan_hash,
    })?;
    Ok(())
}

/// A launch nobody has configured knows of no endpoint, and the approval a user
/// could give on that session reaches nothing.
///
/// The refusal is the interesting half. Describing a request works with nothing
/// configured — that is what lets the confirmation screen exist at all — so the
/// place the absence has to bite is the step after the approval.
#[test]
fn a_fresh_session_has_no_endpoint_and_an_approval_reaches_nothing() {
    let (keep, directory) = scratch();
    let listening = MockLlm::start().expect("something nobody configured");
    let mut session = Session::open(&directory);

    let snapshot = session.snapshot();
    assert!(!snapshot.llm_endpoint_configured);
    assert!(snapshot.fully_closed);
    assert_eq!(
        snapshot.llm_endpoint_notice,
        LLM_ENDPOINT_SESSION_ONLY_NOTICE,
    );

    let refusal = draft_through_the_endpoint(&mut session)
        .expect_err("there is nowhere to send an approved request");
    assert_eq!(refusal.reason_code, "E1_NOT_CONFIGURED");
    assert_eq!(
        listening.request_count(),
        0,
        "a session with nothing configured opened a socket anyway",
    );
    drop(keep);
}

/// Naming an endpoint is not requesting anything from it.
///
/// The address handed in is a server that is running and would answer, so the
/// count below is the whole claim: the form parsed a string and pointed a
/// guard, and nothing went out.
#[test]
fn setting_the_endpoint_configures_the_session_without_contacting_it() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user is about to type in");
    let mut session = Session::open(&directory);

    let snapshot = session
        .set_user_endpoint(&endpoint.base_url())
        .expect("a loopback address is an address");

    assert!(snapshot.llm_endpoint_configured);
    assert!(!snapshot.fully_closed);
    assert!(
        snapshot
            .open_capabilities
            .contains(&"llm_endpoint".to_owned()),
        "the snapshot does not say which capability is open: {:?}",
        snapshot.open_capabilities,
    );
    assert_eq!(
        endpoint.request_count(),
        0,
        "filling in the address contacted it",
    );

    // And the address itself is not in what the interface receives.
    let json = serde_json::to_string(&snapshot).expect("serialize the snapshot");
    assert!(
        !json.contains(&endpoint.port().to_string()),
        "the snapshot carries the endpoint: {json}",
    );
    drop(keep);
}

/// The address the user typed is the one an approved generation goes to.
///
/// Worth doing here rather than only in `draft_commands.rs`, which points a
/// `PolicySession` at a mock directly: what is under test is the session's own
/// wiring, so that a build where `set_user_endpoint` updated the configuration
/// field and forgot the guard would fail rather than look configured.
#[test]
fn an_approved_generation_goes_to_the_address_the_user_typed() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    let mut session = Session::open(&directory);
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("a loopback address is an address");

    draft_through_the_endpoint(&mut session).expect("the endpoint answers");

    assert_eq!(endpoint.request_count(), 1, "one approval, one request");
    assert_eq!(endpoint.requests()[0].path, "/v1/chat/completions");
    drop(keep);
}

/// AC-02 for the endpoint: the next launch is back to reaching nothing.
///
/// Free, and that is the point of not persisting: `Session::open` builds its
/// configuration from `StoredConfig::to_config`, which is `Config::default`
/// plus authorized directories, and its policy from `draft::closed_session`.
/// Nothing has to remember to clear an address, because there is no file it
/// could have been written to.
#[test]
fn a_restart_finds_the_endpoint_gone() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    {
        let mut session = Session::open(&directory);
        finish_the_wizard(&mut session);
        assert!(
            session
                .set_user_endpoint(&endpoint.base_url())
                .expect("the address is taken")
                .llm_endpoint_configured,
        );
    }

    let mut next_launch = Session::open(&directory);
    assert!(
        !next_launch.snapshot().llm_endpoint_configured,
        "AC-02: a restart may not carry an endpoint across",
    );

    // Not merely forgotten in the snapshot: the guard is closed again, so an
    // approval on this launch reaches nothing.
    let refusal =
        draft_through_the_endpoint(&mut next_launch).expect_err("the guard is closed again");
    assert_eq!(refusal.reason_code, "E1_NOT_CONFIGURED");
    assert_eq!(endpoint.request_count(), 0);
    drop(keep);
}

/// The address never reaches the file beside the store.
///
/// Everything that writes `config.json` is done first — the wizard and an
/// authorization — so the file under inspection is one that was written *after*
/// the endpoint was configured. `deny_unknown_fields` already refuses to read a
/// field like this back; what it cannot catch is a version of this code that
/// writes one, so the test reads the bytes.
#[test]
fn the_configuration_file_never_learns_the_address() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    let mut session = Session::open(&directory);

    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("the address is taken");
    finish_the_wizard(&mut session);
    session
        .authorize(&directory.to_string_lossy())
        .expect("a directory that exists");

    let text = config_text(&directory);
    assert!(
        !text.contains(&endpoint.port().to_string()),
        "the endpoint reached config.json, which is the one file that must not carry it:\n{text}",
    );
    for word in ["llm", "endpoint", "http", "collect", "consent"] {
        assert!(
            !text.contains(word),
            "`{word}` reached config.json:\n{text}",
        );
    }

    let value: serde_json::Value = serde_json::from_str(&text).expect("parse");
    let object = value.as_object().expect("the file is a JSON object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["authorized_roots", "wizard_completed"],
        "config.json grew a field, and the whole argument for AC-02 is that it has two",
    );

    let stored = read_stored_config(&directory).expect("the file still parses as a StoredConfig");
    assert!(stored.wizard_completed);
    assert_eq!(stored.authorized_roots.len(), 1);
    assert_eq!(
        stored,
        StoredConfig {
            wizard_completed: true,
            authorized_roots: stored.authorized_roots.clone(),
        },
    );
    drop(keep);
}

/// Something that is not an address is refused, and the refusal does not read
/// it back.
///
/// The credential form is the one that matters. `Origin::parse` rejects an
/// authority containing `@` outright, and its error names the whole string —
/// which for `https://user:hunter2@host` is a password on a screen, and from a
/// screen in a screenshot. So the session drops the parser's message and
/// answers with its own sentence.
#[test]
fn an_address_that_is_not_one_is_refused_without_being_quoted_back() {
    let (keep, directory) = scratch();
    let mut session = Session::open(&directory);

    for given in [
        "",
        "   ",
        "127.0.0.1:11434",
        "ftp://127.0.0.1:11434",
        "http://",
        "http://127.0.0.1:not-a-port",
        "http://user:hunter2@127.0.0.1:11434",
    ] {
        let refusal = session
            .set_user_endpoint(given)
            .expect_err("that is not an address");

        assert_eq!(refusal.reason_code, "EGRESS_TARGET_UNPARSABLE");
        assert_eq!(refusal.explanation, ENDPOINT_UNPARSABLE_NOTICE);
        assert!(
            !refusal.explanation.contains("hunter2"),
            "the refusal read the address back: {}",
            refusal.explanation,
        );
        assert!(
            !session.snapshot().llm_endpoint_configured,
            "a refused address left the session looking configured: {given}",
        );
    }
    drop(keep);
}

/// A refused address does not disturb one that was already accepted.
///
/// `PolicySession::set_user_endpoint` parses before it assigns, which is what
/// makes this true; a version that cleared the guard first would leave a user
/// who mistyped their second address with nothing configured and no warning
/// that the first one had gone.
#[test]
fn a_refused_address_leaves_the_one_that_was_there() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    let mut session = Session::open(&directory);

    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("the address is taken");
    session
        .set_user_endpoint("not an address at all")
        .expect_err("that is not an address");

    assert!(session.snapshot().llm_endpoint_configured);
    draft_through_the_endpoint(&mut session).expect("the first address is still the one");
    assert_eq!(endpoint.request_count(), 1);
    drop(keep);
}

// ------------------------------------------------- AC-13, from the product ---

/// The paste the exemption tests send.
///
/// Somebody else's message, with a number and a handle in it that nobody
/// registered as a contact. A fresh [`Session`] starts from
/// `draft::closed_session`, so its [`KnownIdentifiers`] set is empty and what
/// placeholds these two is the shape scrub rather than the contact graph —
/// which is the interesting case, because it is the one a user who has
/// imported nothing is in. A registered name being placeheld inside an
/// exempted body is `soul-draft`'s
/// `an_exempted_body_still_placeholds_the_name_and_the_number`.
///
/// [`KnownIdentifiers`]: soul_policy::redactor::KnownIdentifiers
const CONFIRMED_PASTE: &str =
    "周五的场地我已经订好了，你直接过来就行，到了打 13800138000 或者找 @xiaoming";

/// AC-13 with the screens in it: one confirmed message travels, the next does
/// not.
///
/// `draft_commands.rs` already proves this of [`DraftSession`] directly. What
/// it cannot show is that a running Soul can reach the state at all: for as
/// long as `prepare_pasted` passed `None`, the confirmation panel's 「有一段是你
/// 二次确认过、按原文带上的。」 was a sentence no user could make true. So the
/// assertions here are on the bytes a real loopback endpoint received, through
/// the same object the desktop shell holds.
///
/// [`DraftSession`]: soulcore::commands::draft::DraftSession
#[test]
fn a_second_confirmation_sends_this_ones_words_and_the_next_preparation_is_placeheld_again() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    let mut session = Session::open(&directory);
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("a loopback address is an address");
    assert_eq!(
        endpoint.request_count(),
        0,
        "filling in the address contacted it",
    );

    // The user read a placeheld plan and pressed 「这一条按原文带上」.
    let exempted = session
        .prepare_draft(CONFIRMED_PASTE, Some(true))
        .expect("a plan");
    assert!(exempted.carries_exempted_original);
    assert_eq!(exempted.third_party_turns, 1);
    assert_eq!(exempted.placeheld_turns, 0);
    assert_eq!(
        endpoint.request_count(),
        0,
        "an exemption is not a generation",
    );
    session
        .generate_draft(&exempted.approval())
        .expect("the endpoint answers");

    // Nobody cleared anything, and the next preparation is placeheld again.
    let after = session
        .prepare_draft(CONFIRMED_PASTE, None)
        .expect("a plan");
    assert!(!after.carries_exempted_original);
    assert_eq!(after.placeheld_turns, 1);
    assert_ne!(
        after.plan_hash, exempted.plan_hash,
        "a differently redacted body is a different plan",
    );
    session
        .generate_draft(&after.approval())
        .expect("the endpoint answers");

    let sent = endpoint.requests();
    assert_eq!(sent.len(), 2, "two approvals, two requests");
    assert!(
        sent[0].body.contains("场地"),
        "the confirmed message did not travel, so the confirmation bought nothing: {}",
        sent[0].body,
    );
    assert!(!sent[0].body.contains(THIRD_PARTY_PLACEHOLDER));
    assert!(
        !sent[1].body.contains("场地"),
        "the exemption was remembered into the next request: {}",
        sent[1].body,
    );
    assert!(sent[1].body.contains(THIRD_PARTY_PLACEHOLDER));

    // The exemption is for one message's prose and nothing else. Confirming to
    // send what somebody wrote is not confirming to publish how to reach them.
    for body in [&sent[0].body, &sent[1].body] {
        assert!(
            !body.contains("13800138000"),
            "the number travelled: {body}"
        );
        assert!(!body.contains("@xiaoming"), "the handle travelled: {body}");
    }
    assert!(
        sent[0].body.contains(ACCOUNT_PLACEHOLDER),
        "the exempted body dropped them rather than placeholding them: {}",
        sent[0].body,
    );

    // And what the interface holds about all this is still counts and
    // identifiers: the plan the user approved carries no prose, and neither
    // does the chain or the snapshot beside it.
    for (what, rendered) in [
        (
            "the exempted plan",
            serde_json::to_string(&exempted).expect("serialize the plan"),
        ),
        (
            "the snapshot",
            serde_json::to_string(&session.snapshot()).expect("serialize the snapshot"),
        ),
        (
            "the audit chain",
            serde_json::to_string(&session.audit().expect("the store opened"))
                .expect("serialize the chain"),
        ),
    ] {
        assert!(
            !rendered.contains("场地"),
            "{what} carries the third party's words: {rendered}",
        );
    }
    drop(keep);
}

/// The second confirmation is an answer, not a setting.
///
/// `config.json` is written after the exempted request has gone out — by an
/// authorization, which is one of the two things that write it — so the file
/// read below is one that was written while the exemption was in flight. It
/// still has two keys, and the launch after it prepares a placeheld request
/// without anything having to remember to clear one.
#[test]
fn an_exemption_reaches_neither_the_configuration_file_nor_the_next_launch() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    {
        let mut session = Session::open(&directory);
        finish_the_wizard(&mut session);
        session
            .set_user_endpoint(&endpoint.base_url())
            .expect("the address is taken");

        let exempted = session
            .prepare_draft(CONFIRMED_PASTE, Some(true))
            .expect("a plan");
        assert!(exempted.carries_exempted_original);
        session
            .generate_draft(&exempted.approval())
            .expect("the endpoint answers");
        session
            .authorize(&directory.to_string_lossy())
            .expect("a directory that exists");
    }

    let text = config_text(&directory);
    for word in ["original", "exempt", "include", "carries", "场地"] {
        assert!(
            !text.contains(word),
            "`{word}` reached config.json:\n{text}"
        );
    }
    let value: serde_json::Value = serde_json::from_str(&text).expect("parse");
    let object = value.as_object().expect("the file is a JSON object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["authorized_roots", "wizard_completed"],
        "config.json grew a field, and an exemption is the last thing that may add one",
    );
    let stored = read_stored_config(&directory).expect("the file still parses as a StoredConfig");
    assert_eq!(
        stored,
        StoredConfig {
            wizard_completed: true,
            authorized_roots: stored.authorized_roots.clone(),
        },
    );

    let mut next_launch = Session::open(&directory);
    let plan = next_launch
        .prepare_draft(CONFIRMED_PASTE, None)
        .expect("a plan");
    assert!(
        !plan.carries_exempted_original,
        "a restart carried an exemption across",
    );
    assert_eq!(plan.placeheld_turns, 1);
    assert_eq!(endpoint.request_count(), 1, "one approval, one request");
    drop(keep);
}

// ------------------------------------------- AC-07, on the endpoint path ---

/// The voice the user pinned is in the bytes the endpoint receives.
///
/// The local template is `session_screens.rs`'s half of AC-07. This is the
/// other one: the brief travels in the request's material slot, above the
/// conversation, so a model asked to write a reply is told 语气：…热络… rather
/// than the neutral four. Until `prepare_pasted` took a brief, it was told the
/// neutral four on every machine — a profile page whose answers reached nothing
/// that writes.
///
/// The assertions are on `endpoint.requests()[0].body`, which is what a real
/// loopback server actually received.
#[test]
fn the_pinned_voice_reaches_the_endpoint_and_the_chain_records_the_request() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    let mut session = Session::open(&directory);

    session
        .set_voice("warmth", "warm")
        .expect("the user sets 温度 to 热络 on the profile page");
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("a loopback address is an address");

    // No exemption: this is the ordinary path, and the third party's words are
    // placeheld in the body the brief travels with.
    draft_through_the_endpoint(&mut session).expect("the endpoint answers");

    let sent = endpoint.requests();
    assert_eq!(sent.len(), 1, "one approval, one request");
    assert!(
        sent[0].body.contains(BRIEF_HEADING),
        "the profile material never reached the request: {}",
        sent[0].body,
    );
    assert!(
        sent[0].body.contains("热络"),
        "the pinned voice never reached the request: {}",
        sent[0].body,
    );
    assert!(
        !sent[0].body.contains("克制"),
        "the request describes a voice the user did not set: {}",
        sent[0].body,
    );
    assert!(sent[0].body.contains(THIRD_PARTY_PLACEHOLDER));

    // AC-23 for this path: one request that left, one draft that came back.
    let chain = session.audit().expect("the store opened");
    assert!(chain.verified, "{:?}", chain.verification_problem);
    for action in ["egress.request", "draft.create"] {
        assert!(
            chain.entries.iter().any(|entry| entry.action == action),
            "`{action}` is missing from the chain: {:?}",
            chain.entries,
        );
    }
    let request = chain
        .entries
        .iter()
        .find(|entry| entry.action == "egress.request")
        .expect("the entry is there");
    assert_eq!(request.egress_class.as_deref(), Some("E1"));
    assert!(request.capability_token_id.is_some());
    assert!(
        chain.entries.iter().all(|entry| entry.follows_previous),
        "{:?}",
        chain.entries,
    );
    let played = format!("{chain:?}");
    for prose in ["场地", "热络", BRIEF_HEADING] {
        assert!(!played.contains(prose), "the chain carries `{prose}`");
    }

    // And none of it is written down. The wizard is finished last so that the
    // file under inspection is one written after the voice was pinned and the
    // address entered.
    finish_the_wizard(&mut session);
    let text = config_text(&directory);
    let port = endpoint.port().to_string();
    for word in ["llm", "endpoint", "http", "warmth", "voice", port.as_str()] {
        assert!(
            !text.contains(word),
            "`{word}` reached config.json:\n{text}",
        );
    }
    let value: serde_json::Value = serde_json::from_str(&text).expect("parse");
    let object = value.as_object().expect("the file is a JSON object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["authorized_roots", "wizard_completed"],
        "config.json grew a field, and a voice is the last thing that may add one",
    );
    drop(keep);
}

/// Clearing puts the session back where it started, guard included.
#[test]
fn clearing_the_endpoint_closes_the_guard_again() {
    let (keep, directory) = scratch();
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    let mut session = Session::open(&directory);
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("the address is taken");

    let snapshot = session.clear_user_endpoint();
    assert!(!snapshot.llm_endpoint_configured);
    assert!(snapshot.fully_closed);
    assert!(snapshot.open_capabilities.is_empty());

    let refusal =
        draft_through_the_endpoint(&mut session).expect_err("there is nowhere to send it now");
    assert_eq!(refusal.reason_code, "E1_NOT_CONFIGURED");
    assert_eq!(
        endpoint.request_count(),
        0,
        "a cleared endpoint was contacted",
    );
    drop(keep);
}
