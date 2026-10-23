//! Strict transport decoding; all business and caller-slot semantics live in Runtime.
use axum::extract::Request;
use axum::http::{header, Method, StatusCode};
use axum::response::Response;
use serde::de::DeserializeOwned;
use soul_runtime::{Caller, Runtime, RuntimeContext};
use soul_types::{
    EmptyRequest, ImportConfirmRequest, MemoryEditRequest, RuntimeForgetConfirmation,
    ScopedConfirmRequest, ScopedDraftRequest, ScopedProposalRequest, SyntheticImportRequest,
    WriteError, WriteErrorCode,
};

use crate::transport::{json, unique_header};

pub(crate) enum WriteOperation {
    ImportPrepare(SyntheticImportRequest),
    ImportConfirm(String, ImportConfirmRequest),
    ImportDiscard(String),
    ScopedDraft(ScopedDraftRequest),
    ScopedPropose(ScopedProposalRequest),
    ScopedConfirm(ScopedConfirmRequest),
    ScopedDiscard(String),
    MemoryEdit(String, MemoryEditRequest),
    ForgetPreview(String),
    ForgetConfirm(RuntimeForgetConfirmation),
}

enum Route {
    ImportPrepare,
    ImportConfirm(String),
    ImportDiscard(String),
    ScopedDraft,
    ScopedPropose,
    ScopedConfirm,
    ScopedDiscard(String),
    MemoryEdit(String),
    ForgetPreview(String),
    ForgetConfirm(String),
}

pub(crate) fn exact_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn resolve(method: &Method, path: &str) -> Result<Route, Response> {
    let parts: Vec<_> = path.split('/').collect();
    let (expected, route) = match parts.as_slice() {
        ["", "v1", "imports", "prepare"] => (Method::POST, Route::ImportPrepare),
        ["", "v1", "imports", id, "confirm"] if exact_id(id) => {
            (Method::POST, Route::ImportConfirm((*id).into()))
        }
        ["", "v1", "imports", id, "discard"] if exact_id(id) => {
            (Method::POST, Route::ImportDiscard((*id).into()))
        }
        ["", "v1", "scoped", "draft"] => (Method::POST, Route::ScopedDraft),
        ["", "v1", "scoped", "propose"] => (Method::POST, Route::ScopedPropose),
        ["", "v1", "scoped", "confirm"] => (Method::POST, Route::ScopedConfirm),
        ["", "v1", "scoped", id, "discard"] if exact_id(id) => {
            (Method::POST, Route::ScopedDiscard((*id).into()))
        }
        ["", "v1", "memories", id] if exact_id(id) => {
            (Method::PATCH, Route::MemoryEdit((*id).into()))
        }
        ["", "v1", "memories", id, "forget-preview"] if exact_id(id) => {
            (Method::POST, Route::ForgetPreview((*id).into()))
        }
        ["", "v1", "memories", id, "forget-confirm"] if exact_id(id) => {
            (Method::POST, Route::ForgetConfirm((*id).into()))
        }
        ["", "v1", "status" | "people"] => return Err(refuse(StatusCode::METHOD_NOT_ALLOWED)),
        _ => return Err(refuse(StatusCode::NOT_FOUND)),
    };
    if method != expected {
        return Err(refuse(StatusCode::METHOD_NOT_ALLOWED));
    }
    Ok(route)
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Response> {
    serde_json::from_slice(bytes).map_err(|_| refuse(StatusCode::BAD_REQUEST))
}

pub(crate) async fn parse(request: Request) -> Result<WriteOperation, Response> {
    if request.uri().query().is_some() {
        return Err(refuse(StatusCode::BAD_REQUEST));
    }
    let route = resolve(request.method(), request.uri().path())?;
    let content_type = unique_header(request.headers(), "content-type")
        .ok_or_else(|| refuse(StatusCode::UNSUPPORTED_MEDIA_TYPE))?;
    if !matches!(
        content_type.to_ascii_lowercase().as_str(),
        "application/json" | "application/json; charset=utf-8"
    ) {
        return Err(refuse(StatusCode::UNSUPPORTED_MEDIA_TYPE));
    }
    if request.headers().contains_key(header::TRANSFER_ENCODING) {
        return Err(refuse(StatusCode::BAD_REQUEST));
    }
    let length = unique_header(request.headers(), "content-length")
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| refuse(StatusCode::LENGTH_REQUIRED))?;
    if length > crate::MAX_REQUEST_BYTES as u64 {
        return Err(refuse(StatusCode::PAYLOAD_TOO_LARGE));
    }
    // The receive deadline is before admission: cancelling an incomplete body
    // cannot abandon Session work or consume any confirmation slot.
    let bytes = match tokio::time::timeout(
        crate::BODY_TIMEOUT,
        axum::body::to_bytes(request.into_body(), crate::MAX_REQUEST_BYTES),
    )
    .await
    {
        Err(_) => return Err(refuse(StatusCode::REQUEST_TIMEOUT)),
        Ok(Err(_)) => return Err(refuse(StatusCode::BAD_REQUEST)),
        Ok(Ok(bytes)) => bytes,
    };
    Ok(match route {
        Route::ImportPrepare => WriteOperation::ImportPrepare(decode(&bytes)?),
        Route::ImportConfirm(id) => WriteOperation::ImportConfirm(id, decode(&bytes)?),
        Route::ImportDiscard(id) => {
            let _: EmptyRequest = decode(&bytes)?;
            WriteOperation::ImportDiscard(id)
        }
        Route::ScopedDraft => WriteOperation::ScopedDraft(decode(&bytes)?),
        Route::ScopedPropose => WriteOperation::ScopedPropose(decode(&bytes)?),
        Route::ScopedConfirm => WriteOperation::ScopedConfirm(decode(&bytes)?),
        Route::ScopedDiscard(id) => {
            let _: EmptyRequest = decode(&bytes)?;
            WriteOperation::ScopedDiscard(id)
        }
        Route::MemoryEdit(id) => WriteOperation::MemoryEdit(id, decode(&bytes)?),
        Route::ForgetPreview(id) => {
            let _: EmptyRequest = decode(&bytes)?;
            WriteOperation::ForgetPreview(id)
        }
        Route::ForgetConfirm(id) => {
            let confirmation: RuntimeForgetConfirmation = decode(&bytes)?;
            if confirmation.memory_id != id {
                return Err(refuse(StatusCode::BAD_REQUEST));
            }
            WriteOperation::ForgetConfirm(confirmation)
        }
    })
}

fn outcome<T: serde::Serialize>(result: Result<T, WriteError>) -> Response {
    match result {
        Ok(value) => json(StatusCode::OK, &value),
        Err(error) => error_response(error),
    }
}

pub(crate) fn execute(runtime: &Runtime, operation: WriteOperation) -> Response {
    // The shared bearer represents exactly one trusted HTTP caller; JSON and
    // optional arbitrary headers cannot impersonate a desktop approval owner.
    let context = RuntimeContext::new(Caller::http(), None);
    match operation {
        WriteOperation::ImportPrepare(request) => {
            outcome(runtime.safe_prepare_synthetic_import(&context, request))
        }
        WriteOperation::ImportConfirm(id, request) => {
            outcome(runtime.safe_confirm_import(&context, &id, &request.preview_id))
        }
        WriteOperation::ImportDiscard(id) => outcome(runtime.safe_discard_import(&context, &id)),
        WriteOperation::ScopedDraft(request) => {
            outcome(runtime.safe_draft_scoped_reply(&context, request))
        }
        WriteOperation::ScopedPropose(request) => {
            outcome(runtime.safe_propose_scoped_register(&context, request))
        }
        WriteOperation::ScopedConfirm(request) => {
            outcome(runtime.safe_confirm_scoped_register(&context, request))
        }
        WriteOperation::ScopedDiscard(id) => {
            outcome(runtime.safe_discard_scoped_register_proposal(&context, &id))
        }
        WriteOperation::MemoryEdit(id, request) => {
            outcome(runtime.safe_edit_memory(&context, &id, request))
        }
        WriteOperation::ForgetPreview(id) => outcome(runtime.safe_preview_forget(&context, &id)),
        WriteOperation::ForgetConfirm(request) => {
            outcome(runtime.safe_forget_memory(&context, request))
        }
    }
}

pub(crate) fn error_response(error: WriteError) -> Response {
    let status = match error.code {
        WriteErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
        WriteErrorCode::CallerMismatch => StatusCode::FORBIDDEN,
        WriteErrorCode::ConfirmationUnavailable | WriteErrorCode::StoreBusy => StatusCode::CONFLICT,
        WriteErrorCode::OperationFailed => StatusCode::SERVICE_UNAVAILABLE,
    };
    json(status, &error)
}

pub(crate) fn refuse(status: StatusCode) -> Response {
    json(
        status,
        &WriteError {
            code: if status.is_server_error() {
                WriteErrorCode::OperationFailed
            } else {
                WriteErrorCode::InvalidRequest
            },
            message: "Local API request refused".into(),
            details: None,
        },
    )
}
