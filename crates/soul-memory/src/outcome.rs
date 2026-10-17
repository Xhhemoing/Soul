//! A committed forget and its two independent follow-up results.
//!
//! Destruction belongs to the store. Once it commits, neither a pending WAL
//! cleanup nor an unconfirmed audit append may turn its receipt into an error
//! that invites the caller to perform the destructive operation again.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_policy::audit::{append_or_store_error, AuditContent, ReasonCode};
use soul_schema::audit::{AuditAction, AuditCounts, AuditDecision};
use soul_store_api::forget::{ForgetCleanup, ForgetOutcomeOps, ForgetReceipt, ForgetUnit};
use soul_store_api::{AuditLog, MemoryStore};

use crate::MemoryResult;

/// An append error does not prove that no audit row was written. In
/// particular, the store can fail to acknowledge an already-durable append.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForgetAudit {
    Recorded,
    Unconfirmed,
}

/// This value exists only after logical destruction committed. Cleanup and
/// audit are separate: successfully retrying cleanup cannot repair or confirm
/// an audit append. No underlying error text, prose, or paths cross this API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryForgetOutcome {
    pub receipt: ForgetReceipt,
    pub cleanup: ForgetCleanup,
    pub audit: ForgetAudit,
}

/// Forget one existing memory, retaining its receipt after commit.
///
/// Err belongs to resolving the memory or obtaining a committed store
/// outcome. The audit is attempted exactly once after that, even when cleanup
/// is pending. An unconfirmed append must not trigger an automatic retry.
pub fn forget_with_outcome<S>(
    store: &mut S,
    memory_id: Uuid,
    now_unix_seconds: i64,
) -> MemoryResult<MemoryForgetOutcome>
where
    S: MemoryStore + ForgetOutcomeOps + AuditLog,
{
    store.get_memory(memory_id)?;
    let outcome = store.execute_forget_outcome(ForgetUnit::Memory(memory_id))?;

    let content = AuditContent::new(AuditAction::ForgetExecute, AuditDecision::Allowed)
        .because(ReasonCode::Routine)
        .about(&[memory_id])
        .counting(AuditCounts {
            items: Some(outcome.receipt.impact.content_key_ids.len() as u64),
            bytes: None,
        });
    let audit = match append_or_store_error(store, content, now_unix_seconds) {
        Ok(_) => ForgetAudit::Recorded,
        Err(_) => ForgetAudit::Unconfirmed,
    };

    Ok(MemoryForgetOutcome {
        receipt: outcome.receipt,
        cleanup: outcome.cleanup,
        audit,
    })
}

/// Only retry WAL cleanup. No memory id, destructive confirmation, audit
/// operation, or claim about an earlier audit is part of this call. This is
/// also available after reopening the store, without an in-memory receipt.
pub fn retry_forget_cleanup<S: ForgetOutcomeOps>(store: &mut S) -> ForgetCleanup {
    store.retry_forget_cleanup()
}
