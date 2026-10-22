//! Experimental ordering sees only identifiers and locally readable fields.
//! No scene, variant, required/update label or relevance oracle crosses here.
use uuid::Uuid;

pub struct Candidate<'a> {
    pub id: Uuid,
    pub title: &'a str,
    pub summary: &'a str,
}

#[derive(Clone, Copy)]
pub enum Strategy {
    SoftRank,
    Interleave,
}

pub fn order(candidates: &[Candidate<'_>], query: Option<&str>, strategy: Strategy) -> Vec<Uuid> {
    let tokens: Vec<_> = query
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_lowercase)
        .collect();
    if tokens.is_empty() {
        return candidates.iter().map(|entry| entry.id).collect();
    }
    let mut ranked: Vec<_> = candidates
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let title = entry.title.to_lowercase();
            let summary = entry.summary.to_lowercase();
            let hits = tokens
                .iter()
                .filter(|token| title.contains(token.as_str()) || summary.contains(token.as_str()))
                .count();
            (index, hits, entry.id)
        })
        .collect();
    ranked.sort_by_key(|(index, hits, _)| (std::cmp::Reverse(*hits), *index));
    if matches!(strategy, Strategy::SoftRank) {
        return ranked.into_iter().map(|(_, _, id)| id).collect();
    }
    let mut matched = ranked.iter().filter(|(_, hits, _)| *hits > 0);
    let mut unmatched = ranked.iter().filter(|(_, hits, _)| *hits == 0);
    let mut output = Vec::with_capacity(candidates.len());
    loop {
        let positive = matched.next();
        let negative = unmatched.next();
        if positive.is_none() && negative.is_none() {
            break;
        }
        if let Some((_, _, id)) = positive {
            output.push(*id);
        }
        if let Some((_, _, id)) = negative {
            output.push(*id);
        }
    }
    output
}
