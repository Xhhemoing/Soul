//! What drafting and summarising refuse to do, and why.
//!
//! Every variant names ids, counts or a denied term. None of them can carry
//! prose: the message body of a pasted turn never reaches an error string, so
//! an error that reaches a log or a UI is as free of third-party content as the
//! audit chain is.

use uuid::Uuid;

use soul_policy::clinical::NonClinicalViolation;

pub type DraftResult<T> = Result<T, DraftError>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DraftError {
    /// The template or the endpoint produced vocabulary this product must not
    /// say. An assertion, not a filter: the caller is expected to surface the
    /// failure, not to strip the word and carry on.
    #[error(transparent)]
    NonClinical(#[from] NonClinicalViolation),

    /// The endpoint answered with something that is not a chat completion.
    /// Its body is data, so a shape this build cannot read is a readable
    /// failure rather than a guess.
    #[error("the endpoint's answer is not a chat completion this build can read")]
    UnreadableAnswer,

    /// An edge with no evidence is a claim nothing supports.
    #[error("tie {relationship_id} cites no evidence, so there is no claim to make")]
    ClaimWithoutEvidence { relationship_id: Uuid },

    /// The rows handed in do not match the ids the edge cites. The store layer
    /// already refuses to resolve a missing id; this is the second net, so a
    /// caller that filtered its own list cannot produce a shorter claim.
    #[error("tie {relationship_id} cites evidence {evidence_id}, which did not resolve")]
    DanglingEvidence {
        relationship_id: Uuid,
        evidence_id: Uuid,
    },

    /// Nothing was ever observed about this person, so there is nothing
    /// supported to say. An empty summary would read like an answer.
    #[error("contact {contact_id} has no observed tie to summarise")]
    NothingObserved { contact_id: Uuid },
}
