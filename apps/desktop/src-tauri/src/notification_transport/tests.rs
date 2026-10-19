//! These exercise the real owned worker/queue/registry with a fake OS. They
//! cannot establish Windows Shell display, permissions, or callback behavior.
use super::engine::{Os, OsSubmission, Shared};
use super::*;
use std::sync::mpsc::{self, Receiver, Sender};

const A: AttemptId = [1; 16];
const B: AttemptId = [2; 16];
const C: AttemptId = [3; 16];
const W1: WithdrawalId = [11; 16];
const W2: WithdrawalId = [12; 16];
const SECOND: Duration = Duration::from_secs(2);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    Submit(u16),
    Delete(u16),
}
struct Gate {
    entered: Sender<()>,
    release: Receiver<()>,
}
impl Gate {
    fn wait(self) {
        let _ = self.entered.send(());
        let _ = self.release.recv_timeout(SECOND);
    }
}
fn gate() -> (Gate, Receiver<()>, Sender<()>) {
    let (entered, observed) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    (
        Gate {
            entered,
            release: wait,
        },
        observed,
        release,
    )
}
struct FakeOs {
    calls: Arc<Mutex<Vec<Call>>>,
    gate: Option<Gate>,
    delete_gate: Option<Gate>,
    accepted: bool,
    delete_ok: bool,
}
impl Os for FakeOs {
    fn submit(&mut self, id: u16) -> OsSubmission {
        self.calls.lock().unwrap().push(Call::Submit(id));
        if let Some(gate) = self.gate.take() {
            gate.wait();
        }
        OsSubmission {
            accepted: self.accepted,
            icon_live: true,
        }
    }
    fn delete(&mut self, id: u16) -> bool {
        self.calls.lock().unwrap().push(Call::Delete(id));
        if let Some(gate) = self.delete_gate.take() {
            gate.wait();
        }
        self.delete_ok
    }
    fn pump(&mut self) {}
}
fn config() -> Config {
    Config {
        live_capacity: 1,
        queue_capacity: 32,
        retention: Duration::from_secs(300),
    }
}
fn fake(
    config: Config,
    gate: Option<Gate>,
    delete_ok: bool,
) -> (NativeTransport, Arc<Mutex<Vec<Call>>>, Arc<Shared>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&calls);
    let (sender, receiver) = mpsc::channel();
    let transport = NativeTransport::spawn_with(config, move |shared| {
        sender.send(Arc::clone(&shared)).unwrap();
        Ok::<_, Failure>(FakeOs {
            calls: observed,
            gate,
            delete_gate: None,
            accepted: true,
            delete_ok,
        })
    })
    .unwrap();
    assert_eq!(transport.wait_ready(SECOND), ReadyOutcome::Ready);
    (transport, calls, receiver.recv_timeout(SECOND).unwrap())
}
fn complete(ticket: &RequestTicket) -> Receipt {
    match ticket.wait_timeout(SECOND) {
        WaitOutcome::Complete(receipt) => receipt,
        other => panic!("unexpected receipt {other:?}"),
    }
}
fn eventually(mut predicate: impl FnMut() -> bool) {
    let until = Instant::now() + SECOND;
    while !predicate() {
        assert!(Instant::now() < until, "state did not settle");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn stop(transport: &NativeTransport) {
    transport.request_stop();
    assert_eq!(transport.wait_stopped(SECOND), StopOutcome::Joined);
}

#[test]
fn receiver_loss_timeout_and_concurrent_duplicates_observe_one_real_submit() {
    let (slow, entered, release) = gate();
    let (transport, calls, _) = fake(config(), Some(slow), true);
    let transport = Arc::new(transport);
    let first = transport.try_submit(A).unwrap();
    entered.recv_timeout(SECOND).unwrap();
    assert!(matches!(
        first.wait_timeout(Duration::ZERO),
        WaitOutcome::Unknown(_)
    ));
    drop(first);
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let transport = Arc::clone(&transport);
            std::thread::spawn(move || {
                let ticket = transport.try_submit(A).unwrap();
                assert!(matches!(
                    ticket.wait_timeout(Duration::ZERO),
                    WaitOutcome::Unknown(_)
                ));
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    release.send(()).unwrap();
    assert_eq!(
        complete(&transport.try_submit(A).unwrap()).submission,
        Submission::SubmittedToOs
    );
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| matches!(call, Call::Submit(_)))
            .count(),
        1
    );
    stop(&transport);
}

#[test]
fn full_queue_cannot_block_stop_and_a_started_call_is_observed_before_join() {
    let (slow, entered, release) = gate();
    let mut config = config();
    config.queue_capacity = 1;
    let (transport, calls, _) = fake(config, Some(slow), true);
    let a = transport.try_submit(A).unwrap();
    entered.recv_timeout(SECOND).unwrap();
    let b = transport.try_submit(B).unwrap();
    assert!(matches!(
        transport.try_submit(C),
        Err(AdmissionError::QueueFull)
    ));
    transport.request_stop();
    assert_eq!(transport.wait_stopped(Duration::ZERO), StopOutcome::Unknown);
    assert_eq!(transport.phase(), Phase::Stopping);
    assert!(matches!(
        transport.try_submit(C),
        Err(AdmissionError::Stopping)
    ));
    release.send(()).unwrap();
    assert_eq!(transport.wait_stopped(SECOND), StopOutcome::Joined);
    assert_eq!(complete(&a).submission, Submission::SubmittedToOs);
    assert_eq!(complete(&b).submission, Submission::NotSubmitted);
    assert_eq!(
        *calls.lock().unwrap(),
        vec![Call::Submit(1), Call::Delete(1)]
    );
}

#[test]
fn withdrawal_of_queued_submit_has_priority_even_when_queue_is_full() {
    let (slow, entered, release) = gate();
    let mut config = config();
    config.queue_capacity = 1;
    config.live_capacity = 2;
    let (transport, calls, _) = fake(config, Some(slow), true);
    let a = transport.try_submit(A).unwrap();
    entered.recv_timeout(SECOND).unwrap();
    let b = transport.try_submit(B).unwrap();
    let withdrawal = transport.try_withdraw(B, W1).unwrap();
    assert!(matches!(
        withdrawal.wait_timeout(Duration::ZERO),
        WaitOutcome::Unknown(_)
    ));
    release.send(()).unwrap();
    complete(&a);
    assert_eq!(
        complete(&withdrawal).withdrawal,
        Withdrawal::WithdrawnBeforeSubmit
    );
    assert_eq!(complete(&b).submission, Submission::NotSubmitted);
    assert!(!calls.lock().unwrap().contains(&Call::Submit(2)));
    stop(&transport);
}

#[test]
fn inflight_submit_then_withdraw_preserves_both_real_outcomes() {
    let (slow, entered, release) = gate();
    let (transport, calls, _) = fake(config(), Some(slow), true);
    let submit = transport.try_submit(A).unwrap();
    entered.recv_timeout(SECOND).unwrap();
    let withdrawal = transport.try_withdraw(A, W1).unwrap();
    release.send(()).unwrap();
    let receipt = complete(&withdrawal);
    assert_eq!(receipt.submission, Submission::SubmittedToOs);
    assert_eq!(receipt.withdrawal, Withdrawal::Withdrawn);
    assert_eq!(complete(&submit).resource, Resource::Retired);
    assert_eq!(
        *calls.lock().unwrap(),
        vec![Call::Submit(1), Call::Delete(1)]
    );
    stop(&transport);
}

#[test]
fn withdrawal_uuid_keeps_old_timeout_ticket_separate_from_a_new_explicit_request() {
    let (slow, entered, release) = gate();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&calls);
    let transport = NativeTransport::spawn_with(config(), move |_| {
        Ok::<_, Failure>(FakeOs {
            calls: observed,
            gate: None,
            delete_gate: Some(slow),
            accepted: true,
            delete_ok: true,
        })
    })
    .unwrap();
    assert_eq!(transport.wait_ready(SECOND), ReadyOutcome::Ready);
    complete(&transport.try_submit(A).unwrap());
    let old = transport.try_withdraw(A, W1).unwrap();
    entered.recv_timeout(SECOND).unwrap();
    assert!(
        matches!(old.wait_timeout(Duration::ZERO), WaitOutcome::Unknown(receipt) if receipt.withdrawal_id == Some(W1))
    );
    let duplicate = transport.try_withdraw(A, W1).unwrap();
    let newer = transport.try_withdraw(A, W2).unwrap();
    assert!(
        matches!(newer.wait_timeout(Duration::ZERO), WaitOutcome::Unknown(receipt) if receipt.withdrawal_id == Some(W2) && receipt.withdrawal == Withdrawal::Pending)
    );
    release.send(()).unwrap();
    let old_receipt = complete(&old);
    let new_receipt = complete(&newer);
    assert_eq!(old_receipt.withdrawal_id, Some(W1));
    assert_eq!(old_receipt.withdrawal, Withdrawal::Withdrawn);
    assert_eq!(new_receipt.withdrawal_id, Some(W2));
    assert_eq!(new_receipt.withdrawal, Withdrawal::NoLiveIcon);
    assert_eq!(complete(&duplicate).withdrawal_id, Some(W1));
    assert_eq!(complete(&duplicate).withdrawal, Withdrawal::Withdrawn);
    assert_eq!(
        transport.withdrawal_snapshot(W1).unwrap().state,
        Withdrawal::Withdrawn
    );
    assert_eq!(transport.snapshot(A).unwrap().withdrawal_id, Some(W2));
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| matches!(call, Call::Delete(_)))
            .count(),
        1
    );
    assert!(matches!(
        transport.try_withdraw(B, W1),
        Err(AdmissionError::RequestConflict)
    ));
    stop(&transport);
}

#[test]
fn failed_withdrawal_new_uuid_does_not_change_the_old_terminal_receipt() {
    let (transport, calls, _) = fake(config(), None, false);
    complete(&transport.try_submit(A).unwrap());
    let old = transport.try_withdraw(A, W1).unwrap();
    assert_eq!(
        complete(&old).withdrawal,
        Withdrawal::Failed(Failure::OsRejected)
    );
    let newer = transport.try_withdraw(A, W2).unwrap();
    assert_eq!(complete(&newer).withdrawal_id, Some(W2));
    assert_eq!(complete(&old).withdrawal_id, Some(W1));
    assert_eq!(
        complete(&old).withdrawal,
        Withdrawal::Failed(Failure::OsRejected)
    );
    complete(&transport.try_withdraw(A, W1).unwrap());
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| matches!(call, Call::Delete(_)))
            .count(),
        2
    );
    stop(&transport);
}

#[test]
fn withdrawal_receipt_capacity_refuses_new_ids_without_evicting_old_receipts() {
    let (transport, calls, shared) = fake(config(), None, true);
    complete(&transport.try_submit(A).unwrap());
    shared.test_fill_withdrawal_receipts(A);
    assert!(matches!(
        transport.try_withdraw(A, W1),
        Err(AdmissionError::ReceiptCapacity)
    ));
    let duplicate = complete(&transport.try_withdraw(A, [0; 16]).unwrap());
    assert_eq!(duplicate.withdrawal_id, Some([0; 16]));
    assert_eq!(duplicate.withdrawal, Withdrawal::NoLiveIcon);
    assert_eq!(*calls.lock().unwrap(), vec![Call::Submit(1)]);
    stop(&transport);
}

#[test]
fn callback_duplicates_are_sticky_and_deleted_id_still_names_the_original_attempt() {
    let (transport, _, shared) = fake(config(), None, true);
    let receipt = complete(&transport.try_submit(A).unwrap());
    complete(&transport.try_withdraw(A, W1).unwrap());
    complete(&transport.try_submit(B).unwrap());
    for kind in [
        EventKind::Shown,
        EventKind::Opened,
        EventKind::Hidden,
        EventKind::TimedOut,
    ] {
        shared.event(Some(receipt.native_id), kind);
        shared.event(Some(receipt.native_id), kind);
    }
    let events = transport.pending_events(32);
    assert_eq!(events.len(), 4);
    assert!(events.iter().all(|event| event.attempt_id == Some(A)));
    assert_eq!(transport.snapshot(B).unwrap().native_id, 2);
    transport.ack_events(events.last().unwrap().sequence);
    shared.event(Some(receipt.native_id), EventKind::Opened);
    assert!(transport.pending_events(32).is_empty());
    stop(&transport);
}

#[test]
fn expiry_reclaims_actual_resource_without_fabricating_withdrawal_or_user_events() {
    let mut config = config();
    config.retention = Duration::from_millis(10);
    let (transport, calls, shared) = fake(config, None, true);
    complete(&transport.try_submit(A).unwrap());
    eventually(|| transport.snapshot(A).unwrap().resource == Resource::Retired);
    let receipt = transport.snapshot(A).unwrap();
    assert!(receipt.expired);
    assert_eq!(receipt.withdrawal, Withdrawal::NotRequested);
    let events = transport.pending_events(32);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::TrackingExpired);
    assert_eq!(events[0].attempt_id, Some(A));
    transport.ack_events(events[0].sequence);
    complete(&transport.try_submit(B).unwrap());
    shared.event(Some(receipt.native_id), EventKind::Opened);
    assert_eq!(transport.pending_events(32)[0].attempt_id, Some(A));
    assert!(calls.lock().unwrap().contains(&Call::Delete(1)));
    stop(&transport);
}

#[test]
fn failed_deletion_does_not_release_actual_live_capacity() {
    let mut config = config();
    config.retention = Duration::from_millis(10);
    let (transport, calls, _) = fake(config, None, false);
    complete(&transport.try_submit(A).unwrap());
    eventually(|| transport.snapshot(A).unwrap().resource == Resource::DeleteFailed);
    assert_eq!(
        complete(&transport.try_submit(B).unwrap()).submission,
        Submission::Failed(Failure::Capacity)
    );
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| matches!(call, Call::Submit(_)))
            .count(),
        1
    );
    assert_eq!(
        complete(&transport.try_withdraw(A, W1).unwrap()).withdrawal,
        Withdrawal::Failed(Failure::OsRejected)
    );
    stop(&transport);
}

#[test]
fn id_exhaustion_never_reuses_a_deleted_or_failed_id() {
    let (transport, _, shared) = fake(config(), None, true);
    shared.test_next_id(u16::MAX as u32);
    let a = complete(&transport.try_submit(A).unwrap());
    assert_eq!(a.native_id, u16::MAX);
    complete(&transport.try_withdraw(A, W1).unwrap());
    assert!(matches!(
        transport.try_submit(B),
        Err(AdmissionError::IdExhausted)
    ));
    shared.event(Some(a.native_id), EventKind::Opened);
    assert_eq!(transport.pending_events(1)[0].attempt_id, Some(A));
    stop(&transport);
}

#[test]
fn startup_timeout_keeps_owner_and_stop_waits_for_actual_initialization_thread() {
    let (slow, entered, release) = gate();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let transport = NativeTransport::spawn_with(config(), move |_| {
        slow.wait();
        Ok::<_, Failure>(FakeOs {
            calls,
            gate: None,
            delete_gate: None,
            accepted: true,
            delete_ok: true,
        })
    })
    .unwrap();
    entered.recv_timeout(SECOND).unwrap();
    assert_eq!(transport.wait_ready(Duration::ZERO), ReadyOutcome::Unknown);
    assert!(matches!(
        transport.try_submit(A),
        Err(AdmissionError::NotReady)
    ));
    transport.request_stop();
    assert_eq!(transport.wait_stopped(Duration::ZERO), StopOutcome::Unknown);
    release.send(()).unwrap();
    assert_eq!(transport.wait_stopped(SECOND), StopOutcome::Joined);
}

#[test]
fn explorer_loss_and_resume_are_coalesced_control_events_and_never_resubmit() {
    let (transport, calls, shared) = fake(config(), None, true);
    complete(&transport.try_submit(A).unwrap());
    shared.event(None, EventKind::ExplorerLost);
    let first_explorer = transport.pending_control_events()[0];
    shared.event(None, EventKind::ExplorerLost);
    shared.event(None, EventKind::Resume);
    let first_resume = *transport.pending_control_events().last().unwrap();
    shared.event(None, EventKind::Resume);
    let events = transport.pending_events(32);
    assert_eq!(events.len(), 2);
    assert!(events[0].sequence > first_explorer.sequence);
    assert!(events[1].sequence > first_resume.sequence);
    transport.ack_events(first_resume.sequence);
    assert_eq!(transport.pending_control_events(), vec![events[1]]);
    assert!(events.iter().all(|event| event.attempt_id.is_none()));
    assert_eq!(
        transport.snapshot(A).unwrap().submission,
        Submission::SubmittedToOs
    );
    transport.ack_events(events.last().unwrap().sequence);
    shared.event(None, EventKind::Resume);
    assert_eq!(transport.pending_events(32).len(), 1);
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| matches!(call, Call::Submit(_)))
            .count(),
        1
    );
    stop(&transport);
}

#[test]
fn every_explorer_loss_retires_new_resources_even_before_older_event_ack() {
    let (transport, _, shared) = fake(config(), None, true);
    complete(&transport.try_submit(A).unwrap());
    shared.event(None, EventKind::ExplorerLost);
    complete(&transport.try_submit(B).unwrap());
    assert_eq!(transport.snapshot(B).unwrap().resource, Resource::Live);
    shared.event(None, EventKind::ExplorerLost);
    assert_eq!(transport.snapshot(B).unwrap().resource, Resource::Retired);
    assert_eq!(
        complete(&transport.try_submit(C).unwrap()).submission,
        Submission::SubmittedToOs
    );
    assert_eq!(transport.pending_events(32).len(), 1);
    stop(&transport);
}

#[test]
fn control_query_bypasses_unacknowledged_business_prefix_without_acknowledging() {
    let mut configuration = config();
    configuration.live_capacity = 8;
    let (transport, _, shared) = fake(configuration, None, true);
    for number in 1..=8u8 {
        complete(&transport.try_submit([number; 16]).unwrap());
        for kind in [
            EventKind::Shown,
            EventKind::Opened,
            EventKind::Hidden,
            EventKind::TimedOut,
        ] {
            shared.event(Some(u16::from(number)), kind);
        }
    }
    shared.event(None, EventKind::ExplorerLost);
    shared.event(None, EventKind::Resume);
    let prefix = transport.pending_events(32);
    assert_eq!(prefix.len(), 32);
    let controls = transport.pending_control_events();
    assert_eq!(
        controls.iter().map(|event| event.kind).collect::<Vec<_>>(),
        vec![EventKind::ExplorerLost, EventKind::Resume]
    );
    assert_eq!(transport.pending_events(32), prefix);
    assert_eq!(transport.pending_control_events(), controls);
    transport.ack_events(controls.last().unwrap().sequence);
    assert!(transport.pending_control_events().is_empty());
    stop(&transport);
}

#[test]
fn callback_cannot_open_a_queued_attempt_and_unknown_ids_are_ignored() {
    let (slow, entered, release) = gate();
    let (transport, _, shared) = fake(config(), Some(slow), true);
    transport.try_submit(A).unwrap();
    entered.recv_timeout(SECOND).unwrap();
    transport.try_submit(B).unwrap();
    shared.event(Some(2), EventKind::Opened);
    shared.event(Some(900), EventKind::Opened);
    assert!(transport.pending_events(32).is_empty());
    release.send(()).unwrap();
    stop(&transport);
}

#[test]
fn only_fixed_generic_text_can_reach_the_os_surface() {
    assert_eq!(NOTIFICATION_TITLE, "Soul");
    assert_eq!(
        NOTIFICATION_BODY,
        "有一项辅助事项需要查看，请打开 Soul 核对。"
    );
    for value in [NOTIFICATION_TITLE, NOTIFICATION_BODY] {
        assert!(!value.contains("http"));
        assert!(!value.contains("合成敏感目标"));
    }
    assert!(NOTIFICATION_BODY.encode_utf16().count() < 256);
    assert!(NOTIFICATION_TITLE.encode_utf16().count() < 64);
}

#[test]
fn initialization_failure_is_fixed_and_still_joined() {
    let transport =
        NativeTransport::spawn_with(config(), |_| Err::<FakeOs, _>(Failure::Initialize)).unwrap();
    assert_eq!(
        transport.wait_ready(SECOND),
        ReadyOutcome::Failed(Failure::Initialize)
    );
    assert_eq!(transport.wait_stopped(SECOND), StopOutcome::Joined);
}

#[cfg(not(windows))]
#[test]
fn real_non_windows_entry_does_not_fake_native_success() {
    let transport = NativeTransport::spawn(config()).unwrap();
    assert_eq!(
        transport.wait_ready(SECOND),
        ReadyOutcome::Failed(Failure::Unsupported)
    );
    assert_eq!(transport.wait_stopped(SECOND), StopOutcome::Joined);
}
