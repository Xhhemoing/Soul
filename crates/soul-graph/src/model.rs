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
/// rule. An edge written before it carries none of them, still reads back
/// through [`crate::view::load`], and is replaced whole by the next rebuild;
/// see [`Wire`] for why they are absent rather than zero on the way out.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub direct_out_count: u64,
    /// One-to-one messages the peer wrote.
    pub direct_in_count: u64,
    /// Messages the user wrote with other people in the room.
    pub group_out_count: u64,
    /// Messages the peer wrote with other people in the room.
    pub group_in_count: u64,
    /// Distinct UTC days with a one-to-one exchange. Never larger than
    /// `active_day_count`.
    pub direct_active_day_count: u64,
    /// The newest one-to-one exchange. `None` means there has never been one,
    /// rather than a 1970 sentinel.
    pub last_direct_contact_utc: Option<Timestamp>,
    /// Whole UTC days between `last_contact_utc` — any venue — and `as_of`.
    pub silent_days: i64,
    /// The one store-wide instant this rebuild scored against. `None` only on
    /// a row written before the frozen rule landed.
    pub as_of_utc: Option<Timestamp>,
    /// Which rule produced the counts. Empty only on a legacy row.
    pub algorithm_id: String,

    /// Set once the user has overruled the band on this edge.
    pub locked_by_user: Option<bool>,
    /// The band the user chose. `Some` exactly when the edge is locked.
    pub user_band: Option<SupportedBand>,
    /// What the frozen rule made of the same counts, kept beside the effective
    /// band so a locked edge can still show both.
    pub machine_band: Option<SupportedBand>,
}

impl TieStrength {
    /// True once the user has overruled the machine on this edge.
    pub fn is_locked_by_user(&self) -> bool {
        self.locked_by_user == Some(true) && self.user_band.is_some()
    }

    /// Whether this strength came out of the frozen rule at all.
    fn names_its_rule(&self) -> bool {
        !self.algorithm_id.is_empty()
    }
}

/// [`TieStrength`] as it is stored.
///
/// `relationship.schema.json` refuses a half-migrated edge: name a rule in
/// `algorithm_id` and the whole reviewable surface must be there, because a
/// band nobody can recount is the thing the contract exists to prevent. The
/// converse is the reason this type exists at all — on an edge from before the
/// rule, those fields were never measured, and writing `direct_out_count: 0`
/// would be this crate asserting a count it never took. Rust cannot say
/// "these fields travel together" on a struct field, so the wire shape is
/// written down once here and the in-memory type stays flat.
#[derive(Serialize, Deserialize)]
struct Wire {
    band: SupportedBand,
    interaction_count: u64,
    outgoing_count: u64,
    incoming_count: u64,
    conversation_count: u64,
    active_day_count: u64,
    first_contact_utc: Timestamp,
    last_contact_utc: Timestamp,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    algorithm_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    direct_out_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    direct_in_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group_out_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group_in_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    direct_active_day_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_direct_contact_utc: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    silent_days: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    as_of_utc: Option<Timestamp>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    locked_by_user: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_band: Option<SupportedBand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    machine_band: Option<SupportedBand>,
}

impl From<&TieStrength> for Wire {
    fn from(strength: &TieStrength) -> Wire {
        let measured = strength.names_its_rule();
        Wire {
            band: strength.band,
            interaction_count: strength.interaction_count,
            outgoing_count: strength.outgoing_count,
            incoming_count: strength.incoming_count,
            conversation_count: strength.conversation_count,
            active_day_count: strength.active_day_count,
            first_contact_utc: strength.first_contact_utc.clone(),
            last_contact_utc: strength.last_contact_utc.clone(),

            algorithm_id: measured.then(|| strength.algorithm_id.clone()),
            direct_out_count: measured.then_some(strength.direct_out_count),
            direct_in_count: measured.then_some(strength.direct_in_count),
            group_out_count: measured.then_some(strength.group_out_count),
            group_in_count: measured.then_some(strength.group_in_count),
            direct_active_day_count: measured.then_some(strength.direct_active_day_count),
            last_direct_contact_utc: measured
                .then(|| strength.last_direct_contact_utc.clone())
                .flatten(),
            silent_days: measured.then_some(strength.silent_days),
            as_of_utc: measured.then(|| strength.as_of_utc.clone()).flatten(),

            locked_by_user: strength.locked_by_user,
            user_band: strength.user_band,
            machine_band: strength.machine_band,
        }
    }
}

impl From<Wire> for TieStrength {
    fn from(wire: Wire) -> TieStrength {
        TieStrength {
            band: wire.band,
            interaction_count: wire.interaction_count,
            outgoing_count: wire.outgoing_count,
            incoming_count: wire.incoming_count,
            conversation_count: wire.conversation_count,
            active_day_count: wire.active_day_count,
            first_contact_utc: wire.first_contact_utc,
            last_contact_utc: wire.last_contact_utc,

            direct_out_count: wire.direct_out_count.unwrap_or_default(),
            direct_in_count: wire.direct_in_count.unwrap_or_default(),
            group_out_count: wire.group_out_count.unwrap_or_default(),
            group_in_count: wire.group_in_count.unwrap_or_default(),
            direct_active_day_count: wire.direct_active_day_count.unwrap_or_default(),
            last_direct_contact_utc: wire.last_direct_contact_utc,
            silent_days: wire.silent_days.unwrap_or_default(),
            as_of_utc: wire.as_of_utc,
            algorithm_id: wire.algorithm_id.unwrap_or_default(),

            locked_by_user: wire.locked_by_user,
            user_band: wire.user_band,
            machine_band: wire.machine_band,
        }
    }
}

impl Serialize for TieStrength {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Wire::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for TieStrength {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<TieStrength, D::Error> {
        Wire::deserialize(deserializer).map(TieStrength::from)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn bare() -> TieStrength {
        TieStrength {
            band: SupportedBand::Moderate,
            interaction_count: 4,
            outgoing_count: 3,
            incoming_count: 1,
            conversation_count: 1,
            active_day_count: 2,
            first_contact_utc: Timestamp::new("2026-07-30T09:00:00Z"),
            last_contact_utc: Timestamp::new("2026-07-31T09:00:00Z"),
            direct_out_count: 0,
            direct_in_count: 0,
            group_out_count: 0,
            group_in_count: 0,
            direct_active_day_count: 0,
            last_direct_contact_utc: None,
            silent_days: 0,
            as_of_utc: None,
            algorithm_id: String::new(),
            locked_by_user: None,
            user_band: None,
            machine_band: None,
        }
    }

    /// A strength that names no rule writes none of the rule's numbers.
    ///
    /// Zero is a count, and this one was never taken. The contract says the
    /// same thing from the other side: `dependentRequired` refuses any of
    /// these keys without `algorithm_id` beside it.
    #[test]
    fn an_unmeasured_split_is_absent_rather_than_zero() {
        let stored = serde_json::to_value(bare()).expect("serializes");
        let object = stored.as_object().expect("an object");
        for never_measured in [
            "algorithm_id",
            "direct_out_count",
            "direct_in_count",
            "group_out_count",
            "group_in_count",
            "direct_active_day_count",
            "silent_days",
            "as_of_utc",
            "last_direct_contact_utc",
        ] {
            assert!(
                !object.contains_key(never_measured),
                "{never_measured} was never measured and must not be written",
            );
        }
        assert_eq!(object["interaction_count"], 4);
        assert_eq!(
            serde_json::from_value::<TieStrength>(stored).expect("reads back"),
            bare(),
        );
    }

    /// Once a rule is named, every number it decided on is written, including
    /// the zeroes — those it did take.
    #[test]
    fn a_measured_split_is_written_out_in_full_zeroes_included() {
        let measured = TieStrength {
            algorithm_id: "T4D".to_owned(),
            as_of_utc: Some(Timestamp::new("2026-08-01T09:00:00Z")),
            direct_out_count: 3,
            direct_in_count: 1,
            ..bare()
        };
        let stored = serde_json::to_value(&measured).expect("serializes");
        let object = stored.as_object().expect("an object");

        assert_eq!(object["algorithm_id"], "T4D");
        assert_eq!(object["group_out_count"], 0);
        assert_eq!(object["group_in_count"], 0);
        assert_eq!(object["silent_days"], 0);
        assert!(
            !object.contains_key("last_direct_contact_utc"),
            "the one field the contract lets an implementation skip",
        );
        assert_eq!(
            serde_json::from_value::<TieStrength>(stored).expect("reads back"),
            measured,
        );
    }
}
