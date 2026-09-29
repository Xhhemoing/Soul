//! Forgetting, expressed as key destruction.
//!
//! PRODUCT_LOCK does not promise physical erasure of SSD blocks. What it does
//! promise is that after `execute_forget` the content key is gone, so the
//! sealed prose can no longer be opened, and anything derived from it is
//! marked `orphaned` rather than quietly kept.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::StoreResult;

/// What the user asked to forget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "unit", content = "id")]
pub enum ForgetUnit {
    Memory(Uuid),
    Contact(Uuid),
    /// Every row sealed under one key, addressed directly.
    ContentKey(Uuid),
}

impl ForgetUnit {
    pub fn id(self) -> Uuid {
        match self {
            ForgetUnit::Memory(id) | ForgetUnit::Contact(id) | ForgetUnit::ContentKey(id) => id,
        }
    }
}

/// What forgetting this unit would cost, shown to the user before they commit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetImpact {
    /// Keys that would be destroyed.
    pub content_key_ids: Vec<Uuid>,
    pub memories_affected: u64,
    pub contacts_affected: u64,
    pub sealed_blobs_destroyed: u64,
    /// Inferences that would lose their evidence and become `orphaned`.
    pub inferences_orphaned: u64,
    /// Audit entries that mention the unit and are deliberately kept. The
    /// audit chain carries no prose, and it must never block a forget.
    pub audit_entries_retained: u64,
}

/// Proof of what a completed forget actually did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetReceipt {
    pub unit: ForgetUnit,
    pub impact: ForgetImpact,
}

pub trait ForgetOps {
    /// Read-only. Must not destroy anything.
    fn preview_impact(&self, unit: ForgetUnit) -> StoreResult<ForgetImpact>;

    /// Destroys the content keys the preview named, then marks the derived
    /// inferences `orphaned`.
    fn execute_forget(&mut self, unit: ForgetUnit) -> StoreResult<ForgetReceipt>;
}

/// All three integers returned by `PRAGMA wal_checkpoint(TRUNCATE)`.
/// A successful SQL query is not proof that the log was truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalCheckpoint {
    pub busy: i64,
    pub log_frames: i64,
    pub checkpointed_frames: i64,
}

impl WalCheckpoint {
    /// TRUNCATE reports three zeroes on completion. In particular, a busy
    /// checkpoint may have copied every frame without truncating the file.
    /// Negative frame counts mean no WAL was observed, not verified cleanup.
    pub fn confirms_truncate(self) -> bool {
        self.busy == 0 && self.log_frames == 0 && self.checkpointed_frames == 0
    }
}

/// Cleanup after logical destruction committed. No variant promises physical
/// erasure of SSD blocks. Pending must never be presented as a rolled-back
/// forget: the live database has already lost the keys, but old WAL frames
/// may still carry their wrapped bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ForgetCleanup {
    Complete,
    Pending {
        /// None means the checkpoint query itself could not be confirmed.
        /// Only counts cross this boundary, never raw database error text.
        checkpoint: Option<WalCheckpoint>,
    },
}

impl ForgetCleanup {
    pub fn from_checkpoint(checkpoint: WalCheckpoint) -> Self {
        if checkpoint.confirms_truncate() {
            Self::Complete
        } else {
            Self::Pending {
                checkpoint: Some(checkpoint),
            }
        }
    }
}

/// A committed logical operation, even when cleanup remains pending.
/// The receipt continues to describe the original operation, not a second
/// preview calculated after its rows were already destroyed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgetOutcome {
    pub receipt: ForgetReceipt,
    pub cleanup: ForgetCleanup,
}

/// An explicit reporting extension. Existing backends implementing ForgetOps
/// do not silently acquire a default that claims their cleanup completed.
pub trait ForgetOutcomeOps: ForgetOps {
    /// Err means no committed receipt was obtained. Once commit succeeds,
    /// cleanup problems are returned in Ok(ForgetOutcome), not as a retryable
    /// destruction error. The audit belongs to the caller and follows this.
    fn execute_forget_outcome(&mut self, unit: ForgetUnit) -> StoreResult<ForgetOutcome>;

    /// Retry only WAL cleanup. This takes no forget unit or confirmation and
    /// must neither destroy keys nor append an audit entry. It is safe to
    /// repeat, including after reopening the store.
    fn retry_forget_cleanup(&mut self) -> ForgetCleanup;
}

#[cfg(test)]
mod outcome_tests {
    use super::{ForgetCleanup, WalCheckpoint};

    #[test]
    fn only_a_confirmed_truncate_is_complete() {
        let complete = WalCheckpoint {
            busy: 0,
            log_frames: 0,
            checkpointed_frames: 0,
        };
        assert!(complete.confirms_truncate());
        assert_eq!(ForgetCleanup::from_checkpoint(complete), ForgetCleanup::Complete);
        for (busy, log_frames, checkpointed_frames) in [
            (1, 5, 3),
            (1, 5, 5),
            (1, 0, 0),
            (0, 5, 5),
            (0, -1, -1),
            (2, 0, 0),
            (0, 0, -1),
        ] {
            let observed = WalCheckpoint {
                busy,
                log_frames,
                checkpointed_frames,
            };
            assert!(!observed.confirms_truncate(), "{observed:?}");
            assert_eq!(
                ForgetCleanup::from_checkpoint(observed),
                ForgetCleanup::Pending {
                    checkpoint: Some(observed),
                }
            );
        }
    }
}
