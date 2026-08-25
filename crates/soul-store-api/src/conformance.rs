//! A reusable acceptance suite for any [`SoulStore`] implementation.
//!
//! WP01 runs it against [`crate::FakeStore`]. WP02's SQLCipher backend must
//! pass exactly the same calls, which is what keeps the storage boundary from
//! quietly meaning two different things.

use uuid::Uuid;

use soul_schema::audit::{AuditAction, AuditDecision, SoulAuditEntry};
use soul_schema::common::{
    ActorSubject, Derivation, EgressPolicy, NotAClinicalClaim, Privacy, Purpose, Retention,
    SchemaVersion, SealedSubject, Sha256Hex, Subject, SupportedBand, Timestamp,
};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_schema::evidence::{EvidenceKind, SoulEvidence};
use soul_schema::inference::SoulInference;
use soul_schema::memory::{ForgetState, MemoryType, SoulMemory};
use soul_schema::relationship::{EgressScope, SoulRelationship};

use crate::forget::ForgetUnit;
use crate::types::{EventFilter, InferenceState, SealRequest, StoreError};
use crate::SoulStore;

/// Run every conformance check against a freshly built store.
///
/// `mk` is called once per check so that a failure in one cannot mask another
/// through leftover state.
pub fn run_conformance<S: SoulStore>(mk: impl Fn() -> S) {
    events_are_appended_and_filterable(&mk);
    inference_needs_resolvable_evidence(&mk);
    sealed_text_round_trips(&mk);
    sealed_text_is_bound_to_its_row_and_field(&mk);
    graph_edges_are_reachable_from_either_end(&mk);
    the_graph_listings_see_everything_that_was_written(&mk);
    forget_preview_is_read_only_and_matches_execution(&mk);
    forget_destroys_the_key_and_orphans_derived_inference(&mk);
    audit_survives_a_forget(&mk);
}

// ---------------------------------------------------------------- checks ---

fn events_are_appended_and_filterable<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();
    let imported = event(EventSource::ImportSoulImportV1, EventKind::ImportItem, "10");
    let foreground = event(
        EventSource::CollectorForegroundApp,
        EventKind::AppForeground,
        "11",
    );
    let imported_id = store.append_event(imported.clone()).expect("append import");
    store.append_event(foreground).expect("append foreground");

    assert_eq!(
        store.get_event(imported_id).expect("read back").event_id,
        imported_id,
    );
    assert_eq!(
        store.count_events(&EventFilter::all()).expect("count"),
        2,
        "both events are visible",
    );
    assert_eq!(
        store
            .count_events(&EventFilter::with_kind(EventKind::AppForeground))
            .expect("count"),
        1,
        "filtering by kind narrows the result",
    );

    assert!(
        matches!(
            store.append_event(imported),
            Err(StoreError::AlreadyExists { .. })
        ),
        "the event log must not accept the same event_id twice",
    );

    let missing = uuid7("ff");
    assert!(
        matches!(store.get_event(missing), Err(StoreError::NotFound { .. })),
        "an unknown event_id is a NotFound, not a panic",
    );
}

fn inference_needs_resolvable_evidence<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();

    let unsupported = inference("20", &[]);
    assert!(
        matches!(
            store.put_inference(unsupported),
            Err(StoreError::ContractViolation(_))
        ),
        "an inference with no evidence must be refused",
    );

    let dangling = inference("21", &[uuid7("a10")]);
    assert!(
        matches!(
            store.put_inference(dangling),
            Err(StoreError::ContractViolation(_))
        ),
        "an inference citing evidence that does not resolve must be refused",
    );

    store.put_evidence(evidence("a10")).expect("store evidence");
    let supported = inference("22", &[uuid7("a10")]);
    let id = store.put_inference(supported).expect("supported inference");
    assert_eq!(
        store.inference_state(id).expect("state"),
        InferenceState::Live,
    );
}

fn sealed_text_round_trips<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();
    let row = uuid7("50");
    let key = uuid7("51");
    let text = "第一次去北京，站台上风很大";

    let sealed = store
        .seal(SealRequest::new(
            key,
            row,
            "summary_ref",
            SealedSubject::Owner,
            text.as_bytes().to_vec(),
        ))
        .expect("seal");

    assert_eq!(sealed.content_key_id, key);
    assert_eq!(
        sealed.char_count,
        text.chars().count() as u64,
        "char_count counts scalars, not bytes",
    );
    assert!(store.has_content_key(key));

    let opened = store.open(&sealed).expect("open");
    assert_eq!(
        String::from_utf8(opened).expect("utf-8 survives sealing"),
        text,
    );
}

fn sealed_text_is_bound_to_its_row_and_field<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();
    let key = uuid7("51");
    let sealed = store
        .seal(SealRequest::new(
            key,
            uuid7("50"),
            "summary_ref",
            SealedSubject::Owner,
            b"bound to one field".to_vec(),
        ))
        .expect("seal");

    let mut replayed_into_another_field = sealed.clone();
    replayed_into_another_field.aad = Some(format!("{}|title_ref", uuid7("50")));
    assert!(
        matches!(
            store.open(&replayed_into_another_field),
            Err(StoreError::SealBroken(_))
        ),
        "a sealed blob must not open under a different field name",
    );

    let mut replayed_into_another_row = sealed;
    replayed_into_another_row.aad = Some(format!("{}|summary_ref", uuid7("52")));
    assert!(
        matches!(
            store.open(&replayed_into_another_row),
            Err(StoreError::SealBroken(_))
        ),
        "a sealed blob must not open under a different row id",
    );
}

fn graph_edges_are_reachable_from_either_end<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();
    let left = uuid7("60");
    let right = uuid7("61");
    store.put_contact(contact(left)).expect("left contact");
    store.put_contact(contact(right)).expect("right contact");
    store.put_evidence(evidence("a10")).expect("evidence");
    let edge_id = store
        .put_relationship(relationship("70", left, right, &[uuid7("a10")]))
        .expect("edge");

    assert_eq!(
        store
            .get_relationship(edge_id)
            .expect("read back")
            .relationship_id,
        edge_id
    );
    assert_eq!(store.relationships_for(left).expect("from side").len(), 1);
    assert_eq!(store.relationships_for(right).expect("to side").len(), 1);
    assert_eq!(store.list_contacts().expect("contacts").len(), 2);
}

/// WP05 derives the whole graph in one pass, so the two listing queries have
/// to see rows written through the single-row entry points. A backend that
/// answered from a stale index would build a graph missing whichever edge it
/// forgot about.
fn the_graph_listings_see_everything_that_was_written<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();
    let left = uuid7("60");
    let right = uuid7("61");
    store.put_contact(contact(left)).expect("left contact");
    store.put_contact(contact(right)).expect("right contact");
    store.put_evidence(evidence("a10")).expect("first evidence");
    store
        .put_evidence(evidence("a11"))
        .expect("second evidence");
    let edge_id = store
        .put_relationship(relationship("70", left, right, &[uuid7("a10")]))
        .expect("edge");

    let listed = store.list_evidence().expect("list evidence");
    assert_eq!(listed.len(), 2, "both evidence rows are listed");
    assert!(
        listed.iter().any(|row| row.evidence_id == uuid7("a11")),
        "the listing must return the rows, not just the right count",
    );

    let edges = store.list_relationships().expect("list relationships");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].relationship_id, edge_id);
    assert_eq!(edges[0].evidence_ids, vec![uuid7("a10")]);

    // Rewriting an edge under the same id updates it rather than adding a
    // second one, which is what makes a graph rebuild idempotent.
    store
        .put_relationship(relationship(
            "70",
            left,
            right,
            &[uuid7("a10"), uuid7("a11")],
        ))
        .expect("rewrite the edge");
    let edges = store.list_relationships().expect("list again");
    assert_eq!(edges.len(), 1, "the rewrite must not duplicate the edge");
    assert_eq!(edges[0].evidence_ids.len(), 2);
}

fn forget_preview_is_read_only_and_matches_execution<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();
    let memory_id = seed_memory_with_inference(&mut store);

    let first = store
        .preview_impact(ForgetUnit::Memory(memory_id))
        .expect("preview");
    let second = store
        .preview_impact(ForgetUnit::Memory(memory_id))
        .expect("preview twice");
    assert_eq!(first, second, "preview must not change anything");
    assert_eq!(
        store
            .get_memory(memory_id)
            .expect("memory still there")
            .forget_state,
        ForgetState::Active,
        "preview must not forget anything",
    );
    assert_eq!(first.memories_affected, 1);
    assert_eq!(first.inferences_orphaned, 1);
    assert!(first.sealed_blobs_destroyed >= 1);

    let receipt = store
        .execute_forget(ForgetUnit::Memory(memory_id))
        .expect("execute");
    assert_eq!(
        receipt.impact, first,
        "the user must not be shown one number and charged another",
    );
}

fn forget_destroys_the_key_and_orphans_derived_inference<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();
    let memory_id = seed_memory_with_inference(&mut store);

    let memory = store.get_memory(memory_id).expect("memory");
    let summary = memory.summary_ref.clone().expect("sealed summary");
    assert_eq!(
        String::from_utf8(store.open(&summary).expect("readable before forget")).expect("utf-8"),
        SUMMARY_TEXT,
    );

    store
        .execute_forget(ForgetUnit::Memory(memory_id))
        .expect("execute forget");

    assert!(
        !store.has_content_key(memory.content_key_id),
        "the content key must be gone",
    );
    assert!(
        matches!(
            store.open(&summary),
            Err(StoreError::ContentKeyDestroyed(_))
        ),
        "the prose must be unreadable once its key is destroyed",
    );
    assert_eq!(
        store
            .get_memory(memory_id)
            .expect("row remains")
            .forget_state,
        ForgetState::Forgotten,
        "the row remains as a tombstone so the user can see the memory existed",
    );

    let inference_id = uuid7("22");
    assert_eq!(
        store.inference_state(inference_id).expect("state"),
        InferenceState::Orphaned,
        "an inference whose evidence was forgotten must be demoted, not kept as-is",
    );
}

fn audit_survives_a_forget<S: SoulStore>(mk: &impl Fn() -> S) {
    let mut store = mk();
    let memory_id = seed_memory_with_inference(&mut store);
    store
        .append_audit(audit_entry(memory_id))
        .expect("append audit");

    let before = store.list_audit().expect("audit before").len();
    let receipt = store
        .execute_forget(ForgetUnit::Memory(memory_id))
        .expect("execute forget");
    let after = store.list_audit().expect("audit after");

    assert_eq!(
        after.len(),
        before,
        "forgetting must not delete audit entries"
    );
    assert_eq!(
        receipt.impact.audit_entries_retained, 1,
        "the preview must be honest that the audit entry is kept",
    );
    let encoded = serde_json::to_string(&after).expect("audit serializes");
    assert!(
        !encoded.contains(SUMMARY_TEXT),
        "the audit chain must never have carried the prose in the first place",
    );
}

// ---------------------------------------------------------------- setup ---

const SUMMARY_TEXT: &str = "第一次去北京，站台上风很大";

/// One memory with sealed prose, one piece of evidence it cites, and one
/// inference resting on that evidence.
fn seed_memory_with_inference<S: SoulStore>(store: &mut S) -> Uuid {
    let memory_id = uuid7("50");
    let content_key_id = uuid7("51");

    let title = store
        .seal(SealRequest::new(
            content_key_id,
            memory_id,
            "title_ref",
            SealedSubject::Owner,
            "北京".as_bytes().to_vec(),
        ))
        .expect("seal title");
    let summary = store
        .seal(SealRequest::new(
            content_key_id,
            memory_id,
            "summary_ref",
            SealedSubject::Mixed,
            SUMMARY_TEXT.as_bytes().to_vec(),
        ))
        .expect("seal summary")
        .clone();

    store.put_evidence(evidence("a10")).expect("evidence");
    store
        .put_inference(inference("22", &[uuid7("a10")]))
        .expect("inference");

    store
        .put_memory(SoulMemory {
            schema_version: SchemaVersion,
            memory_id,
            memory_type: MemoryType::Episodic,
            title_ref: Some(title),
            summary_ref: Some(summary),
            source_event_ids: None,
            evidence_ids: Some(vec![uuid7("a10")]),
            content_key_id,
            forget_state: ForgetState::Active,
            third_party_content_present: Some(true),
        })
        .expect("memory");

    memory_id
}

fn uuid7(tail: &str) -> Uuid {
    let padded = format!("{tail:0>3}");
    format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{padded}")
        .parse()
        .expect("hand-written UUIDv7 literal")
}

fn privacy() -> Privacy {
    Privacy {
        subject: Subject::Owner,
        derivation: Derivation::Raw,
        purposes: vec![Purpose::SoulProfile],
        retention: Retention::until_forgotten(),
        egress: EgressPolicy::default(),
    }
}

fn event(source: EventSource, kind: EventKind, tail: &str) -> SoulEvent {
    SoulEvent {
        schema_version: SchemaVersion,
        event_id: uuid7(tail),
        ts: Timestamp::new("2026-08-24T10:00:00Z"),
        source,
        kind,
        actor_subject: ActorSubject::Owner,
        consent_id: None,
        privacy: privacy(),
        body_ref: None,
    }
}

fn evidence(tail: &str) -> SoulEvidence {
    SoulEvidence {
        schema_version: SchemaVersion,
        evidence_id: uuid7(tail),
        kind: EvidenceKind::UserStatement,
        subject: Subject::Owner,
        source_refs: vec![serde_json::json!({ "origin": "conformance" })],
        strength: SupportedBand::Moderate,
        method: None,
        exportable_to_research: Some(false),
        privacy: Some(privacy()),
    }
}

fn inference(tail: &str, evidence_ids: &[Uuid]) -> SoulInference {
    SoulInference {
        schema_version: SchemaVersion,
        inference_id: uuid7(tail),
        target: serde_json::json!({ "kind": "trait_axis", "axis_id": uuid7("30") }),
        statement_key: "voice.directness.leans_high".into(),
        evidence_ids: evidence_ids.to_vec(),
        evidence_band: SupportedBand::Weak,
        method: None,
        user_verdict: None,
        clinical_claim: NotAClinicalClaim,
        falsifier: None,
    }
}

fn contact(contact_id: Uuid) -> SoulContact {
    SoulContact {
        schema_version: SchemaVersion,
        contact_id,
        contact_class: ContactClass::ThirdParty,
        display_label_ref: None,
        identifiers: None,
        forget_state: ForgetState::Active,
    }
}

fn relationship(tail: &str, from: Uuid, to: Uuid, evidence_ids: &[Uuid]) -> SoulRelationship {
    SoulRelationship {
        schema_version: SchemaVersion,
        relationship_id: uuid7(tail),
        from_contact_id: from,
        to_contact_id: to,
        types: None,
        tie_strength: None,
        evidence_ids: evidence_ids.to_vec(),
        egress_scope: Some(EgressScope::LocalOnly),
    }
}

fn audit_entry(subject: Uuid) -> SoulAuditEntry {
    SoulAuditEntry {
        schema_version: SchemaVersion,
        seq: 0,
        entry_id: uuid7("80"),
        ts: Timestamp::new("2026-08-24T11:00:00Z"),
        prev_hash: Sha256Hex::new("0".repeat(64)),
        entry_hash: Sha256Hex::new("1".repeat(64)),
        action: AuditAction::MemoryWrite,
        decision: AuditDecision::Allowed,
        reason_code: None,
        subject_refs: Some(vec![subject]),
        counts: None,
        plan_hash: None,
        capability_token_id: None,
        egress_class: None,
    }
}
