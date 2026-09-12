//! Failures a graph build or read can hit.
//!
//! Every variant names ids and counts. None of them can carry prose: a message
//! body never reaches this crate, only the evidence that points at one.

use uuid::Uuid;

use soul_store_api::types::StoreError;

pub type GraphResult<T> = Result<T, GraphError>;

#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    #[error("the store refused: {0}")]
    Store(#[from] StoreError),

    /// An interaction points at a contact row that is not there. The importer
    /// writes contacts before evidence, so this means the two disagree.
    #[error(
        "interaction evidence {evidence_id} names contact {contact_id}, which does not resolve"
    )]
    DanglingContact { evidence_id: Uuid, contact_id: Uuid },

    /// An interaction claims the user is on both ends.
    #[error("interaction evidence {evidence_id} has the same contact on both ends")]
    SelfLoop { evidence_id: Uuid },

    /// An interaction carries an instant the build cannot turn into a UTC
    /// second. Damaged evidence fails the rebuild rather than being folded in
    /// under a timestamp nobody could read.
    #[error("interaction evidence {evidence_id} carries a timestamp this build cannot read")]
    UnreadableInteraction { evidence_id: Uuid },

    /// A stored edge carries `types` or `tie_strength` this build cannot read.
    /// The contract leaves both free-form, so an older or hand-edited row can
    /// legitimately end up here; the graph refuses rather than inventing a
    /// strength band the evidence does not support.
    #[error("edge {relationship_id} has a `{field}` this version cannot read")]
    UnreadableEdge {
        relationship_id: Uuid,
        field: &'static str,
    },

    /// The user asked to move the band on an edge one end of which is a
    /// tombstone. A forget leaves the relationship row and the evidence behind
    /// it standing — deleting them would take the whole graph view down with
    /// them — so the stale edge is still on screen with the band words under
    /// it. Writing one would record a fresh `UserCorrection` about somebody the
    /// user asked Soul to drop, and re-file their orphaned tie inference live.
    ///
    /// Written in Chinese, unlike the variants above it: this is the one
    /// failure here a user can reach by pressing a button, and
    /// `SessionRefusal` shows it verbatim. `soul_draft::DraftError::Forgotten`
    /// is the same sentence on the summary side.
    #[error("这个人已经被遗忘了，这一条关系的档位不能再改（关系 {relationship_id}）")]
    Forgotten { relationship_id: Uuid },

    /// Two contacts are marked `self`. The graph is an ego network and would
    /// otherwise silently pick one.
    #[error("the store holds {count} contacts of class `self`; there must be exactly one")]
    AmbiguousOwner { count: usize },
}
