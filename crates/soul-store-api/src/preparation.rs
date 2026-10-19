//! Local template metadata. No rendered body or reusable model permission.
use crate::assistance::AssistanceSource;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalTemplateKind {
    OpeningV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalTemplateVoice {
    pub register: String,
    pub directness: String,
    pub warmth: String,
    pub emoji_use: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalPreparationPreference {
    pub item_id: Uuid,
    pub revision: Option<Uuid>,
    pub enabled: bool,
    pub owner_profile_id: Option<Uuid>,
    pub failure_code: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalTemplateMetadata {
    pub template_id: Uuid,
    pub item_id: Uuid,
    pub item_revision: Uuid,
    pub preference_revision: Uuid,
    pub source: AssistanceSource,
    pub owner_profile_id: Uuid,
    pub voice: LocalTemplateVoice,
    pub prepared_at: i64,
    pub kind: LocalTemplateKind,
}
