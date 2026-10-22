//! Fixed lexical controls; only searchable fields are accepted.
use super::types::InputCase;
use std::collections::{BTreeMap, BTreeSet};

type Term = (char, char);

fn frequencies(text: &str) -> BTreeMap<Term, usize> {
    let chars: Vec<_> = text
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let mut terms = BTreeMap::new();
    for pair in chars.windows(2) {
        *terms.entry((pair[0], pair[1])).or_insert(0) += 1;
    }
    terms
}

pub fn groups(case: &InputCase) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for (index, candidate) in case.candidates.iter().enumerate() {
        if let Some(group) = groups.iter_mut().find(|group| {
            let representative = &case.candidates[group[0]];
            representative.title == candidate.title && representative.summary == candidate.summary
        }) {
            group.push(index);
        } else {
            groups.push(vec![index]);
        }
    }
    groups
}

pub fn rank(case: &InputCase, grouped: bool, diversity: bool) -> Vec<String> {
    let groups = groups(case);
    let indices: Vec<usize> = if grouped {
        groups.iter().map(|group| group[0]).collect()
    } else {
        (0..case.candidates.len()).collect()
    };
    let documents: Vec<_> = indices
        .iter()
        .map(|index| {
            let candidate = &case.candidates[*index];
            frequencies(&format!("{}\n{}", candidate.title, candidate.summary))
        })
        .collect();
    let lengths: Vec<_> = documents
        .iter()
        .map(|terms| terms.values().sum::<usize>() as f64)
        .collect();
    let average = lengths.iter().sum::<f64>() / documents.len() as f64;
    let mut query = BTreeMap::new();
    for (text, weight) in [
        (&case.task.scope, 1.0),
        (&case.task.intent, 2.0),
        (&case.task.constraints, 1.0),
    ] {
        for term in frequencies(text).keys() {
            *query.entry(*term).or_insert(0.0) += weight;
        }
    }
    let scores: Vec<f64> = documents
        .iter()
        .zip(&lengths)
        .map(|(document, length)| {
            query
                .iter()
                .map(|(term, weight)| {
                    let tf = *document.get(term).unwrap_or(&0) as f64;
                    if tf == 0.0 || average == 0.0 {
                        return 0.0;
                    }
                    let df = documents
                        .iter()
                        .filter(|doc| doc.contains_key(term))
                        .count() as f64;
                    let idf = (1.0 + (documents.len() as f64 - df + 0.5) / (df + 0.5)).ln();
                    weight * idf * tf * 2.2 / (tf + 1.2 * (0.25 + 0.75 * length / average))
                })
                .sum()
        })
        .collect();
    let mut remaining: Vec<_> = (0..indices.len()).collect();
    let mut chosen = Vec::new();
    if diversity {
        let sets: Vec<BTreeSet<_>> = documents
            .iter()
            .map(|doc| doc.keys().copied().collect())
            .collect();
        let max_score = scores.iter().copied().fold(0.0, f64::max);
        while !remaining.is_empty() {
            let value = |index: usize, chosen: &[usize]| {
                let relevance = if max_score == 0.0 {
                    0.0
                } else {
                    scores[index] / max_score
                };
                let similarity = chosen
                    .iter()
                    .map(|previous| {
                        let union = sets[index].union(&sets[*previous]).count();
                        if union == 0 {
                            0.0
                        } else {
                            sets[index].intersection(&sets[*previous]).count() as f64 / union as f64
                        }
                    })
                    .fold(0.0, f64::max);
                0.75 * relevance - 0.25 * similarity
            };
            remaining.sort_by(|left, right| {
                value(*right, &chosen)
                    .total_cmp(&value(*left, &chosen))
                    .then_with(|| indices[*left].cmp(&indices[*right]))
            });
            chosen.push(remaining.remove(0));
        }
    } else {
        remaining.sort_by(|left, right| {
            scores[*right]
                .total_cmp(&scores[*left])
                .then_with(|| indices[*left].cmp(&indices[*right]))
        });
        chosen = remaining;
    }
    let mut ordered: Vec<_> = chosen.into_iter().map(|index| indices[index]).collect();
    if grouped {
        let representatives: BTreeSet<_> = indices.into_iter().collect();
        ordered.extend((0..case.candidates.len()).filter(|index| !representatives.contains(index)));
    }
    ordered
        .into_iter()
        .map(|index| case.candidates[index].id.clone())
        .collect()
}
