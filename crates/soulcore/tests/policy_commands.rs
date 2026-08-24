//! WP08's command surface end to end: the steps a caller has to take to make
//! one request leave the machine, and the ones that stop it.
//!
//! `soul-policy` already tests each gate on its own. What is checked here is
//! the wiring — that `soulcore` cannot be persuaded to skip a step by calling
//! the commands in a different order, and that a session with no configured
//! endpoint reaches nothing.

use soul_policy::hitl::{
    ActionKind, ActionRequest, CapabilityScope, HitlDenial, PlanHash, RequestOrigin, TokenError,
};
use soul_policy::redactor::{KnownIdentifiers, Turn, THIRD_PARTY_PLACEHOLDER};
use soul_policy::{EgressClass, ReasonCode};
use soul_schema::common::SealedSubject;
use soul_testkit::leakage::LeakageChecker;
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::policy::{e1_plan, E1Refusal, PolicySession};
use uuid::Uuid;

const MODEL: &str = "local-model";
const NOW_MS: u64 = 1_700_000_000_000;
const THIRD_PARTY_BODY: &str = "周五的场地我已经订好了，你直接过来就行";
const NAME: &str = "王小明";

fn turns() -> Vec<Turn> {
    vec![
        Turn::new(Uuid::now_v7(), SealedSubject::ThirdParty, THIRD_PARTY_BODY),
        Turn::new(
            Uuid::now_v7(),
            SealedSubject::Owner,
            format!("好，我回复{NAME}。"),
        ),
    ]
}

fn session(endpoint: &MockLlm) -> PolicySession {
    PolicySession::with_user_endpoint(
        &endpoint.base_url(),
        KnownIdentifiers::new().with_name(NAME),
    )
    .expect("the user's endpoint parses")
}

/// The whole path: redact, describe, approve, token, send.
#[test]
fn a_generation_needs_a_redaction_a_plan_and_a_token() {
    let endpoint = MockLlm::start().expect("start the mock endpoint");
    let mut session = session(&endpoint);

    let body = session.redact(&turns());
    assert!(body.as_str().contains(THIRD_PARTY_PLACEHOLDER));

    let plan_hash = PlanHash::of(&e1_plan(MODEL, &body));
    let token = session
        .issue_token(CapabilityScope::E1Generate, plan_hash.clone(), NOW_MS)
        .expect("a generation token");

    let outcome = session
        .e1_generate(MODEL, body, token.token_id(), NOW_MS)
        .expect("the mock answers");

    assert_eq!(outcome.status, 200);
    assert_eq!(outcome.egress_class, EgressClass::E1);
    assert_eq!(outcome.plan_hash, plan_hash);
    assert_eq!(endpoint.request_count(), 1);

    // The audit entry for it names counts and hashes, and no prose.
    let entry = outcome
        .audit()
        .into_entry(Uuid::now_v7(), 1_700_000_000)
        .expect("the audit content is prose-free");
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("conversation", THIRD_PARTY_BODY);
    checker.add_known_identifier("name", NAME);
    checker.assert_clean(
        "the audit entry",
        &serde_json::to_string(&entry).expect("serialize"),
    );
    assert_eq!(
        entry.reason_code.as_deref(),
        Some(ReasonCode::ThirdPartyBodyPlaceheld.as_str()),
    );
}

/// The token is good once. A second call with the same one is a replay, and
/// the endpoint sees only the first request.
#[test]
fn the_same_token_cannot_drive_two_requests() {
    let endpoint = MockLlm::start().expect("start the mock endpoint");
    let mut session = session(&endpoint);

    let body = session.redact(&turns());
    let token = session
        .issue_token(
            CapabilityScope::E1Generate,
            PlanHash::of(&e1_plan(MODEL, &body)),
            NOW_MS,
        )
        .expect("a generation token");

    session
        .e1_generate(MODEL, body, token.token_id(), NOW_MS)
        .expect("the first request goes");

    let body = session.redact(&turns());
    let refusal = session
        .e1_generate(MODEL, body, token.token_id(), NOW_MS)
        .expect_err("the second must be refused");

    assert!(matches!(
        refusal,
        E1Refusal::Hitl(HitlDenial::Token(TokenError::Replayed(_))),
    ));
    assert_eq!(refusal.reason_code(), ReasonCode::TokenReplayed);
    assert_eq!(endpoint.request_count(), 1, "only the first request left");
}

/// A plan edited after approval changes its hash, and the token was minted
/// against the old one.
#[test]
fn a_plan_edited_after_approval_is_refused() {
    let endpoint = MockLlm::start().expect("start the mock endpoint");
    let mut session = session(&endpoint);

    let body = session.redact(&turns());
    let token = session
        .issue_token(
            CapabilityScope::E1Generate,
            PlanHash::of(&e1_plan(MODEL, &body)),
            NOW_MS,
        )
        .expect("a generation token");

    // The model is part of what the user approved.
    let refusal = session
        .e1_generate("a-different-model", body, token.token_id(), NOW_MS)
        .expect_err("a changed plan must be refused");

    assert_eq!(refusal.reason_code(), ReasonCode::PlanHashMismatch);
    assert_eq!(endpoint.request_count(), 0);
}

/// v0.1 will not even mint a file-write token.
#[test]
fn no_file_write_token_is_issued() {
    let mut session = PolicySession::closed();
    let refused = session
        .issue_token(
            CapabilityScope::FileWrite,
            PlanHash::of(&serde_json::json!({ "action": "write" })),
            NOW_MS,
        )
        .expect_err("v0.1 has no file-write implementation");
    assert_eq!(refused.reason, ReasonCode::WriteNotImplemented);
}

/// The shipped default reaches nothing, and says why in a code the audit can
/// carry.
#[test]
fn a_closed_session_contacts_nothing() {
    let endpoint = MockLlm::start().expect("start the mock endpoint");
    let mut session = PolicySession::closed();

    let body = session.redact(&turns());
    let token = session
        .issue_token(
            CapabilityScope::E1Generate,
            PlanHash::of(&e1_plan(MODEL, &body)),
            NOW_MS,
        )
        .expect("a token can be issued; it just has nowhere to be spent");

    let refusal = session
        .e1_generate(MODEL, body, token.token_id(), NOW_MS)
        .expect_err("nothing is configured");
    assert_eq!(refusal.reason_code(), ReasonCode::E1NotConfigured);
    assert_eq!(endpoint.request_count(), 0);
}

/// Imported or pasted material cannot ask for anything, whatever it says.
#[test]
fn external_content_cannot_request_an_action() {
    let mut session = PolicySession::closed();

    let denial = session
        .check_action(
            &ActionRequest::new(
                ActionKind::GenerateWithUserEndpoint.as_str(),
                RequestOrigin::ExternalContent,
            ),
            NOW_MS,
        )
        .expect_err("external content is never authority");
    assert_eq!(
        denial.reason_code(),
        ReasonCode::ExternalContentNotAuthority
    );

    let denial = session
        .check_action(
            &ActionRequest::new("exfiltrate.everything", RequestOrigin::User),
            NOW_MS,
        )
        .expect_err("an action this build does not know is refused");
    assert_eq!(denial.reason_code(), ReasonCode::UnknownAction);
}
