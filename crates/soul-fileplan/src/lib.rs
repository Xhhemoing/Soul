//! Read-only directory scan and file-organisation plan preview.
//!
//! This is the agent layer's whole footprint on the filesystem in v0.1, and it
//! is a footprint that leaves no mark. PRODUCT_LOCK's slice item 9 asks for one
//! thing to be demonstrated — that Soul can be pointed at a directory, say what
//! tidying it would mean, and refuse everything else — and DECISIONS D31 is
//! explicit that this stays a proof of the read-only path rather than becoming
//! a feature the product is sold on. The write half is AC-27, in v0.1.1.
//!
//! What that means in code:
//!
//! | Promise | Where it lives |
//! |---|---|
//! | Nothing is authorized until the user says so | [`Authorization::new`], which contains no roots |
//! | An unauthorized path is refused, however it is spelled | [`screen`] on the string, then [`Authorization::resolve`] twice over |
//! | Symbolic links are not followed, in or out | [`Authorization::resolve`] and the walk in [`scan`] |
//! | The scan changes nothing | [`scan::DirectorySnapshot`] taken before and after |
//! | The plan is a proposal | [`plan::OrganizePlan`] has no method that acts |
//! | Asking to execute it is refused | [`execute::refuse_execution`], whose return type has no success variant |
//!
//! There is no write API in this crate, and that is checked rather than
//! remembered: `tests/no_write_api.rs` reads every source file back and looks
//! for one, and has a control case proving the search works.
//!
//! Nothing here reaches storage either. The crate does not depend on
//! `soul-store`, so a scan cannot become collected file metadata — which
//! PRODUCT_LOCK also defers to v0.1.1. A preview lives in memory until whoever
//! asked for it renders it, and the only thing that outlives it is an audit
//! entry carrying a count and a hash.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod authorize;
pub mod execute;
pub mod kind;
pub mod plan;
pub mod preview;
pub mod refusal;
pub mod scan;
pub mod screen;

pub use authorize::{Authorization, AuthorizedRoot, PathMatching, Resolution};
pub use execute::{refuse_execution, ExecutionRefusal, FILEPLAN_ACTIONS};
pub use kind::FileKind;
pub use plan::{LeaveReason, LeftAlone, OrganizePlan, ProposedMove};
pub use preview::{preview, Preview};
pub use refusal::Refusal;
pub use scan::{
    DirectoryScan, DirectorySnapshot, ScanLimits, ScannedEntry, SkipReason, SkippedEntry,
    DEFAULT_MAX_DEPTH, DEFAULT_MAX_ENTRIES,
};
pub use screen::{screen, PathDefect, ScreenedPath};
