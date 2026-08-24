//! Model for `docs/schemas/inference.schema.json`.
//!
//! `evidence_ids` is non-empty by contract: an inference without resolvable
//! evidence must never reach the store.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::common::{NotAClinicalClaim, SchemaVersion, SupportedBand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceMethod {
    UserStated,
    Rule,
    Statistical,
    Llm,
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserVerdict {
    Unreviewed,
    Accepted,
    Corrected,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulInference {
    pub schema_version: SchemaVersion,
    pub inference_id: Uuid,
    pub target: Value,
    pub statement_key: String,
    pub evidence_ids: Vec<Uuid>,
    pub evidence_band: SupportedBand,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<InferenceMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_verdict: Option<UserVerdict>,
    pub clinical_claim: NotAClinicalClaim,
    /// What observation would overturn this inference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub falsifier: Option<String>,
}
