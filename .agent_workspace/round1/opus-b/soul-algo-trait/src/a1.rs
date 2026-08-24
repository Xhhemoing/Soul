//! **A1 — evidence-count band upgrade.** A0 plus one idea: several independent
//! rows that agree are worth more than one row.
//!
//! A0 replays the log and keeps the last row standing. A1 aggregates the whole
//! surviving set instead, which is what lets three separate observations reach
//! `Strong` while a single questionnaire answer never can. The lock is
//! untouched: a correction still wins, and no amount of counting overrides it.
//!
//! # The rules, stated so a user could check them by hand
//!
//! 1. **Drop forgotten rows.** Same as A0; a forget must be recomputable.
//! 2. **Independence is by `evidence_id`.** The same row cited twice is one
//!    piece of evidence, so duplicate ids are collapsed, first occurrence
//!    kept. This is the whole definition of "independent" — A1 does not try to
//!    tell whether two different rows share a cause, because it cannot, and
//!    pretending otherwise would be a weight in disguise.
//! 3. **A correction wins.** If any surviving row is a correction, the axis
//!    takes the *last* correction's position at `Strong`, locked, citing every
//!    correction that agrees with it. Everything else that survived is
//!    returned in the shadow as [`ApplyResult::RefusedLocked`] — including
//!    inferences recorded *before* the correction, which is where A1 differs
//!    from A0: A1 has no notion of "already applied", it re-derives the axis
//!    from the whole set every time.
//! 4. **Disagreement is not resolved by counting.** If one surviving row
//!    claims `leans_low` and another claims `leans_high` — or if any row
//!    claims `mixed` outright — the axis is [`Position::Mixed`] at
//!    [`Band::Weak`]. Outvoting the user's own answers by tally would be a
//!    score with the numbers hidden, so [`DisagreementPolicy::StrictMixed`] is
//!    the default. [`DisagreementPolicy::Majority`] and
//!    [`DisagreementPolicy::BandFloored`] are the two other readings of
//!    "majority position", kept explicit and opt-in so a later round can
//!    compare them on fixtures rather than argue about them; under all three
//!    the band is capped at `Moderate`, because a contradicted claim is never
//!    strong.
//! 5. **Band upgrade.** With one direction standing: three or more independent
//!    rows, each worth at least `Moderate` on its own, agreeing on that
//!    direction → `Strong`. Otherwise the band is the strongest single
//!    surviving row, capped at `Moderate`. The cap is what stops A1 inventing
//!    `Strong` out of one questionnaire answer, or out of one machine
//!    inference that labelled itself `Strong`.
//! 6. **Nothing at all** → `Unknown` / `Band::None`.
//!
//! Rows claiming `Position::Unknown` support nothing and are dropped before
//! aggregation, unless they are corrections — a user saying "you cannot tell"
//! is a correction like any other, and locks the axis at `Unknown`.

use crate::types::{
    ApplyResult, AxisId, AxisOutcome, AxisState, Band, EvidenceRef, Position, ShadowInference,
};

/// Identifier A1 stamps on the states it produces.
pub const A1_ALGORITHM_ID: &str = "a1.evidence_count_band_upgrade.v1";

/// How many independent agreeing rows it takes to reach `Strong`.
pub const A1_INDEPENDENT_FOR_STRONG: usize = 3;

/// The band each of those rows must be worth on its own to count toward the
/// upgrade.
pub const A1_UPGRADE_FLOOR: Band = Band::Moderate;

/// What to do when the surviving evidence contradicts itself.
///
/// Three readings of the same clause, kept side by side so a later round can
/// compare them on fixtures instead of on opinions. All three cap the result
/// at `Moderate`: a contradicted claim is never strong, whichever way it is
/// settled, and an explicit `mixed` row forces `Mixed` under all of them
/// because it is a statement rather than a vote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisagreementPolicy {
    /// Any contradiction yields `Mixed` / `Weak`. The default, and the reading
    /// this crate recommends: it never lets a tally overrule the user.
    StrictMixed,
    /// The side with strictly more independent rows wins; a tie yields
    /// `Mixed` / `Weak`.
    Majority,
    /// The side whose best single row is worth more wins; a tie on band yields
    /// `Mixed` / `Weak`.
    ///
    /// This is the reading that stops one `Weak` machine guess from dragging a
    /// `Moderate` questionnaire answer to `Mixed`, without ever letting a
    /// count outrank the strength of what was actually observed. It is opt-in
    /// rather than default because it is a change to the baseline's meaning,
    /// not a bug fix.
    BandFloored,
}

/// Aggregate one axis under A1 with the default (strict) disagreement policy.
pub fn a1_axis_state(axis: AxisId, evidence: &[EvidenceRef]) -> AxisOutcome {
    a1_axis_state_with(axis, evidence, DisagreementPolicy::StrictMixed)
}

/// Aggregate all five axes under A1, in questionnaire order.
pub fn a1_all_axes(evidence: &[EvidenceRef]) -> Vec<AxisOutcome> {
    AxisId::ALL
        .iter()
        .map(|axis| a1_axis_state(*axis, evidence))
        .collect()
}

/// Aggregate one axis under A1, choosing how contradictions are handled.
pub fn a1_axis_state_with(
    axis: AxisId,
    evidence: &[EvidenceRef],
    policy: DisagreementPolicy,
) -> AxisOutcome {
    let surviving = independent_rows(axis, evidence);
    let mut state = AxisState::unknown(axis, A1_ALGORITHM_ID);
    let mut shadow = Vec::new();

    let corrections: Vec<&EvidenceRef> = surviving
        .iter()
        .copied()
        .filter(|row| row.locked_by_user)
        .collect();

    if let Some(last) = corrections.last() {
        let position = last.position;
        let cited: Vec<u64> = corrections
            .iter()
            .filter(|row| row.position == position)
            .map(|row| row.evidence_id)
            .collect();

        state.position = position;
        state.band = if position == Position::Unknown {
            Band::None
        } else {
            Band::Strong
        };
        state.locked_by_user = true;
        state.evidence_ids = cited;

        for row in &surviving {
            let result = if row.locked_by_user && row.position == position {
                ApplyResult::Applied
            } else {
                ApplyResult::RefusedLocked
            };
            shadow.push(entry(row, result));
        }

        return AxisOutcome {
            state: state.clone(),
            shadow: with_citations(shadow, &state),
        };
    }

    let low: Vec<u64> = ids_claiming(&surviving, Position::LeansLow);
    let high: Vec<u64> = ids_claiming(&surviving, Position::LeansHigh);
    let both: Vec<u64> = ids_claiming(&surviving, Position::Mixed);

    let claims_low = !low.is_empty() || !both.is_empty();
    let claims_high = !high.is_empty() || !both.is_empty();

    let (position, supporters) = if claims_low && claims_high {
        match settle(policy, &surviving, &low, &high, &both) {
            Some(Position::LeansLow) => (Position::LeansLow, low),
            Some(Position::LeansHigh) => (Position::LeansHigh, high),
            _ => (
                Position::Mixed,
                in_input_order(&surviving, &[&low, &high, &both]),
            ),
        }
    } else if claims_low {
        (Position::LeansLow, low)
    } else if claims_high {
        (Position::LeansHigh, high)
    } else {
        let shadow = surviving
            .iter()
            .map(|row| entry(row, ApplyResult::Applied))
            .collect();
        return AxisOutcome { state, shadow };
    };

    state.band = match position {
        // A contradiction is a thin result however it is settled: `Weak` when
        // it stands as `Mixed`, capped at `Moderate` when a policy picks a
        // side.
        Position::Mixed => Band::Weak,
        _ if claims_low && claims_high => strongest_single(&surviving, &supporters),
        _ => upgraded_band(&surviving, &supporters),
    };
    state.position = position;
    state.evidence_ids = supporters;

    for row in &surviving {
        shadow.push(entry(row, ApplyResult::Applied));
    }

    AxisOutcome {
        state: state.clone(),
        shadow: with_citations(shadow, &state),
    }
}

/// Non-forgotten rows for this axis, one per `evidence_id`, first occurrence
/// kept. Rows claiming no direction are dropped unless they are corrections.
fn independent_rows(axis: AxisId, evidence: &[EvidenceRef]) -> Vec<&EvidenceRef> {
    let mut seen: Vec<u64> = Vec::new();
    let mut rows: Vec<&EvidenceRef> = Vec::new();
    for row in evidence {
        if row.axis != axis || row.forgotten {
            continue;
        }
        if row.position == Position::Unknown && !row.locked_by_user {
            continue;
        }
        if seen.contains(&row.evidence_id) {
            continue;
        }
        seen.push(row.evidence_id);
        rows.push(row);
    }
    rows
}

/// Which side, if any, a policy lets win a contradiction.
///
/// `None` means the axis stays `Mixed`. An explicit `mixed` row is never
/// settled away, whatever the policy.
fn settle(
    policy: DisagreementPolicy,
    rows: &[&EvidenceRef],
    low: &[u64],
    high: &[u64],
    both: &[u64],
) -> Option<Position> {
    if !both.is_empty() {
        return None;
    }

    let (low_side, high_side) = match policy {
        DisagreementPolicy::StrictMixed => return None,
        DisagreementPolicy::Majority => (low.len(), high.len()),
        DisagreementPolicy::BandFloored => (
            best_effective(rows, low).rank() as usize,
            best_effective(rows, high).rank() as usize,
        ),
    };

    match low_side.cmp(&high_side) {
        core::cmp::Ordering::Greater => Some(Position::LeansLow),
        core::cmp::Ordering::Less => Some(Position::LeansHigh),
        core::cmp::Ordering::Equal => None,
    }
}

fn ids_claiming(rows: &[&EvidenceRef], position: Position) -> Vec<u64> {
    rows.iter()
        .filter(|row| row.position == position)
        .map(|row| row.evidence_id)
        .collect()
}

fn in_input_order(rows: &[&EvidenceRef], groups: &[&Vec<u64>]) -> Vec<u64> {
    rows.iter()
        .map(|row| row.evidence_id)
        .filter(|id| groups.iter().any(|group| group.contains(id)))
        .collect()
}

/// The strongest single row among `supporters`.
fn best_effective(rows: &[&EvidenceRef], supporters: &[u64]) -> Band {
    rows.iter()
        .filter(|row| supporters.contains(&row.evidence_id))
        .fold(Band::None, |best, row| {
            Band::max_of(best, row.effective_band())
        })
}

/// The strongest single supporting row, capped at `Moderate`.
fn strongest_single(rows: &[&EvidenceRef], supporters: &[u64]) -> Band {
    best_effective(rows, supporters).capped_at(Band::Moderate)
}

/// `Strong` once enough independent rows each worth at least `Moderate` agree.
fn upgraded_band(rows: &[&EvidenceRef], supporters: &[u64]) -> Band {
    let qualifying = rows
        .iter()
        .filter(|row| supporters.contains(&row.evidence_id))
        .filter(|row| row.effective_band().at_least(A1_UPGRADE_FLOOR))
        .count();

    if qualifying >= A1_INDEPENDENT_FOR_STRONG {
        Band::Strong
    } else {
        strongest_single(rows, supporters)
    }
}

fn entry(row: &EvidenceRef, result: ApplyResult) -> ShadowInference {
    ShadowInference {
        evidence_id: row.evidence_id,
        axis: row.axis,
        position: row.position,
        band: row.effective_band(),
        result,
        cited: false,
    }
}

fn with_citations(mut shadow: Vec<ShadowInference>, state: &AxisState) -> Vec<ShadowInference> {
    for entry in &mut shadow {
        entry.cited = state.evidence_ids.contains(&entry.evidence_id);
    }
    shadow
}
