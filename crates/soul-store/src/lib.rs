//! Soul's encrypted main database.
//!
//! This is the only crate that puts business data on disk, and it is the real
//! backend behind [`soul_store_api`]: `tests/conformance_real.rs` runs the same
//! `run_conformance` suite that `FakeStore` passes, so the storage boundary
//! cannot quietly mean two different things.
//!
//! What is load-bearing here, in the order `docs/SECURITY.md` states it:
//!
//! * the file is a SQLCipher database, keyed with the DEK, and [`SqlCipherStore::open`]
//!   refuses to continue against a plain SQLite build rather than writing pages
//!   in the clear;
//! * prose is sealed a second time with XChaCha20-Poly1305 under a
//!   per-forget-unit content key, with `row_id|field` as additional
//!   authenticated data, and the content key itself is stored wrapped under the
//!   KEK;
//! * forgetting deletes that wrapped key, which is why it is irreversible
//!   without claiming anything about SSD blocks;
//! * the audit chain is written by the store, carries no prose, and is never
//!   touched by a forget.
//!
//! Nothing in this crate speaks HTTP, and the research surface
//! ([`research_preview`]) never puts anything in a file.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod audit;
pub mod forget;
pub mod keys;
pub mod research_preview;
pub mod sql;
pub mod store;

pub use audit::{AuditLink, ChainError, GENESIS_PREV_HASH};
pub use keys::{DpapiKeyProvider, KeyError, KeyProvider, KeyResult, SecretKey, TestKeyProvider};
pub use store::SqlCipherStore;

use soul_store_api::types::StoreResult;
use soul_store_api::SoulStore;

/// Names of the crash-injection sites this crate arms.
///
/// They are duplicated from `soul_testkit::crash::failpoints` because the test
/// kit is a development dependency and must never be reachable from shipped
/// code. `tests/crash_recovery.rs` asserts the two lists agree, so the copy
/// cannot drift.
pub mod failpoints {
    /// Inside [`soul_store_api::EventStore::append_event`], after the row is
    /// written and before the transaction commits.
    pub const STORE_EVENT_COMMIT_MID: &str = "soul::store::event::commit_mid";

    /// Inside [`soul_store_api::ForgetOps::execute_forget`], between destroying
    /// one content key and the next.
    pub const FORGET_CK_DELETE_MID: &str = "soul::forget::content_key::delete_mid";

    /// Inside [`soul_store_api::AuditLog::append_audit`], after the linked
    /// entry has been written and before its transaction commits.
    pub const AUDIT_APPEND_PRE_COMMIT: &str = "soul::audit::append::pre_commit";

    /// Inside [`soul_store_api::AuditLog::append_audit`], after the commit and
    /// before the caller is told the entry landed.
    pub const AUDIT_APPEND_POST_WRITE: &str = "soul::audit::append::post_write";
}

impl SoulStore for SqlCipherStore {
    /// Push the write-ahead log into the main file. Callers that are about to
    /// inspect the database bytes need this; ordinary writes are already
    /// durable, because every one of them commits.
    fn flush(&mut self) -> StoreResult<()> {
        self.checkpoint()
    }
}
