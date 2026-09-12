//! What drafting and summarizing refuse to do, and what they say when they do.

use uuid::Uuid;

use soul_policy::clinical::NonClinicalViolation;
use soul_policy::ReasonCode;

pub type DraftResult<T> = Result<T, DraftError>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DraftError {
    /// A summary point with nothing behind it. PRODUCT_LOCK says every
    /// inference carries evidence, and a sentence about a person is an
    /// inference whatever it is called.
    #[error("a summary point must cite at least one evidence row")]
    NoEvidence,

    /// The caller passed fewer resolved rows than the edge cites, or rows that
    /// are not the ones it cites. Returning a shorter list would let a summary
    /// claim support it does not have.
    #[error("evidence {evidence_id} is cited but was not resolved")]
    UnresolvedEvidence { evidence_id: Uuid },

    #[error("the graph has no edge to contact {contact_id}, so there is nothing to summarize")]
    NoSuchTie { contact_id: Uuid },

    /// The contact is a tombstone. A forget destroys the keys their words were
    /// sealed under and leaves the derived rows where they are, so the edge and
    /// the evidence ids behind it survive and a summary built from them would
    /// go on citing a person the user asked Soul to drop. The refusal is the
    /// whole answer: no points, no projection, and nothing offered to an
    /// endpoint.
    #[error("这个人已经被遗忘了，本机没有还能引用的东西可以说（联系人 {contact_id}）")]
    Forgotten { contact_id: Uuid },

    /// Something on its way to the screen carried vocabulary this product must
    /// not use. A bug in the generator, not something to filter and ship.
    #[error(transparent)]
    Clinical(#[from] NonClinicalViolation),

    /// The user's own endpoint did not produce a reply. Drafting refuses
    /// rather than falling back, because "no endpoint was reached" and "the
    /// endpoint said something unusable" are different facts and the second
    /// one is the only one worth degrading over.
    #[error(transparent)]
    Generation(#[from] GenerationRefused),
}

impl DraftError {
    /// The audit `reason_code`, where the refusal has one. Never any prose.
    pub fn reason_code(&self) -> Option<ReasonCode> {
        match self {
            DraftError::Generation(refused) => refused.reason_code,
            _ => None,
        }
    }
}

/// A generation request that did not come back with model text.
///
/// `reason` is for the person reading the failure; it must not repeat anything
/// the endpoint said, because a hostile endpoint controls that text.
/// `soul-egress` already redacts its own error strings for the same reason.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the endpoint did not produce a draft: {reason}")]
pub struct GenerationRefused {
    pub reason: String,
    pub reason_code: Option<ReasonCode>,
}

impl GenerationRefused {
    pub fn new(reason: impl Into<String>) -> GenerationRefused {
        GenerationRefused {
            reason: reason.into(),
            reason_code: None,
        }
    }

    pub fn because(mut self, code: ReasonCode) -> GenerationRefused {
        self.reason_code = Some(code);
        self
    }
}
