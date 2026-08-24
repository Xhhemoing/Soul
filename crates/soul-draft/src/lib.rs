//! WP10: drafting that never sends, and the people summary that informs it.
//!
//! **This crate is empty on purpose.** ST-00 registered it as a workspace
//! member with the dependency set WP10 was scoped against; ST-01 writes the
//! logic. Nothing here pretends to work yet, so there is no `todo!()` waiting
//! to be discovered at runtime and no type that looks implemented from the
//! outside.
//!
//! What ST-01 puts here, and what it must keep out:
//!
//! * pasted text turned into `soul_policy::redactor::Turn` values, a
//!   `soul_profile::voice::VoiceProfile` turned into a deterministic tone
//!   template, and the counting side of the people summary over
//!   `soul-graph` ties — all pure, none of it touching the disk;
//! * no request path. The one E1 call belongs to
//!   `soulcore::commands::draft`, which holds the store handle and the
//!   `PolicySession`. `soul-egress` is not a dependency of this crate, and the
//!   public surface must not grow a name that suggests sending, submitting or
//!   delivering anything.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

/// The work package this crate belongs to, so the module is not literally
/// empty while it waits for ST-01.
pub const WORK_PACKAGE: &str = "WP10";
