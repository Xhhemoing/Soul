//! WP07's command surface: turn foreground collection on, and off.
//!
//! Thin, like the rest of this module. `soul-collect` decides what a session is
//! and `soul-store` holds the events; what is here is the shape the desktop
//! shell binds to, plus the one thing a shell should not have to work out for
//! itself — that the collector and the caller share the store through a lock,
//! because the collector writes from its own thread.
//!
//! There is deliberately no `is_collecting()` that reads a configuration flag.
//! Whether collection is running is [`CollectorHandle::is_running`], and
//! whether it is *allowed* is the consent ledger; a third copy of the answer in
//! a settings file is how a user ends up being shown "off" by a switch while
//! something is still writing.

use std::sync::{Arc, Mutex};

use uuid::Uuid;

use soul_collect::runner;
use soul_store::SqlCipherStore;
use soul_store_api::forget::ForgetUnit;

pub use soul_collect::{
    AppIdentity, CollectError, CollectResult, Collector, CollectorConfig, CollectorHandle,
    CollectorReport, ConsentHandle, FakeForegroundSource, ForegroundSource, SourceError,
    StopReason, COLLECTION_TOPIC, DEFAULT_POLL_INTERVAL, STOP_BUDGET,
};

/// The store, ready to be shared with a collector thread.
pub fn share(store: SqlCipherStore) -> Arc<Mutex<SqlCipherStore>> {
    Arc::new(Mutex::new(store))
}

/// The foreground source for the machine this is running on.
///
/// Fails on anything that is not Windows. v0.1 collects on Windows only, and a
/// source that reported an empty desktop everywhere else would let the user
/// switch collection on and see nothing wrong.
pub fn platform_source() -> Result<Box<dyn ForegroundSource + Send>, SourceError> {
    soul_collect::platform_source()
}

/// Start collecting foreground application duration in the background.
///
/// Refuses unless the user has consented. Writes a `collect.start` audit entry
/// before the first sample.
pub fn start<F>(
    source: F,
    store: Arc<Mutex<SqlCipherStore>>,
    consent: &ConsentHandle,
    config: CollectorConfig,
) -> CollectResult<CollectorHandle>
where
    F: ForegroundSource + Send + 'static,
{
    runner::start(source, store, consent, config)
}

/// Stop collecting. Returns once nothing further will be written, which is
/// within [`STOP_BUDGET`].
pub fn stop(handle: CollectorHandle) -> CollectResult<CollectorReport> {
    handle.stop()
}

/// What the user forgets when they ask to forget what a run collected.
///
/// One content key per run, so this is the whole of it: the application names
/// sealed under that key stop being openable, and the event rows stay as
/// tombstones showing that time was spent without saying where.
pub fn forget_unit(content_key_id: Uuid) -> ForgetUnit {
    ForgetUnit::ContentKey(content_key_id)
}
