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
///
/// Every field below `last_contact_utc` arrived with the frozen tie-strength
/// rule and carries `#[serde(default)]`, so an edge written before it still
/// reads back through [`crate::view::load`] and is replaced whole by the next
/// rebuild.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TieStrength {
    /// The band in force: the user's own when the edge is locked, the
    /// machine's otherwise.
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

    /// One-to-one messages the user wrote.
    #[serde(default)]
    pub direct_out_count: u64,
    /// One-to-one messages the peer wrote.
    #[serde(default)]
    pub direct_in_count: u64,
    /// Messages the user wrote with other people in the room.
    #[serde(default)]
    pub group_out_count: u64,
    /// Messages the peer wrote with other people in the room.
    #[serde(default)]
    pub group_in_count: u64,
    /// Distinct UTC days with a one-to-one exchange. Never larger than
    /// `active_day_count`.
    #[serde(default)]
    pub direct_active_day_count: u64,
    /// The newest one-to-one exchange. `None` means there has never been one,
    /// rather than a 1970 sentinel.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_direct_contact_utc: Option<Timestamp>,
    /// Whole UTC days between `last_contact_utc` — any venue — and `as_of`.
    #[serde(default)]
    pub silent_days: i64,
    /// The one store-wide instant this rebuild scored against. `None` only on
    /// a row written before the frozen rule landed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub as_of_utc: Option<Timestamp>,
    /// Which rule produced the counts. Empty only on a legacy row.
    #[serde(default)]
    pub algorithm_id: String,

    /// Set once the user has overruled the band on this edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_by_user: Option<bool>,
    /// The band the user chose. `Some` exactly when the edge is locked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_band: Option<SupportedBand>,
    /// What the frozen rule made of the same counts, kept beside the effective
    /// band so a locked edge can still show both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_band: Option<SupportedBand>,
}

impl TieStrength {
    /// True once the user has overruled the machine on this edge.
    pub fn is_locked_by_user(&self) -> bool {
        self.locked_by_user == Some(true) && self.user_band.is_some()
    }
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
