//! Stable local assistance identities. Prose remains in Memory's sealed fields.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistanceObservationSource {
    pub item_id: Uuid,
    pub revision: Uuid,
    pub namespace_id: Uuid,
    pub contact_id: Uuid,
    pub bundle_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AssistanceSource {
    ManualGoal {
        memory_id: Uuid,
        observation: Option<AssistanceObservationSource>,
    },
    Observation {
        namespace_id: Uuid,
        contact_id: Uuid,
        bundle_id: Uuid,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceValidity {
    Current,
    Stale,
    Forgotten,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceHandling {
    Open,
    Cancelled,
    ResolvedReported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssistanceItem {
    pub item_id: Uuid,
    /// Opaque equality token, never a clock or sortable sequence.
    pub revision: Uuid,
    pub source: AssistanceSource,
    pub validity: AssistanceValidity,
    pub handling: AssistanceHandling,
    pub deadline_at: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl AssistanceItem {
    pub fn is_open(&self) -> bool {
        self.handling == AssistanceHandling::Open
    }
    pub fn is_current(&self) -> bool {
        self.is_open() && self.validity == AssistanceValidity::Current
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceWorkState {
    None,
    Pending,
    Running,
    Ready,
    SourceUnavailable,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistanceWorkMetadata {
    pub item_id: Uuid,
    pub item_revision: Uuid,
    pub state: AssistanceWorkState,
    pub pending_revision: Option<Uuid>,
    pub claimed_revision: Option<Uuid>,
    pub claim_id: Option<Uuid>,
    pub result_revision: Option<Uuid>,
    pub failure_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistanceWorkClaim {
    pub item_id: Uuid,
    pub revision: Uuid,
    pub claim_id: Uuid,
    pub preparation_revision: Option<Uuid>,
}
