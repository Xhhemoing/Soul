//! What one observed exchange between the user and one other person looks
//! like, and how it is written down.
//!
//! The graph is derived, not imported. Whatever the source format was — a
//! `soul-import-v1` line, a Telegram chat — an importer reduces it to
//! [`InteractionRef`] values and stores them in the `source_refs` of an
//! ordinary evidence row. Everything downstream reads evidence, so an edge can
//! always answer "which observations put you here" with ids that resolve.
//!
//! Forgetting a contact does not take those rows away. It destroys the content
//! keys their words were sealed under and tombstones the contact row, and it
//! leaves the evidence and the edge exactly where they are — deleting them
//! would leave every id the edge cites unresolvable, which is the one thing
//! `crate::view::resolve_evidence` refuses. What tells a reader the person is
//! gone is [`PersonNode::forget_state`](crate::model::PersonNode::forget_state),
//! so anything that would speak about a person has to look there:
//! `crate::build::rebuild` skips their observations, and `soul-draft` refuses
//! their summary.
//!
//! Three things are deliberately absent from this record:
//!
//! * the message text — it lives sealed in the event's `body_ref`, and the
//!   evidence only points at the event;
//! * the platform's conversation id — [`conversation_ref`] hashes it, because
//!   a raw chat id is a third-party identifier;
//! * anything the graph could treat as an instruction. This is data.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use soul_schema::common::{
    Derivation, EgressPolicy, Privacy, Purpose, Retention, SchemaVersion, Sha256Hex, Subject,
    SupportedBand, Timestamp,
};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};

/// Tag written into every interaction `source_ref`.
///
/// `source_refs` is `array of object` in the frozen contract, so anyone may
/// put anything there. The tag is how the graph tells its own rows apart from
/// another module's without guessing at field names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    GraphInteraction,
}

/// Who spoke, from the user's point of view.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// The user wrote it.
    Outgoing,
    /// The other person wrote it.
    Incoming,
}

/// Whether the exchange happened one to one or in front of an audience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Venue {
    Direct,
    Group,
}

/// One observed exchange between the user and one other person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionRef {
    pub ref_kind: RefKind,
    /// The event whose sealed body this observation came from.
    pub event_id: Uuid,
    pub self_contact_id: Uuid,
    pub peer_contact_id: Uuid,
    /// Hash of the source's conversation identifier. Never the identifier.
    pub conversation_ref: Sha256Hex,
    pub direction: Direction,
    pub occurred_at: Timestamp,
    pub venue: Venue,
}

impl InteractionRef {
    pub fn new(
        event_id: Uuid,
        self_contact_id: Uuid,
        peer_contact_id: Uuid,
        conversation_ref: Sha256Hex,
        direction: Direction,
        occurred_at: Timestamp,
        venue: Venue,
    ) -> InteractionRef {
        InteractionRef {
            ref_kind: RefKind::GraphInteraction,
            event_id,
            self_contact_id,
            peer_contact_id,
            conversation_ref,
            direction,
            occurred_at,
            venue,
        }
    }

    pub fn to_source_ref(&self) -> Value {
        serde_json::to_value(self).expect("an InteractionRef serializes to an object")
    }
}

/// Stable, non-reversible handle for one conversation.
///
/// `source` is mixed in so that chat `42` in one export and chat `42` in
/// another do not collide.
pub fn conversation_ref(source: &str, conversation_id: &str) -> Sha256Hex {
    let mut hasher = Sha256::new();
    hasher.update(b"soul.graph.conversation.v1|");
    hasher.update(source.as_bytes());
    hasher.update(b"|");
    hasher.update(conversation_id.as_bytes());
    Sha256Hex::new(hex::encode(hasher.finalize()))
}

/// The evidence row that carries one observation.
///
/// A single message is weak evidence on its own; strength comes from repetition
/// and is expressed on the edge, not here. The row is never exportable to
/// research and its egress policy denies everything, because the other person
/// did not agree to any of this.
pub fn interaction_evidence(
    evidence_id: Uuid,
    subject: Subject,
    interaction: &InteractionRef,
) -> SoulEvidence {
    SoulEvidence {
        schema_version: SchemaVersion,
        evidence_id,
        kind: EvidenceKind::Message,
        subject,
        source_refs: vec![interaction.to_source_ref()],
        strength: SupportedBand::Weak,
        // The user handed over an official export; nothing was inferred to get
        // here, and nothing was scraped.
        method: Some(EvidenceMethod::Manual),
        exportable_to_research: Some(false),
        privacy: Some(Privacy {
            subject,
            derivation: Derivation::Raw,
            purposes: vec![Purpose::Graph, Purpose::SoulProfile],
            retention: Retention::until_forgotten(),
            egress: EgressPolicy::default(),
        }),
    }
}

/// The interactions recorded in one evidence row, ignoring `source_refs` that
/// belong to some other module.
pub fn interactions_in(evidence: &SoulEvidence) -> Vec<InteractionRef> {
    evidence
        .source_refs
        .iter()
        .filter(|value| value.get("ref_kind").and_then(Value::as_str) == Some("graph_interaction"))
        .filter_map(|value| serde_json::from_value(value.clone()).ok())
        .collect()
}

/// Is this evidence row an interaction observation?
pub fn is_interaction(evidence: &SoulEvidence) -> bool {
    evidence.kind == EvidenceKind::Message && !interactions_in(evidence).is_empty()
}
