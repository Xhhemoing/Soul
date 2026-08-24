//! WP10 command surface: drafting, which never sends.
//!
//! Empty, and honestly so. ST-00 declared the module and registered
//! `soul-draft` as a dependency; ST-01 fills this file with the orchestration
//! WP10 needs — the store handle for the voice and the graph, the
//! `PolicySession` for redact → plan → token → generate, and the `DraftCreate`
//! audit entry at the end. Until then there is nothing here to call, which is
//! the accurate state of the work package.
