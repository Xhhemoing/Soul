//! WP02's command surface: open the encrypted store, preview a forget, execute
//! one, and describe what research would see.
//!
//! Deliberately thin. Everything that decides anything lives in `soul-store`;
//! what is here is the shape a caller sees, so the desktop shell in WP09 has
//! something to bind to that is not a database handle. There is no UI, no
//! policy, and no formatting: WP08 owns the audit and permission decisions that
//! will wrap these calls, and each of them is a separate work package.

use std::path::{Path, PathBuf};

use soul_store::{KeyProvider, SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::{ForgetImpact, ForgetOps, ForgetReceipt, ForgetUnit};
use soul_store_api::research::{ResearchPreview, ResearchPreviewReport, ResearchPreviewRequest};
use soul_store_api::types::StoreResult;

/// File name of the main database inside whichever directory holds it.
///
/// On Windows that directory is `%LOCALAPPDATA%\Soul`; resolving it is WP09's
/// job, because it is a shell concern and this crate must stay runnable
/// headless on a Linux CI host.
pub const DATABASE_FILE_NAME: &str = "soul.db";

pub fn database_path(directory: impl AsRef<Path>) -> PathBuf {
    directory.as_ref().join(DATABASE_FILE_NAME)
}

/// Open the store under a caller-supplied key provider.
pub fn open_store(
    directory: impl AsRef<Path>,
    keys: &dyn KeyProvider,
) -> StoreResult<SqlCipherStore> {
    SqlCipherStore::open(database_path(directory), keys)
}

/// Open the store with test key material.
///
/// The name says what it is for. `TestKeyProvider` touches no platform key
/// store, so this is the entry point headless tests and Linux CI use; a real
/// installation goes through [`open_store`] with the platform provider.
pub fn open_test_store(directory: impl AsRef<Path>, seed: &str) -> StoreResult<SqlCipherStore> {
    let keys = TestKeyProvider::from_seed(seed);
    open_store(directory, &keys)
}

/// What forgetting `unit` would cost. Read-only; destroys nothing.
pub fn preview_forget(store: &SqlCipherStore, unit: ForgetUnit) -> StoreResult<ForgetImpact> {
    store.preview_impact(unit)
}

/// Destroy the content keys the preview named.
///
/// Irreversible. The caller is responsible for having shown
/// [`preview_forget`] first; the receipt repeats the impact so the two can be
/// compared after the fact.
pub fn execute_forget(store: &mut SqlCipherStore, unit: ForgetUnit) -> StoreResult<ForgetReceipt> {
    store.execute_forget(unit)
}

/// Describe what the research track would see. Writes nothing, anywhere.
pub fn research_preview(
    store: &SqlCipherStore,
    request: &ResearchPreviewRequest,
) -> StoreResult<ResearchPreviewReport> {
    store.research_preview(request)
}
