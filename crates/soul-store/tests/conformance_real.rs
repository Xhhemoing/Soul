//! The real database runs the suite the fake passes.
//!
//! This is the whole point of `soul-store-api::conformance`: WP01 wrote the
//! storage contract as executable checks against an in-memory fake, and the
//! encrypted backend has to answer the same calls the same way. Anything that
//! passes here but not there, or the reverse, means the boundary means two
//! different things depending on who is behind it.

use std::sync::atomic::{AtomicUsize, Ordering};

use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::conformance::run_conformance;
use soul_store_api::types::StoreError;
use soul_store_api::{BlobStore, EventStore, SoulStore};

use soul_schema::common::SealedSubject;
use soul_store_api::types::{EventFilter, SealRequest};
use uuid::Uuid;

const SEED: &str = "soul-store conformance";

#[test]
fn the_encrypted_store_passes_the_same_conformance_suite_as_the_fake() {
    let dir = tempfile::tempdir().expect("temp dir");
    let keys = TestKeyProvider::from_seed(SEED);
    let opened = AtomicUsize::new(0);

    // `run_conformance` builds a fresh store per check so one failure cannot
    // mask another through leftover state; each one therefore needs its own
    // database file.
    run_conformance(|| {
        let index = opened.fetch_add(1, Ordering::SeqCst);
        let path = dir.path().join(format!("conformance-{index}.db"));
        SqlCipherStore::open(&path, &keys).expect("open the encrypted store")
    });

    assert!(
        opened.load(Ordering::SeqCst) >= 8,
        "the suite should have built a store for every check",
    );
}

#[test]
fn what_was_written_is_still_there_after_a_close_and_reopen() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("reopen.db");
    let keys = TestKeyProvider::from_seed(SEED);

    let row_id = Uuid::now_v7();
    let content_key_id = Uuid::now_v7();
    let text = "重开之后仍然读得回来";

    let sealed = {
        let mut store = SqlCipherStore::open(&path, &keys).expect("open");
        let sealed = store
            .seal(SealRequest::new(
                content_key_id,
                row_id,
                "summary_ref",
                SealedSubject::Owner,
                text.as_bytes().to_vec(),
            ))
            .expect("seal");
        store.flush().expect("flush");
        store.close().expect("close");
        sealed
    };

    let store = SqlCipherStore::open(&path, &keys).expect("reopen");
    assert_eq!(
        String::from_utf8(store.open(&sealed).expect("open sealed")).expect("utf-8"),
        text,
        "a store that cannot survive a restart is not a store",
    );
    assert_eq!(store.content_key_count().expect("keys"), 1);
    assert_eq!(store.blob_count().expect("blobs"), 1);
}

#[test]
fn a_different_key_cannot_open_the_database() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("wrong-key.db");

    {
        let mut store =
            SqlCipherStore::open(&path, &TestKeyProvider::from_seed("right")).expect("open");
        store.flush().expect("flush");
        store.close().expect("close");
    }

    let attempt = SqlCipherStore::open(&path, &TestKeyProvider::from_seed("wrong"));
    assert!(
        matches!(attempt, Err(StoreError::Backend(_))),
        "opening under the wrong key must fail loudly; got {attempt:?}",
    );
}

/// The filters are answered by SQL rather than by re-reading everything, so
/// they get their own check on top of the shared suite.
#[test]
fn event_filters_are_answered_by_the_database() {
    use soul_schema::common::{
        ActorSubject, Derivation, EgressPolicy, Privacy, Purpose, Retention, SchemaVersion,
        Subject, Timestamp,
    };
    use soul_schema::event::{EventKind, EventSource, SoulEvent};

    let dir = tempfile::tempdir().expect("temp dir");
    let mut store = SqlCipherStore::open(
        dir.path().join("filters.db"),
        &TestKeyProvider::from_seed(SEED),
    )
    .expect("open");

    let make = |ts: &str, kind: EventKind, source: EventSource| SoulEvent {
        schema_version: SchemaVersion,
        event_id: Uuid::now_v7(),
        ts: Timestamp::new(ts),
        source,
        kind,
        actor_subject: ActorSubject::Owner,
        consent_id: None,
        privacy: Privacy {
            subject: Subject::Owner,
            derivation: Derivation::Raw,
            purposes: vec![Purpose::Memory],
            retention: Retention::until_forgotten(),
            egress: EgressPolicy::default(),
        },
        body_ref: None,
    };

    for event in [
        make(
            "2026-08-24T09:00:00Z",
            EventKind::AppForeground,
            EventSource::CollectorForegroundApp,
        ),
        make(
            "2026-08-24T10:00:00Z",
            EventKind::AppForeground,
            EventSource::CollectorForegroundApp,
        ),
        make(
            "2026-08-24T11:00:00Z",
            EventKind::ImportItem,
            EventSource::ImportSoulImportV1,
        ),
    ] {
        store.append_event(event).expect("append");
    }

    assert_eq!(store.count_events(&EventFilter::all()).expect("all"), 3);
    assert_eq!(
        store
            .count_events(&EventFilter::with_kind(EventKind::AppForeground))
            .expect("by kind"),
        2,
    );
    assert_eq!(
        store
            .count_events(&EventFilter::with_source(EventSource::ImportSoulImportV1))
            .expect("by source"),
        1,
    );
    assert_eq!(
        store
            .count_events(&EventFilter {
                since: Some("2026-08-24T10:00:00Z".into()),
                ..EventFilter::all()
            })
            .expect("since"),
        2,
    );
    assert_eq!(
        store
            .list_events(&EventFilter {
                limit: Some(1),
                ..EventFilter::all()
            })
            .expect("limit")
            .len(),
        1,
    );
}
