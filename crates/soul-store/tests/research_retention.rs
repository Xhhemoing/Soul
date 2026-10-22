//! Real sealed synthetic sources; this surface only emits aggregate counts.
mod common;

use common::bucketed_event;
use soul_schema::common::{Purpose, ResearchDisposition, SealedSubject, Subject};
use soul_schema::event::{EventKind, SoulEvent};
use soul_store::research_retention::{ActivityBucket, ResearchScope};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::types::{SealRequest, StoreError};
use soul_store_api::{BlobStore, EventStore, ForgetOps, ForgetUnit};
use uuid::Uuid;

const SEED: &str = "research retention synthetic only";

fn setup() -> (tempfile::TempDir, SqlCipherStore) {
    let dir = tempfile::tempdir().unwrap();
    let store = SqlCipherStore::open(
        dir.path().join("research.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .unwrap();
    (dir, store)
}

fn scope() -> ResearchScope {
    ResearchScope {
        from_day: "2024-02-28".into(),
        to_day: "2024-03-02".into(),
    }
}

fn source(store: &mut SqlCipherStore, ts: &str) -> SoulEvent {
    let id = Uuid::now_v7();
    let mut event = bucketed_event(id, ts, EventKind::AppForeground, Subject::Owner);
    event.privacy.purposes = vec![Purpose::Research];
    event.body_ref = Some(
        store
            .seal(SealRequest::new(
                Uuid::now_v7(),
                id,
                "body_ref",
                SealedSubject::Owner,
                b"synthetic private app and duration; never emitted".to_vec(),
            ))
            .unwrap(),
    );
    event
}

fn add(store: &mut SqlCipherStore, ts: &str) -> SoulEvent {
    let event = source(store, ts);
    store.append_event(event.clone()).unwrap();
    event
}

#[test]
fn raw_owner_source_becomes_only_a_count_bucket_without_identifiers_or_body() {
    let (_dir, mut store) = setup();
    let event = add(&mut store, "2024-02-29T09:10:00Z");
    add(&mut store, "2024-02-29T09:40:00Z");
    let held = store.prepare_research_retention(&scope()).unwrap();
    assert_eq!(
        held.dataset().rows,
        vec![ActivityBucket {
            event_kind: "app.foreground".into(),
            time_bucket_utc: "2024-02-29T09:00:00Z".into(),
            aggregate_count: 2,
        }]
    );
    assert_eq!(held.dataset().counts.included_events, 2);
    let json = serde_json::to_string(held.dataset()).unwrap();
    for forbidden in [
        event.event_id.to_string(),
        "synthetic private".into(),
        "duration".into(),
    ] {
        assert!(!json.contains(&forbidden));
        assert!(!format!("{held:?}").contains(&forbidden));
    }
}

#[test]
fn policy_and_unavailable_exclusions_are_mutually_exclusive() {
    let (_dir, mut store) = setup();
    add(&mut store, "2024-02-29T09:10:00Z");
    for subject in [Subject::ThirdParty, Subject::Mixed, Subject::System] {
        let mut event = source(&mut store, "2024-02-29T10:00:00Z");
        event.privacy.subject = subject;
        store.append_event(event).unwrap();
    }
    let mut deny = source(&mut store, "2024-02-29T11:00:00Z");
    deny.privacy.egress.research_export = ResearchDisposition::Deny;
    store.append_event(deny).unwrap();
    let mut no_purpose = source(&mut store, "2024-02-29T12:00:00Z");
    no_purpose.privacy.purposes = vec![Purpose::Memory];
    store.append_event(no_purpose).unwrap();
    let mut no_blob = source(&mut store, "2024-02-29T13:00:00Z");
    no_blob.body_ref = None;
    store.append_event(no_blob).unwrap();
    let held = store.prepare_research_retention(&scope()).unwrap();
    let counts = &held.dataset().counts;
    assert_eq!(counts.candidate_events, 7);
    assert_eq!(counts.included_events, 1);
    assert_eq!(counts.excluded_policy_events, 5);
    assert_eq!(counts.excluded_unavailable_events, 1);
    assert_eq!(counts.excluded_time_events + counts.out_of_range_events, 0);
}

#[test]
fn scope_is_half_open_and_offset_time_is_excluded_instead_of_relabelled() {
    let (_dir, mut store) = setup();
    add(&mut store, "2024-02-28T00:00:00Z");
    add(&mut store, "2024-03-02T00:00:00Z");
    add(&mut store, "2024-02-29T09:00:00+01:00");
    let held = store.prepare_research_retention(&scope()).unwrap();
    assert_eq!(held.dataset().counts.candidate_events, 3);
    assert_eq!(held.dataset().counts.included_events, 1);
    assert_eq!(held.dataset().counts.out_of_range_events, 1);
    assert_eq!(held.dataset().counts.excluded_time_events, 1);
    for (from, to) in [
        ("2023-02-29", "2023-03-01"),
        ("2024-03-02", "2024-02-28"),
        ("2024-02-28", "2024-02-28"),
        ("2024-2-28", "2024-03-02"),
    ] {
        assert!(store
            .prepare_research_retention(&ResearchScope {
                from_day: from.into(),
                to_day: to.into(),
            })
            .is_err());
    }
}

#[test]
fn more_than_fifty_buckets_are_complete_and_stably_ordered() {
    let (_dir, mut store) = setup();
    for day in ["2024-03-01", "2024-02-29", "2024-02-28"] {
        for hour in (0..24).rev() {
            add(&mut store, &format!("{day}T{hour:02}:15:00Z"));
        }
    }
    let held = store.prepare_research_retention(&scope()).unwrap();
    let rows = &held.dataset().rows;
    assert_eq!(rows.len(), 72);
    assert_eq!(held.dataset().counts.included_events, 72);
    assert_eq!(
        rows.first().unwrap().time_bucket_utc,
        "2024-02-28T00:00:00Z"
    );
    assert_eq!(rows.last().unwrap().time_bucket_utc, "2024-03-01T23:00:00Z");
    assert!(rows
        .windows(2)
        .all(|pair| pair[0].time_bucket_utc < pair[1].time_bucket_utc));
    assert!(rows.iter().all(|row| row.aggregate_count == 1));
}

#[test]
fn forget_then_same_bucket_replacement_invalidates_even_when_counts_match() {
    let (_dir, mut store) = setup();
    let original = add(&mut store, "2024-02-29T09:10:00Z");
    let held = store.prepare_research_retention(&scope()).unwrap();
    store
        .execute_forget(ForgetUnit::ContentKey(
            original.body_ref.unwrap().content_key_id,
        ))
        .unwrap();
    assert_eq!(
        store
            .prepare_research_retention(&scope())
            .unwrap()
            .dataset()
            .counts
            .included_events,
        0
    );
    add(&mut store, "2024-02-29T09:10:00Z");
    let replacement = store.prepare_research_retention(&scope()).unwrap();
    assert_eq!(replacement.dataset().rows, held.dataset().rows);
    let mut called = false;
    assert!(store
        .publish_research_retention(&held, || {
            called = true;
            Ok(())
        })
        .is_err());
    assert!(!called);
}

#[test]
fn new_in_range_event_invalidates_but_out_of_range_event_does_not() {
    let (_dir, mut store) = setup();
    add(&mut store, "2024-02-29T09:10:00Z");
    let held = store.prepare_research_retention(&scope()).unwrap();
    add(&mut store, "2024-03-05T09:10:00Z");
    let published = store
        .publish_research_retention(&held, || Ok("published"))
        .unwrap();
    assert_eq!(published.value, "published");
    assert!(published.transaction_closed);
    add(&mut store, "2024-02-29T10:10:00Z");
    assert!(store.publish_research_retention(&held, || Ok(())).is_err());
}

#[test]
fn failed_publisher_returns_its_failure_and_closes_transaction() {
    let (_dir, mut store) = setup();
    add(&mut store, "2024-02-29T09:10:00Z");
    let held = store.prepare_research_retention(&scope()).unwrap();
    let mut called = false;
    let result = store.publish_research_retention::<()>(&held, || {
        called = true;
        Err(StoreError::Backend("synthetic publish failure".into()))
    });
    assert!(called);
    assert_eq!(
        result.unwrap_err(),
        StoreError::Backend("synthetic publish failure".into())
    );
    assert!(!store.transaction_in_progress());
}
