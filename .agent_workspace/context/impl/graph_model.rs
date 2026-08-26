//! The shape of the graph the user is shown.
//!
//! PRODUCT_LOCK names four things an edge must carry: interaction strength,
//! relationship type, last contact, and evidence. All four are here, and all
//! four are derived from observations that can be pointed at.
//!
//! What is *not* here is any prose. A node's label is a [`SealedText`]
//! pointer, identifiers are hashes, and every third-party node is marked
//! `local_only`. A caller that wants to show a name has to open the seal
//! deliberately; a caller that is building a request body cannot do it by
//! accident.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_schema::common::{SealedText, Sha256Hex, SupportedBand, Timestamp};
use soul_schema::contact::ContactClass;
use soul_schema::memory::ForgetState;
use soul_schema::relationship::EgressScope;

/// What kind of tie the observations support.
///
/// These are *observed* shapes, not claims about the relationship. v0.1 has no
/// basis for saying "colleague" or "family" — that is something the user tells
/// Soul, and the contract leaves `types` free-form so a user-stated value can
/// sit in the same array later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TieType {
    /// At least one exchange happened one to one.
    Direct,
    /// Only ever seen with other people in the conversation.
    GroupOnly,
    /// Both sides have written at least once.
    Reciprocal,
    /// Only one side has ever written.
    OneSided,
}

impl TieType {
    pub const fn as_str(self) -> &'static str {
        match self {
            TieType::Direct => "direct",
            TieType::GroupOnly => "group_only",
            TieType::Reciprocal => "reciprocal",
            TieType::OneSided => "one_sided",
        }
    }
}

/// How much contact there has been, and when.
///
/// Counts, not a rating. DECISIONS D22 rules out numeric scales for the
/// psychological model; the same instinct applies here, so the summary value a
/// caller reads is [`SupportedBand`] and the numbers underneath are plain
/// tallies a user could verify by counting messages themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TieStrength {
    pub band: SupportedBand,
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    /// Distinct conversations the two of you have shared.
    pub conversation_count: u64,
    /// Distinct UTC dates on which anything was exchanged.
    pub active_day_count: u64,
    pub first_contact_utc: Timestamp,
    pub last_contact_utc: Timestamp,
}

/// A person.
///
/// `label_ref` is a pointer to sealed text; there is no field on this struct
/// that holds a name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonNode {
    pub contact_id: Uuid,
    pub contact_class: ContactClass,
    pub label_ref: Option<SealedText>,
    pub identifier_hashes: Vec<Sha256Hex>,
    pub forget_state: ForgetState,
    /// `local_only`, which is the only scope the contract admits. A node about
    /// someone else carries it rather than each caller having to remember that
    /// third-party data stays put.
    pub egress_scope: EgressScope,
    pub interaction_count: u64,
    pub last_contact_utc: Option<Timestamp>,
    pub edge_ids: Vec<Uuid>,
}

impl PersonNode {
    pub fn is_owner(&self) -> bool {
        self.contact_class == ContactClass::Owner
    }

    /// True for every node the redactor and the research preview must treat as
    /// someone else's data.
    pub fn is_third_party(&self) -> bool {
        !self.is_owner()
    }
}

/// A tie between two people, with what supports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TieEdge {
    pub relationship_id: Uuid,
    pub from_contact_id: Uuid,
    pub to_contact_id: Uuid,
    pub types: Vec<TieType>,
    pub tie_strength: TieStrength,
    /// Non-empty by construction. An edge nobody observed is not an edge.
    pub evidence_ids: Vec<Uuid>,
    pub egress_scope: EgressScope,
}

impl TieEdge {
    pub fn touches(&self, contact_id: Uuid) -> bool {
        self.from_contact_id == contact_id || self.to_contact_id == contact_id
    }

    /// The end of the edge that is not `contact_id`.
    pub fn other_end(&self, contact_id: Uuid) -> Option<Uuid> {
        match contact_id {
            id if id == self.from_contact_id => Some(self.to_contact_id),
            id if id == self.to_contact_id => Some(self.from_contact_id),
            _ => None,
        }
    }
}

/// The whole graph, as the UI reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulGraph {
    /// The user's own node, if a contact of class `self` exists yet.
    pub self_contact_id: Option<Uuid>,
    pub nodes: Vec<PersonNode>,
    pub edges: Vec<TieEdge>,
}

impl SoulGraph {
    pub fn node(&self, contact_id: Uuid) -> Option<&PersonNode> {
        self.nodes.iter().find(|node| node.contact_id == contact_id)
    }

    pub fn edge(&self, relationship_id: Uuid) -> Option<&TieEdge> {
        self.edges
            .iter()
            .find(|edge| edge.relationship_id == relationship_id)
    }

    pub fn edges_for(&self, contact_id: Uuid) -> Vec<&TieEdge> {
        self.edges
            .iter()
            .filter(|edge| edge.touches(contact_id))
            .collect()
    }

    pub fn third_party_nodes(&self) -> Vec<&PersonNode> {
        self.nodes
            .iter()
            .filter(|node| node.is_third_party())
            .collect()
    }

    /// Every node that is not the user, and every edge, is `local_only`.
    ///
    /// The check is here rather than left to each caller so that a test can
    /// assert the property over a whole graph, and so a future node kind that
    /// forgets to set the scope shows up immediately.
    pub fn third_party_data_is_local_only(&self) -> bool {
        self.third_party_nodes()
            .iter()
            .all(|node| node.egress_scope == EgressScope::LocalOnly)
            && self
                .edges
                .iter()
                .all(|edge| edge.egress_scope == EgressScope::LocalOnly)
    }
}
