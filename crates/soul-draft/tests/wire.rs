//! AC-11, AC-12 and AC-13, checked against what a server received.
//!
//! Everything here goes through a real socket to `soul-testkit`'s loopback
//! endpoint, because the claims are about bytes that left the process and a
//! test that re-serialized the plan would be inspecting its own copy. The
//! mock keeps request bodies verbatim; the assertions read those.
//!
//! The generator below is the seam WP10 leaves for the network, filled in the
//! way `soulcore` fills it: authorize with the guard, build the plan around
//! the permit, hand it to `soul-egress`. It is nine lines and there is
//! nowhere in them to name a host — the URL comes off the permit — which is
//! the shape AC-11 is asking for.

use soul_policy::e1::{E1RequestPlan, DRAFTING_INSTRUCTION, QUOTE_CLOSE, QUOTE_OPEN};
use soul_policy::net_guard::{EgressConfig, NetGuard};
use soul_policy::redactor::{
    ExemptionRequest, KnownIdentifiers, RedactedBody, Redactor, ACCOUNT_PLACEHOLDER,
    NAME_PLACEHOLDER, THIRD_PARTY_PLACEHOLDER,
};
use soul_policy::ReasonCode;
use soul_testkit::leakage::{LeakageChecker, LeakageKind};
use soul_testkit::MockLlm;
use uuid::Uuid;

use soul_draft::brief::{ProfileBrief, BRIEF_HEADING};
use soul_draft::draft::{Degradation, DraftRequest, DraftSource, Drafter, ReplyGenerator};
use soul_draft::error::GenerationRefused;

const MODEL: &str = "local-model";

/// One third-party message with a name, a number and enough prose to trip the
/// eight-scalar rule. Nothing in it is a diagnostic term.
const PASTED: &str = "李雷说他明天上午十点在公司门口等你，别忘了把材料带齐，有事打 13800138000";

/// The name and number a contact graph would already know about.
fn known() -> KnownIdentifiers {
    KnownIdentifiers::new()
        .with_name("李雷")
        .with_account("13800138000")
}

fn drafter() -> Drafter {
    Drafter::new(Redactor::new(known()), MODEL)
}

fn leakage() -> LeakageChecker {
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("pasted", PASTED);
    checker.add_known_identifier("name", "李雷");
    checker.add_known_identifier("phone", "13800138000");
    checker
}

/// The seam, filled the way the shell fills it.
///
/// There is no field here holding a host name: `target` is what the caller
/// asked for and the guard has to agree with it, and the URL that is actually
/// dialled is built from the permit, not from this struct.
#[derive(Debug)]
struct Endpoint {
    guard: NetGuard,
    target: String,
}

impl Endpoint {
    /// A guard configured for exactly the mock's origin.
    fn pointed_at(mock: &MockLlm) -> Endpoint {
        Endpoint {
            guard: NetGuard::new(
                EgressConfig::with_user_endpoint(&mock.base_url()).expect("a loopback origin"),
            ),
            target: mock.chat_completions_url(),
        }
    }

    /// A guard configured for `configured`, asked to reach `wanted`.
    fn configured_for(configured: &MockLlm, wanted: &MockLlm) -> Endpoint {
        Endpoint {
            guard: NetGuard::new(
                EgressConfig::with_user_endpoint(&configured.base_url())
                    .expect("a loopback origin"),
            ),
            target: wanted.chat_completions_url(),
        }
    }
}

impl ReplyGenerator for Endpoint {
    fn generate(&mut self, body: RedactedBody) -> Result<String, GenerationRefused> {
        let permit = self.guard.authorize_e1(&self.target).map_err(|denied| {
            GenerationRefused::new(denied.to_string()).because(denied.reason_code())
        })?;
        let plan = E1RequestPlan::chat_completions(permit, MODEL, body);
        let response = soul_egress::send(&plan).map_err(|error| {
            let refused = GenerationRefused::new(error.to_string());
            match error.reason_code() {
                Some(code) => refused.because(code),
                None => refused,
            }
        })?;
        Ok(response.body)
    }
}

/// The `messages` array as the server parsed it back out of the wire body.
fn messages(raw: &str) -> Vec<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(raw).expect("the mock received JSON");
    value["messages"]
        .as_array()
        .expect("a messages array")
        .iter()
        .map(|message| {
            (
                message["role"].as_str().expect("a role").to_owned(),
                message["content"].as_str().expect("content").to_owned(),
            )
        })
        .collect()
}

// ---------------------------------------------------------------- AC-12 ---

#[test]
fn the_third_partys_words_do_not_reach_the_wire() {
    let mock = MockLlm::start().expect("a loopback endpoint");
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), PASTED);

    let draft = drafter()
        .draft_with(&request, None, &mut Endpoint::pointed_at(&mock))
        .expect("the endpoint was reached");

    let recorded = mock.requests();
    assert_eq!(recorded.len(), 1, "one draft, one request");
    let sent = &recorded[0];
    assert_eq!(sent.path, "/v1/chat/completions");

    leakage().assert_clean("the default E1 body", &sent.body);
    assert!(
        sent.body.contains(THIRD_PARTY_PLACEHOLDER),
        "the turn is present as a placeholder, not simply dropped: {}",
        sent.body,
    );
    // The brief did travel, so "clean" above is not clean-because-empty.
    assert!(sent.body.contains(BRIEF_HEADING));

    assert_eq!(draft.third_party_turns, 1);
    assert_eq!(draft.placeheld_turns, 1);
    assert!(!draft.carries_exempted_original);

    // The mock answers with an empty assistant message, so the user still gets
    // a draft and is told where it came from rather than being shown nothing.
    assert_eq!(draft.source, DraftSource::ToneTemplate);
    assert_eq!(draft.degraded, Some(Degradation::ReplyEmpty));
}

#[test]
fn the_instruction_slot_holds_the_constant_and_the_paste_is_quoted_material() {
    let mock = MockLlm::start().expect("a loopback endpoint");
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), PASTED);
    drafter()
        .draft_with(&request, None, &mut Endpoint::pointed_at(&mock))
        .expect("the endpoint was reached");

    let sent = mock.requests().remove(0);
    let messages = messages(&sent.body);
    assert_eq!(messages.len(), 2, "one instruction, one material slot");

    let (role, content) = &messages[0];
    assert_eq!(role, "system");
    assert_eq!(
        content, DRAFTING_INSTRUCTION,
        "the instruction is the constant in soul-policy and nothing else",
    );

    let (role, content) = &messages[1];
    assert_eq!(role, "user");
    assert!(content.starts_with(QUOTE_OPEN));
    assert!(content.trim_end().ends_with(QUOTE_CLOSE));
    assert!(
        content.contains(BRIEF_HEADING),
        "the profile brief travels as material, above the conversation",
    );
}

// ---------------------------------------------------------------- AC-11 ---

#[test]
fn a_different_port_on_the_same_host_is_a_different_origin() {
    let configured = MockLlm::start().expect("the endpoint the user typed in");
    let other = MockLlm::start().expect("something else listening on loopback");

    let request = DraftRequest::from_paste(ProfileBrief::neutral(), PASTED);
    let error = drafter()
        .draft_with(
            &request,
            None,
            &mut Endpoint::configured_for(&configured, &other),
        )
        .expect_err("a second loopback port is not the configured endpoint");

    assert_eq!(error.reason_code(), Some(ReasonCode::E1OriginMismatch));
    assert_eq!(other.request_count(), 0, "no socket was opened to it");
    assert_eq!(configured.request_count(), 0);
}

#[test]
fn a_redirect_off_the_configured_origin_is_refused_and_the_target_never_hears_from_us() {
    let configured = MockLlm::start().expect("the endpoint the user typed in");
    let elsewhere = MockLlm::start().expect("wherever the redirect points");
    configured.set_redirect(elsewhere.chat_completions_url());

    let request = DraftRequest::from_paste(ProfileBrief::neutral(), PASTED);
    let error = drafter()
        .draft_with(&request, None, &mut Endpoint::pointed_at(&configured))
        .expect_err("the redirect leaves the permitted origin");

    assert_eq!(error.reason_code(), Some(ReasonCode::E1CrossOriginRedirect));
    assert_eq!(
        configured.request_count(),
        1,
        "the first hop is the one the user authorized",
    );
    assert_eq!(
        elsewhere.request_count(),
        0,
        "the second hop is the one AC-11 exists to prevent",
    );
}

#[test]
fn with_no_endpoint_configured_the_draft_path_refuses_before_a_socket_opens() {
    let mock = MockLlm::start().expect("a loopback endpoint nobody configured");
    let mut endpoint = Endpoint {
        guard: NetGuard::closed(),
        target: mock.chat_completions_url(),
    };

    let request = DraftRequest::from_paste(ProfileBrief::neutral(), PASTED);
    let error = drafter()
        .draft_with(&request, None, &mut endpoint)
        .expect_err("a closed guard authorizes nothing");

    assert_eq!(error.reason_code(), Some(ReasonCode::E1NotConfigured));
    assert_eq!(mock.request_count(), 0);

    // And the user is not left without a draft: the no-key path is the one the
    // shell takes when there is no endpoint at all.
    let offline = drafter().draft_offline(&request).expect("a template draft");
    assert_eq!(offline.source, DraftSource::ToneTemplate);
    assert_eq!(mock.request_count(), 0);
}

// ---------------------------------------------------------------- AC-13 ---

#[test]
fn one_exemption_covers_one_request_and_the_next_is_placeheld_again() {
    let mock = MockLlm::start().expect("a loopback endpoint");
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), PASTED);
    let turn_id = request.third_party_turn_ids()[0];
    let drafter = drafter();

    // The user is asked, and answers twice.
    let exemption = ExemptionRequest::for_turn(turn_id)
        .confirm(true)
        .expect("the second confirmation is what produces the exemption");
    let exempted = drafter
        .draft_with(&request, Some(exemption), &mut Endpoint::pointed_at(&mock))
        .expect("the endpoint was reached");
    assert!(exempted.carries_exempted_original);
    assert_eq!(exempted.placeheld_turns, 0);

    // The same request again, with nothing carried over. There is no argument
    // to omit here that would have made the exemption persist — it was moved
    // into the call above and dropped there.
    let after = drafter
        .draft_with(&request, None, &mut Endpoint::pointed_at(&mock))
        .expect("the endpoint was reached");
    assert!(!after.carries_exempted_original);
    assert_eq!(after.placeheld_turns, 1);

    let sent = mock.requests();
    assert_eq!(sent.len(), 2);

    // Request one carries the original, which is what the user confirmed for.
    let findings = leakage().inspect(&sent[0].body);
    assert!(
        findings
            .iter()
            .any(|finding| finding.kind == LeakageKind::ThirdPartyNgram),
        "the exempted request is supposed to carry the original prose",
    );
    assert!(!sent[0].body.contains(THIRD_PARTY_PLACEHOLDER));

    // Request two is placeheld again, without anyone having asked for that.
    leakage().assert_clean("the request after an exemption", &sent[1].body);
    assert!(sent[1].body.contains(THIRD_PARTY_PLACEHOLDER));
}

#[test]
fn an_exemption_names_one_turn_and_not_the_conversation() {
    let mock = MockLlm::start().expect("a loopback endpoint");
    let other = "王芳刚才在群里问方案什么时候能定下来，说客户那边催得紧";
    let request = DraftRequest::new(
        ProfileBrief::neutral(),
        vec![
            soul_policy::Turn::new(
                Uuid::now_v7(),
                soul_schema::common::SealedSubject::ThirdParty,
                PASTED,
            ),
            soul_policy::Turn::new(
                Uuid::now_v7(),
                soul_schema::common::SealedSubject::ThirdParty,
                other,
            ),
        ],
    );
    let first = request.third_party_turn_ids()[0];

    let exemption = ExemptionRequest::for_turn(first)
        .confirm(true)
        .expect("confirmed");
    let draft = drafter()
        .draft_with(&request, Some(exemption), &mut Endpoint::pointed_at(&mock))
        .expect("the endpoint was reached");

    assert_eq!(draft.third_party_turns, 2);
    assert_eq!(
        draft.placeheld_turns, 1,
        "the turn nobody confirmed for is still placeheld",
    );

    let body = &mock.requests()[0].body;
    assert!(body.contains(THIRD_PARTY_PLACEHOLDER));

    let mut second = LeakageChecker::new();
    second.add_third_party_body("other", other);
    second.assert_clean("the turn outside the exemption", body);
}

#[test]
fn an_exempted_body_still_placeholds_the_name_and_the_number() {
    let mock = MockLlm::start().expect("a loopback endpoint");
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), PASTED);
    let exemption = ExemptionRequest::for_turn(request.third_party_turn_ids()[0])
        .confirm(true)
        .expect("confirmed");

    drafter()
        .draft_with(&request, Some(exemption), &mut Endpoint::pointed_at(&mock))
        .expect("the endpoint was reached");

    let body = &mock.requests()[0].body;
    assert!(
        !body.contains("李雷"),
        "confirming to send one message is not confirming to publish a name",
    );
    assert!(!body.contains("13800138000"));
    assert!(body.contains(NAME_PLACEHOLDER));
    assert!(body.contains(ACCOUNT_PLACEHOLDER));

    let findings = leakage().inspect(body);
    assert!(
        findings
            .iter()
            .all(|finding| finding.kind == LeakageKind::ThirdPartyNgram),
        "prose yes, identifiers no: {findings:#?}",
    );
}

#[test]
fn declining_the_second_confirmation_leaves_no_exemption() {
    assert!(
        ExemptionRequest::for_turn(Uuid::now_v7())
            .confirm(false)
            .is_none(),
        "one answer is not two",
    );
}
