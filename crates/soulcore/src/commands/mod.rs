//! Command surface, one file per work package.
//!
//! The convention, so this does not turn into one large module later:
//!
//! * `store.rs`       WP02  opening the encrypted store, forgetting, research
//!                          preview
//! * `policy.rs`      WP08  action checks, capability tokens, redaction, and
//!                          the one E1 request path
//! * `import.rs`      WP06  soul-import-v1 and Telegram `result.json`
//! * `profile.rs`     WP03  trait axes, corrections, correction locking
//! * `memory.rs`      WP04  autobiographical memory and forgetting
//! * `graph.rs`       WP05  contacts and relationships
//! * `collect.rs`     WP07  foreground application duration
//! * `shell.rs`       WP09  what the desktop shell may display: the wizard,
//!                          the configuration summary, the cloud notice
//! * `draft.rs`       WP10  drafting, which never sends
//! * `fileplan.rs`    WP11  read-only scan and plan preview
//! * `session.rs`     WP13  the one open store, and the file beside it that
//!                          remembers the wizard and the authorized directories
//!
//! Each command must go through `soul-policy` for anything with an egress or
//! filesystem consequence, and must leave an audit entry that carries no prose.
//! WP08 landed that crate and `policy.rs` is the surface for it; `store.rs`
//! stays a pass-through, because forgetting and research preview are decided
//! inside `soul-store` and neither reaches the network. `import.rs`,
//! `graph.rs`, `profile.rs`, `memory.rs` and `collect.rs` are pass-throughs
//! too. The first appends the audit entries its crate hands back; the others
//! leave the audit to the crate underneath, which already writes it as part of
//! the same call. `shell.rs` reaches neither the network nor the disk: it answers
//! questions about this build's defaults, which is all a WebView is allowed to
//! ask in WP09. `fileplan.rs` reads the disk and never writes to it; it hands
//! its audit entries back the way `policy.rs` does, and it has no `execute`,
//! because the write half is v0.1.1. `session.rs` is the one exception to
//! "thin": it is where the process's single store handle is opened and where
//! the two answers that survive a restart are written, because somebody has to
//! own those and a shell that owned them would be a shell holding state.

pub mod collect;
pub mod draft;
pub mod fileplan;
pub mod graph;
pub mod import;
pub mod memory;
pub mod policy;
pub mod profile;
pub mod session;
pub mod shell;
pub mod store;
