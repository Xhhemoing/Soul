//! The hash chain over the audit table.
//!
//! `docs/SECURITY.md` asks for an append-only chain with no prose in it, and
//! AC-24 asks that the chain still verify after the process is killed mid-write.
//! Both only mean something if the store, not the caller, decides `seq`,
//! `prev_hash` and `entry_hash`: a caller that supplies its own links can hand
//! in a chain that verifies against nothing.
//!
//! So [`link`] overwrites those three fields on the way in, and [`verify`]
//! recomputes them on the way out. Everything else about the entry is the
//! caller's, and the contract's `additionalProperties: false` is what keeps a
//! stray `body` field out.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use soul_schema::audit::SoulAuditEntry;
use soul_schema::common::Sha256Hex;

/// `prev_hash` of the first entry. There is nothing before it.
pub const GENESIS_PREV_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// One link of the chain as it sits in the `audit` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditLink {
    pub seq: u64,
    pub prev_hash: String,
    pub entry_hash: String,
    pub entry: SoulAuditEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChainError {
    #[error("audit entry {position} has seq {found}, expected {expected}")]
    SeqOutOfOrder {
        position: usize,
        found: u64,
        expected: u64,
    },

    #[error("audit entry {seq} points back at {found}, but entry {previous} hashes to {expected}")]
    BrokenLink {
        seq: u64,
        previous: u64,
        found: String,
        expected: String,
    },

    #[error("audit entry {seq} stores hash {stored} but its contents hash to {recomputed}")]
    ContentAltered {
        seq: u64,
        stored: String,
        recomputed: String,
    },

    #[error("audit entry {seq} could not be hashed: {message}")]
    NotHashable { seq: u64, message: String },
}

/// Stamp an entry into position `seq`, chained to `prev_hash`.
///
/// Returns the entry as it will be stored, with all three chain fields set by
/// this function rather than by whoever built the entry.
pub fn link(mut entry: SoulAuditEntry, seq: u64, prev_hash: &str) -> Result<AuditLink, String> {
    entry.seq = seq;
    entry.prev_hash = Sha256Hex::new(prev_hash.to_owned());
    let entry_hash = hash_entry(&entry)?;
    entry.entry_hash = Sha256Hex::new(entry_hash.clone());
    Ok(AuditLink {
        seq,
        prev_hash: prev_hash.to_owned(),
        entry_hash,
        entry,
    })
}

/// SHA-256 over the entry with `entry_hash` removed.
///
/// `serde_json::Map` is a `BTreeMap` here, so the encoding is key-sorted and
/// the digest is reproducible across processes and platforms.
pub fn hash_entry(entry: &SoulAuditEntry) -> Result<String, String> {
    let mut value = serde_json::to_value(entry).map_err(|e| e.to_string())?;
    let object: &mut Map<String, Value> = value
        .as_object_mut()
        .ok_or_else(|| "an audit entry must serialize to an object".to_owned())?;
    object.remove("entry_hash");
    let canonical = serde_json::to_vec(object).map_err(|e| e.to_string())?;
    Ok(hex::encode(Sha256::digest(canonical)))
}

/// Walk the chain in stored order and report the first place it breaks.
pub fn verify(links: &[AuditLink]) -> Result<(), ChainError> {
    let mut expected_prev = GENESIS_PREV_HASH.to_owned();
    for (position, current) in links.iter().enumerate() {
        let expected_seq = position as u64;
        if current.seq != expected_seq {
            return Err(ChainError::SeqOutOfOrder {
                position,
                found: current.seq,
                expected: expected_seq,
            });
        }
        if current.prev_hash != expected_prev {
            return Err(ChainError::BrokenLink {
                seq: current.seq,
                previous: expected_seq.saturating_sub(1),
                found: current.prev_hash.clone(),
                expected: expected_prev,
            });
        }
        let recomputed = hash_entry(&current.entry).map_err(|message| ChainError::NotHashable {
            seq: current.seq,
            message,
        })?;
        if recomputed != current.entry_hash {
            return Err(ChainError::ContentAltered {
                seq: current.seq,
                stored: current.entry_hash.clone(),
                recomputed,
            });
        }
        expected_prev = current.entry_hash.clone();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use soul_schema::audit::{AuditAction, AuditDecision};
    use soul_schema::common::{SchemaVersion, Timestamp};

    use super::*;

    fn entry(tail: &str) -> SoulAuditEntry {
        SoulAuditEntry {
            schema_version: SchemaVersion,
            seq: 999,
            entry_id: format!("0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4{tail:0>3}")
                .parse::<Uuid>()
                .expect("hand-written UUIDv7 literal"),
            ts: Timestamp::new("2026-08-24T11:00:00Z"),
            prev_hash: Sha256Hex::new("f".repeat(64)),
            entry_hash: Sha256Hex::new("e".repeat(64)),
            action: AuditAction::MemoryWrite,
            decision: AuditDecision::Allowed,
            reason_code: None,
            subject_refs: None,
            counts: None,
            plan_hash: None,
            capability_token_id: None,
            egress_class: None,
        }
    }

    fn chain_of(count: usize) -> Vec<AuditLink> {
        let mut links: Vec<AuditLink> = Vec::new();
        for index in 0..count {
            let prev = links
                .last()
                .map(|l: &AuditLink| l.entry_hash.clone())
                .unwrap_or_else(|| GENESIS_PREV_HASH.to_owned());
            links.push(link(entry(&index.to_string()), index as u64, &prev).expect("link"));
        }
        links
    }

    #[test]
    fn a_freshly_linked_chain_verifies() {
        verify(&chain_of(4)).expect("chain");
    }

    #[test]
    fn the_caller_does_not_get_to_choose_the_links() {
        let linked = link(entry("1"), 0, GENESIS_PREV_HASH).expect("link");
        assert_eq!(linked.entry.seq, 0, "the store assigns seq");
        assert_eq!(linked.entry.prev_hash.as_str(), GENESIS_PREV_HASH);
        assert_ne!(linked.entry.entry_hash.as_str(), &"e".repeat(64));
    }

    #[test]
    fn editing_an_entry_after_the_fact_is_detected() {
        let mut links = chain_of(3);
        links[1].entry.decision = AuditDecision::Denied;
        assert!(matches!(
            verify(&links),
            Err(ChainError::ContentAltered { seq: 1, .. })
        ));
    }

    #[test]
    fn removing_a_link_is_detected() {
        let mut links = chain_of(3);
        links.remove(1);
        assert!(matches!(
            verify(&links),
            Err(ChainError::SeqOutOfOrder { .. })
        ));
    }

    #[test]
    fn a_rewritten_back_pointer_is_detected() {
        let mut links = chain_of(3);
        links[2].prev_hash = GENESIS_PREV_HASH.to_owned();
        assert!(matches!(
            verify(&links),
            Err(ChainError::BrokenLink { seq: 2, .. })
        ));
    }
}
