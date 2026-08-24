//! Deriving the graph from what was observed.
//!
//! One pass over the interaction evidence produces one edge per person the
//! user has exchanged anything with, plus one inference per edge saying how
//! strong the tie looks and why. Both are written through the storage
//! boundary, so both are subject to the same rule as everything else derived:
//! no evidence, no row.
//!
//! The build is idempotent. Edges are matched to the pair they connect and
//! inferences to the edge they are about, so running it again after an import
//! updates the tie instead of growing a second copy of the graph.
//!
//! v0.1 builds an ego network: every edge has the user at one end. Soul
//! observes conversations the user took part in, so an edge between two other
//! people would be a guess, and PRODUCT_LOCK is explicit that inferences carry
//! evidence.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use uuid::Uuid;

use soul_policy::audit::{AuditContent, ReasonCode};
use soul_schema::audit::{AuditAction, AuditCounts};
use soul_schema::common::{NotAClinicalClaim, SchemaVersion, SupportedBand, Timestamp};
use soul_schema::contact::ContactClass;
use soul_schema::inference::{InferenceMethod, SoulInference, UserVerdict};
use soul_schema::relationship::{EgressScope, SoulRelationship};
use soul_store_api::{GraphStore, ProfileStore};

use crate::error::{GraphError, GraphResult};
use crate::interaction::{self, Direction, InteractionRef, Venue};
use crate::model::{TieStrength, TieType};

/// Prefix every statement this module writes shares, so a rebuild can find the
/// inferences it wrote last time without matching on the band.
pub const TIE_STATEMENT_PREFIX: &str = "graph.tie.";

/// Reciprocal contact from here on is more than an exchanged greeting.
pub const MODERATE_MIN_INTERACTIONS: u64 = 3;
/// A strong tie has to be both frequent and spread over several days: twenty
/// messages in one afternoon is one conversation, not a habit.
pub const STRONG_MIN_INTERACTIONS: u64 = 10;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;

/// What one rebuild did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GraphBuild {
    pub interactions_read: u64,
    pub edges_written: Vec<Uuid>,
    pub inferences_written: Vec<Uuid>,
    /// Peers whose evidence was skipped because their contact row is gone —
    /// forgotten, most likely. Reported rather than silently dropped.
    pub peers_unresolved: u64,
    /// The entry the caller owes the chain. Built here, appended by whoever
    /// holds the open store, exactly as `soul-policy` does it.
    pub audit: Vec<AuditContent>,
}

/// Everything one peer's observations add up to.
#[derive(Debug, Clone)]
struct Tally {
    outgoing: u64,
    incoming: u64,
    conversations: BTreeSet<String>,
    active_days: BTreeSet<String>,
    first_contact: String,
    last_contact: String,
    any_direct: bool,
    evidence_ids: BTreeSet<Uuid>,
}

impl Tally {
    fn new(occurred_at: &str) -> Tally {
        Tally {
            outgoing: 0,
            incoming: 0,
            conversations: BTreeSet::new(),
            active_days: BTreeSet::new(),
            first_contact: occurred_at.to_owned(),
            last_contact: occurred_at.to_owned(),
            any_direct: false,
            evidence_ids: BTreeSet::new(),
        }
    }

    fn absorb(&mut self, evidence_id: Uuid, interaction: &InteractionRef) {
        match interaction.direction {
            Direction::Outgoing => self.outgoing += 1,
            Direction::Incoming => self.incoming += 1,
        }
        self.any_direct |= interaction.venue == Venue::Direct;
        self.conversations
            .insert(interaction.conversation_ref.as_str().to_owned());

        let at = interaction.occurred_at.as_str();
        // RFC 3339 in UTC sorts lexicographically, which is why every writer in
        // this workspace normalizes to `Z` before storing a timestamp.
        if at < self.first_contact.as_str() {
            self.first_contact = at.to_owned();
        }
        if at > self.last_contact.as_str() {
            self.last_contact = at.to_owned();
        }
        self.active_days.insert(utc_date(at));

        self.evidence_ids.insert(evidence_id);
    }

    fn interaction_count(&self) -> u64 {
        self.outgoing + self.incoming
    }

    fn is_reciprocal(&self) -> bool {
        self.outgoing > 0 && self.incoming > 0
    }

    fn band(&self) -> SupportedBand {
        let count = self.interaction_count();
        let days = self.active_days.len() as u64;
        if self.is_reciprocal() && count >= STRONG_MIN_INTERACTIONS && days >= STRONG_MIN_ACTIVE_DAYS
        {
            SupportedBand::Strong
        } else if self.is_reciprocal() && count >= MODERATE_MIN_INTERACTIONS {
            SupportedBand::Moderate
        } else {
            SupportedBand::Weak
        }
    }

    fn types(&self) -> Vec<TieType> {
        let mut types = vec![match self.any_direct {
            true => TieType::Direct,
            false => TieType::GroupOnly,
        }];
        types.push(match self.is_reciprocal() {
            true => TieType::Reciprocal,
            false => TieType::OneSided,
        });
        types
    }

    fn strength(&self) -> TieStrength {
        TieStrength {
            band: self.band(),
            interaction_count: self.interaction_count(),
            outgoing_count: self.outgoing,
            incoming_count: self.incoming,
            conversation_count: self.conversations.len() as u64,
            active_day_count: self.active_days.len() as u64,
            first_contact_utc: Timestamp::new(self.first_contact.clone()),
            last_contact_utc: Timestamp::new(self.last_contact.clone()),
        }
    }
}

/// The date part of an RFC 3339 instant, for counting active days.
fn utc_date(timestamp: &str) -> String {
    timestamp.chars().take(10).collect()
}

/// Rebuild every edge and tie inference from the evidence on hand.
pub fn rebuild<S>(store: &mut S) -> GraphResult<GraphBuild>
where
    S: GraphStore + ProfileStore,
{
    let contacts = store.list_contacts()?;
    let owners: Vec<Uuid> = contacts
        .iter()
        .filter(|contact| contact.contact_class == ContactClass::Owner)
        .map(|contact| contact.contact_id)
        .collect();
    if owners.len() > 1 {
        return Err(GraphError::AmbiguousOwner {
            count: owners.len(),
        });
    }
    let Some(owner_id) = owners.first().copied() else {
        // Nothing has been imported yet, so there is nobody to be at the centre
        // of the graph and nothing to derive.
        return Ok(GraphBuild::default());
    };
    let known: BTreeSet<Uuid> = contacts.iter().map(|contact| contact.contact_id).collect();

    let mut tallies: BTreeMap<Uuid, Tally> = BTreeMap::new();
    let mut interactions_read = 0u64;
    let mut peers_unresolved = 0u64;

    for evidence in store.list_evidence()? {
        for observation in interaction::interactions_in(&evidence) {
            if observation.peer_contact_id == observation.self_contact_id {
                return Err(GraphError::SelfLoop {
                    evidence_id: evidence.evidence_id,
                });
            }
            if observation.self_contact_id != owner_id {
                return Err(GraphError::DanglingContact {
                    evidence_id: evidence.evidence_id,
                    contact_id: observation.self_contact_id,
                });
            }
            interactions_read += 1;
            if !known.contains(&observation.peer_contact_id) {
                peers_unresolved += 1;
                continue;
            }
            tallies
                .entry(observation.peer_contact_id)
                .or_insert_with(|| Tally::new(observation.occurred_at.as_str()))
                .absorb(evidence.evidence_id, &observation);
        }
    }

    let existing_edges = store.list_relationships()?;
    let existing_inferences = store.list_inferences()?;

    let mut build = GraphBuild {
        interactions_read,
        peers_unresolved,
        ..GraphBuild::default()
    };

    for (peer_id, tally) in &tallies {
        let relationship_id = existing_edges
            .iter()
            .find(|edge| connects(edge, owner_id, *peer_id))
            .map(|edge| edge.relationship_id)
            .unwrap_or_else(Uuid::now_v7);

        let strength = tally.strength();
        let evidence_ids: Vec<Uuid> = tally.evidence_ids.iter().copied().collect();
        let edge = SoulRelationship {
            schema_version: SchemaVersion,
            relationship_id,
            from_contact_id: owner_id,
            to_contact_id: *peer_id,
            types: Some(
                tally
                    .types()
                    .into_iter()
                    .map(|kind| Value::String(kind.as_str().to_owned()))
                    .collect(),
            ),
            tie_strength: Some(
                serde_json::to_value(&strength).expect("TieStrength serializes to an object"),
            ),
            evidence_ids: evidence_ids.clone(),
            egress_scope: Some(EgressScope::LocalOnly),
        };
        store.put_relationship(edge)?;
        build.edges_written.push(relationship_id);

        let inference_id = existing_inferences
            .iter()
            .find(|inference| is_tie_statement_about(inference, relationship_id))
            .map(|inference| inference.inference_id)
            .unwrap_or_else(Uuid::now_v7);
        store.put_inference(tie_inference(
            inference_id,
            relationship_id,
            *peer_id,
            &strength,
            &evidence_ids,
        ))?;
        build.inferences_written.push(inference_id);
    }

    if !build.edges_written.is_empty() {
        build.audit.push(
            AuditContent::allowed(AuditAction::InferenceWrite, ReasonCode::Routine)
                .about(&build.edges_written)
                .counting(AuditCounts {
                    items: Some(build.inferences_written.len() as u64),
                    bytes: None,
                }),
        );
    }
    Ok(build)
}

fn connects(edge: &SoulRelationship, left: Uuid, right: Uuid) -> bool {
    (edge.from_contact_id == left && edge.to_contact_id == right)
        || (edge.from_contact_id == right && edge.to_contact_id == left)
}

fn is_tie_statement_about(inference: &SoulInference, relationship_id: Uuid) -> bool {
    inference.statement_key.starts_with(TIE_STATEMENT_PREFIX)
        && inference
            .target
            .get("relationship_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id == relationship_id.to_string())
}

fn tie_inference(
    inference_id: Uuid,
    relationship_id: Uuid,
    peer_id: Uuid,
    strength: &TieStrength,
    evidence_ids: &[Uuid],
) -> SoulInference {
    let band = match strength.band {
        SupportedBand::Weak => "weak",
        SupportedBand::Moderate => "moderate",
        SupportedBand::Strong => "strong",
    };
    SoulInference {
        schema_version: SchemaVersion,
        inference_id,
        target: serde_json::json!({
            "kind": "relationship",
            "relationship_id": relationship_id.to_string(),
            "contact_id": peer_id.to_string(),
        }),
        statement_key: format!("{TIE_STATEMENT_PREFIX}{band}"),
        evidence_ids: evidence_ids.to_vec(),
        evidence_band: strength.band,
        // Thresholds, applied to counts. Nothing was asked of a model.
        method: Some(InferenceMethod::Rule),
        user_verdict: Some(UserVerdict::Unreviewed),
        clinical_claim: NotAClinicalClaim,
        falsifier: Some(
            "双方在更长的时间窗口内都没有新的往来，或用户直接改写这条关系，即推翻本判断".into(),
        ),
    }
}
