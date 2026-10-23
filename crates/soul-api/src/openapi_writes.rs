//! Offline path metadata; request and response schemas come from soul-types.
use soul_types::*;

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/imports/prepare",
    request_body = SyntheticImportRequest,
    responses((status = 200, body = RuntimePreparedImport), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn import_prepare() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/imports/{operation_id}/confirm",
    request_body = ImportConfirmRequest,
    params(("operation_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses((status = 200, body = RuntimeImportOperation), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn import_confirm() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/imports/{operation_id}/discard",
    request_body = EmptyRequest,
    params(("operation_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses((status = 200, body = RuntimeImportOperation), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn import_discard() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/imports/{operation_id}",
    params(("operation_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses((status = 200, body = RuntimeImportOperation), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn import_state() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/scoped/draft",
    request_body = ScopedDraftRequest,
    responses((status = 200, body = RuntimeScopedDraft), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn scoped_draft() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/scoped/propose",
    request_body = ScopedProposalRequest,
    responses((status = 200, body = RuntimeScopedProposal), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn scoped_propose() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/scoped/confirm",
    request_body = ScopedConfirmRequest,
    responses((status = 200, body = RuntimeScopedReceipt), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn scoped_confirm() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/scoped/{proposal_id}/discard",
    request_body = EmptyRequest,
    params(("proposal_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses((status = 200, body = bool), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn scoped_discard() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/scoped/status",
    params(ScopedStatusRequest),
    responses((status = 200, body = RuntimeScopedStatus), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn scoped_status() {}

#[allow(dead_code)]
#[utoipa::path(patch, path = "/v1/memories/{memory_id}",
    request_body = MemoryEditRequest,
    params(("memory_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses((status = 200, body = MemoryDetailView), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn memory_edit() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/memories/{memory_id}/forget-preview",
    request_body = EmptyRequest,
    params(("memory_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses((status = 200, body = RuntimeForgetPreview), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn forget_preview() {}

#[allow(dead_code)]
#[utoipa::path(post, path = "/v1/memories/{memory_id}/forget-confirm",
    request_body = RuntimeForgetConfirmation,
    params(("memory_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses((status = 200, body = RuntimeForgetReceipt), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn forget_confirm() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/memories/{memory_id}/state",
    params(("memory_id" = String, Path, description = "Exact canonical hyphenated UUID")),
    responses((status = 200, body = RuntimeMemoryState), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn memory_state() {}

#[allow(dead_code)]
#[utoipa::path(get, path = "/v1/events/snapshot",
    responses((status = 200, body = RuntimeEventSnapshot), (status = "default", description = "Safe refusal; JSON body has finite receive and size limits", body = WriteError))
)]
pub(crate) fn event_snapshot() {}
