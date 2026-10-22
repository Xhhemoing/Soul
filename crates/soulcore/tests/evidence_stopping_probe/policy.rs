//! Machine predictions and stopping rules. No fixture or human truth access.
use super::types::InputCase;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_PREDICTION_BYTES: usize = 64 * 1024;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Need {
    pub need_id: String,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Needs {
    pub task_id: String,
    pub needs: Vec<Need>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Support {
    pub need_id: String,
    pub groups: Vec<Vec<String>>,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SupportMap {
    pub case_id: String,
    pub supports: Vec<Support>,
}

#[derive(Serialize)]
pub struct Satisfaction {
    pub need_id: String,
    pub first_satisfied_at: Option<usize>,
    pub triggering_group: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct Decision {
    pub requested_k: usize,
    pub reason: &'static str,
    pub all_needs_satisfied: bool,
    pub needs: Vec<Satisfaction>,
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, &'static str> {
    if bytes.len() > MAX_PREDICTION_BYTES {
        return Err("prediction_budget");
    }
    serde_json::from_slice(bytes).map_err(|_| "prediction_shape")
}

pub fn parse_needs(bytes: &[u8], task_id: &str) -> Result<Needs, &'static str> {
    let value: Needs = decode(bytes)?;
    if value.task_id != task_id {
        return Err("task_id");
    }
    if value.needs.is_empty() || value.needs.len() > 8 {
        return Err("need_count");
    }
    let mut ids = BTreeSet::new();
    for need in &value.needs {
        if need.need_id.is_empty()
            || need.need_id.len() > 64
            || !need
                .need_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err("need_id");
        }
        if !ids.insert(&need.need_id) {
            return Err("duplicate_need");
        }
        if need.description.trim().is_empty() || need.description.chars().count() > 240 {
            return Err("need_description");
        }
    }
    Ok(value)
}

pub fn parse_support(
    bytes: &[u8],
    case: &InputCase,
    needs: &Needs,
) -> Result<SupportMap, &'static str> {
    let value: SupportMap = decode(bytes)?;
    if value.case_id != case.case_id {
        return Err("case_id");
    }
    let expected: BTreeSet<_> = needs.needs.iter().map(|need| &need.need_id).collect();
    let actual: BTreeSet<_> = value
        .supports
        .iter()
        .map(|support| &support.need_id)
        .collect();
    if value.supports.len() != expected.len() || actual != expected {
        return Err("support_need_set");
    }
    let candidates: BTreeSet<_> = case
        .candidates
        .iter()
        .map(|candidate| &candidate.id)
        .collect();
    for support in &value.supports {
        let mut alternatives = BTreeSet::new();
        for group in &support.groups {
            let ids: BTreeSet<_> = group.iter().collect();
            if group.is_empty() || ids.len() != group.len() || !ids.is_subset(&candidates) {
                return Err("support_group_ids");
            }
            if !alternatives.insert(ids) {
                return Err("duplicate_support_group");
            }
        }
    }
    Ok(value)
}

pub fn decide(
    case: &InputCase,
    needs: &Needs,
    support: &SupportMap,
    full_rank: &[String],
) -> Result<Decision, &'static str> {
    // Validate even the unconsumed tail; no valid prefix can hide an invalid rank.
    let expected: BTreeSet<_> = case
        .candidates
        .iter()
        .map(|candidate| &candidate.id)
        .collect();
    if full_rank.len() != case.candidates.len()
        || full_rank.iter().collect::<BTreeSet<_>>() != expected
    {
        return Err("rank_not_permutation");
    }
    // Public structures cannot bypass parser invariants when used by tests.
    parse_needs(
        &serde_json::to_vec(needs).map_err(|_| "encoding")?,
        &needs.task_id,
    )?;
    parse_support(
        &serde_json::to_vec(support).map_err(|_| "encoding")?,
        case,
        needs,
    )?;
    let cap = soul_memory::SelectionLimits::default()
        .max_selected
        .min(full_rank.len());
    if cap == 0 {
        return Err("empty_candidates");
    }
    let mut results: Vec<_> = needs
        .needs
        .iter()
        .map(|need| Satisfaction {
            need_id: need.need_id.clone(),
            first_satisfied_at: None,
            triggering_group: None,
        })
        .collect();
    for k in 1..=cap {
        for result in &mut results {
            if result.first_satisfied_at.is_some() {
                continue;
            }
            let row = support
                .supports
                .iter()
                .find(|row| row.need_id == result.need_id)
                .ok_or("support_need_set")?;
            if let Some(group) = row
                .groups
                .iter()
                .find(|group| group.iter().all(|id| full_rank[..k].contains(id)))
            {
                result.first_satisfied_at = Some(k);
                result.triggering_group = Some(group.clone());
            }
        }
        if results.iter().all(|need| need.first_satisfied_at.is_some()) {
            return Ok(Decision {
                requested_k: k,
                reason: "needs_satisfied",
                all_needs_satisfied: true,
                needs: results,
            });
        }
    }
    Ok(Decision {
        requested_k: cap,
        reason: "budget_cap",
        all_needs_satisfied: false,
        needs: results,
    })
}
