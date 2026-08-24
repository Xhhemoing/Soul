//! The fallback for when there is no file to import.
//!
//! AC-03: with nothing imported, finishing the questionnaire has to leave a
//! profile that is not empty and whose fields say they came from the user.
//! This module owns the import-side half of that — turning answers into
//! events and `user_stated` evidence — and stops there. What a profile is, and
//! which axis an answer moves, belongs to WP03; the seam between the two is
//! [`UserStatedSink`], which hands over ids and nothing else.
//!
//! The answers are the user's own words about themselves, so they are sealed
//! like any other prose but carry `subject: self`: they are not third-party
//! data and the redactor has no reason to placehold them.

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
}

/// The v0.1 questionnaire.
pub const QUESTIONS: &[Question] = &[
    Question {
        key: "voice.directness",
        prompt: "你平时说话是直接了当，还是喜欢先铺垫？举个你最近的例子。",
    },
    Question {
        key: "voice.register",
        prompt: "给不太熟的人写消息时，你的语气偏正式还是偏随意？",
    },
    Question {
        key: "voice.length",
        prompt: "你更习惯发一长段，还是拆成几条短消息？",
    },
    Question {
        key: "boundary.topics",
        prompt: "有哪些话题，你不希望 Soul 替你起草或分析？",
    },
    Question {
        key: "boundary.availability",
        prompt: "什么时间段你基本不回消息？",
    },
    Question {
        key: "preference.decision_style",
        prompt: "做决定时，你更看重把事情推进，还是先把细节想清楚？",
    },
    Question {
        key: "relationship.close_circle",
        prompt: "你最常联系的人大致是哪几类（同事、家人、老朋友……）？不用写名字。",
    },
    Question {
        key: "value.what_matters",
        prompt: "有没有一件事，是你希望 Soul 无论如何都替你守住的？",
    },
];

pub fn question(key: &str) -> Option<&'static Question> {
    QUESTIONS.iter().find(|question| question.key == key)
}

/// What the user typed, against one question.
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

/// A sink that keeps what it is given. Useful before WP03 exists, and in
/// tests that need to prove the seam is actually driven.
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
        if answer.text.trim().is_empty() {
            continue;
        }

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
                source_refs: vec![serde_json::json!({
                    "ref_kind": QUESTIONNAIRE_REF_KIND,
                    "event_id": event_id.to_string(),
                    "question_key": question.key,
                })],
                // The user said it about themselves. Nothing Soul infers later
                // outranks that; PRODUCT_LOCK makes a correction final.
                strength: SupportedBand::Strong,
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

fn answer_privacy() -> Privacy {
    Privacy {
        subject: Subject::Owner,
        derivation: Derivation::Raw,
        purposes: vec![Purpose::SoulProfile],
        retention: Retention::until_forgotten(),
        egress: EgressPolicy::default(),
    }
}
