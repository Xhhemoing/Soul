//! Local feedback snapshots contain UUIDs, integers and flags, never prose.
use crate::import::ImportCounts;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightValidity {
    Active,
    Invalidated,
}

/// Root constructs and compares the vectors in one fixed, documented order.
/// `ignored` is user feedback; invalidation never changes it into a vote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalInsightBundle {
    pub bundle_id: Uuid,
    pub operation_id: Uuid,
    pub namespace_id: Uuid,
    pub created_at: i64,
    pub validity: InsightValidity,
    pub ignored: bool,
    pub counts: ImportCounts,
    pub dependency_uuids: Vec<Uuid>,
    pub dependency_values: Vec<i64>,
    pub dependency_flags: Vec<bool>,
}
