//! WP10's command surface end to end: the steps between a paste and a draft,
//! and the ones that stop a request.
//!
//! `soul-draft` already tests what a draft says and `soul-policy` tests each
//! gate on its own. What is checked here is the wiring — that `soulcore`
//! cannot be persuaded to skip a step by calling the commands in a different
//! order, that the body the user approved is the body that goes out, and that
//! the whole surface still has no way to send a message to anybody.

use soul_draft::draft::{DraftRequest, DraftSource};
use soul_draft::ProfileBrief;
use soul_policy::hitl::{HitlDenial, PlanHash, RequestOrigin};
use soul_policy::redactor::{ExemptionRequest, KnownIdentifiers, THIRD_PARTY_PLACEHOLDER};
use soul_policy::ReasonCode;
use soul_testkit::leakage::LeakageChecker;
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::draft::{
    Approval, DraftRefusal, DraftRefusalView, DraftSession, E1_PLAN_NOTICE, NOT_SENT_NOTICE,
    TEMPLATE_NOTICE,
};
use soulcore::commands::policy::PolicySession;

const MODEL: &str = "local-model";
const NOW_MS: u64 = 1_700_000_000_000;
const PASTED: &str = "王小明说周五的场地他已经订好了，你直接过来就行，有事打 13800138000";
const NAME: &str = "王小明";

fn identifiers() -> KnownIdentifiers {
    KnownIdentifiers::new()
        .with_name(NAME)
        .with_account("13800138000")
}

fn drafting() -> DraftSession {
    DraftSession::new(MODEL, identifiers())
}

fn pointed_at(endpoint: &MockLlm) -> PolicySession {
    PolicySession::with_user_endpoint(&endpoint.base_url(), identifiers())
        .expect("the user's endpoint parses")
}

fn a_paste() -> DraftRequest {
    DraftRequest::from_paste(ProfileBrief::neutral(), PASTED)
}

fn leakage() -> LeakageChecker {
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("pasted", PASTED);
    checker.add_known_identifier("name", NAME);
    checker.add_known_identifier("phone", "13800138000");
    checker
}

// -------------------------------------------------- the no-endpoint path ---

#[test]
fn with_nothing_configured_the_command_drafts_locally_and_reaches_nothing() {
    let listening = MockLlm::start().expect("something nobody configured");
    let mut policy = PolicySession::closed();

    let drafted = drafting()
        .draft_offline(&mut policy, &a_paste(), RequestOrigin::User, NOW_MS)
        .expect("drafting works with no key");

    assert_eq!(drafted.draft.source, DraftSource::ToneTemplate);
    assert_eq!(drafted.draft.not_sent_notice, NOT_SENT_NOTICE);
    assert_eq!(drafted.draft.source_notice, TEMPLATE_NOTICE);
    assert_eq!(listening.request_count(), 0);

    // The entry the chain is owed carries counts and codes, never the paste.
    assert_eq!(drafted.audit.len(), 1);
    let entry = drafted.audit[0]
        .clone()
        .into_entry(uuid::Uuid::now_v7(), 1_700_000_000)
        .expect("the audit content is prose-free");
    leakage().assert_clean(
        "the draft audit entry",
        &serde_json::to_string(&entry).expect("serialize"),
    );
}

// ------------------------------------------------ the two-step E1 path ---

#[test]
fn preparing_describes_a_request_without_making_one() {
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    let plan = drafting
        .prepare(&mut policy, a_paste(), None, RequestOrigin::User, NOW_MS)
        .expect("a plan the user can read");

    assert_eq!(
        endpoint.request_count(),
        0,
        "describing a request is not making one",
    );
    assert_eq!(plan.model, MODEL);
    assert_eq!(plan.third_party_turns, 1);
    assert_eq!(plan.placeheld_turns, 1);
    assert!(!plan.carries_exempted_original);
    assert_eq!(plan.notice, E1_PLAN_NOTICE);
    assert_eq!(plan.not_sent_notice, NOT_SENT_NOTICE);

    // The plan is what the user is asked about, so it must not be readable as
    // the conversation.
    leakage().assert_clean(
        "the plan shown to the user",
        &serde_json::to_string(&plan).expect("serialize"),
    );
    assert!(!plan.plan_hash.is_empty());
}

#[test]
fn approving_the_plan_sends_exactly_the_body_that_was_described() {
    let endpoint = MockLlm::start().expect("the endpoint the user configured");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    let plan = drafting
        .prepare(&mut policy, a_paste(), None, RequestOrigin::User, NOW_MS)
        .expect("a plan");
    let drafted = drafting
        .generate(&mut policy, &plan.approval(), NOW_MS)
        .expect("the endpoint answers");

    assert_eq!(endpoint.request_count(), 1);
    let sent = &endpoint.requests()[0];
    leakage().assert_clean("the request body", &sent.body);
    assert!(sent.body.contains(THIRD_PARTY_PLACEHOLDER));

    assert_eq!(drafted.draft.third_party_turns, plan.third_party_turns);
    assert_eq!(drafted.draft.placeheld_turns, plan.placeheld_turns);
    assert_eq!(drafted.draft.not_sent_notice, NOT_SENT_NOTICE);

    // One entry for the request that left, one for the draft that came back.
    assert_eq!(drafted.audit.len(), 2);
    for content in &drafted.audit {
        content
            .clone()
            .into_entry(uuid::Uuid::now_v7(), 1_700_000_000)
            .expect("prose-free");
    }
}

#[test]
fn a_prepared_body_can_be_sent_once_and_not_twice() {
    let endpoint = MockLlm::start().expect("the endpoint");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    let plan = drafting
        .prepare(&mut policy, a_paste(), None, RequestOrigin::User, NOW_MS)
        .expect("a plan");
    drafting
        .generate(&mut policy, &plan.approval(), NOW_MS)
        .expect("the first send");

    let refused = drafting
        .generate(&mut policy, &plan.approval(), NOW_MS)
        .expect_err("the body was consumed by the first call");
    assert!(matches!(refused, DraftRefusal::NothingPrepared));
    assert_eq!(endpoint.request_count(), 1, "one approval, one request");
}

#[test]
fn approving_a_shape_that_is_not_the_prepared_one_sends_nothing() {
    let endpoint = MockLlm::start().expect("the endpoint");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    let plan = drafting
        .prepare(&mut policy, a_paste(), None, RequestOrigin::User, NOW_MS)
        .expect("a plan");
    let refused = drafting
        .generate(
            &mut policy,
            &Approval {
                plan_hash: PlanHash::from_hex("00").as_str().to_owned(),
                ..plan.approval()
            },
            NOW_MS,
        )
        .expect_err("that is not the plan that was prepared");

    assert!(matches!(
        refused,
        DraftRefusal::Hitl(HitlDenial::PlanHashMismatch { .. }),
    ));
    assert_eq!(refused.reason_code(), ReasonCode::PlanHashMismatch);
    assert_eq!(endpoint.request_count(), 0);
}

/// Two pastes of the same shape produce the same plan hash — the plan
/// carries counts and no prose, on purpose. So the hash alone cannot say
/// which message is about to go out, and the preparation id is what does.
///
/// Without it, this test would send the second message under an approval the
/// user gave for the first, and every assertion about the hash would still
/// pass.
#[test]
fn an_approval_for_one_message_does_not_send_a_different_one_of_the_same_shape() {
    let endpoint = MockLlm::start().expect("the endpoint");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    let first = drafting
        .prepare(&mut policy, a_paste(), None, RequestOrigin::User, NOW_MS)
        .expect("a plan");
    let second = drafting
        .prepare(
            &mut policy,
            DraftRequest::from_paste(ProfileBrief::neutral(), "改一下：周六行吗，周五我有事"),
            None,
            RequestOrigin::User,
            NOW_MS,
        )
        .expect("a second plan replaces the first");

    assert_eq!(
        first.plan_hash, second.plan_hash,
        "the shapes really are identical, which is what makes the id load-bearing",
    );
    assert_ne!(first.preparation_id, second.preparation_id);

    let refused = drafting
        .generate(&mut policy, &first.approval(), NOW_MS)
        .expect_err("the user approved the first message, not this one");
    assert!(matches!(refused, DraftRefusal::NotThePreparedRequest));
    assert_eq!(endpoint.request_count(), 0);
}

/// A second preparation replaces the first, so an unapproved body cannot sit
/// around waiting for an approval meant for another one.
#[test]
fn only_the_preparation_on_screen_can_be_approved() {
    let endpoint = MockLlm::start().expect("the endpoint");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    drafting
        .prepare(&mut policy, a_paste(), None, RequestOrigin::User, NOW_MS)
        .expect("a plan");
    let current = drafting
        .prepare(
            &mut policy,
            DraftRequest::from_paste(ProfileBrief::neutral(), "改一下：周六行吗，周五我有事"),
            None,
            RequestOrigin::User,
            NOW_MS,
        )
        .expect("a second plan");

    assert_eq!(
        drafting.pending_plan_hash().map(PlanHash::as_str),
        Some(current.plan_hash.as_str()),
    );
    drafting
        .generate(&mut policy, &current.approval(), NOW_MS)
        .expect("the plan the user is looking at approves");
    assert_eq!(endpoint.request_count(), 1);
}

#[test]
fn saying_no_to_a_plan_throws_the_body_away() {
    let endpoint = MockLlm::start().expect("the endpoint");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    let plan = drafting
        .prepare(&mut policy, a_paste(), None, RequestOrigin::User, NOW_MS)
        .expect("a plan");
    assert!(drafting.discard(), "there was something to discard");
    assert!(!drafting.discard(), "and now there is not");
    assert!(drafting.pending_plan_hash().is_none());

    let refused = drafting
        .generate(&mut policy, &plan.approval(), NOW_MS)
        .expect_err("the user said no");
    assert!(matches!(refused, DraftRefusal::NothingPrepared));
    assert_eq!(endpoint.request_count(), 0);
}

/// AC-13 across the command surface: the exemption is spent by the first
/// preparation and the next one places the turn back behind a placeholder.
#[test]
fn an_exemption_covers_one_prepared_request_and_no_more() {
    let endpoint = MockLlm::start().expect("the endpoint");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    let request = a_paste();
    let exemption = ExemptionRequest::for_turn(request.third_party_turn_ids()[0])
        .confirm(true)
        .expect("the user confirmed twice");

    let exempted = drafting
        .prepare(
            &mut policy,
            request,
            Some(exemption),
            RequestOrigin::User,
            NOW_MS,
        )
        .expect("a plan");
    assert!(exempted.carries_exempted_original);
    assert_eq!(exempted.placeheld_turns, 0);
    drafting
        .generate(&mut policy, &exempted.approval(), NOW_MS)
        .expect("sent");

    let after = drafting
        .prepare(&mut policy, a_paste(), None, RequestOrigin::User, NOW_MS)
        .expect("a plan");
    assert!(!after.carries_exempted_original);
    assert_eq!(after.placeheld_turns, 1);
    assert_ne!(
        after.plan_hash, exempted.plan_hash,
        "a differently redacted body is a different plan",
    );

    drafting
        .generate(&mut policy, &after.approval(), NOW_MS)
        .expect("sent");

    let sent = endpoint.requests();
    assert_eq!(sent.len(), 2);
    assert!(!sent[0].body.contains(THIRD_PARTY_PLACEHOLDER));
    assert!(sent[1].body.contains(THIRD_PARTY_PLACEHOLDER));
    // Even the exempted request keeps the name and the number out.
    assert!(!sent[0].body.contains(NAME));
    assert!(!sent[0].body.contains("13800138000"));
    leakage().assert_clean("the request after the exemption", &sent[1].body);
}

// ---------------------------------------------------------------- AC-25 ---

#[test]
fn a_command_reached_from_pasted_content_is_refused() {
    let endpoint = MockLlm::start().expect("the endpoint");
    let mut policy = pointed_at(&endpoint);
    let mut drafting = drafting();

    for refused in [
        drafting
            .draft_offline(
                &mut policy,
                &a_paste(),
                RequestOrigin::ExternalContent,
                NOW_MS,
            )
            .map(|_| ())
            .expect_err("external content is never authority"),
        drafting
            .prepare(
                &mut policy,
                a_paste(),
                None,
                RequestOrigin::ExternalContent,
                NOW_MS,
            )
            .map(|_| ())
            .expect_err("external content is never authority"),
    ] {
        assert_eq!(
            refused.reason_code(),
            ReasonCode::ExternalContentNotAuthority,
        );
    }

    assert!(
        drafting.pending_plan_hash().is_none(),
        "a refused preparation must not leave a body behind",
    );
    assert_eq!(endpoint.request_count(), 0);
}

// ------------------------------------------------- AC-16, through a store ---

/// The whole chain the shell would run: import a file, derive the graph,
/// summarize one person from it.
///
/// Worth doing here rather than only in `soul-draft` because the evidence has
/// to survive the encrypted store and come back resolvable. A summary whose
/// points cite rows the store cannot return is a summary with nothing behind
/// it, and that is exactly the failure AC-16 is about.
#[test]
fn a_person_summary_off_a_real_import_cites_rows_the_store_returns() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store =
        soulcore::commands::store::open_test_store(dir.path(), "soulcore draft commands")
            .expect("open");

    let text = soul_testkit::fixtures::read_text("import/soul-import-v1/three_partners.jsonl")
        .expect("fixture");
    let staged = soulcore::commands::import::read_soul_import_v1(&text).expect("valid");
    soulcore::commands::import::commit(&mut store, &staged, 1_787_500_000).expect("commit");
    soulcore::commands::graph::rebuild(&mut store, 1_787_500_000).expect("rebuild");

    let graph = soulcore::commands::graph::load(&store).expect("load");
    let someone = graph
        .third_party_nodes()
        .into_iter()
        .max_by_key(|node| node.interaction_count)
        .expect("the import has partners in it")
        .contact_id;

    let mut policy = PolicySession::closed();
    let view = soulcore::commands::draft::summarize_person(
        &mut policy,
        &store,
        someone,
        RequestOrigin::User,
        NOW_MS,
    )
    .expect("a summary");

    assert_eq!(view.contact_id, someone.to_string());
    assert_eq!(view.source, "counts");
    assert!(!view.clinical_claim);
    assert_eq!(view.notice, "工作假设，非临床结论");
    assert!(!view.points.is_empty());

    for point in &view.points {
        assert!(
            !point.evidence_ids.is_empty(),
            "`{}` cites nothing",
            point.statement,
        );
        for cited in &point.evidence_ids {
            let evidence_id = cited.parse().expect("a uuid");
            soul_store_api::ProfileStore::get_evidence(&store, evidence_id)
                .expect("the cited row comes back out of the store");
        }
        assert!(view.text.contains(&point.statement));
    }
    soul_policy::assert_non_clinical(&view.text).expect("nothing a medical product would say");

    // The summary refers to its subject rather than naming them.
    assert!(view.text.contains("这个人"));
    assert!(!view.text.contains(&someone.to_string()));
}

#[test]
fn a_summary_of_somebody_the_graph_has_never_seen_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = soulcore::commands::store::open_test_store(dir.path(), "soulcore draft strangers")
        .expect("open");
    let mut policy = PolicySession::closed();

    let refused = soulcore::commands::draft::summarize_person(
        &mut policy,
        &store,
        uuid::Uuid::now_v7(),
        RequestOrigin::User,
        NOW_MS,
    )
    .expect_err("there is nothing to summarize");
    assert!(matches!(refused, DraftRefusal::Draft(_)));
}

#[test]
fn asking_for_a_summary_from_pasted_content_is_refused() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = soulcore::commands::store::open_test_store(dir.path(), "soulcore draft origin")
        .expect("open");
    let mut policy = PolicySession::closed();

    let refused = soulcore::commands::draft::summarize_person(
        &mut policy,
        &store,
        uuid::Uuid::now_v7(),
        RequestOrigin::ExternalContent,
        NOW_MS,
    )
    .expect_err("external content is never authority");
    assert_eq!(
        refused.reason_code(),
        ReasonCode::ExternalContentNotAuthority,
    );
}

// ------------------------------------------------------ what the UI sees ---

#[test]
fn a_refusal_the_webview_sees_is_a_code_and_a_sentence() {
    let view = DraftRefusalView::of(&DraftRefusal::NothingPrepared);
    assert_eq!(view.reason_code, ReasonCode::PlanHashMismatch.as_str());
    assert!(!view.explanation.is_empty());

    // Round-trips, so the interface cannot be handed a field the core does
    // not know about.
    let value = serde_json::to_value(&view).expect("serialize");
    let decoded: DraftRefusalView = serde_json::from_value(value).expect("round trip");
    assert_eq!(decoded, view);
}

/// The surface has no way to send a draft to a person, and the check is a
/// search rather than a habit.
#[test]
fn this_command_surface_offers_no_way_to_send_a_message() {
    const SOURCE: &str = include_str!("../src/commands/draft.rs");

    for forbidden in [
        "pub fn send",
        "pub fn deliver",
        "pub fn post",
        "recipient",
        "to_contact",
        "phone_number",
    ] {
        assert!(
            !SOURCE.contains(forbidden),
            "`{forbidden}` appears in the drafting command surface",
        );
    }
    assert!(SOURCE.contains("pub fn draft_offline"));
    assert!(SOURCE.contains("pub fn generate"));
    assert!(
        SOURCE.len() > 2_000,
        "the include above read the wrong file",
    );
}
