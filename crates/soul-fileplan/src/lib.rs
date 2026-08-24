//! WP11: a read-only scan of an authorized root, and a plan preview nothing
//! applies.
//!
//! **This crate is empty on purpose.** ST-00 registered it as a workspace
//! member with the dependency set WP11 was scoped against; ST-02 writes the
//! logic. Nothing here pretends to work yet, so there is no `todo!()` waiting
//! to be discovered at runtime and no type that looks implemented from the
//! outside.
//!
//! What ST-02 puts here, and what it must keep out:
//!
//! * authorization decided against roots passed in as arguments
//!   (canonicalized, then prefix-checked), a `walkdir` traversal that only
//!   reads, and a plan preview whose every field is a suggestion;
//! * no write API of any kind, and no execution. v0.1 previews and stops;
//!   applying a plan is v0.1.1. A test reads these sources back to check that
//!   promise, because a promise about what code does *not* do cannot be
//!   demonstrated by running it.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

/// The work package this crate belongs to, so the module is not literally
/// empty while it waits for ST-02.
pub const WORK_PACKAGE: &str = "WP11";
