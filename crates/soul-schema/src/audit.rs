//! Model for `docs/schemas/audit.schema.json`.
//!
//! The audit chain records that something happened, never what was said. The
//! schema is `additionalProperties: false` precisely so a stray `body` field
//! fails validation instead of leaking prose.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{SchemaVersion, Sha256Hex, Timestamp};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditAction {
    #[serde(rename = "consent.grant")]
    ConsentGrant,
    #[serde(rename = "collect.start")]
    CollectStart,
    #[serde(rename = "collect.stop")]
    CollectStop,
    #[serde(rename = "import.commit")]
    ImportCommit,
    #[serde(rename = "inference.write")]
    InferenceWrite,
    #[serde(rename = "profile.correct")]
    ProfileCorrect,
    #[serde(rename = "memory.write")]
    MemoryWrite,
    #[serde(rename = "forget.execute")]
    ForgetExecute,
    #[serde(rename = "draft.create")]
    DraftCreate,
    #[serde(rename = "egress.request")]
    EgressRequest,
    #[serde(rename = "file.plan")]
    FilePlan,
    #[serde(rename = "hitl.deny")]
    HitlDeny,
    #[serde(rename = "capability.reject")]
    CapabilityReject,
    #[serde(rename = "injection.blocked")]
    InjectionBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditDecision {
    #[serde(rename = "allowed")]
    Allowed,
    #[serde(rename = "denied")]
    Denied,
    #[serde(rename = "deferred")]
    Deferred,
    #[serde(rename = "n/a")]
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EgressClass {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "E0")]
    E0,
    #[serde(rename = "E1")]
    E1,
    #[serde(rename = "L")]
    L,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AuditCounts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulAuditEntry {
    pub schema_version: SchemaVersion,
    pub seq: u64,
    pub entry_id: Uuid,
    pub ts: Timestamp,
    pub prev_hash: Sha256Hex,
    pub entry_hash: Sha256Hex,
    pub action: AuditAction,
    pub decision: AuditDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<String>,
    /// Bare UUIDs may outlive the rows they point at; forgetting must not be
    /// blocked by the audit chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject_refs: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counts: Option<AuditCounts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_hash: Option<Sha256Hex>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_token_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_class: Option<EgressClass>,
}
