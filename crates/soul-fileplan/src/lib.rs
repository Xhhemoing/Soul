//! WP11: a read-only scan of an authorized directory, and a plan preview
//! nothing carries out.
//!
//! Two entry points, [`scan`] and [`plan`], and one type that stands between
//! a request and the disk: [`AuthorizedRoots`]. Everything the user
//! authorized arrives as an argument — this crate reads no configuration,
//! holds no database handle, and opens no socket — so what may be looked at
//! is decided by the caller and enforced here, in one place, in one order.
//!
//! What is deliberately absent:
//!
//! * any write API. No call that creates, truncates, removes, moves or
//!   re-permissions anything. `tests/no_write_api.rs` reads these sources
//!   back and fails on the vocabulary of writing, because a promise about
//!   what code does *not* do cannot be demonstrated by running it;
//! * any execution. There is no `execute`, `apply` or `undo` function to
//!   find. Carrying a plan out is v0.1.1, and until it exists a preview is
//!   the whole product;
//! * any capability token. `ScanDirectory` and `PlanFiles` do not need one,
//!   so a token type appearing in here would mean something had gone wrong.
//!
//! Orchestration — the action check and the audit entry — lives in
//! `soulcore::commands::fileplan`. This crate decides what is on disk; that
//! module decides whether anyone was allowed to ask.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod authorize;
pub mod error;
pub mod plan;
pub mod scan;

pub use authorize::{AuthorizedRoots, AuthorizedTarget};
pub use error::FilePlanError;
pub use plan::{plan, FilePlanPreview, PlanAction, PlanEntry};
pub use scan::{scan, EntryKind, ScanEntry, ScanReport, FILE_NAME_CHANNEL};

/// The work package this crate belongs to.
pub const WORK_PACKAGE: &str = "WP11";
