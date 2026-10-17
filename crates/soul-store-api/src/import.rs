//! Local import bookkeeping. Arbitrary external identifiers stay sealed.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::StoreResult;
use soul_schema::common::SealedText;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportSourceKind {
    #[serde(rename = "soul-import-v1")]
    SoulImportV1,
    #[serde(rename = "telegram-desktop")]
    TelegramDesktop,
}

impl ImportSourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SoulImportV1 => "soul-import-v1",
            Self::TelegramDesktop => "telegram-desktop",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyImportChoice {
    #[default]
    Reject,
    ForwardOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportCoverage {
    FullFromCreation,
    ForwardOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportNamespaceState {
    Active,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportNamespace {
    pub namespace_id: Uuid,
    pub source: ImportSourceKind,
    pub state: ImportNamespaceState,
    pub coverage: ImportCoverage,
    pub owner_contact_id: Option<Uuid>,
    pub account_ref: Option<SealedText>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportOperationState {
    Reserved,
    Running,
    Committed,
    Rejected,
    Discarded,
    Invalidated,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportAuditState {
    NotApplicable,
    Recorded,
    Unconfirmed,
}

/// Fixed codes only; backend exceptions and parser input are not persisted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ImportOperationError {
    SourceChanged,
    IdentityConflict,
    IdentityUnavailable,
    LegacyChoiceRequired,
    NamespaceRevoked,
    StoreUnavailable,
    WorkerFailed,
    Interrupted,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportCounts {
    pub participants: u64,
    pub messages: u64,
    pub duplicates: u64,
    pub contacts_created: u64,
    pub contacts_matched: u64,
    pub events_written: u64,
    pub evidence_written: u64,
    pub messages_with_injection_markers: u64,
    pub ties_rebuilt: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportOperation {
    pub operation_id: Uuid,
    pub preview_id: Uuid,
    pub source: ImportSourceKind,
    pub namespace_id: Uuid,
    pub legacy_choice: LegacyImportChoice,
    pub state: ImportOperationState,
    pub preview: ImportCounts,
    pub receipt: Option<ImportCounts>,
    pub error_code: Option<ImportOperationError>,
    pub audit_state: ImportAuditState,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportParticipantIdentity {
    pub identity_id: Uuid,
    pub namespace_id: Uuid,
    pub contact_id: Uuid,
    pub content_key_id: Uuid,
    pub identity_ref: SealedText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportMessageIdentity {
    pub identity_id: Uuid,
    pub namespace_id: Uuid,
    pub event_id: Uuid,
    pub sender_contact_id: Uuid,
    pub content_key_id: Uuid,
    pub identity_ref: SealedText,
    pub group: bool,
    pub is_owner: bool,
}

/// Participation in the caller's existing transaction; no method commits it.
pub trait ImportIndexStore {
    fn import_namespace(&self, id: Uuid) -> StoreResult<ImportNamespace>;
    fn import_namespaces(&self) -> StoreResult<Vec<ImportNamespace>>;
    fn put_import_namespace(&mut self, namespace: &ImportNamespace) -> StoreResult<()>;
    fn import_legacy_events(&self, source: ImportSourceKind) -> StoreResult<u64>;
    fn import_key_destroyed(&self, key: Uuid) -> StoreResult<bool>;
    fn import_contact_keys(&self, contact: Uuid) -> StoreResult<Vec<Uuid>>;
    fn import_participant_batch(
        &self,
        namespace: Uuid,
        after: Option<Uuid>,
        limit: usize,
    ) -> StoreResult<Vec<ImportParticipantIdentity>>;
    fn import_message_batch(
        &self,
        namespace: Uuid,
        after: Option<Uuid>,
        limit: usize,
    ) -> StoreResult<Vec<ImportMessageIdentity>>;
    fn put_import_participant(&mut self, identity: &ImportParticipantIdentity) -> StoreResult<()>;
    fn put_import_message(&mut self, identity: &ImportMessageIdentity) -> StoreResult<()>;
}
