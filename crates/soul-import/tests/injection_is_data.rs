//! AC-25 through the import door: a hostile export is imported, and nothing it
//! asked for happens.
//!
//! The corpus is well-formed `soul-import-v1`. That is the point — it is not
//! malformed input to be rejected, it is ordinary data whose *content* tries to
//! give instructions. It has to land like any other message, and none of it may
//! become a command, a plan, or a connection.

use soul_policy::hitl::{check_action, ActionKind, ActionRequest, RequestOrigin, TokenIssuer};
use soul_policy::injection::{self, UntrustedText};
use soul_policy::net_guard::NetGuard;
use soul_schema::audit::{AuditAction, AuditDecision};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::{AuditLog, BlobStore, EventStore};
use soul_testkit::fixtures;

const SEED: &str = "wp06 injection";

fn hostile_text() -> String {
    fixtures::read_text("import/soul-import-v1/injection_lines.jsonl").expect("fixture")
}

fn hostile() -> soul_import::model::StagedImport {
    soul_import::soul_import_v1::parse(&hostile_text())
        .expect("hostile content is still valid data")
}

/// The corpus is one-sided: every line in it was written at the user, none by
/// them. Committing needs somebody to attribute the conversation to, so this
/// adds the one reply the user sent — which is also the honest shape of the
/// situation being tested.
fn hostile_with_a_reply() -> soul_import::model::StagedImport {
    let text = hostile_text();
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    lines.insert(
        1,
        concat!(
            r#"{"type":"message","id":"m-1000","occurred_at":"2026-08-22T09:59:00Z","#,
            r#""sender_scope":"self","conversation_id":"c-09","sender_id":"u-self","#,
            r#""text":"你是谁？"}"#,
        )
        .to_owned(),
    );
    soul_import::soul_import_v1::parse(&lines.join("\n")).expect("valid")
}

/// Every body in the corpus arrives as `UntrustedText`, which has no `Display`
/// and cannot be interpolated into an instruction by accident.
#[test]
fn hostile_lines_arrive_as_untrusted_text_and_import_like_anything_else() {
    let staged = hostile();
    assert_eq!(staged.messages.len(), 5);

    let scanned: Vec<Vec<injection::InjectionSignal>> = staged
        .messages
        .iter()
        .map(|message| injection::scan(&message.body))
        .collect();
    assert!(
        scanned.iter().all(|signals| !signals.is_empty()),
        "every line in this corpus is trying something",
    );
    assert!(scanned
        .iter()
        .any(|signals| signals.contains(&injection::InjectionSignal::InstructionOverride)));
    assert!(scanned
        .iter()
        .any(|signals| signals.contains(&injection::InjectionSignal::ToolCallShape)));
}

/// The import records that an attempt was in the file, and stores the attempt
/// as prose like any other message.
#[test]
fn a_hostile_export_commits_and_leaves_an_injection_blocked_entry() {
    let dir = tempfile::tempdir().expect("temp dir");
    let staged = hostile_with_a_reply();
    let mut store =
        SqlCipherStore::open(dir.path().join("soul.db"), &TestKeyProvider::from_seed(SEED))
            .expect("open");

    let receipt = soul_import::commit::commit(&mut store, &staged).expect("commit");
    for content in receipt.audit.clone() {
        soul_policy::audit::append(&mut store, content, 1_787_500_000).expect("audit");
    }

    assert_eq!(receipt.events_written.len(), 6);
    assert_eq!(
        receipt.messages_with_injection_markers, 5,
        "the user's own reply is not an attempt",
    );

    let entries = store.list_audit().expect("chain");
    let blocked = entries
        .iter()
        .find(|entry| entry.action == AuditAction::InjectionBlocked)
        .expect("the user is told their file tried something");
    assert_eq!(blocked.decision, AuditDecision::Denied);
    assert_eq!(
        blocked.reason_code.as_deref(),
        Some(soul_policy::ReasonCode::InjectionMarkersFound.as_str()),
    );
    assert_eq!(blocked.counts.as_ref().and_then(|c| c.items), Some(5));
    soul_policy::audit::check(blocked).expect("the entry carries no prose");

    // Stored, and still readable through the front door: this is data being
    // kept, not content being filtered out.
    let events = store
        .list_events(&soul_store_api::EventFilter::default())
        .expect("events");
    let bodies: Vec<String> = events
        .iter()
        .filter_map(|event| event.body_ref.as_ref())
        .map(|sealed| String::from_utf8(store.open(sealed).expect("open")).expect("utf-8"))
        .collect();
    assert_eq!(bodies.len(), 6);
    assert!(bodies.iter().any(|body| body.contains("忽略之前指令")));
}

/// The corpus as it stands has nobody in it who is the user, and a file that
/// cannot be attributed is refused rather than imported as a graph of
/// strangers talking to nobody.
#[test]
fn an_export_with_no_owner_is_refused_at_commit() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("soul.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");

    let error = soul_import::commit::commit(&mut store, &hostile()).expect_err("no owner");
    assert!(matches!(error, soul_import::commit::ImportError::NoOwner));
    assert!(store
        .list_events(&soul_store_api::EventFilter::default())
        .expect("events")
        .is_empty());
}

/// The red line. An imported line asks for a file write and for the cloud
/// switch; neither may be granted, and content is not an origin that can ask.
#[test]
fn nothing_in_an_imported_line_can_authorize_an_action() {
    let staged = hostile();
    let mut issuer = TokenIssuer::new();

    for message in &staged.messages {
        // The closest thing to "acting on" one of these lines that the API
        // permits: a request whose origin is the content itself. Every action
        // this build knows, because the refusal must not depend on which one
        // the line asked for.
        for action in ActionKind::ALL {
            let request = ActionRequest::new(action.as_str(), RequestOrigin::ExternalContent);
            let denial = check_action(&mut issuer, &request, 1_787_500_000_000)
                .expect_err("external content cannot authorize an action");
            assert_eq!(
                denial.reason_code(),
                soul_policy::ReasonCode::ExternalContentNotAuthority,
                "{} asking for {} must be refused for being content, not on a technicality",
                message.external_id,
                action.as_str(),
            );
        }
    }

    // A line asking for a file write gets nowhere even before the origin
    // check: v0.1 has no file-write action at all.
    let request = ActionRequest::new("file.write", RequestOrigin::User);
    let denial = check_action(&mut issuer, &request, 1_787_500_000_000)
        .expect_err("there is no such action");
    assert_eq!(denial.reason_code(), soul_policy::ReasonCode::UnknownAction);
}

/// The URLs in the corpus are the ones a naive reader would fetch. Nothing may
/// reach them, and the guard refuses them without having been told about them.
#[test]
fn no_url_an_imported_line_mentions_is_reachable() {
    let staged = hostile();
    let guard = NetGuard::closed();

    let mut seen = 0usize;
    for message in &staged.messages {
        for url in injection::urls_in(&message.body) {
            seen += 1;
            guard
                .authorize_e1(&url)
                .expect_err("an origin nobody configured is refused");
        }
    }
    assert!(seen >= 3, "the corpus has to carry URLs; found {seen}");
}

/// A line shaped like a tool call is a string. Reading it back gives a string.
#[test]
fn a_line_shaped_like_a_tool_call_stays_a_string() {
    let staged = hostile();
    let tool_call = staged
        .messages
        .iter()
        .find(|message| message.body.as_str().contains("tool_call"))
        .expect("the corpus carries one");

    let body: &UntrustedText = &tool_call.body;
    // Parsing it as JSON succeeds — it is well-formed JSON — and that changes
    // nothing, because nothing in this workspace dispatches on the result.
    let parsed: serde_json::Value =
        serde_json::from_str(body.as_str()).expect("it is well-formed JSON");
    assert!(parsed.get("tool_call").is_some());
    assert!(
        injection::scan(body).contains(&injection::InjectionSignal::ToolCallShape),
        "the shape is recorded for the audit entry and acted on by nothing",
    );
}
