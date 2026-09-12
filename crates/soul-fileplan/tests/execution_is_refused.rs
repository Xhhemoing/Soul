//! AC-19 at the file-plan surface: an unknown action, a plan whose hash moved,
//! and a replayed token are each refused, and so is a request with nothing at
//! all wrong with it.
//!
//! WP08 already proves the three refusals inside `soul-policy`. This file
//! proves they are the answers a caller gets *here*, which is a different
//! claim: a crate can hold a correct gate and route around it.
//!
//! The last case is the one that matters most. A perfectly formed request, from
//! the user, with a live token, against the exact plan they approved, is
//! refused with `WRITE_NOT_IMPLEMENTED`. There is no input that produces
//! anything else, and the return type has no variant that could carry one.

mod common;

use common::{walk, Tree};

use soul_fileplan::{Authorization, ScanLimits};
use soul_policy::hitl::{
    ActionKind, ActionRequest, CapabilityScope, PlanHash, RequestOrigin, TokenIssuer,
};
use soul_policy::ReasonCode;
use soul_schema::audit::AuditAction;

const NOW_MS: u64 = 1_787_529_600_000;

struct Approved {
    tree: Tree,
    authorization: Authorization,
    issuer: TokenIssuer,
    plan: serde_json::Value,
    plan_hash: PlanHash,
}

fn approved_plan() -> Approved {
    let tree = Tree::build();
    let mut authorization = Authorization::new();
    authorization
        .authorize(&tree.alpha())
        .expect("authorize Alpha");
    let mut issuer = TokenIssuer::new();

    let preview = soul_fileplan::preview(
        &authorization,
        &mut issuer,
        &tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("a preview to approve");

    Approved {
        plan: preview.plan().to_json(),
        plan_hash: preview.plan_hash().clone(),
        tree,
        authorization,
        issuer,
    }
}

fn request(approved: &Approved) -> ActionRequest {
    ActionRequest::new(ActionKind::PlanFiles.as_str(), RequestOrigin::User)
        .with_plan(approved.plan.clone())
        .approved_as(approved.plan_hash.clone())
}

#[test]
fn an_unknown_action_is_refused_by_name() {
    let approved = approved_plan();

    for name in [
        "plan.execute",
        "file.move",
        "files.apply",
        "PLAN.FILES",
        "plan.files ",
        "",
    ] {
        let outcome = soul_fileplan::refuse_execution(
            &approved.issuer,
            &ActionRequest::new(name, RequestOrigin::User).with_plan(approved.plan.clone()),
        );
        assert_eq!(
            outcome.reason_code(),
            ReasonCode::UnknownAction,
            "`{name}` should not be an action this surface knows",
        );
    }
}

/// Actions other work packages own are unknown here too. A file-plan surface
/// that accepted `forget.execute` would be a second door into forgetting, and
/// the token it was carrying would be spent on the way through.
#[test]
fn an_action_belonging_to_another_work_package_is_unknown_here() {
    let approved = approved_plan();

    for kind in ActionKind::ALL {
        if soul_fileplan::FILEPLAN_ACTIONS.contains(kind) {
            continue;
        }
        let outcome = soul_fileplan::refuse_execution(
            &approved.issuer,
            &ActionRequest::new(kind.as_str(), RequestOrigin::User),
        );
        assert_eq!(
            outcome.reason_code(),
            ReasonCode::UnknownAction,
            "{} reached the file-plan surface",
            kind.as_str(),
        );
    }
}

#[test]
fn a_plan_that_changed_after_approval_is_refused() {
    let approved = approved_plan();

    let mut edited = approved.plan.clone();
    edited["moves"][0]["to"] = serde_json::json!("图片/somewhere-else.jpg");
    let outcome =
        soul_fileplan::refuse_execution(&approved.issuer, &request(&approved).with_plan(edited));
    assert_eq!(outcome.reason_code(), ReasonCode::PlanHashMismatch);

    // Even a change nobody would notice. The hash is over the whole value.
    let mut nudged = approved.plan.clone();
    nudged["scanned_entries"] = serde_json::json!(9_999);
    let outcome =
        soul_fileplan::refuse_execution(&approved.issuer, &request(&approved).with_plan(nudged));
    assert_eq!(outcome.reason_code(), ReasonCode::PlanHashMismatch);
}

/// A plan approved before the user saved a file into the directory no longer
/// matches the plan a rescan produces. This is the same refusal as above,
/// arrived at the way it will actually happen.
#[test]
fn a_plan_approved_before_the_directory_moved_no_longer_matches() {
    let mut approved = approved_plan();

    std::fs::write(
        std::path::Path::new(&approved.tree.alpha()).join("late.png"),
        "arrived after approval",
    )
    .expect("the user saves a file");

    let rescan = soul_fileplan::preview(
        &approved.authorization,
        &mut approved.issuer,
        &approved.tree.alpha(),
        RequestOrigin::User,
        ScanLimits::default(),
        NOW_MS,
    )
    .expect("a second preview");

    let outcome = soul_fileplan::refuse_execution(
        &approved.issuer,
        &ActionRequest::new(ActionKind::PlanFiles.as_str(), RequestOrigin::User)
            .with_plan(rescan.plan().to_json())
            .approved_as(approved.plan_hash.clone()),
    );
    assert_eq!(outcome.reason_code(), ReasonCode::PlanHashMismatch);
}

#[test]
fn a_replayed_token_is_refused_as_a_replay() {
    let mut approved = approved_plan();

    // A token for something this build does implement, so that spending it is
    // a real event rather than a refusal in disguise.
    let token = approved.issuer.issue(
        CapabilityScope::ForgetExecute,
        approved.plan_hash.clone(),
        NOW_MS,
    );
    approved
        .issuer
        .consume(
            token.token_id(),
            CapabilityScope::ForgetExecute,
            &approved.plan_hash,
            NOW_MS,
        )
        .expect("the first use");

    let outcome = soul_fileplan::refuse_execution(
        &approved.issuer,
        &request(&approved).with_token(token.token_id()),
    );
    assert_eq!(outcome.reason_code(), ReasonCode::TokenReplayed);
    assert_eq!(outcome.audit().action, AuditAction::CapabilityReject);
}

/// The standing refusal. Nothing is wrong with this request.
#[test]
fn a_request_with_nothing_wrong_with_it_is_refused_anyway() {
    let approved = approved_plan();

    let outcome = soul_fileplan::refuse_execution(&approved.issuer, &request(&approved));
    assert!(outcome.is_write_not_implemented());
    assert_eq!(outcome.reason_code(), ReasonCode::WriteNotImplemented);
    assert_eq!(outcome.audit().action, AuditAction::FilePlan);
    assert!(outcome.explanation().contains("v0.1.1"));
}

/// The promise WP08 wrote down: v0.1 issues no file-write token and consumes
/// none. Presenting one here leaves it exactly as it was found.
#[test]
fn a_file_write_token_is_neither_honoured_nor_spent() {
    let mut approved = approved_plan();

    let token = approved.issuer.issue(
        CapabilityScope::FileWrite,
        approved.plan_hash.clone(),
        NOW_MS,
    );
    assert!(!CapabilityScope::FileWrite.is_implemented_in_v0_1());

    let outcome = soul_fileplan::refuse_execution(
        &approved.issuer,
        &request(&approved).with_token(token.token_id()),
    );
    assert!(outcome.is_write_not_implemented());
    assert!(
        !approved.issuer.is_spent(token.token_id()),
        "v0.1 must not consume a file-write token, not even to refuse it",
    );

    // And again, because a refusal that quietly burnt the token would show up
    // the second time as a replay rather than as the same standing refusal.
    let again = soul_fileplan::refuse_execution(
        &approved.issuer,
        &request(&approved).with_token(token.token_id()),
    );
    assert_eq!(again.reason_code(), ReasonCode::WriteNotImplemented);
}

/// No fileplan action asks for a token at all, so nothing on this path can
/// reach the ledger. Asserted over the action set rather than over the two
/// names, so adding a third action that needs one fails here.
#[test]
fn no_action_on_this_surface_needs_a_capability_token() {
    for kind in soul_fileplan::FILEPLAN_ACTIONS {
        assert!(!kind.needs_capability_token(), "{}", kind.as_str());
    }
    assert!(soul_fileplan::FILEPLAN_ACTIONS.contains(&ActionKind::ScanDirectory));
    assert!(soul_fileplan::FILEPLAN_ACTIONS.contains(&ActionKind::PlanFiles));
}

/// Refusing costs nothing on disk, however many times it is asked for.
#[test]
fn asking_for_execution_over_and_over_changes_nothing() {
    let approved = approved_plan();
    let before = walk(approved.tree.base());

    for _ in 0..25 {
        let outcome = soul_fileplan::refuse_execution(&approved.issuer, &request(&approved));
        assert!(outcome.is_write_not_implemented());
    }

    assert_eq!(before, walk(approved.tree.base()));
}

/// External content asking for execution is refused as external content, not
/// as a write that happens to be unimplemented. The two say different things to
/// whoever reads the audit chain.
#[test]
fn external_content_asking_for_execution_is_refused_for_being_external() {
    let approved = approved_plan();

    let outcome = soul_fileplan::refuse_execution(
        &approved.issuer,
        &ActionRequest::new(
            ActionKind::PlanFiles.as_str(),
            RequestOrigin::ExternalContent,
        )
        .with_plan(approved.plan.clone()),
    );
    assert_eq!(
        outcome.reason_code(),
        ReasonCode::ExternalContentNotAuthority,
    );
}

/// Every refusal here agrees with the gate in `soul-policy` about the cases
/// that gate has an opinion on. `refuse_execution` spells the checks out again
/// so it can take the ledger immutably; this is what keeps the second spelling
/// honest.
#[test]
fn the_refusals_agree_with_the_policy_gate() {
    let mut approved = approved_plan();

    let cases = [
        ActionRequest::new("plan.execute", RequestOrigin::User),
        ActionRequest::new(
            ActionKind::PlanFiles.as_str(),
            RequestOrigin::ExternalContent,
        ),
        request(&approved).with_plan(serde_json::json!({"edited": true})),
    ];

    for case in cases {
        let ours = soul_fileplan::refuse_execution(&approved.issuer, &case);
        let theirs = soul_policy::hitl::check_action(&mut approved.issuer, &case, NOW_MS)
            .expect_err("the policy gate refuses this too");
        assert_eq!(
            ours.reason_code(),
            theirs.reason_code(),
            "the two gates disagree about `{}`",
            case.action,
        );
    }
}
