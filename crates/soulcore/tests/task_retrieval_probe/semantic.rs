//! Strict whole-batch parsing. Unusable output is reported, never repaired.
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::types::InputCase;

pub const MAX_RESPONSE_BYTES: usize = 64 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Response {
    case_id: String,
    ranked_ids: Vec<String>,
}

pub fn parse(
    bytes: &[u8],
    cases: &[InputCase],
) -> Result<BTreeMap<String, Vec<String>>, &'static str> {
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err("response_budget");
    }
    let rows: Vec<Response> = serde_json::from_slice(bytes).map_err(|_| "invalid_json_or_shape")?;
    if rows.len() != cases.len() {
        return Err("case_count");
    }
    let mut output = BTreeMap::new();
    for row in rows {
        let expected = cases
            .iter()
            .find(|case| case.case_id == row.case_id)
            .ok_or("unknown_case")?;
        super::types::case_bytes(expected)?;
        if row.ranked_ids.len() != expected.candidates.len() {
            return Err("rank_count");
        }
        let actual: BTreeSet<_> = row.ranked_ids.iter().collect();
        if actual.len() != expected.candidates.len() {
            return Err("duplicate_id");
        }
        let allowed: BTreeSet<_> = expected
            .candidates
            .iter()
            .map(|candidate| &candidate.id)
            .collect();
        if actual != allowed {
            return Err("unknown_or_missing_id");
        }
        if output.insert(row.case_id, row.ranked_ids).is_some() {
            return Err("duplicate_case");
        }
    }
    if output.len() != cases.len() {
        return Err("case_set");
    }
    Ok(output)
}

/// Re-encode validated results in original case order without changing ranks.
pub fn encode_ordered(
    cases: &[InputCase],
    mut predictions: BTreeMap<String, Vec<String>>,
) -> Result<Vec<u8>, &'static str> {
    if predictions.len() != cases.len() {
        return Err("case_count");
    }
    let rows = cases
        .iter()
        .map(|case| {
            Ok(Response {
                case_id: case.case_id.clone(),
                ranked_ids: predictions.remove(&case.case_id).ok_or("case_set")?,
            })
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    if !predictions.is_empty() {
        return Err("case_set");
    }
    let bytes = serde_json::to_vec(&rows).map_err(|_| "response_encoding")?;
    parse(&bytes, cases)?;
    Ok(bytes)
}
