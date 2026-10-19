//! Owned, attempt-only native notification transport. No domain or UI authority.
//!
//! Shell acceptance is not display, an opened callback is not task completion,
//! and a waiter timing out does not cancel its actual worker. Never wait, join,
//! or drop this owner while holding a Session/store lock. Drop joins as a final
//! ownership safeguard; callers should request_stop and wait_stopped explicitly.

mod engine;
#[cfg(test)]
mod tests;
#[cfg(windows)]
mod windows;

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub type AttemptId = [u8; 16];
pub type WithdrawalId = [u8; 16];
pub const NOTIFICATION_TITLE: &str = "Soul";
pub const NOTIFICATION_BODY: &str = "有一项辅助事项需要查看，请打开 Soul 核对。";

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub live_capacity: usize,
    pub queue_capacity: usize,
    pub retention: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Unsupported,
    Initialize,
    OsRejected,
    Capacity,
    IdExhausted,
    WorkerPanicked,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnError {
    InvalidConfig,
    Thread,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    NotReady,
    Stopping,
    QueueFull,
    IdExhausted,
    UnknownAttempt,
    ReceiptCapacity,
    RequestConflict,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Starting,
    Ready,
    Stopping,
    Stopped,
    Failed(Failure),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Submission {
    Queued,
    InFlight,
    SubmittedToOs,
    Failed(Failure),
    NotSubmitted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Withdrawal {
    NotRequested,
    Pending,
    InFlight,
    WithdrawnBeforeSubmit,
    Withdrawn,
    Failed(Failure),
    NoLiveIcon,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resource {
    NotCreated,
    Reserved,
    Live,
    Deleting,
    Retired,
    DeleteFailed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub attempt_id: AttemptId,
    pub native_id: u16,
    pub submission: Submission,
    pub withdrawal: Withdrawal,
    pub withdrawal_id: Option<WithdrawalId>,
    pub resource: Resource,
    /// Resource retention elapsed; says nothing about what the user saw.
    pub expired: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WithdrawalReceipt {
    pub withdrawal_id: WithdrawalId,
    pub attempt_id: AttemptId,
    pub state: Withdrawal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventKind {
    Shown,
    Opened,
    Hidden,
    TimedOut,
    TrackingExpired,
    ExplorerLost,
    Resume,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeEvent {
    pub sequence: u64,
    pub attempt_id: Option<AttemptId>,
    pub kind: EventKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadyOutcome {
    Ready,
    Failed(Failure),
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopOutcome {
    Joined,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaitOutcome {
    Complete(Receipt),
    Unknown(Receipt),
    WorkerFailed(Failure, Receipt),
}

pub struct NativeTransport {
    shared: Arc<engine::Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

pub struct RequestTicket {
    shared: Arc<engine::Shared>,
    attempt_id: AttemptId,
    withdrawal: Option<WithdrawalId>,
}

impl NativeTransport {
    pub fn spawn(config: Config) -> Result<Self, SpawnError> {
        #[cfg(windows)]
        {
            Self::spawn_with(config, |shared| windows::WindowsOs::new(shared))
        }
        #[cfg(not(windows))]
        {
            Self::spawn_with(config, |_| {
                Err::<engine::UnsupportedOs, _>(Failure::Unsupported)
            })
        }
    }

    fn spawn_with<O, F>(config: Config, factory: F) -> Result<Self, SpawnError>
    where
        O: engine::Os + 'static,
        F: FnOnce(Arc<engine::Shared>) -> Result<O, Failure> + Send + 'static,
    {
        if config.live_capacity == 0
            || config.live_capacity > u16::MAX as usize
            || config.queue_capacity == 0
            || config.retention.is_zero()
            || Instant::now().checked_add(config.retention).is_none()
        {
            return Err(SpawnError::InvalidConfig);
        }
        let shared = Arc::new(engine::Shared::new(config));
        let owned = Arc::clone(&shared);
        let worker = std::thread::Builder::new()
            .name("soul-notification".into())
            .spawn(move || engine::run(owned, factory))
            .map_err(|_| SpawnError::Thread)?;
        Ok(Self {
            shared,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn wait_ready(&self, timeout: Duration) -> ReadyOutcome {
        self.shared.wait_ready(timeout)
    }
    pub fn phase(&self) -> Phase {
        self.shared.lock().phase
    }
    pub fn try_submit(&self, attempt_id: AttemptId) -> Result<RequestTicket, AdmissionError> {
        self.shared.submit(attempt_id)?;
        Ok(RequestTicket {
            shared: Arc::clone(&self.shared),
            attempt_id,
            withdrawal: None,
        })
    }
    pub fn try_withdraw(
        &self,
        attempt_id: AttemptId,
        withdrawal_id: WithdrawalId,
    ) -> Result<RequestTicket, AdmissionError> {
        self.shared.withdraw(attempt_id, withdrawal_id)?;
        Ok(RequestTicket {
            shared: Arc::clone(&self.shared),
            attempt_id,
            withdrawal: Some(withdrawal_id),
        })
    }
    pub fn snapshot(&self, attempt_id: AttemptId) -> Option<Receipt> {
        self.shared
            .lock()
            .records
            .get(&attempt_id)
            .map(|r| r.receipt)
    }
    pub fn withdrawal_snapshot(&self, withdrawal_id: WithdrawalId) -> Option<WithdrawalReceipt> {
        self.shared.lock().withdrawals.get(&withdrawal_id).copied()
    }
    pub fn pending_events(&self, limit: usize) -> Vec<NativeEvent> {
        self.shared
            .lock()
            .events
            .values()
            .take(limit)
            .copied()
            .collect()
    }
    pub fn ack_events(&self, through_sequence: u64) {
        self.shared.ack_events(through_sequence);
    }
    /// Control observations remain visible even when an earlier business event
    /// cannot yet be persisted. This query does not acknowledge any event.
    pub fn pending_control_events(&self) -> Vec<NativeEvent> {
        self.shared
            .lock()
            .events
            .values()
            .filter(|event| matches!(event.kind, EventKind::ExplorerLost | EventKind::Resume))
            .copied()
            .collect()
    }
    pub fn request_stop(&self) {
        self.shared.stop();
    }
    pub fn wait_stopped(&self, timeout: Duration) -> StopOutcome {
        if !self.shared.wait_done(timeout) {
            return StopOutcome::Unknown;
        }
        if let Some(worker) = self.worker.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = worker.join();
        }
        StopOutcome::Joined
    }
}
impl Drop for NativeTransport {
    fn drop(&mut self) {
        self.request_stop();
        if let Some(worker) = self
            .worker
            .get_mut()
            .unwrap_or_else(|p| p.into_inner())
            .take()
        {
            let _ = worker.join();
        }
    }
}
impl RequestTicket {
    pub fn wait_timeout(&self, timeout: Duration) -> WaitOutcome {
        self.shared
            .wait_receipt(self.attempt_id, self.withdrawal, timeout)
    }
}
