//! WP02's command surface: open the encrypted store, preview a forget, execute
//! one, and describe what research would see.
//!
//! Deliberately thin. Everything that decides anything lives in `soul-store`;
//! what is here is the shape a caller sees, so the desktop shell in WP09 has
//! something to bind to that is not a database handle. There is no UI, no
//! policy, and no formatting: WP08 owns the audit and permission decisions that
//! will wrap these calls, and each of them is a separate work package.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use soul_store_api::forget::{ForgetImpact, ForgetOps, ForgetReceipt, ForgetUnit};
use soul_store_api::research::{ResearchPreview, ResearchPreviewReport, ResearchPreviewRequest};
use soul_store_api::types::StoreResult;

use crate::commands::collect::share;
use crate::commands::shell::KeyProtection;

/// The storage types a host needs to name, re-exported so it does not have to
/// depend on `soul-store` itself.
///
/// `apps/desktop/src-tauri` has one path dependency and it is this crate. A
/// second one would be a second tree outside the reach of `deny.toml` and
/// `xtask e0-audit`, added by whoever was in a hurry to reach a type.
pub use soul_store::{
    DpapiKeyProvider, KeyError, KeyProvider, KeyResult, SqlCipherStore, TestKeyProvider,
};

/// File name of the main database inside whichever directory holds it.
///
/// On Windows that directory is `%LOCALAPPDATA%\Soul`; resolving it is WP09's
/// job, because it is a shell concern and this crate must stay runnable
/// headless on a Linux CI host.
pub const DATABASE_FILE_NAME: &str = "soul.db";

/// Where the DPAPI-protected seed would live once that provider works.
///
/// Nothing writes this file in v0.1. The name exists so [`open_store_for_session`]
/// can ask the platform provider the same question a finished implementation
/// would answer, instead of deciding by `cfg!(windows)` that the answer is no.
pub const DPAPI_BLOB_FILE_NAME: &str = "keys.dpapi";

pub fn database_path(directory: impl AsRef<Path>) -> PathBuf {
    directory.as_ref().join(DATABASE_FILE_NAME)
}

pub fn dpapi_blob_path(directory: impl AsRef<Path>) -> PathBuf {
    directory.as_ref().join(DPAPI_BLOB_FILE_NAME)
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

/// Open the store under a key file kept beside the database.
///
/// The seed is written in the clear by `TestKeyProvider::in_dir`, so the key
/// and the thing it opens end up in the same directory. That is not a design;
/// it is what is left when the platform provider cannot answer, and the caller
/// is told so through [`KeyProtection::UnprotectedKeyFile`] rather than being
/// allowed to assume otherwise.
pub fn open_store_with_test_file_keys(directory: impl AsRef<Path>) -> StoreResult<SqlCipherStore> {
    let directory = directory.as_ref();
    open_store(directory, &TestKeyProvider::in_dir(directory))
}

/// The one store handle a process gets, and the truth about its key.
#[derive(Debug)]
pub struct SessionStore {
    handle: Arc<Mutex<SqlCipherStore>>,
    key_protection: KeyProtection,
}

impl SessionStore {
    /// A clone of the shared handle. Clone it as often as you like; opening a
    /// second store is what this type exists to prevent, because two
    /// connections write two write-ahead logs.
    pub fn handle(&self) -> Arc<Mutex<SqlCipherStore>> {
        Arc::clone(&self.handle)
    }

    pub fn into_handle(self) -> Arc<Mutex<SqlCipherStore>> {
        self.handle
    }

    /// Where the key came from, as observed at open time.
    pub fn key_protection(&self) -> KeyProtection {
        self.key_protection
    }
}

/// Open the single store a session runs on, and choose the key provider here.
///
/// Choosing a provider is a security decision, so it lives in this crate and
/// not in the shell: a policy written in the UI layer is a policy that can be
/// changed by a pull request nobody reviews as a data-plane change.
///
/// The platform provider is asked first and every time. It refuses today —
/// both `DpapiKeyProvider` accessors return `KeyError::Unsupported` until the
/// Win32 binding lands — so what actually opens the database is a plain key
/// file, and the caller receives that fact rather than a boolean it could
/// default the wrong way.
pub fn open_store_for_session(directory: impl AsRef<Path>) -> StoreResult<SessionStore> {
    let directory = directory.as_ref();
    let platform = DpapiKeyProvider::new(dpapi_blob_path(directory));

    match platform.key_encryption_key() {
        Ok(_) => Ok(SessionStore {
            handle: share(open_store(directory, &platform)?),
            key_protection: KeyProtection::PlatformKeyStore,
        }),
        Err(_) => Ok(SessionStore {
            handle: share(open_store_with_test_file_keys(directory)?),
            key_protection: KeyProtection::UnprotectedKeyFile,
        }),
    }
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
