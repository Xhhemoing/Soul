//! N3 boundary types. Caller metadata is output-only and never request authority.
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum CallerTransport {
    Desktop,
    Http,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    ImportPrepare,
    ImportConfirm,
    ImportDiscard,
    ScopedDraft,
    ScopedPropose,
    ScopedConfirm,
    ScopedDiscard,
    MemoryEdit,
    ForgetPreview,
    ForgetConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum OperationPhase {
    Started,
    Settled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum OperationOutcome {
    Success,
    Refused,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum BuiltinFixtureId {
    Basic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeOccasion {
    Personal,
    Work,
    Study,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeRegister {
    Casual,
    Plain,
    Formal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum WriteErrorCode {
    InvalidRequest,
    CallerMismatch,
    ConfirmationUnavailable,
    StoreBusy,
    OperationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeImportState {
    Reserved,
    Running,
    Committed,
    Rejected,
    Discarded,
    Invalidated,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuntimePreferenceState {
    Absent,
    Active,
    Inactive,
    Forgotten,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAuditState {
    NotApplicable,
    Recorded,
    Unconfirmed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeCaller {
    pub transport: CallerTransport,
    pub webview_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeOperation {
    pub operation_id: String,
    pub caller: RuntimeCaller,
    pub correlation_id: Option<String>,
    pub kind: OperationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeEvent {
    pub epoch: String,
    #[ts(type = "number")]
    pub sequence: u64,
    pub operation_id: String,
    pub caller: RuntimeCaller,
    pub correlation_id: Option<String>,
    pub kind: OperationKind,
    pub phase: OperationPhase,
    pub outcome: Option<OperationOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeEventSnapshot {
    pub epoch: String,
    #[ts(type = "number")]
    pub sequence: u64,
    pub active_operations: Vec<RuntimeOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct SyntheticImportRequest {
    pub fixture_id: BuiltinFixtureId,
}

/// The operation ID is carried by the versioned route, never selected implicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct ImportConfirmRequest {
    pub preview_id: String,
}

/// Strict empty JSON for operations whose full address is in the route.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", derive(utoipa::IntoParams))]
#[cfg_attr(feature = "openapi", into_params(parameter_in = Query))]
#[serde(deny_unknown_fields)]
pub struct ScopedStatusRequest {
    pub person_id: String,
    pub occasion: RuntimeOccasion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeImportCounts {
    #[ts(type = "number")]
    pub participants: u64,
    #[ts(type = "number")]
    pub messages: u64,
    #[ts(type = "number")]
    pub duplicates: u64,
    #[ts(type = "number")]
    pub contacts_created: u64,
    #[ts(type = "number")]
    pub contacts_matched: u64,
    #[ts(type = "number")]
    pub events_written: u64,
    #[ts(type = "number")]
    pub evidence_written: u64,
    #[ts(type = "number")]
    pub messages_with_injection_markers: u64,
    #[ts(type = "number")]
    pub ties_rebuilt: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeImportOperation {
    pub operation_id: String,
    pub preview_id: String,
    pub namespace_id: String,
    pub state: RuntimeImportState,
    pub preview: RuntimeImportCounts,
    pub receipt: Option<RuntimeImportCounts>,
    pub audit_state: RuntimeAuditState,
    pub namespace_revoked: bool,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimePreparedImport {
    pub operation: RuntimeImportOperation,
    pub notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct ScopedDraftRequest {
    pub pasted: String,
    pub person_id: String,
    pub occasion: RuntimeOccasion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct ScopedProposalRequest {
    pub result_id: String,
    #[ts(type = "number")]
    pub edit_revision: u64,
    pub register: RuntimeRegister,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct ScopedConfirmRequest {
    pub proposal_id: String,
    pub expected_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeScopedReceipt {
    pub preference_id: String,
    pub revision: String,
    pub memory_id: String,
    pub register: RuntimeRegister,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeLocalDraft {
    pub text: String,
    pub not_sent_notice: String,
    pub source_notice: String,
    pub third_party_turns: usize,
    pub placeheld_turns: usize,
    pub carries_exempted_original: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeScopedDraft {
    pub result_id: String,
    pub person_id: String,
    pub occasion: RuntimeOccasion,
    pub draft: RuntimeLocalDraft,
    pub applied_preference: Option<RuntimeScopedReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeScopedProposal {
    pub proposal_id: String,
    pub result_id: String,
    #[ts(type = "number")]
    pub edit_revision: u64,
    pub person_id: String,
    pub occasion: RuntimeOccasion,
    pub register: RuntimeRegister,
    pub expected_revision: Option<String>,
    pub notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeScopedStatus {
    pub person_id: String,
    pub occasion: RuntimeOccasion,
    pub state: RuntimePreferenceState,
    pub preference: Option<RuntimeScopedReceipt>,
    pub memory_id: Option<String>,
    pub notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct MemoryEditRequest {
    pub memory_type: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeForgetConfirmation {
    pub preview_id: String,
    pub memory_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeForgetPreview {
    pub preview_id: String,
    pub memory_id: String,
    pub content_key_count: usize,
    #[ts(type = "number")]
    pub memories_affected: u64,
    #[ts(type = "number")]
    pub contacts_affected: u64,
    #[ts(type = "number")]
    pub sealed_blobs_destroyed: u64,
    #[ts(type = "number")]
    pub inferences_orphaned: u64,
    #[ts(type = "number")]
    pub import_namespaces_revoked: u64,
    #[ts(type = "number")]
    pub local_insights_invalidated: u64,
    #[ts(type = "number")]
    pub assistance_items_invalidated: u64,
    #[ts(type = "number")]
    pub audit_entries_retained: u64,
    pub destroys_anything: bool,
    pub notice: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeWalCheckpoint {
    #[ts(type = "number")]
    pub busy: i64,
    #[ts(type = "number")]
    pub log_frames: i64,
    #[ts(type = "number")]
    pub checkpointed_frames: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeForgetReceipt {
    pub memory_id: String,
    pub content_keys_destroyed: usize,
    #[ts(type = "number")]
    pub sealed_blobs_destroyed: u64,
    #[ts(type = "number")]
    pub inferences_orphaned: u64,
    #[ts(type = "number")]
    pub import_namespaces_revoked: u64,
    #[ts(type = "number")]
    pub local_insights_invalidated: u64,
    #[ts(type = "number")]
    pub assistance_items_invalidated: u64,
    pub matched_preview: bool,
    pub logical_committed: bool,
    pub cleanup: RuntimeForgetCleanup,
    pub audit: RuntimeAuditState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct RuntimeMemoryState {
    pub memory_id: String,
    pub forget_state: String,
    pub third_party_content_present: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(deny_unknown_fields)]
pub struct WriteError {
    pub code: WriteErrorCode,
    pub message: String,
    pub details: Option<super::ReadErrorDetail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RuntimeForgetCleanup {
    Complete,
    Pending {
        checkpoint: Option<RuntimeWalCheckpoint>,
    },
}

pub(crate) fn declarations() -> Vec<String> {
    vec![
        CallerTransport::decl(),
        OperationKind::decl(),
        OperationPhase::decl(),
        OperationOutcome::decl(),
        BuiltinFixtureId::decl(),
        RuntimeOccasion::decl(),
        RuntimeRegister::decl(),
        WriteErrorCode::decl(),
        RuntimeImportState::decl(),
        RuntimePreferenceState::decl(),
        RuntimeAuditState::decl(),
        RuntimeCaller::decl(),
        RuntimeOperation::decl(),
        RuntimeEvent::decl(),
        RuntimeEventSnapshot::decl(),
        SyntheticImportRequest::decl(),
        ImportConfirmRequest::decl(),
        EmptyRequest::decl(),
        ScopedStatusRequest::decl(),
        RuntimeImportCounts::decl(),
        RuntimeImportOperation::decl(),
        RuntimePreparedImport::decl(),
        ScopedDraftRequest::decl(),
        ScopedProposalRequest::decl(),
        ScopedConfirmRequest::decl(),
        RuntimeScopedReceipt::decl(),
        RuntimeLocalDraft::decl(),
        RuntimeScopedDraft::decl(),
        RuntimeScopedProposal::decl(),
        RuntimeScopedStatus::decl(),
        MemoryEditRequest::decl(),
        RuntimeForgetConfirmation::decl(),
        RuntimeForgetPreview::decl(),
        RuntimeWalCheckpoint::decl(),
        RuntimeForgetReceipt::decl(),
        RuntimeMemoryState::decl(),
        WriteError::decl(),
        RuntimeForgetCleanup::decl(),
    ]
}
