//! The storage boundary every Soul module talks to.
//!
//! WP01 ships the contract and an in-memory [`FakeStore`], not a database. The
//! real SQLCipher-backed implementation arrives in WP02 and must pass the same
//! [`conformance`] suite.
//!
//! Two properties are baked into the traits rather than left to the backend:
//!
//! * prose is only ever reachable through [`BlobStore`], sealed under a content
//!   key, so "forget" has something concrete to destroy;
//! * [`ForgetOps::execute_forget`] destroys that key, which makes the plaintext
//!   unrecoverable and demotes derived inferences to `orphaned`.
//!
//! [`research::ResearchPreview`] sits deliberately outside [`SoulStore`]: the
//! research track reads, and only reads.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod conformance;
pub mod fake;
pub mod forget;
pub mod research;
pub mod types;

pub use fake::FakeStore;
pub use forget::{ForgetImpact, ForgetOps, ForgetReceipt, ForgetUnit};
pub use research::{ResearchPreview, ResearchPreviewReport, ResearchPreviewRequest};
pub use types::{EventFilter, InferenceState, SealRequest, StoreError, StoreResult};

use uuid::Uuid;

use soul_schema::audit::SoulAuditEntry;
use soul_schema::common::SealedText;
use soul_schema::contact::SoulContact;
use soul_schema::event::SoulEvent;
use soul_schema::evidence::SoulEvidence;
use soul_schema::inference::SoulInference;
use soul_schema::memory::SoulMemory;
use soul_schema::profile::SoulProfile;
use soul_schema::relationship::SoulRelationship;

/// Append-only event log.
pub trait EventStore {
    fn append_event(&mut self, event: SoulEvent) -> StoreResult<Uuid>;
    fn get_event(&self, event_id: Uuid) -> StoreResult<SoulEvent>;
    fn list_events(&self, filter: &EventFilter) -> StoreResult<Vec<SoulEvent>>;

    fn count_events(&self, filter: &EventFilter) -> StoreResult<u64> {
        Ok(self.list_events(filter)?.len() as u64)
    }
}

/// Profile, the evidence behind it, and the inferences derived from it.
pub trait ProfileStore {
    fn put_profile(&mut self, profile: SoulProfile) -> StoreResult<Uuid>;
    fn get_profile(&self, profile_id: Uuid) -> StoreResult<SoulProfile>;

    fn put_evidence(&mut self, evidence: SoulEvidence) -> StoreResult<Uuid>;
    fn get_evidence(&self, evidence_id: Uuid) -> StoreResult<SoulEvidence>;

    /// Must reject an inference whose `evidence_ids` is empty or dangling.
    fn put_inference(&mut self, inference: SoulInference) -> StoreResult<Uuid>;
    fn get_inference(&self, inference_id: Uuid) -> StoreResult<SoulInference>;
    fn list_inferences(&self) -> StoreResult<Vec<SoulInference>>;

    /// `Orphaned` once the evidence it rests on has been forgotten.
    fn inference_state(&self, inference_id: Uuid) -> StoreResult<InferenceState>;
}

/// Autobiographical memory. Titles and summaries live in [`BlobStore`].
pub trait MemoryStore {
    fn put_memory(&mut self, memory: SoulMemory) -> StoreResult<Uuid>;
    fn get_memory(&self, memory_id: Uuid) -> StoreResult<SoulMemory>;
    fn list_memories(&self) -> StoreResult<Vec<SoulMemory>>;
}

/// People and the edges between them. Never leaves the machine.
pub trait GraphStore {
    fn put_contact(&mut self, contact: SoulContact) -> StoreResult<Uuid>;
    fn get_contact(&self, contact_id: Uuid) -> StoreResult<SoulContact>;
    fn list_contacts(&self) -> StoreResult<Vec<SoulContact>>;

    fn put_relationship(&mut self, relationship: SoulRelationship) -> StoreResult<Uuid>;
    fn get_relationship(&self, relationship_id: Uuid) -> StoreResult<SoulRelationship>;
    fn relationships_for(&self, contact_id: Uuid) -> StoreResult<Vec<SoulRelationship>>;
}

/// Sealed prose. The only way plaintext enters or leaves the store.
pub trait BlobStore {
    /// Encrypt `plaintext` under a content key, binding row identity and field
    /// name into the AEAD tag.
    fn seal(&mut self, request: SealRequest) -> StoreResult<SealedText>;

    /// Fails with [`StoreError::ContentKeyDestroyed`] once the key is gone.
    fn open(&self, sealed: &SealedText) -> StoreResult<Vec<u8>>;

    fn has_content_key(&self, content_key_id: Uuid) -> bool;
}

/// Hash-chained, append-only, and free of prose by contract.
///
/// Forgetting must never be blocked by the audit chain, so entries survive the
/// rows they refer to and keep only bare UUIDs.
pub trait AuditLog {
    fn append_audit(&mut self, entry: SoulAuditEntry) -> StoreResult<Uuid>;
    fn list_audit(&self) -> StoreResult<Vec<SoulAuditEntry>>;
}

/// Everything a Soul backend must provide.
pub trait SoulStore:
    EventStore + ProfileStore + MemoryStore + GraphStore + BlobStore + AuditLog + ForgetOps
{
    /// Persist anything buffered. In-memory backends may no-op.
    fn flush(&mut self) -> StoreResult<()> {
        Ok(())
    }
}
