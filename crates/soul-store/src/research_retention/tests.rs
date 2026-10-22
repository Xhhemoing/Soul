use super::*;
use serde::Deserialize;
use soul_schema::common::{Purpose, SealedSubject, Subject};
use soul_schema::event::{EventKind, SoulEvent};
use soul_schema::validate::SchemaId;
use soul_store_api::types::SealRequest;
use soul_store_api::{BlobStore, EventStore};
use uuid::Uuid;

#[path = "../../tests/common/mod.rs"]
mod fixtures;

const SEED: &str = "retention adversarial synthetic";

fn setup() -> (tempfile::TempDir, SqlCipherStore, SoulEvent) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SqlCipherStore::open(
        dir.path().join("retention.db"),
        &crate::TestKeyProvider::from_seed(SEED),
    )
    .unwrap();
    let id = Uuid::now_v7();
    let mut event = fixtures::bucketed_event(
        id,
        "2024-02-29T09:10:00Z",
        EventKind::AppForeground,
        Subject::Owner,
    );
    event.privacy.purposes = vec![Purpose::Research];
    event.body_ref = Some(
        store
            .seal(SealRequest::new(
                Uuid::now_v7(),
                id,
                "body_ref",
                SealedSubject::Owner,
                b"synthetic body".to_vec(),
            ))
            .unwrap(),
    );
    store.append_event(event.clone()).unwrap();
    (dir, store, event)
}

fn scope() -> ResearchScope {
    ResearchScope {
        from_day: "2024-02-29".into(),
        to_day: "2024-03-01".into(),
    }
}

#[test]
fn borrowed_and_owned_event_values_preserve_optional_unicode_and_bad_type_results() {
    // Equivalence protection for replacing the owned Value conversion, not a
    // new classifier: every variant also goes through the real preparation.
    let (_dir, store, event) = setup();
    let original = serde_json::to_string(&event).unwrap();
    let expected = store.prepare_research_retention(&scope()).unwrap();
    assert_eq!(expected.dataset().counts.included_events, 1);
    for (sql, typed_ok, included) in [
        ("UPDATE events SET doc=?1", true, true),
        (
            "UPDATE events SET doc=json_set(?1,'$.consent_id',NULL,
             '$.privacy.retention.days',NULL,'$.body_ref.placeholder',NULL)",
            true,
            true,
        ),
        (
            "UPDATE events SET doc=json_set(?1,
             '$.consent_id','0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4000',
             '$.privacy.retention.days',30,'$.body_ref.placeholder','合成占位：🧪 é')",
            true,
            true,
        ),
        (
            "UPDATE events SET doc=json_remove(?1,'$.body_ref')",
            true,
            false,
        ),
        (
            "UPDATE events SET doc=json_set(?1,'$.body_ref',NULL)",
            true,
            false,
        ),
        ("UPDATE events SET doc=json_set(?1,'$.ts',17)", false, false),
        (
            "UPDATE events SET doc=json_set(?1,'$.body_ref.char_count','17')",
            false,
            false,
        ),
        (
            "UPDATE events SET doc=json_set(?1,'$.consent_id',json('[]'))",
            false,
            false,
        ),
        (
            "UPDATE events SET doc=json_set(?1,'$.privacy.retention.days',-1)",
            false,
            false,
        ),
        (
            "UPDATE events SET doc=json_set(?1,'$.body_ref.placeholder',json('[\"合成\"]'))",
            false,
            false,
        ),
    ] {
        store.conn.execute(sql, [&original]).unwrap();
        let raw: String = store
            .conn
            .query_row("SELECT doc FROM events", [], |row| row.get(0))
            .unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let owned = serde_json::from_value::<SoulEvent>(doc.clone());
        let borrowed = SoulEvent::deserialize(&doc);
        assert_eq!(owned.is_ok(), typed_ok, "{sql}");
        assert_eq!(borrowed.is_ok(), typed_ok, "{sql}");
        if let Ok(owned) = owned {
            assert_eq!(borrowed.unwrap(), owned, "{sql}");
        }
        assert_eq!(
            store.schemas.is_valid(SchemaId::Event, &doc),
            store.schemas.validate(SchemaId::Event, &doc).is_ok(),
            "{sql}"
        );
        let held = store.prepare_research_retention(&scope()).unwrap();
        assert_eq!(
            held.dataset().counts,
            ResearchCounts {
                candidate_events: 1,
                included_events: u64::from(included),
                excluded_unavailable_events: u64::from(!included),
                ..ResearchCounts::default()
            },
            "{sql}"
        );
        if included {
            assert_eq!(held.dataset(), expected.dataset(), "{sql}");
        } else {
            assert!(held.dataset().rows.is_empty(), "{sql}");
        }
    }
}

#[test]
fn typed_readable_schema_errors_still_exclude_the_source() {
    for (sql, minimum_errors) in [
        (
            "UPDATE events SET doc=json_set(doc,'$.synthetic_unknown_field',1)",
            1,
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.synthetic_unknown_field',1,
             '$.body_ref.synthetic_unknown_field','合成',
             '$.privacy.purposes',json('[\"research\",\"research\"]'),
             '$.privacy.retention.days',0)",
            2,
        ),
    ] {
        let (_dir, store, _) = setup();
        store.conn.execute_batch(sql).unwrap();
        let raw: String = store
            .conn
            .query_row("SELECT doc FROM events", [], |row| row.get(0))
            .unwrap();
        let doc: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let owned = serde_json::from_value::<SoulEvent>(doc.clone()).unwrap();
        assert_eq!(SoulEvent::deserialize(&doc).unwrap(), owned, "{sql}");
        let errors = store.schemas.validate(SchemaId::Event, &doc).unwrap_err();
        assert!(errors.errors.len() >= minimum_errors, "{sql}");
        assert!(!store.schemas.is_valid(SchemaId::Event, &doc), "{sql}");
        let held = store.prepare_research_retention(&scope()).unwrap();
        assert!(held.dataset().rows.is_empty(), "{sql}");
        assert_eq!(
            held.dataset().counts,
            ResearchCounts {
                candidate_events: 1,
                excluded_unavailable_events: 1,
                ..ResearchCounts::default()
            },
            "{sql}"
        );
    }
}

#[test]
fn overlapping_defects_keep_policy_unavailable_and_time_exclusions_in_order() {
    for (sql, excluded) in [
        (
            "UPDATE events SET doc=json_set(doc,'$.privacy.egress.research_export','deny',
             '$.body_ref.char_count','bad','$.synthetic_unknown_field',1,
             '$.ts','2024-02-29T25:10:00Z'),ts='2024-02-29T25:10:00Z'",
            [1, 0, 0],
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.body_ref.char_count','bad',
             '$.synthetic_unknown_field',1,'$.ts','2024-02-29T25:10:00Z'),
             ts='2024-02-29T25:10:00Z'",
            [0, 1, 0],
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.ts','2024-02-29T25:10:00Z',
             '$.synthetic_unknown_field',1)",
            [0, 1, 0],
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.ts','2024-02-29T25:10:00Z',
             '$.synthetic_unknown_field',1),ts='2024-02-29T25:10:00Z';
             UPDATE sealed_blobs SET field='wrong'",
            [0, 0, 1],
        ),
    ] {
        let (_dir, store, _) = setup();
        store.conn.execute_batch(sql).unwrap();
        let held = store.prepare_research_retention(&scope()).unwrap();
        assert!(held.dataset().rows.is_empty(), "{sql}");
        let counts = &held.dataset().counts;
        assert_eq!(counts.candidate_events, 1, "{sql}");
        assert_eq!(
            counts.included_events + counts.out_of_range_events,
            0,
            "{sql}"
        );
        assert_eq!(
            [
                counts.excluded_policy_events,
                counts.excluded_unavailable_events,
                counts.excluded_time_events,
            ],
            excluded,
            "{sql}"
        );
    }
}

#[test]
fn broken_blob_and_key_bindings_are_unavailable_without_decryption() {
    for sql in [
        "DELETE FROM sealed_blobs",
        "DELETE FROM content_keys",
        "INSERT INTO destroyed_content_keys SELECT content_key_id FROM content_keys",
        "UPDATE sealed_blobs SET row_id='0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4000'",
        "UPDATE sealed_blobs SET field='other'",
        "UPDATE sealed_blobs SET subject='third_party'",
        "UPDATE sealed_blobs SET content_key_id='0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4000'",
        "UPDATE events SET body_blob_id=NULL",
        "UPDATE events SET ts='2024-02-29T08:00:00Z'",
    ] {
        let (_dir, store, _) = setup();
        store.conn.execute_batch(sql).unwrap();
        let held = store.prepare_research_retention(&scope()).unwrap();
        assert!(held.dataset().rows.is_empty(), "{sql}");
        assert_eq!(
            held.dataset().counts.excluded_unavailable_events,
            1,
            "{sql}"
        );
    }
}

#[test]
fn policy_and_typed_changes_fail_closed_and_invalidate_held_sources() {
    for (sql, policy, unavailable) in [
        (
            "UPDATE events SET doc=json_set(doc,'$.privacy.subject','unknown'),privacy_subject='unknown'",
            1,
            0,
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.privacy.purposes',json('[\"memory\"]'))",
            1,
            0,
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.body_ref.char_count','bad')",
            0,
            1,
        ),
    ] {
        let (_dir, mut store, _) = setup();
        let held = store.prepare_research_retention(&scope()).unwrap();
        store.conn.execute_batch(sql).unwrap();
        let changed = store.prepare_research_retention(&scope()).unwrap();
        assert!(changed.dataset().rows.is_empty());
        assert_eq!(
            changed.dataset().counts,
            ResearchCounts {
                candidate_events: 1,
                excluded_policy_events: policy,
                excluded_unavailable_events: unavailable,
                ..ResearchCounts::default()
            },
            "{sql}"
        );
        let mut called = false;
        let error = store
            .publish_research_retention(&held, || {
                called = true;
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error, refused("RESEARCH_SOURCE_CHANGED"), "{sql}");
        assert!(!called);
        assert!(!store.transaction_in_progress());
    }
}

#[test]
fn already_excluded_sources_still_invalidate_publication_when_their_metadata_changes() {
    // These are equivalence protections: the original full preparation also
    // snapshots excluded sources, even when neither rows nor counts change.
    for (before, after, excluded) in [
        (
            "UPDATE events SET doc=json_set(doc,'$.privacy.egress.research_export','deny')",
            "UPDATE events SET doc=json_set(doc,'$.privacy.purposes',json('[\"memory\"]'))",
            [1, 0, 0],
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.synthetic_unknown_field',1)",
            "UPDATE events SET doc=json_set(doc,'$.synthetic_unknown_field',2)",
            [0, 1, 0],
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.body_ref.char_count','bad-before')",
            "UPDATE events SET doc=json_set(doc,'$.body_ref.char_count','bad-after')",
            [0, 1, 0],
        ),
        (
            "UPDATE events SET doc='{'",
            "UPDATE events SET doc='['",
            [0, 1, 0],
        ),
        (
            "UPDATE events SET ts='2024-03-01T09:10:00Z'",
            "UPDATE events SET ts='2024-03-02T09:10:00Z'",
            [0, 1, 0],
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.ts','2024-03-01T09:10:00Z')",
            "UPDATE events SET doc=json_set(doc,'$.ts','2024-03-02T09:10:00Z')",
            [0, 1, 0],
        ),
        (
            "UPDATE events SET source='import.soul_import_v1'",
            "UPDATE events SET source='ui.paste'",
            [0, 1, 0],
        ),
        (
            "UPDATE sealed_blobs SET field='wrong_before'",
            "UPDATE sealed_blobs SET field='wrong_after'",
            [0, 1, 0],
        ),
        (
            "UPDATE events SET ts='2024-02-29T25:10:00Z',doc=json_set(doc,'$.ts','2024-02-29T25:10:00Z')",
            "UPDATE events SET ts='2024-02-29T26:10:00Z',doc=json_set(doc,'$.ts','2024-02-29T26:10:00Z')",
            [0, 0, 1],
        ),
    ] {
        let (_dir, mut store, _) = setup();
        store.conn.execute_batch(before).unwrap();
        let held = store.prepare_research_retention(&scope()).unwrap();
        assert!(held.dataset().rows.is_empty(), "{before}");
        let counts = &held.dataset().counts;
        assert_eq!(counts.candidate_events, 1, "{before}");
        assert_eq!(
            counts.included_events + counts.out_of_range_events,
            0,
            "{before}"
        );
        assert_eq!(
            [
                counts.excluded_policy_events,
                counts.excluded_unavailable_events,
                counts.excluded_time_events,
            ],
            excluded,
            "{before}"
        );
        store.conn.execute_batch(after).unwrap();
        let changed = store.prepare_research_retention(&scope()).unwrap();
        assert_eq!(changed.dataset(), held.dataset(), "{after}");
        let mut called = false;
        let error = store
            .publish_research_retention(&held, || {
                called = true;
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error, refused("RESEARCH_SOURCE_CHANGED"), "{after}");
        assert!(!called, "{after}");
        assert!(!store.transaction_in_progress(), "{after}");
    }
}

#[test]
fn matching_canonical_out_of_range_sources_can_change_without_invalidating_publication() {
    for day in ["2024-02-28", "2024-03-01"] {
        let (_dir, mut store, _) = setup();
        let before = format!("{day}T09:10:00Z");
        store
            .conn
            .execute(
                "UPDATE events SET ts=?1,doc=json_set(doc,'$.ts',?1)",
                [&before],
            )
            .unwrap();
        let held = store.prepare_research_retention(&scope()).unwrap();
        assert!(held.dataset().rows.is_empty());
        assert_eq!(held.dataset().counts.candidate_events, 1);
        assert_eq!(held.dataset().counts.out_of_range_events, 1);
        let after = format!("{day}T10:10:00Z");
        // Once matching canonical timestamps prove the source unrelated,
        // policy, schema and binding defects do not make it a held source.
        store
            .conn
            .execute(
                "UPDATE events SET ts=?1,doc=json_set(doc,'$.ts',?1,
                 '$.privacy.egress.research_export','deny','$.synthetic_unknown_field',1)",
                [&after],
            )
            .unwrap();
        store
            .conn
            .execute_batch("UPDATE sealed_blobs SET field='synthetic_wrong_field'")
            .unwrap();
        let changed = store.prepare_research_retention(&scope()).unwrap();
        assert_eq!(changed.dataset(), held.dataset());
        let published = store
            .publish_research_retention(&held, || Ok("published"))
            .unwrap();
        assert_eq!(published.value, "published");
        assert!(published.transaction_closed);
    }
}

#[test]
fn publication_enforces_global_limits_even_when_all_current_candidates_are_out_of_range() {
    for (sql, expected) in [
        (
            "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<100000)
             INSERT INTO events SELECT 'synthetic-'||x,ts,source,kind,actor_subject,privacy_subject,
             body_blob_id,doc FROM n CROSS JOIN (SELECT * FROM events LIMIT 1)",
            "RESEARCH_CANDIDATE_LIMIT",
        ),
        (
            "UPDATE events SET doc=json_set(doc,'$.synthetic_padding',
             replace(hex(zeroblob(8388608)),'0','x'))",
            "RESEARCH_METADATA_LIMIT",
        ),
    ] {
        let (_dir, mut store, _) = setup();
        store
            .conn
            .execute_batch(
                "UPDATE events SET ts='2024-03-01T09:10:00Z',
                 doc=json_set(doc,'$.ts','2024-03-01T09:10:00Z')",
            )
            .unwrap();
        let held = store.prepare_research_retention(&scope()).unwrap();
        assert_eq!(held.dataset().counts.out_of_range_events, 1);
        store.conn.execute_batch(sql).unwrap();
        assert_eq!(
            store.prepare_research_retention(&scope()).unwrap_err(),
            refused(expected)
        );
        let mut called = false;
        let error = store
            .publish_research_retention(&held, || {
                called = true;
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error, refused(expected));
        assert!(!called);
        assert!(!store.transaction_in_progress());
    }
}

#[test]
fn same_id_body_rebinding_invalidates_even_if_the_aggregate_is_identical() {
    let (_dir, mut store, event) = setup();
    let held = store.prepare_research_retention(&scope()).unwrap();
    let replacement = store
        .seal(SealRequest::new(
            Uuid::now_v7(),
            event.event_id,
            "body_ref",
            SealedSubject::Owner,
            b"replacement".to_vec(),
        ))
        .unwrap();
    let mut changed = event;
    changed.body_ref = Some(replacement.clone());
    store
        .conn
        .execute(
            "UPDATE events SET body_blob_id=?1,doc=?2",
            rusqlite::params![
                replacement.blob_id.to_string(),
                serde_json::to_string(&changed).unwrap()
            ],
        )
        .unwrap();
    assert_eq!(
        store
            .prepare_research_retention(&scope())
            .unwrap()
            .dataset()
            .rows,
        held.dataset().rows
    );
    assert!(store.publish_research_retention(&held, || Ok(())).is_err());
}

#[test]
fn publication_holds_an_immediate_transaction_against_a_second_writer() {
    let (_dir, mut store, _) = setup();
    let second =
        SqlCipherStore::open(store.path(), &crate::TestKeyProvider::from_seed(SEED)).unwrap();
    second.conn.busy_timeout(std::time::Duration::ZERO).unwrap();
    let held = store.prepare_research_retention(&scope()).unwrap();
    let result = store
        .publish_research_retention(&held, || {
            let error = second.conn.execute_batch("BEGIN IMMEDIATE").unwrap_err();
            assert!(matches!(error, rusqlite::Error::SqliteFailure(ref code, _)
            if code.code == rusqlite::ErrorCode::DatabaseBusy));
            Ok("published")
        })
        .unwrap();
    assert!(result.transaction_closed);
    second
        .conn
        .execute_batch("BEGIN IMMEDIATE; ROLLBACK")
        .unwrap();
}

#[test]
fn publication_value_survives_a_failure_to_close_the_transaction() {
    let (_dir, mut store, _) = setup();
    let held = store.prepare_research_retention(&scope()).unwrap();
    let result = store
        .publish_research_with_store(&held, |store| {
            // End the actual transaction so the wrapping COMMIT genuinely fails.
            // This seam stays private: production callers cannot mutate the store.
            store.conn.execute_batch("ROLLBACK").unwrap();
            Ok("already published")
        })
        .unwrap();
    assert_eq!(result.value, "already published");
    assert!(!result.transaction_closed);
    assert!(!store.transaction_in_progress());
}

#[test]
fn excessive_candidate_count_and_metadata_are_refused_not_truncated() {
    let (_dir, store, _) = setup();
    assert_eq!(
        store
            .prepare_research_retention(&scope())
            .unwrap()
            .dataset()
            .counts
            .included_events,
        1
    );
    store
        .conn
        .execute_batch(
            "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<100000)
         INSERT INTO events SELECT 'synthetic-'||x,ts,source,kind,actor_subject,privacy_subject,
         body_blob_id,doc FROM n CROSS JOIN (SELECT * FROM events LIMIT 1)",
        )
        .unwrap();
    assert_eq!(
        store.prepare_research_retention(&scope()).unwrap_err(),
        refused("RESEARCH_CANDIDATE_LIMIT")
    );
    let (_dir, store, _) = setup();
    store
        .conn
        .execute(
            "UPDATE events SET doc=?1",
            ["x".repeat(16 * 1024 * 1024 + 1)],
        )
        .unwrap();
    assert_eq!(
        store.prepare_research_retention(&scope()).unwrap_err(),
        refused("RESEARCH_METADATA_LIMIT")
    );
}

#[test]
fn query_never_opens_the_body_or_uses_other_sources() {
    let (_dir, store, _) = setup();
    // Same-size invalid ciphertext would fail AEAD if this path decrypted it.
    store
        .conn
        .execute_batch("UPDATE sealed_blobs SET ciphertext=zeroblob(length(ciphertext))")
        .unwrap();
    assert_eq!(
        store
            .prepare_research_retention(&scope())
            .unwrap()
            .dataset()
            .counts
            .included_events,
        1
    );
    store
        .conn
        .execute_batch(
            "UPDATE events SET source='import.soul_import_v1',
         doc=json_set(doc,'$.source','import.soul_import_v1')",
        )
        .unwrap();
    let held = store.prepare_research_retention(&scope()).unwrap();
    assert_eq!(held.dataset().counts.candidate_events, 0);
    assert!(held.dataset().rows.is_empty());
}
