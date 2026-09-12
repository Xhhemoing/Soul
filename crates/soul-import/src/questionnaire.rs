//! The fallback for when there is no file to import, and the one question set
//! v0.1 asks.
//!
//! AC-03: with nothing imported, finishing the questionnaire has to leave a
//! profile that is not empty and whose fields say they came from the user.
//! This module owns the recording half of that — turning answers into events
//! and `user_stated` evidence — and stops there. What a profile is, and which
//! field an answer moves, belongs to WP03; the seam between the two is
//! [`UserStatedSink`], which hands over ids and a chosen option, never prose.
//!
//! [`QUESTIONS`] is the canonical list, and it is canonical in the strong
//! sense: `soul-profile` builds its own questionnaire from this table rather
//! than keeping a second one, and `fixtures/questionnaire/v0_1.json` pins the
//! ids so neither side can move one without the other's tests going red. There
//! used to be two lists — eight questions here, seven over there — which meant
//! a user who took both paths was asked about their own voice twice.
//!
//! An option key such as `leans_high` or `formal` is an opaque token to this
//! crate. It is declared here because the recorder has to be able to refuse an
//! answer that is not one of the offered options; what the token *means* is
//! decided by whatever implements [`UserStatedSink`].
//!
//! The prose answers are the user's own words about themselves, so they are
//! sealed like any other prose but carry `subject: self`: they are not
//! third-party data and the redactor has no reason to placehold them.

use uuid::Uuid;

use soul_policy::audit::{AuditContent, ReasonCode};
use soul_schema::audit::{AuditAction, AuditCounts};
use soul_schema::common::{
    ActorSubject, Derivation, EgressPolicy, Privacy, Purpose, Retention, SchemaVersion,
    SealedSubject, Subject, SupportedBand, Timestamp,
};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_store_api::types::SealRequest;
use soul_store_api::{BlobStore, EventStore, ProfileStore};

use crate::commit::ImportError;
use crate::model::StagedImport;

/// Tag on the `source_refs` of a questionnaire answer, so WP03 can find them
/// without matching on field names.
pub const QUESTIONNAIRE_REF_KIND: &str = "questionnaire_answer";

/// What a questionnaire answer is worth as evidence.
///
/// Moderate, not strong: the user is describing themselves from memory, which
/// is better than a guess and weaker than a correction made while looking at
/// what the profile actually says. WP03 grades a correction `strong` for
/// exactly that reason, and the two have to agree now that one questionnaire
/// feeds both.
pub const QUESTIONNAIRE_STRENGTH: SupportedBand = SupportedBand::Moderate;

/// How the wizard collects an answer, and therefore what the recorder can
/// check an answer against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerShape {
    /// A fixed set of options; the answer is one of these keys and nothing
    /// else. The keys are tokens here, not meanings.
    Choice(&'static [&'static str]),
    /// A text box. The answer is prose and is sealed.
    Prose,
}

/// The three directions a trait-axis question offers. `unknown` is not among
/// them: not answering is how a user says they do not know, and it leaves no
/// row rather than writing one that says nothing.
const LEANING: &[&str] = &["leans_low", "mixed", "leans_high"];
const REGISTER: &[&str] = &["casual", "plain", "formal"];
const DIRECTNESS: &[&str] = &["reserved", "balanced", "direct"];
const EMOJI_USE: &[&str] = &["never", "sparing", "frequent"];

/// One thing the wizard asks.
///
/// Deliberately about voice, boundaries and preferences — the things the
/// profile is made of — and deliberately not about mood or wellbeing. Soul is
/// not a medical product and does not ask medical questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Question {
    /// Stable key. WP03 maps it to a profile field; it never changes once
    /// shipped, because stored evidence points at it.
    pub key: &'static str,
    pub prompt: &'static str,
    pub shape: AnswerShape,
}

impl Question {
    /// The options this question offers, or an empty slice for a prose
    /// question.
    pub fn options(&self) -> &'static [&'static str] {
        match self.shape {
            AnswerShape::Choice(options) => options,
            AnswerShape::Prose => &[],
        }
    }

    pub fn is_prose(&self) -> bool {
        matches!(self.shape, AnswerShape::Prose)
    }

    /// The option key equal to `given`, borrowed from this question's own
    /// list, so a recorded answer can only ever carry an option that exists.
    fn option(&self, given: &str) -> Option<&'static str> {
        self.options().iter().copied().find(|key| *key == given)
    }
}

/// The v0.1 questionnaire. Eleven questions, asked once.
///
/// Eight of them are one tap and land on a profile field directly; three are
/// text boxes and may be left blank. See `fixtures/questionnaire/v0_1.json`
/// for the same list with what each one moves, and for the three WP06
/// questions this merge dropped.
pub const QUESTIONS: &[Question] = &[
    Question {
        key: "q.axis.curiosity",
        prompt: "遇到没做过的事，你更想试试，还是先按熟悉的来？",
        shape: AnswerShape::Choice(LEANING),
    },
    Question {
        key: "q.axis.orderliness",
        prompt: "开始一件事之前，你更常先列计划，还是先动手？",
        shape: AnswerShape::Choice(LEANING),
    },
    Question {
        key: "q.axis.social_energy",
        prompt: "一天下来，和人待着让你更有劲，还是独处更有劲？",
        shape: AnswerShape::Choice(LEANING),
    },
    Question {
        key: "q.axis.accommodation",
        prompt: "有分歧时，你更常直说，还是先照顾对方的感受？",
        shape: AnswerShape::Choice(LEANING),
    },
    Question {
        key: "q.axis.emotional_steadiness",
        prompt: "最近这段时间，你的情绪起伏算平缓还是明显？",
        shape: AnswerShape::Choice(LEANING),
    },
    Question {
        key: "q.voice.register",
        prompt: "给不太熟的人写消息时，你的语气偏正式还是偏随意？",
        shape: AnswerShape::Choice(REGISTER),
    },
    Question {
        key: "q.voice.directness",
        prompt: "写消息时，你更常直说，还是先铺垫？",
        shape: AnswerShape::Choice(DIRECTNESS),
    },
    Question {
        key: "q.voice.emoji_use",
        prompt: "你平时用表情符号多吗？",
        shape: AnswerShape::Choice(EMOJI_USE),
    },
    Question {
        key: "q.boundary.topics",
        prompt: "有哪些话题，你不希望 Soul 替你起草或分析？",
        shape: AnswerShape::Prose,
    },
    Question {
        key: "q.boundary.availability",
        prompt: "什么时间段你基本不回消息？",
        shape: AnswerShape::Prose,
    },
    Question {
        key: "q.value.what_matters",
        prompt: "有没有一件事，是你希望 Soul 无论如何都替你守住的？",
        shape: AnswerShape::Prose,
    },
];

pub fn question(key: &str) -> Option<&'static Question> {
    QUESTIONS.iter().find(|question| question.key == key)
}

/// What the user gave, against one question.
///
/// `text` is the option key for a choice question and the user's own words for
/// a prose one. Blank means the question was skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub question_key: String,
    pub text: String,
}

impl Answer {
    pub fn new(question_key: impl Into<String>, text: impl Into<String>) -> Answer {
        Answer {
            question_key: question_key.into(),
            text: text.into(),
        }
    }
}

/// One stored answer, as ids. This is what crosses the seam to WP03.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedAnswer {
    pub event_id: Uuid,
    pub evidence_id: Uuid,
    /// Always [`EvidenceMethod::UserStated`]. Present as a field so the seam
    /// carries the provenance rather than relying on the reader to assume it.
    pub method: EvidenceMethod,
    /// Which question. Borrowed from [`QUESTIONS`], so it is always a key WP03
    /// knows.
    pub question: &'static Question,
    /// Which option, for a choice question, borrowed from the question's own
    /// list. `None` for a prose answer: an option key is a token from a closed
    /// set and may cross the seam, and what the user typed may not.
    pub choice: Option<&'static str>,
}

/// The seam with WP03.
///
/// `soul-import` does not know what a profile is and must not learn: it
/// produces evidence and hands over ids. Whatever builds the profile
/// implements this, decides which field the answer belongs to, and writes it
/// with `user_stated` provenance so a later inference cannot overwrite it.
pub trait UserStatedSink {
    fn accept(&mut self, answer: &RecordedAnswer) -> Result<(), SinkError>;
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the profile refused a user-stated answer for `{question_key}`: {reason}")]
pub struct SinkError {
    pub question_key: String,
    pub reason: String,
}

/// A sink that keeps what it is given, for tests that need to prove the seam
/// is actually driven without pulling a profile in.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CollectingSink {
    pub accepted: Vec<RecordedAnswer>,
}

impl UserStatedSink for CollectingSink {
    fn accept(&mut self, answer: &RecordedAnswer) -> Result<(), SinkError> {
        self.accepted.push(*answer);
        Ok(())
    }
}

/// Is the questionnaire the path to take?
///
/// True when the user brought no file, and also when the file they brought
/// held no messages: an export with nothing in it leaves exactly as little to
/// build a profile from as no export at all.
pub fn fallback_needed(staged: Option<&StagedImport>) -> bool {
    staged.is_none_or(|import| import.messages.is_empty())
}

/// What one questionnaire run wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestionnaireReceipt {
    pub answers: Vec<RecordedAnswer>,
    /// One key for the whole run, so "forget what I told the wizard" is a
    /// single [`soul_store_api::forget::ForgetUnit::ContentKey`].
    pub content_key_id: Option<Uuid>,
    pub audit: Vec<AuditContent>,
}

#[derive(Debug, thiserror::Error)]
pub enum QuestionnaireError {
    #[error(transparent)]
    Import(#[from] ImportError),

    #[error(transparent)]
    Sink(#[from] SinkError),

    /// An answer names a question this build does not ask. Stored evidence
    /// points at question keys, so an unknown one would be a dangling
    /// reference the moment WP03 tried to read it.
    #[error("`{key}` is not one of the questions this build asks")]
    UnknownQuestion { key: String },

    /// An answer to a choice question that is not one of the options. The
    /// offending value is not repeated: a wizard that sent prose to a choice
    /// question sent the user's prose, and a refusal does not echo it.
    #[error(
        "`{key}` is answered by choosing one of {offered} options, and this is not one of them"
    )]
    UnknownOption { key: String, offered: usize },
}

/// Record the answers, then push them across the seam.
///
/// `answered_at` is passed in rather than read from a clock so a test and a
/// replay produce the same rows.
pub fn record<S>(
    store: &mut S,
    answers: &[Answer],
    answered_at: &Timestamp,
    sink: &mut dyn UserStatedSink,
) -> Result<QuestionnaireReceipt, QuestionnaireError>
where
    S: EventStore + ProfileStore + BlobStore,
{
    let mut receipt = QuestionnaireReceipt::default();
    if answers.is_empty() {
        return Ok(receipt);
    }
    let content_key_id = Uuid::now_v7();
    receipt.content_key_id = Some(content_key_id);

    for answer in answers {
        let question =
            question(&answer.question_key).ok_or(QuestionnaireError::UnknownQuestion {
                key: answer.question_key.clone(),
            })?;
        let given = answer.text.trim();
        if given.is_empty() {
            continue;
        }
        let choice = match question.shape {
            AnswerShape::Prose => None,
            AnswerShape::Choice(options) => Some(question.option(given).ok_or(
                QuestionnaireError::UnknownOption {
                    key: question.key.to_owned(),
                    offered: options.len(),
                },
            )?),
        };

        let event_id = Uuid::now_v7();
        let body_ref = store
            .seal(SealRequest::new(
                content_key_id,
                event_id,
                "body_ref",
                SealedSubject::Owner,
                answer.text.as_bytes().to_vec(),
            ))
            .map_err(ImportError::from)?;

        store
            .append_event(SoulEvent {
                schema_version: SchemaVersion,
                event_id,
                ts: answered_at.clone(),
                source: EventSource::UiQuestionnaire,
                kind: EventKind::QuestionnaireAnswer,
                actor_subject: ActorSubject::Owner,
                consent_id: None,
                privacy: answer_privacy(),
                body_ref: Some(body_ref),
            })
            .map_err(ImportError::from)?;

        let evidence_id = Uuid::now_v7();
        store
            .put_evidence(SoulEvidence {
                schema_version: SchemaVersion,
                evidence_id,
                kind: EvidenceKind::Questionnaire,
                subject: Subject::Owner,
                source_refs: vec![source_ref(event_id, question, choice)],
                strength: QUESTIONNAIRE_STRENGTH,
                method: Some(EvidenceMethod::UserStated),
                exportable_to_research: Some(false),
                privacy: Some(answer_privacy()),
            })
            .map_err(ImportError::from)?;

        let recorded = RecordedAnswer {
            event_id,
            evidence_id,
            method: EvidenceMethod::UserStated,
            question,
            choice,
        };
        sink.accept(&recorded)?;
        receipt.answers.push(recorded);
    }

    if !receipt.answers.is_empty() {
        receipt.audit.push(
            AuditContent::allowed(AuditAction::ImportCommit, ReasonCode::Routine).counting(
                AuditCounts {
                    items: Some(receipt.answers.len() as u64),
                    bytes: None,
                },
            ),
        );
    }
    Ok(receipt)
}

/// Where an answer came from: the event, the question, and — when the question
/// was a choice — which of its options. All three are ids or closed tokens, so
/// the evidence row explains itself without opening the sealed body.
fn source_ref(
    event_id: Uuid,
    question: &Question,
    choice: Option<&'static str>,
) -> serde_json::Value {
    let mut reference = serde_json::json!({
        "ref_kind": QUESTIONNAIRE_REF_KIND,
        "event_id": event_id.to_string(),
        "question_key": question.key,
    });
    if let (Some(option), Some(object)) = (choice, reference.as_object_mut()) {
        object.insert("option_key".to_owned(), serde_json::json!(option));
    }
    reference
}

fn answer_privacy() -> Privacy {
    Privacy {
        subject: Subject::Owner,
        derivation: Derivation::Raw,
        purposes: vec![Purpose::SoulProfile],
        retention: Retention::until_forgotten(),
        egress: EgressPolicy::default(),
    }
}
