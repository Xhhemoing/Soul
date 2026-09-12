//! Model for `docs/schemas/evidence.schema.json`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::common::{Privacy, SchemaVersion, Subject, SupportedBand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Message,
    AppUsage,
    Questionnaire,
    UserStatement,
    UserCorrection,
    Aggregate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceMethod {
    UserStated,
    Manual,
    Heuristic,
    Statistical,
    Llm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulEvidence {
    pub schema_version: SchemaVersion,
    pub evidence_id: Uuid,
    pub kind: EvidenceKind,
    pub subject: Subject,
    /// Opaque back-pointers; shape is owned by whichever module produced them.
    pub source_refs: Vec<Value>,
    pub strength: SupportedBand,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<EvidenceMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exportable_to_research: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy: Option<Privacy>,
}
