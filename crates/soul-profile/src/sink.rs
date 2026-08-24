//! The profile's half of WP06's seam.
//!
//! `soul-import` records a questionnaire answer — an event with the words
//! sealed, and a `user_stated` evidence row — and then hands the ids across
//! [`UserStatedSink`]. This is the implementation of that trait, and it is the
//! only place that decides what an answer *means*: `leans_high` on
//! `q.axis.curiosity` is a position on an axis, `formal` on `q.voice.register`
//! is an instruction the drafting layer obeys, and the answer to a boundary
//! question is prose that stays where it was sealed.
//!
//! The sink only interprets. Writing is [`crate::service::intake`]'s job,
//! which is not squeamishness about layering: the recorder holds the store
//! mutably for the length of the run, so a sink that also wrote would be a
//! second writer inside someone else's borrow. Staging the answers and
//! applying them afterwards also means a refusal — an option nobody offered, a
//! question with no profile field — stops the run before the profile is
//! touched.

use uuid::Uuid;

use soul_import::questionnaire::{RecordedAnswer, SinkError, UserStatedSink};
use soul_schema::profile::AxisPosition;

use crate::axes;
use crate::questionnaire::{target_of, QuestionTarget};
use crate::voice::VoiceSetting;

/// What one recorded answer turned out to mean.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagedValue {
    Position(AxisPosition),
    Voice(VoiceSetting),
    /// Prose, still sealed in the event. The profile records that the user
    /// said something here and points at where it is.
    Stated,
}

/// One answer the recorder wrote, resolved to the field it moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StagedAnswer {
    /// Borrowed from the canonical question list, so it is always an id the
    /// profile knows.
    pub question_id: &'static str,
    pub event_id: Uuid,
    pub evidence_id: Uuid,
    pub target: QuestionTarget,
    pub value: StagedValue,
}

/// The sink `soul-import` drives while it records.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProfileSink {
    accepted: Vec<StagedAnswer>,
}

impl ProfileSink {
    pub fn new() -> ProfileSink {
        ProfileSink::default()
    }

    /// The answers, in the order they were recorded.
    pub fn accepted(&self) -> &[StagedAnswer] {
        &self.accepted
    }

    pub fn is_empty(&self) -> bool {
        self.accepted.is_empty()
    }
}

impl UserStatedSink for ProfileSink {
    fn accept(&mut self, answer: &RecordedAnswer) -> Result<(), SinkError> {
        let refuse = |reason: &str| SinkError {
            question_key: answer.question.key.to_owned(),
            reason: reason.to_owned(),
        };

        let target = target_of(answer.question.key)
            .ok_or_else(|| refuse("this build of the profile has no field for that question"))?;

        let value = match (target, answer.choice) {
            (QuestionTarget::Axis(_), Some(option)) => StagedValue::Position(
                axes::position_by_key(option)
                    .ok_or_else(|| refuse("that option is not a direction an axis can be in"))?,
            ),
            (QuestionTarget::Voice(field), Some(option)) => StagedValue::Voice(
                VoiceSetting::from_option(field, option)
                    .ok_or_else(|| refuse("that option is not a value this voice field takes"))?,
            ),
            (QuestionTarget::Stated(_), None) => StagedValue::Stated,
            // A choice answer to a prose field, or prose to a field that takes
            // one of a fixed set. Either way the wizard and the profile
            // disagree about what was asked, and guessing would put something
            // in the profile the user did not say.
            _ => return Err(refuse("the answer is not the shape this field takes")),
        };

        self.accepted.push(StagedAnswer {
            question_id: answer.question.key,
            event_id: answer.event_id,
            evidence_id: answer.evidence_id,
            target,
            value,
        });
        Ok(())
    }
}
