//! AC-19: an unknown action, an approved plan that changed, and a replayed
//! token are all refused.
//!
//! The acceptance matrix asks for those three. DECISIONS D24 and the v0.1
//! scope add three more that belong to the same gate and are tested here:
//! a token is scoped, a token expires, and a file-write token is refused even
//! when it is otherwise perfect — v0.1 has no write implementation, and a
//! token that would start working the day one appears is a trap.
//!
//! Every refusal is checked for its `ReasonCode` as well as its variant,
//! because the reason code is what reaches the audit chain and a refusal that
//! records the wrong reason is a refusal nobody can review.

use uuid::Uuid;

use soul_policy::audit::ReasonCode;
use soul_policy::hitl::{
    check_action, ActionKind, ActionRequest, CapabilityScope, HitlDenial, PlanHash, RequestOrigin,
    TokenError, TokenIssuer, DEFAULT_TOKEN_TTL_MS,
};

const NOW: u64 = 1_787_529_600_000;

fn plan() -> serde_json::Value {
    serde_json::json!({
        "kind": "forget",
        "unit": "memory",
        "content_keys": 2,
    })
}

fn edited_plan() -> serde_json::Value {
    serde_json::json!({
        "kind": "forget",
        "unit": "memory",
        "content_keys": 3,
    })
}

fn forget_request(token_id: Uuid, plan: serde_json::Value) -> ActionRequest {
    let hash = PlanHash::of(&plan);
    ActionRequest::new(ActionKind::ExecuteForget.as_str(), RequestOrigin::User)
        .with_plan(plan)
        .approved_as(hash)
        .with_token(token_id)
}

// ------------------------------------------------------- unknown actions ---

#[test]
fn an_unknown_action_is_refused() {
    let mut issuer = TokenIssuer::new();
    for name in [
        "shell.exec",
        "file.write",
        "message.send",
        "cloud.upload",
        "draft.reply.v2",
        "",
        "DRAFT.REPLY",
    ] {
        let request = ActionRequest::new(name, RequestOrigin::User);
        let denial = check_action(&mut issuer, &request, NOW)
            .expect_err(&format!("`{name}` must not be a known action"));
        assert!(matches!(denial, HitlDenial::UnknownAction(_)), "{name}");
        assert_eq!(denial.reason_code(), ReasonCode::UnknownAction);
    }
}

#[test]
fn every_known_action_parses_back_to_itself_and_nothing_else_does() {
    for kind in ActionKind::ALL {
        assert_eq!(ActionKind::parse(kind.as_str()), Some(*kind));
    }
    assert_eq!(ActionKind::parse("file.write"), None);
    assert_eq!(
        ActionKind::ALL.len(),
        8,
        "adding an action is a scope decision, not a refactor",
    );
}

#[test]
fn an_action_that_needs_no_token_still_has_to_be_known() {
    let mut issuer = TokenIssuer::new();
    let approved = check_action(
        &mut issuer,
        &ActionRequest::new(ActionKind::DraftReply.as_str(), RequestOrigin::User),
        NOW,
    )
    .expect("drafting is a known action");
    assert_eq!(approved.kind, ActionKind::DraftReply);
    assert!(approved.spent_token.is_none());
    assert_eq!(issuer.issued_count(), 0);
}

// ------------------------------------------------------------ plan hashes ---

#[test]
fn a_plan_edited_after_approval_is_refused() {
    let mut issuer = TokenIssuer::new();
    let approved_hash = PlanHash::of(&plan());
    let token = issuer.issue(CapabilityScope::ForgetExecute, approved_hash.clone(), NOW);

    let request = ActionRequest::new(ActionKind::ExecuteForget.as_str(), RequestOrigin::User)
        .with_plan(edited_plan())
        .approved_as(approved_hash)
        .with_token(token.token_id());

    let denial = check_action(&mut issuer, &request, NOW).expect_err("the plan changed");
    assert!(matches!(denial, HitlDenial::PlanHashMismatch { .. }));
    assert_eq!(denial.reason_code(), ReasonCode::PlanHashMismatch);
}

/// The hash has to be over the plan's content, not its spelling: two encodings
/// of the same plan must agree, and one changed field must not.
#[test]
fn the_plan_hash_follows_the_content() {
    let same = serde_json::json!({ "unit": "memory", "kind": "forget", "content_keys": 2 });
    assert_eq!(PlanHash::of(&plan()), PlanHash::of(&same));
    assert_ne!(PlanHash::of(&plan()), PlanHash::of(&edited_plan()));
    assert_eq!(PlanHash::of(&plan()).as_str().len(), 64);
}

/// A token is bound to the plan it was approved for, so a token minted for one
/// plan cannot be spent on another even if the caller claims the right hash.
#[test]
fn a_token_is_bound_to_the_plan_it_was_issued_for() {
    let mut issuer = TokenIssuer::new();
    let token = issuer.issue(CapabilityScope::ForgetExecute, PlanHash::of(&plan()), NOW);

    let request = forget_request(token.token_id(), edited_plan());
    let denial = check_action(&mut issuer, &request, NOW).expect_err("a different plan");
    assert_eq!(denial.reason_code(), ReasonCode::PlanHashMismatch);
}

// --------------------------------------------------------------- tokens ---

#[test]
fn a_capability_token_works_once_and_a_replay_is_refused() {
    let mut issuer = TokenIssuer::new();
    let token = issuer.issue(CapabilityScope::ForgetExecute, PlanHash::of(&plan()), NOW);
    let token_id = token.token_id();

    let approved = check_action(&mut issuer, &forget_request(token_id, plan()), NOW)
        .expect("the first use must be honoured");
    assert_eq!(
        approved.spent_token.expect("a token was spent").token_id(),
        token_id,
    );
    assert!(issuer.is_spent(token_id));

    let denial = check_action(&mut issuer, &forget_request(token_id, plan()), NOW)
        .expect_err("the second use must be refused");
    assert!(matches!(denial, HitlDenial::Token(TokenError::Replayed(_)),));
    assert_eq!(denial.reason_code(), ReasonCode::TokenReplayed);
}

/// A refused presentation still burns the token. Otherwise a caller could
/// probe: present against one plan, learn it is wrong, present against
/// another.
#[test]
fn a_refused_presentation_still_burns_the_token() {
    let mut issuer = TokenIssuer::new();
    let token = issuer.issue(CapabilityScope::ForgetExecute, PlanHash::of(&plan()), NOW);
    let token_id = token.token_id();

    let first = check_action(&mut issuer, &forget_request(token_id, edited_plan()), NOW)
        .expect_err("wrong plan");
    assert_eq!(first.reason_code(), ReasonCode::PlanHashMismatch);

    let second = check_action(&mut issuer, &forget_request(token_id, plan()), NOW)
        .expect_err("the token is gone, right plan or not");
    assert_eq!(second.reason_code(), ReasonCode::TokenReplayed);
}

#[test]
fn an_expired_token_is_refused() {
    let mut issuer = TokenIssuer::new();
    let token = issuer.issue(CapabilityScope::ForgetExecute, PlanHash::of(&plan()), NOW);

    assert!(!token.is_expired_at(NOW + DEFAULT_TOKEN_TTL_MS - 1));
    assert!(token.is_expired_at(NOW + DEFAULT_TOKEN_TTL_MS));

    let denial = check_action(
        &mut issuer,
        &forget_request(token.token_id(), plan()),
        NOW + DEFAULT_TOKEN_TTL_MS,
    )
    .expect_err("the token has expired");
    assert_eq!(denial.reason_code(), ReasonCode::TokenExpired);
}

#[test]
fn a_token_issued_for_another_scope_is_refused() {
    let mut issuer = TokenIssuer::new();
    let token = issuer.issue(CapabilityScope::E1Generate, PlanHash::of(&plan()), NOW);

    let denial = check_action(&mut issuer, &forget_request(token.token_id(), plan()), NOW)
        .expect_err("a generation token does not authorize a forget");
    assert_eq!(denial.reason_code(), ReasonCode::TokenScopeMismatch);
}

#[test]
fn a_token_this_ledger_never_issued_is_refused() {
    let mut issuer = TokenIssuer::new();
    let denial = check_action(&mut issuer, &forget_request(Uuid::now_v7(), plan()), NOW)
        .expect_err("an invented token id");
    assert_eq!(denial.reason_code(), ReasonCode::TokenUnknown);
}

#[test]
fn an_action_that_needs_a_token_is_refused_without_one() {
    let mut issuer = TokenIssuer::new();
    let request = ActionRequest::new(ActionKind::ExecuteForget.as_str(), RequestOrigin::User)
        .with_plan(plan());
    let denial = check_action(&mut issuer, &request, NOW).expect_err("no token was presented");
    assert_eq!(denial.reason_code(), ReasonCode::TokenRequired);
}

/// v0.1 refuses a file-write token on presentation, not at some later gate,
/// and refuses it even though it is unexpired, unspent and correctly scoped.
#[test]
fn a_file_write_token_is_refused_even_when_it_is_otherwise_perfect() {
    let mut issuer = TokenIssuer::new();
    let plan_hash = PlanHash::of(&plan());
    let token = issuer.issue(CapabilityScope::FileWrite, plan_hash.clone(), NOW);

    let error = issuer
        .consume(
            token.token_id(),
            CapabilityScope::FileWrite,
            &plan_hash,
            NOW,
        )
        .expect_err("v0.1 has no write implementation");
    assert!(matches!(error, TokenError::WriteNotImplemented(_)));
    assert_eq!(error.reason_code(), ReasonCode::WriteNotImplemented);
    assert!(
        issuer.is_spent(token.token_id()),
        "a refused write token is still burnt",
    );
    assert!(!CapabilityScope::FileWrite.is_implemented_in_v0_1());
}

/// No action maps to the file-write scope, so a write cannot be reached
/// through the gate at all — the refusal above is the second line, not the
/// first.
#[test]
fn no_known_action_asks_for_the_file_write_scope() {
    for kind in ActionKind::ALL {
        assert_ne!(
            kind.as_str(),
            CapabilityScope::FileWrite.as_str(),
            "v0.1 exposes no action that performs a file write",
        );
    }
    assert!(ActionKind::parse(CapabilityScope::FileWrite.as_str()).is_none());
}

// ------------------------------------------------------------- provenance ---

/// AC-25's other half: content that arrived from outside cannot authorize
/// anything, even when it names a known action and carries a valid token.
#[test]
fn external_content_cannot_authorize_a_known_action() {
    let mut issuer = TokenIssuer::new();
    let token = issuer.issue(CapabilityScope::ForgetExecute, PlanHash::of(&plan()), NOW);

    let request = ActionRequest::new(
        ActionKind::ExecuteForget.as_str(),
        RequestOrigin::ExternalContent,
    )
    .with_plan(plan())
    .approved_as(PlanHash::of(&plan()))
    .with_token(token.token_id());

    let denial = check_action(&mut issuer, &request, NOW).expect_err("external content is data");
    assert!(matches!(denial, HitlDenial::ExternalContentNotAuthority));
    assert_eq!(
        denial.reason_code(),
        ReasonCode::ExternalContentNotAuthority,
    );
    assert!(
        !issuer.is_spent(token.token_id()),
        "a request that never got to the token stage must not spend one",
    );
}
