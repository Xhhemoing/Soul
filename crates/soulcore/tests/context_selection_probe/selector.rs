//! Selection sees searchable input and machine predictions only, never fixture truth.
use super::{policy, types::InputCase};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize)]
pub struct ChosenGroup {
    pub need_id: String,
    pub source_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct Selection {
    pub available: bool,
    pub reason: &'static str,
    pub full_rank: Vec<String>,
    pub requested_ids: Vec<String>,
    pub chosen_groups: Vec<ChosenGroup>,
    pub unsatisfied_need_ids: Vec<String>,
}

pub fn select(
    case: &InputCase,
    needs: &policy::Needs,
    support: &policy::SupportMap,
    rank: &[String],
    plus_initial4: bool,
) -> Result<Selection, &'static str> {
    // Reuse full-rank and public-prediction invariant validation, not its prefix decision.
    policy::decide(case, needs, support, rank)?;
    let positions: BTreeMap<_, _> = rank
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index))
        .collect();
    let mut selected = BTreeSet::new();
    let mut chosen_groups = Vec::new();
    let mut unsatisfied_need_ids = Vec::new();
    for need in &needs.needs {
        let row = support
            .supports
            .iter()
            .find(|row| row.need_id == need.need_id)
            .ok_or("support_need_set")?;
        // Frozen before replay: new-source count, latest original rank position,
        // then canonical ID order. A group is an AND, alternatives are ORs.
        let key = |group: &Vec<String>| {
            let mut ids = group.clone();
            ids.sort();
            (
                group.iter().filter(|id| !selected.contains(*id)).count(),
                group.iter().map(|id| positions[id.as_str()]).max().unwrap(),
                ids,
            )
        };
        if let Some(group) = row.groups.iter().min_by_key(|group| key(group)) {
            let mut source_ids = group.clone();
            source_ids.sort();
            selected.extend(source_ids.iter().cloned());
            chosen_groups.push(ChosenGroup {
                need_id: need.need_id.clone(),
                source_ids,
            });
        } else {
            unsatisfied_need_ids.push(need.need_id.clone());
        }
    }
    if plus_initial4 {
        selected.extend(rank.iter().take(4).cloned());
    }
    let requested_ids: Vec<_> = rank
        .iter()
        .filter(|id| selected.contains(*id))
        .cloned()
        .collect();
    let mut full_rank = requested_ids.clone();
    full_rank.extend(rank.iter().filter(|id| !selected.contains(*id)).cloned());
    let reason = if !unsatisfied_need_ids.is_empty() {
        "mapping_unsatisfied"
    } else if requested_ids.len() > soul_memory::SelectionLimits::default().max_selected {
        "selection_budget"
    } else {
        "support_groups_selected"
    };
    Ok(Selection {
        available: reason == "support_groups_selected",
        reason,
        full_rank,
        requested_ids,
        chosen_groups,
        unsatisfied_need_ids,
    })
}

pub fn groups_retained(groups: &[ChosenGroup], retained: &[String]) -> bool {
    groups
        .iter()
        .all(|group| group.source_ids.iter().all(|id| retained.contains(id)))
}
