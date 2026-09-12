//! Correcting a tie, and the way back out of it.
//!
//! PRODUCT_LOCK puts the people graph among the things the user *can see and
//! correct*, and non-negotiable constraint 10 rules out an uncorrectable black
//! box. This module is the graph's half of that promise, written to be read
//! beside `soul_profile::correct_axis`: a `UserCorrection` evidence row, then
//! the state, then the lock, then the audit entry — four steps in the same
//! order, so a reviewer can hold the two side by side.
//!
//! What a correction fixes is the **band**, never the counts. The numbers on an
//! edge are observations the user could verify by counting messages, and they
//! go on accumulating after a correction exactly as before; what the user has
//! overruled is the one summary word derived from them. That is also why the
//! machine's own reading is kept rather than dropped: `machine_band` records
//! what the frozen rule made of the same counts, so the interface can show both
//! without asking the scorer to run again.
//!
//! Nothing here produces a sentence. `docs/algorithms/COPY_ZH.md` is frozen and
//! holds no wording for a user-set band, so a receipt from this module is
//! identifiers and vocabulary tokens; whoever draws the screen composes it from
//! copy that is already frozen.

use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use soul_policy::audit::{append_or_store_error, AuditContent, ReasonCode};
use soul_schema::audit::AuditAction;
use soul_schema::common::{Privacy, Purpose, SchemaVersion, Subject, SupportedBand};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_schema::inference::UserVerdict;
use soul_schema::memory::ForgetState;
use soul_schema::relationship::SoulRelationship;
use soul_store_api::{AuditLog, GraphStore, ProfileStore};

use crate::build::is_tie_statement_about;
use crate::error::{GraphError, GraphResult};
use crate::model::TieStrength;
use crate::view::read_strength;

/// What a correction is worth as evidence. The user is looking at the claim and
/// rejecting it, which is the strongest signal this product can get — the same
/// grade `soul_profile::CORRECTION_STRENGTH` gives the axis side. It is spelled
/// again here rather than imported because the graph does not depend on the
/// profile, and it is a constant of the product rather than of either crate.
pub const CORRECTION_STRENGTH: SupportedBand = SupportedBand::Strong;

/// `source_refs[0].origin` on the row a correction writes.
pub const CORRECTION_ORIGIN: &str = "graph_correction";

/// `source_refs[0].origin` on the row a release writes. Shares the prefix, so
/// one predicate finds both.
pub const RELEASE_ORIGIN: &str = "graph_correction_release";

/// What one correction or release did.
///
/// Identifiers and two vocabulary tokens. `band` is what the edge says now and
/// `machine_band` is what the counts say, which is the whole of what an
/// interface needs to show the two side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TieCorrection {
    pub relationship_id: Uuid,
    /// The `UserCorrection` row this call wrote. It resolves, and the edge
    /// cites it.
    pub evidence_id: Uuid,
    /// The band in force after the call. Equal to `machine_band` after a
    /// release.
    pub band: SupportedBand,
    /// What the frozen rule made of the counts, kept for display.
    pub machine_band: SupportedBand,
}

impl TieCorrection {
    /// Whether the user and the machine now disagree about this edge.
    pub fn overrides_machine(&self) -> bool {
        self.band != self.machine_band
    }
}

/// Fix the band on one tie, and hold it there.
///
/// The edge is read first: a `relationship_id` nobody has an edge for fails
/// with the store's own `NotFound` and leaves the store untouched, rather than
/// writing an evidence row for a correction to nothing. v0.1 has no way to
/// invent an edge — an edge nothing was observed for would be the one thing the
/// evidence rule forbids — so this refuses instead of creating one.
///
/// Correcting an edge that is already corrected is allowed and simply records
/// the newer verdict.
pub fn correct_tie<S>(
    store: &mut S,
    relationship_id: Uuid,
    band: SupportedBand,
    at_unix_seconds: i64,
) -> GraphResult<TieCorrection>
where
    S: GraphStore + ProfileStore + AuditLog,
{
    let stored = store.get_relationship(relationship_id)?;
    refuse_a_forgotten_peer(store, &stored)?;
    let mut strength = read_strength(&stored)?;
    let machine_band = machine_reading(&strength);

    let evidence_id = Uuid::now_v7();
    store.put_evidence(correction_evidence(
        evidence_id,
        relationship_id,
        band,
        CORRECTION_ORIGIN,
    ))?;

    strength.band = band;
    strength.locked_by_user = Some(true);
    strength.user_band = Some(band);
    strength.machine_band = Some(machine_band);
    store.put_relationship(rewritten(stored, &strength, evidence_id)?)?;

    set_verdict(store, relationship_id, UserVerdict::Corrected)?;
    record(store, relationship_id, evidence_id, at_unix_seconds)?;

    Ok(TieCorrection {
        relationship_id,
        evidence_id,
        band,
        machine_band,
    })
}

/// Hand the band back to the counts.
///
/// Takes effect immediately rather than at the next rebuild, for the same
/// reason [`correct_tie`] does: the user is looking at the edge when they ask,
/// and a lock with no visible way out is a lock the support burden falls on.
///
/// Releasing an edge nobody locked leaves the band where it already was, and is
/// still recorded — the user asked for the counts to speak, and after the call
/// they do.
pub fn release_tie<S>(
    store: &mut S,
    relationship_id: Uuid,
    at_unix_seconds: i64,
) -> GraphResult<TieCorrection>
where
    S: GraphStore + ProfileStore + AuditLog,
{
    let stored = store.get_relationship(relationship_id)?;
    refuse_a_forgotten_peer(store, &stored)?;
    let mut strength = read_strength(&stored)?;
    let machine_band = machine_reading(&strength);

    let evidence_id = Uuid::now_v7();
    store.put_evidence(correction_evidence(
        evidence_id,
        relationship_id,
        machine_band,
        RELEASE_ORIGIN,
    ))?;

    strength.band = machine_band;
    strength.locked_by_user = None;
    strength.user_band = None;
    strength.machine_band = None;
    store.put_relationship(rewritten(stored, &strength, evidence_id)?)?;

    set_verdict(store, relationship_id, UserVerdict::Unreviewed)?;
    record(store, relationship_id, evidence_id, at_unix_seconds)?;

    Ok(TieCorrection {
        relationship_id,
        evidence_id,
        band: machine_band,
        machine_band,
    })
}

/// Which edge an evidence row is a correction of, if it is one at all.
///
/// A correction row is evidence like any other and has to survive a rebuild
/// beside the observations — an edge that cited only the messages would drop
/// the one row explaining why its band is what it is. This is the predicate
/// that finds them in a pass over the evidence table.
pub fn corrected_relationship(evidence: &SoulEvidence) -> Option<Uuid> {
    if evidence.kind != EvidenceKind::UserCorrection {
        return None;
    }
    let reference = evidence.source_refs.first()?;
    let origin = reference.get("origin")?.as_str()?;
    if !origin.starts_with(CORRECTION_ORIGIN) {
        return None;
    }
    reference
        .get("relationship_id")?
        .as_str()?
        .parse::<Uuid>()
        .ok()
}

// ------------------------------------------------------------- internals ---

/// Refuse to move the band on an edge either end of which has been forgotten.
///
/// A forget tombstones the contact row, destroys the keys their words were
/// sealed under and demotes the inferences resting on their evidence to
/// `orphaned` — and deliberately leaves the relationship row and the evidence
/// ids alone, because `resolve_evidence` fails rather than returning a short
/// list and the graph view runs it over every edge. `rebuild` then skips the
/// tombstoned peer, so their edge stays exactly as the last live rebuild wrote
/// it: on screen, with the three band words under it.
///
/// Pressing one of them wrote through. Nothing on this path read the peer's
/// [`ForgetState`]: [`set_verdict`] finds the tie inference with
/// `list_inferences`, which does not filter on state, and `put_inference`
/// files whatever it is handed as live — so a correction on a tombstone's
/// stale tie put the forgotten person's inference back and added a
/// `UserCorrection` row about them. Refusing here is the whole fix; the row
/// itself stays where the forget left it.
///
/// Both ends are checked rather than "the peer", because an edge is a pair of
/// contact ids and nothing in the contract says which of them is the owner.
fn refuse_a_forgotten_peer<S>(store: &S, stored: &SoulRelationship) -> GraphResult<()>
where
    S: GraphStore,
{
    for contact_id in [stored.from_contact_id, stored.to_contact_id] {
        if store.get_contact(contact_id)?.forget_state != ForgetState::Active {
            return Err(GraphError::Forgotten {
                relationship_id: stored.relationship_id,
            });
        }
    }
    Ok(())
}

/// What the counts say, whether or not the user has overruled it.
///
/// An edge written before the lock fields existed carries no `machine_band`,
/// and on such an edge `band` *is* the machine's reading — there was nothing
/// else it could have been.
fn machine_reading(strength: &TieStrength) -> SupportedBand {
    strength.machine_band.unwrap_or(strength.band)
}

/// The edge with the new strength on it, citing the row that changed it.
fn rewritten(
    stored: SoulRelationship,
    strength: &TieStrength,
    evidence_id: Uuid,
) -> GraphResult<SoulRelationship> {
    let mut evidence_ids = stored.evidence_ids;
    if !evidence_ids.contains(&evidence_id) {
        evidence_ids.push(evidence_id);
    }
    Ok(SoulRelationship {
        tie_strength: Some(serde_json::to_value(strength).map_err(|_| {
            crate::error::GraphError::UnreadableEdge {
                relationship_id: stored.relationship_id,
                field: "tie_strength",
            }
        })?),
        evidence_ids,
        ..stored
    })
}

/// One evidence row saying the user ruled on one edge.
///
/// Identifiers and a band word. No name, no message body, nothing a redactor
/// would have to work on — which is why it can be `Subject::Owner`: the fact
/// recorded is the user's own verdict, not anything about the other person.
fn correction_evidence(
    evidence_id: Uuid,
    relationship_id: Uuid,
    band: SupportedBand,
    origin: &str,
) -> SoulEvidence {
    SoulEvidence {
        schema_version: SchemaVersion,
        evidence_id,
        kind: EvidenceKind::UserCorrection,
        subject: Subject::Owner,
        source_refs: vec![json!({
            "origin": origin,
            "relationship_id": relationship_id,
            "band": band,
        })],
        strength: CORRECTION_STRENGTH,
        method: Some(EvidenceMethod::UserStated),
        exportable_to_research: Some(false),
        privacy: Some(Privacy::local_only(Subject::Owner, vec![Purpose::Graph])),
    }
}

/// Record the user's verdict on the tie inference behind this edge.
///
/// `statement_key`, `evidence_band` and `evidence_ids` are left alone: they are
/// the machine's assertion and what supports it, and the user is entitled to
/// see that the machine still thinks otherwise. Stored and not applied, the
/// same shape the axis side gives a refused inference.
///
/// An edge with no tie inference behind it — one written by hand, or one whose
/// rebuild has not run — is left as it is rather than given an inference this
/// module did not derive.
fn set_verdict<S>(store: &mut S, relationship_id: Uuid, verdict: UserVerdict) -> GraphResult<()>
where
    S: ProfileStore,
{
    let Some(mut inference) = store
        .list_inferences()?
        .into_iter()
        .find(|inference| is_tie_statement_about(inference, relationship_id))
    else {
        return Ok(());
    };
    inference.user_verdict = Some(verdict);
    store.put_inference(inference)?;
    Ok(())
}

/// The chain entry a correction owes.
///
/// `ProfileCorrect` rather than a graph-specific action: the audit vocabulary
/// is frozen, the semantics hold — the user corrected a working hypothesis the
/// soul layer was holding — and widening the enum is a contract change nobody
/// needs for this. `about` names the edge and the row, the way the axis side
/// names the profile, the axis and the row.
fn record<S: AuditLog>(
    store: &mut S,
    relationship_id: Uuid,
    evidence_id: Uuid,
    at_unix_seconds: i64,
) -> GraphResult<()> {
    append_or_store_error(
        store,
        AuditContent::allowed(AuditAction::ProfileCorrect, ReasonCode::Routine)
            .about(&[relationship_id, evidence_id]),
        at_unix_seconds,
    )?;
    Ok(())
}
