//! DATA N1 storage experiment, shared with N2 only by the non-default test feature.
//! No general recovery or ABA claim. No normal product dependency enables this.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use soul_schema::common::field_aad;
#[cfg(test)]
use soul_schema::common::{SchemaVersion, SealedSubject};
#[cfg(test)]
use soul_schema::memory::MemoryType;
use soul_schema::memory::{ForgetState, SoulMemory};
use soul_schema::SchemaId;
#[cfg(test)]
use soul_store_api::types::SealRequest;
use soul_store_api::types::{StoreError, StoreResult};
#[cfg(test)]
use soul_store_api::AuditLog;
use soul_store_api::{BlobStore, MemoryStore};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::store::{backend, enum_text};
use crate::SqlCipherStore;
#[cfg(test)]
use crate::TestKeyProvider;

// This is the existing schema property, not a new field or production constant.
// No dependency on soul-memory: this experiment does not test its read adapter
// or its SelectionLimits default. `budget` is an explicit synthetic byte budget.
const FIELD: &str = "summary_ref";

#[cfg(test)]
mod tests;

// No Serialize/Clone: held bytes are private, single-use and never logged.
#[doc(hidden)]
pub struct Held {
    memory: SoulMemory,
    blob: Blob,
    key: WrappedKey,
    destroyed: bool,
}

impl std::fmt::Debug for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Held(<redacted>)")
    }
}

#[derive(PartialEq, Eq)]
struct Blob {
    row_id: String,
    field: String,
    key_id: String,
    subject: String,
    nonce: Vec<u8>,
    ciphertext: Zeroizing<Vec<u8>>,
}

#[derive(PartialEq, Eq)]
struct WrappedKey {
    wrapped: Zeroizing<Vec<u8>>,
    nonce: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Point {
    Locked,
    Validated,
    Updated,
    BeforeCommit,
}

fn reject() -> StoreError {
    StoreError::ContractViolation("N1_REJECTED".into())
}

fn blob(store: &SqlCipherStore, id: Uuid) -> StoreResult<Blob> {
    store.conn.query_row(
        "SELECT row_id,field,content_key_id,subject,nonce,ciphertext FROM sealed_blobs WHERE blob_id=?1",
        [id.to_string()],
        |r| Ok(Blob {
            row_id: r.get(0)?, field: r.get(1)?, key_id: r.get(2)?,
            subject: r.get(3)?, nonce: r.get(4)?,
            ciphertext: Zeroizing::new(r.get(5)?),
        }),
    ).map_err(backend)
}

fn wrapped_key(store: &SqlCipherStore, id: Uuid) -> StoreResult<WrappedKey> {
    store
        .conn
        .query_row(
            "SELECT wrapped_key,wrap_nonce FROM content_keys WHERE content_key_id=?1",
            [id.to_string()],
            |r| {
                Ok(WrappedKey {
                    wrapped: Zeroizing::new(r.get(0)?),
                    nonce: r.get(1)?,
                })
            },
        )
        .map_err(backend)
}

fn text(bytes: &[u8], chars: u64, budget: usize) -> StoreResult<()> {
    if bytes.len() > budget {
        return Err(reject());
    }
    let value = std::str::from_utf8(bytes).map_err(|_| reject())?;
    if value.trim().is_empty() || value.chars().count() as u64 != chars {
        return Err(reject());
    }
    Ok(())
}

// Memory/association reads propagate errors instead of substituting emptiness.
// The existing scalar preflight can collapse a key-query failure into
// LOCAL_SOURCE_UNAVAILABLE. Successful admission still requires the fallible
// destroyed/content_key/wrapped_key checks below; original Backend error
// classification is not promised for every rejection.
fn state(store: &SqlCipherStore, id: Uuid, budget: usize) -> StoreResult<Held> {
    let memory = store.get_memory(id)?;
    store.check(SchemaId::Memory, &memory)?;
    if memory.forget_state != ForgetState::Active
        || memory
            .source_event_ids
            .as_ref()
            .is_some_and(|v| !v.is_empty())
        || memory.evidence_ids.as_ref().is_some_and(|v| !v.is_empty())
    {
        return Err(reject());
    }
    let pointer = memory.summary_ref.as_ref().ok_or_else(reject)?;
    if pointer.content_key_id != memory.content_key_id
        || pointer.aad.as_deref() != Some(field_aad(&id, FIELD).as_str())
    {
        return Err(reject());
    }
    let columns: (String, String) = store
        .conn
        .query_row(
            "SELECT content_key_id,forget_state FROM memories WHERE memory_id=?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(backend)?;
    if columns
        != (
            memory.content_key_id.to_string(),
            enum_text(&memory.forget_state)?,
        )
    {
        return Err(reject());
    }
    // Scalar bound before loading the payload. This existing preflight also
    // checks actual row/field/key/subject/nonce length, but is not authentication.
    if store.sealed_plaintext_bytes_for_field(id, FIELD, pointer)? > budget as u64 {
        return Err(reject());
    }
    let destroyed = store.content_key_destroyed(memory.content_key_id)?;
    if destroyed || store.content_key(memory.content_key_id)?.is_none() {
        return Err(reject());
    }
    let key = wrapped_key(store, memory.content_key_id)?;
    let blob = blob(store, pointer.blob_id)?;
    Ok(Held {
        memory,
        blob,
        key,
        destroyed,
    })
}

#[cfg(test)]
fn capture(store: &mut SqlCipherStore, id: Uuid, budget: usize) -> StoreResult<Held> {
    capture_checked(store, id, budget, |_| Ok(()))
}

/// Run the caller's ordinary read in the same snapshot as the private capture.
/// The callback gets only the existing read-only store reference, never conn/CK.
#[doc(hidden)]
pub fn capture_checked(
    store: &mut SqlCipherStore,
    id: Uuid,
    budget: usize,
    read: impl FnOnce(&SqlCipherStore) -> StoreResult<()>,
) -> StoreResult<Held> {
    store.transact(|store| {
        let held = state(store, id, budget)?;
        // Read both present fields through the real BlobStore in this snapshot.
        // Opened bytes exist only for this validation and are zeroized on drop.
        for pointer in [&held.memory.title_ref, &held.memory.summary_ref]
            .into_iter()
            .flatten()
        {
            let field = if Some(pointer) == held.memory.summary_ref.as_ref() {
                FIELD
            } else {
                "title_ref"
            };
            if store.sealed_plaintext_bytes_for_field(id, field, pointer)? > budget as u64 {
                return Err(reject());
            }
            let plaintext = Zeroizing::new(store.open(pointer)?);
            text(&plaintext, pointer.char_count, budget)?;
        }
        read(store)?;
        Ok(held)
    })
}

#[doc(hidden)]
pub fn repair(store: &mut SqlCipherStore, held: Held, budget: usize) -> StoreResult<usize> {
    repair_with(store, held, budget, |_, _| Ok(()))
}

fn repair_with(
    store: &mut SqlCipherStore,
    held: Held,
    budget: usize,
    mut hook: impl FnMut(Point, &mut SqlCipherStore) -> StoreResult<()>,
) -> StoreResult<usize> {
    store.transact(|store| {
        hook(Point::Locked, store)?;
        let current = state(store, held.memory.memory_id, budget)?;
        if current.memory != held.memory
            || current.key != held.key
            || current.destroyed != held.destroyed
            || current.blob.row_id != held.blob.row_id
            || current.blob.field != held.blob.field
            || current.blob.key_id != held.blob.key_id
            || current.blob.subject != held.blob.subject
            || current.blob.nonce != held.blob.nonce
        {
            return Err(reject());
        }
        let pointer = held.memory.summary_ref.as_ref().ok_or_else(reject)?;
        let key = store
            .content_key(pointer.content_key_id)?
            .ok_or_else(reject)?;
        let original = Zeroizing::new(
            XChaCha20Poly1305::new(Key::from_slice(key.expose()))
                .decrypt(
                    XNonce::from_slice(&held.blob.nonce),
                    Payload {
                        msg: &held.blob.ciphertext,
                        aad: field_aad(&held.memory.memory_id, FIELD).as_bytes(),
                    },
                )
                .map_err(|_| reject())?,
        );
        text(&original, pointer.char_count, budget)?;
        if current.blob.ciphertext == held.blob.ciphertext {
            return Ok(0);
        }
        match store.open(pointer) {
            Ok(bytes) => {
                let _opened = Zeroizing::new(bytes);
                return Ok(0);
            }
            Err(StoreError::SealBroken(id)) if id == pointer.blob_id => {}
            Err(error) => return Err(error),
        }
        hook(Point::Validated, store)?;
        let changed = store
            .conn
            .execute(
                "UPDATE sealed_blobs SET ciphertext=?1
             WHERE blob_id=?2 AND row_id=?3 AND field=?4 AND content_key_id=?5
               AND subject=?6 AND nonce=?7 AND ciphertext=?8",
                rusqlite::params![
                    held.blob.ciphertext.as_slice(),
                    pointer.blob_id.to_string(),
                    held.blob.row_id,
                    held.blob.field,
                    held.blob.key_id,
                    held.blob.subject,
                    held.blob.nonce,
                    current.blob.ciphertext.as_slice(),
                ],
            )
            .map_err(backend)?;
        if changed != 1 {
            return Err(reject());
        }
        hook(Point::Updated, store)?;
        let restored = Zeroizing::new(store.open(pointer)?);
        text(&restored, pointer.char_count, budget)?;
        hook(Point::BeforeCommit, store)?;
        Ok(changed)
    })
}

#[cfg(test)]
fn fixture(store: &mut SqlCipherStore) -> SoulMemory {
    let id = Uuid::now_v7();
    let key = Uuid::now_v7();
    let title = store
        .seal(SealRequest::new(
            key,
            id,
            "title_ref",
            SealedSubject::Owner,
            b"synthetic title".to_vec(),
        ))
        .unwrap();
    let summary = store
        .seal(SealRequest::new(
            key,
            id,
            FIELD,
            SealedSubject::Owner,
            b"synthetic summary".to_vec(),
        ))
        .unwrap();
    let memory = SoulMemory {
        schema_version: SchemaVersion,
        memory_id: id,
        memory_type: MemoryType::Semantic,
        title_ref: Some(title),
        summary_ref: Some(summary),
        source_event_ids: None,
        evidence_ids: None,
        content_key_id: key,
        forget_state: ForgetState::Active,
        third_party_content_present: Some(false),
    };
    store.put_memory(memory.clone()).unwrap();
    store
        .append_audit(
            serde_json::from_value(serde_json::json!({
                "schema_version": "1.0.0", "seq": 0, "entry_id": Uuid::now_v7(),
                "ts": "2026-10-20T00:00:00Z",
                "prev_hash": crate::GENESIS_PREV_HASH, "entry_hash": crate::GENESIS_PREV_HASH,
                "action": "memory.write", "decision": "allowed",
                "subject_refs": [id]
            }))
            .unwrap(),
        )
        .unwrap();
    memory
}

#[test]
fn n1_restores_only_corrupted_ciphertext_through_real_store() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = SqlCipherStore::open(
        dir.path().join("n1.db"),
        &TestKeyProvider::from_seed("n1 synthetic fixture"),
    )
    .unwrap();
    let memory = fixture(&mut store);
    let pointer = memory.summary_ref.as_ref().unwrap();
    let original_state = tests::database_rows(&store);
    assert!(store.open(pointer).unwrap() == b"synthetic summary");
    let held = capture(&mut store, memory.memory_id, 128).expect("capture normal snapshot");
    assert!(held.memory == memory);
    let mut corrupted = held.blob.ciphertext.to_vec();
    corrupted[0] ^= 1;
    assert_eq!(
        store
            .conn
            .execute(
                "UPDATE sealed_blobs SET ciphertext=?1 WHERE blob_id=?2",
                rusqlite::params![corrupted, pointer.blob_id.to_string()],
            )
            .unwrap(),
        1
    );
    assert_eq!(
        store.open(pointer),
        Err(StoreError::SealBroken(pointer.blob_id))
    );
    assert_eq!(repair(&mut store, held, 128).unwrap(), 1);
    assert!(store.open(pointer).unwrap() == b"synthetic summary");
    assert!(store.get_memory(memory.memory_id).unwrap() == memory);
    assert!(tests::database_rows(&store) == original_state);
}
