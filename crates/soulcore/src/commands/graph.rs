//! WP05's command surface: rebuild the people graph, read it back, and resolve
//! what an edge rests on.
//!
//! Thin, like the rest of this module. `soul-graph` decides what an edge is and
//! `soul-store` holds it; what is here is the wiring, plus the one thing
//! neither of them can do alone — appending the audit entry the rebuild owes
//! the chain, which needs the open store and the caller's clock.
//!
//! [`PeopleGraphView`] is the same kind of value `fileplan.rs` and `draft.rs`
//! hand the shell: counts and identifiers, no prose. A node carries no name —
//! `PersonNode::label_ref` is a pointer into sealed text and nothing here
//! opens it — so what the interface can draw is who was talked to, how often,
//! and which evidence rows say so.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_graph::model::{PersonNode, SoulGraph, TieEdge};
use soul_graph::{GraphBuild, GraphError, TieCorrection};
use soul_policy::audit::append_or_store_error;
use soul_schema::common::{Sha256Hex, SupportedBand};
use soul_schema::evidence::SoulEvidence;
use soul_schema::memory::ForgetState;
use soul_store::SqlCipherStore;

/// 工作假设，非临床结论. Re-exported rather than copied so the desktop shell and
/// its tests read the same constant `soul-policy`'s denylist tests hold.
pub use soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE;

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

/// Fix the band on one tie because the user says so, and hold it there.
///
/// The graph half of AC-07's instinct: a working hypothesis the user has ruled
/// on is theirs, and a later rebuild recomputes the counts without moving the
/// band. `at_unix_seconds` is the caller's clock, so a replay writes the same
/// audit entry.
pub fn correct_tie(
    store: &mut SqlCipherStore,
    relationship_id: Uuid,
    band: SupportedBand,
    at_unix_seconds: i64,
) -> Result<TieCorrection, GraphError> {
    soul_graph::correct_tie(store, relationship_id, band, at_unix_seconds)
}

/// Hand the band back to the counts.
pub fn release_tie(
    store: &mut SqlCipherStore,
    relationship_id: Uuid,
    at_unix_seconds: i64,
) -> Result<TieCorrection, GraphError> {
    soul_graph::release_tie(store, relationship_id, at_unix_seconds)
}

/// The band a vocabulary word names, and nothing else.
///
/// A closed set, the way `profile::position_named` is: three words in, three
/// bands out, and anything else is not a band this product has. The inverse of
/// [`band_word`], so the shell can round-trip what a view handed it.
pub fn band_named(key: &str) -> Option<SupportedBand> {
    match key {
        "weak" => Some(SupportedBand::Weak),
        "moderate" => Some(SupportedBand::Moderate),
        "strong" => Some(SupportedBand::Strong),
        _ => None,
    }
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

/// The graph as the desktop shell may draw it.
///
/// Every edge arrives with its evidence already resolved, so a tie the
/// interface shows is a tie something in the store still supports. AC-06 says
/// an inference has to dereference; [`people_view`] fails rather than handing
/// back an edge whose rows have gone.
pub fn people_view(store: &SqlCipherStore) -> Result<PeopleGraphView, GraphError> {
    let graph = load(store)?;
    let mut ties = Vec::with_capacity(graph.edges.len());
    for edge in &graph.edges {
        ties.push(TieEdgeView::of(edge, &edge_evidence(store, edge)?));
    }
    Ok(PeopleGraphView::of(&graph, ties))
}

/// What the shell may display about the whole graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeopleGraphView {
    pub self_contact_id: Option<String>,
    pub people: Vec<PersonNodeView>,
    pub ties: Vec<TieEdgeView>,
    /// `工作假设，非临床结论`, from `soul-policy`.
    pub notice: String,
    /// Every node that is not the user, and every edge, stays on this machine.
    pub third_party_data_is_local_only: bool,
}

impl PeopleGraphView {
    fn of(graph: &SoulGraph, ties: Vec<TieEdgeView>) -> PeopleGraphView {
        PeopleGraphView {
            self_contact_id: graph.self_contact_id.map(|id| id.to_string()),
            people: graph
                .nodes
                .iter()
                .map(|node| PersonNodeView::of(node, graph.self_contact_id))
                .collect(),
            ties,
            notice: WORKING_HYPOTHESIS_NOTICE.to_owned(),
            third_party_data_is_local_only: graph.third_party_data_is_local_only(),
        }
    }
}

/// One person, with nothing on it that could be read as a name.
///
/// `identifier_hint` is the leading characters of an identifier digest. It
/// exists so two people can be told apart on screen without the interface
/// having to open a seal to do it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonNodeView {
    pub contact_id: String,
    pub is_you: bool,
    pub identifier_hint: String,
    pub interaction_count: u64,
    pub last_contact_utc: Option<String>,
    pub tie_count: usize,
    /// True once the rows behind this person have been forgotten.
    pub forgotten: bool,
}

/// How many characters of a digest are enough to tell two people apart on
/// screen, and few enough that it reads as an identifier rather than a name.
const IDENTIFIER_HINT_LEN: usize = 8;

impl PersonNodeView {
    fn of(node: &PersonNode, self_contact_id: Option<Uuid>) -> PersonNodeView {
        PersonNodeView {
            contact_id: node.contact_id.to_string(),
            is_you: Some(node.contact_id) == self_contact_id,
            identifier_hint: identifier_hint(&node.identifier_hashes, node.contact_id),
            interaction_count: node.interaction_count,
            last_contact_utc: node
                .last_contact_utc
                .as_ref()
                .map(|at| at.as_str().to_owned()),
            tie_count: node.edge_ids.len(),
            forgotten: node.forget_state == ForgetState::Forgotten,
        }
    }
}

fn identifier_hint(hashes: &[Sha256Hex], contact_id: Uuid) -> String {
    let source = match hashes.first() {
        Some(hash) => hash.as_str().to_owned(),
        None => contact_id.to_string(),
    };
    source.chars().take(IDENTIFIER_HINT_LEN).collect()
}

/// One tie, with the counts it was derived from and the rows behind it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TieEdgeView {
    pub relationship_id: String,
    pub from_contact_id: String,
    pub to_contact_id: String,
    /// Observed shapes — `direct`, `group_only`, `reciprocal`, `one_sided` —
    /// never a claim about what the relationship is.
    pub types: Vec<String>,
    /// The band in force: the user's own on a corrected edge, the machine's
    /// otherwise. Every reader that only wants "how close are these two" can go
    /// on reading this field and will respect a correction without knowing one
    /// happened.
    pub band: String,
    /// True once the user has ruled on this edge.
    pub locked_by_user: bool,
    /// The band the user chose, present exactly when `locked_by_user`.
    pub user_band: Option<String>,
    /// What the counts say, kept beside the effective band so a corrected edge
    /// can show both. The interface composes the sentence; this is the token.
    pub machine_band: Option<String>,
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    pub conversation_count: u64,
    pub active_day_count: u64,
    pub first_contact_utc: String,
    pub last_contact_utc: String,
    pub local_only: bool,
    /// Never empty: an edge nobody observed is not an edge.
    pub evidence: Vec<EvidenceRowView>,
}

impl TieEdgeView {
    fn of(edge: &TieEdge, evidence: &[SoulEvidence]) -> TieEdgeView {
        TieEdgeView {
            relationship_id: edge.relationship_id.to_string(),
            from_contact_id: edge.from_contact_id.to_string(),
            to_contact_id: edge.to_contact_id.to_string(),
            types: edge
                .types
                .iter()
                .map(|kind| kind.as_str().to_owned())
                .collect(),
            band: band_word(edge.tie_strength.band).to_owned(),
            locked_by_user: edge.tie_strength.is_locked_by_user(),
            user_band: edge
                .tie_strength
                .user_band
                .map(|band| band_word(band).to_owned()),
            machine_band: edge
                .tie_strength
                .machine_band
                .map(|band| band_word(band).to_owned()),
            interaction_count: edge.tie_strength.interaction_count,
            outgoing_count: edge.tie_strength.outgoing_count,
            incoming_count: edge.tie_strength.incoming_count,
            conversation_count: edge.tie_strength.conversation_count,
            active_day_count: edge.tie_strength.active_day_count,
            first_contact_utc: edge.tie_strength.first_contact_utc.as_str().to_owned(),
            last_contact_utc: edge.tie_strength.last_contact_utc.as_str().to_owned(),
            local_only: edge.egress_scope == soul_schema::relationship::EgressScope::LocalOnly,
            evidence: evidence.iter().map(EvidenceRowView::of).collect(),
        }
    }
}

/// One evidence row, as an identifier and two vocabulary words.
///
/// No `source_refs` and no sealed pointer: what the user is being shown is
/// that the row exists and what kind of observation it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRowView {
    pub evidence_id: String,
    pub kind: String,
    pub method: String,
    pub strength: String,
}

impl EvidenceRowView {
    fn of(evidence: &SoulEvidence) -> EvidenceRowView {
        EvidenceRowView {
            evidence_id: evidence.evidence_id.to_string(),
            kind: vocabulary_word(&evidence.kind),
            method: evidence
                .method
                .as_ref()
                .map(vocabulary_word)
                .unwrap_or_else(|| "unstated".to_owned()),
            strength: band_word(evidence.strength).to_owned(),
        }
    }
}

/// The contract's own spelling of a small enum.
///
/// Serialized rather than matched by hand: these enums are `soul-schema`'s and
/// a second spelling here would be a second vocabulary to keep in step. Every
/// one of them serializes to a string, so the fallback is unreachable; it is
/// an empty word rather than a panic because a view is not a place to abort.
fn vocabulary_word<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The word for an evidence band, spelled once for every view in this crate.
pub fn band_word(band: SupportedBand) -> &'static str {
    match band {
        SupportedBand::Weak => "weak",
        SupportedBand::Moderate => "moderate",
        SupportedBand::Strong => "strong",
    }
}
