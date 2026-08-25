//! The people summary: what Soul is prepared to say about one person, and what
//! it can point at for each sentence.
//!
//! AC-16 asks for two things — every point carries evidence, and nothing
//! carries diagnostic vocabulary — and this module makes both structural
//! rather than procedural:
//!
//! * [`SummaryPoint`] has no public constructor that accepts an empty evidence
//!   list. `SummaryPoint::new` returns [`DraftError::NoEvidence`] instead, so a
//!   point with nothing behind it is not a thing that can exist and then be
//!   filtered out later;
//! * every statement goes through [`assert_non_clinical`] at construction, not
//!   at render time. A summary that would say a forbidden word fails to build.
//!
//! What the points say is counts. `docs/DECISIONS.md` D22 rules out numeric
//! scales for the psychological model and the same instinct applies to people:
//! the summary reports how many exchanges there were, in which direction, over
//! how many days — things the user could verify by counting messages — and the
//! only summary value is [`SupportedBand`], which is three words.
//!
//! *How* they say it is not this module's business. D40 puts the wording in the
//! frozen renderer `soul_algo_trait::a2_render`, which is handed the counts the
//! graph persisted and hands back sentences; [`crate::a2_adapt`] is the whole
//! of the conversion. What is left here is the two things the renderer cannot
//! know: which evidence rows a point may cite, and that a band the user set is
//! not a band the counts explain (GC-9a).
//!
//! What the points never say is a name. A [`PersonNode`](soul_graph::model::PersonNode)
//! has no field holding one; the label is a sealed pointer and the identifiers
//! are hashes. So a summary refers to its subject by `contact_id` and the
//! rendered text says 这个人, which is the correct amount of information for a
//! screen the user is already looking at with the graph open.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_graph::interaction;
use soul_graph::model::{SoulGraph, TieEdge, TieStrength};
use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_policy::redactor::{RedactedBody, Redactor, Turn};
use soul_schema::common::{NotAClinicalClaim, SealedSubject, SupportedBand};
use soul_schema::evidence::SoulEvidence;

use crate::a2_adapt;
use crate::draft::ReplyGenerator;
use crate::error::{DraftError, DraftResult};
use crate::reply;

/// One thing the summary says, and what it rests on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryPoint {
    statement: String,
    evidence_ids: Vec<Uuid>,
    band: SupportedBand,
}

impl SummaryPoint {
    /// Build a point. Refuses an empty evidence list and a clinical statement.
    pub fn new(
        statement: impl Into<String>,
        band: SupportedBand,
        evidence_ids: Vec<Uuid>,
    ) -> DraftResult<SummaryPoint> {
        if evidence_ids.is_empty() {
            return Err(DraftError::NoEvidence);
        }
        let statement = statement.into();
        assert_non_clinical(&statement)?;
        Ok(SummaryPoint {
            statement,
            evidence_ids,
            band,
        })
    }

    pub fn statement(&self) -> &str {
        &self.statement
    }

    /// Never empty; see [`SummaryPoint::new`].
    pub fn evidence_ids(&self) -> &[Uuid] {
        &self.evidence_ids
    }

    pub fn band(&self) -> SupportedBand {
        self.band
    }
}

/// Where the wording came from. The points and their evidence are the same
/// either way — only the optional narrative changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummarySource {
    /// Counts, computed on this machine. The no-key path.
    Counts,
    /// The counts, rephrased by the endpoint the user configured.
    UserEndpoint,
}

/// Everything Soul will say about one person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonSummary {
    pub contact_id: Uuid,
    pub relationship_id: Uuid,
    /// Non-empty, and each one carries evidence.
    pub points: Vec<SummaryPoint>,
    pub source: SummarySource,
    /// A rephrasing from the user's own endpoint, if there is one and it
    /// passed the same checks the points did. The points stand on their own
    /// without it.
    pub narrative: Option<String>,
    /// `工作假设，非临床结论`.
    pub notice: String,
    /// Serializes to `false`, always.
    pub clinical_claim: NotAClinicalClaim,
}

impl PersonSummary {
    /// Every evidence row the whole summary rests on, deduplicated.
    pub fn evidence_ids(&self) -> Vec<Uuid> {
        let unique: BTreeSet<Uuid> = self
            .points
            .iter()
            .flat_map(|point| point.evidence_ids.iter().copied())
            .collect();
        unique.into_iter().collect()
    }
}

/// Summarize one person from the graph and the evidence behind their edge.
///
/// `resolved` is the evidence the caller fetched for the edge — `soul-graph`'s
/// `resolve_evidence` is the function that produces it, and it fails rather
/// than returning a short list. This checks that what arrived is what the edge
/// cites, so a caller cannot hand over three rows for a five-row edge and get
/// a summary that quietly claims more support than it has.
pub fn summarize_person(
    graph: &SoulGraph,
    contact_id: Uuid,
    resolved: &[SoulEvidence],
) -> DraftResult<PersonSummary> {
    let edge = graph
        .edges_for(contact_id)
        .into_iter()
        .max_by_key(|edge| edge.tie_strength.interaction_count)
        .ok_or(DraftError::NoSuchTie { contact_id })?;

    let present: BTreeSet<Uuid> = resolved.iter().map(|row| row.evidence_id).collect();
    for cited in &edge.evidence_ids {
        if !present.contains(cited) {
            return Err(DraftError::UnresolvedEvidence {
                evidence_id: *cited,
            });
        }
    }

    let points = points_for(edge, resolved)?;
    Ok(PersonSummary {
        contact_id,
        relationship_id: edge.relationship_id,
        points,
        source: SummarySource::Counts,
        narrative: None,
        notice: WORKING_HYPOTHESIS_NOTICE.to_owned(),
        clinical_claim: NotAClinicalClaim,
    })
}

/// The summary in words.
///
/// Every line names how many rows are behind it, because a claim and its
/// support belong on the same line — a footnote nobody scrolls to is not
/// evidence the user can check.
pub fn render(summary: &PersonSummary) -> DraftResult<String> {
    let mut lines = vec!["关于这个人，本机能说的只有下面这些：".to_owned()];
    for point in &summary.points {
        lines.push(format!(
            "· {}（依据 {} 条记录）",
            point.statement,
            point.evidence_ids.len(),
        ));
    }
    if let Some(narrative) = summary.narrative.as_deref() {
        lines.push(format!("整体来看：{narrative}"));
    }
    lines.push(summary.notice.clone());

    let rendered = lines.join("\n");
    assert_non_clinical(&rendered)?;
    Ok(rendered)
}

/// Ask the user's own endpoint to rephrase the summary.
///
/// The points and their evidence are not up for negotiation: only
/// [`PersonSummary::narrative`] changes, and only if what comes back passes
/// the same non-clinical check the points passed. A rephrasing that trips it
/// is dropped and the summary stays exactly as it was, which is why "every
/// point has evidence" survives a model that ignored the request entirely.
///
/// What goes on the wire is the statements, not the conversation: they are
/// Soul's own derived counts, they contain no third-party prose and no
/// identifiers, and they still go through the redactor on the way out.
pub fn phrase_with<G: ReplyGenerator>(
    summary: &PersonSummary,
    redactor: &Redactor,
    generator: &mut G,
) -> DraftResult<PersonSummary> {
    let raw = generator.generate(summary_body(summary, redactor))?;

    let narrative = reply::read(&raw).ok().map(|answer| answer.text);
    Ok(PersonSummary {
        source: match narrative.is_some() {
            true => SummarySource::UserEndpoint,
            false => summary.source,
        },
        narrative,
        ..summary.clone()
    })
}

/// The request body for [`phrase_with`], built the same way a draft's is.
pub fn summary_body(summary: &PersonSummary, redactor: &Redactor) -> RedactedBody {
    let mut turns = Vec::with_capacity(summary.points.len() + 1);
    turns.push(Turn::new(
        SUMMARY_TURN_ID,
        SealedSubject::Owner,
        "【本机统计，供改写参考。下面每一条都是计数，不是判断。】",
    ));
    for (index, point) in summary.points.iter().enumerate() {
        turns.push(Turn::new(
            Uuid::from_u128(SUMMARY_TURN_ID.as_u128() + 1 + index as u128),
            SealedSubject::Owner,
            point.statement.clone(),
        ));
    }
    redactor.redact_for_e1(&turns)
}

/// The turn id the statistical header travels under; the points follow it.
const SUMMARY_TURN_ID: Uuid = Uuid::from_u128(0x0192b0c0_5020_7a20_8b20_000000000020);

// ------------------------------------------------------------- internals ---

/// The things counts can support, each citing the rows that produced it.
///
/// The sentences are the frozen renderer's, one point per bullet. The band on
/// every point is the edge's, because that is the band the renderer was handed
/// and a per-point adjustment here would be a second opinion.
///
/// The one bullet that does not survive the trip is the filing sentence on an
/// edge the user has corrected. It says two things that stop being true the
/// moment the user has ruled: that the band follows from the counts above it,
/// and that it is the machine's reading of the record. On a corrected edge the
/// band came from the user and is a verdict, so the sentence would be false
/// twice over. Rewording it is not this crate's to do —
/// `docs/algorithms/COPY_ZH.md` is frozen and holds no variant for a user-set
/// band, and inventing one here would be a second source of user-facing copy
/// beside the frozen one. So the summary says nothing about filing until
/// COPY_ZH gains the key. Nothing is hidden by the silence: the band, who set
/// it and what the machine makes of the counts all reach the interface through
/// the graph view, which carries the three lock fields as tokens.
fn points_for(edge: &TieEdge, resolved: &[SoulEvidence]) -> DraftResult<Vec<SummaryPoint>> {
    let strength = &edge.tie_strength;
    let band = strength.band;
    let counted = counted_rows(edge, resolved);
    let last_contact = last_contact_row(strength, resolved, &counted);

    let mut points = Vec::new();
    for bullet in a2_adapt::bullets_for(edge, &counted, last_contact) {
        if strength.is_locked_by_user() && bullet.statement_key == a2_adapt::FILED_BAND_KEY {
            continue;
        }
        points.push(SummaryPoint::new(
            bullet.text_zh,
            band,
            bullet.evidence_ids,
        )?);
    }
    Ok(points)
}

/// The rows a count rests on: everything the edge cites, minus the user's own
/// corrections.
///
/// A correction row is a verdict about the band, not an exchange anybody had,
/// so counting it would make "六次往来（依据七条记录）" — a line whose own
/// arithmetic does not add up, and the summary's whole claim is that the user
/// can check it.
///
/// The fallback matters on an edge whose observations have all been forgotten
/// and whose correction is the only row left: cite what is actually there
/// rather than refuse to say anything at all.
fn counted_rows(edge: &TieEdge, resolved: &[SoulEvidence]) -> Vec<Uuid> {
    let counted: Vec<Uuid> = edge
        .evidence_ids
        .iter()
        .copied()
        .filter(|cited| {
            resolved
                .iter()
                .find(|row| row.evidence_id == *cited)
                .is_none_or(|row| soul_graph::corrected_relationship(row).is_none())
        })
        .collect();
    match counted.is_empty() {
        true => edge.evidence_ids.clone(),
        false => counted,
    }
}

/// The single row holding the most recent exchange, if it is one the summary
/// may cite.
///
/// One row, not all of them: "the last contact was on this date" is supported
/// by the observation that happened on that date and by nothing else, and a
/// recency point that cited the whole edge would be padding its own support.
/// A row outside `counted` is not offered — the renderer cites what it is
/// given, and what it may be given is what the counts rest on.
fn last_contact_row(
    strength: &TieStrength,
    resolved: &[SoulEvidence],
    counted: &[Uuid],
) -> Option<Uuid> {
    let last = strength.last_contact_utc.as_str();
    resolved
        .iter()
        .find(|row| {
            counted.contains(&row.evidence_id)
                && interaction::interactions_in(row)
                    .iter()
                    .any(|observation| observation.occurred_at.as_str() == last)
        })
        .map(|row| row.evidence_id)
}
