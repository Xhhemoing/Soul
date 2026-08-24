//! Command surface, one file per work package.
//!
//! Nothing is wired yet. The convention, so this does not turn into one large
//! module later:
//!
//! * `import.rs`      WP06  soul-import-v1 and Telegram `result.json`
//! * `profile.rs`     WP03  trait axes, corrections, correction locking
//! * `memory.rs`      WP04  autobiographical memory and forgetting
//! * `graph.rs`       WP05  contacts and relationships
//! * `collect.rs`     WP07  foreground application duration
//! * `draft.rs`       WP10  drafting, which never sends
//! * `fileplan.rs`    WP11  read-only scan and plan preview
//! * `research.rs`    WP02  research preview, which never writes a file
//!
//! Each command must go through `soul-policy` for anything with an egress or
//! filesystem consequence, and must leave an audit entry that carries no prose.
