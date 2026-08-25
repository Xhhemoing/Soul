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

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_memory::{MemoryContent, MemoryDigest, MemoryDraft, MemoryEdit, MemoryError};
use soul_schema::memory::{MemoryType, SoulMemory};
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

// ------------------------------------------------- what a screen may draw ---

/// The kinds of memory a screen may offer, as the contract spells them.
///
/// Read off the enum rather than written out again, so a kind added to
/// `memory.schema.json` appears in the interface instead of being silently
/// unreachable.
pub const MEMORY_TYPES: [MemoryType; 5] = [
    MemoryType::Episodic,
    MemoryType::Semantic,
    MemoryType::Procedural,
    MemoryType::Preference,
    MemoryType::Commitment,
];

/// What forgetting means here, in the core's own words.
///
/// It says destruction rather than deletion because that is what happens: the
/// content key goes and the row stays as a tombstone. Held on this side so the
/// screen cannot soften it.
///
/// The middle sentence used to read 也不写任何文件, which was false of the one
/// storage a forget cannot avoid: [`ForgetOps::execute_forget`] opens a
/// transaction, deletes the content-key and sealed-blob rows, marks tombstones
/// and orphans the inferences that cited them, and commits — and the service
/// appends an audit record after it. Soul's own database and its journal are
/// written every time. What the sentence can honestly promise is the scope:
/// nothing outside Soul's data directory is touched.
///
/// The last sentence is the second half of D15. PRODUCT_LOCK refuses to
/// promise an SSD physical erase *and* requires the UI to write that honestly,
/// so the limit travels with the notice instead of waiting for a screen to
/// remember it: without it, "forgotten" reads as "the bits are gone". The two
/// limits are different and both stay — one is which files change, the other
/// is what stays behind in the blocks that already held the ciphertext.
///
/// [`ForgetOps::execute_forget`]: soul_store_api::forget::ForgetOps::execute_forget
pub const FORGET_NOTICE: &str =
    "遗忘销毁的是这条记忆的内容密钥：正文从此打不开，行会留成一块墓碑，\
    引用过它的推断会被标成失去依据。这一步不可撤销。它不动 Soul 数据目录以外的\
    任何文件，但 Soul 自己的加密库要写：密钥行和密文行被删掉，墓碑、失据标记和\
    一条审计记录被写进去，数据库文件和它的日志都会跟着变。\
    这不是把磁盘块擦干净：SSD 上可能还留着旧密文，只是没有密钥再也打不开。";

/// One memory in a list, with the prose left sealed.
///
/// The two character counts come off the sealed pointers. They are metadata,
/// not text — enough for a list to say which entry is the long one, and not
/// enough to say anything about what it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryRow {
    pub memory_id: String,
    pub memory_type: String,
    /// `active`, `pending_forget` or `forgotten`.
    pub forget_state: String,
    pub third_party_content_present: bool,
    pub title_chars: u64,
    pub summary_chars: u64,
}

impl MemoryRow {
    fn of(digest: &MemoryDigest) -> MemoryRow {
        MemoryRow {
            memory_id: digest.memory_id.to_string(),
            memory_type: word(&digest.memory_type),
            forget_state: word(&digest.forget_state),
            third_party_content_present: digest.third_party_content_present,
            title_chars: digest.title_chars,
            summary_chars: digest.summary_chars,
        }
    }
}

/// The list, plus the vocabulary a screen needs to draw a form for it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryList {
    pub memories: Vec<MemoryRow>,
    pub memory_types: Vec<String>,
    pub forget_notice: String,
}

/// One memory with its prose opened, because the user asked for this one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryDetail {
    pub memory_id: String,
    pub memory_type: String,
    pub title: String,
    pub summary: String,
    pub third_party_content_present: bool,
    pub content_key_id: String,
}

impl MemoryDetail {
    fn of(content: &MemoryContent) -> MemoryDetail {
        MemoryDetail {
            memory_id: content.memory.memory_id.to_string(),
            memory_type: word(&content.memory.memory_type),
            title: content.title.clone(),
            summary: content.summary.clone(),
            third_party_content_present: content
                .memory
                .third_party_content_present
                .unwrap_or(false),
            content_key_id: content.memory.content_key_id.to_string(),
        }
    }
}

/// A memory the user is about to store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewMemory {
    pub memory_type: String,
    pub title: String,
    pub summary: String,
}

/// A change to one. An absent field is left as it is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryChange {
    pub memory_type: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
}

/// What forgetting this memory would cost, and the identity of the answer.
///
/// `preview_id` is what makes the preview more than a screen. Forgetting is
/// irreversible, so the session refuses a forget that does not echo the
/// preview the user was actually shown — the same shape the endpoint drafting
/// path uses, for the same reason. WP04 left this open on purpose and named
/// it: the numbers can change between the two calls, and nothing was stopping
/// a second click from destroying more than what was on screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgetPreview {
    pub preview_id: String,
    pub memory_id: String,
    pub content_key_count: usize,
    pub memories_affected: u64,
    pub contacts_affected: u64,
    pub sealed_blobs_destroyed: u64,
    pub inferences_orphaned: u64,
    /// Kept deliberately: the chain records that a forget happened and must
    /// never be in a position to prevent one.
    pub audit_entries_retained: u64,
    /// Always false. Reading what a forget would cost destroys nothing.
    pub destroys_anything: bool,
    pub notice: String,
}

impl ForgetPreview {
    pub(crate) fn of(preview_id: Uuid, memory_id: Uuid, impact: &ForgetImpact) -> ForgetPreview {
        ForgetPreview {
            preview_id: preview_id.to_string(),
            memory_id: memory_id.to_string(),
            content_key_count: impact.content_key_ids.len(),
            memories_affected: impact.memories_affected,
            contacts_affected: impact.contacts_affected,
            sealed_blobs_destroyed: impact.sealed_blobs_destroyed,
            inferences_orphaned: impact.inferences_orphaned,
            audit_entries_retained: impact.audit_entries_retained,
            destroys_anything: false,
            notice: FORGET_NOTICE.to_owned(),
        }
    }
}

/// What the user echoes back to say they read the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgetConfirmation {
    pub preview_id: String,
    pub memory_id: String,
}

/// Proof of what a completed forget did, against what was quoted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgetReceiptView {
    pub memory_id: String,
    pub content_keys_destroyed: usize,
    pub sealed_blobs_destroyed: u64,
    pub inferences_orphaned: u64,
    /// Whether the receipt charges what the preview quoted.
    pub matched_preview: bool,
}

impl ForgetReceiptView {
    pub(crate) fn of(
        memory_id: Uuid,
        receipt: &ForgetReceipt,
        quoted: &ForgetImpact,
    ) -> ForgetReceiptView {
        ForgetReceiptView {
            memory_id: memory_id.to_string(),
            content_keys_destroyed: receipt.impact.content_key_ids.len(),
            sealed_blobs_destroyed: receipt.impact.sealed_blobs_destroyed,
            inferences_orphaned: receipt.impact.inferences_orphaned,
            matched_preview: &receipt.impact == quoted,
        }
    }
}

/// Every memory, as a list view may show them.
pub fn rows(store: &SqlCipherStore) -> Result<MemoryList, MemoryError> {
    Ok(MemoryList {
        memories: list(store)?.iter().map(MemoryRow::of).collect(),
        memory_types: MEMORY_TYPES.iter().map(word).collect(),
        forget_notice: FORGET_NOTICE.to_owned(),
    })
}

/// One memory, opened.
pub fn detail(store: &SqlCipherStore, memory_id: Uuid) -> Result<MemoryDetail, MemoryError> {
    Ok(MemoryDetail::of(&read(store, memory_id)?))
}

/// Store what the user typed. The subject is the owner: this screen writes the
/// user's own memories, and there is no field on [`NewMemory`] that could say
/// otherwise.
pub fn write_new(
    store: &mut SqlCipherStore,
    new: &NewMemory,
    at_unix_seconds: i64,
) -> Result<MemoryDetail, MemoryError> {
    let memory_type = memory_type_named(&new.memory_type).ok_or(MemoryError::Empty("type"))?;
    let memory = create(
        store,
        &MemoryDraft::own(memory_type, &new.title, &new.summary),
        at_unix_seconds,
    )?;
    detail(store, memory.memory_id)
}

/// Edit one in place, resealing what changed under the same key.
pub fn write_change(
    store: &mut SqlCipherStore,
    memory_id: Uuid,
    change: &MemoryChange,
    at_unix_seconds: i64,
) -> Result<MemoryDetail, MemoryError> {
    let mut edit = MemoryEdit {
        title: change.title.clone(),
        summary: change.summary.clone(),
        memory_type: None,
    };
    if let Some(named) = change.memory_type.as_deref() {
        edit.memory_type = Some(memory_type_named(named).ok_or(MemoryError::Empty("type"))?);
    }
    update(store, memory_id, &edit, at_unix_seconds)?;
    detail(store, memory_id)
}

/// The memory type this word names, if the contract has one.
pub fn memory_type_named(named: &str) -> Option<MemoryType> {
    MEMORY_TYPES
        .into_iter()
        .find(|candidate| word(candidate) == named)
}

/// The contract's own spelling of a small enum, taken from serde rather than
/// written out a second time.
fn word<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}
