//! What the memory surface refuses to do, and why.

use uuid::Uuid;

use soul_store_api::types::StoreError;

pub type MemoryResult<T> = Result<T, MemoryError>;

#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("a memory needs a {0}")]
    Empty(&'static str),

    /// The row is a tombstone. The user can still see that the memory existed,
    /// which is why this is a distinct error rather than a `NotFound`.
    #[error("memory {0} was forgotten; its content key is gone and the text cannot be recovered")]
    Forgotten(Uuid),

    /// The key behind a memory that still claims to be active is missing. Not
    /// expected, and not something to paper over with an empty string.
    #[error("memory {memory_id} is marked active but its content key {content_key_id} is gone")]
    ContentUnreadable {
        memory_id: Uuid,
        content_key_id: Uuid,
    },

    #[error("the sealed text of memory {0} did not decode as UTF-8")]
    NotUtf8(Uuid),
}
