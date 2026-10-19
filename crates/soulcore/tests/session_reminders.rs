//! Explicit reminder, notification history and feedback fault boundaries.
use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
use soul_store_api::MemoryStore;
use soulcore::commands::memory::ForgetConfirmation;
use soulcore::commands::session::*;
use soulcore::commands::store::database_path;
use std::time::{Duration, Instant};
const BASE: &str = "2030-01-01T00:00:00Z";
fn base() -> i64 {
    assistance_reminder_timestamp_ms(BASE).unwrap()
}
fn setup() -> (tempfile::TempDir, Session, AssistanceItemRow) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Session::open(dir.path());
    let i = s
        .create_assistance_goal(CreateAssistanceGoal {
            title: "synthetic reminder goal".into(),
            summary: "synthetic reminder detail".into(),
            deadline_at: None,
            source_observation: None,
        })
        .unwrap()
        .item;
    (dir, s, i)
}
fn raw(dir: &std::path::Path) -> rusqlite::Connection {
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(dir.join(KEY_BLOB_FILE_NAME)))
    } else {
        Box::new(TestKeyProvider::in_dir(dir))
    };
    let c = rusqlite::Connection::open(database_path(dir)).unwrap();
    c.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    c
}
fn schedule(
    s: &mut Session,
    i: &AssistanceItemRow,
    expected: Option<AssistanceNotificationId>,
    at: &str,
) -> AssistanceReminderView {
    s.set_assistance_reminder_at(
        SetAssistanceReminder {
            item_id: i.item_id,
            expected_revision: i.revision,
            expected_schedule_revision: expected,
            remind_at: Some(at.into()),
        },
        base(),
    )
    .unwrap()
}
fn primed(s: &mut Session) -> Instant {
    let t = Instant::now();
    assert!(s
        .begin_assistance_reminder_check_at(base(), t)
        .unwrap()
        .is_none());
    t
}
fn attempt(
    s: &mut Session,
    i: &AssistanceItemRow,
) -> (AssistanceReminderView, AssistanceNotificationId) {
    let t = primed(s);
    let r = schedule(s, i, None, "2030-01-01T00:00:01Z");
    let w = s
        .begin_assistance_reminder_check_at(base() + 1000, t + Duration::from_secs(1))
        .unwrap()
        .unwrap();
    let a = s
        .complete_assistance_reminder_check(w.execute_at(base() + 1000))
        .unwrap()
        .unwrap()
        .attempt_id;
    (r, a)
}

#[test]
fn fraction_precision_never_fires_early_and_inputs_require_explicit_future_offset() {
    assert_eq!(
        assistance_reminder_timestamp_ms("2030-01-01T00:00:00.000Z").unwrap(),
        base()
    );
    assert_eq!(
        assistance_reminder_timestamp_ms("2030-01-01T08:00:00.000001+08:00").unwrap(),
        base() + 1
    );
    assert_eq!(
        assistance_reminder_timestamp_ms("2030-01-01T00:00:00.999999Z").unwrap(),
        base() + 1000
    );
    let (_dir, mut s, i) = setup();
    for at in [
        BASE,
        "tomorrow",
        "2030-01-01T00:00:02",
        "2030-01-01T00:00:02-00:00",
    ] {
        assert!(s
            .set_assistance_reminder_at(
                SetAssistanceReminder {
                    item_id: i.item_id,
                    expected_revision: i.revision,
                    expected_schedule_revision: None,
                    remind_at: Some(at.into())
                },
                base()
            )
            .is_err());
    }
    let t = primed(&mut s);
    schedule(&mut s, &i, None, "2030-01-01T00:00:01.000001Z");
    assert!(s
        .begin_assistance_reminder_check_at(base() + 1000, t + Duration::from_millis(1000))
        .unwrap()
        .is_none());
    let w = s
        .begin_assistance_reminder_check_at(base() + 1001, t + Duration::from_millis(1001))
        .unwrap()
        .unwrap();
    assert!(s
        .complete_assistance_reminder_check(w.execute_at(base() + 1001))
        .unwrap()
        .is_some());
}

#[test]
fn schedules_have_independent_cas_and_each_schedule_has_one_dispatch_permission() {
    let (_dir, mut s, i) = setup();
    let (r, a) = attempt(&mut s, &i);
    assert!(s.authorize_assistance_notification_dispatch(a).unwrap());
    assert!(s
        .set_assistance_reminder_at(
            SetAssistanceReminder {
                item_id: i.item_id,
                expected_revision: i.revision,
                expected_schedule_revision: None,
                remind_at: None
            },
            base()
        )
        .is_err());
    assert_eq!(
        s.assistance_item(&i.item_id.to_string())
            .unwrap()
            .item
            .revision,
        i.revision
    );
    let t = Instant::now();
    assert!(s
        .begin_assistance_reminder_check_at(base() + 2000, t)
        .unwrap()
        .is_none());
    let off = s
        .set_assistance_reminder_at(
            SetAssistanceReminder {
                item_id: i.item_id,
                expected_revision: i.revision,
                expected_schedule_revision: r.schedule_revision,
                remind_at: None,
            },
            base(),
        )
        .unwrap();
    assert_eq!(off.state, ReminderState::Off);
    assert!(!s.authorize_assistance_notification_dispatch(a).unwrap());
    assert!(s
        .claim_assistance_notification_withdrawal()
        .unwrap()
        .is_some());
}

#[test]
fn cancellation_after_claim_preserves_actual_submission_and_only_requests_withdrawal() {
    let (_dir, mut s, i) = setup();
    let (_r, a) = attempt(&mut s, &i);
    s.act_assistance_item(AssistanceActionRequest {
        item_id: i.item_id,
        expected_revision: i.revision,
        action: AssistanceAction::Cancel,
    })
    .unwrap();
    assert!(!s.authorize_assistance_notification_dispatch(a).unwrap());
    s.record_assistance_notification_delivery(a, NotificationDelivery::Unknown)
        .unwrap();
    s.record_assistance_notification_delivery(a, NotificationDelivery::SubmittedToOs)
        .unwrap();
    for event in [
        NotificationNativeEvent::Shown,
        NotificationNativeEvent::Opened,
        NotificationNativeEvent::Hidden,
        NotificationNativeEvent::TimedOut,
        NotificationNativeEvent::TrackingExpired,
    ] {
        s.record_assistance_notification_event(a, event).unwrap();
    }
    s.record_assistance_notification_delivery(a, NotificationDelivery::Unknown)
        .unwrap();
    let view = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    let history = view.latest_attempt.unwrap();
    assert_eq!(history.delivery, NotificationDelivery::SubmittedToOs);
    assert!(
        history.shown_at.is_some()
            && history.opened_at.is_some()
            && history.hidden_at.is_some()
            && history.timed_out_at.is_some()
            && history.tracking_expired_at.is_some()
    );
    let target = s.resolve_assistance_notification(a).unwrap();
    assert_eq!(target.item_id, i.item_id);
    assert_ne!(target.current_revision, i.revision);
    assert_eq!(
        s.assistance_item(&i.item_id.to_string())
            .unwrap()
            .item
            .handling,
        AssistanceHandling::Cancelled
    );
    assert_eq!(
        history.latest_withdrawal.unwrap().state,
        NotificationWithdrawalState::Pending
    );
}

#[test]
fn withdrawal_retries_are_new_generations_and_foreign_or_duplicate_receipts_cannot_forge_success() {
    let (_dir, mut s, i) = setup();
    let (_r, a) = attempt(&mut s, &i);
    let w1 = s
        .retry_assistance_notification_withdrawal(RetryAssistanceNotificationWithdrawal {
            attempt_id: a,
            expected_withdrawal_id: None,
        })
        .unwrap();
    let claimed = s
        .claim_assistance_notification_withdrawal()
        .unwrap()
        .unwrap();
    assert_eq!(claimed.withdrawal_id, w1.withdrawal_id);
    s.complete_assistance_notification_withdrawal(
        a,
        w1.withdrawal_id,
        NotificationWithdrawalState::Unknown,
    )
    .unwrap();
    let w2 = s
        .retry_assistance_notification_withdrawal(RetryAssistanceNotificationWithdrawal {
            attempt_id: a,
            expected_withdrawal_id: Some(w1.withdrawal_id),
        })
        .unwrap();
    assert_ne!(w1.withdrawal_id, w2.withdrawal_id);
    s.complete_assistance_notification_withdrawal(
        a,
        w1.withdrawal_id,
        NotificationWithdrawalState::Withdrawn,
    )
    .unwrap();
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .latest_attempt
            .unwrap()
            .latest_withdrawal
            .unwrap(),
        w2
    );
    s.claim_assistance_notification_withdrawal().unwrap();
    s.complete_assistance_notification_withdrawal(
        a,
        w2.withdrawal_id,
        NotificationWithdrawalState::NotPresent,
    )
    .unwrap();
    s.complete_assistance_notification_withdrawal(
        a,
        w2.withdrawal_id,
        NotificationWithdrawalState::NotPresent,
    )
    .unwrap();
    assert!(s
        .complete_assistance_notification_withdrawal(
            a,
            w2.withdrawal_id,
            NotificationWithdrawalState::Unknown
        )
        .is_err());
    let foreign = AssistanceNotificationId::now_v7();
    s.record_assistance_notification_delivery(a, NotificationDelivery::Unknown)
        .unwrap();
    s.record_assistance_notification_delivery(a, NotificationDelivery::Failed)
        .unwrap();
    s.record_assistance_notification_delivery(a, NotificationDelivery::Unknown)
        .unwrap();
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .latest_attempt
            .unwrap()
            .delivery,
        NotificationDelivery::Failed
    );
    s.record_assistance_notification_delivery(a, NotificationDelivery::SubmittedToOs)
        .unwrap();
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .latest_attempt
            .unwrap()
            .delivery,
        NotificationDelivery::SubmittedToOs
    );
    assert!(s
        .record_assistance_notification_delivery(foreign, NotificationDelivery::SubmittedToOs)
        .is_err());
    assert!(s
        .record_assistance_notification_event(foreign, NotificationNativeEvent::Opened)
        .is_err());
    assert!(s.resolve_assistance_notification(foreign).is_err());
    assert!(s
        .complete_assistance_notification_withdrawal(
            foreign,
            w2.withdrawal_id,
            NotificationWithdrawalState::Withdrawn
        )
        .is_err());
}

#[test]
fn withdrawal_claim_unknown_commit_coordinates_only_its_exact_owner_without_os_replay() {
    let (dir, mut s, i) = setup();
    let (_r, a) = attempt(&mut s, &i);
    let w1 = s
        .retry_assistance_notification_withdrawal(RetryAssistanceNotificationWithdrawal {
            attempt_id: a,
            expected_withdrawal_id: None,
        })
        .unwrap();
    let conn = raw(dir.path());
    let other = AssistanceNotificationId::now_v7();
    conn.execute("INSERT INTO notification_withdrawals(withdrawal_id,attempt_id,state,claim_token,created_order) VALUES(?1,?2,'claimed','other-real-owner',-1)",rusqlite::params![other.to_string(),a.to_string()]).unwrap();
    let point = format!(
        "soul::notification::withdrawal_claim_after_commit/{}",
        w1.withdrawal_id
    );
    fail::cfg(&point, "return").unwrap();
    let result = s.claim_assistance_notification_withdrawal();
    fail::remove(&point);
    assert!(result.is_err());
    let state = |id: AssistanceNotificationId| {
        conn.query_row(
            "SELECT state FROM notification_withdrawals WHERE withdrawal_id=?1",
            [id.to_string()],
            |r| r.get::<_, String>(0),
        )
        .unwrap()
    };
    assert_eq!(state(w1.withdrawal_id), "claimed");
    assert_eq!(state(other), "claimed");
    assert!(s
        .claim_assistance_notification_withdrawal()
        .unwrap()
        .is_none());
    assert_eq!(state(w1.withdrawal_id), "unknown");
    assert_eq!(state(other), "claimed");
    for _ in 0..3 {
        assert!(s
            .claim_assistance_notification_withdrawal()
            .unwrap()
            .is_none());
    }
    let w2 = s
        .retry_assistance_notification_withdrawal(RetryAssistanceNotificationWithdrawal {
            attempt_id: a,
            expected_withdrawal_id: Some(w1.withdrawal_id),
        })
        .unwrap();
    assert_eq!(
        s.claim_assistance_notification_withdrawal()
            .unwrap()
            .unwrap()
            .withdrawal_id,
        w2.withdrawal_id
    );
    s.complete_assistance_notification_withdrawal(
        a,
        w1.withdrawal_id,
        NotificationWithdrawalState::Withdrawn,
    )
    .unwrap();
    assert_eq!(state(w2.withdrawal_id), "claimed");
    assert_eq!(state(other), "claimed");
}

#[test]
fn withdrawal_claim_rolled_back_is_ended_explicitly_instead_of_hot_retrying() {
    let (dir, mut s, i) = setup();
    let (_r, a) = attempt(&mut s, &i);
    let w = s
        .retry_assistance_notification_withdrawal(RetryAssistanceNotificationWithdrawal {
            attempt_id: a,
            expected_withdrawal_id: None,
        })
        .unwrap();
    let conn = raw(dir.path());
    conn.execute_batch("CREATE TRIGGER fail_withdrawal_claim BEFORE UPDATE ON notification_withdrawals WHEN NEW.state='claimed' BEGIN SELECT RAISE(ABORT,'synthetic withdrawal failure'); END").unwrap();
    assert!(s.claim_assistance_notification_withdrawal().is_err());
    conn.execute_batch("DROP TRIGGER fail_withdrawal_claim")
        .unwrap();
    assert!(s
        .claim_assistance_notification_withdrawal()
        .unwrap()
        .is_none());
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .latest_attempt
            .unwrap()
            .latest_withdrawal
            .unwrap()
            .state,
        NotificationWithdrawalState::Unknown
    );
    assert!(s
        .claim_assistance_notification_withdrawal()
        .unwrap()
        .is_none());
    let next = s
        .retry_assistance_notification_withdrawal(RetryAssistanceNotificationWithdrawal {
            attempt_id: a,
            expected_withdrawal_id: Some(w.withdrawal_id),
        })
        .unwrap();
    assert_ne!(next.withdrawal_id, w.withdrawal_id);
}

#[test]
fn lost_completion_and_unknown_sql_claim_never_replay_the_os_attempt() {
    let (dir, mut s, i) = setup();
    let t = primed(&mut s);
    schedule(&mut s, &i, None, "2030-01-01T00:00:01Z");
    let w = s
        .begin_assistance_reminder_check_at(base() + 1000, t + Duration::from_secs(1))
        .unwrap()
        .unwrap();
    drop(w.execute_at(base() + 1000));
    s.reconcile_assistance_reminder_check().unwrap();
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .latest_attempt
            .unwrap()
            .delivery,
        NotificationDelivery::Unknown
    );
    let conn = raw(dir.path());
    let current = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    schedule(
        &mut s,
        &i,
        current.schedule_revision,
        "2030-01-01T00:01:02Z",
    );
    conn.execute_batch("CREATE TRIGGER fail_attempt BEFORE INSERT ON notification_attempts BEGIN SELECT RAISE(ABORT,'synthetic attempt failure'); END").unwrap();
    s.begin_assistance_reminder_check_at(base() + 61_000, t + Duration::from_secs(61))
        .unwrap(); // gap deliberately expires no backlog
    let current = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    schedule(
        &mut s,
        &i,
        current.schedule_revision,
        "2030-01-01T00:01:02Z",
    );
    let w = s
        .begin_assistance_reminder_check_at(base() + 62_000, t + Duration::from_secs(62))
        .unwrap()
        .unwrap();
    assert!(s
        .complete_assistance_reminder_check(w.execute_at(base() + 62_000))
        .is_err());
    s.reconcile_assistance_reminder_check().unwrap();
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string()).unwrap().state,
        ReminderState::CheckFailed
    );
    conn.execute_batch("DROP TRIGGER fail_attempt").unwrap();
    for n in 63..66 {
        assert!(s
            .begin_assistance_reminder_check_at(
                base() + n * 1000,
                t + Duration::from_secs(n as u64)
            )
            .unwrap()
            .is_none());
    }
    let count: i64 = conn
        .query_row("SELECT count(*) FROM notification_attempts", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn a_real_new_holder_recovers_notice_claims_once_and_never_replays_past_schedules() {
    let (dir, mut s, i) = setup();
    let (_r, a) = attempt(&mut s, &i);
    let withdrawal = s
        .retry_assistance_notification_withdrawal(RetryAssistanceNotificationWithdrawal {
            attempt_id: a,
            expected_withdrawal_id: None,
        })
        .unwrap();
    s.claim_assistance_notification_withdrawal()
        .unwrap()
        .unwrap();
    let expired = s
        .create_assistance_goal(CreateAssistanceGoal {
            title: "synthetic expired goal".into(),
            summary: "synthetic detail".into(),
            deadline_at: None,
            source_observation: None,
        })
        .unwrap()
        .item;
    s.set_assistance_reminder_at(
        SetAssistanceReminder {
            item_id: expired.item_id,
            expected_revision: expired.revision,
            expected_schedule_revision: None,
            remind_at: Some("2020-01-01T00:00:00Z".into()),
        },
        assistance_reminder_timestamp_ms("2019-01-01T00:00:00Z").unwrap(),
    )
    .unwrap();
    let future = s
        .create_assistance_goal(CreateAssistanceGoal {
            title: "synthetic future goal".into(),
            summary: "synthetic detail".into(),
            deadline_at: None,
            source_observation: None,
        })
        .unwrap()
        .item;
    let schedule = schedule(&mut s, &future, None, "2030-01-01T00:02:00Z");
    drop(s);
    let mut reopened = Session::open(dir.path());
    let history = reopened
        .assistance_reminder(&i.item_id.to_string())
        .unwrap()
        .latest_attempt
        .unwrap();
    assert_eq!(history.delivery, NotificationDelivery::Unknown);
    assert_eq!(
        history.latest_withdrawal.unwrap().withdrawal_id,
        withdrawal.withdrawal_id
    );
    assert_eq!(
        reopened
            .assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .latest_attempt
            .unwrap()
            .latest_withdrawal
            .unwrap()
            .state,
        NotificationWithdrawalState::Unknown
    );
    assert_eq!(
        reopened
            .assistance_reminder(&expired.item_id.to_string())
            .unwrap()
            .state,
        ReminderState::ExpiredInInbox
    );
    let retained = reopened
        .assistance_reminder(&future.item_id.to_string())
        .unwrap();
    assert_eq!(retained.schedule_revision, schedule.schedule_revision);
    assert_eq!(retained.state, ReminderState::Scheduled);
    assert!(reopened
        .begin_assistance_reminder_check()
        .unwrap()
        .is_none());
    assert!(reopened
        .claim_assistance_notification_withdrawal()
        .unwrap()
        .is_none());
    let conn = raw(dir.path());
    assert_eq!(
        conn.query_row("SELECT count(*) FROM notification_attempts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    // Recovery belongs to the new lease, not each command in that holder.
    conn.execute(
        "UPDATE notification_attempts SET delivery='claimed' WHERE attempt_id=?1",
        [a.to_string()],
    )
    .unwrap();
    assert_eq!(
        reopened
            .assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .latest_attempt
            .unwrap()
            .delivery,
        NotificationDelivery::Claimed
    );
    assert!(reopened
        .begin_assistance_reminder_check()
        .unwrap()
        .is_none());
    assert_eq!(
        reopened
            .assistance_reminder(&i.item_id.to_string())
            .unwrap()
            .latest_attempt
            .unwrap()
            .delivery,
        NotificationDelivery::Claimed
    );
    assert!(!reopened
        .audit()
        .unwrap()
        .entries
        .iter()
        .any(|e| e.action == "egress.request"));
}

#[test]
fn restart_resume_clock_gaps_and_minute_throttle_leave_overdue_work_in_inbox() {
    let (_dir, mut s, i) = setup();
    let t = primed(&mut s);
    let r = schedule(&mut s, &i, None, "2030-01-01T00:00:01Z");
    assert!(s
        .begin_assistance_reminder_check_at(base() + 6000, t + Duration::from_secs(6))
        .unwrap()
        .is_none());
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string()).unwrap().state,
        ReminderState::ExpiredInInbox
    );
    schedule(&mut s, &i, r.schedule_revision, "2030-01-01T00:00:07Z");
    s.reset_assistance_reminder_clock();
    assert!(s
        .begin_assistance_reminder_check_at(base() + 7000, t + Duration::from_secs(7))
        .unwrap()
        .is_none());
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string()).unwrap().state,
        ReminderState::ExpiredInInbox
    );
    let r = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    schedule(&mut s, &i, r.schedule_revision, "2030-01-01T00:00:08Z");
    let w = s
        .begin_assistance_reminder_check_at(base() + 8000, t + Duration::from_secs(8))
        .unwrap()
        .unwrap();
    s.complete_assistance_reminder_check(w.execute_at(base() + 8000))
        .unwrap()
        .unwrap();
    let r = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    schedule(&mut s, &i, r.schedule_revision, "2030-01-01T00:00:09Z");
    let w = s
        .begin_assistance_reminder_check_at(base() + 9000, t + Duration::from_secs(9))
        .unwrap()
        .unwrap();
    assert!(s
        .complete_assistance_reminder_check(w.execute_at(base() + 9000))
        .unwrap()
        .is_none());
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string()).unwrap().state,
        ReminderState::ExpiredInInbox
    );
}

#[test]
fn final_clock_window_expires_a_slow_check_even_when_scheduler_permission_remains_valid() {
    let (_dir, mut s, i) = setup();
    let t = primed(&mut s);
    schedule(&mut s, &i, None, "2030-01-01T00:00:01Z");
    let w = s
        .begin_assistance_reminder_check_at(base() + 1000, t + Duration::from_secs(1))
        .unwrap()
        .unwrap();
    // No stop, reset or source mutation: only the final real-wall budget fails.
    assert!(s
        .complete_assistance_reminder_check(w.execute_at(base() + 7001))
        .unwrap()
        .is_none());
    let r = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    assert_eq!(r.state, ReminderState::ExpiredInInbox);
    assert!(r.latest_attempt.is_none());
}

#[test]
fn slow_source_check_is_not_a_late_dispatch_and_stop_is_pure_memory_even_while_store_is_locked() {
    let (_dir, mut s, i) = setup();
    let t = primed(&mut s);
    schedule(&mut s, &i, None, "2030-01-01T00:00:01Z");
    let shared = s.store().unwrap();
    let w = s
        .begin_assistance_reminder_check_at(base() + 1000, t + Duration::from_secs(1))
        .unwrap()
        .unwrap();
    let guard = shared.lock().unwrap();
    let start = Instant::now();
    s.stop_assistance_reminder_scheduler();
    s.reset_assistance_reminder_clock();
    assert!(s.assistance_reminder(&i.item_id.to_string()).is_err());
    assert!(s
        .assistance_notification_runtime_status()
        .last_error
        .is_none());
    s.clear_user_endpoint();
    s.revoke_collect_consent().unwrap();
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(guard);
    assert!(s
        .complete_assistance_reminder_check(w.execute_at(base() + 8000))
        .unwrap()
        .is_none());
    let r = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    assert!(r.latest_attempt.is_none());
    assert_eq!(r.state, ReminderState::ExpiredInInbox);
}

#[test]
fn operational_source_error_is_check_failed_without_item_staleness_or_hot_retry() {
    let (dir, mut s, i) = setup();
    let t = primed(&mut s);
    schedule(&mut s, &i, None, "2030-01-01T00:00:01Z");
    let conn = raw(dir.path());
    let shared = s.store().unwrap();
    let AssistanceSource::ManualGoal { memory_id, .. } = i.source else {
        panic!("goal")
    };
    let blob = shared
        .lock()
        .unwrap()
        .get_memory(memory_id)
        .unwrap()
        .title_ref
        .unwrap()
        .blob_id
        .to_string();
    conn.execute("DELETE FROM sealed_blobs WHERE blob_id=?1", [blob])
        .unwrap();
    let w = s
        .begin_assistance_reminder_check_at(base() + 1000, t + Duration::from_secs(1))
        .unwrap()
        .unwrap();
    assert!(s
        .complete_assistance_reminder_check(w.execute_at(base() + 1000))
        .unwrap()
        .is_none());
    assert_eq!(
        shared.lock().unwrap().assistance_item(i.item_id).unwrap(),
        i
    );
    let r = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    assert_eq!(r.state, ReminderState::CheckFailed);
    assert!(r.latest_attempt.is_none());
    assert_eq!(
        r.failure_code.as_deref(),
        Some("NOTIFICATION_SOURCE_READ_FAILED")
    );
    for n in 2..5 {
        assert!(s
            .begin_assistance_reminder_check_at(
                base() + n * 1000,
                t + Duration::from_secs(n as u64)
            )
            .unwrap()
            .is_none());
    }
}

#[test]
fn feedback_and_category_preferences_are_independent_cas_and_restoring_a_category_never_backfills()
{
    let (_dir, mut s, i) = setup();
    let (_r, a) = attempt(&mut s, &i);
    let feedback = s
        .set_assistance_feedback(SetAssistanceFeedback {
            item_id: i.item_id,
            expected_revision: i.revision,
            expected_feedback_revision: None,
            value: AssistanceFeedbackChange::Helpful,
        })
        .unwrap();
    assert_eq!(feedback.value, AssistanceFeedbackValue::Helpful);
    assert_eq!(s.assistance_item(&i.item_id.to_string()).unwrap().item, i);
    assert!(s.authorize_assistance_notification_dispatch(a).unwrap());
    assert!(s
        .set_assistance_feedback(SetAssistanceFeedback {
            item_id: i.item_id,
            expected_revision: i.revision,
            expected_feedback_revision: None,
            value: AssistanceFeedbackChange::Clear
        })
        .is_err());
    let muted = s
        .set_assistance_category_preference(SetAssistanceCategoryPreference {
            category: AssistanceCategory::ManualGoal,
            expected_revision: None,
            muted: true,
        })
        .unwrap();
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string()).unwrap().state,
        ReminderState::Suppressed
    );
    assert!(!s.authorize_assistance_notification_dispatch(a).unwrap());
    assert!(s
        .set_assistance_category_preference(SetAssistanceCategoryPreference {
            category: AssistanceCategory::ManualGoal,
            expected_revision: None,
            muted: false
        })
        .is_err());
    s.set_assistance_category_preference(SetAssistanceCategoryPreference {
        category: AssistanceCategory::ManualGoal,
        expected_revision: muted.revision,
        muted: false,
    })
    .unwrap();
    assert_eq!(
        s.assistance_reminder(&i.item_id.to_string()).unwrap().state,
        ReminderState::Suppressed
    );
    assert_eq!(
        s.assistance_item(&i.item_id.to_string())
            .unwrap()
            .item
            .handling,
        AssistanceHandling::Open
    );
    assert!(!s
        .audit()
        .unwrap()
        .entries
        .iter()
        .any(|e| e.action == "egress.request"));
}

#[test]
fn forget_scope_checks_same_count_schedule_changes_and_destroys_content_despite_bad_auxiliary_ids()
{
    let (dir, mut s, i) = setup();
    let (_r, a) = attempt(&mut s, &i);
    let AssistanceSource::ManualGoal { memory_id, .. } = i.source else {
        panic!("goal")
    };
    s.set_assistance_feedback(SetAssistanceFeedback {
        item_id: i.item_id,
        expected_revision: i.revision,
        expected_feedback_revision: None,
        value: AssistanceFeedbackChange::NotRelevant,
    })
    .unwrap();
    let quote = s.preview_forget(&memory_id.to_string()).unwrap();
    let r = s.assistance_reminder(&i.item_id.to_string()).unwrap();
    schedule(&mut s, &i, r.schedule_revision, "2030-01-01T00:02:00Z");
    assert!(s
        .forget_memory(&ForgetConfirmation {
            preview_id: quote.preview_id,
            memory_id: memory_id.to_string()
        })
        .is_err());
    let conn = raw(dir.path());
    conn.execute(
        "UPDATE notification_attempts SET attempt_id='bad-attempt-id' WHERE attempt_id=?1",
        [a.to_string()],
    )
    .unwrap();
    conn.execute_batch("UPDATE notification_withdrawals SET withdrawal_id='bad-withdrawal-id',attempt_id='bad-attempt-id'; UPDATE assistance_feedback SET revision='bad-feedback-revision'; UPDATE assistance_reminders SET schedule_revision='bad-schedule-revision'").unwrap();
    let quote = s.preview_forget(&memory_id.to_string()).unwrap();
    assert!(
        s.forget_memory(&ForgetConfirmation {
            preview_id: quote.preview_id,
            memory_id: memory_id.to_string()
        })
        .unwrap()
        .logical_committed
    );
    assert!(s.memory(&memory_id.to_string()).is_err());
    let count: i64 = conn
        .query_row("SELECT count(*) FROM assistance_feedback", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    let at: Option<String> = conn
        .query_row("SELECT remind_at FROM assistance_reminders", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(at.is_none());
}
