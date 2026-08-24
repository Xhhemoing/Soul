//! A1: the independence key, F16, F16b, and the conservative mixed rule.
//!
//! F16 and F16b are the death line and the survival line `EVAL_MATRIX.md` set
//! for this candidate. F16: five re-fills of the same questionnaire on one day
//! must not upgrade. F16b: a questionnaire answer plus three behavioural rows
//! on three separate days must.

use soul_algo_trait::a1::{
    a1_axis_state, a1_axis_state_with, a1_can_fire_on_v01_data, A1Independence, A1Reason,
    A1_DEFAULT_INDEPENDENCE, A1_EMPTY_ON_V01, A1_GROUPS_FOR_STRONG,
};
use soul_algo_trait::fixtures::{axis_evidence_matrix, DAY, FIXTURE_DAY_ZERO_UNIX};
use soul_algo_trait::types::{
    ApplyResult, AxisId, Band, EvidenceKind, EvidenceRef, Position, SECONDS_PER_DAY,
};

const AXIS: AxisId = AxisId::SocialEnergy;

fn day(n: i64) -> i64 {
    FIXTURE_DAY_ZERO_UNIX + n * DAY
}

fn fixture(name: &str) -> Vec<EvidenceRef> {
    axis_evidence_matrix()
        .into_iter()
        .find(|(key, _)| *key == name)
        .unwrap_or_else(|| panic!("no fixture named {name}"))
        .1
}

fn behavioural(id: u64, position: Position, at: i64) -> EvidenceRef {
    EvidenceRef::inference(id, AXIS, position, Band::Weak, EvidenceKind::Message, at)
}

// ------------------------------------------------------------------- F16 ---

#[test]
fn f16_five_refills_in_one_afternoon_do_not_upgrade() {
    let outcome = a1_axis_state(AXIS, &fixture("f16_five_refills_same_day"));

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(
        outcome.state.band,
        Band::Moderate,
        "five rows, one questionnaire, one day: one group"
    );
    assert_eq!(outcome.groups.len(), 1);
    assert_eq!(outcome.groups[0].evidence_ids, vec![1, 2, 3, 4, 5]);
    assert_eq!(outcome.supporting_group_count(), 1);
    assert!(matches!(
        outcome.reason,
        A1Reason::NotEnoughIndependentGroups { groups: 1 }
    ));
    assert_eq!(
        outcome.state.evidence_ids,
        vec![1, 2, 3, 4, 5],
        "all five are still cited; they are just not five reasons"
    );
}

#[test]
fn f16_holds_however_many_times_the_questionnaire_is_refilled_in_a_day() {
    let rows: Vec<EvidenceRef> = (1..=50)
        .map(|n| EvidenceRef::questionnaire(n as u64, AXIS, Position::LeansHigh, day(0) + n * 900))
        .collect();

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.groups.len(), 1);
    assert_eq!(outcome.state.band, Band::Moderate);
}

#[test]
fn a_day_boundary_is_a_utc_day_boundary() {
    // One second either side of midnight is two groups; 23 hours inside one day
    // is still one. The key has to be a calendar day or "same day" is a matter
    // of opinion.
    let midnight = day(10);
    let same_day = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, midnight),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, midnight + SECONDS_PER_DAY - 1),
    ];
    assert_eq!(a1_axis_state(AXIS, &same_day).groups.len(), 1);

    let across_midnight = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, midnight - 1),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, midnight),
    ];
    assert_eq!(a1_axis_state(AXIS, &across_midnight).groups.len(), 2);
}

#[test]
fn the_day_key_works_either_side_of_the_epoch() {
    let before = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, -1),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, -SECONDS_PER_DAY + 1),
    ];
    assert_eq!(
        a1_axis_state(AXIS, &before).groups.len(),
        1,
        "1969-12-31 is one day, not two half-days rounded toward the epoch"
    );

    let straddling = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, -1),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, 0),
    ];
    assert_eq!(a1_axis_state(AXIS, &straddling).groups.len(), 2);
}

// ------------------------------------------------------------------ F16b ---

#[test]
fn f16b_three_independent_agreeing_groups_reach_strong() {
    let outcome = a1_axis_state(
        AXIS,
        &fixture("f16b_questionnaire_plus_three_independent_days"),
    );

    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(outcome.groups.len(), 4);
    assert_eq!(outcome.supporting_group_count(), 4);
    assert_eq!(outcome.state.evidence_ids, vec![1, 2, 3, 4]);
    assert!(matches!(
        outcome.reason,
        A1Reason::UpgradedByIndependentGroups { groups: 4 }
    ));
}

#[test]
fn exactly_three_groups_is_the_upgrade_boundary() {
    // Three groups spanning two kinds, so the group count is the only thing
    // being tested here and the Round 3 kind requirement is already satisfied.
    let three = vec![
        behavioural(1, Position::LeansHigh, day(1)),
        behavioural(2, Position::LeansHigh, day(2)),
        EvidenceRef::questionnaire(3, AXIS, Position::LeansHigh, day(3)),
    ];
    assert_eq!(a1_axis_state(AXIS, &three).state.band, Band::Strong);
    assert_eq!(A1_GROUPS_FOR_STRONG, 3);

    let two = &three[..2];
    let outcome = a1_axis_state(AXIS, two);
    assert_eq!(outcome.state.band, Band::Weak, "strongest single row");
    assert!(matches!(
        outcome.reason,
        A1Reason::NotEnoughIndependentGroups { groups: 2 }
    ));
}

#[test]
fn three_different_sources_on_one_day_are_three_groups() {
    // The key is (kind, utc_day), not the day alone: a message observation and
    // an app-usage observation recorded the same afternoon are two things that
    // happened, not one thing counted twice.
    let rows = vec![
        EvidenceRef::inference(
            1,
            AXIS,
            Position::LeansHigh,
            Band::Weak,
            EvidenceKind::Message,
            day(1),
        ),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansHigh,
            Band::Weak,
            EvidenceKind::AppUsage,
            day(1) + 60,
        ),
        EvidenceRef::inference(
            3,
            AXIS,
            Position::LeansHigh,
            Band::Weak,
            EvidenceKind::Aggregate,
            day(1) + 120,
        ),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.groups.len(), 3);
    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(outcome.distinct_supporting_days(), 1);
    assert_eq!(outcome.distinct_supporting_kinds(), 3);
}

#[test]
fn a_forgotten_row_takes_its_group_with_it() {
    let outcome = a1_axis_state(AXIS, &fixture("three_agreeing_one_forgotten"));
    assert_eq!(outcome.groups.len(), 2);
    assert_eq!(outcome.state.band, Band::Moderate);
    assert!(!outcome.state.evidence_ids.contains(&3));
}

#[test]
fn one_row_cited_three_times_is_one_row() {
    let outcome = a1_axis_state(AXIS, &fixture("one_row_cited_three_times"));
    assert_eq!(outcome.groups.len(), 1);
    assert_eq!(outcome.state.evidence_ids, vec![9]);
    assert_eq!(outcome.state.band, Band::Moderate);
}

// ------------------------------------------------------- disagreement ---

#[test]
fn one_group_each_way_is_already_mixed() {
    let rows = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansLow, day(0)),
        behavioural(2, Position::LeansHigh, day(5)),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
    assert_eq!(outcome.reason, A1Reason::Contradicted);
    assert_eq!(
        outcome.state.evidence_ids,
        vec![1, 2],
        "the disagreement is the finding, so both sides are cited"
    );
}

#[test]
fn a_majority_never_settles_a_disagreement() {
    let rows = vec![
        behavioural(1, Position::LeansHigh, day(1)),
        behavioural(2, Position::LeansHigh, day(2)),
        behavioural(3, Position::LeansHigh, day(3)),
        behavioural(4, Position::LeansHigh, day(4)),
        EvidenceRef::questionnaire(5, AXIS, Position::LeansLow, day(9)),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(
        outcome.state.position,
        Position::Mixed,
        "four against one is still not a vote"
    );
    assert_eq!(outcome.state.band, Band::Weak);
}

#[test]
fn a_source_that_contradicts_itself_in_a_day_supports_neither_side() {
    let rows = vec![
        behavioural(1, Position::LeansHigh, day(1)),
        behavioural(2, Position::LeansLow, day(1) + 3_600),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.groups.len(), 1);
    assert_eq!(outcome.groups[0].position, Position::Mixed);
    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
}

#[test]
fn an_explicit_mixed_answer_is_a_statement_not_a_tie() {
    let rows = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::Mixed, day(0)),
        behavioural(2, Position::LeansHigh, day(5)),
        behavioural(3, Position::LeansHigh, day(9)),
        behavioural(4, Position::LeansHigh, day(14)),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.state.position, Position::Mixed);
    assert_eq!(outcome.state.band, Band::Weak);
}

// -------------------------------------------------------------- the lock ---

#[test]
fn a_correction_settles_the_axis_before_any_counting_happens() {
    let rows = vec![
        behavioural(1, Position::LeansHigh, day(1)),
        behavioural(2, Position::LeansHigh, day(2)),
        behavioural(3, Position::LeansHigh, day(3)),
        EvidenceRef::correction(4, AXIS, Position::LeansLow, day(4)),
    ];

    let outcome = a1_axis_state(AXIS, &rows);
    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.band, Band::Strong);
    assert!(outcome.state.locked_by_user);
    assert_eq!(outcome.state.evidence_ids, vec![4]);
    assert_eq!(outcome.reason, A1Reason::UserLocked);
    assert!(
        outcome.groups.is_empty(),
        "no grouping runs once the user has spoken"
    );
    assert_eq!(outcome.refused_ids(), vec![1, 2, 3]);
    assert!(outcome
        .shadow
        .iter()
        .take(3)
        .all(|entry| entry.result == ApplyResult::RefusedLocked));
}

#[test]
fn a_correction_to_unknown_locks_the_axis_at_unknown() {
    let outcome = a1_axis_state(AXIS, &fixture("correction_to_unknown"));
    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert!(outcome.state.locked_by_user);
}

#[test]
fn nothing_at_all_is_unknown() {
    let outcome = a1_axis_state(AXIS, &[]);
    assert_eq!(outcome.state.position, Position::Unknown);
    assert_eq!(outcome.state.band, Band::None);
    assert_eq!(outcome.reason, A1Reason::NoEvidence);

    let all_gone = a1_axis_state(AXIS, &fixture("everything_forgotten"));
    assert_eq!(all_gone.state.position, Position::Unknown);
    assert_eq!(all_gone.reason, A1Reason::NoEvidence);
}

// ------------------------------------- the Round 3 default, and EMPTY_ON_V01 ---

#[test]
fn the_default_is_two_kinds_across_days() {
    assert_eq!(A1_DEFAULT_INDEPENDENCE, A1Independence::TwoKindsAcrossDays);
    assert_eq!(
        A1Independence::default(),
        A1Independence::TwoKindsAcrossDays,
        "the enum default and the named default have to be the same thing"
    );
    assert_eq!(
        A1_DEFAULT_INDEPENDENCE.as_str(),
        "two_kinds_across_days",
        "the audit spelling of the default"
    );
}

#[test]
fn the_default_does_not_upgrade_three_same_kind_days() {
    // The Round 2 degenerate upgrade, now refused. Three questionnaire re-fills
    // on three separate UTC days are three independent *groups* and still one
    // instrument, so the axis stays at what a questionnaire answer is worth.
    let outcome = a1_axis_state(AXIS, &fixture("three_questionnaire_refills_on_three_days"));

    assert_eq!(outcome.groups.len(), 3, "still three groups");
    assert_eq!(outcome.distinct_supporting_kinds(), 1, "and one instrument");
    assert_eq!(outcome.state.band, Band::Moderate);
    assert_eq!(outcome.reason, A1Reason::NeedsASecondSource { groups: 3 });
    assert_eq!(
        outcome.state.evidence_ids,
        vec![1, 2, 3],
        "all three are still cited; they are just not three reasons"
    );
}

#[test]
fn the_default_does_not_upgrade_three_same_kind_days_of_behaviour_either() {
    // The rule is "two instruments", not "not the questionnaire". Three message
    // observations on three days are the same shape and get the same answer,
    // which is what stops the rule reading as a special case aimed at intake.
    let outcome = a1_axis_state(AXIS, &fixture("three_message_days_same_kind"));

    assert_eq!(outcome.groups.len(), 3);
    assert_eq!(outcome.distinct_supporting_kinds(), 1);
    assert_eq!(
        outcome.state.band,
        Band::Weak,
        "the strongest single row, capped at Moderate: these rows are Weak"
    );
    assert_eq!(outcome.reason, A1Reason::NeedsASecondSource { groups: 3 });
}

#[test]
fn the_named_alternative_still_upgrades_three_same_kind_days() {
    // `KindAndDay` is kept, and kept reachable only by name. This is the Round 2
    // behaviour, preserved so the ablation can still run and so the divergence
    // from `CANDIDATE_SPEC.md` stays a value somebody can point at.
    let outcome = a1_axis_state_with(
        AXIS,
        &fixture("three_questionnaire_refills_on_three_days"),
        A1Independence::KindAndDay,
    );

    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(
        outcome.reason,
        A1Reason::UpgradedByIndependentGroups { groups: 3 }
    );
}

#[test]
fn empty_on_v01_is_true_under_the_default_and_false_under_the_alternative() {
    // `EMPTY_ON_V01` is derived from the reachable evidence kinds rather than
    // asserted, so a third v0.1 producer against an axis flips it instead of
    // silently invalidating the claim.
    assert_eq!(
        A1_EMPTY_ON_V01,
        !a1_can_fire_on_v01_data(A1_DEFAULT_INDEPENDENCE),
        "the constant has to agree with the derivation, or it is just a comment"
    );
    // And the derivation says inert, which is what makes the constant `true`.
    assert!(!a1_can_fire_on_v01_data(A1Independence::TwoKindsAcrossDays));
    assert!(a1_can_fire_on_v01_data(A1Independence::KindAndDay));
}

#[test]
fn no_v01_reachable_log_reaches_strong_without_a_correction() {
    // The behavioural half of `EMPTY_ON_V01`: every fixture, stripped to the
    // rows a v0.1 install can actually produce. Whatever survives, the only
    // `Strong` left is a locked one.
    for (name, log) in axis_evidence_matrix() {
        let v01: Vec<EvidenceRef> = log
            .into_iter()
            .filter(|row| row.kind.reachable_for_axis_in_v01())
            .collect();

        for axis in AxisId::ALL {
            let state = a1_axis_state(axis, &v01).state;
            if state.band == Band::Strong {
                assert!(
                    state.locked_by_user,
                    "{name}/{}: A1 reached Strong on v0.1 data with no correction",
                    axis.key()
                );
            }
        }
    }
}

#[test]
fn the_default_still_passes_f16b() {
    // The survival line is untouched: a questionnaire answer plus behavioural
    // rows on three separate days is two instruments, so it upgrades.
    let outcome = a1_axis_state(
        AXIS,
        &fixture("f16b_questionnaire_plus_three_independent_days"),
    );
    assert_eq!(outcome.state.band, Band::Strong);
    assert_eq!(outcome.distinct_supporting_kinds(), 2);
}

#[test]
fn only_two_evidence_kinds_can_reach_an_axis_in_v01() {
    // The fact the whole EMPTY_ON_V01 argument rests on, kept as a test so that
    // adding a third producer breaks something visible.
    let reachable: Vec<&str> = EvidenceKind::ALL
        .iter()
        .filter(|kind| kind.reachable_for_axis_in_v01())
        .map(|kind| kind.as_str())
        .collect();
    assert_eq!(reachable, vec!["questionnaire", "user_correction"]);
}

// ------------------------------------------------------------ explanation ---

#[test]
fn every_explanation_is_countable_and_publishable() {
    for (name, log) in axis_evidence_matrix() {
        for independence in A1Independence::ALL {
            let text = a1_axis_state_with(AXIS, &log, independence).explain_zh();
            assert!(!text.is_empty(), "{name}");
            soul_algo_trait::denylist::assert_publishable(&text)
                .unwrap_or_else(|hit| panic!("{name}: {hit:?} in {text}"));
        }
    }
}

#[test]
fn the_upgrade_explanation_names_the_groups_days_and_sources() {
    let outcome = a1_axis_state(
        AXIS,
        &fixture("f16b_questionnaire_plus_three_independent_days"),
    );
    let text = outcome.explain_zh();
    assert!(text.contains('4'), "four groups: {text}");
    assert!(text.contains('2'), "two kinds of source: {text}");
}

#[test]
fn the_refusal_explanation_does_not_ask_for_more_of_the_same() {
    // A user who has answered the same question three times and is still at
    // Moderate is owed the actual reason. "Only 3 groups so far" would be a lie
    // by omission — a fourth re-fill would not help either.
    let text =
        a1_axis_state(AXIS, &fixture("three_questionnaire_refills_on_three_days")).explain_zh();

    assert!(text.contains("同一种来源"), "{text}");
    assert!(
        !text.contains("还不够升为强"),
        "that is the too-few-groups wording, and it is the wrong one here: {text}"
    );
    soul_algo_trait::denylist::assert_publishable(&text).unwrap();
}
