//! Model for `docs/schemas/relationship.schema.json`.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::common::SchemaVersion;

/// The graph never leaves the machine; the contract admits one value only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressScope {
    LocalOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulRelationship {
    pub schema_version: SchemaVersion,
    pub relationship_id: Uuid,
    pub from_contact_id: Uuid,
    pub to_contact_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tie_strength: Option<Value>,
    pub evidence_ids: Vec<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress_scope: Option<EgressScope>,
}
