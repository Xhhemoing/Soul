//! The no-import fallback, and the one question set the product asks.
//!
//! PRODUCT_LOCK's v0.1 slice says the user either imports something or answers
//! a questionnaire, and AC-03 is the second case: after the questionnaire the
//! profile must be non-empty and every field in it must be traceable to
//! something the user said. That "traceable" is the reason each answer becomes
//! a [`soul_schema::evidence::SoulEvidence`] row of its own before any axis
//! moves — an axis carrying `evidence_ids` that resolve to a questionnaire
//! answer can be explained; an axis that was simply set cannot.
//!
//! The questions themselves are not declared here. They live in
//! [`soul_import::questionnaire::QUESTIONS`], which is the list the recorder
//! validates against, and this module says what each one *means*: which axis
//! it moves, which voice field it pins, or which stated field it lands in.
//! Before WP13 there were two lists, and a user who imported nothing was asked
//! about their own voice twice — once by each crate, into two sets of evidence
//! that never met. [`target_of`] is now the only mapping, and
//! `fixtures/questionnaire/v0_1.json` pins both halves.

use serde::{Deserialize, Serialize};

use soul_import::questionnaire::{self as recorder, AnswerShape};
use soul_schema::profile::AxisPosition;

use crate::axes::{self, AxisDefinition, DEFAULT_AXES};
use crate::error::{ProfileError, ProfileResult};
use crate::voice::{VoiceField, VoiceSetting};

/// A profile field that holds what the user stated rather than a closed
/// value: `profile.values` and `profile.boundaries`.
///
/// What the user wrote stays sealed in the event the recorder wrote. The
/// profile keeps a pointer to it, never the words — `docs/SECURITY.md`
/// reserves prose for `sealedText`, and the profiles table is not that.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatedField {
    Boundary,
    Value,
}

impl StatedField {
    pub const fn as_str(self) -> &'static str {
        match self {
            StatedField::Boundary => "boundary",
            StatedField::Value => "value",
        }
    }
}

/// What a question is asking about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionTarget {
    /// A direction axis. The answer is a position, never a number.
    Axis(AxisDefinition),
    /// A voice field, which the user's answer pins straight away.
    Voice(VoiceField),
    /// A boundary or a value, in the user's own words.
    Stated(StatedField),
}

impl QuestionTarget {
    pub fn label(self) -> &'static str {
        match self {
            QuestionTarget::Axis(_) => "a trait axis",
            QuestionTarget::Voice(_) => "a voice field",
            QuestionTarget::Stated(StatedField::Boundary) => "a boundary",
            QuestionTarget::Stated(StatedField::Value) => "something the user values",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question {
    pub question_id: &'static str,
    pub prompt: &'static str,
    pub target: QuestionTarget,
    pub shape: AnswerShape,
}

impl Question {
    /// The options the wizard offers, or an empty slice for a prose question.
    pub fn options(&self) -> &'static [&'static str] {
        match self.shape {
            AnswerShape::Choice(options) => options,
            AnswerShape::Prose => &[],
        }
    }
}

/// Which profile field a canonical question moves.
///
/// `None` means the recorder asks something this build of the profile has no
/// home for, which is the divergence this mapping exists to make impossible:
/// `tests/one_questionnaire.rs` asserts every question in the canonical list
/// resolves here.
pub fn target_of(question_id: &str) -> Option<QuestionTarget> {
    if let Some(axis) = DEFAULT_AXES
        .iter()
        .find(|axis| axis.question_id == question_id)
    {
        return Some(QuestionTarget::Axis(*axis));
    }
    let target = match question_id {
        "q.voice.register" => QuestionTarget::Voice(VoiceField::Register),
        "q.voice.directness" => QuestionTarget::Voice(VoiceField::Directness),
        "q.voice.emoji_use" => QuestionTarget::Voice(VoiceField::EmojiUse),
        "q.boundary.topics" | "q.boundary.availability" => {
            QuestionTarget::Stated(StatedField::Boundary)
        }
        "q.value.what_matters" => QuestionTarget::Stated(StatedField::Value),
        _ => return None,
    };
    Some(target)
}

/// Every question, in the order the wizard asks them, each paired with what it
/// moves. Built from the recorder's list rather than restating it.
pub fn questionnaire() -> Vec<Question> {
    recorder::QUESTIONS
        .iter()
        .filter_map(|question| {
            target_of(question.key).map(|target| Question {
                question_id: question.key,
                prompt: question.prompt,
                target,
                shape: question.shape,
            })
        })
        .collect()
}

pub fn question(question_id: &str) -> Option<Question> {
    questionnaire()
        .into_iter()
        .find(|question| question.question_id == question_id)
}

/// The question that asks about one voice field, if the wizard asks about it.
pub fn voice_question_id(field: VoiceField) -> Option<&'static str> {
    questionnaire()
        .into_iter()
        .find(|question| question.target == QuestionTarget::Voice(field))
        .map(|question| question.question_id)
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
    /// The user's own words. Sealed by the recorder; the profile keeps only a
    /// pointer.
    Prose { question_id: String, text: String },
}

impl Answer {
    pub fn question_id(&self) -> &str {
        match self {
            Answer::Axis { question_id, .. }
            | Answer::Voice { question_id, .. }
            | Answer::Prose { question_id, .. } => question_id,
        }
    }

    /// Build the answer a canonical question takes, from what a UI collected:
    /// an option key for a choice question, prose for a text box.
    ///
    /// This is how a wizard turns "the user tapped the third option" into
    /// something [`crate::service::intake`] accepts, without the UI having to
    /// know that `leans_high` is an axis position and `formal` is a register.
    pub fn for_question(question_id: &str, given: &str) -> ProfileResult<Answer> {
        let question = question(question_id)
            .ok_or_else(|| ProfileError::UnknownQuestion(question_id.into()))?;
        let unusable = || ProfileError::UnofferedOption {
            question_id: question_id.to_owned(),
            offered: question.options().len(),
        };

        let answer = match question.target {
            QuestionTarget::Axis(_) => Answer::Axis {
                question_id: question_id.to_owned(),
                position: axes::position_by_key(given).ok_or_else(unusable)?,
            },
            QuestionTarget::Voice(field) => Answer::Voice {
                question_id: question_id.to_owned(),
                setting: VoiceSetting::from_option(field, given).ok_or_else(unusable)?,
            },
            QuestionTarget::Stated(_) => Answer::Prose {
                question_id: question_id.to_owned(),
                text: given.to_owned(),
            },
        };
        Ok(answer)
    }

    fn label(&self) -> &'static str {
        match self {
            Answer::Axis { .. } => "a trait axis",
            Answer::Voice { .. } => "a voice field",
            Answer::Prose { .. } => "prose",
        }
    }
}

/// A completed questionnaire. Matches `fixtures/questionnaire/*.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionnaireResponse {
    /// RFC 3339, UTC. Recorded on the events an answer produces, so an answer
    /// can be dated.
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
    Prose {
        question_id: String,
        field: StatedField,
        text: String,
    },
}

impl CheckedAnswer {
    pub fn question_id(&self) -> &str {
        match self {
            CheckedAnswer::Axis { question_id, .. }
            | CheckedAnswer::Voice { question_id, .. }
            | CheckedAnswer::Prose { question_id, .. } => question_id,
        }
    }

    /// The answer as the recorder takes it: a question key and either an
    /// option token or the user's words.
    pub(crate) fn to_recorded(&self) -> recorder::Answer {
        match self {
            CheckedAnswer::Axis {
                question_id,
                position,
                ..
            } => recorder::Answer::new(question_id, axes::position_key(*position)),
            CheckedAnswer::Voice {
                question_id,
                setting,
            } => recorder::Answer::new(question_id, setting.option_key()),
            CheckedAnswer::Prose {
                question_id, text, ..
            } => recorder::Answer::new(question_id, text),
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
            (QuestionTarget::Stated(field), Answer::Prose { text, .. }) => CheckedAnswer::Prose {
                question_id: question_id.to_owned(),
                field,
                text: text.clone(),
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
    let answers: Vec<Answer> = DEFAULT_AXES
        .iter()
        .map(|axis| Answer::Axis {
            question_id: axis.question_id.to_owned(),
            position,
        })
        .collect();
    QuestionnaireResponse::new(answered_at, answers)
}
