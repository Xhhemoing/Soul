//! An in-memory backend used by tests and by the headless Linux CI.
//!
//! It is a fake, not a mock: sealing really encrypts with XChaCha20-Poly1305
//! and binds `row_id|field` as additional authenticated data, so "destroy the
//! content key and the prose is unreadable" is demonstrated rather than
//! asserted. What it does not do is persist anything.

use std::collections::{BTreeMap, BTreeSet};

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use uuid::Uuid;

use soul_schema::audit::SoulAuditEntry;
use soul_schema::common::{field_aad, SealAlg, SealedText};
use soul_schema::contact::SoulContact;
use soul_schema::event::SoulEvent;
use soul_schema::evidence::SoulEvidence;
use soul_schema::inference::SoulInference;
use soul_schema::memory::{ForgetState, SoulMemory};
use soul_schema::profile::SoulProfile;
use soul_schema::relationship::SoulRelationship;
use soul_schema::validate::SchemaId;
use soul_schema::SchemaSet;

use crate::forget::{ForgetImpact, ForgetOps, ForgetReceipt, ForgetUnit};
use crate::types::{EventFilter, InferenceState, SealRequest, StoreError, StoreResult};
use crate::{AuditLog, BlobStore, EventStore, GraphStore, MemoryStore, ProfileStore, SoulStore};

#[derive(Debug, Clone)]
struct SealedBlob {
    content_key_id: Uuid,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

/// Fully in-memory [`SoulStore`].
#[derive(Debug)]
pub struct FakeStore {
    schemas: SchemaSet,
    events: Vec<SoulEvent>,
    profiles: BTreeMap<Uuid, SoulProfile>,
    evidence: BTreeMap<Uuid, SoulEvidence>,
    inferences: BTreeMap<Uuid, SoulInference>,
    inference_states: BTreeMap<Uuid, InferenceState>,
    memories: BTreeMap<Uuid, SoulMemory>,
    contacts: BTreeMap<Uuid, SoulContact>,
    relationships: BTreeMap<Uuid, SoulRelationship>,
    audit: Vec<SoulAuditEntry>,
    content_keys: BTreeMap<Uuid, Vec<u8>>,
    blobs: BTreeMap<Uuid, SealedBlob>,
}

impl Default for FakeStore {
    fn default() -> Self {
        FakeStore::new()
    }
}

impl FakeStore {
    pub fn new() -> Self {
        FakeStore {
            schemas: SchemaSet::load().expect("the frozen contracts compile"),
            events: Vec::new(),
            profiles: BTreeMap::new(),
            evidence: BTreeMap::new(),
            inferences: BTreeMap::new(),
            inference_states: BTreeMap::new(),
            memories: BTreeMap::new(),
            contacts: BTreeMap::new(),
            relationships: BTreeMap::new(),
            audit: Vec::new(),
            content_keys: BTreeMap::new(),
            blobs: BTreeMap::new(),
        }
    }

    /// Reject anything that would not validate against its own contract, so a
    /// backend bug shows up at the write rather than at the next read.
    fn check<T: serde::Serialize>(&self, id: SchemaId, model: &T) -> StoreResult<()> {
        self.schemas
            .validate_model(id, model)
            .map(|_| ())
            .map_err(|failure| StoreError::ContractViolation(failure.to_string()))
    }

    fn cipher(&self, content_key_id: Uuid) -> StoreResult<XChaCha20Poly1305> {
        let raw = self
            .content_keys
            .get(&content_key_id)
            .ok_or(StoreError::ContentKeyDestroyed(content_key_id))?;
        Ok(XChaCha20Poly1305::new(Key::from_slice(raw)))
    }

    /// Content keys reachable from one forget unit.
    fn keys_of(&self, unit: ForgetUnit) -> Vec<Uuid> {
        let mut keys = BTreeSet::new();
        match unit {
            ForgetUnit::ContentKey(id) => {
                keys.insert(id);
            }
            ForgetUnit::Memory(id) => {
                if let Some(memory) = self.memories.get(&id) {
                    keys.insert(memory.content_key_id);
                    for sealed in [&memory.title_ref, &memory.summary_ref]
                        .into_iter()
                        .flatten()
                    {
                        keys.insert(sealed.content_key_id);
                    }
                }
            }
            ForgetUnit::Contact(id) => {
                if let Some(contact) = self.contacts.get(&id) {
                    if let Some(sealed) = &contact.display_label_ref {
                        keys.insert(sealed.content_key_id);
                    }
                }
            }
        }
        keys.into_iter().collect()
    }

    fn memories_under(&self, keys: &[Uuid]) -> Vec<Uuid> {
        self.memories
            .values()
            .filter(|memory| {
                keys.contains(&memory.content_key_id)
                    || [&memory.title_ref, &memory.summary_ref]
                        .into_iter()
                        .flatten()
                        .any(|sealed| keys.contains(&sealed.content_key_id))
            })
            .map(|memory| memory.memory_id)
            .collect()
    }

    fn contacts_under(&self, keys: &[Uuid]) -> Vec<Uuid> {
        self.contacts
            .values()
            .filter(|contact| {
                contact
                    .display_label_ref
                    .as_ref()
                    .is_some_and(|sealed| keys.contains(&sealed.content_key_id))
            })
            .map(|contact| contact.contact_id)
            .collect()
    }

    /// Evidence that becomes unreadable when these rows are forgotten.
    ///
    /// Memories cite their evidence directly; contacts do so through the graph
    /// edges they take part in.
    fn evidence_losing_support(&self, memories: &[Uuid], contacts: &[Uuid]) -> BTreeSet<Uuid> {
        let mut ids = BTreeSet::new();
        for memory_id in memories {
            if let Some(memory) = self.memories.get(memory_id) {
                ids.extend(memory.evidence_ids.iter().flatten().copied());
            }
        }
        for contact_id in contacts {
            for edge in self.relationships.values() {
                if edge.from_contact_id == *contact_id || edge.to_contact_id == *contact_id {
                    ids.extend(edge.evidence_ids.iter().copied());
                }
            }
        }
        ids
    }

    fn inferences_resting_on(&self, evidence: &BTreeSet<Uuid>) -> Vec<Uuid> {
        self.inferences
            .values()
            .filter(|inference| {
                self.inference_states
                    .get(&inference.inference_id)
                    .copied()
                    .unwrap_or(InferenceState::Live)
                    == InferenceState::Live
                    && inference
                        .evidence_ids
                        .iter()
                        .any(|id| evidence.contains(id))
            })
            .map(|inference| inference.inference_id)
            .collect()
    }

    fn impact_of(&self, unit: ForgetUnit) -> ForgetImpact {
        let keys = self.keys_of(unit);
        let memories = self.memories_under(&keys);
        let contacts = self.contacts_under(&keys);
        let evidence = self.evidence_losing_support(&memories, &contacts);
        let orphaned = self.inferences_resting_on(&evidence);

        let touched: BTreeSet<Uuid> = memories
            .iter()
            .chain(contacts.iter())
            .copied()
            .chain(std::iter::once(unit.id()))
            .collect();

        ForgetImpact {
            sealed_blobs_destroyed: self
                .blobs
                .values()
                .filter(|blob| keys.contains(&blob.content_key_id))
                .count() as u64,
            memories_affected: memories.len() as u64,
            contacts_affected: contacts.len() as u64,
            inferences_orphaned: orphaned.len() as u64,
            audit_entries_retained: self
                .audit
                .iter()
                .filter(|entry| {
                    entry
                        .subject_refs
                        .iter()
                        .flatten()
                        .any(|id| touched.contains(id))
                })
                .count() as u64,
            content_key_ids: keys,
        }
    }

    /// Number of content keys still held. Useful for asserting destruction.
    pub fn content_key_count(&self) -> usize {
        self.content_keys.len()
    }

    /// Number of sealed blobs still held.
    pub fn blob_count(&self) -> usize {
        self.blobs.len()
    }
}

impl EventStore for FakeStore {
    fn append_event(&mut self, event: SoulEvent) -> StoreResult<Uuid> {
        self.check(SchemaId::Event, &event)?;
        let id = event.event_id;
        if self.events.iter().any(|e| e.event_id == id) {
            return Err(StoreError::AlreadyExists { kind: "event", id });
        }
        self.events.push(event);
        Ok(id)
    }

    fn get_event(&self, event_id: Uuid) -> StoreResult<SoulEvent> {
        self.events
            .iter()
            .find(|e| e.event_id == event_id)
            .cloned()
            .ok_or_else(|| StoreError::not_found("event", event_id))
    }

    fn list_events(&self, filter: &EventFilter) -> StoreResult<Vec<SoulEvent>> {
        let mut out: Vec<SoulEvent> = self
            .events
            .iter()
            .filter(|e| filter.source.is_none_or(|s| s == e.source))
            .filter(|e| filter.kind.is_none_or(|k| k == e.kind))
            .filter(|e| {
                filter
                    .since
                    .as_deref()
                    .is_none_or(|since| e.ts.as_str() >= since)
            })
            .cloned()
            .collect();
        if let Some(limit) = filter.limit {
            out.truncate(limit);
        }
        Ok(out)
    }
}

impl ProfileStore for FakeStore {
    fn put_profile(&mut self, profile: SoulProfile) -> StoreResult<Uuid> {
        self.check(SchemaId::Profile, &profile)?;
        let id = profile.profile_id;
        self.profiles.insert(id, profile);
        Ok(id)
    }

    fn get_profile(&self, profile_id: Uuid) -> StoreResult<SoulProfile> {
        self.profiles
            .get(&profile_id)
            .cloned()
            .ok_or_else(|| StoreError::not_found("profile", profile_id))
    }

    fn put_evidence(&mut self, evidence: SoulEvidence) -> StoreResult<Uuid> {
        self.check(SchemaId::Evidence, &evidence)?;
        let id = evidence.evidence_id;
        self.evidence.insert(id, evidence);
        Ok(id)
    }

    fn get_evidence(&self, evidence_id: Uuid) -> StoreResult<SoulEvidence> {
        self.evidence
            .get(&evidence_id)
            .cloned()
            .ok_or_else(|| StoreError::not_found("evidence", evidence_id))
    }

    fn put_inference(&mut self, inference: SoulInference) -> StoreResult<Uuid> {
        self.check(SchemaId::Inference, &inference)?;
        if inference.evidence_ids.is_empty() {
            return Err(StoreError::ContractViolation(
                "an inference with no evidence must not be stored".into(),
            ));
        }
        for evidence_id in &inference.evidence_ids {
            if !self.evidence.contains_key(evidence_id) {
                return Err(StoreError::ContractViolation(format!(
                    "inference {} cites evidence {evidence_id}, which does not resolve",
                    inference.inference_id
                )));
            }
        }
        let id = inference.inference_id;
        self.inferences.insert(id, inference);
        self.inference_states.insert(id, InferenceState::Live);
        Ok(id)
    }

    fn get_inference(&self, inference_id: Uuid) -> StoreResult<SoulInference> {
        self.inferences
            .get(&inference_id)
            .cloned()
            .ok_or_else(|| StoreError::not_found("inference", inference_id))
    }

    fn list_inferences(&self) -> StoreResult<Vec<SoulInference>> {
        Ok(self.inferences.values().cloned().collect())
    }

    fn inference_state(&self, inference_id: Uuid) -> StoreResult<InferenceState> {
        self.inference_states
            .get(&inference_id)
            .copied()
            .ok_or_else(|| StoreError::not_found("inference", inference_id))
    }
}

impl MemoryStore for FakeStore {
    fn put_memory(&mut self, memory: SoulMemory) -> StoreResult<Uuid> {
        self.check(SchemaId::Memory, &memory)?;
        let id = memory.memory_id;
        self.memories.insert(id, memory);
        Ok(id)
    }

    fn get_memory(&self, memory_id: Uuid) -> StoreResult<SoulMemory> {
        self.memories
            .get(&memory_id)
            .cloned()
            .ok_or_else(|| StoreError::not_found("memory", memory_id))
    }

    fn list_memories(&self) -> StoreResult<Vec<SoulMemory>> {
        Ok(self.memories.values().cloned().collect())
    }
}

impl GraphStore for FakeStore {
    fn put_contact(&mut self, contact: SoulContact) -> StoreResult<Uuid> {
        self.check(SchemaId::Contact, &contact)?;
        let id = contact.contact_id;
        self.contacts.insert(id, contact);
        Ok(id)
    }

    fn get_contact(&self, contact_id: Uuid) -> StoreResult<SoulContact> {
        self.contacts
            .get(&contact_id)
            .cloned()
            .ok_or_else(|| StoreError::not_found("contact", contact_id))
    }

    fn list_contacts(&self) -> StoreResult<Vec<SoulContact>> {
        Ok(self.contacts.values().cloned().collect())
    }

    fn put_relationship(&mut self, relationship: SoulRelationship) -> StoreResult<Uuid> {
        self.check(SchemaId::Relationship, &relationship)?;
        let id = relationship.relationship_id;
        self.relationships.insert(id, relationship);
        Ok(id)
    }

    fn get_relationship(&self, relationship_id: Uuid) -> StoreResult<SoulRelationship> {
        self.relationships
            .get(&relationship_id)
            .cloned()
            .ok_or_else(|| StoreError::not_found("relationship", relationship_id))
    }

    fn relationships_for(&self, contact_id: Uuid) -> StoreResult<Vec<SoulRelationship>> {
        Ok(self
            .relationships
            .values()
            .filter(|edge| edge.from_contact_id == contact_id || edge.to_contact_id == contact_id)
            .cloned()
            .collect())
    }
}

impl BlobStore for FakeStore {
    fn seal(&mut self, request: SealRequest) -> StoreResult<SealedText> {
        let raw_key = self
            .content_keys
            .entry(request.content_key_id)
            .or_insert_with(|| XChaCha20Poly1305::generate_key(&mut OsRng).to_vec())
            .clone();
        let cipher = XChaCha20Poly1305::new(Key::from_slice(&raw_key));
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
        let aad = field_aad(&request.row_id, &request.field);

        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: &request.plaintext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| StoreError::Backend("sealing failed".into()))?;

        let blob_id = Uuid::now_v7();
        self.blobs.insert(
            blob_id,
            SealedBlob {
                content_key_id: request.content_key_id,
                nonce: nonce.to_vec(),
                ciphertext,
            },
        );

        Ok(SealedText {
            content_key_id: request.content_key_id,
            blob_id,
            alg: SealAlg,
            aad: Some(aad),
            subject: request.subject,
            char_count: String::from_utf8_lossy(&request.plaintext).chars().count() as u64,
            placeholder: request.placeholder,
        })
    }

    fn open(&self, sealed: &SealedText) -> StoreResult<Vec<u8>> {
        let cipher = self.cipher(sealed.content_key_id)?;
        let blob = self
            .blobs
            .get(&sealed.blob_id)
            .ok_or(StoreError::BlobMissing(sealed.blob_id))?;
        let aad = sealed.aad.as_deref().unwrap_or_default();
        cipher
            .decrypt(
                XNonce::from_slice(&blob.nonce),
                Payload {
                    msg: &blob.ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| StoreError::SealBroken(sealed.blob_id))
    }

    fn has_content_key(&self, content_key_id: Uuid) -> bool {
        self.content_keys.contains_key(&content_key_id)
    }
}

impl AuditLog for FakeStore {
    fn append_audit(&mut self, entry: SoulAuditEntry) -> StoreResult<Uuid> {
        self.check(SchemaId::Audit, &entry)?;
        let id = entry.entry_id;
        self.audit.push(entry);
        Ok(id)
    }

    fn list_audit(&self) -> StoreResult<Vec<SoulAuditEntry>> {
        Ok(self.audit.clone())
    }
}

impl ForgetOps for FakeStore {
    fn preview_impact(&self, unit: ForgetUnit) -> StoreResult<ForgetImpact> {
        Ok(self.impact_of(unit))
    }

    fn execute_forget(&mut self, unit: ForgetUnit) -> StoreResult<ForgetReceipt> {
        let impact = self.impact_of(unit);
        let keys = impact.content_key_ids.clone();

        let memories = self.memories_under(&keys);
        let contacts = self.contacts_under(&keys);
        let evidence = self.evidence_losing_support(&memories, &contacts);
        let orphaned = self.inferences_resting_on(&evidence);

        for key in &keys {
            self.content_keys.remove(key);
        }
        self.blobs
            .retain(|_, blob| !keys.contains(&blob.content_key_id));

        for memory_id in memories {
            if let Some(memory) = self.memories.get_mut(&memory_id) {
                memory.forget_state = ForgetState::Forgotten;
            }
        }
        for contact_id in contacts {
            if let Some(contact) = self.contacts.get_mut(&contact_id) {
                contact.forget_state = ForgetState::Forgotten;
            }
        }
        for inference_id in orphaned {
            self.inference_states
                .insert(inference_id, InferenceState::Orphaned);
        }

        // The audit chain is deliberately untouched. It carries no prose, and
        // PRODUCT_LOCK forbids it from standing in the way of a forget.
        Ok(ForgetReceipt { unit, impact })
    }
}

impl SoulStore for FakeStore {}
