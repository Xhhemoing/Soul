//! AC-23: after the matrix of actions, the audit chain verifies and carries no
//! prose and no names.
//!
//! The test writes one entry for every action in the frozen vocabulary through
//! the real SQLCipher store — not a fake — because the chain fields are the
//! store's job and a fake would let this pass while the shipped path was
//! broken. It then serializes the whole audit table and points
//! `soul_testkit::LeakageChecker` at the bytes, with a corpus made of the very
//! third-party prose, names, handles, numbers and addresses that the actions
//! were about.
//!
//! The two halves matter together. A chain that verifies over an empty table
//! proves nothing, and a leakage check against a corpus nothing was ever
//! written from proves nothing either, so the negative control at the bottom
//! writes the same prose into a JSON blob and asserts the checker screams.

use uuid::Uuid;

use soul_policy::audit::{self, AuditContent, ReasonCode, FORBIDDEN_FIELDS};
use soul_policy::clock::rfc3339_utc;
use soul_policy::hitl::PlanHash;
use soul_schema::audit::{
    AuditAction, AuditCounts, AuditDecision, EgressClass as AuditEgressClass, SoulAuditEntry,
};
use soul_schema::common::{SchemaVersion, Sha256Hex, Timestamp};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::AuditLog;
use soul_testkit::LeakageChecker;

const SEED: &str = "wp08 audit chain";
const AT: i64 = 1_787_529_600; // 2026-08-24T00:00:00Z

/// The prose and identifiers the recorded actions were about. None of it may
/// reach the audit table.
const THIRD_PARTY_BODY: &str = "明天上午十点在公司门口见，别迟到";
const OWNER_BODY: &str = "我回复说会准时到，并且提醒自己带上合同";
const NAME: &str = "李雷";
const HANDLE: &str = "@wang_xiao2";
const PHONE: &str = "13800138000";
const EMAIL: &str = "lilei@example.invalid";

fn open(directory: &std::path::Path) -> SqlCipherStore {
    SqlCipherStore::open(
        directory.join("audit-chain.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open the encrypted store")
}

fn checker() -> LeakageChecker {
    let mut checker = LeakageChecker::new();
    checker.add_third_party_body("conversation", THIRD_PARTY_BODY);
    checker.add_third_party_body("own_reply", OWNER_BODY);
    checker.add_known_identifier("name", NAME);
    checker.add_known_identifier("handle", HANDLE);
    checker.add_known_identifier("phone", PHONE);
    checker.add_known_identifier("email", EMAIL);
    checker
}

/// One entry per action the acceptance matrix names, in the order a session
/// would produce them.
fn matrix_of_actions() -> Vec<AuditContent> {
    let subject = Uuid::now_v7();
    let plan = PlanHash::of(&serde_json::json!({ "kind": "draft", "turns": 2 }));

    vec![
        AuditContent::allowed(AuditAction::ConsentGrant, ReasonCode::ConsentGranted),
        AuditContent::allowed(AuditAction::CollectStart, ReasonCode::Routine),
        AuditContent::allowed(AuditAction::CollectStop, ReasonCode::Routine),
        AuditContent::allowed(AuditAction::ImportCommit, ReasonCode::Routine)
            .about(&[subject])
            .counting(AuditCounts {
                items: Some(5),
                bytes: Some(2_048),
            }),
        AuditContent::allowed(AuditAction::InferenceWrite, ReasonCode::Routine).about(&[subject]),
        AuditContent::allowed(AuditAction::ProfileCorrect, ReasonCode::Routine).about(&[subject]),
        AuditContent::allowed(AuditAction::MemoryWrite, ReasonCode::Routine).about(&[subject]),
        AuditContent::allowed(AuditAction::ForgetExecute, ReasonCode::Routine).about(&[subject]),
        AuditContent::allowed(
            AuditAction::DraftCreate,
            ReasonCode::ThirdPartyBodyPlaceheld,
        )
        .for_plan(plan.as_str()),
        AuditContent::allowed(
            AuditAction::EgressRequest,
            ReasonCode::ThirdPartyBodyPlaceheld,
        )
        .over(AuditEgressClass::E1)
        .for_plan(plan.as_str()),
        AuditContent::denied(AuditAction::EgressRequest, ReasonCode::E0NoCodePath)
            .over(AuditEgressClass::None),
        AuditContent::allowed(AuditAction::FilePlan, ReasonCode::Routine).for_plan(plan.as_str()),
        AuditContent::denied(AuditAction::HitlDeny, ReasonCode::PlanHashMismatch)
            .for_plan(plan.as_str()),
        AuditContent::denied(AuditAction::CapabilityReject, ReasonCode::TokenReplayed)
            .with_token(Uuid::now_v7()),
        AuditContent::denied(
            AuditAction::InjectionBlocked,
            ReasonCode::ExternalContentNotAuthority,
        ),
    ]
}

#[test]
fn the_recorded_matrix_verifies_and_carries_no_prose_or_names() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());

    let actions = matrix_of_actions();
    let expected = actions.len();
    assert!(
        expected >= 15,
        "the matrix should cover every action in the frozen vocabulary",
    );

    for (index, content) in actions.into_iter().enumerate() {
        audit::append(&mut store, content, AT + index as i64).expect("append an audit entry");
    }

    store
        .verify_audit_chain()
        .expect("the chain must verify after the whole matrix");

    let entries = store.list_audit().expect("read the chain back");
    assert_eq!(entries.len(), expected);

    // Every action in the contract must be exercised, or "the matrix passed"
    // means less than it sounds.
    for action in [
        AuditAction::ConsentGrant,
        AuditAction::CollectStart,
        AuditAction::CollectStop,
        AuditAction::ImportCommit,
        AuditAction::InferenceWrite,
        AuditAction::ProfileCorrect,
        AuditAction::MemoryWrite,
        AuditAction::ForgetExecute,
        AuditAction::DraftCreate,
        AuditAction::EgressRequest,
        AuditAction::FilePlan,
        AuditAction::HitlDeny,
        AuditAction::CapabilityReject,
        AuditAction::InjectionBlocked,
    ] {
        assert!(
            entries.iter().any(|entry| entry.action == action),
            "{action:?} is in the contract and must appear in the recorded matrix",
        );
    }

    let serialized = serde_json::to_string(&entries).expect("serialize the chain");
    checker().assert_clean("the serialized audit chain", &serialized);

    for field in FORBIDDEN_FIELDS {
        assert!(
            !serialized.contains(&format!("\"{field}\"")),
            "a `{field}` key reached the audit table",
        );
    }
}

/// The leakage check above would pass over any string that happens not to
/// contain the corpus. This proves the checker used there can fail.
#[test]
fn the_same_checker_catches_the_prose_when_it_is_present() {
    let leaky = serde_json::json!({
        "action": "draft.create",
        "note": format!("{NAME}说：{THIRD_PARTY_BODY}，电话 {PHONE}"),
    })
    .to_string();
    let findings = checker().inspect(&leaky);
    assert!(
        findings.len() >= 2,
        "the corpus must catch both the prose and the name: {findings:#?}",
    );
}

#[test]
fn a_prose_field_is_refused_before_it_reaches_the_store() {
    // `SoulAuditEntry` has no prose field, so the only way to smuggle one in is
    // past serde. `audit::check` runs over the serialized form for exactly this
    // reason: it keeps working when the struct changes.
    let entry = SoulAuditEntry {
        schema_version: SchemaVersion,
        seq: 0,
        entry_id: Uuid::now_v7(),
        ts: Timestamp::new(rfc3339_utc(AT)),
        prev_hash: Sha256Hex::new("0".repeat(64)),
        entry_hash: Sha256Hex::new("0".repeat(64)),
        action: AuditAction::DraftCreate,
        decision: AuditDecision::Allowed,
        reason_code: Some("NOT_A_KNOWN_CODE".into()),
        subject_refs: None,
        counts: None,
        plan_hash: None,
        capability_token_id: None,
        egress_class: None,
    };
    assert!(
        matches!(
            audit::check(&entry),
            Err(soul_policy::AuditContentError::UnknownReasonCode(_)),
        ),
        "a reason code outside the closed vocabulary must be refused",
    );

    let mut value = serde_json::to_value(&entry).expect("serialize");
    value["body"] = serde_json::Value::String(THIRD_PARTY_BODY.into());
    let smuggled: Result<SoulAuditEntry, _> = serde_json::from_value(value);
    assert!(
        smuggled.is_err() || audit::check(&smuggled.expect("deserialize")).is_err(),
        "an entry with a body field must not survive the round trip",
    );
}

#[test]
fn every_reason_code_satisfies_the_frozen_pattern() {
    // audit.schema.json: ^[A-Z][A-Z0-9_]{2,63}$
    for code in ReasonCode::ALL {
        let text = code.as_str();
        assert!((3..=64).contains(&text.len()), "{text} has a bad length");
        let mut chars = text.chars();
        assert!(
            chars.next().is_some_and(|c| c.is_ascii_uppercase()),
            "{text} must start with an uppercase letter",
        );
        assert!(
            chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'),
            "{text} may only hold uppercase letters, digits and underscores",
        );
    }

    let mut seen: Vec<&str> = ReasonCode::ALL.iter().map(|code| code.as_str()).collect();
    let count = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(count, seen.len(), "two reason codes render the same string");
}

/// The store, not the caller, owns the links. WP02 made that true; this asserts
/// the policy-side builder did not quietly reintroduce a way around it.
#[test]
fn the_builder_cannot_choose_its_own_place_in_the_chain() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = open(dir.path());

    for index in 0..3 {
        audit::append(
            &mut store,
            AuditContent::allowed(AuditAction::MemoryWrite, ReasonCode::Routine),
            AT + index,
        )
        .expect("append");
    }

    let links = store.audit_links().expect("links");
    assert_eq!(
        links.iter().map(|link| link.seq).collect::<Vec<_>>(),
        vec![0, 1, 2],
    );
    assert_eq!(links[0].prev_hash, soul_store::GENESIS_PREV_HASH);
    for pair in links.windows(2) {
        assert_eq!(pair[1].prev_hash, pair[0].entry_hash);
    }
}
