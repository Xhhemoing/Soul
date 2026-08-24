//! Reading the graph back, with the evidence still attached.
//!
//! AC-06 is the reason the resolution helpers are here rather than in the UI:
//! every edge names evidence ids, and those ids have to lead somewhere. A view
//! that returned ids nobody had checked would look exactly the same on screen
//! and mean nothing.

use uuid::Uuid;

use soul_schema::contact::ContactClass;
use soul_schema::evidence::SoulEvidence;
use soul_schema::relationship::{EgressScope, SoulRelationship};
use soul_store_api::{GraphStore, ProfileStore};

use crate::error::{GraphError, GraphResult};
use crate::model::{PersonNode, SoulGraph, TieEdge, TieStrength, TieType};

/// Load the graph the last [`crate::build::rebuild`] left behind.
pub fn load<S: GraphStore>(store: &S) -> GraphResult<SoulGraph> {
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

    let mut edges = Vec::new();
    for stored in store.list_relationships()? {
        edges.push(read_edge(&stored)?);
    }

    let nodes = contacts
        .into_iter()
        .map(|contact| {
            let touching: Vec<&TieEdge> = edges
                .iter()
                .filter(|edge| edge.touches(contact.contact_id))
                .collect();
            PersonNode {
                contact_id: contact.contact_id,
                contact_class: contact.contact_class,
                label_ref: contact.display_label_ref,
                identifier_hashes: contact
                    .identifiers
                    .unwrap_or_default()
                    .into_iter()
                    .map(|identifier| identifier.value_hash)
                    .collect(),
                forget_state: contact.forget_state,
                egress_scope: EgressScope::LocalOnly,
                interaction_count: touching
                    .iter()
                    .map(|edge| edge.tie_strength.interaction_count)
                    .sum(),
                last_contact_utc: touching
                    .iter()
                    .map(|edge| edge.tie_strength.last_contact_utc.clone())
                    .max_by(|left, right| left.as_str().cmp(right.as_str())),
                edge_ids: touching.iter().map(|edge| edge.relationship_id).collect(),
            }
        })
        .collect();

    Ok(SoulGraph {
        self_contact_id: owners.first().copied(),
        nodes,
        edges,
    })
}

/// Every evidence row one edge rests on.
///
/// Fails rather than skipping: an id that does not resolve means the edge is
/// making a claim nothing supports, and the caller needs to know that instead
/// of being handed a shorter list.
pub fn resolve_evidence<S: ProfileStore>(
    store: &S,
    edge: &TieEdge,
) -> GraphResult<Vec<SoulEvidence>> {
    let mut resolved = Vec::with_capacity(edge.evidence_ids.len());
    for evidence_id in &edge.evidence_ids {
        resolved.push(store.get_evidence(*evidence_id)?);
    }
    Ok(resolved)
}

/// Turn a stored edge back into the typed form.
///
/// `types` and `tie_strength` are free-form in the frozen contract, so this is
/// where the graph's own reading of them is enforced.
fn read_edge(stored: &SoulRelationship) -> GraphResult<TieEdge> {
    let unreadable = |field: &'static str| GraphError::UnreadableEdge {
        relationship_id: stored.relationship_id,
        field,
    };

    let raw_types = stored.types.clone().ok_or_else(|| unreadable("types"))?;
    let mut types = Vec::with_capacity(raw_types.len());
    for value in raw_types {
        types.push(serde_json::from_value::<TieType>(value).map_err(|_| unreadable("types"))?);
    }

    let raw_strength = stored
        .tie_strength
        .clone()
        .ok_or_else(|| unreadable("tie_strength"))?;
    let tie_strength: TieStrength =
        serde_json::from_value(raw_strength).map_err(|_| unreadable("tie_strength"))?;

    Ok(TieEdge {
        relationship_id: stored.relationship_id,
        from_contact_id: stored.from_contact_id,
        to_contact_id: stored.to_contact_id,
        types,
        tie_strength,
        evidence_ids: stored.evidence_ids.clone(),
        egress_scope: stored.egress_scope.unwrap_or(EgressScope::LocalOnly),
    })
}
