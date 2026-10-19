//! Production bridge between durable core attempts and the owned OS transport.
//! An unrecorded result stays here for reconciliation; it is never resubmitted.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use soulcore::commands::session::{
    AssistanceNotificationId as Uuid, NotificationDelivery, NotificationNativeEvent,
    NotificationWithdrawalState, Session, SessionRefusal,
};

use crate::notification_transport::{
    AdmissionError, AttemptId, EventKind, Failure, NativeEvent, NativeTransport, Phase, Receipt,
    Submission, Withdrawal, WithdrawalId, WithdrawalReceipt,
};

const RECEIPT_WAIT: Duration = Duration::from_secs(5);
const BATCH: usize = 32;

/// A narrow observation seam; the shipped implementation below owns the real
/// queue/thread. Tests supply fixed OS observations, never business decisions.
pub trait Transport {
    fn phase(&self) -> Phase;
    fn submit(&self, attempt: AttemptId) -> Result<(), AdmissionError>;
    fn withdraw(&self, attempt: AttemptId, withdrawal: WithdrawalId) -> Result<(), AdmissionError>;
    fn receipt(&self, attempt: AttemptId) -> Option<Receipt>;
    fn withdrawal_receipt(&self, withdrawal: WithdrawalId) -> Option<WithdrawalReceipt>;
    fn events(&self, limit: usize) -> Vec<NativeEvent>;
    fn controls(&self) -> Vec<NativeEvent>;
    fn ack(&self, sequence: u64);
}

impl Transport for NativeTransport {
    fn phase(&self) -> Phase {
        self.phase()
    }
    fn submit(&self, attempt: AttemptId) -> Result<(), AdmissionError> {
        self.try_submit(attempt).map(drop)
    }
    fn withdraw(&self, attempt: AttemptId, withdrawal: WithdrawalId) -> Result<(), AdmissionError> {
        self.try_withdraw(attempt, withdrawal).map(drop)
    }
    fn receipt(&self, attempt: AttemptId) -> Option<Receipt> {
        self.snapshot(attempt)
    }
    fn withdrawal_receipt(&self, withdrawal: WithdrawalId) -> Option<WithdrawalReceipt> {
        self.withdrawal_snapshot(withdrawal)
    }
    fn events(&self, limit: usize) -> Vec<NativeEvent> {
        self.pending_events(limit)
    }
    fn controls(&self) -> Vec<NativeEvent> {
        self.pending_control_events()
    }
    fn ack(&self, sequence: u64) {
        self.ack_events(sequence);
    }
}

struct DeliveryWait {
    started: Instant,
    fixed: Option<NotificationDelivery>,
    recorded: Option<NotificationDelivery>,
}
struct WithdrawalWait {
    attempt: Uuid,
    started: Instant,
    fixed: Option<NotificationWithdrawalState>,
    recorded: Option<NotificationWithdrawalState>,
}

#[derive(Default)]
pub struct Driver {
    deliveries: BTreeMap<Uuid, DeliveryWait>,
    withdrawals: BTreeMap<Uuid, WithdrawalWait>,
    observed_controls: BTreeSet<u64>,
}

#[derive(Default, Debug)]
pub struct Progress {
    pub coordination_failed: bool,
}

fn held(session: &Arc<Mutex<Session>>) -> MutexGuard<'_, Session> {
    session.lock().unwrap_or_else(|error| error.into_inner())
}
fn failed(error: &SessionRefusal) -> bool {
    !matches!(
        error.reason_code.as_str(),
        "IMPORT_BUSY" | "ASSISTANCE_BUSY" | "REMINDER_BUSY"
    )
}

impl Driver {
    /// Reconcile receipts and safety controls before considering a new notice.
    pub fn step(
        &mut self,
        session: &Arc<Mutex<Session>>,
        transport: &impl Transport,
        stopping: &AtomicBool,
        on_open: &mut impl FnMut(Uuid),
    ) -> Progress {
        let mut progress = self.flush(session, transport, &mut |item| {
            if !stopping.load(Ordering::Acquire) {
                on_open(item);
            }
        });
        if stopping.load(Ordering::Acquire) {
            return progress;
        }

        // Withdrawal is a separate, priority control path. A full submit queue
        // cannot turn this into another notification submission.
        for _ in 0..BATCH {
            let claim = {
                let mut session = held(session);
                if stopping.load(Ordering::Acquire) {
                    break;
                }
                session.claim_assistance_notification_withdrawal()
            };
            let withdrawal = match claim {
                Ok(Some(value)) => value,
                Ok(None) => break,
                Err(error) => {
                    progress.coordination_failed |= failed(&error);
                    break;
                }
            };
            let fixed = match transport.withdraw(
                *withdrawal.attempt_id.as_bytes(),
                *withdrawal.withdrawal_id.as_bytes(),
            ) {
                Ok(()) => None,
                Err(
                    AdmissionError::UnknownAttempt
                    | AdmissionError::NotReady
                    | AdmissionError::Stopping,
                ) => Some(NotificationWithdrawalState::Unknown),
                Err(_) => Some(NotificationWithdrawalState::Failed),
            };
            self.withdrawals.insert(
                withdrawal.withdrawal_id,
                WithdrawalWait {
                    attempt: withdrawal.attempt_id,
                    started: Instant::now(),
                    fixed,
                    recorded: None,
                },
            );
        }

        if transport.phase() != Phase::Ready {
            if let Err(error) = held(session).expire_due_assistance_reminders() {
                progress.coordination_failed |= failed(&error);
            }
            return progress;
        }
        let work = {
            let mut session = held(session);
            // Stop may have happened while obtaining the Session mutex.
            if stopping.load(Ordering::Acquire) {
                return progress;
            }
            match session.reconcile_assistance_reminder_check() {
                Ok(()) => session.begin_assistance_reminder_check(),
                Err(error) => Err(error),
            }
        };
        let work = match work {
            Ok(Some(value)) => value,
            Ok(None) => return progress,
            Err(error) => {
                progress.coordination_failed |= failed(&error);
                return progress;
            }
        };
        // Strict source validation can be long; never carry a Session guard.
        let completion = work.execute();
        let dispatch = held(session).complete_assistance_reminder_check(completion);
        let dispatch = match dispatch {
            Ok(Some(value)) => value,
            Ok(None) => return progress,
            Err(error) => {
                progress.coordination_failed |= failed(&error);
                return progress;
            }
        };
        let authorized = {
            let session = held(session);
            if stopping.load(Ordering::Acquire) {
                Ok(false)
            } else {
                session.authorize_assistance_notification_dispatch(dispatch.attempt_id)
            }
        };
        let fixed = match authorized {
            Ok(true) => match transport.submit(*dispatch.attempt_id.as_bytes()) {
                Ok(()) => None,
                Err(_) => Some(NotificationDelivery::Failed),
            },
            Ok(false) => Some(NotificationDelivery::Failed),
            // No native call happened, but coordination failed. Keep the exact
            // attempt unknown; it is not a reason to acquire a fresh permit.
            Err(error) => {
                progress.coordination_failed |= failed(&error);
                Some(NotificationDelivery::Unknown)
            }
        };
        self.deliveries.insert(
            dispatch.attempt_id,
            DeliveryWait {
                started: Instant::now(),
                fixed,
                recorded: None,
            },
        );
        let flushed = self.flush(session, transport, &mut |item| {
            if !stopping.load(Ordering::Acquire) {
                on_open(item);
            }
        });
        progress.coordination_failed |= flushed.coordination_failed;
        progress
    }

    /// Also used after a real native join. This does no new claim or OS call.
    pub fn flush(
        &mut self,
        session: &Arc<Mutex<Session>>,
        transport: &impl Transport,
        on_open: &mut impl FnMut(Uuid),
    ) -> Progress {
        let mut progress = Progress::default();
        let events = transport.events(BATCH);
        let controls = transport.controls();
        // System recovery must be observed even if an earlier database event
        // cannot yet be acknowledged. Do not reapply the same reset each tick.
        self.observed_controls
            .retain(|seq| controls.iter().any(|event| event.sequence == *seq));
        for event in &controls {
            if matches!(event.kind, EventKind::Resume | EventKind::ExplorerLost)
                && self.observed_controls.insert(event.sequence)
            {
                held(session).reset_assistance_reminder_clock();
            }
        }

        let mut completed = Vec::new();
        for (id, waiting) in &mut self.withdrawals {
            let (state, terminal) = if let Some(fixed) = waiting.fixed {
                (Some(fixed), true)
            } else {
                withdrawal_outcome(
                    transport.withdrawal_receipt(*id.as_bytes()),
                    transport.phase(),
                    waiting.started.elapsed(),
                )
            };
            if let Some(state) = state {
                if waiting.recorded != Some(state) {
                    match held(session).complete_assistance_notification_withdrawal(
                        waiting.attempt,
                        *id,
                        state,
                    ) {
                        Ok(()) => waiting.recorded = Some(state),
                        Err(error) => {
                            progress.coordination_failed |= failed(&error);
                            continue;
                        }
                    }
                }
                if terminal {
                    completed.push(*id);
                }
            }
        }
        for id in completed {
            self.withdrawals.remove(&id);
        }

        let mut completed = Vec::new();
        for (id, waiting) in &mut self.deliveries {
            let (state, terminal) = if let Some(fixed) = waiting.fixed {
                (Some(fixed), true)
            } else {
                delivery_outcome(
                    transport.receipt(*id.as_bytes()),
                    transport.phase(),
                    waiting.started.elapsed(),
                )
            };
            if let Some(state) = state {
                if waiting.recorded != Some(state) {
                    match held(session).record_assistance_notification_delivery(*id, state) {
                        Ok(()) => waiting.recorded = Some(state),
                        Err(error) => {
                            progress.coordination_failed |= failed(&error);
                            continue;
                        }
                    }
                }
                if terminal {
                    completed.push(*id);
                }
            }
        }
        for id in completed {
            self.deliveries.remove(&id);
        }

        // Acknowledge only a successfully reconciled prefix. Failed writes are
        // retained by the native owner; repeated event writes are idempotent.
        for event in events {
            let result = match event.attempt_id {
                None => Ok(None),
                Some(attempt) => {
                    let attempt = Uuid::from_bytes(attempt);
                    let kind = match event.kind {
                        EventKind::Shown => Some(NotificationNativeEvent::Shown),
                        EventKind::Opened => Some(NotificationNativeEvent::Opened),
                        EventKind::Hidden => Some(NotificationNativeEvent::Hidden),
                        EventKind::TimedOut => Some(NotificationNativeEvent::TimedOut),
                        EventKind::TrackingExpired => {
                            Some(NotificationNativeEvent::TrackingExpired)
                        }
                        EventKind::Resume | EventKind::ExplorerLost => None,
                    };
                    let mut session = held(session);
                    match kind {
                        Some(kind) => session
                            .record_assistance_notification_event(attempt, kind)
                            .and_then(|()| {
                                if kind == NotificationNativeEvent::Opened {
                                    session
                                        .resolve_assistance_notification(attempt)
                                        .map(|target| target.available.then_some(target.item_id))
                                } else {
                                    Ok(None)
                                }
                            }),
                        None => Ok(None),
                    }
                }
            };
            match result {
                Ok(item) => {
                    // UI navigation is outside the Session lock and never an
                    // approval, completion, source selection, or draft action.
                    if let Some(item) = item {
                        on_open(item);
                    }
                    transport.ack(event.sequence);
                }
                Err(error) => {
                    progress.coordination_failed |= failed(&error);
                    break;
                }
            }
        }
        progress
    }
}

pub fn delivery_outcome(
    receipt: Option<Receipt>,
    phase: Phase,
    elapsed: Duration,
) -> (Option<NotificationDelivery>, bool) {
    let ended = matches!(phase, Phase::Failed(_) | Phase::Stopped);
    match receipt.map(|value| value.submission) {
        Some(Submission::SubmittedToOs) => (Some(NotificationDelivery::SubmittedToOs), true),
        Some(Submission::Failed(Failure::WorkerPanicked)) => {
            (Some(NotificationDelivery::Unknown), true)
        }
        Some(Submission::Failed(_) | Submission::NotSubmitted) => {
            (Some(NotificationDelivery::Failed), true)
        }
        Some(Submission::Queued) if ended => (Some(NotificationDelivery::Failed), true),
        _ if ended => (Some(NotificationDelivery::Unknown), true),
        _ if elapsed >= RECEIPT_WAIT => (Some(NotificationDelivery::Unknown), false),
        _ => (None, false),
    }
}

pub fn withdrawal_outcome(
    receipt: Option<WithdrawalReceipt>,
    phase: Phase,
    elapsed: Duration,
) -> (Option<NotificationWithdrawalState>, bool) {
    let ended = matches!(phase, Phase::Failed(_) | Phase::Stopped);
    match receipt.map(|value| value.state) {
        Some(Withdrawal::Withdrawn) => (Some(NotificationWithdrawalState::Withdrawn), true),
        Some(Withdrawal::WithdrawnBeforeSubmit | Withdrawal::NoLiveIcon) => {
            (Some(NotificationWithdrawalState::NotPresent), true)
        }
        Some(Withdrawal::Failed(Failure::WorkerPanicked)) => {
            (Some(NotificationWithdrawalState::Unknown), true)
        }
        Some(Withdrawal::Failed(_)) => (Some(NotificationWithdrawalState::Failed), true),
        _ if ended => (Some(NotificationWithdrawalState::Unknown), true),
        _ if elapsed >= RECEIPT_WAIT => (Some(NotificationWithdrawalState::Unknown), false),
        _ => (None, false),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
