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
