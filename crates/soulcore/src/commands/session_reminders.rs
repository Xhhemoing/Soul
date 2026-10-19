//! Explicit reminders and feedback. OS submission remains outside this module.
use super::*;
pub use soul_store_api::reminders::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAssistanceReminder {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
    pub expected_schedule_revision: Option<Uuid>,
    pub remind_at: Option<String>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetryAssistanceNotificationWithdrawal {
    pub attempt_id: Uuid,
    pub expected_withdrawal_id: Option<Uuid>,
}
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceFeedbackChange {
    Helpful,
    NotRelevant,
    Clear,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAssistanceFeedback {
    pub item_id: Uuid,
    pub expected_revision: Uuid,
    pub expected_feedback_revision: Option<Uuid>,
    pub value: AssistanceFeedbackChange,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetAssistanceCategoryPreference {
    pub category: AssistanceCategory,
    pub expected_revision: Option<Uuid>,
    pub muted: bool,
}
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceNotificationRuntimeState {
    NotStarted,
    Running,
    Unsupported,
    Failed,
    Stopping,
    Stopped,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssistanceNotificationRuntimeStatus {
    pub state: AssistanceNotificationRuntimeState,
    pub last_error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct AssistanceNotificationTarget {
    pub item_id: Uuid,
    pub current_revision: Uuid,
    pub available: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct NotificationDispatch {
    pub attempt_id: Uuid,
}
#[derive(Debug, Clone)]
pub(super) struct ReminderClock {
    pub(super) last: Option<(i64, Instant)>,
    pub(super) reset: bool,
    pub(super) enabled: bool,
    pub(super) permit: Arc<AtomicBool>,
}
impl Default for ReminderClock {
    fn default() -> Self {
        Self {
            last: None,
            reset: true,
            enabled: true,
            permit: Arc::new(AtomicBool::new(true)),
        }
    }
}
#[derive(Debug, Clone)]
pub(super) struct ActiveReminder {
    id: Uuid,
    candidate: ReminderCandidate,
    permit: Arc<AtomicBool>,
    unknown: bool,
}
#[derive(Debug, Clone)]
pub(super) struct PendingWithdrawalClaim {
    candidate: NotificationWithdrawal,
    token: Uuid,
}
pub struct SessionReminderWork {
    id: Uuid,
    candidate: ReminderCandidate,
    store: Arc<Mutex<SqlCipherStore>>,
    permit: Arc<AtomicBool>,
    started: Instant,
    wall_started: i64,
    lease: DraftLease,
}
pub struct SessionReminderCompletion {
    id: Uuid,
    result: Result<Option<NotificationDispatch>, SessionRefusal>,
    _lease: DraftLease,
}
impl std::fmt::Debug for SessionReminderWork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionReminderWork")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Debug for SessionReminderCompletion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionReminderCompletion")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}
fn refusal(code: &str) -> SessionRefusal {
    SessionRefusal {
        reason_code: code.into(),
        explanation: "提醒状态已变化或尚未核实，请重新读取；不会自动重复投递。".into(),
    }
}
fn now_ms() -> i64 {
    i64::try_from(now_unix_millis()).unwrap_or(i64::MAX)
}
/// Ceiling avoids firing early for RFC3339 fractions finer than a millisecond.
pub fn assistance_reminder_timestamp_ms(value: &str) -> Result<i64, SessionRefusal> {
    if value.ends_with("-00:00") {
        return Err(refusal("ASSISTANCE_REMINDER_TIME_INVALID"));
    }
    let seconds = soul_graph::t4d_adapt::rfc3339_to_unix(value)
        .map_err(|_| refusal("ASSISTANCE_REMINDER_TIME_INVALID"))?;
    let mut fraction = 0i64;
    if value.as_bytes().get(19) == Some(&b'.') {
        let digits: Vec<_> = value.as_bytes()[20..]
            .iter()
            .copied()
            .take_while(u8::is_ascii_digit)
            .collect();
        for i in 0..3 {
            fraction = fraction * 10 + i64::from(digits.get(i).copied().unwrap_or(b'0') - b'0');
        }
        if digits.iter().skip(3).any(|c| *c != b'0') {
            fraction += 1;
        }
    }
    seconds
        .checked_mul(1000)
        .and_then(|v| v.checked_add(fraction))
        .ok_or_else(|| refusal("ASSISTANCE_REMINDER_TIME_INVALID"))
}

impl SessionReminderWork {
    pub fn execute(self) -> SessionReminderCompletion {
        self.execute_inner(None)
    }
    #[doc(hidden)]
    pub fn execute_at(self, wall_ms: i64) -> SessionReminderCompletion {
        self.execute_inner(Some(wall_ms))
    }
    fn execute_inner(self, test_wall: Option<i64>) -> SessionReminderCompletion {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut store = hold(&self.store);
            store.transact(
                |store| -> Result<Option<NotificationDispatch>, SessionRefusal> {
                    let item = store.assistance_item(self.candidate.item_id)?;
                    let validation =
                        if item.revision == self.candidate.item_revision && item.is_current() {
                            assistance::validate(store, &item).map(|_| ())
                        } else {
                            Err(refusal("ASSISTANCE_CHANGED"))
                        };
                    // Re-read the real clocks after the heavy check, inside the
                    // same transaction that can create the durable dispatch grant.
                    let wall = test_wall.unwrap_or_else(now_ms);
                    let elapsed =
                        i64::try_from(self.started.elapsed().as_millis()).unwrap_or(i64::MAX);
                    let estimated = self.wall_started.saturating_add(elapsed);
                    let time_ok = wall >= self.candidate.due_ms
                        && wall.saturating_sub(self.candidate.due_ms) <= 5000
                        && elapsed <= 5000
                        && wall.abs_diff(estimated) <= 5000;
                    let failed = validation.as_ref().is_err_and(|e| {
                        !matches!(
                            e.reason_code.as_str(),
                            "ASSISTANCE_CHANGED" | "LOCAL_INSIGHT_UNAVAILABLE"
                        )
                    });
                    let eligible =
                        self.permit.load(Ordering::Acquire) && time_ok && validation.is_ok();
                    let attempt =
                        store.finish_reminder_check(&self.candidate, wall, eligible, failed)?;
                    Ok(attempt.map(|attempt_id| NotificationDispatch { attempt_id }))
                },
            )
        }))
        .unwrap_or_else(|_| Err(refusal("NOTIFICATION_CHECK_UNKNOWN")));
        SessionReminderCompletion {
            id: self.id,
            result,
            _lease: self.lease,
        }
    }
}

impl Session {
    pub fn assistance_reminder(
        &self,
        item_id: &str,
    ) -> Result<AssistanceReminderView, SessionRefusal> {
        let id = parse_id(item_id, "事项")?;
        let shared = self.opened_store()?;
        let result = hold(&shared).assistance_reminder(id)?;
        Ok(result)
    }
    pub fn set_assistance_reminder(
        &mut self,
        request: SetAssistanceReminder,
    ) -> Result<AssistanceReminderView, SessionRefusal> {
        self.set_assistance_reminder_at(request, now_ms())
    }
    #[doc(hidden)]
    pub fn set_assistance_reminder_at(
        &mut self,
        request: SetAssistanceReminder,
        now: i64,
    ) -> Result<AssistanceReminderView, SessionRefusal> {
        let due = request
            .remind_at
            .as_deref()
            .map(assistance_reminder_timestamp_ms)
            .transpose()?;
        if due.is_some_and(|due| due <= now) {
            return Err(refusal("ASSISTANCE_REMINDER_TIME_INVALID"));
        }
        let shared = self.opened_store()?;
        let result = hold(&shared).set_assistance_reminder(
            request.item_id,
            request.expected_revision,
            request.expected_schedule_revision,
            request.remind_at.as_deref(),
            due,
        )?;
        Ok(result)
    }
    pub fn assistance_feedback(
        &self,
        item_id: &str,
    ) -> Result<AssistanceFeedbackView, SessionRefusal> {
        let id = parse_id(item_id, "事项")?;
        let shared = self.opened_store()?;
        let result = hold(&shared).assistance_feedback(id)?;
        Ok(result)
    }
    pub fn set_assistance_feedback(
        &mut self,
        request: SetAssistanceFeedback,
    ) -> Result<AssistanceFeedbackView, SessionRefusal> {
        let value = match request.value {
            AssistanceFeedbackChange::Helpful => AssistanceFeedbackValue::Helpful,
            AssistanceFeedbackChange::NotRelevant => AssistanceFeedbackValue::NotRelevant,
            AssistanceFeedbackChange::Clear => AssistanceFeedbackValue::None,
        };
        let shared = self.opened_store()?;
        let result = hold(&shared).set_assistance_feedback(
            request.item_id,
            request.expected_revision,
            request.expected_feedback_revision,
            value,
        )?;
        Ok(result)
    }
    pub fn assistance_category_preferences(
        &self,
    ) -> Result<AssistanceCategoryPreferences, SessionRefusal> {
        let shared = self.opened_store()?;
        let store = hold(&shared);
        Ok(AssistanceCategoryPreferences {
            preferences: vec![
                store.assistance_category_preference(AssistanceCategory::ManualGoal)?,
                store.assistance_category_preference(AssistanceCategory::Observation)?,
            ],
        })
    }
    pub fn set_assistance_category_preference(
        &mut self,
        request: SetAssistanceCategoryPreference,
    ) -> Result<AssistanceCategoryPreference, SessionRefusal> {
        let shared = self.opened_store()?;
        let result = hold(&shared).set_assistance_category_preference(
            request.category,
            request.expected_revision,
            request.muted,
        )?;
        Ok(result)
    }
    pub fn retry_assistance_notification_withdrawal(
        &mut self,
        request: RetryAssistanceNotificationWithdrawal,
    ) -> Result<NotificationWithdrawal, SessionRefusal> {
        let shared = self.opened_store()?;
        let result = hold(&shared)
            .retry_notification_withdrawal(request.attempt_id, request.expected_withdrawal_id)?;
        Ok(result)
    }
    pub fn assistance_notification_runtime_status(&self) -> AssistanceNotificationRuntimeStatus {
        self.notification_runtime.clone()
    }
    pub fn record_assistance_notification_runtime_state(
        &mut self,
        state: AssistanceNotificationRuntimeState,
        code: Option<&str>,
    ) {
        self.notification_runtime = AssistanceNotificationRuntimeStatus {
            state,
            last_error: code.map(|c| match c {
                "NOTIFICATION_UNSUPPORTED"
                | "NOTIFICATION_START_FAILED"
                | "NOTIFICATION_TRANSPORT_FAILED"
                | "NOTIFICATION_STOP_UNKNOWN"
                | "NOTIFICATION_RUNTIME_PANICKED"
                | "NOTIFICATION_COORDINATION_FAILED" => c.into(),
                _ => "NOTIFICATION_TRANSPORT_FAILED".into(),
            }),
        };
    }
    pub fn reset_assistance_reminder_clock(&mut self) {
        self.reminder_clock.permit.store(false, Ordering::Release);
        self.reminder_clock.permit = Arc::new(AtomicBool::new(self.reminder_clock.enabled));
        self.reminder_clock.reset = true;
    }
    pub fn stop_assistance_reminder_scheduler(&mut self) {
        self.reminder_clock.enabled = false;
        self.reset_assistance_reminder_clock();
    }
    pub fn start_assistance_reminder_scheduler(&mut self) {
        self.reminder_clock.enabled = true;
        self.reset_assistance_reminder_clock();
    }
    pub fn expire_due_assistance_reminders(&mut self) -> Result<(), SessionRefusal> {
        let shared = self.opened_store()?;
        hold(&shared).expire_reminders(now_ms())?;
        Ok(())
    }
    pub fn begin_assistance_reminder_check(
        &mut self,
    ) -> Result<Option<SessionReminderWork>, SessionRefusal> {
        self.begin_assistance_reminder_check_at(now_ms(), Instant::now())
    }
    #[doc(hidden)]
    pub fn begin_assistance_reminder_check_at(
        &mut self,
        wall: i64,
        mono: Instant,
    ) -> Result<Option<SessionReminderWork>, SessionRefusal> {
        self.reconcile_assistance_reminder_check()?;
        let previous = self.reminder_clock.last.replace((wall, mono));
        let continuous = previous.is_some_and(|(last, instant)| {
            let elapsed = mono
                .checked_duration_since(instant)
                .and_then(|d| i64::try_from(d.as_millis()).ok());
            elapsed.is_some_and(|elapsed| {
                elapsed <= 5000
                    && wall >= last
                    && wall.saturating_sub(last).abs_diff(elapsed) <= 5000
            })
        });
        if !continuous {
            self.reset_assistance_reminder_clock();
        }
        if !self.reminder_clock.enabled
            || self.active_draft.is_some()
            || self.assistance_foreground_held()
        {
            return Ok(None);
        }
        let shared = self.opened_store()?;
        if self.reminder_clock.reset {
            hold(&shared).expire_reminders(wall)?;
            self.reminder_clock.reset = false;
            return Ok(None);
        }
        let Some((after, _)) = previous else {
            return Ok(None);
        };
        let candidate = {
            let mut store = hold(&shared);
            store.transact(|s| s.reminder_candidate(after, wall))?
        };
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        let id = Uuid::now_v7();
        let finished = Arc::new(AtomicBool::new(false));
        let permit = self.reminder_clock.permit.clone();
        self.active_draft = Some(ActiveDraft {
            id,
            kind: E1WorkerKind::ReminderCheck,
            finished: finished.clone(),
        });
        self.active_reminder = Some(ActiveReminder {
            id,
            candidate: candidate.clone(),
            permit: permit.clone(),
            unknown: false,
        });
        Ok(Some(SessionReminderWork {
            id,
            candidate,
            store: shared,
            permit,
            started: Instant::now(),
            wall_started: wall,
            lease: DraftLease(finished),
        }))
    }
    pub fn complete_assistance_reminder_check(
        &mut self,
        completed: SessionReminderCompletion,
    ) -> Result<Option<NotificationDispatch>, SessionRefusal> {
        if !self
            .active_draft
            .as_ref()
            .is_some_and(|a| a.id == completed.id && a.kind == E1WorkerKind::ReminderCheck)
        {
            return Err(refusal("NOTIFICATION_CHECK_UNKNOWN"));
        }
        match completed.result {
            Ok(result) => {
                let valid = self
                    .active_reminder
                    .as_ref()
                    .is_some_and(|a| a.permit.load(Ordering::Acquire));
                if !valid {
                    if let Some(active) = self.active_reminder.clone() {
                        let shared = match &self.store {
                            StoreHandle::Open(store) => store.clone(),
                            _ => return Err(refusal("NOTIFICATION_CHECK_UNKNOWN")),
                        };
                        hold(&shared).reconcile_reminder_check(&active.candidate)?;
                    }
                }
                self.active_reminder = None;
                self.active_draft = None;
                let _ = self.flush_deferred_revoke();
                Ok(if valid { result } else { None })
            }
            Err(error) => {
                if let Some(active) = &mut self.active_reminder {
                    active.unknown = true;
                }
                Err(error)
            }
        }
    }
    pub fn reconcile_assistance_reminder_check(&mut self) -> Result<(), SessionRefusal> {
        let Some(active) = self.active_reminder.clone() else {
            return Ok(());
        };
        if !self
            .active_draft
            .as_ref()
            .is_some_and(|a| a.id == active.id && a.finished.load(Ordering::Acquire))
        {
            return Ok(());
        }
        let result = (|| -> Result<(), SessionRefusal> {
            let shared = self.opened_store()?;
            let mut store = hold(&shared);
            if store.transaction_in_progress() {
                return Err(refusal("NOTIFICATION_CHECK_UNKNOWN"));
            }
            store.reconcile_reminder_check(&active.candidate)?;
            Ok(())
        })();
        if result.is_err() {
            if let Some(active) = &mut self.active_reminder {
                active.unknown = true;
            }
            return Err(refusal("NOTIFICATION_CHECK_UNKNOWN"));
        }
        self.active_reminder = None;
        self.active_draft = None;
        let _ = self.flush_deferred_revoke();
        Ok(())
    }
    pub fn authorize_assistance_notification_dispatch(
        &self,
        attempt: Uuid,
    ) -> Result<bool, SessionRefusal> {
        if !self.reminder_clock.enabled || !self.reminder_clock.permit.load(Ordering::Acquire) {
            return Ok(false);
        }
        let shared = self.opened_store()?;
        let result = hold(&shared).authorize_notification_dispatch(attempt)?;
        Ok(result)
    }
    pub fn record_assistance_notification_delivery(
        &mut self,
        attempt: Uuid,
        outcome: NotificationDelivery,
    ) -> Result<(), SessionRefusal> {
        let shared = self.opened_store()?;
        hold(&shared).record_notification_delivery(attempt, outcome, now_ms())?;
        Ok(())
    }
    pub fn record_assistance_notification_event(
        &mut self,
        attempt: Uuid,
        event: NotificationNativeEvent,
    ) -> Result<(), SessionRefusal> {
        let shared = self.opened_store()?;
        hold(&shared).record_notification_event(attempt, event, now_ms())?;
        Ok(())
    }
    pub fn resolve_assistance_notification(
        &self,
        attempt: Uuid,
    ) -> Result<AssistanceNotificationTarget, SessionRefusal> {
        let shared = self.opened_store()?;
        let store = hold(&shared);
        let attempt = store.notification_attempt(attempt)?;
        let item = store.assistance_item(attempt.item_id)?;
        Ok(AssistanceNotificationTarget {
            item_id: item.item_id,
            current_revision: item.revision,
            available: item.validity != AssistanceValidity::Forgotten,
        })
    }
    pub fn reconcile_assistance_notification_withdrawal_claim(
        &mut self,
    ) -> Result<(), SessionRefusal> {
        let Some(pending) = self.pending_withdrawal_claim.clone() else {
            return Ok(());
        };
        let shared = self.opened_store()?;
        hold(&shared)
            .reconcile_notification_withdrawal_claim(&pending.candidate, pending.token)
            .map_err(|_| refusal("NOTIFICATION_WITHDRAWAL_CLAIM_UNKNOWN"))?;
        self.pending_withdrawal_claim = None;
        Ok(())
    }
    pub fn claim_assistance_notification_withdrawal(
        &mut self,
    ) -> Result<Option<NotificationWithdrawal>, SessionRefusal> {
        if self.store_work_busy() {
            return Ok(None);
        }
        if self.pending_withdrawal_claim.is_some() {
            self.reconcile_assistance_notification_withdrawal_claim()?;
            return Ok(None);
        }
        let shared = self.opened_store()?;
        let mut store = hold(&shared);
        let Some(candidate) = store.pending_notification_withdrawal()? else {
            return Ok(None);
        };
        let token = Uuid::now_v7();
        self.pending_withdrawal_claim = Some(PendingWithdrawalClaim {
            candidate: candidate.clone(),
            token,
        });
        let result = store
            .claim_notification_withdrawal_exact(&candidate, token)
            .map_err(|_| refusal("NOTIFICATION_WITHDRAWAL_CLAIM_UNKNOWN"))?;
        self.pending_withdrawal_claim = None;
        Ok(Some(result))
    }
    pub fn complete_assistance_notification_withdrawal(
        &mut self,
        attempt: Uuid,
        withdrawal: Uuid,
        outcome: NotificationWithdrawalState,
    ) -> Result<(), SessionRefusal> {
        let shared = self.opened_store()?;
        hold(&shared).complete_notification_withdrawal(attempt, withdrawal, outcome)?;
        Ok(())
    }
}
