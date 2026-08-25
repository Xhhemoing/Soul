//! WP10: drafting that never goes out, and the people summary that informs it.
//!
//! Everything here is pure. There is no store handle, no clock, no socket and
//! no HTTP client anywhere in this crate's dependency graph — the one
//! generation request WP10 makes belongs to `soulcore::commands::draft`, which
//! holds the `PolicySession` and calls `soul-egress` through it. A drafting
//! crate that could reach the network would be a drafting crate that could put
//! a draft on the wire, so this one cannot.
//!
//! What that leaves here:
//!
//! * [`PastedTurn`], the only way pasted prose enters, and the audit entries
//!   the injection scan owes ([`turns`]);
//! * [`template_draft`], the deterministic draft this product writes when no
//!   endpoint is configured, and [`tone_turn`], the one line that carries the
//!   user's voice into a request body ([`tone`]);
//! * [`answer_text`], which reads an endpoint's answer as data ([`answer`]);
//! * [`summarize`], the counting side of a people summary, where every claim
//!   carries the evidence ids its edge cites ([`summary`]).
//!
//! The public surface has no method that hands a draft to anything, and no
//! field that says a draft is ready to go out. `tests/no_send_api.rs` reads
//! this crate's sources back and checks that it stays that way.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod answer;
pub mod error;
pub mod summary;
pub mod tone;
pub mod turns;

pub use answer::answer_text;
pub use error::{DraftError, DraftResult};
pub use summary::{summarize, PeopleSummary, ResolvedTie, SummaryClaim};
pub use tone::{
    check_draft, template_draft, tone_directive, tone_turn, voice_plan, DraftOutcome, DraftRoute,
    DraftStats,
};
pub use turns::{injection_audit, injection_signals, turns_from, PastedTurn};

/// The work package this crate belongs to.
pub const WORK_PACKAGE: &str = "WP10";
