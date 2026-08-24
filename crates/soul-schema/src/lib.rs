//! Soul data contracts.
//!
//! This crate is the single source of truth for the shapes that cross a Soul
//! module boundary. It performs no IO and opens no sockets: the JSON Schema
//! documents under `docs/schemas/` are embedded at compile time.
//!
//! One module per schema document, plus [`common`] for the shared `_defs`
//! vocabulary and [`validate`] for the draft 2020-12 validators.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod audit;
pub mod common;
pub mod contact;
pub mod event;
pub mod evidence;
pub mod export_manifest;
pub mod inference;
pub mod memory;
pub mod profile;
pub mod relationship;
pub mod soul_import_v1;
pub mod validate;

pub use common::{
    EvidenceBand, Privacy, SchemaVersion, SealedText, Sha256Hex, Subject, Timestamp,
};
pub use validate::{SchemaId, SchemaSet, ValidationFailure};
