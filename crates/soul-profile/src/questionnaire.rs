//! The no-import fallback.
//!
//! PRODUCT_LOCK's v0.1 slice says the user either imports something or answers
//! a questionnaire, and AC-03 is the second case: after the questionnaire the
//! profile must be non-empty and every field in it must be traceable to
//! something the user said. That "traceable" is the reason each answer becomes
//! a [`SoulEvidence`] row of its own before any axis moves — an axis carrying
//! `evidence_ids` that resolve to a questionnaire answer can be explained; an
//! axis that was simply set cannot.
//!
//! The questionnaire is fixed rather than authored at runtime. There are seven
//! questions, they are the same seven on every install, and their ids are
//! derived from the axis and voice keys, so an answer file can be checked
//! against the definition instead of trusted.

use serde::{Deserialize, Serialize};

use soul_schema::profile::AxisPosition;

use crate::axes::{AxisDefinition, DEFAULT_AXES};
use crate::error::{ProfileError, ProfileResult};
use crate::voice::{EmojiUse, VoiceDirectness, VoiceField, VoiceSetting};

/// What a question is asking about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionTarget {
    /// A direction axis. The answer is a position, never a number.
    Axis(AxisDefinition),
    /// A voice field, which the user's answer pins straight away.
    Voice(VoiceField),
}

impl QuestionTarget {
    pub fn label(self) -> &'static str {
        match self {
            QuestionTarget::Axis(_) => "a trait axis",
            QuestionTarget::Voice(_) => "a voice field",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question {
    pub question_id: &'static str,
    pub prompt: &'static str,
    pub target: QuestionTarget,
}

/// The two voice questions the wizard asks. The other voice fields are left to
/// the neutral default until the user changes them in the profile view.
pub const VOICE_QUESTIONS: [(&str, VoiceField, &str); 2] = [
    (
        "q.voice.directness",
        VoiceField::Directness,
        "写消息时，你更常直说，还是先铺垫？",
    ),
    (
        "q.voice.emoji_use",
        VoiceField::EmojiUse,
        "你平时用表情符号多吗？",
    ),
];

pub fn voice_question_id(field: VoiceField) -> &'static str {
    VOICE_QUESTIONS
        .iter()
        .find(|(_, candidate, _)| *candidate == field)
        .map(|(id, _, _)| *id)
        .unwrap_or("q.voice.unasked")
}

/// Every question, in the order the wizard asks them.
pub fn questionnaire() -> Vec<Question> {
    let mut questions: Vec<Question> = DEFAULT_AXES
        .iter()
        .map(|axis| Question {
            question_id: axis.question_id,
            prompt: axis.question,
            target: QuestionTarget::Axis(*axis),
        })
        .collect();
    questions.extend(VOICE_QUESTIONS.iter().map(|(id, field, prompt)| Question {
        question_id: id,
        prompt,
        target: QuestionTarget::Voice(*field),
    }));
    questions
}

pub fn question(question_id: &str) -> Option<Question> {
    questionnaire()
        .into_iter()
        .find(|q| q.question_id == question_id)
}

/// One answer, as it arrives from the wizard or from a fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Answer {
    /// Which way the user says they lean. The four positions in
    /// `profile.schema.json` are the only choices offered.
    Axis {
        question_id: String,
        position: AxisPosition,
    },
    Voice {
        question_id: String,
        setting: VoiceSetting,
    },
}

impl Answer {
    pub fn question_id(&self) -> &str {
        match self {
            Answer::Axis { question_id, .. } | Answer::Voice { question_id, .. } => question_id,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Answer::Axis { .. } => "a trait axis",
            Answer::Voice { .. } => "a voice field",
        }
    }
}

/// A completed questionnaire. Matches `fixtures/profile/*.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionnaireResponse {
    /// RFC 3339, UTC. Recorded on the evidence rows so an answer can be dated.
    pub answered_at: String,
    pub answers: Vec<Answer>,
}

impl QuestionnaireResponse {
    pub fn new(answered_at: impl Into<String>, answers: Vec<Answer>) -> Self {
        QuestionnaireResponse {
            answered_at: answered_at.into(),
            answers,
        }
    }
}

/// One answer, checked against the question it claims to answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckedAnswer {
    Axis {
        question_id: String,
        axis: AxisDefinition,
        position: AxisPosition,
    },
    Voice {
        question_id: String,
        setting: VoiceSetting,
    },
}

impl CheckedAnswer {
    pub fn question_id(&self) -> &str {
        match self {
            CheckedAnswer::Axis { question_id, .. } | CheckedAnswer::Voice { question_id, .. } => {
                question_id
            }
        }
    }
}

/// Resolve every answer against the fixed questionnaire.
///
/// An unknown question id, an answer whose kind does not match its question,
/// or the same question answered twice are all errors: the profile that comes
/// out of this is supposed to be explainable, and none of those three can be.
pub fn check(response: &QuestionnaireResponse) -> ProfileResult<Vec<CheckedAnswer>> {
    if response.answers.is_empty() {
        return Err(ProfileError::EmptyQuestionnaire);
    }

    let mut seen: Vec<&str> = Vec::new();
    let mut checked = Vec::with_capacity(response.answers.len());

    for answer in &response.answers {
        let question_id = answer.question_id();
        let Some(question) = question(question_id) else {
            return Err(ProfileError::UnknownQuestion(question_id.to_owned()));
        };
        if seen.contains(&question_id) {
            return Err(ProfileError::DuplicateAnswer(question_id.to_owned()));
        }
        seen.push(question.question_id);

        checked.push(match (question.target, answer) {
            (QuestionTarget::Axis(axis), Answer::Axis { position, .. }) => CheckedAnswer::Axis {
                question_id: question_id.to_owned(),
                axis,
                position: *position,
            },
            (QuestionTarget::Voice(_), Answer::Voice { setting, .. }) => CheckedAnswer::Voice {
                question_id: question_id.to_owned(),
                setting: *setting,
            },
            (target, answer) => {
                return Err(ProfileError::AnswerTargetMismatch {
                    question_id: question_id.to_owned(),
                    expected: target.label(),
                    found: answer.label(),
                })
            }
        });
    }

    Ok(checked)
}

/// A questionnaire with every axis question answered the same way, for tests
/// and for the wizard's "all five at once" path.
pub fn every_axis(position: AxisPosition, answered_at: &str) -> QuestionnaireResponse {
    let mut answers: Vec<Answer> = DEFAULT_AXES
        .iter()
        .map(|axis| Answer::Axis {
            question_id: axis.question_id.to_owned(),
            position,
        })
        .collect();
    answers.push(Answer::Voice {
        question_id: voice_question_id(VoiceField::Directness).to_owned(),
        setting: VoiceSetting::Directness(VoiceDirectness::Balanced),
    });
    answers.push(Answer::Voice {
        question_id: voice_question_id(VoiceField::EmojiUse).to_owned(),
        setting: VoiceSetting::EmojiUse(EmojiUse::Sparing),
    });
    QuestionnaireResponse::new(answered_at, answers)
}
