//! Serde models and JSON Schema must describe the same shape.
//!
//! Each case builds a model in Rust, serializes it, validates the output
//! against the contract, then reads it back and checks the value survived.

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use soul_schema::audit::{
    AuditAction, AuditCounts, AuditDecision, EgressClass, SoulAuditEntry,
};
use soul_schema::common::{
    field_aad, ActorSubject, Derivation, E1Disposition, EgressPolicy, EvidenceBand,
    NotAClinicalClaim, Privacy, Purpose, ResearchDisposition, Retention, SchemaVersion,
    SealAlg, SealedSubject, SealedText, Sha256Hex, Subject, SupportedBand, Timestamp,
};
use soul_schema::contact::{ContactClass, ContactIdentifier, IdentifierKind, SoulContact};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_schema::export_manifest::{
    ExportField, ExportKind, ExportRow, RedactionProfile, SoulExportManifest,
    ThirdPartyBodyExcluded, ZeroThirdPartyRows,
};
use soul_schema::inference::{InferenceMethod, SoulInference, UserVerdict};
use soul_schema::memory::{ForgetState, MemoryType, SoulMemory};
use soul_schema::profile::{AxisPosition, SoulProfile, TraitAxis};
use soul_schema::relationship::{EgressScope, SoulRelationship};
use soul_schema::soul_import_v1::{
    ImportFormatTag, ImportHeader, ImportLine, ImportMessage, ImportVersionTag, SenderScope,
};
use soul_schema::validate::SchemaId;
use soul_schema::SchemaSet;

fn uuid7(tail: &str) -> Uuid {
    format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{tail}")
        .parse()
        .expect("hand-written UUIDv7 literal")
}

fn digest(input: &str) -> Sha256Hex {
    Sha256Hex::new(hex::encode(Sha256::digest(input.as_bytes())))
}

/// Serialize, validate against the contract, deserialize, compare.
fn roundtrip<T>(set: &SchemaSet, id: SchemaId, model: &T) -> Value
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let encoded = match set.validate_model(id, model) {
        Ok(value) => value,
        Err(failure) => panic!("{id} rejected its own serde model:\n{failure}"),
    };
    let decoded: T = serde_json::from_value(encoded.clone())
        .unwrap_or_else(|e| panic!("{id} model failed to deserialize its own output: {e}"));
    assert_eq!(&decoded, model, "{id} model did not survive a round trip");
    encoded
}

fn sealed(row: Uuid, field: &str, subject: SealedSubject, chars: u64) -> SealedText {
    SealedText {
        content_key_id: uuid7("a51"),
        blob_id: uuid7("a52"),
        alg: SealAlg,
        aad: Some(field_aad(&row, field)),
        subject,
        char_count: chars,
        placeholder: None,
    }
}

fn local_privacy() -> Privacy {
    Privacy::local_only(Subject::Owner, vec![Purpose::SoulProfile])
}

#[test]
fn event_round_trips() {
    let set = SchemaSet::load().expect("contracts compile");
    let event_id = uuid7("a01");
    let model = SoulEvent {
        schema_version: SchemaVersion,
        event_id,
        ts: Timestamp::new("2026-08-24T10:00:00Z"),
        source: EventSource::ImportSoulImportV1,
        kind: EventKind::ImportItem,
        actor_subject: ActorSubject::Owner,
        consent_id: Some(uuid7("a04")),
        privacy: Privacy {
            egress: EgressPolicy {
                e1: E1Disposition::Placeholder,
                ..EgressPolicy::default()
            },
            ..local_privacy()
        },
        body_ref: Some(sealed(event_id, "body_ref", SealedSubject::Owner, 42)),
    };
    let encoded = roundtrip(&set, SchemaId::Event, &model);
    assert_eq!(encoded["privacy"]["egress"]["e0"], json!("deny"));
    assert_eq!(
        encoded["body_ref"]["aad"],
        json!(format!("{event_id}|body_ref"))
    );
}

#[test]
fn evidence_round_trips() {
    let set = SchemaSet::load().expect("contracts compile");
    let model = SoulEvidence {
        schema_version: SchemaVersion,
        evidence_id: uuid7("a10"),
        kind: EvidenceKind::Message,
        subject: Subject::Mixed,
        source_refs: vec![json!({ "event_id": uuid7("a01") })],
        strength: SupportedBand::Moderate,
        method: Some(EvidenceMethod::Heuristic),
        exportable_to_research: Some(false),
        privacy: Some(Privacy {
            derivation: Derivation::Derived,
            purposes: vec![Purpose::SoulProfile, Purpose::Graph],
            retention: Retention::until_forgotten(),
            egress: EgressPolicy {
                research_export: ResearchDisposition::Deny,
                ..EgressPolicy::default()
            },
            subject: Subject::Mixed,
        }),
    };
    roundtrip(&set, SchemaId::Evidence, &model);
}

#[test]
fn inference_round_trips_and_requires_evidence() {
    let set = SchemaSet::load().expect("contracts compile");
    let model = SoulInference {
        schema_version: SchemaVersion,
        inference_id: uuid7("a20"),
        target: json!({ "kind": "trait_axis", "axis_id": uuid7("a30") }),
        statement_key: "voice.directness.leans_high".into(),
        evidence_ids: vec![uuid7("a10")],
        evidence_band: SupportedBand::Moderate,
        method: Some(InferenceMethod::Rule),
        user_verdict: Some(UserVerdict::Unreviewed),
        clinical_claim: NotAClinicalClaim,
        falsifier: Some("三条以上使用缓和语气的自述消息即推翻本推断".into()),
    };
    let mut encoded = roundtrip(&set, SchemaId::Inference, &model);

    encoded["evidence_ids"] = json!([]);
    assert!(
        set.validate(SchemaId::Inference, &encoded).is_err(),
        "an inference with no evidence must never validate",
    );
}

#[test]
fn profile_round_trips() {
    let set = SchemaSet::load().expect("contracts compile");
    let model = SoulProfile {
        schema_version: SchemaVersion,
        profile_id: uuid7("a40"),
        voice: json!({ "register": "plain", "source": "user_stated" }),
        trait_axes: vec![
            TraitAxis {
                axis_id: uuid7("a30"),
                label: Some("直接 ↔ 委婉".into()),
                position: AxisPosition::LeansHigh,
                evidence_band: EvidenceBand::Moderate,
                evidence_ids: Some(vec![uuid7("a10")]),
                locked_by_user: Some(true),
                clinical_claim: NotAClinicalClaim,
            },
            TraitAxis {
                axis_id: uuid7("a31"),
                label: Some("计划 ↔ 随性".into()),
                position: AxisPosition::Unknown,
                evidence_band: EvidenceBand::None,
                evidence_ids: Some(vec![]),
                locked_by_user: Some(false),
                clinical_claim: NotAClinicalClaim,
            },
        ],
        values: Some(vec![]),
        boundaries: Some(vec![]),
        clinical_claim: NotAClinicalClaim,
    };
    roundtrip(&set, SchemaId::Profile, &model);
}

#[test]
fn memory_round_trips() {
    let set = SchemaSet::load().expect("contracts compile");
    let memory_id = uuid7("a50");
    let model = SoulMemory {
        schema_version: SchemaVersion,
        memory_id,
        memory_type: MemoryType::Episodic,
        title_ref: Some(sealed(memory_id, "title_ref", SealedSubject::Owner, 9)),
        summary_ref: Some(sealed(memory_id, "summary_ref", SealedSubject::Mixed, 120)),
        source_event_ids: Some(vec![uuid7("a01")]),
        evidence_ids: Some(vec![uuid7("a10")]),
        content_key_id: uuid7("a51"),
        forget_state: ForgetState::Active,
        third_party_content_present: Some(true),
    };
    roundtrip(&set, SchemaId::Memory, &model);
}

#[test]
fn contact_round_trips_with_hashed_identifiers() {
    let set = SchemaSet::load().expect("contracts compile");
    let contact_id = uuid7("a60");
    let model = SoulContact {
        schema_version: SchemaVersion,
        contact_id,
        contact_class: ContactClass::ThirdParty,
        display_label_ref: Some(sealed(
            contact_id,
            "display_label_ref",
            SealedSubject::ThirdParty,
            2,
        )),
        identifiers: Some(vec![ContactIdentifier {
            kind: IdentifierKind::Handle,
            value_hash: digest("@wang_xiao2"),
        }]),
        forget_state: ForgetState::Active,
    };
    let encoded = roundtrip(&set, SchemaId::Contact, &model);
    let serialized = serde_json::to_string(&encoded).expect("re-encodes");
    assert!(
        !serialized.contains("wang_xiao2"),
        "a contact row must carry the digest, never the handle",
    );
}

#[test]
fn relationship_round_trips() {
    let set = SchemaSet::load().expect("contracts compile");
    let model = SoulRelationship {
        schema_version: SchemaVersion,
        relationship_id: uuid7("a70"),
        from_contact_id: uuid7("a60"),
        to_contact_id: uuid7("a63"),
        types: Some(vec![json!("colleague")]),
        tie_strength: Some(json!({ "band": "moderate" })),
        evidence_ids: vec![uuid7("a10")],
        egress_scope: Some(EgressScope::LocalOnly),
    };
    roundtrip(&set, SchemaId::Relationship, &model);
}

#[test]
fn audit_entry_round_trips_and_rejects_prose() {
    let set = SchemaSet::load().expect("contracts compile");
    let model = SoulAuditEntry {
        schema_version: SchemaVersion,
        seq: 17,
        entry_id: uuid7("a80"),
        ts: Timestamp::new("2026-08-24T11:00:00Z"),
        prev_hash: digest("genesis"),
        entry_hash: digest("entry-17"),
        action: AuditAction::ForgetExecute,
        decision: AuditDecision::Allowed,
        reason_code: Some("USER_REQUESTED".into()),
        subject_refs: Some(vec![uuid7("a50")]),
        counts: Some(AuditCounts {
            items: Some(1),
            bytes: None,
        }),
        plan_hash: None,
        capability_token_id: None,
        egress_class: Some(EgressClass::None),
    };
    let mut encoded = roundtrip(&set, SchemaId::Audit, &model);

    encoded["body"] = json!("好的没问题，明天见");
    assert!(
        set.validate(SchemaId::Audit, &encoded).is_err(),
        "the audit contract must refuse any field that could carry prose",
    );
}

#[test]
fn export_manifest_round_trips_and_stays_preview_only() {
    let set = SchemaSet::load().expect("contracts compile");
    let model = SoulExportManifest {
        schema_version: SchemaVersion,
        manifest_id: uuid7("a90"),
        export_kind: ExportKind::ResearchPreview,
        fields: Some(vec![ExportField::EventKind, ExportField::AggregateCount]),
        rows: vec![ExportRow {
            event_kind: Some("app.foreground".into()),
            aggregate_count: Some(4),
            ..ExportRow::default()
        }],
        third_party_rows: ZeroThirdPartyRows,
        written_to_disk: false,
        redaction_profile: RedactionProfile {
            third_party_body: ThirdPartyBodyExcluded,
            one_time_override_allowed: None,
        },
    };
    let mut encoded = roundtrip(&set, SchemaId::ExportManifest, &model);

    encoded["written_to_disk"] = json!(true);
    assert!(
        set.validate(SchemaId::ExportManifest, &encoded).is_err(),
        "a research preview must not be able to claim it was written to disk",
    );
}

#[test]
fn import_lines_round_trip() {
    let set = SchemaSet::load().expect("contracts compile");

    let header = ImportLine::Header(ImportHeader {
        format: ImportFormatTag,
        version: ImportVersionTag,
        exported_at: Timestamp::new("2026-08-24T08:00:00Z"),
    });
    let encoded = roundtrip(&set, SchemaId::SoulImportV1, &header);
    assert_eq!(encoded["version"], json!(1), "version is a JSON number");

    let message = ImportLine::Message(ImportMessage {
        id: "m-0001".into(),
        occurred_at: Timestamp::new("2026-08-20T09:12:00Z"),
        sender_scope: SenderScope::ThirdParty,
        conversation_id: "c-01".into(),
        sender_id: "u-lilei".into(),
        text: "好的没问题，明天见".into(),
    });
    roundtrip(&set, SchemaId::SoulImportV1, &message);
}

#[test]
fn schema_version_literal_is_pinned() {
    let wrong = json!({ "schema_version": "2.0.0" });
    assert!(
        serde_json::from_value::<SoulEventVersionProbe>(wrong).is_err(),
        "a model must refuse to deserialize a foreign schema_version",
    );
}

#[derive(Debug, serde::Deserialize)]
struct SoulEventVersionProbe {
    #[allow(dead_code)]
    schema_version: SchemaVersion,
}
