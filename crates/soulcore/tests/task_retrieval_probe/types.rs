//! Searchable input only. Gold and source authorization snapshots are separate.
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskInput {
    pub scope: String,
    pub intent: String,
    pub constraints: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub title: String,
    pub summary: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputCase {
    pub case_id: String,
    pub task: TaskInput,
    pub candidates: Vec<Candidate>,
}

pub const MAX_CASE_BYTES: usize = 32 * 1024;

pub fn case_bytes(case: &InputCase) -> Result<usize, &'static str> {
    if case.candidates.is_empty()
        || case.candidates.len() > soul_memory::SelectionLimits::default().max_candidates
    {
        return Err("candidate_count");
    }
    if case
        .candidates
        .iter()
        .map(|candidate| &candidate.id)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != case.candidates.len()
    {
        return Err("duplicate_candidate_id");
    }
    let size = serde_json::to_vec(case)
        .map_err(|_| "input_encoding")?
        .len();
    if size > MAX_CASE_BYTES {
        return Err("input_budget");
    }
    Ok(size)
}
