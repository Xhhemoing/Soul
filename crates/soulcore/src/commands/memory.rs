//! WP04's command surface: autobiographical memory, and forgetting one.
//!
//! Thin, like the rest of this module. `soul-memory` decides what a memory is
//! and `soul-store` holds it; the clock is the caller's, so a replay writes the
//! same audit entries.
//!
//! [`preview_forget`] exists as its own command because the UI must be able to
//! show the user what a forget costs without performing it. The numbers it
//! returns are the store's own query results, so the count on the confirmation
//! screen is the count that will actually be destroyed.

use uuid::Uuid;

use soul_memory::{MemoryContent, MemoryDigest, MemoryDraft, MemoryEdit, MemoryError};
use soul_schema::memory::SoulMemory;
use soul_store::SqlCipherStore;
use soul_store_api::forget::{ForgetImpact, ForgetReceipt};

/// Seal a memory's title and summary and store the row that points at them.
pub fn create(
    store: &mut SqlCipherStore,
    draft: &MemoryDraft,
    at_unix_seconds: i64,
) -> Result<SoulMemory, MemoryError> {
    soul_memory::create(store, draft, at_unix_seconds)
}

/// Open a memory's prose. Errors rather than returning blanks once the memory
/// has been forgotten.
pub fn read(store: &SqlCipherStore, memory_id: Uuid) -> Result<MemoryContent, MemoryError> {
    soul_memory::read(store, memory_id)
}

/// Every memory, prose left sealed. What a list view is allowed to show before
/// the user asks to open one.
pub fn list(store: &SqlCipherStore) -> Result<Vec<MemoryDigest>, MemoryError> {
    soul_memory::list(store)
}

/// Edit a memory in place, resealing what changed under the same content key.
pub fn update(
    store: &mut SqlCipherStore,
    memory_id: Uuid,
    edit: &MemoryEdit,
    at_unix_seconds: i64,
) -> Result<SoulMemory, MemoryError> {
    soul_memory::update(store, memory_id, edit, at_unix_seconds)
}

/// What forgetting this memory would cost. Read-only.
pub fn preview_forget(
    store: &SqlCipherStore,
    memory_id: Uuid,
) -> Result<ForgetImpact, MemoryError> {
    soul_memory::preview_forget(store, memory_id)
}

/// Destroy the content key behind this memory. Irreversible, and the audit
/// entry is written after the destruction so the chain can never block it.
pub fn forget(
    store: &mut SqlCipherStore,
    memory_id: Uuid,
    at_unix_seconds: i64,
) -> Result<ForgetReceipt, MemoryError> {
    soul_memory::forget(store, memory_id, at_unix_seconds)
}
