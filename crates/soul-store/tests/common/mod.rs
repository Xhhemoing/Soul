//! Shared fixtures for the WP02 acceptance tests.
//!
//! Everything here is deliberately deterministic: the crash tests re-execute
//! the test binary as a child process, and the parent has to be able to name
//! the rows the child wrote without being told.

// Each test binary uses a different subset of these builders.
#![allow(dead_code)]

use uuid::Uuid;

use soul_schema::audit::{AuditAction, AuditDecision, SoulAuditEntry};
use soul_schema::common::{
    ActorSubject, Derivation, E0Deny, E1Disposition, EgressPolicy, NotAClinicalClaim, Privacy,
    Purpose, ResearchDisposition, Retention, SchemaVersion, SealedSubject, Sha256Hex, Subject,
    SupportedBand, Timestamp,
};
use soul_schema::contact::{ContactClass, SoulContact};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_schema::evidence::{EvidenceKind, SoulEvidence};
use soul_schema::inference::SoulInference;
use soul_schema::memory::{ForgetState, MemoryType, SoulMemory};
use soul_schema::profile::{AxisPosition, SoulProfile, TraitAxis};
use soul_schema::relationship::{EgressScope, SoulRelationship};
use soul_schema::EvidenceBand;

/// A UUIDv7-shaped literal, stable across processes.
pub fn id(tail: &str) -> Uuid {
    let padded = format!("{tail:0>3}");
    format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{padded}")
        .parse()
        .expect("hand-written UUIDv7 literal")
}

pub fn privacy(subject: Subject) -> Privacy {
    Privacy {
        subject,
        derivation: Derivation::Raw,
        purposes: vec![Purpose::Memory],
        retention: Retention::until_forgotten(),
        egress: EgressPolicy::default(),
    }
}

/// The same privacy with the research disposition spelled out.
///
/// [`EgressPolicy::default`] is `research_export: deny`, which is what most of
/// the store's rows really carry. A fixture that wants a row the research
/// rollup may publish has to say so, because the rollup reads the field rather
/// than the subject alone.
pub fn privacy_for_research(subject: Subject, research_export: ResearchDisposition) -> Privacy {
    Privacy {
        egress: EgressPolicy {
            e0: E0Deny,
            e1: E1Disposition::Deny,
            research_export,
        },
        ..privacy(subject)
    }
}

/// An event whose stored disposition is `deny`, as most of them are.
pub fn event(event_id: Uuid, ts: &str, kind: EventKind, subject: Subject) -> SoulEvent {
    SoulEvent {
        schema_version: SchemaVersion,
        event_id,
        ts: Timestamp::new(ts),
        source: match kind {
            EventKind::AppForeground => EventSource::CollectorForegroundApp,
            EventKind::MessageObserved => EventSource::ImportTelegramDesktop,
            _ => EventSource::ImportSoulImportV1,
        },
        kind,
        actor_subject: match subject {
            Subject::ThirdParty => ActorSubject::ThirdParty,
            Subject::System => ActorSubject::System,
            _ => ActorSubject::Owner,
        },
        consent_id: None,
        privacy: privacy(subject),
        body_ref: None,
    }
}

/// An event marked `research_export: bucket`, the way the collector writes
/// one. The only shape the research rollup publishes.
pub fn bucketed_event(event_id: Uuid, ts: &str, kind: EventKind, subject: Subject) -> SoulEvent {
    SoulEvent {
        privacy: privacy_for_research(subject, ResearchDisposition::Bucket),
        ..event(event_id, ts, kind, subject)
    }
}

pub fn evidence(evidence_id: Uuid, subject: Subject) -> SoulEvidence {
    SoulEvidence {
        schema_version: SchemaVersion,
        evidence_id,
        kind: EvidenceKind::UserStatement,
        subject,
        source_refs: vec![serde_json::json!({ "origin": "wp02-fixture" })],
        strength: SupportedBand::Moderate,
        method: None,
        exportable_to_research: Some(false),
        privacy: Some(privacy(subject)),
    }
}

pub fn inference(inference_id: Uuid, evidence_ids: &[Uuid]) -> SoulInference {
    SoulInference {
        schema_version: SchemaVersion,
        inference_id,
        target: serde_json::json!({ "kind": "trait_axis", "axis_id": id("30") }),
        statement_key: "voice.directness.leans_high".into(),
        evidence_ids: evidence_ids.to_vec(),
        evidence_band: SupportedBand::Weak,
        method: None,
        user_verdict: None,
        clinical_claim: NotAClinicalClaim,
        falsifier: None,
    }
}

pub fn contact(contact_id: Uuid, class: ContactClass) -> SoulContact {
    SoulContact {
        schema_version: SchemaVersion,
        contact_id,
        contact_class: class,
        display_label_ref: None,
        identifiers: None,
        forget_state: ForgetState::Active,
    }
}

pub fn relationship(
    relationship_id: Uuid,
    from: Uuid,
    to: Uuid,
    evidence_ids: &[Uuid],
) -> SoulRelationship {
    SoulRelationship {
        schema_version: SchemaVersion,
        relationship_id,
        from_contact_id: from,
        to_contact_id: to,
        types: None,
        tie_strength: None,
        evidence_ids: evidence_ids.to_vec(),
        egress_scope: Some(EgressScope::LocalOnly),
    }
}

pub fn memory(
    memory_id: Uuid,
    content_key_id: Uuid,
    title: Option<soul_schema::common::SealedText>,
    summary: Option<soul_schema::common::SealedText>,
    evidence_ids: &[Uuid],
) -> SoulMemory {
    SoulMemory {
        schema_version: SchemaVersion,
        memory_id,
        memory_type: MemoryType::Episodic,
        title_ref: title,
        summary_ref: summary,
        source_event_ids: None,
        evidence_ids: Some(evidence_ids.to_vec()),
        content_key_id,
        forget_state: ForgetState::Active,
        third_party_content_present: Some(false),
    }
}

/// An audit entry with the chain fields left at placeholder values. The store
/// overwrites `seq`, `prev_hash` and `entry_hash` on the way in, which is what
/// makes the chain worth verifying.
pub fn audit_entry(entry_id: Uuid, action: AuditAction, subjects: &[Uuid]) -> SoulAuditEntry {
    SoulAuditEntry {
        schema_version: SchemaVersion,
        seq: 0,
        entry_id,
        ts: Timestamp::new("2026-08-24T11:00:00Z"),
        prev_hash: Sha256Hex::new("0".repeat(64)),
        entry_hash: Sha256Hex::new("0".repeat(64)),
        action,
        decision: AuditDecision::Allowed,
        reason_code: None,
        subject_refs: Some(subjects.to_vec()),
        counts: None,
        plan_hash: None,
        capability_token_id: None,
        egress_class: None,
    }
}

pub fn profile(profile_id: Uuid, axes: &[(Uuid, EvidenceBand)]) -> SoulProfile {
    SoulProfile {
        schema_version: SchemaVersion,
        profile_id,
        voice: serde_json::json!({ "register": "plain" }),
        trait_axes: axes
            .iter()
            .map(|(axis_id, band)| TraitAxis {
                axis_id: *axis_id,
                label: Some("直接程度".into()),
                position: AxisPosition::LeansHigh,
                evidence_band: *band,
                evidence_ids: None,
                locked_by_user: None,
                clinical_claim: NotAClinicalClaim,
            })
            .collect(),
        values: None,
        boundaries: None,
        clinical_claim: NotAClinicalClaim,
    }
}

pub fn owner_seal(
    content_key_id: Uuid,
    row_id: Uuid,
    field: &str,
    text: &str,
) -> soul_store_api::types::SealRequest {
    soul_store_api::types::SealRequest::new(
        content_key_id,
        row_id,
        field,
        SealedSubject::Owner,
        text.as_bytes().to_vec(),
    )
}

pub fn third_party_seal(
    content_key_id: Uuid,
    row_id: Uuid,
    field: &str,
    text: &str,
) -> soul_store_api::types::SealRequest {
    soul_store_api::types::SealRequest::new(
        content_key_id,
        row_id,
        field,
        SealedSubject::ThirdParty,
        text.as_bytes().to_vec(),
    )
    .with_placeholder("[第三人内容已占位]")
}
