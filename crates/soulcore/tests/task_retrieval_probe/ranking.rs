//! Fixed character-bigram Dice ranking. No dictionaries, scene labels or gold.
use std::collections::BTreeSet;

use super::types::InputCase;

fn bigrams(text: &str) -> BTreeSet<(char, char)> {
    let characters: Vec<_> = text
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    characters
        .windows(2)
        .map(|pair| (pair[0], pair[1]))
        .collect()
}

pub fn rank(case: &InputCase, full_task: bool) -> Vec<String> {
    let task = if full_task {
        format!(
            "{}\n{}\n{}",
            case.task.scope, case.task.intent, case.task.constraints
        )
    } else {
        case.task.scope.clone()
    };
    let query = bigrams(&task);
    let mut scores: Vec<_> = case
        .candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let terms = bigrams(&format!("{}\n{}", candidate.title, candidate.summary));
            let denominator = query.len() + terms.len();
            let score = if denominator == 0 {
                0.0
            } else {
                2.0 * query.intersection(&terms).count() as f64 / denominator as f64
            };
            (index, score)
        })
        .collect();
    scores.sort_by(|(left_index, left), (right_index, right)| {
        right
            .total_cmp(left)
            .then_with(|| left_index.cmp(right_index))
    });
    scores
        .into_iter()
        .map(|(index, _)| case.candidates[index].id.clone())
        .collect()
}
