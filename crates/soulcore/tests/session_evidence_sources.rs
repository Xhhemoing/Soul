//! Synthetic SQLCipher and loopback checks of explicit, single-event local reads.
use soul_graph::interaction::{Direction, Venue};
use soul_schema::common::{Purpose, Subject};
use soul_schema::event::SoulEvent;
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_store_api::{BlobStore, EventStore, ForgetOps, ForgetUnit, GraphStore, ProfileStore};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::import::MAX_MESSAGE_CHARS;
use soulcore::commands::session::Session;
use soulcore::commands::sources::{
    EvidenceEventAvailability, EvidenceEventRequest, EvidenceSourcesRequest,
};
use uuid::Uuid;

fn imported(source: ImportSourceKind, text: String) -> (tempfile::TempDir, Session) {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    assert!(session.status().store_opened);
    let work = session
        .begin_import_prepare(text, source, None, LegacyImportChoice::Reject)
        .unwrap();
    let preview = session.complete_import_prepare(work.execute()).unwrap();
    let work = session
        .begin_import(
            &preview.operation.operation_id.to_string(),
            &preview.operation.preview_id.to_string(),
        )
        .unwrap();
    session.complete_import(work.execute()).unwrap();
    (dir, session)
}
fn fixture() -> (tempfile::TempDir, Session) {
    imported(
        ImportSourceKind::SoulImportV1,
        fixtures::read_text("import/soul-import-v1/three_partners.jsonl").unwrap(),
    )
}
fn requests(session: &Session) -> Vec<EvidenceSourcesRequest> {
    let shared = session.store().unwrap();
    let store = shared.lock().unwrap();
    let graph = soul_graph::load(&*store).unwrap();
    graph
        .edges
        .iter()
        .flat_map(|edge| {
            let contact = if Some(edge.from_contact_id) == graph.self_contact_id {
                edge.to_contact_id
            } else {
                edge.from_contact_id
            };
            edge.evidence_ids
                .iter()
                .map(move |id| EvidenceSourcesRequest {
                    contact_id: contact,
                    relationship_id: edge.relationship_id,
                    evidence_id: *id,
                })
        })
        .collect()
}
fn raw(request: &EvidenceSourcesRequest, event_id: Uuid) -> EvidenceEventRequest {
    EvidenceEventRequest {
        contact_id: request.contact_id,
        relationship_id: request.relationship_id,
        evidence_id: request.evidence_id,
        event_id,
    }
}
fn first(session: &Session) -> (EvidenceSourcesRequest, EvidenceEventRequest) {
    let request = requests(session).remove(0);
    let event = session.evidence_sources(&request).unwrap().events[0].event_id;
    (request.clone(), raw(&request, event))
}
fn fault_connection(dir: &std::path::Path) -> rusqlite::Connection {
    use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(
            dir.join(soulcore::commands::session::KEY_BLOB_FILE_NAME),
        ))
    } else {
        Box::new(TestKeyProvider::in_dir(dir))
    };
    let conn = rusqlite::Connection::open(soulcore::commands::store::database_path(dir)).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    conn
}
fn replace_event(dir: &std::path::Path, event: &SoulEvent) {
    fault_connection(dir)
        .execute(
            "UPDATE events SET doc=?2 WHERE event_id=?1",
            rusqlite::params![
                event.event_id.to_string(),
                serde_json::to_string(event).unwrap()
            ],
        )
        .unwrap();
}

#[test]
fn jsonl_and_telegram_metadata_and_one_body_are_local_exact_and_leave_audit_unchanged() {
    for (source, path) in [
        (
            ImportSourceKind::SoulImportV1,
            "import/soul-import-v1/three_partners.jsonl",
        ),
        (
            ImportSourceKind::TelegramDesktop,
            "import/telegram/result_basic.json",
        ),
    ] {
        let input = fixtures::read_text(path).unwrap();
        let staged = match source {
            ImportSourceKind::SoulImportV1 => soul_import::soul_import_v1::parse(&input).unwrap(),
            ImportSourceKind::TelegramDesktop => {
                soul_import::telegram::parse(&serde_json::from_str(&input).unwrap()).unwrap()
            }
        };
        let expected: Vec<_> = staged
            .messages
            .iter()
            .map(|message| message.body.as_str().to_owned())
            .collect();
        let (_dir, mut session) = imported(source, input.clone());
        let endpoint = MockLlm::start().unwrap();
        session.set_user_endpoint(&endpoint.base_url()).unwrap();
        let before = serde_json::to_value(session.audit().unwrap()).unwrap();
        let mut outgoing = false;
        let mut incoming = false;
        let mut incoming_group = false;
        let mut no_body_events = std::collections::BTreeSet::new();
        for request in requests(&session) {
            let metadata = session.evidence_sources(&request).unwrap();
            assert_eq!(metadata.evidence_id, request.evidence_id);
            assert!(!metadata.events.is_empty());
            for event in &metadata.events {
                if event.availability == EvidenceEventAvailability::NoBody {
                    assert!(event.namespace_id.is_some());
                    assert_eq!(event.body_chars, None);
                    assert!(
                        staged
                            .messages
                            .iter()
                            .any(|message| message.occurred_at == event.occurred_at
                                && message.body.is_empty()),
                        "NoBody must correspond to a genuinely empty staged message"
                    );
                    let refusal = session
                        .read_evidence_event(&raw(&request, event.event_id))
                        .unwrap_err();
                    assert_eq!(refusal.reason_code, "EVIDENCE_SOURCE_UNAVAILABLE");
                    no_body_events.insert(event.event_id);
                    continue;
                }
                assert_eq!(event.availability, EvidenceEventAvailability::Available);
                let value = session
                    .read_evidence_event(&raw(&request, event.event_id))
                    .unwrap();
                assert_eq!(value.contact_id, request.contact_id);
                assert_eq!(value.relationship_id, request.relationship_id);
                assert_eq!(value.evidence_id, request.evidence_id);
                assert_eq!(value.event_id, event.event_id);
                assert_eq!(Some(value.namespace_id), event.namespace_id);
                assert_eq!(value.source, event.source);
                assert_eq!(value.occurred_at, event.occurred_at);
                assert_eq!(value.direction, event.direction);
                assert_eq!(value.venue, event.venue);
                assert!(!value.body.is_empty());
                assert!(
                    expected.contains(&value.body),
                    "the body must be original fixture text"
                );
                assert!(!format!("{value:?}").contains(&value.body));
                outgoing |= value.direction == Direction::Outgoing;
                incoming |= value.direction == Direction::Incoming;
                incoming_group |=
                    value.direction == Direction::Incoming && value.venue == Venue::Group;
            }
        }
        assert!(incoming && outgoing && incoming_group);
        assert_eq!(
            no_body_events.len(),
            staged
                .messages
                .iter()
                .filter(|message| message.body.is_empty())
                .count()
        );
        assert_eq!(endpoint.request_count(), 0);
        assert_eq!(
            serde_json::to_value(session.audit().unwrap()).unwrap(),
            before
        );
    }
}

#[test]
fn metadata_does_not_authenticate_or_open_message_bodies_and_one_raw_read_ignores_other_corrupt_bodies(
) {
    let (dir, session) = fixture();
    let (request, selected) = first(&session);
    let shared = session.store().unwrap();
    let store = shared.lock().unwrap();
    let selected_event = store.get_event(selected.event_id).unwrap();
    drop(store);
    let another = requests(&session).into_iter().find_map(|candidate| {
        let metadata = session.evidence_sources(&candidate).unwrap();
        metadata
            .events
            .into_iter()
            .find(|event| event.event_id != selected.event_id)
            .map(|event| event.event_id)
    });
    let another = another.unwrap();
    let store = shared.lock().unwrap();
    let other = store.get_event(another).unwrap().body_ref.unwrap();
    drop(store);
    let conn = fault_connection(dir.path());
    conn.execute(
        "UPDATE sealed_blobs SET ciphertext=zeroblob(length(ciphertext)) WHERE blob_id=?1",
        [other.blob_id.to_string()],
    )
    .unwrap();
    assert!(session.read_evidence_event(&selected).is_ok());
    let pointer = selected_event.body_ref.unwrap();
    conn.execute(
        "UPDATE sealed_blobs SET ciphertext=zeroblob(length(ciphertext)) WHERE blob_id=?1",
        [pointer.blob_id.to_string()],
    )
    .unwrap();
    let metadata = session.evidence_sources(&request).unwrap();
    assert_eq!(
        metadata.events[0].availability,
        EvidenceEventAvailability::Available,
        "metadata never opens the body to check its authentication tag"
    );
    assert!(session.read_evidence_event(&selected).is_err());
}

#[test]
fn mismatched_contact_relationship_evidence_or_event_never_returns_a_body() {
    let (_dir, session) = fixture();
    let (request, selected) = first(&session);
    let before = serde_json::to_value(session.audit().unwrap()).unwrap();
    let mut changed = selected.clone();
    changed.contact_id = Uuid::now_v7();
    assert!(session.read_evidence_event(&changed).is_err());
    changed = selected.clone();
    changed.relationship_id = Uuid::now_v7();
    assert!(session.read_evidence_event(&changed).is_err());
    changed = selected.clone();
    changed.evidence_id = Uuid::now_v7();
    assert!(session.read_evidence_event(&changed).is_err());
    changed = selected.clone();
    changed.event_id = Uuid::now_v7();
    assert!(session.read_evidence_event(&changed).is_err());
    assert!(session
        .evidence_sources(&EvidenceSourcesRequest {
            contact_id: Uuid::now_v7(),
            ..request
        })
        .is_err());
    assert_eq!(
        serde_json::to_value(session.audit().unwrap()).unwrap(),
        before
    );
}

#[test]
fn changed_conversation_venue_direction_subject_or_mixed_refs_refuse_the_current_anchor() {
    for fault in [
        "conversation",
        "venue",
        "direction",
        "subject",
        "mixed",
        "purpose",
    ] {
        let (_dir, session) = fixture();
        let (request, selected) = first(&session);
        let shared = session.store().unwrap();
        let mut store = shared.lock().unwrap();
        let mut row = store.get_evidence(request.evidence_id).unwrap();
        match fault {
            "conversation" => {
                row.source_refs[0]["conversation_ref"] = serde_json::json!("a1".repeat(32))
            }
            "venue" => {
                let old = row.source_refs[0]["venue"].as_str().unwrap();
                row.source_refs[0]["venue"] =
                    serde_json::json!(if old == "direct" { "group" } else { "direct" });
            }
            "direction" => {
                let old = row.source_refs[0]["direction"].as_str().unwrap();
                row.source_refs[0]["direction"] = serde_json::json!(if old == "incoming" {
                    "outgoing"
                } else {
                    "incoming"
                });
            }
            "subject" => {
                row.subject = Subject::System;
                row.privacy.as_mut().unwrap().subject = Subject::System;
            }
            "purpose" => row.privacy.as_mut().unwrap().purposes = vec![Purpose::Research],
            _ => row
                .source_refs
                .push(serde_json::json!({"opaque":"not an interaction"})),
        }
        store.put_evidence(row).unwrap();
        drop(store);
        assert!(
            session.read_evidence_event(&selected).is_err(),
            "accepted {fault}"
        );
    }
}

#[test]
fn event_source_time_subject_and_purpose_are_revalidated_independently_of_the_graph_refs() {
    for fault in ["time", "source", "actor", "subject", "purpose"] {
        let (dir, session) = fixture();
        let (_, selected) = first(&session);
        let mut event = session
            .store()
            .unwrap()
            .lock()
            .unwrap()
            .get_event(selected.event_id)
            .unwrap();
        match fault {
            "time" => event.ts = soul_schema::common::Timestamp::new("2026-01-01T00:00:00Z"),
            "source" => event.source = soul_schema::event::EventSource::UiPaste,
            "actor" => event.actor_subject = soul_schema::common::ActorSubject::System,
            "subject" => event.privacy.subject = Subject::System,
            _ => event.privacy.purposes = vec![Purpose::Research],
        }
        replace_event(dir.path(), &event);
        assert!(
            session.read_evidence_event(&selected).is_err(),
            "accepted {fault}"
        );
    }
}

#[test]
fn legacy_without_identity_and_revoked_namespace_do_not_guess_a_source() {
    for revoked in [false, true] {
        let (dir, session) = fixture();
        let (request, selected) = first(&session);
        let namespace = session.evidence_sources(&request).unwrap().events[0]
            .namespace_id
            .unwrap();
        let conn = fault_connection(dir.path());
        if revoked {
            conn.execute(
                "UPDATE import_namespaces SET state='revoked' WHERE namespace_id=?1",
                [namespace.to_string()],
            )
            .unwrap();
        } else {
            conn.execute(
                "DELETE FROM import_message_keys WHERE event_id=?1",
                [selected.event_id.to_string()],
            )
            .unwrap();
        }
        assert_eq!(
            session.evidence_sources(&request).unwrap().events[0].availability,
            EvidenceEventAvailability::Unavailable
        );
        assert!(session.read_evidence_event(&selected).is_err());
    }
}

#[test]
fn forget_destroys_the_source_even_when_the_ui_still_has_its_identifiers() {
    let (_dir, session) = fixture();
    let (request, selected) = first(&session);
    session
        .store()
        .unwrap()
        .lock()
        .unwrap()
        .execute_forget(ForgetUnit::Contact(request.contact_id))
        .unwrap();
    assert!(session.evidence_sources(&request).is_err());
    assert!(session.read_evidence_event(&selected).is_err());
}

#[test]
fn a_real_indexed_owner_group_event_cannot_be_attached_to_a_peer_by_a_forged_interaction() {
    let (_dir, session) = fixture();
    let (request, _) = first(&session);
    let shared = session.store().unwrap();
    let mut store = shared.lock().unwrap();
    let (event, identity, namespace) = store
        .list_events(&soul_store_api::types::EventFilter::all())
        .unwrap()
        .into_iter()
        .find_map(|event| {
            let (identity, namespace) = store
                .import_source_for_event(event.event_id, (4 * MAX_MESSAGE_CHARS) as u64)
                .unwrap()?;
            (identity.group && identity.is_owner).then_some((event, identity, namespace))
        })
        .expect("the fixture contains a real indexed owner group message");
    let conversation =
        soul_import::indexed::message_conversation_ref(&*store, namespace.source, &identity)
            .unwrap();
    let mut evidence = store.get_evidence(request.evidence_id).unwrap();
    evidence.subject = Subject::Mixed;
    evidence.privacy.as_mut().unwrap().subject = Subject::Mixed;
    let observation = soul_graph::interaction::InteractionRef::new(
        event.event_id,
        namespace.owner_contact_id.unwrap(),
        request.contact_id,
        conversation,
        Direction::Outgoing,
        event.ts.clone(),
        Venue::Group,
    );
    evidence.source_refs = vec![observation.to_source_ref()];
    store.put_evidence(evidence).unwrap();
    drop(store);
    assert_eq!(
        session.evidence_sources(&request).unwrap().events[0].availability,
        EvidenceEventAvailability::Unavailable
    );
    assert!(session
        .read_evidence_event(&raw(&request, event.event_id))
        .is_err());
}

#[test]
fn utf8_character_boundary_is_supported_and_bad_plaintext_counts_or_large_ciphertext_fail_closed() {
    let body = "🦊".repeat(MAX_MESSAGE_CHARS);
    let input = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl")
        .unwrap()
        .replace("好的没问题", &body);
    let (_dir, session) = imported(ImportSourceKind::SoulImportV1, input);
    let request = requests(&session)
        .into_iter()
        .find(|request| {
            session.evidence_sources(request).unwrap().events[0].body_chars
                == Some(MAX_MESSAGE_CHARS as u64)
        })
        .unwrap();
    let selected = raw(
        &request,
        session.evidence_sources(&request).unwrap().events[0].event_id,
    );
    assert_eq!(session.read_evidence_event(&selected).unwrap().body, body);

    for fault in ["count", "utf8", "oversized", "aad", "cross_row"] {
        let (dir, session) = fixture();
        let (request, selected) = first(&session);
        let shared = session.store().unwrap();
        let mut store = shared.lock().unwrap();
        let mut event = store.get_event(selected.event_id).unwrap();
        let original = event.body_ref.as_ref().unwrap().clone();
        match fault {
            "count" => event.body_ref.as_mut().unwrap().char_count += 1,
            "aad" => event.body_ref.as_mut().unwrap().aad = Some("wrong field".into()),
            "cross_row" => {
                let sealed = store
                    .seal(soul_store_api::types::SealRequest::new(
                        original.content_key_id,
                        Uuid::now_v7(),
                        "body_ref",
                        original.subject,
                        b"unrelated source".to_vec(),
                    ))
                    .unwrap();
                event.body_ref = Some(sealed);
            }
            "utf8" => {
                event.body_ref = Some(
                    store
                        .seal(soul_store_api::types::SealRequest::new(
                            original.content_key_id,
                            event.event_id,
                            "body_ref",
                            original.subject,
                            vec![255],
                        ))
                        .unwrap(),
                )
            }
            _ => {}
        }
        drop(store);
        replace_event(dir.path(), &event);
        if fault == "oversized" {
            fault_connection(dir.path())
                .execute(
                    "UPDATE sealed_blobs SET ciphertext=zeroblob(?2) WHERE blob_id=?1",
                    rusqlite::params![
                        original.blob_id.to_string(),
                        (4 * MAX_MESSAGE_CHARS + 17) as i64
                    ],
                )
                .unwrap();
            assert_eq!(
                session.evidence_sources(&request).unwrap().events[0].availability,
                EvidenceEventAvailability::TooLarge
            );
        }
        assert!(
            session.read_evidence_event(&selected).is_err(),
            "accepted {fault}"
        );
    }
}

#[test]
fn overlarge_index_identity_is_a_local_view_limit_without_changing_import_or_dedup() {
    let long_id = "x".repeat(4 * MAX_MESSAGE_CHARS + 1);
    let input = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl")
        .unwrap()
        .replace("m-0002", &long_id);
    let (_dir, mut session) = imported(ImportSourceKind::SoulImportV1, input.clone());
    let unavailable = requests(&session)
        .into_iter()
        .find(|request| {
            session.evidence_sources(request).unwrap().events[0].availability
                == EvidenceEventAvailability::TooLarge
        })
        .unwrap();
    let metadata = session.evidence_sources(&unavailable).unwrap();
    let error = session
        .read_evidence_event(&raw(&unavailable, metadata.events[0].event_id))
        .unwrap_err();
    assert_eq!(error.reason_code, "EVIDENCE_SOURCE_TOO_LARGE");
    assert!(!format!("{error:?}").contains(&long_id));
    let bundle = session.local_insights().unwrap().remove(0);
    assert_eq!(bundle.counts.messages, 5);
    let work = session
        .begin_import_prepare(
            input,
            ImportSourceKind::SoulImportV1,
            Some(bundle.namespace_id),
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let prepared = session.complete_import_prepare(work.execute()).unwrap();
    assert_eq!(prepared.operation.preview.duplicates, 5);
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    let counts = session
        .complete_import(work.execute())
        .unwrap()
        .operation
        .receipt
        .unwrap();
    assert_eq!(counts.duplicates, 5);
    assert_eq!(counts.events_written, 0);
}

#[test]
fn an_indexed_event_without_a_saved_body_has_no_body_instead_of_another_events_text() {
    let input = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl")
        .unwrap()
        .replace("好的没问题", "");
    let (_dir, session) = imported(ImportSourceKind::SoulImportV1, input);
    let request = requests(&session)
        .into_iter()
        .find(|request| {
            session.evidence_sources(request).unwrap().events[0].availability
                == EvidenceEventAvailability::NoBody
        })
        .unwrap();
    let metadata = session.evidence_sources(&request).unwrap();
    assert!(metadata.events[0].namespace_id.is_some());
    assert_eq!(metadata.events[0].body_chars, None);
    assert!(session
        .read_evidence_event(&raw(&request, metadata.events[0].event_id))
        .is_err());
}

#[test]
fn structured_user_correction_has_no_chat_body_and_requests_cannot_supply_pointers() {
    let (_dir, mut session) = fixture();
    let (request, _) = first(&session);
    session
        .correct_tie(&request.relationship_id.to_string(), "strong")
        .unwrap();
    let correction = requests(&session)
        .into_iter()
        .find(|request| {
            session
                .store()
                .unwrap()
                .lock()
                .unwrap()
                .get_evidence(request.evidence_id)
                .unwrap()
                .kind
                == soul_schema::evidence::EvidenceKind::UserCorrection
        })
        .unwrap();
    assert!(session
        .evidence_sources(&correction)
        .unwrap()
        .events
        .is_empty());
    let wire = serde_json::json!({"contact_id":request.contact_id,"relationship_id":request.relationship_id,"evidence_id":request.evidence_id,"blob_id":Uuid::now_v7()});
    assert!(serde_json::from_value::<EvidenceSourcesRequest>(wire).is_err());
}

fn outgoing(session: &Session) -> (EvidenceSourcesRequest, EvidenceEventRequest) {
    requests(session)
        .into_iter()
        .find_map(|request| {
            let metadata = session.evidence_sources(&request).unwrap();
            metadata
                .events
                .iter()
                .find(|event| {
                    event.direction == Direction::Outgoing
                        && event.availability == EvidenceEventAvailability::Available
                })
                .map(|event| (request.clone(), raw(&request, event.event_id)))
        })
        .expect("a legitimate outgoing direct event has a current incoming speaker witness")
}

#[test]
fn a_real_owner_to_a_event_cannot_be_relabelled_as_b_even_in_the_same_namespace() {
    let (_dir, session) = fixture();
    let (a, selected) = outgoing(&session);
    assert!(session.read_evidence_event(&selected).is_ok());
    let b = requests(&session)
        .into_iter()
        .find(|request| request.contact_id != a.contact_id)
        .unwrap();
    let shared = session.store().unwrap();
    let mut store = shared.lock().unwrap();
    let a_evidence = store.get_evidence(a.evidence_id).unwrap();
    let mut b_evidence = store.get_evidence(b.evidence_id).unwrap();
    b_evidence.subject = Subject::Mixed;
    b_evidence.privacy.as_mut().unwrap().subject = Subject::Mixed;
    b_evidence.source_refs = a_evidence.source_refs;
    b_evidence.source_refs[0]["peer_contact_id"] = serde_json::json!(b.contact_id);
    store.put_evidence(b_evidence).unwrap();
    drop(store);
    let metadata = session.evidence_sources(&b).unwrap();
    assert_eq!(
        metadata.events[0].availability,
        EvidenceEventAvailability::Unavailable
    );
    assert!(session
        .read_evidence_event(&raw(&b, selected.event_id))
        .is_err());
}

#[test]
fn outgoing_reads_require_a_live_incoming_speaker_and_bound_all_witness_work() {
    for fault in ["missing", "withdrawn_purpose", "rows", "refs"] {
        let (_dir, session) = fixture();
        let (request, selected) = outgoing(&session);
        let shared = session.store().unwrap();
        let mut store = shared.lock().unwrap();
        let mut edge = store.get_relationship(request.relationship_id).unwrap();
        let incoming_ids: Vec<_> = edge
            .evidence_ids
            .iter()
            .copied()
            .filter(|id| {
                let row = store.get_evidence(*id).unwrap();
                soul_graph::interaction::interactions_in(&row)
                    .iter()
                    .any(|reference| reference.direction == Direction::Incoming)
            })
            .collect();
        assert!(!incoming_ids.is_empty());
        match fault {
            "missing" => {
                edge.evidence_ids.retain(|id| !incoming_ids.contains(id));
                store.put_relationship(edge).unwrap();
            }
            "withdrawn_purpose" => {
                for id in incoming_ids {
                    let mut row = store.get_evidence(id).unwrap();
                    row.privacy.as_mut().unwrap().purposes = vec![Purpose::Research];
                    store.put_evidence(row).unwrap();
                }
            }
            "rows" => {
                let original = store.get_evidence(incoming_ids[0]).unwrap();
                for _ in 0..65 {
                    let mut row = original.clone();
                    row.evidence_id = Uuid::now_v7();
                    edge.evidence_ids.push(row.evidence_id);
                    store.put_evidence(row).unwrap();
                }
                store.put_relationship(edge).unwrap();
            }
            _ => {
                let mut row = store.get_evidence(incoming_ids[0]).unwrap();
                row.source_refs = vec![row.source_refs[0].clone(); 65];
                store.put_evidence(row).unwrap();
            }
        }
        drop(store);
        assert!(
            session.read_evidence_event(&selected).is_err(),
            "accepted {fault}"
        );
    }
}

#[test]
fn metadata_two_outgoing_events_share_one_budget_even_when_each_raw_read_fits() {
    let (_dir, session) = fixture();
    let shared = session.store().unwrap();
    let mut store = shared.lock().unwrap();
    let graph = soul_graph::load(&*store).unwrap();
    let (edge_id, outgoing) = graph
        .edges
        .iter()
        .find_map(|edge| {
            let rows: Vec<_> = edge
                .evidence_ids
                .iter()
                .map(|id| store.get_evidence(*id).unwrap())
                .filter(|row| {
                    soul_graph::interaction::interactions_in(row)
                        .iter()
                        .all(|reference| reference.direction == Direction::Outgoing)
                })
                .collect();
            (rows.len() >= 2).then_some((edge.relationship_id, rows))
        })
        .unwrap();
    let mut combined = outgoing[0].clone();
    combined.source_refs.extend(outgoing[1].source_refs.clone());
    let observations = soul_graph::interaction::interactions_in(&combined);
    assert_eq!(observations.len(), 2);
    store.put_evidence(combined.clone()).unwrap();
    soul_graph::correct_tie(
        &mut *store,
        edge_id,
        soul_schema::common::SupportedBand::Strong,
        2_000_000_000,
    )
    .unwrap();
    let mut edge = store.get_relationship(edge_id).unwrap();
    let correction = edge
        .evidence_ids
        .iter()
        .map(|id| store.get_evidence(*id).unwrap())
        .find(|row| row.kind == soul_schema::evidence::EvidenceKind::UserCorrection)
        .unwrap();
    for _ in 0..25 {
        let mut row = correction.clone();
        row.evidence_id = Uuid::now_v7();
        edge.evidence_ids.push(row.evidence_id);
        store.put_evidence(row).unwrap();
    }
    assert!(edge.evidence_ids.len() < 64);
    store.put_relationship(edge).unwrap();
    let request = EvidenceSourcesRequest {
        contact_id: observations[0].peer_contact_id,
        relationship_id: edge_id,
        evidence_id: combined.evidence_id,
    };
    drop(store);
    // Both standalone exact-event reads have a valid incoming witness and fit.
    for observation in observations {
        assert!(session
            .read_evidence_event(&raw(&request, observation.event_id))
            .is_ok());
    }
    // The metadata call must not silently reset that work budget per event.
    assert!(session.evidence_sources(&request).is_err());
}
