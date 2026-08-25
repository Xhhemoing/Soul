//! Create, read, update, list, and forget.
//!
//! The D of CRUD is [`forget`], and it is not a delete. `docs/DECISIONS.md` D15
//! defines forgetting as destroying the content key: the row stays as a
//! tombstone the user can see, the ciphertext stops being openable, and
//! anything inferred from the evidence the memory cited is demoted to
//! `orphaned`. All of that already lives in `soul-store`; this module reaches
//! it through [`ForgetOps`] rather than reimplementing any of it, and the
//! preview a caller shows the user is the store's own query result.
//!
//! Two things this module is careful about:
//!
//! * **One content key per memory.** It is minted at creation and reused by
//!   every edit, so a memory is exactly one forget unit. An update that minted
//!   a second key would leave the old title readable after a forget.
//! * **The audit entry comes after the forget, never before it.** PRODUCT_LOCK
//!   says the chain must not be able to block a forget, so the write that could
//!   fail is the one that happens second.

use uuid::Uuid;

use soul_policy::audit::{append_or_store_error, AuditContent, ReasonCode};
use soul_schema::audit::{AuditAction, AuditCounts, AuditDecision};
use soul_schema::common::{SchemaVersion, SealedText};
use soul_schema::memory::{ForgetState, SoulMemory};
use soul_store_api::forget::{ForgetImpact, ForgetOps, ForgetReceipt, ForgetUnit};
use soul_store_api::types::{SealRequest, StoreError};
use soul_store_api::{AuditLog, BlobStore, MemoryStore};

use crate::draft::{
    MemoryContent, MemoryDigest, MemoryDraft, MemoryEdit, SUMMARY_FIELD, TITLE_FIELD,
};
use crate::error::{MemoryError, MemoryResult};

/// Seal a memory's title and summary and store the row that points at them.
pub fn create<S>(
    store: &mut S,
    draft: &MemoryDraft,
    now_unix_seconds: i64,
) -> MemoryResult<SoulMemory>
where
    S: MemoryStore + BlobStore + AuditLog,
{
    draft.validate()?;

    let memory_id = Uuid::now_v7();
    // The forget unit. Both sealed fields go under it, so destroying it takes
    // the whole memory with it and nothing else.
    let content_key_id = Uuid::now_v7();

    let title = seal_field(
        store,
        draft,
        content_key_id,
        memory_id,
        TITLE_FIELD,
        &draft.title,
    )?;
    let summary = seal_field(
        store,
        draft,
        content_key_id,
        memory_id,
        SUMMARY_FIELD,
        &draft.summary,
    )?;

    let memory = SoulMemory {
        schema_version: SchemaVersion,
        memory_id,
        memory_type: draft.memory_type,
        title_ref: Some(title),
        summary_ref: Some(summary),
        source_event_ids: none_if_empty(&draft.source_event_ids),
        evidence_ids: none_if_empty(&draft.evidence_ids),
        content_key_id,
        forget_state: ForgetState::Active,
        third_party_content_present: Some(draft.third_party_content_present()),
    };
    store.put_memory(memory.clone())?;

    audit(
        store,
        AuditAction::MemoryWrite,
        &[memory_id],
        Some(2),
        now_unix_seconds,
    )?;
    Ok(memory)
}

/// Open a memory's prose.
pub fn read<S>(store: &S, memory_id: Uuid) -> MemoryResult<MemoryContent>
where
    S: MemoryStore + BlobStore,
{
    let memory = store.get_memory(memory_id)?;
    if memory.forget_state == ForgetState::Forgotten {
        return Err(MemoryError::Forgotten(memory_id));
    }

    let title = open_field(store, &memory, memory.title_ref.as_ref())?;
    let summary = open_field(store, &memory, memory.summary_ref.as_ref())?;
    Ok(MemoryContent {
        memory,
        title,
        summary,
    })
}

/// Every memory, prose left sealed.
pub fn list<S: MemoryStore>(store: &S) -> MemoryResult<Vec<MemoryDigest>> {
    Ok(store
        .list_memories()?
        .into_iter()
        .map(|memory| MemoryDigest {
            memory_id: memory.memory_id,
            memory_type: memory.memory_type,
            forget_state: memory.forget_state,
            third_party_content_present: memory.third_party_content_present.unwrap_or(false),
            title_chars: memory.title_ref.as_ref().map_or(0, |s| s.char_count),
            summary_chars: memory.summary_ref.as_ref().map_or(0, |s| s.char_count),
        })
        .collect())
}

/// Edit a memory in place, resealing what changed under the same key.
///
/// The superseded ciphertext stays in the blob table. It is sealed under the
/// same content key, so it dies in the same forget; `tests/forget_reopen.rs`
/// asserts that rather than leaving it as an assumption.
pub fn update<S>(
    store: &mut S,
    memory_id: Uuid,
    edit: &MemoryEdit,
    now_unix_seconds: i64,
) -> MemoryResult<SoulMemory>
where
    S: MemoryStore + BlobStore + AuditLog,
{
    edit.validate()?;
    let mut memory = store.get_memory(memory_id)?;
    if memory.forget_state == ForgetState::Forgotten {
        return Err(MemoryError::Forgotten(memory_id));
    }

    let mut resealed = 0u64;
    if let Some(memory_type) = edit.memory_type {
        memory.memory_type = memory_type;
    }
    if let Some(title) = edit.title.as_deref() {
        memory.title_ref = Some(reseal(store, &memory, TITLE_FIELD, title)?);
        resealed += 1;
    }
    if let Some(summary) = edit.summary.as_deref() {
        memory.summary_ref = Some(reseal(store, &memory, SUMMARY_FIELD, summary)?);
        resealed += 1;
    }

    store.put_memory(memory.clone())?;
    audit(
        store,
        AuditAction::MemoryWrite,
        &[memory_id],
        Some(resealed),
        now_unix_seconds,
    )?;
    Ok(memory)
}

/// What forgetting this memory would cost. Read-only, and every number in it
/// comes from the store's own query.
pub fn preview_forget<S: ForgetOps>(store: &S, memory_id: Uuid) -> MemoryResult<ForgetImpact> {
    Ok(store.preview_impact(ForgetUnit::Memory(memory_id))?)
}

/// Destroy the content key behind this memory. Irreversible.
pub fn forget<S>(
    store: &mut S,
    memory_id: Uuid,
    now_unix_seconds: i64,
) -> MemoryResult<ForgetReceipt>
where
    S: MemoryStore + ForgetOps + AuditLog,
{
    // Resolve first, so forgetting something that was never stored is a
    // NotFound rather than a receipt for nothing.
    store.get_memory(memory_id)?;

    let receipt = store.execute_forget(ForgetUnit::Memory(memory_id))?;

    // Deliberately after the destruction: the chain records that a forget
    // happened and must never be in a position to prevent one.
    audit(
        store,
        AuditAction::ForgetExecute,
        &[memory_id],
        Some(receipt.impact.content_key_ids.len() as u64),
        now_unix_seconds,
    )?;
    Ok(receipt)
}

// ------------------------------------------------------------- internals ---

fn none_if_empty(ids: &[Uuid]) -> Option<Vec<Uuid>> {
    match ids.is_empty() {
        true => None,
        false => Some(ids.to_vec()),
    }
}

fn seal_field<S: BlobStore>(
    store: &mut S,
    draft: &MemoryDraft,
    content_key_id: Uuid,
    memory_id: Uuid,
    field: &str,
    text: &str,
) -> MemoryResult<SealedText> {
    let mut request = SealRequest::new(
        content_key_id,
        memory_id,
        field,
        draft.subject,
        text.as_bytes().to_vec(),
    );
    request.placeholder = draft.placeholder();
    Ok(store.seal(request)?)
}

fn reseal<S: BlobStore>(
    store: &mut S,
    memory: &SoulMemory,
    field: &str,
    text: &str,
) -> MemoryResult<SealedText> {
    // Subject and placeholder come from what is already stored: an edit
    // changes the words, not whose words they are.
    let existing = match field {
        TITLE_FIELD => memory.title_ref.as_ref(),
        _ => memory.summary_ref.as_ref(),
    };
    let subject = existing.map_or(soul_schema::common::SealedSubject::Owner, |s| s.subject);
    let placeholder = existing.and_then(|s| s.placeholder.clone());

    let mut request = SealRequest::new(
        memory.content_key_id,
        memory.memory_id,
        field,
        subject,
        text.as_bytes().to_vec(),
    );
    request.placeholder = placeholder;
    Ok(store.seal(request)?)
}

fn open_field<S: BlobStore>(
    store: &S,
    memory: &SoulMemory,
    sealed: Option<&SealedText>,
) -> MemoryResult<String> {
    let Some(sealed) = sealed else {
        return Ok(String::new());
    };
    match store.open(sealed) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|_| MemoryError::NotUtf8(memory.memory_id)),
        Err(StoreError::ContentKeyDestroyed(content_key_id)) => {
            Err(MemoryError::ContentUnreadable {
                memory_id: memory.memory_id,
                content_key_id,
            })
        }
        Err(error) => Err(error.into()),
    }
}

/// One audit entry, carrying ids and a count and nothing else.
fn audit<S: AuditLog>(
    store: &mut S,
    action: AuditAction,
    subjects: &[Uuid],
    items: Option<u64>,
    now_unix_seconds: i64,
) -> MemoryResult<()> {
    let mut content = AuditContent::new(action, AuditDecision::Allowed)
        .because(ReasonCode::Routine)
        .about(subjects);
    if let Some(items) = items {
        content = content.counting(AuditCounts {
            items: Some(items),
            bytes: None,
        });
    }
    append_or_store_error(store, content, now_unix_seconds)?;
    Ok(())
}
