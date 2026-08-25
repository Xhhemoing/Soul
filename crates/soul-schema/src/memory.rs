//! Model for `docs/schemas/memory.schema.json`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{SchemaVersion, SealedText};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    Episodic,
    Semantic,
    Procedural,
    Preference,
    Commitment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgetState {
    Active,
    PendingForget,
    Forgotten,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulMemory {
    pub schema_version: SchemaVersion,
    pub memory_id: Uuid,
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_ref: Option<SealedText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_ref: Option<SealedText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_event_ids: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_ids: Option<Vec<Uuid>>,
    /// Destroying this key is what "forget" means.
    pub content_key_id: Uuid,
    pub forget_state: ForgetState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub third_party_content_present: Option<bool>,
}
