//! Post-runtime evaluation only. This module never supplies information to diagnosis.
use super::{data::*, replay::Outcome};
use std::collections::BTreeSet;

pub fn score(gold: &Gold, prediction: Option<&Prediction>, outcome: &Outcome) -> serde_json::Value {
    let required: BTreeSet<_> = gold.required_ids.iter().collect();
    let predicted: BTreeSet<_> = outcome
        .required_ids
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .collect();
    let retained = outcome.retained_ids.as_deref().unwrap_or(&[]);
    let supported: Vec<_> = gold
        .assessments
        .iter()
        .filter(|a| required.contains(&a.need_id) && a.status == Status::Supported)
        .collect();
    let covered = supported
        .iter()
        .filter(|assessment| {
            assessment
                .support_groups
                .iter()
                .any(|group| group.iter().all(|id| retained.contains(id)))
        })
        .count();
    let all_fact_groups_covered = covered == supported.len();
    let expected_sources: BTreeSet<_> = gold
        .assessments
        .iter()
        .flat_map(|a| {
            a.support_groups
                .iter()
                .flatten()
                .chain(a.conflict_ids.iter())
        })
        .collect();
    let extra_source_ids: Vec<_> = retained
        .iter()
        .filter(|id| !expected_sources.contains(id))
        .collect();
    let status_correct = outcome
        .assessments
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter(|a| {
            required.contains(&a.need_id)
                && gold
                    .assessments
                    .iter()
                    .any(|truth| truth.need_id == a.need_id && truth.status == a.status)
        })
        .count();
    let action_correct = outcome.available && outcome.action == gold.action;
    let requirement_count = |value| outcome.diagnosis_method.then_some(value);
    let group_assessment_correct = |assessment: &Assessment| {
        gold.assessments
            .iter()
            .find(|truth| truth.need_id == assessment.need_id)
            .is_some_and(|truth| {
                assessment.status == truth.status
                    && match assessment.status {
                        Status::Supported => assessment.support_groups.iter().all(|claim| {
                            truth
                                .support_groups
                                .iter()
                                .any(|group| group.iter().all(|id| claim.contains(id)))
                        }),
                        Status::Conflict => match gold.conflict_groups.get(&assessment.need_id) {
                            Some(groups) => groups.iter().any(|group| {
                                !group.is_empty()
                                    && group.iter().all(|id| assessment.conflict_ids.contains(id))
                            }),
                            None => truth
                                .conflict_ids
                                .iter()
                                .all(|id| assessment.conflict_ids.contains(id)),
                        },
                        Status::Missing | Status::Unknown => true,
                    }
            })
    };
    let diagnosis_correct = outcome.diagnosis_method.then(|| {
        outcome.required_ids.is_some()
            && predicted == required
            && outcome
                .assessments
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .all(group_assessment_correct)
            && outcome.predicted_action == Some(gold.action)
    });
    serde_json::json!({
        "case_denominator":1,"action_denominator":1,"action_correct":action_correct,
        "diagnosis_denominator":usize::from(outcome.diagnosis_method),"diagnosis_correct":diagnosis_correct,
        "prediction_available":prediction.is_some(),
        "required_tp":requirement_count(predicted.intersection(&required).count()),
        "required_fp":requirement_count(predicted.difference(&required).count()),
        "required_fn":requirement_count(required.difference(&predicted).count()),
        "required_gold_count":required.len(),"required_predicted_count":requirement_count(predicted.len()),
        "necessity_output_available":outcome.required_ids.is_some(),
        "required_status_correct":requirement_count(status_correct),"required_status_denominator":requirement_count(required.len()),
        "prediction_action_correct":outcome.predicted_action.map(|a|a == gold.action),
        "prediction_wrong_sufficient":outcome.predicted_action.map(|a|a == Action::Ready && gold.action != Action::Ready),
        "wrong_sufficient":outcome.action == Action::Ready && (gold.action != Action::Ready || !all_fact_groups_covered),
        "unnecessary_clarify":gold.action == Action::Ready && matches!(outcome.action, Action::ClarifyMissing | Action::ClarifyConflict),
        "required_supported_need_count":supported.len(),"required_supported_need_coverage":covered,
        "all_required_fact_groups_retained":all_fact_groups_covered,
        "retained_count":retained.len(),"extra_source_ids":extra_source_ids,"extra_source_count":extra_source_ids.len()
    })
}
