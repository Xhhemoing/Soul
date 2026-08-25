//! What can go wrong between a sample and a stored event.

use soul_policy::audit::{AuditContentError, AuditWriteError};
use soul_policy::consent::ConsentMissing;
use soul_store_api::types::StoreError;

use crate::source::SourceError;

pub type CollectResult<T> = Result<T, CollectError>;

#[derive(Debug, thiserror::Error)]
pub enum CollectError {
    /// Collection has not been consented to. Returned by `start`, and again by
    /// the write path, so a revocation between the two changes the answer.
    #[error(transparent)]
    Consent(#[from] ConsentMissing),

    #[error(transparent)]
    Source(#[from] SourceError),

    #[error("the store refused: {0}")]
    Store(#[from] StoreError),

    #[error("the audit entry was refused: {0}")]
    Audit(#[from] AuditContentError),

    #[error("a foreground session could not be serialized: {0}")]
    Serialize(String),

    #[error("the collector thread could not be started: {0}")]
    ThreadNotStarted(String),

    /// The worker thread panicked. The handle cannot report what it collected,
    /// but the events it wrote before the panic are in the store.
    #[error("the collector thread ended abnormally")]
    ThreadLost,
}

impl From<serde_json::Error> for CollectError {
    fn from(error: serde_json::Error) -> CollectError {
        CollectError::Serialize(error.to_string())
    }
}

impl From<AuditWriteError> for CollectError {
    fn from(error: AuditWriteError) -> CollectError {
        match error {
            AuditWriteError::Content(content) => CollectError::Audit(content),
            AuditWriteError::Store(store) => CollectError::Store(store),
        }
    }
}
