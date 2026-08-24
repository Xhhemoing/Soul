//! WP06's command surface: read one of the two v0.1 export formats, put it in
//! the store, or fall back to the questionnaire when there is no file.
//!
//! Reading and committing are separate calls because they are separate
//! decisions. [`read_soul_import_v1`] and [`read_telegram`] touch nothing; a
//! caller can show the user what a file contains, and what is wrong with it,
//! before anything is written. [`commit`] is the step that writes.
//!
//! Nothing here interprets an imported message. Bodies arrive as
//! `UntrustedText`, are sealed, and are pointed at; the injection scan that
//! runs during a commit feeds an audit entry and decides nothing. This module
//! never builds a tool plan, and no code path in it reads what was imported in
//! order to choose what to do next.

use serde_json::Value;

use soul_import::commit::{ImportError, ImportReceipt};
use soul_import::defect::ImportFailure;
use soul_import::model::StagedImport;
use soul_policy::audit::append_or_store_error;
use soul_store::SqlCipherStore;

/// Parse a `soul-import-v1` JSONL file. Writes nothing.
pub fn read_soul_import_v1(text: &str) -> Result<StagedImport, ImportFailure> {
    soul_import::soul_import_v1::parse(text)
}

/// Parse a Telegram Desktop `result.json` document. Writes nothing.
///
/// The caller supplies the parsed JSON. This crate does not open files, and
/// v0.1 does not sniff archives: the user points at the `result.json` that
/// Telegram's own *Export chat history* produced.
pub fn read_telegram(document: &Value) -> Result<StagedImport, ImportFailure> {
    soul_import::telegram::parse(document)
}

/// Write a parsed import into the encrypted store.
///
/// Every audit entry the commit produced is appended here, including the
/// `injection.blocked` one when the export carried an attempt — the user is
/// told that their file tried something and that it went nowhere.
pub fn commit(
    store: &mut SqlCipherStore,
    staged: &StagedImport,
    at_unix_seconds: i64,
) -> Result<ImportReceipt, ImportError> {
    let receipt = soul_import::commit::commit(store, staged)?;
    for content in receipt.audit.clone() {
        append_or_store_error(store, content, at_unix_seconds)?;
    }
    Ok(receipt)
}

/// Is the questionnaire the path to take?
pub fn questionnaire_needed(staged: Option<&StagedImport>) -> bool {
    soul_import::questionnaire::fallback_needed(staged)
}

/// The questions this build asks, for a UI that has to draw them.
///
/// The same list [`crate::commands::profile::questions`] returns, paired there
/// with what each one moves. There is one questionnaire in v0.1 and it is
/// asked once: whichever path the user came down, the answers go in through
/// [`crate::commands::profile::intake`], which records them and builds the
/// profile in one call.
pub fn questions() -> &'static [soul_import::questionnaire::Question] {
    soul_import::questionnaire::QUESTIONS
}
