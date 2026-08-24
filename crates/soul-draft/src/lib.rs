//! WP10: drafting that never sends, and people summaries that never claim more
//! than the evidence behind them.
//!
//! Two surfaces, one rule each, and both rules are types rather than checks a
//! caller could forget:
//!
//! | Promise | Where |
//! |---|---|
//! | v0.1 drafts, it does not send | [`draft::NeverSent`] has one inhabitant, and nothing here has a recipient |
//! | The prompt uses the voice the user set | [`brief::ProfileBrief::apply_inferred`] refuses a field the user pinned (AC-07) |
//! | With no key, drafting is a deterministic template | [`template::render`] is a pure function of the voice and one count (AC-17) |
//! | Third-party prose is placeheld before E1 | [`draft::Drafter::redact`] is the only body builder, and it goes through the redactor (AC-12) |
//! | One exemption covers one turn, once | The exemption is consumed by value and nothing holds state about it (AC-13) |
//! | External content is never instruction | The instruction slot is a constant in `soul-policy`; everything runtime lands in the material slot (AC-25) |
//! | Every summary point cites evidence | [`analysis::SummaryPoint::new`] refuses an empty list (AC-16) |
//! | Soul makes no medical claims | Every readable string goes through `soul_policy::assert_non_clinical` |
//!
//! ## What is not here
//!
//! No HTTP client, no store, and no way to reach either. The one seam to the
//! network is [`draft::ReplyGenerator`], which is handed a
//! [`RedactedBody`](soul_policy::RedactedBody) — a type only the redactor can
//! produce — and whose implementations live where the permits do. There is no
//! function in this crate that takes a URL.
//!
//! Nothing here reads the clock either. A draft is a function of the profile,
//! the conversation and, on the endpoint path, one answer; so the same inputs
//! produce the same draft, which is what makes the fallback testable.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod analysis;
pub mod brief;
pub mod draft;
pub mod error;
pub mod reply;
pub mod template;

pub use analysis::{
    phrase_with, render as render_summary, summarize_person, PersonSummary, SummaryPoint,
    SummarySource,
};
pub use brief::{AxisReading, ProfileBrief};
pub use draft::{
    Degradation, Draft, DraftRequest, DraftSource, Drafter, NeverSent, ReplyGenerator,
    DEGRADED_NOTICE, ENDPOINT_NOTICE, NOT_SENT_NOTICE, TEMPLATE_NOTICE,
};
pub use error::{DraftError, DraftResult, GenerationRefused};
pub use reply::{ModelReply, ReplyDefect};
pub use template::{render as render_template, TemplateContext, BODY_SLOT};
