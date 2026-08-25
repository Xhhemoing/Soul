//! **A1 — independent-group band upgrade.** A0 plus one idea: several
//! *independent* observations that agree are worth more than one observation
//! repeated.
//!
//! Round 1 left A1 counting rows, which is the version that F16 kills: five
//! re-fills of the same questionnaire on the same afternoon are five rows and
//! one fact. Round 2 adopted the independence key `CANDIDATE_SPEC.md` froze.
//!
//! # Round 3 status: A1 is not a retained algorithm
//!
//! `R2-SYNTHESIS.md` keeps exactly two things for v0.1: a tie-strength
//! algorithm, and **A0**. A1 is neither of them. It has no data plane of its
//! own, no output type of its own, and cannot exist without A0's evidence log —
//! it is an aggregation rule that A0 could adopt later, and it ships here so
//! that the rule is written down, tested and inert rather than reinvented from
//! memory in v0.2. Nothing in this crate's frozen path calls it: `a0.rs` does
//! not import it, and the v0.1 profile is [`crate::a0::a0_all_axes`].
//!
//! # Round 3 changed the default: `TwoKindsAcrossDays`
//!
//! Round 2 shipped `(kind, utc_day)` alone as the default and reported
//! `EMPTY_ON_V01 = false`: three questionnaire re-fills on three different days
//! were three independent groups, so answering the same question three times
//! reached `Strong`. That is a degenerate upgrade. Independence is a property
//! of the **instrument**, not of the calendar; the same person recalling the
//! same thing from the same questionnaire on a Tuesday and again in March is
//! one observation made three times.
//!
//! So [`A1_DEFAULT_INDEPENDENCE`] is now [`A1Independence::TwoKindsAcrossDays`]
//! and [`A1_EMPTY_ON_V01`] is `true`: on a v0.1 install A1 provably cannot
//! fire, because a v0.1 install can only produce `questionnaire` and
//! `user_correction` rows against an axis (see
//! [`crate::types::EvidenceKind::reachable_for_axis_in_v01`]) and a correction
//! locks the axis before any counting happens, so the second kind the upgrade
//! needs is unreachable. `a1_independence.rs` proves both halves rather than
//! asserting them.
//!
//! [`A1Independence::KindAndDay`] — the Round 2 default, and the literal
//! reading of `CANDIDATE_SPEC.md` — stays as a named alternative on the enum so
//! that the ablation can still be run and the divergence stays visible. It is
//! not reachable without naming it.
//!
//! # The rules, stated so a user could check them by hand
//!
//! 1. **Drop forgotten rows.** Same as A0; a forget must be recomputable.
//! 2. **A correction wins, always.** If any surviving row is a correction the
//!    axis takes the last correction's position at `Strong`, locked. No amount
//!    of counting touches it, and everything else is returned in the shadow as
//!    [`ApplyResult::RefusedLocked`].
//! 3. **Independence is `(kind, utc_day)`.** Rows are grouped by where they
//!    came from and which UTC day they were recorded on. Any number of rows
//!    sharing a key is **one group**. This is the whole definition; A1 does not
//!    try to tell whether two rows from different sources share a cause,
//!    because it cannot, and pretending otherwise would be a weight in
//!    disguise.
//! 4. **Three independent agreeing groups spanning at least two kinds →
//!    `Strong`** ([`A1_GROUPS_FOR_STRONG`], [`A1_DEFAULT_INDEPENDENCE`]).
//! 5. **Disagreement is not resolved by counting.** One group in each
//!    direction is already a disagreement: the axis is [`Position::Mixed`] at
//!    [`Band::Weak`], and so is any axis with a group that contradicts itself.
//!    This is deliberately more conservative than the frozen spec, which only
//!    called it mixed at two groups a side — see `REPORT.md`. Outvoting the
//!    user by tally would be a score with the numbers hidden.
//! 6. **Otherwise** the band is the strongest single surviving row, capped at
//!    `Moderate`. The cap is what stops one questionnaire answer, or one
//!    machine inference that labelled itself `Strong`, from reaching `Strong`
//!    on its own.
//! 7. **Nothing at all** → `Unknown` / `Band::None`.
//!
//! Rows claiming `Position::Unknown` support nothing and are dropped before
//! grouping, unless they are corrections — a user saying "you cannot tell" is a
//! correction like any other, and locks the axis at `Unknown`.
//!
//! # There is no per-row band floor, on purpose
//!
//! [`A1_GROUP_FLOOR`] is `Band::Weak`, which is to say every directional group
//! counts. Three independent `Weak` groups reach `Strong`. That is the frozen
//! spec's reading and F16b's survival condition, and the argument for it is
//! that **independence, not per-row strength, is the guard**: under the
//! `(kind, utc_day)` key, three `Weak` groups already mean three different days
//! or three different sources, which is not the failure mode anybody was
//! worried about. The constant is named and public so a later round can raise
//! it without hunting for the comparison.

use crate::types::{
    ApplyResult, AxisId, AxisState, Band, EvidenceKind, EvidenceRef, Position, ShadowInference,
};

/// Identifier A1 stamps on the states it produces.
///
/// `v3` because Round 3 changed the default independence variant, which changes
/// what the same evidence log produces. A stamped state names the rule that
/// produced it, so the rule changing has to change the stamp.
pub const A1_ALGORITHM_ID: &str = "a1.independent_group_upgrade.v3";

/// How many independent agreeing groups it takes to reach `Strong`.
pub const A1_GROUPS_FOR_STRONG: usize = 3;

/// What A1 counts as independent unless a caller says otherwise.
///
/// [`A1Independence::TwoKindsAcrossDays`] as of Round 3. See the module docs.
pub const A1_DEFAULT_INDEPENDENCE: A1Independence = A1Independence::TwoKindsAcrossDays;

/// Whether A1 is inert on a v0.1 install under [`A1_DEFAULT_INDEPENDENCE`].
///
/// `true`, and it is a claim with a proof rather than a note: see
/// [`a1_can_fire_on_v01_data`], which derives it from
/// [`crate::types::EvidenceKind::reachable_for_axis_in_v01`] instead of
/// restating it.
pub const A1_EMPTY_ON_V01: bool = true;

/// The band a group must be worth for it to count toward the upgrade.
///
/// `Weak`, i.e. no floor. See the module docs for why.
pub const A1_GROUP_FLOOR: Band = Band::Weak;

/// How many groups on each side make a disagreement.
///
/// One. `CANDIDATE_SPEC.md` froze two; this crate is stricter, and `REPORT.md`
/// records the divergence.
pub const A1_GROUPS_FOR_DISAGREEMENT: usize = 1;

/// What counts as two independent observations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum A1Independence {
    /// `(kind, utc_day)` **and** the upgrading groups must span at least two
    /// distinct kinds. **The Round 3 default.**
    ///
    /// The frozen key, plus a requirement that the machine has heard about the
    /// axis from more than one direction. This is the variant under which A1
    /// provably cannot fire on a v0.1 install, because a v0.1 install can only
    /// produce `questionnaire` rows and `user_correction` rows against an axis,
    /// and a correction locks the axis before any counting happens.
    ///
    /// It costs nothing that anyone wanted: F16b — a questionnaire answer plus
    /// behavioural rows on three separate days — still reaches `Strong`,
    /// because that is two kinds. What it stops is the questionnaire upgrading
    /// itself by being answered again on another day.
    #[default]
    TwoKindsAcrossDays,
    /// `(kind, utc_day)` alone, exactly as `CANDIDATE_SPEC.md` froze it and as
    /// Round 2 shipped it. Two rows are independent when they come from
    /// different sources **or** were recorded on different UTC days.
    ///
    /// **A named alternative, not the default.** Kept so that the Round 2
    /// reading can still be run side by side in the ablation, and so that the
    /// divergence from `CANDIDATE_SPEC.md` is a value a reader can find rather
    /// than a deleted branch. Under it `EMPTY_ON_V01` is `false`: three
    /// questionnaire re-fills on three days reach `Strong` with no behavioural
    /// evidence at all.
    KindAndDay,
}

impl A1Independence {
    /// Both variants, default first.
    pub const ALL: [A1Independence; 2] = [
        A1Independence::TwoKindsAcrossDays,
        A1Independence::KindAndDay,
    ];

    /// A stable name for a report row or an audit line.
    pub fn as_str(self) -> &'static str {
        match self {
            A1Independence::TwoKindsAcrossDays => "two_kinds_across_days",
            A1Independence::KindAndDay => "kind_and_day",
        }
    }
}

/// Whether A1 could reach `Strong` on a v0.1 install under this variant,
/// without a user correction.
///
/// Derived rather than asserted. The v0.1 data plane is whatever
/// [`crate::types::EvidenceKind::reachable_for_axis_in_v01`] admits, minus
/// `UserCorrection`, which locks the axis before any counting happens and so
/// never reaches the grouping stage at all. What is left is the set of kinds
/// that can ever appear in a qualifying group; if it holds fewer than the
/// variant requires, no arrangement of rows can upgrade.
///
/// Adding a third v0.1 producer therefore flips this function rather than
/// quietly invalidating a sentence in a document.
pub fn a1_can_fire_on_v01_data(independence: A1Independence) -> bool {
    let countable_kinds = EvidenceKind::ALL
        .iter()
        .filter(|kind| kind.reachable_for_axis_in_v01())
        .filter(|kind| **kind != EvidenceKind::UserCorrection)
        .count();

    match independence {
        // Days are unlimited, so one countable kind is enough to build
        // `A1_GROUPS_FOR_STRONG` groups.
        A1Independence::KindAndDay => countable_kinds >= 1,
        A1Independence::TwoKindsAcrossDays => countable_kinds >= 2,
    }
}

/// What makes two observations independent of each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndependenceKey {
    /// Where the observation came from.
    pub kind: EvidenceKind,
    /// The UTC day it was recorded on, counted from the epoch.
    pub utc_day: i64,
}

/// One independent group: everything that shares an [`IndependenceKey`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceGroup {
    /// What makes this group one group.
    pub key: IndependenceKey,
    /// What the group claims. [`Position::Mixed`] when the rows inside it do
    /// not agree with each other — a source that contradicts itself in a day
    /// is not evidence for either side.
    pub position: Position,
    /// The strongest single row in the group.
    pub band: Band,
    /// The rows in the group, in input order. Never empty.
    pub evidence_ids: Vec<u64>,
}

/// Why A1 landed where it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum A1Reason {
    /// Nothing survived to aggregate.
    NoEvidence,
    /// A user correction settled it.
    UserLocked,
    /// Both directions had at least [`A1_GROUPS_FOR_DISAGREEMENT`] group, or a
    /// group contradicted itself.
    Contradicted,
    /// Enough independent groups agreed.
    UpgradedByIndependentGroups {
        /// How many agreed.
        groups: usize,
    },
    /// One direction, but not enough independent groups to upgrade.
    NotEnoughIndependentGroups {
        /// How many agreed.
        groups: usize,
    },
    /// One direction, enough groups, but all of them came from the same kind of
    /// source, so [`A1Independence::TwoKindsAcrossDays`] did not upgrade.
    ///
    /// This is the reason the v0.1 questionnaire always lands on, and it is
    /// separate from [`A1Reason::NotEnoughIndependentGroups`] because the two
    /// say different things to a user: "answer more questions" would be wrong
    /// advice here, and the explanation has to be able to say so.
    NeedsASecondSource {
        /// How many agreed.
        groups: usize,
    },
}

/// What A1 returns: the state, every observation it read, and the grouping it
/// used to get there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct A1Outcome {
    /// The axis as it now stands.
    pub state: AxisState,
    /// Every non-forgotten, non-duplicate observation, in input order.
    pub shadow: Vec<ShadowInference>,
    /// The independent groups, in first-appearance order. Empty when a
    /// correction settled the axis: no counting happened.
    pub groups: Vec<EvidenceGroup>,
    /// Why the band is what it is.
    pub reason: A1Reason,
}

impl A1Outcome {
    /// The groups that support the position the axis is in now.
    pub fn supporting_groups(&self) -> Vec<&EvidenceGroup> {
        self.groups
            .iter()
            .filter(|group| group.position == self.state.position)
            .collect()
    }

    /// How many independent groups back the current position.
    pub fn supporting_group_count(&self) -> usize {
        self.supporting_groups().len()
    }

    /// The observations that did not reach the axis.
    pub fn refused_ids(&self) -> Vec<u64> {
        self.shadow
            .iter()
            .filter(|entry| entry.result != ApplyResult::Applied)
            .map(|entry| entry.evidence_id)
            .collect()
    }

    /// Why the axis reads the way it does, in words a user can check against
    /// their own history. Counts of groups and days; no score, no date
    /// formatting, no third-party anything.
    pub fn explain_zh(&self) -> String {
        match self.reason {
            A1Reason::NoEvidence => "这条还没有任何依据。".to_owned(),
            A1Reason::UserLocked => {
                "这一条来自你本人的纠正，已锁定，之后的推断不会覆盖它。".to_owned()
            }
            A1Reason::Contradicted => {
                "两个方向都有依据，所以先记成「两端都有」，不按条数取多数。".to_owned()
            }
            A1Reason::UpgradedByIndependentGroups { groups } => format!(
                "这个方向有 {groups} 组互相独立的依据（来自 {} 个不同日子、{} 类来源），所以从中等升为强。",
                self.distinct_supporting_days(),
                self.distinct_supporting_kinds(),
            ),
            A1Reason::NotEnoughIndependentGroups { groups } => format!(
                "这个方向目前只有 {groups} 组独立依据，还不够升为强；同一天同一来源再多几条也只算一组。"
            ),
            A1Reason::NeedsASecondSource { groups } => format!(
                "这个方向有 {groups} 组依据，但都来自同一种来源；换个日子把同一份问卷再答一遍不算新的旁证，所以先留在中等。"
            ),
        }
    }

    /// How many distinct UTC days back the current position.
    pub fn distinct_supporting_days(&self) -> usize {
        let mut days: Vec<i64> = Vec::new();
        for group in self.supporting_groups() {
            if !days.contains(&group.key.utc_day) {
                days.push(group.key.utc_day);
            }
        }
        days.len()
    }

    /// How many distinct evidence kinds back the current position.
    pub fn distinct_supporting_kinds(&self) -> usize {
        distinct_kinds(&self.supporting_groups())
    }
}

/// Aggregate one axis under A1 with [`A1_DEFAULT_INDEPENDENCE`].
pub fn a1_axis_state(axis: AxisId, evidence: &[EvidenceRef]) -> A1Outcome {
    a1_axis_state_with(axis, evidence, A1_DEFAULT_INDEPENDENCE)
}

/// Aggregate all five axes under A1, in questionnaire order.
pub fn a1_all_axes(evidence: &[EvidenceRef]) -> Vec<A1Outcome> {
    AxisId::ALL
        .iter()
        .map(|axis| a1_axis_state(*axis, evidence))
        .collect()
}

/// Aggregate one axis under A1, choosing what counts as independent.
pub fn a1_axis_state_with(
    axis: AxisId,
    evidence: &[EvidenceRef],
    independence: A1Independence,
) -> A1Outcome {
    let surviving = surviving_rows(axis, evidence);
    let mut state = AxisState::unknown(axis, A1_ALGORITHM_ID);

    if surviving.is_empty() {
        return A1Outcome {
            state,
            shadow: Vec::new(),
            groups: Vec::new(),
            reason: A1Reason::NoEvidence,
        };
    }

    // Rule 2: a correction settles the axis before any counting happens.
    let corrections: Vec<&EvidenceRef> = surviving
        .iter()
        .copied()
        .filter(|row| row.locked_by_user)
        .collect();

    if let Some(last) = corrections.last() {
        let position = last.position;
        state.position = position;
        state.band = if position == Position::Unknown {
            Band::None
        } else {
            Band::Strong
        };
        state.locked_by_user = true;
        state.evidence_ids = corrections
            .iter()
            .filter(|row| row.position == position)
            .map(|row| row.evidence_id)
            .collect();

        let shadow = surviving
            .iter()
            .map(|row| {
                let result = if row.locked_by_user && row.position == position {
                    ApplyResult::Applied
                } else {
                    ApplyResult::RefusedLocked
                };
                entry(row, result)
            })
            .collect();

        return A1Outcome {
            state: state.clone(),
            shadow: cite(shadow, &state),
            groups: Vec::new(),
            reason: A1Reason::UserLocked,
        };
    }

    // Rule 3: group by (kind, utc_day).
    let groups = group_rows(&surviving);

    let low = groups_claiming(&groups, Position::LeansLow);
    let high = groups_claiming(&groups, Position::LeansHigh);
    let self_contradicting = groups_claiming(&groups, Position::Mixed);

    let contradicted = !self_contradicting.is_empty()
        || (low.len() >= A1_GROUPS_FOR_DISAGREEMENT && high.len() >= A1_GROUPS_FOR_DISAGREEMENT);

    let (position, supporting) = if contradicted {
        // Rule 5. Everything that survived is cited: the user is owed the whole
        // picture when the picture is the disagreement.
        (Position::Mixed, groups.iter().collect::<Vec<_>>())
    } else if !low.is_empty() {
        (Position::LeansLow, low)
    } else if !high.is_empty() {
        (Position::LeansHigh, high)
    } else {
        let shadow = surviving
            .iter()
            .map(|row| entry(row, ApplyResult::Applied))
            .collect();
        return A1Outcome {
            state,
            shadow,
            groups,
            reason: A1Reason::NoEvidence,
        };
    };

    let qualifying: Vec<&EvidenceGroup> = supporting
        .iter()
        .copied()
        .filter(|group| group.band.at_least(A1_GROUP_FLOOR))
        .collect();

    let (band, reason) = if contradicted {
        (Band::Weak, A1Reason::Contradicted)
    } else if upgrades(&qualifying, independence) {
        (
            Band::Strong,
            A1Reason::UpgradedByIndependentGroups {
                groups: qualifying.len(),
            },
        )
    } else {
        (
            strongest(&supporting).capped_at(Band::Moderate),
            not_upgraded(&qualifying),
        )
    };

    state.position = position;
    state.band = band;
    state.evidence_ids = in_input_order(&surviving, &supporting);

    let shadow = surviving
        .iter()
        .map(|row| entry(row, ApplyResult::Applied))
        .collect();

    A1Outcome {
        state: state.clone(),
        shadow: cite(shadow, &state),
        groups,
        reason,
    }
}

// ------------------------------------------------------------- internals ---

/// Non-forgotten rows for this axis, one per `evidence_id`, first occurrence
/// kept. Rows claiming no direction are dropped unless they are corrections.
fn surviving_rows(axis: AxisId, evidence: &[EvidenceRef]) -> Vec<&EvidenceRef> {
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

/// Collapse rows into independent groups, in first-appearance order.
fn group_rows(rows: &[&EvidenceRef]) -> Vec<EvidenceGroup> {
    let mut groups: Vec<EvidenceGroup> = Vec::new();

    for row in rows {
        let key = IndependenceKey {
            kind: row.kind,
            utc_day: row.utc_day(),
        };

        match groups.iter_mut().find(|group| group.key == key) {
            Some(group) => {
                if group.position != row.position {
                    group.position = Position::Mixed;
                }
                group.band = Band::max_of(group.band, row.effective_band());
                group.evidence_ids.push(row.evidence_id);
            }
            None => groups.push(EvidenceGroup {
                key,
                position: row.position,
                band: row.effective_band(),
                evidence_ids: vec![row.evidence_id],
            }),
        }
    }

    groups
}

fn groups_claiming(groups: &[EvidenceGroup], position: Position) -> Vec<&EvidenceGroup> {
    groups
        .iter()
        .filter(|group| group.position == position)
        .collect()
}

fn distinct_kinds(groups: &[&EvidenceGroup]) -> usize {
    let mut kinds: Vec<EvidenceKind> = Vec::new();
    for group in groups {
        if !kinds.contains(&group.key.kind) {
            kinds.push(group.key.kind);
        }
    }
    kinds.len()
}

/// Whether these groups are enough to reach `Strong`.
fn upgrades(qualifying: &[&EvidenceGroup], independence: A1Independence) -> bool {
    if qualifying.len() < A1_GROUPS_FOR_STRONG {
        return false;
    }
    match independence {
        A1Independence::KindAndDay => true,
        A1Independence::TwoKindsAcrossDays => distinct_kinds(qualifying) >= 2,
    }
}

/// Which of the two "did not upgrade" reasons applies: too few groups, or
/// enough groups that all say the same thing because they all came from the
/// same instrument.
fn not_upgraded(qualifying: &[&EvidenceGroup]) -> A1Reason {
    let groups = qualifying.len();
    if groups >= A1_GROUPS_FOR_STRONG {
        A1Reason::NeedsASecondSource { groups }
    } else {
        A1Reason::NotEnoughIndependentGroups { groups }
    }
}

fn strongest(groups: &[&EvidenceGroup]) -> Band {
    groups
        .iter()
        .fold(Band::None, |best, group| Band::max_of(best, group.band))
}

fn in_input_order(rows: &[&EvidenceRef], groups: &[&EvidenceGroup]) -> Vec<u64> {
    rows.iter()
        .map(|row| row.evidence_id)
        .filter(|id| groups.iter().any(|group| group.evidence_ids.contains(id)))
        .collect()
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

fn cite(mut shadow: Vec<ShadowInference>, state: &AxisState) -> Vec<ShadowInference> {
    for entry in &mut shadow {
        entry.cited = state.evidence_ids.contains(&entry.evidence_id);
    }
    shadow
}
