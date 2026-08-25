//! Consent starts closed, and turning something on leaves an audit entry.
//!
//! AC-02 says a freshly finished wizard has collection off, cloud off and no
//! model endpoint. AC-09 says an unconsented collector produces zero events.
//! Both are downstream of one property: the ledger's default grants nothing,
//! and there is no constructor, no deserialization path and no missing-entry
//! case that turns into a grant.

use soul_policy::audit::{self, ReasonCode};
use soul_policy::consent::{ConsentLedger, ConsentState, ConsentTopic};
use soul_schema::audit::{AuditAction, AuditDecision};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::AuditLog;

const AT: i64 = 1_787_529_600;

#[test]
fn a_fresh_ledger_grants_nothing() {
    let ledger = ConsentLedger::default();
    assert_eq!(ledger, ConsentLedger::closed());

    for topic in ConsentTopic::ALL {
        assert_eq!(ledger.state(*topic), ConsentState::Withheld, "{topic:?}");
        assert!(!ledger.is_granted(*topic), "{topic:?}");
        assert!(ledger.record(*topic).is_none(), "{topic:?}");
        assert!(ledger.require(*topic).is_err(), "{topic:?}");
    }
    assert!(ledger.granted_topics().is_empty());
}

#[test]
fn granting_one_topic_leaves_the_others_alone() {
    let mut ledger = ConsentLedger::closed();
    ledger.grant(ConsentTopic::CollectForegroundApp, AT);

    assert!(ledger.is_granted(ConsentTopic::CollectForegroundApp));
    assert_eq!(
        ledger.granted_topics(),
        vec![ConsentTopic::CollectForegroundApp],
    );
    for topic in ConsentTopic::ALL {
        if *topic != ConsentTopic::CollectForegroundApp {
            assert!(!ledger.is_granted(*topic), "{topic:?} was not asked about");
        }
    }
    assert_eq!(
        ledger
            .record(ConsentTopic::CollectForegroundApp)
            .expect("a record exists")
            .changed_at_unix_seconds,
        AT,
    );
}

#[test]
fn revoking_puts_it_back() {
    let mut ledger = ConsentLedger::closed();
    ledger.grant(ConsentTopic::ResearchPreview, AT);
    assert!(ledger.is_granted(ConsentTopic::ResearchPreview));

    ledger.revoke(ConsentTopic::ResearchPreview, AT + 60);
    assert!(!ledger.is_granted(ConsentTopic::ResearchPreview));
    assert!(ledger.require(ConsentTopic::ResearchPreview).is_err());
    assert_eq!(
        ledger
            .record(ConsentTopic::ResearchPreview)
            .expect("the record survives the revoke")
            .changed_at_unix_seconds,
        AT + 60,
    );
}

/// A stored ledger round-trips, and an unknown or absent topic in the file
/// reads back as withheld rather than as a grant.
#[test]
fn persistence_cannot_widen_what_was_granted() {
    let mut ledger = ConsentLedger::closed();
    ledger.grant(ConsentTopic::UseUserEndpoint, AT);

    let json = serde_json::to_string(&ledger).expect("serialize");
    let restored: ConsentLedger = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, ledger);
    assert!(restored.is_granted(ConsentTopic::UseUserEndpoint));
    assert!(!restored.is_granted(ConsentTopic::CollectForegroundApp));

    let empty: ConsentLedger = serde_json::from_str("{}").expect("an empty file is a closed one");
    assert_eq!(empty, ConsentLedger::closed());

    // A file naming a topic this build does not know must not be silently
    // reinterpreted as one it does.
    let stray: ConsentLedger = serde_json::from_str(
        r#"{"granted":{"collect.everything":{"state":"granted","changed_at_unix_seconds":0}}}"#,
    )
    .expect("unknown keys are data, not grants");
    for topic in ConsentTopic::ALL {
        assert!(!stray.is_granted(*topic), "{topic:?}");
    }
}

#[test]
fn every_topic_has_a_distinct_stable_name() {
    let mut names: Vec<&str> = ConsentTopic::ALL
        .iter()
        .map(|topic| topic.as_str())
        .collect();
    let count = names.len();
    names.sort_unstable();
    names.dedup();
    assert_eq!(count, names.len(), "two topics share a stored name");
    assert_eq!(count, 5, "adding a topic is a product decision");
}

/// The grant returns the audit content for the change, so a caller can write
/// it but cannot invent a different one. The entry must land on the chain and
/// carry no prose.
#[test]
fn a_grant_and_a_revoke_both_reach_the_audit_chain() {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("consent.db"),
        &TestKeyProvider::from_seed("wp08 consent"),
    )
    .expect("open the encrypted store");

    let mut ledger = ConsentLedger::closed();

    let granted = ledger.grant(ConsentTopic::CollectForegroundApp, AT);
    assert_eq!(granted.action, AuditAction::ConsentGrant);
    assert_eq!(granted.decision, AuditDecision::Allowed);
    assert_eq!(granted.reason_code, Some(ReasonCode::ConsentGranted));
    audit::append(&mut store, granted, AT).expect("append the grant");

    let revoked = ledger.revoke(ConsentTopic::CollectForegroundApp, AT + 60);
    assert_eq!(revoked.decision, AuditDecision::Denied);
    assert_eq!(revoked.reason_code, Some(ReasonCode::ConsentRevoked));
    audit::append(&mut store, revoked, AT + 60).expect("append the revoke");

    store.verify_audit_chain().expect("chain");
    let entries = store.list_audit().expect("read back");
    assert_eq!(entries.len(), 2);
    assert!(entries
        .iter()
        .all(|entry| entry.action == AuditAction::ConsentGrant));

    // The topic itself is not in the entry: a consent record is a state
    // change, and the audit table holds no strings the user typed.
    let serialized = serde_json::to_string(&entries).expect("serialize");
    assert!(
        !serialized.contains("collect.foreground_app"),
        "the audit entry must not carry the topic string: {serialized}",
    );
}
