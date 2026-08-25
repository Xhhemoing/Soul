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

use soul_algo_tie::TieScore;
use soul_policy::audit::{AuditContent, ReasonCode};
use soul_schema::audit::{AuditAction, AuditCounts};
use soul_schema::common::{NotAClinicalClaim, SchemaVersion, SupportedBand, Timestamp};
use soul_schema::contact::ContactClass;
use soul_schema::inference::{InferenceMethod, SoulInference, UserVerdict};
use soul_schema::relationship::{EgressScope, SoulRelationship};
use soul_store_api::{GraphStore, ProfileStore};

use crate::error::{GraphError, GraphResult};
use crate::model::{TieStrength, TieType};
use crate::t4d_adapt::InteractionInterner;
use crate::{interaction, interaction::Venue};

/// Prefix every statement this module writes shares, so a rebuild can find the
/// inferences it wrote last time without matching on the band.
pub const TIE_STATEMENT_PREFIX: &str = "graph.tie.";

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

/// One peer's observations, gathered for the scorer.
///
/// The scorer works in whole Unix seconds, but what an edge displays is the
/// instant as it was stored. Both are kept: the adapted rows go to
/// `soul_algo_tie`, the original RFC 3339 strings go on the edge.
#[derive(Debug, Clone)]
struct PeerEvidence {
    /// The interned id every row of this peer carries. Scratch, never stored.
    interned_peer_id: u64,
    /// This peer's adapted rows, in evidence order. Input to the scorer.
    rows: Vec<soul_algo_tie::Interaction>,
    evidence_ids: BTreeSet<Uuid>,
    /// RFC 3339 in UTC sorts lexicographically, which is why every writer in
    /// this workspace normalizes to `Z` before storing a timestamp. Comparing
    /// the stored strings keeps the exact instant a caller wrote, fractional
    /// seconds included.
    first_contact: String,
    last_contact: String,
    /// Newest one-to-one instant. `None` means the two have never spoken
    /// privately.
    last_direct_contact: Option<String>,
}

impl PeerEvidence {
    fn new(interned_peer_id: u64, occurred_at: &str) -> PeerEvidence {
        PeerEvidence {
            interned_peer_id,
            rows: Vec::new(),
            evidence_ids: BTreeSet::new(),
            first_contact: occurred_at.to_owned(),
            last_contact: occurred_at.to_owned(),
            last_direct_contact: None,
        }
    }

    fn absorb(
        &mut self,
        evidence_id: Uuid,
        row: soul_algo_tie::Interaction,
        occurred_at: &str,
        venue: Venue,
    ) {
        self.rows.push(row);
        self.evidence_ids.insert(evidence_id);
        if occurred_at < self.first_contact.as_str() {
            self.first_contact = occurred_at.to_owned();
        }
        if occurred_at > self.last_contact.as_str() {
            self.last_contact = occurred_at.to_owned();
        }
        if venue == Venue::Direct
            && self
                .last_direct_contact
                .as_deref()
                .is_none_or(|held| occurred_at > held)
        {
            self.last_direct_contact = Some(occurred_at.to_owned());
        }
    }
}

/// The band as the storage layer spells it.
fn supported_band(band: soul_algo_tie::Band) -> SupportedBand {
    match band {
        soul_algo_tie::Band::Weak => SupportedBand::Weak,
        soul_algo_tie::Band::Moderate => SupportedBand::Moderate,
        soul_algo_tie::Band::Strong => SupportedBand::Strong,
    }
}

/// The score, plus the display instants the scorer never saw, as one stored
/// object.
///
/// The counts all come from the score: they are the numbers the band was
/// decided on, and re-deriving any of them here would be a second opinion.
fn tie_strength_of(
    score: &TieScore,
    acc: &PeerEvidence,
    locked: Option<&TieStrength>,
) -> TieStrength {
    let machine_band = supported_band(score.band);
    let locked = locked.filter(|held| held.is_locked_by_user());
    TieStrength {
        band: match locked.and_then(|held| held.user_band) {
            Some(chosen) => chosen,
            None => machine_band,
        },
        interaction_count: score.interaction_count,
        outgoing_count: score.outgoing_count,
        incoming_count: score.incoming_count,
        conversation_count: score.conversation_count,
        active_day_count: score.active_day_count,
        first_contact_utc: Timestamp::new(acc.first_contact.clone()),
        last_contact_utc: Timestamp::new(acc.last_contact.clone()),
        direct_out_count: score.direct_out_count,
        direct_in_count: score.direct_in_count,
        group_out_count: score.group_out_count,
        group_in_count: score.group_in_count,
        direct_active_day_count: score.direct_active_day_count,
        last_direct_contact_utc: acc.last_direct_contact.clone().map(Timestamp::new),
        silent_days: score.silent_days,
        as_of_utc: Some(Timestamp::new(soul_policy::clock::rfc3339_utc(
            score.as_of_unix,
        ))),
        algorithm_id: score.algorithm_id.to_owned(),
        locked_by_user: locked.and_then(|held| held.locked_by_user),
        user_band: locked.and_then(|held| held.user_band),
        machine_band: Some(machine_band),
    }
}

/// Observed shapes, which stay any-venue.
///
/// `Reciprocal` here means both sides wrote something, anywhere. The band is
/// where the counting unit narrowed to one-to-one traffic; these two say what
/// was seen, not how much it is worth.
fn types_of(score: &TieScore) -> Vec<TieType> {
    vec![
        match score.any_direct() {
            true => TieType::Direct,
            false => TieType::GroupOnly,
        },
        match score.is_reciprocal() {
            true => TieType::Reciprocal,
            false => TieType::OneSided,
        },
    ]
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

    let mut interner = InteractionInterner::default();
    let mut peers: BTreeMap<Uuid, PeerEvidence> = BTreeMap::new();
    let mut interactions_read = 0u64;
    let mut peers_unresolved = 0u64;
    // The one instant this rebuild scores against: the newest observation in
    // the whole store. Never a clock, so the same evidence bands the same way
    // tomorrow, and never per peer, which would make every dormant tie look
    // current.
    let mut as_of_unix: Option<i64> = None;

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
            let row =
                interner
                    .adapt(&observation)
                    .map_err(|_| GraphError::UnreadableInteraction {
                        evidence_id: evidence.evidence_id,
                    })?;
            // A row whose peer has been forgotten still moves `as_of`:
            // forgetting a contact must not make everybody else look more
            // recently in touch than they are.
            as_of_unix = Some(match as_of_unix {
                Some(held) => held.max(row.occurred_at_unix),
                None => row.occurred_at_unix,
            });
            if !known.contains(&observation.peer_contact_id) {
                peers_unresolved += 1;
                continue;
            }
            peers
                .entry(observation.peer_contact_id)
                .or_insert_with(|| PeerEvidence::new(row.peer_id, observation.occurred_at.as_str()))
                .absorb(
                    evidence.evidence_id,
                    row,
                    observation.occurred_at.as_str(),
                    observation.venue,
                );
        }
    }

    let existing_edges = store.list_relationships()?;
    let existing_inferences = store.list_inferences()?;

    let mut build = GraphBuild {
        interactions_read,
        peers_unresolved,
        ..GraphBuild::default()
    };

    let Some(as_of_unix) = as_of_unix else {
        return Ok(build);
    };

    for (peer_id, acc) in &peers {
        let existing_edge = existing_edges
            .iter()
            .find(|edge| connects(edge, owner_id, *peer_id));
        let relationship_id = existing_edge
            .map(|edge| edge.relationship_id)
            .unwrap_or_else(Uuid::now_v7);
        let held = existing_edge.map(read_strength).transpose()?.flatten();

        let score = soul_algo_tie::score(acc.interned_peer_id, &acc.rows, as_of_unix);
        let strength = tie_strength_of(&score, acc, held.as_ref());
        let evidence_ids: Vec<Uuid> = acc.evidence_ids.iter().copied().collect();
        let edge = SoulRelationship {
            schema_version: SchemaVersion,
            relationship_id,
            from_contact_id: owner_id,
            to_contact_id: *peer_id,
            types: Some(
                types_of(&score)
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

        let existing_inference = existing_inferences
            .iter()
            .find(|inference| is_tie_statement_about(inference, relationship_id));
        let inference_id = existing_inference
            .map(|inference| inference.inference_id)
            .unwrap_or_else(Uuid::now_v7);
        store.put_inference(tie_inference(
            inference_id,
            relationship_id,
            *peer_id,
            &strength,
            &evidence_ids,
            existing_inference.and_then(|inference| inference.user_verdict),
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

/// What the last rebuild — or a correction since — left on this edge.
///
/// Refuses rather than guesses: an edge whose `tie_strength` cannot be read is
/// an edge that may be carrying a band the user chose, and overwriting it
/// would be the clobber the lock exists to prevent.
fn read_strength(edge: &SoulRelationship) -> GraphResult<Option<TieStrength>> {
    edge.tie_strength
        .clone()
        .map(|raw| {
            serde_json::from_value(raw).map_err(|_| GraphError::UnreadableEdge {
                relationship_id: edge.relationship_id,
                field: "tie_strength",
            })
        })
        .transpose()
}

fn connects(edge: &SoulRelationship, left: Uuid, right: Uuid) -> bool {
    (edge.from_contact_id == left && edge.to_contact_id == right)
        || (edge.from_contact_id == right && edge.to_contact_id == left)
}

/// Whether this inference is the tie statement about that edge.
///
/// `pub(crate)` for [`crate::correct`], which has to find the same inference a
/// rebuild would in order to record the user's verdict on it.
pub(crate) fn is_tie_statement_about(inference: &SoulInference, relationship_id: Uuid) -> bool {
    inference.statement_key.starts_with(TIE_STATEMENT_PREFIX)
        && inference
            .target
            .get("relationship_id")
            .and_then(Value::as_str)
            .is_some_and(|id| id == relationship_id.to_string())
}

/// The inference behind one edge.
///
/// `held_verdict` is whatever the user last said about this tie. A rebuild
/// updates what the machine thinks and leaves that answer alone: an opinion
/// the user already reviewed is not unreviewed again because new evidence
/// arrived.
fn tie_inference(
    inference_id: Uuid,
    relationship_id: Uuid,
    peer_id: Uuid,
    strength: &TieStrength,
    evidence_ids: &[Uuid],
    held_verdict: Option<UserVerdict>,
) -> SoulInference {
    // The machine's own reading, which keeps being written down even on an
    // edge whose effective band the user has taken over.
    let machine_band = strength.machine_band.unwrap_or(strength.band);
    let band = match machine_band {
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
        evidence_band: machine_band,
        // Counts, run through the frozen rule. Nothing was asked of a model.
        method: Some(InferenceMethod::Rule),
        user_verdict: Some(match held_verdict {
            Some(held) if held != UserVerdict::Unreviewed => held,
            _ => UserVerdict::Unreviewed,
        }),
        clinical_claim: NotAClinicalClaim,
        falsifier: Some(
            "双方在更长的时间窗口内都没有新的往来，或用户直接改写这条关系，即推翻本判断".into(),
        ),
    }
}
