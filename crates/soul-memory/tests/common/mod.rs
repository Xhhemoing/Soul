//! The memory fixture, and a real store to put it in.
//!
//! Both acceptance tests here run against `SqlCipherStore` rather than
//! `FakeStore`. AC-14 is about what survives a write, and AC-15 is about what
//! does not survive a forget; neither question can be answered by an in-memory
//! double, because the interesting failure mode in both is a value that is
//! correct in process and wrong on disk.

// Each test binary uses a different subset of these.
#![allow(dead_code)]

use serde::Deserialize;
use uuid::Uuid;

use soul_memory::{MemoryDraft, MemoryEdit};
use soul_schema::common::{
    Derivation, EgressPolicy, NotAClinicalClaim, Privacy, Purpose, Retention, SchemaVersion,
    SealedSubject, Subject, SupportedBand,
};
use soul_schema::evidence::{EvidenceKind, SoulEvidence};
use soul_schema::inference::SoulInference;
use soul_schema::memory::MemoryType;
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::ProfileStore;
use soul_testkit::fixtures;

/// 2026-08-24T00:00:00Z. Passed in rather than read from the clock, so the
/// audit entries a test compares are the same on every run.
pub const NOW: i64 = 1_787_529_600;

/// The database passphrase seed. Fixed, so a reopen in the same test uses the
/// same key material and a failure to decrypt means something real.
pub const SEED: &str = "wp04 autobiographical memory";

// --------------------------------------------------------------- fixture ---

/// `fixtures/memory/memories_basic.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct MemoryFixture {
    pub memories: Vec<FixtureMemory>,
    pub edit: FixtureEdit,
}

/// One memory as the user just finished typing it: still plaintext, because
/// that is the only moment at which it legitimately is.
#[derive(Debug, Clone, Deserialize)]
pub struct FixtureMemory {
    /// A stable handle for the test to name this row by. Not stored.
    pub id: String,
    pub memory_type: MemoryType,
    pub subject: SealedSubject,
    pub title: String,
    pub summary: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FixtureEdit {
    pub target: String,
    pub title: String,
    pub summary: String,
}

impl MemoryFixture {
    pub fn load() -> Self {
        fixtures::read_json("memory/memories_basic.json").expect("load the memory fixture")
    }

    pub fn get(&self, id: &str) -> &FixtureMemory {
        self.memories
            .iter()
            .find(|memory| memory.id == id)
            .unwrap_or_else(|| panic!("the fixture has no memory called {id}"))
    }

    /// Every title and summary in the fixture, which is the corpus a leakage
    /// check runs the audit chain against.
    pub fn prose(&self) -> Vec<&str> {
        let mut out = Vec::new();
        for memory in &self.memories {
            out.push(memory.title.as_str());
            out.push(memory.summary.as_str());
        }
        out.push(self.edit.title.as_str());
        out.push(self.edit.summary.as_str());
        out
    }
}

impl FixtureMemory {
    pub fn draft(&self) -> MemoryDraft {
        MemoryDraft::own(self.memory_type, self.title.as_str(), self.summary.as_str())
            .about(self.subject)
    }
}

impl FixtureEdit {
    pub fn edit(&self) -> MemoryEdit {
        MemoryEdit::title(self.title.as_str()).and_summary(self.summary.as_str())
    }
}

// ----------------------------------------------------------------- store ---

pub fn keys() -> TestKeyProvider {
    TestKeyProvider::from_seed(SEED)
}

pub fn open(path: impl AsRef<std::path::Path>) -> SqlCipherStore {
    SqlCipherStore::open(path, &keys()).expect("open the store")
}

// --------------------------------------------- rows this crate does not own --

fn privacy() -> Privacy {
    Privacy {
        subject: Subject::Owner,
        derivation: Derivation::Raw,
        purposes: vec![Purpose::Memory],
        retention: Retention::until_forgotten(),
        egress: EgressPolicy::default(),
    }
}

/// An evidence row a memory can cite. WP04 does not mint evidence — that is
/// WP03's and WP05's job — but AC-15 needs something for a forget to orphan.
pub fn evidence(evidence_id: Uuid) -> SoulEvidence {
    SoulEvidence {
        schema_version: SchemaVersion,
        evidence_id,
        kind: EvidenceKind::UserStatement,
        subject: Subject::Owner,
        source_refs: vec![serde_json::json!({ "origin": "wp04-fixture" })],
        strength: SupportedBand::Moderate,
        method: None,
        exportable_to_research: Some(false),
        privacy: Some(privacy()),
    }
}

/// An inference resting on that evidence, so the forget has something to demote.
pub fn inference(inference_id: Uuid, axis_id: Uuid, evidence_ids: &[Uuid]) -> SoulInference {
    SoulInference {
        schema_version: SchemaVersion,
        inference_id,
        target: serde_json::json!({ "kind": "trait_axis", "axis_id": axis_id }),
        statement_key: "trait.curiosity.leans_high".into(),
        evidence_ids: evidence_ids.to_vec(),
        evidence_band: SupportedBand::Weak,
        method: None,
        user_verdict: None,
        clinical_claim: NotAClinicalClaim,
        falsifier: None,
    }
}

/// Store the evidence and the inference that will be orphaned later.
pub fn seed_derivation(
    store: &mut SqlCipherStore,
    evidence_id: Uuid,
    inference_id: Uuid,
    axis_id: Uuid,
) {
    store.put_evidence(evidence(evidence_id)).expect("evidence");
    store
        .put_inference(inference(inference_id, axis_id, &[evidence_id]))
        .expect("inference");
}
