use super::super::{
    presentation::{SimulatedFeedback, SimulatedSideFeedback},
    records::*,
};
use serde::{Deserialize, Deserializer};
use std::time::Duration;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Action {
    State {},
    Start {},
    Stop {},
    Withdraw {},
    Report {},
    Reveal { card: usize },
    Next { card: usize },
    Submit { card: usize, feedback: Feedback },
}

fn required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Feedback {
    preference: Preference,
    blind_status: BlindStatus,
    sides: [Side; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Side {
    completed: bool,
    #[serde(deserialize_with = "required_option")]
    first_pass: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    edit_seconds: Option<u64>,
    #[serde(deserialize_with = "required_option")]
    pause_seconds: Option<u64>,
    help_count: u32,
    #[serde(deserialize_with = "required_option")]
    unsupported_claim: Option<bool>,
    #[serde(deserialize_with = "required_option")]
    fact_errors: Option<u32>,
    safety_stop: bool,
    t1_slot_filled: bool,
}

impl Feedback {
    pub(super) fn convert(
        self,
        task: TaskType,
        presented: Duration,
        deadline: Duration,
    ) -> Result<SimulatedFeedback, &'static str> {
        let [left, right] = self.sides;
        Ok(SimulatedFeedback {
            preference: self.preference,
            blind_status: self.blind_status,
            sides: [
                left.convert(task, presented, deadline)?,
                right.convert(task, presented, deadline)?,
            ],
        })
    }
}

impl Side {
    fn convert(
        self,
        task: TaskType,
        presented: Duration,
        deadline: Duration,
    ) -> Result<SimulatedSideFeedback, &'static str> {
        // Independently rounded intervals can sum above their true total.
        // Check the sum, including overflow, before accepting browser numbers.
        let bound = presented
            .as_secs()
            .saturating_add(2)
            .min(deadline.as_secs());
        let total = self
            .edit_seconds
            .unwrap_or(0)
            .checked_add(self.pause_seconds.unwrap_or(0))
            .ok_or("feedback")?;
        if total > bound
            || (self.safety_stop && self.completed)
            || (task == TaskType::T1
                && (self.unsupported_claim.is_some() || self.fact_errors.is_some()))
            || (task == TaskType::T2 && self.fact_errors.is_some())
            || (task != TaskType::T1 && self.t1_slot_filled)
        {
            return Err("feedback");
        }
        let reason = if self.completed {
            if self.edit_seconds.is_none() || self.pause_seconds.is_none() {
                Some(MissingReason::NotMeasured)
            } else {
                None
            }
        } else {
            Some(MissingReason::NotCompleted)
        };
        let timing = |value: Option<u64>| {
            if self.completed {
                value.map_or(
                    Measured::Missing(MissingReason::NotMeasured),
                    Measured::Value,
                )
            } else {
                Measured::Missing(MissingReason::NotCompleted)
            }
        };
        Ok(SimulatedSideFeedback {
            completed: self.completed,
            first_pass: self
                .first_pass
                .map_or(Measured::Missing(MissingReason::NotRated), Measured::Value),
            edit_seconds: timing(self.edit_seconds),
            pause_seconds: timing(self.pause_seconds),
            help_count: Measured::Value(self.help_count),
            unsupported_claim: if task == TaskType::T1 {
                Measured::NotApplicable
            } else {
                self.unsupported_claim
                    .map_or(Measured::Missing(MissingReason::NotRated), Measured::Value)
            },
            fact_errors: if task == TaskType::T3 {
                self.fact_errors
                    .map_or(Measured::Missing(MissingReason::NotRated), Measured::Value)
            } else {
                Measured::NotApplicable
            },
            safety_stop: self.safety_stop,
            t1_slot_filled: self.t1_slot_filled,
            missing_reason: reason,
        })
    }
}
