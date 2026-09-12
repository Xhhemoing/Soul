//! What the profile surface refuses to do, and why.

use uuid::Uuid;

use soul_policy::clinical::NonClinicalViolation;
use soul_store_api::types::StoreError;

use crate::numeric::NumericRating;

pub type ProfileResult<T> = Result<T, ProfileError>;

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error(transparent)]
    Store(#[from] StoreError),

    /// PRODUCT_LOCK: "推断必须有证据". Refused here rather than at the store so
    /// the profile is not half-updated before the write is rejected.
    #[error("an inference must cite at least one piece of evidence, and this one cites none")]
    NoEvidence,

    #[error("{0} is not one of the profile's trait axes")]
    UnknownAxis(Uuid),

    #[error("`{0}` is not a question in this questionnaire")]
    UnknownQuestion(String),

    #[error("question `{question_id}` is about {expected}, but the answer is about {found}")]
    AnswerTargetMismatch {
        question_id: String,
        expected: &'static str,
        found: &'static str,
    },

    /// A wizard offered an answer the question does not have. The value is not
    /// repeated: on a choice question it is a token nobody offered, and on the
    /// path where it came from a text box it would be the user's own words.
    #[error("question `{question_id}` offers {offered} option(s), and this answer is not one")]
    UnofferedOption { question_id: String, offered: usize },

    #[error("question `{0}` was answered twice")]
    DuplicateAnswer(String),

    /// The recorder refused, or the store did while it was recording. The
    /// questionnaire is written by `soul-import` and interpreted here, so its
    /// failures reach the caller as they are.
    #[error(transparent)]
    Recording(#[from] soul_import::questionnaire::QuestionnaireError),

    #[error("a questionnaire with no answers would produce an empty profile")]
    EmptyQuestionnaire,

    /// An inference cited evidence, but the evidence is no longer readable.
    /// AC-06 is only worth anything if a dangling reference is an error.
    #[error("inference {inference_id} cites evidence {evidence_id}, which does not resolve")]
    DanglingEvidence {
        inference_id: Uuid,
        evidence_id: Uuid,
    },

    #[error(transparent)]
    NonClinical(#[from] NonClinicalViolation),

    #[error(transparent)]
    NumericRating(#[from] NumericRating),
}
