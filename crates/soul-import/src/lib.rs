//! The two ways data gets into Soul in v0.1, and what happens when there is
//! neither.
//!
//! PRODUCT_LOCK names exactly two importers — `soul-import-v1` JSONL and the
//! `result.json` from Telegram Desktop's *Export chat history → Machine-readable
//! JSON* — and one fallback, the questionnaire. There is no OAuth, no scraping,
//! and no generic archive sniffing: a format Soul cannot name is a format Soul
//! cannot promise anything about.
//!
//! ```text
//! parse ──▶ StagedImport ──▶ commit ──▶ contacts + sealed bodies
//!                                       + events + interaction evidence
//! ```
//!
//! [`soul_import_v1::parse`] and [`telegram::parse`] each produce a
//! [`model::StagedImport`]; [`commit::commit`] is the only thing that writes.
//! Splitting it that way is what lets a malformed file be refused *before*
//! anything lands, and it is why every acceptance test can look at the staged
//! form without a database.
//!
//! Two properties hold across all of it.
//!
//! **External content is data.** A message body is an [`UntrustedText`] from
//! the moment it is read until it is sealed. That type has no `Display`, so it
//! cannot be interpolated into an instruction by accident, and nothing in this
//! crate branches on what a body says. Bodies are scanned once, for the
//! `injection.blocked` audit entry, and the result changes nothing.
//!
//! **A rejection does not quote the file.** Every message a failure carries is
//! built from field names and schema constraints and then passed through
//! [`redact::ContentGuard`], which replaces anything that repeats what someone
//! wrote. See [`defect`].
//!
//! [`UntrustedText`]: soul_policy::injection::UntrustedText

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod commit;
pub mod defect;
pub mod instant;
pub mod model;
pub mod questionnaire;
pub mod redact;
pub mod soul_import_v1;
pub mod telegram;

pub use commit::{commit, ImportError, ImportReceipt};
pub use defect::{Defect, ImportFailure, Locator};
pub use model::{ImportSource, ParticipantHandle, StagedImport, StagedMessage, StagedParticipant};
pub use questionnaire::{Answer, Question, QuestionnaireReceipt, RecordedAnswer, UserStatedSink};
