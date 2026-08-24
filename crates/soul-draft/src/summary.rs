//! The counting side of a people summary.
//!
//! AC-16 asks that every reading a summary offers can be traced to something
//! stored, so a claim here is a sentence plus the evidence ids the edge cites,
//! and the ids are copied from the edge — never from a model's answer, which
//! is text and not a set of facts.
//!
//! The numbers are tallies: how many exchanges, in which direction, over how
//! many days and conversations. `soul-graph` chose counts over a rating for
//! the same reason DECISIONS D22 rules a scale out of the trait axes, and a
//! summary that turned those tallies into a number would be reintroducing the
//! thing both of them refused.
//!
//! Nothing in this module reads the store. The caller resolves the evidence
//! and hands the rows in, which is what makes the second net in
//! [`summarize`] worth having: it compares the rows it was given with the ids
//! the edge cites, so a caller that quietly dropped an unresolvable one is
//! caught here rather than believed.

use uuid::Uuid;

use soul_graph::model::{TieEdge, TieType};
use soul_policy::clinical::{assert_non_clinical, WORKING_HYPOTHESIS_NOTICE};
use soul_schema::common::SupportedBand;
use soul_schema::evidence::SoulEvidence;

use crate::error::{DraftError, DraftResult};

/// One edge with the rows it rests on, already resolved by the caller.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTie<'a> {
    pub edge: &'a TieEdge,
    pub evidence: &'a [SoulEvidence],
}

impl<'a> ResolvedTie<'a> {
    pub fn new(edge: &'a TieEdge, evidence: &'a [SoulEvidence]) -> ResolvedTie<'a> {
        ResolvedTie { edge, evidence }
    }
}

/// One reading, and what supports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummaryClaim {
    reading: String,
    band: SupportedBand,
    relationship_id: Uuid,
    evidence_ids: Vec<Uuid>,
}

impl SummaryClaim {
    pub fn reading(&self) -> &str {
        &self.reading
    }

    /// Weak, moderate or strong. There is no number, and no method that
    /// returns one.
    pub fn band(&self) -> SupportedBand {
        self.band
    }

    pub fn relationship_id(&self) -> Uuid {
        self.relationship_id
    }

    /// Non-empty by construction, and every id came from the edge.
    pub fn evidence_ids(&self) -> &[Uuid] {
        &self.evidence_ids
    }
}

/// What is supported about one person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeopleSummary {
    contact_id: Uuid,
    claims: Vec<SummaryClaim>,
}

impl PeopleSummary {
    pub fn contact_id(&self) -> Uuid {
        self.contact_id
    }

    pub fn claims(&self) -> &[SummaryClaim] {
        &self.claims
    }

    /// The summary in prose, with the working-hypothesis notice last.
    ///
    /// The notice is `soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE` and
    /// the check is `assert_non_clinical`: a summary that trips it does not
    /// get rendered with the word removed.
    pub fn render(&self) -> DraftResult<String> {
        let mut lines: Vec<&str> = self
            .claims
            .iter()
            .map(|claim| claim.reading.as_str())
            .collect();
        lines.push(WORKING_HYPOTHESIS_NOTICE);
        let text = lines.join("\n");
        assert_non_clinical(&text)?;
        Ok(text)
    }
}

/// Turn resolved ties into claims.
///
/// Three refusals, all of them before anything is rendered: a contact with no
/// observed tie, an edge that cites no evidence, and an edge whose cited ids
/// are not the rows that came back.
pub fn summarize(contact_id: Uuid, ties: &[ResolvedTie<'_>]) -> DraftResult<PeopleSummary> {
    if ties.is_empty() {
        return Err(DraftError::NothingObserved { contact_id });
    }

    let mut claims = Vec::with_capacity(ties.len());
    for tie in ties {
        let relationship_id = tie.edge.relationship_id;
        if tie.edge.evidence_ids.is_empty() {
            return Err(DraftError::ClaimWithoutEvidence { relationship_id });
        }
        for evidence_id in &tie.edge.evidence_ids {
            let resolved = tie
                .evidence
                .iter()
                .any(|row| row.evidence_id == *evidence_id);
            if !resolved {
                return Err(DraftError::DanglingEvidence {
                    relationship_id,
                    evidence_id: *evidence_id,
                });
            }
        }

        let reading = reading_for(tie.edge, tie.evidence.len());
        assert_non_clinical(&reading)?;
        claims.push(SummaryClaim {
            reading,
            band: tie.edge.tie_strength.band,
            relationship_id,
            evidence_ids: tie.edge.evidence_ids.clone(),
        });
    }

    Ok(PeopleSummary { contact_id, claims })
}

/// Weak, moderate, strong — the same three words the profile view uses.
pub const fn band_word(band: SupportedBand) -> &'static str {
    match band {
        SupportedBand::Weak => "弱",
        SupportedBand::Moderate => "中",
        SupportedBand::Strong => "强",
    }
}

/// What shape the exchanges took, as observed.
pub const fn tie_word(tie: TieType) -> &'static str {
    match tie {
        TieType::Direct => "一对一说过话",
        TieType::GroupOnly => "只在群里遇到",
        TieType::Reciprocal => "两边都写过",
        TieType::OneSided => "只有一边写过",
    }
}

fn shape_of(edge: &TieEdge) -> String {
    match edge.types.is_empty() {
        true => "有过往来".to_owned(),
        false => edge
            .types
            .iter()
            .map(|tie| tie_word(*tie))
            .collect::<Vec<_>>()
            .join("、"),
    }
}

/// One sentence of counts. Every number in it is a tally the user could get to
/// by counting messages themselves.
fn reading_for(edge: &TieEdge, evidence_rows: usize) -> String {
    let strength = &edge.tie_strength;
    format!(
        "{}：往来 {} 次（你发出 {} 次，对方发来 {} 次），分布在 {} 个会话、{} 天，最近一次在 {}；支持强度{}，由 {} 条本机证据支持。",
        shape_of(edge),
        strength.interaction_count,
        strength.outgoing_count,
        strength.incoming_count,
        strength.conversation_count,
        strength.active_day_count,
        strength.last_contact_utc.as_str(),
        band_word(strength.band),
        evidence_rows,
    )
}
