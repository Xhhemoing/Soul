//! Runtime diagnosis sees input and predictions only. No fixture truth is imported.
use super::{data::*, policy, selector, semantic, types};
use std::collections::BTreeSet;

pub fn validate_input(input: &Input) -> Result<(), &'static str> {
    validate_input_inner(input, false)
}

/// Fixed retrieval does not consume an information-option list.
pub fn validate_fixed_input(input: &Input) -> Result<(), &'static str> {
    validate_input_inner(input, true)
}

fn validate_input_inner(input: &Input, allow_empty_options: bool) -> Result<(), &'static str> {
    types::case_bytes(&input.case())?;
    if input.case_id.is_empty()
        || input.case_id.len() > 128
        || !input
            .case_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("input_case_id");
    }
    if (!allow_empty_options && input.information_options.is_empty())
        || input.information_options.len() > 8
    {
        return Err("input_unsupported_option_count");
    }
    let options: BTreeSet<_> = input.information_options.iter().map(|o| &o.id).collect();
    if options.len() != input.information_options.len()
        || input.information_options.iter().any(|o| {
            o.id.is_empty()
                || o.id.len() > 64
                || !o.id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || o.description.trim().is_empty()
                || o.description.chars().count() > 240
        })
    {
        return Err("input_information_options");
    }
    let pool: BTreeSet<_> = input.candidates.iter().map(|c| &c.id).collect();
    let unavailable: BTreeSet<_> = input.unavailable_ids.iter().collect();
    if unavailable.len() != input.unavailable_ids.len() || !unavailable.is_subset(&pool) {
        return Err("input_unavailable_ids");
    }
    validate_metadata(input)?;
    if serde_json::to_vec(input)
        .map_err(|_| "input_encoding")?
        .len()
        > policy::MAX_PREDICTION_BYTES
    {
        return Err("input_budget");
    }
    Ok(())
}

fn validate_metadata(input: &Input) -> Result<(), &'static str> {
    if input.source_metadata.is_empty() {
        return Ok(());
    }
    let pool: BTreeSet<_> = input.candidates.iter().map(|c| &c.id).collect();
    let metadata_ids: BTreeSet<_> = input.source_metadata.iter().map(|m| &m.source_id).collect();
    if metadata_ids != pool || metadata_ids.len() != input.source_metadata.len() {
        return Err("source_metadata_ids");
    }
    let bounded = |text: &String| !text.trim().is_empty() && text.chars().count() <= 240;
    for metadata in &input.source_metadata {
        let supersedes: BTreeSet<_> = metadata.supersedes.iter().collect();
        if !bounded(&metadata.subject)
            || !bounded(&metadata.scope)
            || !bounded(&metadata.valid_time)
            || metadata.topics.is_empty()
            || metadata.topics.len() > 4
            || !metadata.topics.iter().all(bounded)
            || supersedes.len() != metadata.supersedes.len()
            || !supersedes.is_subset(&pool)
            || supersedes.contains(&metadata.source_id)
        {
            return Err("source_metadata_shape");
        }
    }
    if input
        .candidates
        .iter()
        .any(|c| input.unavailable_ids.contains(&c.id) != (c.summary == "[来源不可读取]"))
    {
        return Err("source_readability_mismatch");
    }
    Ok(())
}

pub fn parse(bytes: &[u8], input: &Input) -> Result<Prediction, &'static str> {
    validate_input(input)?;
    if bytes.len() > policy::MAX_PREDICTION_BYTES {
        return Err("prediction_budget");
    }
    let prediction: Prediction = serde_json::from_slice(bytes).map_err(|_| "prediction_shape")?;
    validate_prediction(input, &prediction)?;
    Ok(prediction)
}

pub fn validate_prediction(input: &Input, prediction: &Prediction) -> Result<(), &'static str> {
    if prediction.case_id != input.case_id {
        return Err("prediction_case_id");
    }
    let options: BTreeSet<_> = input.information_options.iter().map(|o| &o.id).collect();
    let required: BTreeSet<_> = prediction.required_ids.iter().collect();
    if required.len() != prediction.required_ids.len() || !required.is_subset(&options) {
        return Err("required_id_set");
    }
    let assessed: BTreeSet<_> = prediction.assessments.iter().map(|a| &a.need_id).collect();
    if !required.is_subset(&assessed)
        || !assessed.is_subset(&options)
        || prediction.assessments.len() != assessed.len()
    {
        return Err("assessment_need_set");
    }
    let readable: BTreeSet<_> = input
        .candidates
        .iter()
        .filter(|c| !input.unavailable_ids.contains(&c.id))
        .map(|c| &c.id)
        .collect();
    for assessment in &prediction.assessments {
        let mut alternatives = BTreeSet::new();
        for group in &assessment.support_groups {
            let ids: BTreeSet<_> = group.iter().collect();
            if group.is_empty() || ids.len() != group.len() || !ids.is_subset(&readable) {
                return Err("support_source_ids");
            }
            if !alternatives.insert(ids) {
                return Err("duplicate_support_group");
            }
        }
        let conflict: BTreeSet<_> = assessment.conflict_ids.iter().collect();
        if conflict.len() != assessment.conflict_ids.len() || !conflict.is_subset(&readable) {
            return Err("conflict_source_ids");
        }
        let valid_shape = match assessment.status {
            Status::Supported => !assessment.support_groups.is_empty() && conflict.is_empty(),
            Status::Missing | Status::Unknown => {
                assessment.support_groups.is_empty() && conflict.is_empty()
            }
            Status::Conflict => assessment.support_groups.is_empty() && conflict.len() >= 2,
        };
        if !valid_shape {
            return Err("assessment_status_shape");
        }
    }
    Ok(())
}

pub fn action(prediction: &Prediction) -> Action {
    if prediction
        .assessments
        .iter()
        .filter(|a| prediction.required_ids.contains(&a.need_id))
        .any(|a| a.status == Status::Unknown)
    {
        Action::Unknown
    } else if prediction
        .assessments
        .iter()
        .filter(|a| prediction.required_ids.contains(&a.need_id))
        .any(|a| a.status == Status::Conflict)
    {
        Action::ClarifyConflict
    } else if prediction
        .assessments
        .iter()
        .filter(|a| prediction.required_ids.contains(&a.need_id))
        .any(|a| a.status == Status::Missing)
    {
        Action::ClarifyMissing
    } else {
        Action::Ready
    }
}

pub fn all_options(input: &Input, prediction: &Prediction) -> Prediction {
    Prediction {
        case_id: input.case_id.clone(),
        required_ids: input
            .information_options
            .iter()
            .map(|o| o.id.clone())
            .collect(),
        assessments: input
            .information_options
            .iter()
            .map(|option| {
                prediction
                    .assessments
                    .iter()
                    .find(|a| a.need_id == option.id)
                    .cloned()
                    .unwrap_or_else(|| Assessment {
                        need_id: option.id.clone(),
                        status: Status::Unknown,
                        support_groups: Vec::new(),
                        conflict_ids: Vec::new(),
                    })
            })
            .collect(),
    }
}

pub fn validate_rank(input: &Input, rank: &[String]) -> Result<(), &'static str> {
    let encoded =
        serde_json::to_vec(&serde_json::json!([{"case_id":input.case_id,"ranked_ids":rank}]))
            .map_err(|_| "ranking_encoding")?;
    semantic::parse(&encoded, &[input.case()])?;
    Ok(())
}

pub fn selection(
    input: &Input,
    prediction: &Prediction,
    rank: &[String],
) -> Result<selector::Selection, &'static str> {
    validate_input(input)?;
    validate_prediction(input, prediction)?;
    validate_rank(input, rank)?;
    if action(prediction) != Action::Ready {
        return Err("diagnosis_not_ready");
    }
    if prediction.required_ids.is_empty() {
        return Ok(selector::Selection {
            available: true,
            reason: "no_required_information",
            full_rank: rank.to_vec(),
            requested_ids: Vec::new(),
            chosen_groups: Vec::new(),
            unsatisfied_need_ids: Vec::new(),
        });
    }
    let needs = policy::Needs {
        task_id: "synthetic_task_requirements".into(),
        needs: prediction
            .required_ids
            .iter()
            .map(|id| policy::Need {
                need_id: id.clone(),
                description: input
                    .information_options
                    .iter()
                    .find(|o| &o.id == id)
                    .unwrap()
                    .description
                    .clone(),
            })
            .collect(),
    };
    let supports = policy::SupportMap {
        case_id: input.case_id.clone(),
        supports: prediction
            .assessments
            .iter()
            .filter(|a| prediction.required_ids.contains(&a.need_id))
            .map(|a| policy::Support {
                need_id: a.need_id.clone(),
                groups: a.support_groups.clone(),
            })
            .collect(),
    };
    selector::select(&input.case(), &needs, &supports, rank, false)
}
