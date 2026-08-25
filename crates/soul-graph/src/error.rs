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

    /// A stored edge carries a band no run of the frozen rule stands behind —
    /// an edge written before the rule landed, on a store that no longer holds
    /// the observations to score it. The band cannot be made reviewable, so it
    /// is not one the user can be allowed to lock either.
    #[error(
        "edge {relationship_id} carries a band no algorithm produced, and a rebuild did not \
         score it; the observations behind this tie have to be in the store and rebuilt before \
         the band can be corrected"
    )]
    UnscoredEdge { relationship_id: Uuid },

    /// Two contacts are marked `self`. The graph is an ego network and would
    /// otherwise silently pick one.
    #[error("the store holds {count} contacts of class `self`; there must be exactly one")]
    AmbiguousOwner { count: usize },
}
