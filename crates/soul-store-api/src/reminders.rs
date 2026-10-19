//! Local reminder metadata. Notification transport receives only attempt IDs.
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReminderState {
    Off,
    Scheduled,
    Invalidated,
    ExpiredInInbox,
    Suppressed,
    CheckFailed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDelivery {
    Claimed,
    SubmittedToOs,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationWithdrawalState {
    Pending,
    Claimed,
    Withdrawn,
    NotPresent,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationNativeEvent {
    Shown,
    Opened,
    Hidden,
    TimedOut,
    TrackingExpired,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationWithdrawal {
    pub withdrawal_id: Uuid,
    pub attempt_id: Uuid,
    pub state: NotificationWithdrawalState,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistanceNotificationAttempt {
    pub attempt_id: Uuid,
    pub item_id: Uuid,
    pub item_revision: Uuid,
    pub schedule_revision: Uuid,
    pub delivery: NotificationDelivery,
    pub shown_at: Option<i64>,
    pub opened_at: Option<i64>,
    pub hidden_at: Option<i64>,
    pub timed_out_at: Option<i64>,
    pub tracking_expired_at: Option<i64>,
    pub latest_withdrawal: Option<NotificationWithdrawal>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistanceReminderView {
    pub item_id: Uuid,
    pub item_revision: Uuid,
    pub schedule_revision: Option<Uuid>,
    pub state: ReminderState,
    pub remind_at: Option<String>,
    pub latest_attempt: Option<AssistanceNotificationAttempt>,
    pub failure_code: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceCategory {
    ManualGoal,
    Observation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistanceCategoryPreference {
    pub category: AssistanceCategory,
    pub revision: Option<Uuid>,
    pub muted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistanceCategoryPreferences {
    pub preferences: Vec<AssistanceCategoryPreference>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistanceFeedbackValue {
    None,
    Helpful,
    NotRelevant,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistanceFeedbackView {
    pub item_id: Uuid,
    pub item_revision: Uuid,
    pub feedback_revision: Option<Uuid>,
    pub value: AssistanceFeedbackValue,
}
#[derive(Debug, Clone)]
pub struct ReminderCandidate {
    pub item_id: Uuid,
    pub item_revision: Uuid,
    pub schedule_revision: Uuid,
    pub due_ms: i64,
    pub category: AssistanceCategory,
    pub category_revision: Option<Uuid>,
    pub attempt_id: Uuid,
    pub check_id: Uuid,
}
