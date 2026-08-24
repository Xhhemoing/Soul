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
//! What the points never say is a name. A [`PersonNode`](soul_graph::model::PersonNode)
//! has no field holding one; the label is a sealed pointer and the identifiers
//! are hashes. So a summary refers to its subject by `contact_id` and the
//! rendered text says 这个人, which is the correct amount of information for a
//! screen the user is already looking at with the graph open.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_graph::interaction::{self, InteractionRef, Venue};
use soul_graph::model::{SoulGraph, TieEdge, TieStrength, TieType};
use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_policy::redactor::{RedactedBody, Redactor, Turn};
use soul_schema::common::{NotAClinicalClaim, SealedSubject, SupportedBand};
use soul_schema::evidence::SoulEvidence;

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

/// Five things counts can support, each citing the rows that produced it.
fn points_for(edge: &TieEdge, resolved: &[SoulEvidence]) -> DraftResult<Vec<SummaryPoint>> {
    let strength = &edge.tie_strength;
    let band = strength.band;
    let all: Vec<Uuid> = edge.evidence_ids.clone();

    let mut points = vec![
        SummaryPoint::new(
            format!(
                "你和这个人一共有 {} 次往来，分布在 {} 个自然日、{} 个会话里。",
                strength.interaction_count, strength.active_day_count, strength.conversation_count,
            ),
            band,
            all.clone(),
        )?,
        SummaryPoint::new(direction_statement(strength), band, all.clone())?,
    ];

    if let Some(point) = venue_point(edge, resolved)? {
        points.push(point);
    }
    if let Some(point) = recency_point(strength, resolved)? {
        points.push(point);
    }

    points.push(SummaryPoint::new(
        format!(
            "按上面的计数，这段往来归在「{}」一档；这是一个工作假设，不是对这个人的判断。",
            band_word(band),
        ),
        band,
        all,
    )?);
    Ok(points)
}

fn direction_statement(strength: &TieStrength) -> String {
    let shape = match (strength.outgoing_count, strength.incoming_count) {
        (0, _) => "到目前为止都是对方在说。",
        (_, 0) => "到目前为止都是你在说。",
        (out, inc) if out > inc * 2 => "多数时候是你先开口。",
        (out, inc) if inc > out * 2 => "多数时候是对方先开口。",
        _ => "两边说得差不多。",
    };
    format!(
        "你发出 {} 条，对方发出 {} 条，{shape}",
        strength.outgoing_count, strength.incoming_count,
    )
}

/// Which rows show a one-to-one exchange, and which only a group one.
fn venue_point(edge: &TieEdge, resolved: &[SoulEvidence]) -> DraftResult<Option<SummaryPoint>> {
    let direct: Vec<Uuid> = rows_where(edge, resolved, |observation| {
        observation.venue == Venue::Direct
    });
    if !direct.is_empty() {
        return SummaryPoint::new(
            "你们有过一对一的交流，不只是在群里碰见。",
            SupportedBand::Weak,
            direct,
        )
        .map(Some);
    }

    let group: Vec<Uuid> = rows_where(edge, resolved, |observation| {
        observation.venue == Venue::Group
    });
    if group.is_empty() {
        return Ok(None);
    }
    if !edge.types.contains(&TieType::GroupOnly) {
        return Ok(None);
    }
    SummaryPoint::new(
        "目前只在群聊里见过对方发言，没有一对一的记录。",
        SupportedBand::Weak,
        group,
    )
    .map(Some)
}

/// The single row holding the most recent exchange.
///
/// One row, not all of them: "the last contact was on this date" is supported
/// by the observation that happened on that date and by nothing else, and a
/// point that cited the whole edge here would be padding its own support.
fn recency_point(
    strength: &TieStrength,
    resolved: &[SoulEvidence],
) -> DraftResult<Option<SummaryPoint>> {
    let last = strength.last_contact_utc.as_str();
    let Some(evidence_id) = resolved
        .iter()
        .find(|row| {
            interaction::interactions_in(row)
                .iter()
                .any(|observation| observation.occurred_at.as_str() == last)
        })
        .map(|row| row.evidence_id)
    else {
        return Ok(None);
    };

    SummaryPoint::new(
        format!("最近一次往来在 {}（UTC）。", utc_date(last)),
        SupportedBand::Weak,
        vec![evidence_id],
    )
    .map(Some)
}

fn rows_where(
    edge: &TieEdge,
    resolved: &[SoulEvidence],
    mut accept: impl FnMut(&InteractionRef) -> bool,
) -> Vec<Uuid> {
    resolved
        .iter()
        .filter(|row| edge.evidence_ids.contains(&row.evidence_id))
        .filter(|row| interaction::interactions_in(row).iter().any(&mut accept))
        .map(|row| row.evidence_id)
        .collect()
}

fn utc_date(timestamp: &str) -> String {
    timestamp.chars().take(10).collect()
}

fn band_word(band: SupportedBand) -> &'static str {
    match band {
        SupportedBand::Weak => "往来不多",
        SupportedBand::Moderate => "往来中等",
        SupportedBand::Strong => "往来密集",
    }
}
