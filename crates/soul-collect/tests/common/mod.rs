//! A real encrypted store, shared with a collector thread.
//!
//! Every test in this directory runs against `SqlCipherStore`, not `FakeStore`.
//! AC-09 says the number of foreground events is zero when consent is withheld,
//! and a count is only worth something if the thing being counted could
//! actually have landed: the fake store has no SQL, no schema validation and no
//! file on disk, so "nothing was written" would be a much weaker claim there.
//!
//! The one substitution is the source, which is the seam WP07 exists to
//! provide. Everything downstream of it — the consent gate, the sealing, the
//! event append, the audit chain — is the code a Windows machine runs.

// Each test binary uses a different subset of these.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use soul_collect::{ConsentHandle, COLLECTION_TOPIC};
use soul_schema::event::{EventSource, SoulEvent};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soul_store_api::types::EventFilter;
use soul_store_api::{AuditLog, EventStore};

/// 2026-08-24T00:00:00Z, for the audit entries a test wants to be reproducible.
pub const NOW: i64 = 1_787_529_600;

pub const SEED: &str = "wp07 foreground application duration";

/// Fast enough that an acceptance test does not spend its life asleep, slow
/// enough that the collector is genuinely polling rather than spinning.
pub const TEST_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// A store behind the lock the collector shares with its caller.
pub fn open_shared(path: impl AsRef<std::path::Path>) -> Arc<Mutex<SqlCipherStore>> {
    let keys = TestKeyProvider::from_seed(SEED);
    Arc::new(Mutex::new(
        SqlCipherStore::open(path, &keys).expect("open the encrypted store"),
    ))
}

pub fn granted_consent() -> ConsentHandle {
    let consent = ConsentHandle::closed();
    consent.grant(COLLECTION_TOPIC, NOW);
    consent
}

/// Every event the collector wrote, in the order the store returns them.
pub fn foreground_events(store: &Arc<Mutex<SqlCipherStore>>) -> Vec<SoulEvent> {
    store
        .lock()
        .expect("the store lock")
        .list_events(&EventFilter::with_source(
            EventSource::CollectorForegroundApp,
        ))
        .expect("list the foreground events")
}

pub fn foreground_event_count(store: &Arc<Mutex<SqlCipherStore>>) -> usize {
    foreground_events(store).len()
}

/// Every event of any kind. AC-09's "zero" has to mean zero rows, not zero
/// rows of the kind the collector would have written.
pub fn all_event_count(store: &Arc<Mutex<SqlCipherStore>>) -> usize {
    store
        .lock()
        .expect("the store lock")
        .list_events(&EventFilter::all())
        .expect("list every event")
        .len()
}

pub fn audit_json(store: &Arc<Mutex<SqlCipherStore>>) -> String {
    let entries = store
        .lock()
        .expect("the store lock")
        .list_audit()
        .expect("read the audit chain");
    serde_json::to_string(&entries).expect("serialize the audit chain")
}

/// Wait until `predicate` holds, or give up. Returns whether it held.
///
/// Used instead of a fixed sleep so a slow CI host makes the test slower rather
/// than flaky, and a broken collector fails after the deadline rather than
/// passing because the sleep happened to be long enough.
pub fn wait_until(deadline: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    while started.elapsed() < deadline {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    predicate()
}
