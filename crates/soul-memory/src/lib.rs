//! Autobiographical memory.
//!
//! `docs/DECISIONS.md` D20 makes memory a gate for v0.1, and D15 defines what
//! deleting one means: destroy the content key. This crate is the shape of a
//! memory on top of the storage boundary — a title and a summary sealed under
//! one key per memory, plus the CRUD around them — and nothing more. It holds
//! no database, no cipher and no forget transaction of its own; all three
//! belong to `soul-store`, and reimplementing any of them here would give the
//! product two answers to "is this text still readable?".
//!
//! What the split leaves this crate responsible for:
//!
//! * every piece of prose goes through [`soul_store_api::BlobStore`], so there
//!   is no path by which a memory's words reach a plain column;
//! * one content key per memory, minted at creation and reused by every edit,
//!   so a memory is exactly one forget unit;
//! * an audit entry for every write, carrying ids and counts and no text, and
//!   written *after* a forget rather than before it.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod draft;
pub mod error;
pub mod outcome;
pub mod service;

pub use draft::{
    MemoryContent, MemoryDigest, MemoryDraft, MemoryEdit, DEFAULT_PLACEHOLDER, SUMMARY_FIELD,
    TITLE_FIELD,
};
pub use error::{MemoryError, MemoryResult};
pub use outcome::{forget_with_outcome, retry_forget_cleanup, ForgetAudit, MemoryForgetOutcome};
pub use service::{create, forget, list, preview_forget, read, update};
