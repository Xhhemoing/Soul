//! The collector as a background thread, and stopping it inside a second.
//!
//! AC-10 asks for two things that pull against each other: a poll interval slow
//! enough that watching the foreground costs nothing, and a stop fast enough
//! that the user sees it take effect. A thread that slept for the poll interval
//! would make the second depend on the first — turn collection off with a one
//! minute interval and it keeps running for up to a minute.
//!
//! So the wait is a condition variable, not a sleep. [`CollectorHandle::stop`]
//! sets the flag and wakes the thread, which then does exactly one thing before
//! it exits: writes the session in flight, because the time the user spent in
//! the application they are still looking at is theirs and dropping it would be
//! a silent data loss. A revocation is different — see [`StopReason`].
//!
//! [`STOP_BUDGET`] is the promise in PRODUCT_LOCK ("关闭后 1 秒内无新事件"),
//! stated here as a constant the acceptance test asserts against.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use uuid::Uuid;

use soul_policy::audit::ReasonCode;
use soul_policy::clock::{now_unix_millis, now_unix_seconds};

use crate::collector::{CollectSink, Collector, Poll, Tally};
use crate::consent::ConsentHandle;
use crate::error::{CollectError, CollectResult};
use crate::lock::lock;
use crate::source::{ForegroundSource, SourceError};

/// How often the foreground is sampled when nobody says otherwise.
///
/// A second is under the resolution of "how long was I in that application"
/// and far above the cost of one Win32 call.
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// The upper bound on how long stopping may take.
pub const STOP_BUDGET: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollectorConfig {
    pub poll_interval: Duration,
}

impl Default for CollectorConfig {
    fn default() -> CollectorConfig {
        CollectorConfig {
            poll_interval: DEFAULT_POLL_INTERVAL,
        }
    }
}

impl CollectorConfig {
    pub fn every(poll_interval: Duration) -> CollectorConfig {
        CollectorConfig { poll_interval }
    }
}

/// Why a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    /// Somebody called [`CollectorHandle::stop`]. The session in flight is
    /// written before the thread exits.
    Requested,
    /// Consent was withdrawn while the collector was running. The session in
    /// flight is discarded: turning collection off is not a request for one
    /// more row.
    ConsentWithdrawn,
    /// The platform source said this machine has no collector.
    SourceUnavailable,
    /// A write failed. Details are in [`CollectorReport::last_error`].
    WriteFailed,
}

impl StopReason {
    /// The reason code the `collect.stop` audit entry carries.
    fn reason_code(self) -> ReasonCode {
        match self {
            StopReason::ConsentWithdrawn => ReasonCode::ConsentRevoked,
            _ => ReasonCode::Routine,
        }
    }
}

/// What a finished run collected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorReport {
    pub content_key_id: Uuid,
    pub tally: Tally,
    pub stopped_because: StopReason,
    pub source: &'static str,
    /// The first failure that ended or marred the run, as a sentence. Never a
    /// sample and never a name.
    pub last_error: Option<String>,
}

/// A running collector.
///
/// Dropping it stops the thread. A collector that outlived the handle nobody
/// holds any more would keep writing rows with no way to switch it off.
#[derive(Debug)]
pub struct CollectorHandle {
    stopper: Arc<Stopper>,
    worker: Option<JoinHandle<CollectorReport>>,
    content_key_id: Uuid,
}

impl CollectorHandle {
    pub fn content_key_id(&self) -> Uuid {
        self.content_key_id
    }

    pub fn is_running(&self) -> bool {
        self.worker
            .as_ref()
            .map(|worker| !worker.is_finished())
            .unwrap_or(false)
    }

    /// Stop collecting and wait for the thread to finish winding down.
    ///
    /// Returns once nothing further will be written, which is the property
    /// AC-10 measures.
    pub fn stop(mut self) -> CollectResult<CollectorReport> {
        self.stopper.request_stop();
        match self.worker.take() {
            Some(worker) => worker.join().map_err(|_| CollectError::ThreadLost),
            None => Err(CollectError::ThreadLost),
        }
    }
}

impl Drop for CollectorHandle {
    fn drop(&mut self) {
        self.stopper.request_stop();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Start collecting in the background.
///
/// Refuses outright when collection has not been consented to. That is the
/// first of AC-09's two nets; the second is in
/// [`crate::collector::Collector::record`], which checks again immediately
/// before it writes.
pub fn start<F, S>(
    source: F,
    sink: Arc<Mutex<S>>,
    consent: &ConsentHandle,
    config: CollectorConfig,
) -> CollectResult<CollectorHandle>
where
    F: ForegroundSource + Send + 'static,
    S: CollectSink + Send + 'static,
{
    consent.require_collection()?;

    let mut collector = Collector::new(source, sink, consent.clone());
    let content_key_id = collector.content_key_id();
    collector.audit_start(now_unix_seconds())?;

    let stopper = Arc::new(Stopper::default());
    let worker = {
        let stopper = Arc::clone(&stopper);
        let interval = config.poll_interval;
        std::thread::Builder::new()
            .name("soul-collect".to_owned())
            .spawn(move || run(collector, &stopper, interval))
            .map_err(|error| CollectError::ThreadNotStarted(error.to_string()))?
    };

    Ok(CollectorHandle {
        stopper,
        worker: Some(worker),
        content_key_id,
    })
}

/// The loop. Poll, and stop the moment either the user or the platform says so.
fn run<F, S>(
    mut collector: Collector<F, S>,
    stopper: &Stopper,
    poll_interval: Duration,
) -> CollectorReport
where
    F: ForegroundSource,
    S: CollectSink,
{
    let source = collector.describe_source();
    let mut last_error: Option<String> = None;
    let mut stopped_because = StopReason::Requested;

    while !stopper.wait_for_stop(poll_interval) {
        match collector.poll_once(now_unix_millis()) {
            Ok(Poll::ConsentWithheld) => {
                stopped_because = StopReason::ConsentWithdrawn;
                break;
            }
            Ok(_) => {}
            Err(CollectError::Consent(_)) => {
                stopped_because = StopReason::ConsentWithdrawn;
                break;
            }
            Err(CollectError::Source(SourceError::Unsupported)) => {
                stopped_because = StopReason::SourceUnavailable;
                last_error.get_or_insert_with(|| SourceError::Unsupported.to_string());
                break;
            }
            // A platform call that failed once is not a reason to stop
            // watching; it is counted, and the next tick tries again.
            Err(CollectError::Source(error)) => {
                last_error.get_or_insert_with(|| error.to_string());
            }
            Err(error) => {
                stopped_because = StopReason::WriteFailed;
                last_error.get_or_insert_with(|| error.to_string());
                break;
            }
        }
    }

    match stopped_because {
        // Consent is gone: the session in flight is dropped unwritten.
        StopReason::ConsentWithdrawn => collector.discard_open(),
        _ => {
            if let Err(error) = collector.finish(now_unix_millis()) {
                last_error.get_or_insert_with(|| error.to_string());
            }
        }
    }

    // The chain still records that the run ended, even after a revocation:
    // `collect.stop` carries a reason code and a count, and no prose.
    if let Err(error) = collector.audit_stop(stopped_because.reason_code(), now_unix_seconds()) {
        last_error.get_or_insert_with(|| error.to_string());
    }

    CollectorReport {
        content_key_id: collector.content_key_id(),
        tally: collector.tally(),
        stopped_because,
        source,
        last_error,
    }
}

/// A stop flag the worker can wait on and a caller can trip.
#[derive(Debug, Default)]
struct Stopper {
    stop: Mutex<bool>,
    wake: Condvar,
}

impl Stopper {
    fn request_stop(&self) {
        *lock(&self.stop) = true;
        self.wake.notify_all();
    }

    /// Wait up to `timeout`, returning true if a stop was requested. Waking on
    /// the condition variable rather than sleeping is what keeps stopping
    /// inside [`STOP_BUDGET`] no matter how long the poll interval is.
    fn wait_for_stop(&self, timeout: Duration) -> bool {
        let stop = lock(&self.stop);
        if *stop {
            return true;
        }
        let (stop, _) = self
            .wake
            .wait_timeout(stop, timeout)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *stop
    }
}
