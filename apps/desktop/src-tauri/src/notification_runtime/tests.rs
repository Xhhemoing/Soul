use super::*;
use crate::notification_transport::Resource;
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::session::*;
use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

struct FakeTransport {
    phase: Phase,
    receipts: RefCell<BTreeMap<AttemptId, Receipt>>,
    withdrawals: RefCell<BTreeMap<WithdrawalId, WithdrawalReceipt>>,
    events: RefCell<Vec<NativeEvent>>,
    submissions: RefCell<Vec<AttemptId>>,
}
impl Default for FakeTransport {
    fn default() -> Self {
        Self {
            phase: Phase::Ready,
            receipts: RefCell::new(BTreeMap::new()),
            withdrawals: RefCell::new(BTreeMap::new()),
            events: RefCell::new(Vec::new()),
            submissions: RefCell::new(Vec::new()),
        }
    }
}
impl Transport for FakeTransport {
    fn phase(&self) -> Phase {
        self.phase
    }
    fn submit(&self, id: AttemptId) -> Result<(), AdmissionError> {
        self.submissions.borrow_mut().push(id);
        self.receipts.borrow_mut().insert(
            id,
            Receipt {
                attempt_id: id,
                native_id: 1,
                submission: Submission::InFlight,
                withdrawal: Withdrawal::NotRequested,
                withdrawal_id: None,
                resource: Resource::Reserved,
                expired: false,
            },
        );
        Ok(())
    }
    fn withdraw(&self, attempt: AttemptId, withdrawal: WithdrawalId) -> Result<(), AdmissionError> {
        self.withdrawals.borrow_mut().insert(
            withdrawal,
            WithdrawalReceipt {
                withdrawal_id: withdrawal,
                attempt_id: attempt,
                state: Withdrawal::InFlight,
            },
        );
        Ok(())
    }
    fn receipt(&self, id: AttemptId) -> Option<Receipt> {
        self.receipts.borrow().get(&id).copied()
    }
    fn withdrawal_receipt(&self, id: WithdrawalId) -> Option<WithdrawalReceipt> {
        self.withdrawals.borrow().get(&id).copied()
    }
    fn events(&self, limit: usize) -> Vec<NativeEvent> {
        self.events.borrow().iter().take(limit).copied().collect()
    }
    fn controls(&self) -> Vec<NativeEvent> {
        self.events
            .borrow()
            .iter()
            .filter(|e| e.attempt_id.is_none())
            .copied()
            .collect()
    }
    fn ack(&self, seq: u64) {
        self.events.borrow_mut().retain(|e| e.sequence > seq);
    }
}
fn setup() -> (tempfile::TempDir, Arc<Mutex<Session>>, AssistanceItemRow) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    let item = session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "Synthetic notification goal".into(),
            summary: "Synthetic local preparation".into(),
            deadline_at: None,
            source_observation: None,
        })
        .unwrap()
        .item;
    (directory, Arc::new(Mutex::new(session)), item)
}
fn timestamp(ms: i64) -> String {
    let seconds = soul_policy::clock::rfc3339_utc(ms.div_euclid(1000));
    format!(
        "{}.{:03}Z",
        seconds.trim_end_matches('Z'),
        ms.rem_euclid(1000)
    )
}
fn schedule(session: &Arc<Mutex<Session>>, item: &AssistanceItemRow, now: i64, due: i64) {
    let mut session = held(session);
    let expected = session
        .assistance_reminder(&item.item_id.to_string())
        .unwrap()
        .schedule_revision;
    session
        .set_assistance_reminder_at(
            SetAssistanceReminder {
                item_id: item.item_id,
                expected_revision: item.revision,
                expected_schedule_revision: expected,
                remind_at: Some(timestamp(due)),
            },
            now,
        )
        .unwrap();
}
fn attempt(session: &Arc<Mutex<Session>>, item: &AssistanceItemRow) -> Uuid {
    let start = 1_800_000_000_000;
    schedule(session, item, start, start + 1);
    let mono = Instant::now();
    assert!(held(session)
        .begin_assistance_reminder_check_at(start, mono)
        .unwrap()
        .is_none());
    let work = held(session)
        .begin_assistance_reminder_check_at(start + 1, mono + Duration::from_millis(1))
        .unwrap()
        .unwrap();
    let completion = work.execute_at(start + 1);
    held(session)
        .complete_assistance_reminder_check(completion)
        .unwrap()
        .unwrap()
        .attempt_id
}
fn latest(
    session: &Arc<Mutex<Session>>,
    item: &AssistanceItemRow,
) -> AssistanceNotificationAttempt {
    held(session)
        .assistance_reminder(&item.item_id.to_string())
        .unwrap()
        .latest_attempt
        .unwrap()
}

#[test]
fn production_tick_submits_once_and_late_receipt_replaces_unknown_without_replay() {
    let (_directory, session, item) = setup();
    let endpoint = MockLlm::start().unwrap();
    held(&session)
        .set_user_endpoint(&endpoint.base_url())
        .unwrap();
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    schedule(&session, &item, now, now + 150);
    let os = FakeTransport::default();
    let mut driver = Driver::default();
    let stop = AtomicBool::new(false);
    driver.step(&session, &os, &stop, &mut |_| panic!("no open event"));
    thread::sleep(Duration::from_millis(180));
    driver.step(&session, &os, &stop, &mut |_| panic!("no open event"));
    assert_eq!(os.submissions.borrow().len(), 1);
    let id = latest(&session, &item).attempt_id;
    driver.deliveries.get_mut(&id).unwrap().started = Instant::now() - RECEIPT_WAIT;
    driver.flush(&session, &os, &mut |_| {});
    assert_eq!(
        latest(&session, &item).delivery,
        NotificationDelivery::Unknown
    );
    assert!(driver.deliveries.contains_key(&id));
    os.receipts
        .borrow_mut()
        .get_mut(id.as_bytes())
        .unwrap()
        .submission = Submission::SubmittedToOs;
    driver.step(&session, &os, &stop, &mut |_| {});
    driver.step(&session, &os, &stop, &mut |_| {});
    assert_eq!(
        latest(&session, &item).delivery,
        NotificationDelivery::SubmittedToOs
    );
    assert!(!driver.deliveries.contains_key(&id));
    assert_eq!(os.submissions.borrow().len(), 1);
    assert_eq!(endpoint.request_count(), 0);
}

#[test]
fn old_withdrawal_late_success_does_not_overwrite_explicit_retry_generation() {
    let (_directory, session, item) = setup();
    let id = attempt(&session, &item);
    let reminder = held(&session)
        .assistance_reminder(&item.item_id.to_string())
        .unwrap();
    held(&session)
        .set_assistance_reminder(SetAssistanceReminder {
            item_id: item.item_id,
            expected_revision: item.revision,
            expected_schedule_revision: reminder.schedule_revision,
            remind_at: None,
        })
        .unwrap();
    let os = FakeTransport::default();
    let mut driver = Driver::default();
    let stop = AtomicBool::new(false);
    driver.step(&session, &os, &stop, &mut |_| {});
    let old = latest(&session, &item)
        .latest_withdrawal
        .unwrap()
        .withdrawal_id;
    driver.withdrawals.get_mut(&old).unwrap().started = Instant::now() - RECEIPT_WAIT;
    driver.flush(&session, &os, &mut |_| {});
    assert_eq!(
        latest(&session, &item).latest_withdrawal.unwrap().state,
        NotificationWithdrawalState::Unknown
    );
    let new = held(&session)
        .retry_assistance_notification_withdrawal(RetryAssistanceNotificationWithdrawal {
            attempt_id: id,
            expected_withdrawal_id: Some(old),
        })
        .unwrap()
        .withdrawal_id;
    driver.step(&session, &os, &stop, &mut |_| {});
    os.withdrawals
        .borrow_mut()
        .get_mut(old.as_bytes())
        .unwrap()
        .state = Withdrawal::Withdrawn;
    os.withdrawals
        .borrow_mut()
        .get_mut(new.as_bytes())
        .unwrap()
        .state = Withdrawal::Failed(Failure::OsRejected);
    driver.flush(&session, &os, &mut |_| {});
    let latest = latest(&session, &item).latest_withdrawal.unwrap();
    assert_eq!(latest.withdrawal_id, new);
    assert_eq!(latest.state, NotificationWithdrawalState::Failed);
    // Same old terminal receipt is idempotent and cannot target the new UUID.
    held(&session)
        .complete_assistance_notification_withdrawal(
            id,
            old,
            NotificationWithdrawalState::Withdrawn,
        )
        .unwrap();
    assert!(os.submissions.borrow().is_empty());
}

#[test]
fn busy_receipt_prefix_retains_events_but_cannot_hide_resume_control() {
    let (_directory, session, item) = setup();
    let id = attempt(&session, &item);
    let work = held(&session).begin_assistance_work().unwrap().unwrap();
    let os = FakeTransport::default();
    // The seam represents a backlog larger than one processing batch; the OS
    // state machine separately tests its per-attempt event deduplication.
    *os.events.borrow_mut() = (1..=40)
        .map(|sequence| NativeEvent {
            sequence,
            attempt_id: Some(*id.as_bytes()),
            kind: EventKind::Shown,
        })
        .chain([NativeEvent {
            sequence: 41,
            attempt_id: None,
            kind: EventKind::Resume,
        }])
        .collect();
    let mut driver = Driver::default();
    driver.flush(&session, &os, &mut |_| {});
    assert_eq!(os.events.borrow().len(), 41);
    assert!(driver.observed_controls.contains(&41));
    driver.flush(&session, &os, &mut |_| {});
    assert_eq!(driver.observed_controls.len(), 1);
    let completion = work.execute();
    held(&session).complete_assistance_work(completion).unwrap();
    driver.flush(&session, &os, &mut |_| {});
    driver.flush(&session, &os, &mut |_| {});
    assert!(os.events.borrow().is_empty());
    assert!(latest(&session, &item).shown_at.is_some());
}

#[test]
fn opened_event_only_navigates_current_identity_and_shutdown_suppresses_navigation() {
    let (_directory, session, item) = setup();
    let id = attempt(&session, &item);
    let endpoint = MockLlm::start().unwrap();
    held(&session)
        .set_user_endpoint(&endpoint.base_url())
        .unwrap();
    let os = FakeTransport::default();
    os.events.borrow_mut().push(NativeEvent {
        sequence: 1,
        attempt_id: Some(*id.as_bytes()),
        kind: EventKind::Opened,
    });
    let mut driver = Driver::default();
    let mut opened = Vec::new();
    driver.flush(&session, &os, &mut |item| opened.push(item));
    assert_eq!(opened, vec![item.item_id]);
    assert!(latest(&session, &item).opened_at.is_some());
    assert_eq!(
        held(&session)
            .assistance_feedback(&item.item_id.to_string())
            .unwrap()
            .value,
        AssistanceFeedbackValue::None
    );
    assert_eq!(
        held(&session)
            .assistance_item(&item.item_id.to_string())
            .unwrap()
            .item
            .revision,
        item.revision
    );
    os.events.borrow_mut().push(NativeEvent {
        sequence: 2,
        attempt_id: Some(*id.as_bytes()),
        kind: EventKind::Opened,
    });
    driver.step(&session, &os, &AtomicBool::new(true), &mut |_| {
        panic!("shutdown must not reveal a window")
    });
    assert!(os.events.borrow().is_empty());
    assert_eq!(endpoint.request_count(), 0);
}

#[test]
fn stop_while_waiting_for_session_does_not_claim_due_work() {
    let (_directory, session, item) = setup();
    let now = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    schedule(&session, &item, now, now + 100);
    let stop = Arc::new(AtomicBool::new(false));
    let worker_session = session.clone();
    let worker_stop = stop.clone();
    let guard = held(&session);
    let (tx, rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        let os = FakeTransport::default();
        tx.send(()).unwrap();
        Driver::default().step(&worker_session, &os, &worker_stop, &mut |_| {});
        assert!(os.submissions.borrow().is_empty());
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap();
    stop.store(true, Ordering::Release);
    drop(guard);
    worker.join().unwrap();
    assert!(held(&session)
        .assistance_reminder(&item.item_id.to_string())
        .unwrap()
        .latest_attempt
        .is_none());
}

#[test]
fn unavailable_transport_keeps_overdue_notice_in_inbox_without_claiming() {
    let (_directory, session, item) = setup();
    schedule(&session, &item, 1_700_000_000_000, 1_700_000_000_001);
    let os = FakeTransport {
        phase: Phase::Failed(Failure::Unsupported),
        ..Default::default()
    };
    Driver::default().step(&session, &os, &AtomicBool::new(false), &mut |_| {});
    let view = held(&session)
        .assistance_reminder(&item.item_id.to_string())
        .unwrap();
    assert_eq!(view.state, ReminderState::ExpiredInInbox);
    assert!(view.latest_attempt.is_none());
    assert!(os.submissions.borrow().is_empty());
}

#[cfg(not(windows))]
#[test]
fn real_unsupported_owner_reports_platform_limit_and_joins_before_stopped() {
    let (_directory, session, _item) = setup();
    let runtime = NotificationRuntime::start(session.clone(), |_| {
        panic!("unsupported OS cannot open a notification")
    });
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let state = held(&session).assistance_notification_runtime_status();
        if state.state == AssistanceNotificationRuntimeState::Unsupported {
            assert_eq!(
                state.last_error.as_deref(),
                Some("NOTIFICATION_UNSUPPORTED")
            );
            break;
        }
        assert!(Instant::now() < until);
        thread::sleep(Duration::from_millis(2));
    }
    runtime.request_stop();
    runtime.shutdown();
    runtime.shutdown();
    assert_eq!(
        held(&session)
            .assistance_notification_runtime_status()
            .state,
        AssistanceNotificationRuntimeState::Stopped
    );
}
