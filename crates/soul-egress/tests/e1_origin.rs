//! AC-11 on the wire: only the configured origin is contacted, a cross-origin
//! redirect is refused, and with no endpoint configured nothing non-loopback
//! is reachable at all.
//!
//! `soul_testkit::MockLlm` is a real loopback server that records every
//! request verbatim, which is the only way to check what left the process. A
//! test that asserted against an in-process fake would be checking the same
//! code twice.
//!
//! The redirect case is the one worth being careful about. A client that
//! follows redirects by default turns a `302` from the user's endpoint into an
//! outbound request to wherever the response pointed — an E0 connection nobody
//! in the workspace asked for. So the mock is told to answer `302` to another
//! origin, and the assertion is both that `send` fails and that the redirect
//! target never received anything.

use uuid::Uuid;

use soul_egress::{send, EgressError};
use soul_policy::e1::{E1RequestPlan, DRAFTING_INSTRUCTION, QUOTE_CLOSE, QUOTE_OPEN};
use soul_policy::net_guard::{EgressClass, EgressConfig, NetGuard};
use soul_policy::redactor::{
    KnownIdentifiers, RedactedBody, Redactor, Turn, THIRD_PARTY_PLACEHOLDER,
};
use soul_policy::ReasonCode;
use soul_schema::common::SealedSubject;
use soul_testkit::leakage::LeakageChecker;
use soul_testkit::MockLlm;

const MODEL: &str = "local-model";
const THIRD_PARTY_BODY: &str = "明天上午十点在公司门口见，别迟到";
const NAME: &str = "李雷";
const PHONE: &str = "13800138000";

fn guard_for(mock: &MockLlm) -> NetGuard {
    NetGuard::new(EgressConfig::with_user_endpoint(&mock.base_url()).expect("the user's endpoint"))
}

fn redactor() -> Redactor {
    Redactor::new(KnownIdentifiers::new().with_name(NAME).with_account(PHONE))
}

/// A draft against one third-party message and one of the user's own.
fn default_body() -> RedactedBody {
    redactor().redact_for_e1(&[
        Turn::new(Uuid::now_v7(), SealedSubject::ThirdParty, THIRD_PARTY_BODY),
        Turn::new(
            Uuid::now_v7(),
            SealedSubject::Owner,
            format!("我准备回复{NAME}，必要时打 {PHONE}。"),
        ),
    ])
}

fn checker() -> LeakageChecker {
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("conversation", THIRD_PARTY_BODY);
    checker.add_known_identifier("name", NAME);
    checker.add_known_identifier("phone", PHONE);
    checker
}

#[test]
fn a_request_reaches_the_configured_origin_and_only_that_one() {
    let mock = MockLlm::start().expect("start the mock endpoint");
    let elsewhere = MockLlm::start().expect("start a second endpoint");

    let permit = guard_for(&mock)
        .authorize_e1(&mock.chat_completions_url())
        .expect("the configured origin");
    assert_eq!(permit.class(), EgressClass::E1);

    let plan = E1RequestPlan::chat_completions(permit, MODEL, default_body());
    let response = send(&plan).expect("the mock answers");

    assert_eq!(response.status, 200);
    assert_eq!(mock.request_count(), 1, "exactly one request was sent");
    assert_eq!(
        elsewhere.request_count(),
        0,
        "no other origin may be contacted",
    );

    let recorded = &mock.requests()[0];
    assert_eq!(recorded.method, "POST");
    assert_eq!(recorded.path, "/v1/chat/completions");
}

/// AC-12 on the wire. The body the server actually received, not a
/// re-serialization of it, is what the leakage checker reads.
#[test]
fn the_body_on_the_wire_carries_no_third_party_prose() {
    let mock = MockLlm::start().expect("start the mock endpoint");
    let permit = guard_for(&mock)
        .authorize_e1(&mock.chat_completions_url())
        .expect("permit");

    send(&E1RequestPlan::chat_completions(
        permit,
        MODEL,
        default_body(),
    ))
    .expect("the mock answers");

    let recorded = mock.requests().remove(0);
    checker().assert_clean("the request body as the server received it", &recorded.body);
    assert!(recorded.body.contains(THIRD_PARTY_PLACEHOLDER));

    // The instruction and the quoted material are in different messages, and
    // the external material is fenced.
    assert!(recorded.body.contains(QUOTE_OPEN));
    assert!(recorded.body.contains(QUOTE_CLOSE));
    let instruction_prefix: String = DRAFTING_INSTRUCTION.chars().take(12).collect();
    let instruction_at = recorded
        .body
        .find(&instruction_prefix)
        .expect("the system instruction is present");
    let quote_at = recorded
        .body
        .find(QUOTE_OPEN)
        .expect("the fence is present");
    assert!(
        instruction_at < quote_at,
        "the instruction must not come from the quoted material",
    );
}

/// AC-11's redirect half.
#[test]
fn a_cross_origin_redirect_is_refused_and_the_target_is_never_contacted() {
    let mock = MockLlm::start().expect("start the mock endpoint");
    let elsewhere = MockLlm::start().expect("start the redirect target");
    mock.set_redirect(elsewhere.chat_completions_url());

    let permit = guard_for(&mock)
        .authorize_e1(&mock.chat_completions_url())
        .expect("permit");
    let error = send(&E1RequestPlan::chat_completions(
        permit,
        MODEL,
        default_body(),
    ))
    .expect_err("a redirect off the configured origin must fail");

    assert!(
        matches!(error, EgressError::CrossOriginRedirect { .. }),
        "expected a refused redirect, got {error:?}",
    );
    assert_eq!(
        error.reason_code(),
        Some(ReasonCode::E1CrossOriginRedirect),
        "the refusal has to be auditable",
    );

    assert_eq!(mock.request_count(), 1, "the first hop happened");
    assert_eq!(
        elsewhere.request_count(),
        0,
        "the redirect target must never be contacted",
    );
}

/// Only the port differs, which is the case a naive host comparison would let
/// through.
#[test]
fn a_redirect_to_another_port_on_the_same_host_is_still_cross_origin() {
    let mock = MockLlm::start().expect("start the mock endpoint");
    let other_port = MockLlm::start().expect("start a second port on the same host");
    assert_ne!(mock.port(), other_port.port());
    mock.set_redirect(other_port.chat_completions_url());

    let permit = guard_for(&mock)
        .authorize_e1(&mock.chat_completions_url())
        .expect("permit");
    let error = send(&E1RequestPlan::chat_completions(
        permit,
        MODEL,
        default_body(),
    ))
    .expect_err("a different port is a different origin");

    assert!(matches!(error, EgressError::CrossOriginRedirect { .. }));
    assert_eq!(other_port.request_count(), 0);
}

/// The control for the two redirect tests: with the redirect cleared, the same
/// request succeeds. Otherwise a client that failed on everything would pass.
#[test]
fn clearing_the_redirect_lets_the_same_request_through() {
    let mock = MockLlm::start().expect("start the mock endpoint");
    let elsewhere = MockLlm::start().expect("start the redirect target");

    mock.set_redirect(elsewhere.chat_completions_url());
    let permit = guard_for(&mock)
        .authorize_e1(&mock.chat_completions_url())
        .expect("permit");
    assert!(send(&E1RequestPlan::chat_completions(
        permit,
        MODEL,
        default_body()
    ))
    .is_err());

    mock.clear_redirect();
    let permit = guard_for(&mock)
        .authorize_e1(&mock.chat_completions_url())
        .expect("permit");
    let response = send(&E1RequestPlan::chat_completions(
        permit,
        MODEL,
        default_body(),
    ))
    .expect("the same request succeeds once the endpoint stops redirecting");
    assert_eq!(response.status, 200);
}

/// AC-21 and AC-17: with no endpoint configured, there is no permit, so there
/// is nothing to send and nothing to count.
#[test]
fn with_no_endpoint_configured_there_is_no_permit_to_send_with() {
    let mock = MockLlm::start().expect("start the mock endpoint");
    let guard = NetGuard::closed();

    let denied = guard
        .authorize_e1(&mock.chat_completions_url())
        .expect_err("nothing is configured");
    assert_eq!(denied.reason_code(), ReasonCode::E1NotConfigured);
    assert_eq!(
        mock.request_count(),
        0,
        "a refused authorization must not have opened a connection",
    );
}

/// The URL is built from the permit's origin, so it cannot be pointed
/// somewhere the guard did not approve even if the path says otherwise.
#[test]
fn the_request_url_comes_from_the_permit_not_from_the_caller() {
    let mock = MockLlm::start().expect("start the mock endpoint");
    let permit = guard_for(&mock)
        .authorize_e1(&mock.chat_completions_url())
        .expect("permit");

    let plan = E1RequestPlan::new(permit, "v1/models", MODEL, default_body());
    assert_eq!(plan.url(), format!("{}/v1/models", mock.base_url()));

    send(&plan).expect("the mock answers");
    assert_eq!(mock.requests()[0].path, "/v1/models");
}

/// `send` is the only public way this crate reaches the network, and it cannot
/// be called without a permit and a redacted body. The signature is the
/// enforcement; this reads the source back so a back door added later shows up
/// as a failing test rather than as a review someone has to remember to do.
#[test]
fn the_crate_exposes_no_way_to_send_without_a_permit() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read the egress source");

    // Free functions only — an indented `pub fn` is a method on a type this
    // crate already returns, and cannot be reached without one.
    let public_fns: Vec<&str> = source
        .lines()
        .filter(|line| line.starts_with("pub fn "))
        .collect();
    assert_eq!(
        public_fns,
        vec![
            "pub fn send(plan: &E1RequestPlan) -> Result<E1Response, EgressError> {",
            "pub fn class_of(plan: &E1RequestPlan) -> EgressClass {",
        ],
        "the public surface must stay permit-gated",
    );

    // No exposed client, and nothing that takes a bare URL.
    for back_door in [
        "pub fn client",
        "pub fn get(",
        "pub fn post(",
        "pub struct Client",
        "pub use reqwest",
        "url: &str",
    ] {
        assert!(
            !source.contains(back_door),
            "`{back_door}` would be a way around the permit",
        );
    }
}
