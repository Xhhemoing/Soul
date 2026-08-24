//! WP02's command surface: open the encrypted store, preview a forget, execute
//! one, and describe what research would see.
//!
//! Deliberately thin. Everything that decides anything lives in `soul-store`;
//! what is here is the shape a caller sees, so the desktop shell in WP09 has
//! something to bind to that is not a database handle. There is no UI, no
//! policy, and no formatting: WP08 owns the audit and permission decisions that
//! will wrap these calls, and each of them is a separate work package.
//!
//! [`StoreSlot`] arrived with the WP09 views and is the exception that proves
//! the rule: a host manages one before it has opened anything, so a build that
//! never opens a database — the mock runtime every desktop test runs under —
//! answers "no store" instead of panicking on state that was never inserted.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

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
    DpapiKeyProvider, KeyError, KeyProvider, KeyResult, SecretKey, SqlCipherStore, TestKeyProvider,
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

/// Where a host puts the one store handle, and what it holds before that.
///
/// Empty is a state, not a failure: a process that has not opened a database
/// yet is the ordinary condition of the mock runtime, and a view that asks for
/// one gets a sentence rather than a panic. The alternative — managing the
/// handle itself — makes "the database is not open" unrepresentable, so every
/// caller in a runtime without one crashes instead of refusing.
///
/// A [`OnceLock`] rather than a `Mutex<Option<…>>` for two reasons. It makes
/// the "filled exactly once" rule the type's own, so [`StoreSlot::install`]
/// cannot be talked into replacing a live handle with a second connection to
/// the same file; and it lets [`StoreSlot::lock`] hand back a guard borrowed
/// from the slot, which a value behind an outer lock could not do.
#[derive(Debug, Default)]
pub struct StoreSlot {
    handle: OnceLock<Arc<Mutex<SqlCipherStore>>>,
}

impl StoreSlot {
    /// Fill the slot. True the first time and false afterwards.
    ///
    /// The second call is refused rather than ignored: two handles means two
    /// write-ahead logs against one database, and a host that thinks it has
    /// re-opened the store should hear so at the call site.
    #[must_use]
    pub fn install(&self, handle: Arc<Mutex<SqlCipherStore>>) -> bool {
        self.handle.set(handle).is_ok()
    }

    /// The open store, if this process has one.
    ///
    /// A poisoned lock recovers the way [`crate::commands::shell::Session`]
    /// does: what is behind it is a database connection, and a thread that
    /// panicked while holding it has not made the rows unreadable.
    pub fn lock(&self) -> Option<MutexGuard<'_, SqlCipherStore>> {
        Some(
            self.handle
                .get()?
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
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
    open_store_choosing_keys(directory, &platform)
}

/// Open the store after asking `platform` for a KEK.
///
/// A plaintext key file is used only when the platform says it cannot protect
/// a KEK at all. Any other error is a real failure: treating it as
/// Unsupported would write a fresh seed beside a database we could not open,
/// and then claim that was the plan. Tests inject a stub here so that
/// distinction can go red.
pub fn open_store_choosing_keys(
    directory: impl AsRef<Path>,
    platform: &dyn KeyProvider,
) -> StoreResult<SessionStore> {
    let directory = directory.as_ref();
    match platform.key_encryption_key() {
        Ok(_) => Ok(SessionStore {
            handle: share(open_store(directory, platform)?),
            key_protection: KeyProtection::PlatformKeyStore,
        }),
        Err(KeyError::Unsupported(_)) => Ok(SessionStore {
            handle: share(open_store_with_test_file_keys(directory)?),
            key_protection: KeyProtection::UnprotectedKeyFile,
        }),
        Err(error) => Err(soul_store_api::types::StoreError::Backend(
            error.to_string(),
        )),
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
