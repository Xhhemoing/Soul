//! WP05's command surface: rebuild the people graph, read it back, and resolve
//! what an edge rests on.
//!
//! Thin, like the rest of this module. `soul-graph` decides what an edge is and
//! `soul-store` holds it; what is here is the wiring, plus the one thing
//! neither of them can do alone — appending the audit entry the rebuild owes
//! the chain, which needs the open store and the caller's clock.

use uuid::Uuid;

use soul_graph::model::{SoulGraph, TieEdge};
use soul_graph::{GraphBuild, GraphError};
use soul_policy::audit::append_or_store_error;
use soul_schema::evidence::SoulEvidence;
use soul_store::SqlCipherStore;

/// Derive every edge and tie inference from the evidence on hand.
///
/// Idempotent: running it after a second import updates the ties rather than
/// growing a second graph. `at_unix_seconds` is the caller's clock, passed in
/// so a replay writes the same audit entry.
pub fn rebuild(store: &mut SqlCipherStore, at_unix_seconds: i64) -> Result<GraphBuild, GraphError> {
    let build = soul_graph::rebuild(store)?;
    for content in build.audit.clone() {
        append_or_store_error(store, content, at_unix_seconds)?;
    }
    Ok(build)
}

/// The graph as the UI reads it. Reads nothing but what a rebuild left behind.
pub fn load(store: &SqlCipherStore) -> Result<SoulGraph, GraphError> {
    soul_graph::load(store)
}

/// The evidence rows one edge cites.
///
/// AC-06 on the graph side: an edge names ids, and this is what turns them
/// back into rows. It fails rather than returning a shorter list, because an
/// edge whose evidence has gone is making a claim nothing supports.
pub fn edge_evidence(
    store: &SqlCipherStore,
    edge: &TieEdge,
) -> Result<Vec<SoulEvidence>, GraphError> {
    soul_graph::resolve_evidence(store, edge)
}

/// The evidence behind the edge with this id.
pub fn evidence_for(
    store: &SqlCipherStore,
    relationship_id: Uuid,
) -> Result<Vec<SoulEvidence>, GraphError> {
    let graph = load(store)?;
    match graph.edge(relationship_id) {
        Some(edge) => edge_evidence(store, edge),
        None => Ok(Vec::new()),
    }
}
