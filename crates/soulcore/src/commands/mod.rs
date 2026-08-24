//! Command surface, one file per work package.
//!
//! The convention, so this does not turn into one large module later:
//!
//! * `store.rs`       WP02  opening the encrypted store, forgetting, research
//!                          preview
//! * `import.rs`      WP06  soul-import-v1 and Telegram `result.json`
//! * `profile.rs`     WP03  trait axes, corrections, correction locking
//! * `memory.rs`      WP04  autobiographical memory and forgetting
//! * `graph.rs`       WP05  contacts and relationships
//! * `collect.rs`     WP07  foreground application duration
//! * `draft.rs`       WP10  drafting, which never sends
//! * `fileplan.rs`    WP11  read-only scan and plan preview
//!
//! Each command must go through `soul-policy` for anything with an egress or
//! filesystem consequence, and must leave an audit entry that carries no prose.
//! `soul-policy` is WP08; until it exists, `store.rs` is a pass-through and
//! says so.

pub mod store;
