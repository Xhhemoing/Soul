//! Reminder/feedback metadata in the same transactions as their domain items.
use crate::store::{backend, enum_text, SqlCipherStore};
use rusqlite::{params, Connection, OptionalExtension};
use serde::de::DeserializeOwned;
use soul_store_api::assistance::{AssistanceItem, AssistanceSource, AssistanceValidity};
use soul_store_api::reminders::*;
use soul_store_api::types::{StoreError, StoreResult};
use uuid::Uuid;
fn invalid() -> StoreError {
    StoreError::ContractViolation("ASSISTANCE_REMINDER_CHANGED".into())
}
fn word<T: DeserializeOwned>(value: String) -> StoreResult<T> {
    serde_json::from_value(serde_json::Value::String(value)).map_err(|_| invalid())
}
fn id(value: String) -> StoreResult<Uuid> {
    value.parse().map_err(|_| invalid())
}
fn category(item: &AssistanceItem) -> AssistanceCategory {
    match item.source {
        AssistanceSource::ManualGoal { .. } => AssistanceCategory::ManualGoal,
        AssistanceSource::Observation { .. } => AssistanceCategory::Observation,
    }
}

fn insert_withdrawal(conn: &Connection, attempt: &str) -> StoreResult<()> {
    conn.execute("INSERT INTO notification_withdrawals(withdrawal_id,attempt_id,state,created_order) SELECT ?1,?2,'pending',COALESCE(max(created_order),0)+1 FROM notification_withdrawals",params![Uuid::now_v7().to_string(),attempt]).map_err(backend)?;
    Ok(())
}
pub(crate) fn request_withdrawal(conn: &Connection, attempt: &str) -> StoreResult<()> {
    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM notification_withdrawals WHERE attempt_id=?1)",
            [attempt],
            |r| r.get(0),
        )
        .map_err(backend)?;
    // Repeated domain changes never automatically retry failed/unknown removes.
    if !exists {
        insert_withdrawal(conn, attempt)?;
    }
    Ok(())
}
pub(crate) fn invalidate_item(conn: &Connection, item: &str) -> StoreResult<()> {
    let obsolete:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM assistance_reminders r JOIN assistance_items i ON i.item_id=r.item_id WHERE r.item_id=?1 AND (r.item_revision!=i.revision OR i.handling!='open' OR i.validity!='current'))",[item],|r|r.get(0)).map_err(backend)?;
    if obsolete {
        conn.execute("UPDATE assistance_reminders SET state='invalidated',remind_at=NULL,due_ms=NULL,failure_code=NULL WHERE item_id=?1 AND state!='off'",[item]).map_err(backend)?;
        withdraw_item(conn, item)?;
    }
    Ok(())
}
fn withdraw_item(conn: &Connection, item: &str) -> StoreResult<()> {
    let mut statement=conn.prepare("SELECT attempt_id FROM notification_attempts WHERE item_id=?1 AND (delivery!='failed' OR shown_at IS NOT NULL)").map_err(backend)?;
    let ids = statement
        .query_map([item], |r| r.get::<_, String>(0))
        .map_err(backend)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(backend)?;
    drop(statement);
    for id in ids {
        request_withdrawal(conn, &id)?;
    }
    Ok(())
}
pub(crate) fn forget_item(conn: &Connection, item: &str) -> StoreResult<()> {
    conn.execute("UPDATE assistance_reminders SET state='invalidated',remind_at=NULL,due_ms=NULL,failure_code=NULL WHERE item_id=?1",[item]).map_err(backend)?;
    conn.execute("DELETE FROM assistance_feedback WHERE item_id=?1", [item])
        .map_err(backend)?;
    withdraw_item(conn, item)
}

impl SqlCipherStore {
    pub(crate) fn recover_reminders(&self, now: i64) -> StoreResult<()> {
        self.conn
            .execute(
                "UPDATE notification_attempts SET delivery='unknown' WHERE delivery='claimed'",
                [],
            )
            .map_err(backend)?;
        self.conn
            .execute(
                "UPDATE notification_withdrawals SET state='unknown' WHERE state='claimed'",
                [],
            )
            .map_err(backend)?;
        self.expire_reminders(now)
    }
    pub fn expire_reminders(&self, now: i64) -> StoreResult<()> {
        self.conn.execute("UPDATE assistance_reminders SET state='expired_in_inbox' WHERE state='scheduled' AND due_ms<=?1",[now]).map_err(backend)?;
        Ok(())
    }
    pub fn assistance_category_preference(
        &self,
        kind: AssistanceCategory,
    ) -> StoreResult<AssistanceCategoryPreference> {
        let row: Option<(String, bool)> = self
            .conn
            .query_row(
                "SELECT revision,muted FROM assistance_category_preferences WHERE category=?1",
                [enum_text(&kind)?],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(backend)?;
        let (revision, muted) = match row {
            Some((r, m)) => (Some(id(r)?), m),
            None => (None, false),
        };
        Ok(AssistanceCategoryPreference {
            category: kind,
            revision,
            muted,
        })
    }
    pub fn set_assistance_category_preference(
        &mut self,
        kind: AssistanceCategory,
        expected: Option<Uuid>,
        muted: bool,
    ) -> StoreResult<AssistanceCategoryPreference> {
        self.transact(|store|{
            if store.assistance_category_preference(kind)?.revision!=expected{return Err(invalid())}
            store.conn.execute("INSERT INTO assistance_category_preferences(category,revision,muted) VALUES(?1,?2,?3) ON CONFLICT(category) DO UPDATE SET revision=excluded.revision,muted=excluded.muted",params![enum_text(&kind)?,Uuid::now_v7().to_string(),muted]).map_err(backend)?;
            if muted {
                let kind=enum_text(&kind)?;
                store.conn.execute("UPDATE assistance_reminders SET state='suppressed' WHERE state='scheduled' AND item_id IN (SELECT item_id FROM assistance_items WHERE kind=?1)",[&kind]).map_err(backend)?;
                let mut statement=store.conn.prepare("SELECT item_id FROM assistance_items WHERE kind=?1").map_err(backend)?;
                let ids=statement.query_map([kind],|r|r.get::<_,String>(0)).map_err(backend)?.collect::<Result<Vec<_>,_>>().map_err(backend)?;drop(statement);
                for item in ids {withdraw_item(&store.conn,&item)?;}
            }
            store.assistance_category_preference(kind)
        })
    }
    pub fn assistance_feedback(&self, item_id: Uuid) -> StoreResult<AssistanceFeedbackView> {
        let item = self.assistance_item(item_id)?;
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT revision,value FROM assistance_feedback WHERE item_id=?1",
                [item_id.to_string()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(backend)?;
        let (revision, value) = match row {
            Some((r, v)) => (Some(id(r)?), word(v)?),
            None => (None, AssistanceFeedbackValue::None),
        };
        Ok(AssistanceFeedbackView {
            item_id,
            item_revision: item.revision,
            feedback_revision: revision,
            value,
        })
    }
    pub fn set_assistance_feedback(
        &mut self,
        item: Uuid,
        item_revision: Uuid,
        expected: Option<Uuid>,
        value: AssistanceFeedbackValue,
    ) -> StoreResult<AssistanceFeedbackView> {
        self.transact(|store|{
            let actual=store.assistance_item(item)?;
            if actual.revision!=item_revision || actual.validity==AssistanceValidity::Forgotten || store.assistance_feedback(item)?.feedback_revision!=expected{return Err(invalid())}
            store.conn.execute("INSERT INTO assistance_feedback(item_id,revision,value) VALUES(?1,?2,?3) ON CONFLICT(item_id) DO UPDATE SET revision=excluded.revision,value=excluded.value",params![item.to_string(),Uuid::now_v7().to_string(),enum_text(&value)?]).map_err(backend)?;
            store.assistance_feedback(item)
        })
    }
    fn withdrawal(&self, withdrawal: &str) -> StoreResult<NotificationWithdrawal> {
        let row:(String,String,String)=self.conn.query_row("SELECT withdrawal_id,attempt_id,state FROM notification_withdrawals WHERE withdrawal_id=?1",[withdrawal],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(backend)?;
        Ok(NotificationWithdrawal {
            withdrawal_id: id(row.0)?,
            attempt_id: id(row.1)?,
            state: word(row.2)?,
        })
    }
    pub fn latest_notification_withdrawal(
        &self,
        attempt: Uuid,
    ) -> StoreResult<Option<NotificationWithdrawal>> {
        let row:Option<String>=self.conn.query_row("SELECT withdrawal_id FROM notification_withdrawals WHERE attempt_id=?1 ORDER BY created_order DESC LIMIT 1",[attempt.to_string()],|r|r.get(0)).optional().map_err(backend)?;
        row.map(|r| self.withdrawal(&r)).transpose()
    }
    pub fn notification_attempt(
        &self,
        attempt: Uuid,
    ) -> StoreResult<AssistanceNotificationAttempt> {
        let row=self.conn.query_row("SELECT item_id,item_revision,schedule_revision,delivery,shown_at,opened_at,hidden_at,timed_out_at,tracking_expired_at FROM notification_attempts WHERE attempt_id=?1",[attempt.to_string()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?))).optional().map_err(backend)?.ok_or_else(||StoreError::not_found("notification attempt",attempt))?;
        Ok(AssistanceNotificationAttempt {
            attempt_id: attempt,
            item_id: id(row.0)?,
            item_revision: id(row.1)?,
            schedule_revision: id(row.2)?,
            delivery: word(row.3)?,
            shown_at: row.4,
            opened_at: row.5,
            hidden_at: row.6,
            timed_out_at: row.7,
            tracking_expired_at: row.8,
            latest_withdrawal: self.latest_notification_withdrawal(attempt)?,
        })
    }
    pub fn assistance_reminder(&self, item_id: Uuid) -> StoreResult<AssistanceReminderView> {
        let item = self.assistance_item(item_id)?;
        let row:Option<(String,String,Option<String>,Option<String>)>=self.conn.query_row("SELECT schedule_revision,state,remind_at,failure_code FROM assistance_reminders WHERE item_id=?1",[item_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(backend)?;
        let (revision, state, at, failure) = match row {
            Some((r, s, a, f)) => (Some(id(r)?), word(s)?, a, f),
            None => (None, ReminderState::Off, None, None),
        };
        let attempt:Option<String>=self.conn.query_row("SELECT attempt_id FROM notification_attempts WHERE item_id=?1 ORDER BY claimed_at DESC,rowid DESC LIMIT 1",[item_id.to_string()],|r|r.get(0)).optional().map_err(backend)?;
        Ok(AssistanceReminderView {
            item_id,
            item_revision: item.revision,
            schedule_revision: revision,
            state,
            remind_at: at,
            latest_attempt: attempt
                .map(|a| self.notification_attempt(id(a)?))
                .transpose()?,
            failure_code: failure,
        })
    }
    pub fn set_assistance_reminder(
        &mut self,
        item_id: Uuid,
        item_revision: Uuid,
        expected: Option<Uuid>,
        at: Option<&str>,
        due: Option<i64>,
    ) -> StoreResult<AssistanceReminderView> {
        self.transact(|store|{
            let item=store.assistance_item(item_id)?;
            if at.is_some()!=due.is_some() || item.revision!=item_revision || (at.is_some() && !item.is_current()) || store.assistance_reminder(item_id)?.schedule_revision!=expected{return Err(invalid())}
            let pref=store.assistance_category_preference(category(&item))?;
            if at.is_some() && pref.muted{return Err(StoreError::ContractViolation("ASSISTANCE_CATEGORY_MUTED".into()))}
            withdraw_item(&store.conn,&item_id.to_string())?;
            store.conn.execute("INSERT INTO assistance_reminders(item_id,schedule_revision,item_revision,remind_at,due_ms,state) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(item_id) DO UPDATE SET schedule_revision=excluded.schedule_revision,item_revision=excluded.item_revision,remind_at=excluded.remind_at,due_ms=excluded.due_ms,state=excluded.state,failure_code=NULL,check_receipt_id=NULL",params![item_id.to_string(),Uuid::now_v7().to_string(),item_revision.to_string(),at,due,if at.is_some(){"scheduled"}else{"off"}]).map_err(backend)?;
            store.assistance_reminder(item_id)
        })
    }
    pub fn reminder_candidate(
        &self,
        after: i64,
        now: i64,
    ) -> StoreResult<Option<ReminderCandidate>> {
        self.conn.execute("UPDATE assistance_reminders SET state='expired_in_inbox' WHERE state='scheduled' AND due_ms<=?1",[after]).map_err(backend)?;
        let row:Option<(String,String,String,i64)>=self.conn.query_row("SELECT r.item_id,r.item_revision,r.schedule_revision,r.due_ms FROM assistance_reminders r JOIN assistance_items i ON i.item_id=r.item_id WHERE r.state='scheduled' AND r.due_ms>?1 AND r.due_ms<=?2 AND r.item_revision=i.revision AND i.handling='open' AND i.validity='current' AND NOT EXISTS(SELECT 1 FROM notification_attempts a WHERE a.schedule_revision=r.schedule_revision) ORDER BY r.due_ms,r.item_id LIMIT 1",params![after,now],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(backend)?;
        let Some((item, item_revision, schedule, due_ms)) = row else {
            return Ok(None);
        };
        let item_id = id(item)?;
        let item = self.assistance_item(item_id)?;
        let category = category(&item);
        let pref = self.assistance_category_preference(category)?;
        Ok(Some(ReminderCandidate {
            item_id,
            item_revision: id(item_revision)?,
            schedule_revision: id(schedule)?,
            due_ms,
            category,
            category_revision: pref.revision,
            attempt_id: Uuid::now_v7(),
            check_id: Uuid::now_v7(),
        }))
    }
    pub fn finish_reminder_check(
        &self,
        c: &ReminderCandidate,
        now: i64,
        eligible: bool,
        failed: bool,
    ) -> StoreResult<Option<Uuid>> {
        if self.conn.is_autocommit() {
            return Err(invalid());
        }
        let item = self.assistance_item(c.item_id)?;
        let pref = self.assistance_category_preference(c.category)?;
        let current:bool=self.conn.query_row("SELECT EXISTS(SELECT 1 FROM assistance_reminders WHERE item_id=?1 AND schedule_revision=?2 AND item_revision=?3 AND state='scheduled')",params![c.item_id.to_string(),c.schedule_revision.to_string(),c.item_revision.to_string()],|r|r.get(0)).map_err(backend)?;
        if !current || item.revision != c.item_revision || !item.is_current() {
            return Ok(None);
        }
        let last: Option<i64> = self
            .conn
            .query_row(
                "SELECT max(claimed_at) FROM notification_attempts",
                [],
                |r| r.get(0),
            )
            .map_err(backend)?;
        let rate_ok = last.is_none_or(|last| now >= last && now - last >= 60_000);
        if !eligible || failed || pref.muted || pref.revision != c.category_revision || !rate_ok {
            self.conn.execute("UPDATE assistance_reminders SET state=?3,failure_code=?4,check_receipt_id=?5 WHERE item_id=?1 AND schedule_revision=?2",params![c.item_id.to_string(),c.schedule_revision.to_string(),if failed{"check_failed"}else{"expired_in_inbox"},if failed{Some("NOTIFICATION_SOURCE_READ_FAILED")}else{None},c.check_id.to_string()]).map_err(backend)?;
            return Ok(None);
        }
        self.conn.execute("INSERT INTO notification_attempts(attempt_id,schedule_revision,item_id,item_revision,category,category_revision,claimed_at,delivery) VALUES(?1,?2,?3,?4,?5,?6,?7,'claimed')",params![c.attempt_id.to_string(),c.schedule_revision.to_string(),c.item_id.to_string(),c.item_revision.to_string(),enum_text(&c.category)?,c.category_revision.map(|v|v.to_string()),now]).map_err(backend)?;
        self.conn.execute("UPDATE assistance_reminders SET check_receipt_id=?3 WHERE item_id=?1 AND schedule_revision=?2",params![c.item_id.to_string(),c.schedule_revision.to_string(),c.check_id.to_string()]).map_err(backend)?;
        Ok(Some(c.attempt_id))
    }
    pub fn reconcile_reminder_check(&mut self, c: &ReminderCandidate) -> StoreResult<()> {
        self.transact(|store|{
            let attempt:bool=store.conn.query_row("SELECT EXISTS(SELECT 1 FROM notification_attempts WHERE attempt_id=?1 AND schedule_revision=?2)",params![c.attempt_id.to_string(),c.schedule_revision.to_string()],|r|r.get(0)).map_err(backend)?;
            if attempt {store.conn.execute("UPDATE notification_attempts SET delivery='unknown' WHERE attempt_id=?1 AND delivery='claimed'",[c.attempt_id.to_string()]).map_err(backend)?;return Ok(())}
            store.conn.execute("UPDATE assistance_reminders SET state='check_failed',failure_code='NOTIFICATION_CHECK_INTERRUPTED',check_receipt_id=?3 WHERE item_id=?1 AND schedule_revision=?2 AND state='scheduled' AND (check_receipt_id IS NULL OR check_receipt_id!=?3)",params![c.item_id.to_string(),c.schedule_revision.to_string(),c.check_id.to_string()]).map_err(backend)?;Ok(())
        })
    }
    pub fn authorize_notification_dispatch(&self, attempt: Uuid) -> StoreResult<bool> {
        self.conn.query_row("SELECT EXISTS(SELECT 1 FROM notification_attempts a JOIN assistance_items i ON i.item_id=a.item_id JOIN assistance_reminders r ON r.item_id=i.item_id LEFT JOIN assistance_category_preferences p ON p.category=a.category WHERE a.attempt_id=?1 AND a.delivery='claimed' AND i.revision=a.item_revision AND i.handling='open' AND i.validity='current' AND r.schedule_revision=a.schedule_revision AND r.state='scheduled' AND COALESCE(p.muted,0)=0 AND p.revision IS a.category_revision AND NOT EXISTS(SELECT 1 FROM notification_withdrawals w WHERE w.attempt_id=a.attempt_id))",[attempt.to_string()],|r|r.get(0)).map_err(backend)
    }
    pub fn record_notification_delivery(
        &self,
        attempt: Uuid,
        outcome: NotificationDelivery,
        now: i64,
    ) -> StoreResult<()> {
        if outcome == NotificationDelivery::Claimed {
            return Err(invalid());
        }
        let current = self.notification_attempt(attempt)?;
        // Late genuine success after an unknown request is retained as history.
        if current.delivery == NotificationDelivery::SubmittedToOs
            && outcome != NotificationDelivery::SubmittedToOs
        {
            return Ok(());
        }
        if current.delivery == NotificationDelivery::Failed
            && outcome == NotificationDelivery::Unknown
        {
            return Ok(());
        }
        self.conn.execute("UPDATE notification_attempts SET delivery=?2,submitted_at=CASE WHEN ?2='submitted_to_os' THEN COALESCE(submitted_at,?3) ELSE submitted_at END WHERE attempt_id=?1",params![attempt.to_string(),enum_text(&outcome)?,now]).map_err(backend)?;
        Ok(())
    }
    pub fn record_notification_event(
        &self,
        attempt: Uuid,
        event: NotificationNativeEvent,
        now: i64,
    ) -> StoreResult<()> {
        self.notification_attempt(attempt)?;
        let field = match event {
            NotificationNativeEvent::Shown => "shown_at",
            NotificationNativeEvent::Opened => "opened_at",
            NotificationNativeEvent::Hidden => "hidden_at",
            NotificationNativeEvent::TimedOut => "timed_out_at",
            NotificationNativeEvent::TrackingExpired => "tracking_expired_at",
        };
        self.conn.execute(&format!("UPDATE notification_attempts SET {field}=COALESCE({field},?2) WHERE attempt_id=?1"),params![attempt.to_string(),now]).map_err(backend)?;
        Ok(())
    }
    pub fn retry_notification_withdrawal(
        &mut self,
        attempt: Uuid,
        expected: Option<Uuid>,
    ) -> StoreResult<NotificationWithdrawal> {
        self.transact(|store| {
            store.notification_attempt(attempt)?;
            let current = store.latest_notification_withdrawal(attempt)?;
            if current.as_ref().map(|w| w.withdrawal_id) != expected
                || current.as_ref().is_some_and(|w| {
                    !matches!(
                        w.state,
                        NotificationWithdrawalState::Failed | NotificationWithdrawalState::Unknown
                    )
                })
            {
                return Err(invalid());
            }
            insert_withdrawal(&store.conn, &attempt.to_string())?;
            store
                .latest_notification_withdrawal(attempt)?
                .ok_or_else(invalid)
        })
    }
    pub fn pending_notification_withdrawal(&self) -> StoreResult<Option<NotificationWithdrawal>> {
        let next:Option<String>=self.conn.query_row("SELECT withdrawal_id FROM notification_withdrawals WHERE state='pending' ORDER BY created_order LIMIT 1",[],|r|r.get(0)).optional().map_err(backend)?;
        next.map(|next| self.withdrawal(&next)).transpose()
    }
    /// The caller retains candidate + token before attempting this commit.
    pub fn claim_notification_withdrawal_exact(
        &mut self,
        candidate: &NotificationWithdrawal,
        token: Uuid,
    ) -> StoreResult<NotificationWithdrawal> {
        let result=self.transact(|store|{
            let changed=store.conn.execute("UPDATE notification_withdrawals SET state='claimed',claim_token=?3 WHERE withdrawal_id=?1 AND attempt_id=?2 AND state='pending' AND claim_token IS NULL",params![candidate.withdrawal_id.to_string(),candidate.attempt_id.to_string(),token.to_string()]).map_err(backend)?;
            if changed!=1{return Err(invalid())}store.withdrawal(&candidate.withdrawal_id.to_string())
        })?;
        // Test the real committed-but-unacknowledged boundary, without resending.
        let _ack_point = format!(
            "soul::notification::withdrawal_claim_after_commit/{}",
            candidate.withdrawal_id
        );
        fail::fail_point!(_ack_point.as_str(), |_| Err(invalid()));
        Ok(result)
    }
    pub fn reconcile_notification_withdrawal_claim(
        &mut self,
        candidate: &NotificationWithdrawal,
        token: Uuid,
    ) -> StoreResult<()> {
        if self.transaction_in_progress() {
            return Err(invalid());
        }
        self.transact(|store|{
            let row:(String,Option<String>,String)=store.conn.query_row("SELECT state,claim_token,attempt_id FROM notification_withdrawals WHERE withdrawal_id=?1",[candidate.withdrawal_id.to_string()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(backend)?;
            if row.2!=candidate.attempt_id.to_string(){return Err(invalid())}
            if row.1.as_deref()==Some(token.to_string().as_str()) {
                if row.0=="claimed" {store.conn.execute("UPDATE notification_withdrawals SET state='unknown' WHERE withdrawal_id=?1 AND claim_token=?2",params![candidate.withdrawal_id.to_string(),token.to_string()]).map_err(backend)?;}
                return Ok(())
            }
            // A provably rolled-back claim is also ended without an OS request.
            // A different owner's claim must never be downgraded.
            if row.0!="pending" || row.1.is_some(){return Err(invalid())}
            store.conn.execute("UPDATE notification_withdrawals SET state='unknown',claim_token=?2 WHERE withdrawal_id=?1 AND state='pending' AND claim_token IS NULL",params![candidate.withdrawal_id.to_string(),token.to_string()]).map_err(backend)?;Ok(())
        })
    }
    pub fn complete_notification_withdrawal(
        &self,
        attempt: Uuid,
        withdrawal: Uuid,
        outcome: NotificationWithdrawalState,
    ) -> StoreResult<()> {
        if !matches!(
            outcome,
            NotificationWithdrawalState::Withdrawn
                | NotificationWithdrawalState::NotPresent
                | NotificationWithdrawalState::Failed
                | NotificationWithdrawalState::Unknown
        ) {
            return Err(invalid());
        }
        let old = self.withdrawal(&withdrawal.to_string())?;
        if old.attempt_id != attempt {
            return Err(invalid());
        }
        if old.state == outcome {
            return Ok(());
        }
        if matches!(
            old.state,
            NotificationWithdrawalState::Withdrawn | NotificationWithdrawalState::NotPresent
        ) {
            return Err(invalid());
        }
        if !matches!(
            old.state,
            NotificationWithdrawalState::Claimed | NotificationWithdrawalState::Unknown
        ) {
            return Err(invalid());
        }
        self.conn.execute("UPDATE notification_withdrawals SET state=?3 WHERE withdrawal_id=?1 AND attempt_id=?2",params![withdrawal.to_string(),attempt.to_string(),enum_text(&outcome)?]).map_err(backend)?;
        Ok(())
    }
}
