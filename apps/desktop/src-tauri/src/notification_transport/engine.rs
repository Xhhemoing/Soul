use super::*;
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, MutexGuard};

pub(super) trait Os {
    fn submit(&mut self, id: u16) -> OsSubmission;
    fn delete(&mut self, id: u16) -> bool;
    fn pump(&mut self);
}
pub(super) struct OsSubmission {
    pub accepted: bool,
    pub icon_live: bool,
}
#[cfg(not(windows))]
pub(super) struct UnsupportedOs;
#[cfg(not(windows))]
impl Os for UnsupportedOs {
    fn submit(&mut self, _: u16) -> OsSubmission {
        OsSubmission {
            accepted: false,
            icon_live: false,
        }
    }
    fn delete(&mut self, _: u16) -> bool {
        false
    }
    fn pump(&mut self) {}
}
pub(super) struct Record {
    pub receipt: Receipt,
    retire_at: Option<Instant>,
    seen_events: u8,
    started: bool,
    expiry_attempted: bool,
    shutdown_attempted: bool,
}
pub(super) struct State {
    pub phase: Phase,
    pub records: BTreeMap<AttemptId, Record>,
    pub events: BTreeMap<u64, NativeEvent>,
    pub withdrawals: BTreeMap<WithdrawalId, WithdrawalReceipt>,
    withdrawal_queue: VecDeque<WithdrawalId>,
    by_native: BTreeMap<u16, AttemptId>,
    queue: VecDeque<AttemptId>,
    next_id: u32,
    next_event: u64,
    done: bool,
}
pub(super) struct Shared {
    config: Config,
    state: Mutex<State>,
    changed: Condvar,
    stopping: AtomicBool,
}
impl Shared {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            state: Mutex::new(State {
                phase: Phase::Starting,
                records: BTreeMap::new(),
                events: BTreeMap::new(),
                withdrawals: BTreeMap::new(),
                withdrawal_queue: VecDeque::new(),
                by_native: BTreeMap::new(),
                queue: VecDeque::new(),
                next_id: 1,
                next_event: 1,
                done: false,
            }),
            changed: Condvar::new(),
            stopping: AtomicBool::new(false),
        }
    }
    pub fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|p| p.into_inner())
    }
    fn wait<'a>(&self, guard: MutexGuard<'a, State>, duration: Duration) -> MutexGuard<'a, State> {
        self.changed
            .wait_timeout(guard, duration)
            .unwrap_or_else(|p| p.into_inner())
            .0
    }
    pub fn submit(&self, id: AttemptId) -> Result<(), AdmissionError> {
        let mut state = self.lock();
        // A duplicate only observes its original receipt, including after stop.
        if state.records.contains_key(&id) {
            return Ok(());
        }
        if self.stopping.load(Ordering::Acquire) {
            return Err(AdmissionError::Stopping);
        }
        if state.phase != Phase::Ready {
            return Err(AdmissionError::NotReady);
        }
        if state.queue.len() >= self.config.queue_capacity {
            return Err(AdmissionError::QueueFull);
        }
        if state.next_id > u16::MAX as u32 {
            return Err(AdmissionError::IdExhausted);
        }
        let native_id = state.next_id as u16;
        state.next_id += 1;
        state.by_native.insert(native_id, id);
        state.records.insert(
            id,
            Record {
                receipt: Receipt {
                    attempt_id: id,
                    native_id,
                    submission: Submission::Queued,
                    withdrawal: Withdrawal::NotRequested,
                    withdrawal_id: None,
                    resource: Resource::NotCreated,
                    expired: false,
                },
                retire_at: None,
                seen_events: 0,
                started: false,
                expiry_attempted: false,
                shutdown_attempted: false,
            },
        );
        state.queue.push_back(id);
        self.changed.notify_all();
        Ok(())
    }
    pub fn withdraw(
        &self,
        id: AttemptId,
        withdrawal_id: WithdrawalId,
    ) -> Result<(), AdmissionError> {
        let mut state = self.lock();
        if let Some(receipt) = state.withdrawals.get(&withdrawal_id) {
            return if receipt.attempt_id == id {
                Ok(())
            } else {
                Err(AdmissionError::RequestConflict)
            };
        }
        if state.done {
            return Err(AdmissionError::Stopping);
        }
        // Independent, bounded control receipts. No ordinary queue slot is needed.
        if state.withdrawals.len() >= u16::MAX as usize {
            return Err(AdmissionError::ReceiptCapacity);
        }
        let record = state
            .records
            .get_mut(&id)
            .ok_or(AdmissionError::UnknownAttempt)?;
        record.receipt.withdrawal_id = Some(withdrawal_id);
        record.receipt.withdrawal = Withdrawal::Pending;
        state.withdrawals.insert(
            withdrawal_id,
            WithdrawalReceipt {
                withdrawal_id,
                attempt_id: id,
                state: Withdrawal::Pending,
            },
        );
        state.withdrawal_queue.push_back(withdrawal_id);
        self.changed.notify_all();
        Ok(())
    }
    pub fn stop(&self) {
        self.stopping.store(true, Ordering::Release);
        let mut state = self.lock();
        if !state.done {
            state.phase = Phase::Stopping;
        }
        self.changed.notify_all();
    }
    pub fn event(&self, native_id: Option<u16>, kind: EventKind) {
        let mut state = self.lock();
        let attempt_id = if let Some(id) = native_id {
            let Some(attempt) = state.by_native.get(&id).copied() else {
                return;
            };
            let record = state
                .records
                .get_mut(&attempt)
                .expect("native map owns a record");
            if !record.started {
                return;
            }
            let bit = match kind {
                EventKind::Shown => 1,
                EventKind::Opened => 2,
                EventKind::Hidden => 4,
                EventKind::TimedOut => 8,
                EventKind::TrackingExpired => 16,
                _ => return,
            };
            if record.seen_events & bit != 0 {
                return;
            }
            record.seen_events |= bit;
            Some(attempt)
        } else {
            if !matches!(kind, EventKind::ExplorerLost | EventKind::Resume) {
                return;
            }
            // Every Explorer loss retires actual resources, even when its UI
            // event coalesces with an older, not-yet-acknowledged occurrence.
            if kind == EventKind::ExplorerLost {
                for record in state.records.values_mut() {
                    if occupied(record.receipt.resource) {
                        record.receipt.resource = Resource::Retired;
                    }
                }
            }
            // Keep one latest observation of each control kind, but advance
            // its sequence for every real occurrence. An unacknowledged older
            // occurrence must not hide a subsequent system resume/restart.
            state
                .events
                .retain(|_, event| !(event.attempt_id.is_none() && event.kind == kind));
            None
        };
        let sequence = state.next_event;
        let Some(next) = sequence.checked_add(1) else {
            state.phase = Phase::Failed(Failure::IdExhausted);
            self.stopping.store(true, Ordering::Release);
            return;
        };
        state.next_event = next;
        state.events.insert(
            sequence,
            NativeEvent {
                sequence,
                attempt_id,
                kind,
            },
        );
        self.changed.notify_all();
    }
    pub fn ack_events(&self, through: u64) {
        self.lock().events.retain(|sequence, _| *sequence > through);
    }
    #[cfg(test)]
    pub fn test_next_id(&self, value: u32) {
        self.lock().next_id = value;
    }
    #[cfg(test)]
    pub fn test_fill_withdrawal_receipts(&self, attempt_id: AttemptId) {
        let mut state = self.lock();
        for index in 0..u16::MAX {
            let mut withdrawal_id = [0; 16];
            withdrawal_id[..2].copy_from_slice(&index.to_le_bytes());
            state.withdrawals.insert(
                withdrawal_id,
                WithdrawalReceipt {
                    withdrawal_id,
                    attempt_id,
                    state: Withdrawal::NoLiveIcon,
                },
            );
        }
    }
    pub fn wait_ready(&self, timeout: Duration) -> ReadyOutcome {
        let until = Instant::now().checked_add(timeout);
        let mut state = self.lock();
        loop {
            match state.phase {
                Phase::Ready => return ReadyOutcome::Ready,
                Phase::Failed(error) => return ReadyOutcome::Failed(error),
                Phase::Stopped => return ReadyOutcome::Failed(Failure::Initialize),
                _ => {}
            }
            let remaining = remaining(until);
            if remaining.is_zero() {
                return ReadyOutcome::Unknown;
            }
            state = self.wait(state, remaining);
        }
    }
    pub fn wait_done(&self, timeout: Duration) -> bool {
        let until = Instant::now().checked_add(timeout);
        let mut state = self.lock();
        while !state.done {
            let remaining = remaining(until);
            if remaining.is_zero() {
                return false;
            }
            state = self.wait(state, remaining);
        }
        true
    }
    pub fn wait_receipt(
        &self,
        id: AttemptId,
        withdrawal: Option<WithdrawalId>,
        timeout: Duration,
    ) -> WaitOutcome {
        let until = Instant::now().checked_add(timeout);
        let mut state = self.lock();
        loop {
            let mut receipt = state
                .records
                .get(&id)
                .expect("tickets retain their admitted record")
                .receipt;
            let complete = if let Some(withdrawal_id) = withdrawal {
                let own = state
                    .withdrawals
                    .get(&withdrawal_id)
                    .expect("withdrawal tickets retain their own receipt");
                receipt.withdrawal_id = Some(withdrawal_id);
                receipt.withdrawal = own.state;
                !matches!(
                    own.state,
                    Withdrawal::Pending | Withdrawal::InFlight | Withdrawal::NotRequested
                )
            } else {
                !matches!(
                    receipt.submission,
                    Submission::Queued | Submission::InFlight
                )
            };
            if complete {
                return WaitOutcome::Complete(receipt);
            }
            if let Phase::Failed(failure) = state.phase {
                return WaitOutcome::WorkerFailed(failure, receipt);
            }
            let remaining = remaining(until);
            if remaining.is_zero() {
                return WaitOutcome::Unknown(receipt);
            }
            state = self.wait(state, remaining);
        }
    }
}
fn remaining(until: Option<Instant>) -> Duration {
    until
        .map(|at| at.saturating_duration_since(Instant::now()))
        .unwrap_or(Duration::ZERO)
}
fn occupied(resource: Resource) -> bool {
    matches!(
        resource,
        Resource::Reserved | Resource::Live | Resource::Deleting | Resource::DeleteFailed
    )
}
fn set_withdrawal(state: &mut State, withdrawal_id: WithdrawalId, value: Withdrawal) {
    let attempt = {
        let receipt = state
            .withdrawals
            .get_mut(&withdrawal_id)
            .expect("owned withdrawal receipt");
        receipt.state = value;
        receipt.attempt_id
    };
    let record = state
        .records
        .get_mut(&attempt)
        .expect("withdrawal owns an attempt");
    if record.receipt.withdrawal_id == Some(withdrawal_id) {
        record.receipt.withdrawal = value;
    }
}
enum DeleteReason {
    Withdraw(WithdrawalId),
    Expire,
    Shutdown,
}
enum Action {
    Submit(AttemptId, u16),
    Delete(AttemptId, u16, DeleteReason),
    Finish,
    Idle,
}

fn next_action(shared: &Shared, state: &mut State) -> Action {
    let stopping = shared.stopping.load(Ordering::Acquire);
    if stopping {
        while let Some(id) = state.queue.pop_front() {
            let record = state.records.get_mut(&id).expect("queued record");
            record.receipt.submission = Submission::NotSubmitted;
        }
    }
    loop {
        // Each explicit withdrawal UUID has its own immutable request identity.
        if let Some(withdrawal_id) = state.withdrawal_queue.pop_front() {
            let id = state
                .withdrawals
                .get(&withdrawal_id)
                .expect("queued withdrawal")
                .attempt_id;
            let record = state.records.get_mut(&id).expect("withdraw record");
            if matches!(
                record.receipt.submission,
                Submission::Queued | Submission::NotSubmitted
            ) {
                record.receipt.submission = Submission::NotSubmitted;
                set_withdrawal(state, withdrawal_id, Withdrawal::WithdrawnBeforeSubmit);
                state.queue.retain(|queued| *queued != id);
                continue;
            }
            if occupied(record.receipt.resource) {
                record.receipt.resource = Resource::Deleting;
                let native = record.receipt.native_id;
                set_withdrawal(state, withdrawal_id, Withdrawal::InFlight);
                return Action::Delete(id, native, DeleteReason::Withdraw(withdrawal_id));
            }
            // Expiry or an earlier Shell loss can remove a resource independently
            // of this request. Do not rewrite that history as user withdrawal.
            set_withdrawal(state, withdrawal_id, Withdrawal::NoLiveIcon);
            continue;
        }
        for (id, record) in &mut state.records {
            if occupied(record.receipt.resource) {
                let reason = if stopping && !record.shutdown_attempted {
                    record.shutdown_attempted = true;
                    Some(DeleteReason::Shutdown)
                } else if !stopping
                    && !record.expiry_attempted
                    && record.retire_at.is_some_and(|at| Instant::now() >= at)
                {
                    record.expiry_attempted = true;
                    record.receipt.expired = true;
                    Some(DeleteReason::Expire)
                } else {
                    None
                };
                if let Some(reason) = reason {
                    record.receipt.resource = Resource::Deleting;
                    return Action::Delete(*id, record.receipt.native_id, reason);
                }
            }
        }
        if stopping {
            return Action::Finish;
        }
        if let Some(id) = state.queue.pop_front() {
            let live = state
                .records
                .values()
                .filter(|r| occupied(r.receipt.resource))
                .count();
            let record = state.records.get_mut(&id).expect("queued record");
            if live >= shared.config.live_capacity {
                record.receipt.submission = Submission::Failed(Failure::Capacity);
                continue;
            }
            record.started = true;
            record.receipt.submission = Submission::InFlight;
            record.receipt.resource = Resource::Reserved;
            return Action::Submit(id, record.receipt.native_id);
        }
        return Action::Idle;
    }
}

pub(super) fn run<O, F>(shared: Arc<Shared>, factory: F)
where
    O: Os,
    F: FnOnce(Arc<Shared>) -> Result<O, Failure>,
{
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut os = factory(Arc::clone(&shared))?;
        {
            let mut state = shared.lock();
            if !shared.stopping.load(Ordering::Acquire) {
                state.phase = Phase::Ready;
            }
            shared.changed.notify_all();
        }
        loop {
            os.pump();
            let action = {
                let mut state = shared.lock();
                let action = next_action(&shared, &mut state);
                shared.changed.notify_all();
                action
            };
            match action {
                Action::Submit(id, native) => {
                    let result = os.submit(native);
                    let mut state = shared.lock();
                    let record = state.records.get_mut(&id).expect("owned submit record");
                    record.receipt.submission = if result.accepted {
                        Submission::SubmittedToOs
                    } else {
                        Submission::Failed(Failure::OsRejected)
                    };
                    record.receipt.resource = if result.icon_live {
                        Resource::Live
                    } else {
                        Resource::NotCreated
                    };
                    record.retire_at = Instant::now().checked_add(shared.config.retention);
                    shared.changed.notify_all();
                }
                Action::Delete(id, native, reason) => {
                    if matches!(reason, DeleteReason::Expire) {
                        shared.event(Some(native), EventKind::TrackingExpired);
                    }
                    let deleted = os.delete(native);
                    let mut state = shared.lock();
                    let record = state.records.get_mut(&id).expect("owned delete record");
                    record.receipt.resource = if deleted {
                        Resource::Retired
                    } else {
                        Resource::DeleteFailed
                    };
                    if let DeleteReason::Withdraw(withdrawal_id) = reason {
                        set_withdrawal(
                            &mut state,
                            withdrawal_id,
                            if deleted {
                                Withdrawal::Withdrawn
                            } else {
                                Withdrawal::Failed(Failure::OsRejected)
                            },
                        );
                    }
                    shared.changed.notify_all();
                }
                Action::Finish => break,
                Action::Idle => {
                    // Wake control immediately; also pump native messages independently
                    // of callbacks and retain-time cleanup. This is not domain scheduling.
                    let state = shared.lock();
                    let _guard = shared.wait(state, Duration::from_millis(20));
                }
            }
        }
        drop(os);
        Ok::<(), Failure>(())
    }));
    let mut state = shared.lock();
    state.phase = match result {
        Ok(Ok(())) => Phase::Stopped,
        Ok(Err(failure)) => Phase::Failed(failure),
        Err(_) => Phase::Failed(Failure::WorkerPanicked),
    };
    state.done = true;
    shared.changed.notify_all();
}
