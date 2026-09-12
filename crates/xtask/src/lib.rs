//! Repository guardrails, kept as library functions so they can be tested.
//!
//! Each audit is a pure function over a directory tree or a dependency graph.
//! `src/main.rs` is only argument parsing and exit codes; `tests/self_test.rs`
//! points the same functions at synthetic trees to prove they actually fail
//! when they should.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod denylist;
pub mod egress;
pub mod sbom;
pub mod schema_freeze;

use std::path::{Path, PathBuf};

/// The repository root, derived from this crate's manifest location.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/xtask sits two levels below the repository root")
        .to_path_buf()
}
