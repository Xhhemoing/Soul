//! Shared synthetic execution, without fixture loading or gold scoring.
use super::{
    adapter::{Harness, Snapshot},
    data::*,
    diagnosis, selector,
};
use serde::Serialize;
use std::{collections::BTreeSet, time::Instant};

#[derive(Serialize)]
pub struct Outcome {
    pub diagnosis_method: bool,
    pub available: bool,
    pub reason: &'static str,
    pub action: Action,
    pub predicted_action: Option<Action>,
    pub required_ids: Option<Vec<String>>,
    pub assessments: Option<Vec<Assessment>>,
    pub full_rank: Option<Vec<String>>,
    pub requested_ids: Option<Vec<String>>,
    pub chosen_groups: Option<Vec<selector::ChosenGroup>>,
    pub selected_ids: Option<Vec<String>>,
    pub retained_ids: Option<Vec<String>>,
    pub group_retained_complete: Option<bool>,
    pub requested_to_selected_loss: Option<usize>,
    pub selected_to_retained_loss: Option<usize>,
    pub attempted_prepare: bool,
    pub preparation: Option<serde_json::Value>,
    pub runtime_ms: f64,
    pub network_requests: usize,
}

impl Outcome {
    pub fn unavailable(reason: &'static str) -> Self {
        Self {
            diagnosis_method: true,
            available: false,
            reason,
            action: Action::Unknown,
            predicted_action: None,
            required_ids: None,
            assessments: None,
            full_rank: None,
            requested_ids: None,
            chosen_groups: None,
            selected_ids: None,
            retained_ids: None,
            group_retained_complete: None,
            requested_to_selected_loss: None,
            selected_to_retained_loss: None,
            attempted_prepare: false,
            preparation: None,
            runtime_ms: 0.0,
            network_requests: 0,
        }
    }
}

fn execute_inner(
    input: &Input,
    prediction: Result<&Prediction, &'static str>,
    rank: Result<&[String], &'static str>,
    method: &str,
    harness: &mut Harness,
    snapshot: &Snapshot,
) -> Outcome {
    let mut result = Outcome::unavailable("unknown");
    result.diagnosis_method = matches!(method, "all_options" | "predicted_required");
    let input_check = if matches!(method, "fixed4" | "fixed8") {
        diagnosis::validate_fixed_input(input)
    } else {
        diagnosis::validate_input(input)
    };
    if let Err(reason) = input_check.and_then(|_| harness.validate(snapshot)) {
        result.reason = reason;
        return result;
    }
    let fixed = match method {
        "fixed4" => Some(4),
        "fixed8" => Some(8),
        _ => None,
    };
    let effective;
    if fixed.is_none() {
        let prediction = match prediction {
            Ok(prediction) => prediction,
            Err(reason) => {
                result.reason = reason;
                return result;
            }
        };
        if let Err(reason) = diagnosis::validate_prediction(input, prediction) {
            result.reason = reason;
            return result;
        }
        effective = match method {
            "predicted_required" => Prediction {
                case_id: prediction.case_id.clone(),
                required_ids: prediction.required_ids.clone(),
                assessments: prediction
                    .assessments
                    .iter()
                    .filter(|a| prediction.required_ids.contains(&a.need_id))
                    .cloned()
                    .collect(),
            },
            "all_options" => diagnosis::all_options(input, prediction),
            _ => {
                result.reason = "unknown_method";
                return result;
            }
        };
        result.action = diagnosis::action(&effective);
        result.predicted_action = Some(result.action);
        result.required_ids = Some(effective.required_ids.clone());
        result.assessments = Some(effective.assessments.clone());
        if result.action != Action::Ready {
            result.available = true;
            result.reason = match result.action {
                Action::ClarifyConflict => "required_information_conflict",
                Action::ClarifyMissing => "required_information_missing",
                _ => "required_information_unknown",
            };
            return result;
        }
        if effective.required_ids.is_empty() {
            result.available = true;
            result.reason = "no_required_information";
            result.selected_ids = Some(Vec::new());
            result.retained_ids = Some(Vec::new());
            result.chosen_groups = Some(Vec::new());
            return result;
        }
    } else {
        effective = Prediction {
            case_id: input.case_id.clone(),
            required_ids: Vec::new(),
            assessments: Vec::new(),
        };
        result.predicted_action = Some(Action::Ready);
    }
    let rank = match rank {
        Ok(rank) => rank,
        Err(reason) => {
            result.reason = reason;
            result.action = Action::Unknown;
            return result;
        }
    };
    if let Err(reason) = diagnosis::validate_rank(input, rank) {
        result.reason = reason;
        result.action = Action::Unknown;
        return result;
    }
    let selection = if let Some(k) = fixed {
        let requested_ids: Vec<_> = rank
            .iter()
            .filter(|id| !input.unavailable_ids.contains(id))
            .take(k)
            .cloned()
            .collect();
        let mut full_rank = requested_ids.clone();
        full_rank.extend(
            rank.iter()
                .filter(|id| !requested_ids.contains(id))
                .cloned(),
        );
        selector::Selection {
            available: true,
            reason: "fixed_readable_prefix",
            full_rank,
            requested_ids,
            chosen_groups: Vec::new(),
            unsatisfied_need_ids: Vec::new(),
        }
    } else {
        match diagnosis::selection(input, &effective, rank) {
            Ok(selection) => selection,
            Err(reason) => {
                result.reason = reason;
                result.action = Action::Unknown;
                return result;
            }
        }
    };
    result.reason = selection.reason;
    result.full_rank = Some(selection.full_rank.clone());
    result.requested_ids = Some(selection.requested_ids.clone());
    if !selection.available {
        result.chosen_groups = Some(selection.chosen_groups);
        result.action = Action::Unknown;
        return result;
    }
    if selection.requested_ids.is_empty() {
        result.available = fixed.is_none();
        result.action = if result.available {
            Action::Ready
        } else {
            Action::Unknown
        };
        result.reason = if result.available {
            "no_required_information"
        } else {
            "no_readable_candidates"
        };
        result.selected_ids = Some(Vec::new());
        result.retained_ids = Some(Vec::new());
        result.chosen_groups = Some(selection.chosen_groups);
        return result;
    }
    assert!(selection
        .requested_ids
        .iter()
        .all(|id| !input.unavailable_ids.contains(id)));
    result.attempted_prepare = true;
    match harness.observe_prefix_allow_loss(
        snapshot,
        &selection.full_rank,
        selection.requested_ids.len(),
        true,
    ) {
        Err(reason) => {
            result.reason = reason;
            result.action = Action::Unknown;
        }
        Ok(observation) => {
            let requested: BTreeSet<_> = selection.requested_ids.iter().collect();
            let exact = observation.selected.iter().collect::<BTreeSet<_>>() == requested
                && observation.retained.iter().collect::<BTreeSet<_>>() == requested
                && observation.selected.len() == requested.len()
                && observation.retained.len() == requested.len();
            let group_retained = (!selection.chosen_groups.is_empty()).then(|| {
                selector::groups_retained(&selection.chosen_groups, &observation.retained)
            });
            result.available = exact && group_retained.unwrap_or(true);
            result.action = if result.available {
                Action::Ready
            } else {
                Action::Unknown
            };
            if !result.available {
                result.reason = "serialized_selection_loss";
            }
            result.group_retained_complete = group_retained;
            result.requested_to_selected_loss =
                Some(selection.requested_ids.len() - observation.selected.len());
            result.selected_to_retained_loss =
                Some(observation.selected.len() - observation.retained.len());
            result.preparation = Some(observation.metrics);
            result.selected_ids = Some(observation.selected);
            result.retained_ids = Some(observation.retained);
        }
    }
    result.chosen_groups = Some(selection.chosen_groups);
    result
}

pub fn execute(
    input: &Input,
    prediction: Result<&Prediction, &'static str>,
    rank: Result<&[String], &'static str>,
    method: &str,
    harness: &mut Harness,
    snapshot: &Snapshot,
) -> Outcome {
    let started = Instant::now();
    let mut result = execute_inner(input, prediction, rank, method, harness, snapshot);
    result.runtime_ms = started.elapsed().as_secs_f64() * 1000.0;
    if let Err(reason) = harness.validate(snapshot) {
        result.available = false;
        result.action = Action::Unknown;
        result.reason = reason;
    }
    result
}
