//! Synthetic arithmetic only. Metrics are descriptive and never issue a PASS.
use super::{pairs::ValidatedEvaluation, records::*};
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use std::collections::BTreeMap;

fn strata_rows<S, K, V>(map: &BTreeMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    K: Serialize,
    V: Serialize,
{
    #[derive(Serialize)]
    struct Row<'a, K, V> {
        stratum: &'a K,
        summary: &'a V,
    }
    let mut rows = serializer.serialize_seq(Some(map.len()))?;
    for (stratum, summary) in map {
        rows.serialize_element(&Row { stratum, summary })?;
    }
    rows.end()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Funnel {
    pub assigned: usize,
    pub started: usize,
    pub generated: usize,
    pub rated: usize,
    pub completed: usize,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Fraction {
    pub numerator: usize,
    pub denominator: usize,
}
impl Fraction {
    pub fn value(self) -> Option<f64> {
        (self.denominator != 0).then(|| self.numerator as f64 / self.denominator as f64)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PreferenceStats {
    pub target_wins: usize,
    pub ties: usize,
    pub denominator: usize,
}
impl PreferenceStats {
    pub fn value(self) -> Option<f64> {
        (self.denominator != 0)
            .then(|| (self.target_wins as f64 + 0.5 * self.ties as f64) / self.denominator as f64)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceLayer {
    LocalTemplate,
    UndegradedEndpoint,
    DegradedTemplate,
    NotGenerated,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct PairStratum {
    pub evaluation_run: String,
    pub participant_scope: ParticipantScope,
    pub task_type: TaskType,
    pub semantic_stratum: SemanticStratum,
    pub contrast: Contrast,
    pub conditions: Conditions,
    pub source: SourceLayer,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CandidateStratum {
    pub pair: PairStratum,
    pub variant: Variant,
    pub source: SourceLayer,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CohortStratum {
    pub participant_scope: ParticipantScope,
    pub task_type: TaskType,
    pub semantic_stratum: SemanticStratum,
    pub contrast: Contrast,
    pub conditions: Conditions,
    pub source: SourceLayer,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct ParticipantPreferences {
    pub per_run: BTreeMap<String, f64>,
    pub median: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EditPair {
    pub pair_id: String,
    pub control_seconds: u64,
    pub target_seconds: u64,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct PairSummary {
    pub funnel: Funnel,
    pub degraded_pairs: usize,
    pub safety_excluded_pairs: usize,
    pub preference: PreferenceStats,
    pub edits: Vec<EditPair>,
    pub edit_valid_pairs: usize,
    pub edit_missing_pairs: usize,
    pub median_difference: Option<f64>,
    pub median_ratio: Option<f64>,
}
#[derive(Clone, Debug, Default, Serialize)]
pub struct CandidateSummary {
    pub assigned: usize,
    pub generated: usize,
    pub completed: usize,
    pub first_pass: Fraction,
    pub first_pass_missing: usize,
    pub fact_correct: Fraction,
    pub fact_missing: usize,
    pub safety_stops: Fraction,
    pub safety_missing: usize,
    pub help_counts: Vec<u32>,
    pub missing: BTreeMap<MissingReason, usize>,
    pub edit_missing: BTreeMap<MissingReason, usize>,
    pub pause_missing: BTreeMap<MissingReason, usize>,
    pub failures: BTreeMap<FailureCode, usize>,
}
#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub funnel: Funnel,
    pub withdrawn_runs: usize,
    pub generated_candidates: usize,
    #[serde(serialize_with = "strata_rows")]
    pub pairs: BTreeMap<PairStratum, PairSummary>,
    #[serde(serialize_with = "strata_rows")]
    pub candidates: BTreeMap<CandidateStratum, CandidateSummary>,
    #[serde(serialize_with = "strata_rows")]
    pub participant_preferences: BTreeMap<CohortStratum, ParticipantPreferences>,
}
fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let mid = values.len() / 2;
    Some(if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    })
}

fn candidate_source(candidate: &Candidate) -> SourceLayer {
    match (candidate.draft_source, candidate.degraded_reason) {
        (DraftSource::None, _) => SourceLayer::NotGenerated,
        (DraftSource::UserEndpoint, _) => SourceLayer::UndegradedEndpoint,
        (DraftSource::ToneTemplate, DegradedReason::None) => SourceLayer::LocalTemplate,
        (DraftSource::ToneTemplate, _) => SourceLayer::DegradedTemplate,
    }
}

fn add_funnel(funnel: &mut Funnel, pair: &Pair, candidates: &[&Candidate]) {
    funnel.assigned += 1;
    funnel.started += usize::from(pair.pair_started_at.is_some());
    funnel.generated += usize::from(candidates.iter().all(|candidate| candidate.generated));
    // Historical process, independent from permission to use utility scores now.
    funnel.rated += usize::from(
        pair.preference != Preference::Unrated
            && pair.blind_status == BlindStatus::Intact
            && pair.comparability_status == Comparability::Comparable,
    );
    funnel.completed += usize::from(pair.pair_completion_status == PairCompletion::Completed);
}

pub fn summarize(validated: &ValidatedEvaluation<'_>) -> Summary {
    let data = validated.data;
    let mut report = Summary {
        withdrawn_runs: data
            .runs
            .iter()
            .filter(|run| run.consent_state == ConsentState::Withdrawn)
            .count(),
        generated_candidates: data
            .candidates
            .iter()
            .filter(|candidate| candidate.generated)
            .count(),
        ..Summary::default()
    };
    for pair in &data.pairs {
        let run = data
            .runs
            .iter()
            .find(|run| run.evaluation_run == pair.evaluation_run)
            .expect("validated run reference");
        let candidates: Vec<_> = data
            .candidates
            .iter()
            .filter(|candidate| candidate.pair_id == pair.pair_id)
            .collect();
        let degraded = candidates
            .iter()
            .any(|c| candidate_source(c) == SourceLayer::DegradedTemplate);
        let safety_excluded = pair.pair_completion_status == PairCompletion::SafetyStop
            || candidates.iter().any(|candidate| {
                candidate.privacy_or_safety_stop == Measured::Value(true)
                    || candidate.candidate_completion_status == CandidateCompletion::SafetyStop
                    || candidate.failure_code == FailureCode::SafetyStop
            });
        // Pair assignment is fixed by the planned path. A failure/degradation
        // cannot move an assigned pair out of its completion-rate denominator.
        let source = if pair.task_type == TaskType::T1 {
            SourceLayer::LocalTemplate
        } else {
            SourceLayer::UndegradedEndpoint
        };
        let key = PairStratum {
            evaluation_run: run.evaluation_run.clone(),
            participant_scope: run.participant_scope,
            task_type: pair.task_type,
            semantic_stratum: pair.semantic_stratum,
            contrast: pair.contrast,
            conditions: pair.conditions.clone(),
            source,
        };
        add_funnel(&mut report.funnel, pair, &candidates);
        let group = report.pairs.entry(key.clone()).or_default();
        add_funnel(&mut group.funnel, pair, &candidates);
        group.degraded_pairs += usize::from(degraded);
        group.safety_excluded_pairs += usize::from(safety_excluded);
        let compatible = run.consent_state == ConsentState::Granted
            && run.feedback_purpose == FeedbackPurpose::ProductEvaluation
            && pair.comparability_status == Comparability::Comparable
            && !degraded
            && !safety_excluded;
        if compatible
            && candidates.iter().all(|c| c.generated)
            && pair.blind_status == BlindStatus::Intact
            && pair.preference != Preference::Unrated
        {
            group.preference.denominator += 1;
            group.preference.ties += usize::from(pair.preference == Preference::Tie);
            let winner = match pair.preference {
                Preference::Left => Some(pair.left_variant),
                Preference::Right => Some(pair.right_variant),
                _ => None,
            };
            group.preference.target_wins += usize::from(winner == Some(pair.contrast.target()));
        }
        // Breaking the preference blind does not erase observed task edit times.
        if compatible && pair.pair_completion_status == PairCompletion::Completed {
            let target = candidates
                .iter()
                .find(|c| c.requested_variant == pair.contrast.target())
                .expect("validated contrast target");
            let control = candidates
                .iter()
                .find(|c| c.requested_variant != pair.contrast.target())
                .expect("validated contrast control");
            match (control.edit_seconds.value(), target.edit_seconds.value()) {
                (Some(control_seconds), Some(target_seconds)) => group.edits.push(EditPair {
                    pair_id: pair.pair_id.clone(),
                    control_seconds: *control_seconds,
                    target_seconds: *target_seconds,
                }),
                _ => group.edit_missing_pairs += 1,
            }
        }
        for candidate in candidates {
            let candidate_layer = candidate_source(candidate);
            let mut candidate_pair_key = key.clone();
            // A candidate's source is independent from its partner's degradation.
            candidate_pair_key.source = candidate_layer;
            let row = report
                .candidates
                .entry(CandidateStratum {
                    pair: candidate_pair_key,
                    variant: candidate.requested_variant,
                    source: candidate_layer,
                })
                .or_default();
            row.assigned += 1;
            row.generated += usize::from(candidate.generated);
            row.completed += usize::from(
                candidate.candidate_completion_status == CandidateCompletion::Completed,
            );
            row.safety_stops.denominator += 1;
            row.safety_stops.numerator +=
                usize::from(candidate.privacy_or_safety_stop == Measured::Value(true));
            row.safety_missing += usize::from(matches!(
                candidate.privacy_or_safety_stop,
                Measured::Missing(_)
            ));
            if let Some(count) = candidate.help_count.value() {
                row.help_counts.push(*count);
            }
            if let Some(reason) = candidate.missing_reason {
                *row.missing.entry(reason).or_default() += 1;
            }
            if let Measured::Missing(reason) = candidate.edit_seconds {
                *row.edit_missing.entry(reason).or_default() += 1;
            }
            if let Measured::Missing(reason) = candidate.pause_seconds {
                *row.pause_missing.entry(reason).or_default() += 1;
            }
            if candidate.failure_code != FailureCode::None {
                *row.failures.entry(candidate.failure_code).or_default() += 1;
            }
            if run.consent_state == ConsentState::Granted
                && run.feedback_purpose == FeedbackPurpose::ProductEvaluation
                && candidate.generated
                && !safety_excluded
            {
                row.first_pass.denominator += 1;
                row.first_pass.numerator +=
                    usize::from(candidate.first_pass_accepted == Measured::Value(true));
                row.first_pass_missing += usize::from(matches!(
                    candidate.first_pass_accepted,
                    Measured::Missing(_)
                ));
                if pair.task_type == TaskType::T3
                    && candidate_layer == SourceLayer::UndegradedEndpoint
                {
                    row.fact_correct.denominator += 1;
                    row.fact_correct.numerator +=
                        usize::from(candidate.required_fact_error_count == Measured::Value(0));
                    row.fact_missing += usize::from(matches!(
                        candidate.required_fact_error_count,
                        Measured::Missing(_)
                    ));
                }
            }
        }
    }
    for (key, group) in &mut report.pairs {
        group.edit_valid_pairs = group.edits.len();
        group.median_difference = median(
            group
                .edits
                .iter()
                .map(|edit| edit.control_seconds as f64 - edit.target_seconds as f64)
                .collect(),
        );
        group.median_ratio = median(
            group
                .edits
                .iter()
                .filter(|edit| edit.control_seconds != 0)
                .map(|edit| {
                    (edit.control_seconds as f64 - edit.target_seconds as f64)
                        / edit.control_seconds as f64
                })
                .collect(),
        );
        if let Some(value) = group.preference.value() {
            let cohort = CohortStratum {
                participant_scope: key.participant_scope,
                task_type: key.task_type,
                semantic_stratum: key.semantic_stratum,
                contrast: key.contrast,
                conditions: key.conditions.clone(),
                source: key.source,
            };
            report
                .participant_preferences
                .entry(cohort)
                .or_default()
                .per_run
                .insert(key.evaluation_run.clone(), value);
        }
    }
    for preferences in report.participant_preferences.values_mut() {
        preferences.median = median(preferences.per_run.values().copied().collect());
    }
    report
}
