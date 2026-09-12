//! Model for `docs/schemas/profile.schema.json`.
//!
//! Trait axes carry a direction and an evidence band, never a numeric rating.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::common::{EvidenceBand, NotAClinicalClaim, SchemaVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AxisPosition {
    LeansLow,
    Mixed,
    LeansHigh,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitAxis {
    pub axis_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub position: AxisPosition,
    pub evidence_band: EvidenceBand,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_ids: Option<Vec<Uuid>>,
    /// A user correction pins the axis; later inference must not overwrite it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_by_user: Option<bool>,
    pub clinical_claim: NotAClinicalClaim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulProfile {
    pub schema_version: SchemaVersion,
    pub profile_id: Uuid,
    pub voice: Value,
    pub trait_axes: Vec<TraitAxis>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boundaries: Option<Vec<Value>>,
    pub clinical_claim: NotAClinicalClaim,
}
