//! Shared request, filter and error types for the storage boundary.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_schema::common::SealedSubject;
use soul_schema::event::{EventKind, EventSource};

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StoreError {
    #[error("no {kind} with id {id}")]
    NotFound { kind: &'static str, id: Uuid },

    #[error("{kind} {id} already exists")]
    AlreadyExists { kind: &'static str, id: Uuid },

    /// The plaintext is gone for good. This is what forgetting means.
    #[error("content key {0} was destroyed; the sealed text cannot be recovered")]
    ContentKeyDestroyed(Uuid),

    #[error("sealed blob {0} is missing")]
    BlobMissing(Uuid),

    /// The AEAD tag did not verify, which means the blob was moved between rows
    /// or fields, or the ciphertext was altered.
    #[error("sealed blob {0} failed authentication")]
    SealBroken(Uuid),

    /// A write that the frozen contracts forbid, caught before it lands.
    #[error("{0}")]
    ContractViolation(String),

    #[error("storage backend failed: {0}")]
    Backend(String),
}

impl StoreError {
    pub fn not_found(kind: &'static str, id: Uuid) -> Self {
        StoreError::NotFound { kind, id }
    }
}

/// Filter for [`crate::EventStore::list_events`]. All fields are `AND`-ed;
/// `Default` matches everything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventFilter {
    pub source: Option<EventSource>,
    pub kind: Option<EventKind>,
    /// Inclusive lower bound on `ts`, compared as an RFC 3339 string.
    pub since: Option<String>,
    pub limit: Option<usize>,
}

impl EventFilter {
    pub fn all() -> Self {
        EventFilter::default()
    }

    pub fn with_kind(kind: EventKind) -> Self {
        EventFilter {
            kind: Some(kind),
            ..EventFilter::default()
        }
    }

    pub fn with_source(source: EventSource) -> Self {
        EventFilter {
            source: Some(source),
            ..EventFilter::default()
        }
    }
}

/// A request to seal one field of one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealRequest {
    /// The forget unit this text belongs to. Destroying it destroys this text.
    pub content_key_id: Uuid,
    /// Identity of the row that owns the field.
    pub row_id: Uuid,
    /// Column name, bound into the AEAD tag alongside `row_id`.
    pub field: String,
    pub subject: SealedSubject,
    pub plaintext: Vec<u8>,
    /// What the UI and any E1 request body show instead of the plaintext.
    pub placeholder: Option<String>,
}

impl SealRequest {
    pub fn new(
        content_key_id: Uuid,
        row_id: Uuid,
        field: impl Into<String>,
        subject: SealedSubject,
        plaintext: impl Into<Vec<u8>>,
    ) -> Self {
        SealRequest {
            content_key_id,
            row_id,
            field: field.into(),
            subject,
            plaintext: plaintext.into(),
            placeholder: None,
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }
}

/// Whether an inference still rests on readable evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceState {
    Live,
    /// Its evidence was forgotten. The row stays so the user can see that a
    /// conclusion once existed, but it must not be treated as supported.
    Orphaned,
}
