//! Round X: `A1_EMPTY_ON_V01` by exhaustion over the v0.1 row alphabet.
//!
//! The claim already has two supports in the crate, and neither of them runs
//! the algorithm over the whole space it is a claim about:
//!
//! - `a1_can_fire_on_v01_data` counts the evidence kinds a v0.1 install can
//!   write against an axis and compares that count to what the variant needs.
//!   It is a statement about `EvidenceKind`, not about `a1_axis_state`;
//!   `empty_on_v01_is_true_under_the_default_and_false_under_the_alternative`
//!   pins the constant against it, so the two agree with each other and neither
//!   touches the aggregation.
//! - `no_v01_reachable_log_reaches_strong_without_a_correction` does run the
//!   aggregation, but only over the fifteen hand-written logs in
//!   `axis_evidence_matrix`, none of which was written to attack the upgrade
//!   branch.
//!
//! So the gap is the join: nothing checks that the kind-counting argument and
//! the code that actually decides the band agree for every log a v0.1 install
//! could produce. That space is small enough to enumerate, because the alphabet
//! is small: `only_two_evidence_kinds_can_reach_an_axis_in_v01` pins it at
//! questionnaire and correction, and a row is then fixed by its position and
//! the UTC day it landed on.
//!
//! Forgotten rows are left out of the enumeration on purpose. A forget removes
//! a row before grouping, so a log of length n with a forgotten row aggregates
//! exactly as the log of length n-1 without it, and the shorter log is already
//! enumerated here.

use soul_algo_trait::a1::{
    a1_axis_state, a1_can_fire_on_v01_data, A1Reason, A1_DEFAULT_INDEPENDENCE, A1_EMPTY_ON_V01,
    A1_GROUPS_FOR_STRONG,
};
use soul_algo_trait::fixtures::{DAY, FIXTURE_DAY_ZERO_UNIX};
use soul_algo_trait::types::{AxisId, Band, EvidenceKind, EvidenceRef, Position};

const AXIS: AxisId = AxisId::SocialEnergy;

/// The published claim, asserted at compile time so that flipping the constant
/// stops this file building rather than changing what it proves.
/// Constant on purpose: the point is that it stops compiling, which is what
/// `clippy::assertions_on_constants` is warning about.
#[allow(clippy::assertions_on_constants)]
const _: () = assert!(A1_EMPTY_ON_V01);

/// Distinct UTC days a generated log may use. Three, because
/// `A1_GROUPS_FOR_STRONG` is three: with three days available a log of length
/// three can already present the algorithm with the exact number of independent
/// groups the upgrade asks for, and a fourth row on top of that is what a
/// fourth position in the log buys.
const DAYS: [i64; 3] = [0, 1, 2];

const POSITIONS: [Position; 4] = [
    Position::LeansLow,
    Position::Mixed,
    Position::LeansHigh,
    Position::Unknown,
];

/// One row, minus its id: what a v0.1 producer chooses when it writes.
#[derive(Clone, Copy)]
struct Template {
    kind: EvidenceKind,
    position: Position,
    day: i64,
}

/// Every row shape a v0.1 install can write against a trait axis.
///
/// The kinds come from `EvidenceKind::reachable_for_axis_in_v01` rather than
/// from a list here, so a third producer widens this enumeration instead of
/// slipping past it.
fn v01_alphabet() -> Vec<Template> {
    let mut alphabet = Vec::new();
    for kind in EvidenceKind::ALL {
        if !kind.reachable_for_axis_in_v01() {
            continue;
        }
        for position in POSITIONS {
            for day in DAYS {
                alphabet.push(Template {
                    kind,
                    position,
                    day,
                });
            }
        }
    }
    alphabet
}

/// Turn a sequence of templates into a log. Ids are the 1-based index, so no
/// two rows in one log collapse under A1's duplicate-id rule and every log of
/// length n really is n rows.
fn build(templates: &[Template]) -> Vec<EvidenceRef> {
    templates
        .iter()
        .enumerate()
        .map(|(index, template)| {
            let id = index as u64 + 1;
            let at = FIXTURE_DAY_ZERO_UNIX + template.day * DAY;
            match template.kind {
                EvidenceKind::UserCorrection => {
                    EvidenceRef::correction(id, AXIS, template.position, at)
                }
                _ => EvidenceRef::questionnaire(id, AXIS, template.position, at),
            }
        })
        .collect()
}

/// Call `visit` once for every sequence of length 1..=`max_len` over
/// `alphabet`, reusing one buffer so the enumeration does not allocate per log.
fn for_each_sequence(alphabet: &[Template], max_len: usize, mut visit: impl FnMut(&[Template])) {
    fn recurse(
        alphabet: &[Template],
        max_len: usize,
        buffer: &mut Vec<Template>,
        visit: &mut impl FnMut(&[Template]),
    ) {
        if buffer.len() == max_len {
            return;
        }
        for template in alphabet {
            buffer.push(*template);
            visit(buffer);
            recurse(alphabet, max_len, buffer, visit);
            buffer.pop();
        }
    }

    let mut buffer = Vec::with_capacity(max_len);
    recurse(alphabet, max_len, &mut buffer, &mut visit);
}

fn describe(templates: &[Template]) -> String {
    templates
        .iter()
        .map(|template| {
            format!(
                "{}/{}/d{}",
                template.kind.as_str(),
                template.position.as_str(),
                template.day
            )
        })
        .collect::<Vec<_>>()
        .join(" + ")
}

#[test]
fn the_upgrade_branch_is_unreachable_on_every_v01_log_up_to_four_rows() {
    // The join the crate was missing: not "fewer than two kinds are reachable"
    // and not "these sixteen fixtures behave", but every log the v0.1 data
    // plane can produce, run through the aggregation that ships.
    //
    // Two things are asserted per log, and they are not the same thing. The
    // first is that A1 never takes the upgrade branch at all — that is what
    // "A1 is inert" means, and it is stronger than the band coming out right.
    // The second is the user-visible half: a `Strong` axis means a correction,
    // never a tally.
    let alphabet = v01_alphabet();
    assert_eq!(
        alphabet.len(),
        2 * POSITIONS.len() * DAYS.len(),
        "the v0.1 alphabet is two kinds wide; a third producer changes this test"
    );

    let mut logs = 0_u64;
    for_each_sequence(&alphabet, 4, |templates| {
        logs += 1;
        let log = build(templates);
        let outcome = a1_axis_state(AXIS, &log);

        assert!(
            !matches!(outcome.reason, A1Reason::UpgradedByIndependentGroups { .. }),
            "A1 fired on a v0.1 log: {}",
            describe(templates)
        );

        if outcome.state.band == Band::Strong {
            assert!(
                outcome.state.locked_by_user,
                "Strong without a correction: {}",
                describe(templates)
            );
            assert_eq!(
                outcome.reason,
                A1Reason::UserLocked,
                "Strong for a reason other than the user: {}",
                describe(templates)
            );
        }
    });

    assert_eq!(
        logs,
        24 + 576 + 13_824 + 331_776,
        "the whole space was walked"
    );
}

#[test]
fn more_questionnaire_days_never_help_however_many_there_are() {
    // The other axis of the same space, traded the other way: fewer rows per
    // log, more distinct days than the enumeration above can hold. Six
    // questionnaire answers on six separate days is twice the group count the
    // upgrade asks for, and it is still one instrument, so the answer has to be
    // the same one a single answer gets.
    const DAYS_APART: usize = 6;
    const _: () = assert!(DAYS_APART > A1_GROUPS_FOR_STRONG);

    let mut logs = 0_u64;
    let mut choice = [Position::LeansHigh; DAYS_APART];
    let total = POSITIONS.len().pow(DAYS_APART as u32);

    for index in 0..total {
        let mut remainder = index;
        for slot in choice.iter_mut() {
            *slot = POSITIONS[remainder % POSITIONS.len()];
            remainder /= POSITIONS.len();
        }

        let log: Vec<EvidenceRef> = choice
            .iter()
            .enumerate()
            .map(|(day, position)| {
                EvidenceRef::questionnaire(
                    day as u64 + 1,
                    AXIS,
                    *position,
                    FIXTURE_DAY_ZERO_UNIX + day as i64 * DAY,
                )
            })
            .collect();

        let outcome = a1_axis_state(AXIS, &log);
        logs += 1;

        assert!(
            !matches!(outcome.reason, A1Reason::UpgradedByIndependentGroups { .. }),
            "six questionnaire days upgraded: {choice:?}"
        );
        assert_ne!(outcome.state.band, Band::Strong, "{choice:?}");
    }

    assert_eq!(logs as usize, total);
}

#[test]
fn the_constant_the_derivation_and_the_run_are_all_the_same_claim() {
    // Three statements of one fact, tied together in one place: the published
    // constant, the kind-counting derivation, and the exhaustion above. Any
    // future change that flips one of them without the others has to fail here
    // rather than leave the crate half-migrated. The published constant is the
    // compile-time assertion at the top of this file; the derivation is here.
    assert!(
        !a1_can_fire_on_v01_data(A1_DEFAULT_INDEPENDENCE),
        "the derivation behind A1_EMPTY_ON_V01"
    );

    // And the counterexample that keeps the claim from being vacuous: the same
    // enumeration under the named alternative does reach the upgrade branch, so
    // "no log fires" is a property of the default rather than of the alphabet.
    let fires = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, FIXTURE_DAY_ZERO_UNIX),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, FIXTURE_DAY_ZERO_UNIX + DAY),
        EvidenceRef::questionnaire(
            3,
            AXIS,
            Position::LeansHigh,
            FIXTURE_DAY_ZERO_UNIX + 2 * DAY,
        ),
    ];
    assert!(matches!(
        soul_algo_trait::a1::a1_axis_state_with(
            AXIS,
            &fires,
            soul_algo_trait::a1::A1Independence::KindAndDay
        )
        .reason,
        A1Reason::UpgradedByIndependentGroups { .. }
    ));
    assert!(!matches!(
        a1_axis_state(AXIS, &fires).reason,
        A1Reason::UpgradedByIndependentGroups { .. }
    ));
}
