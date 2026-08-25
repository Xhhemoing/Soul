//! A0: the correction lock, on both paths, and the last-write-wins contract.
//!
//! The Round 1 defect these tests exist for: Goal 1's `intake` calls
//! `place_axis` without asking `axis_is_locked`, so re-running the
//! questionnaire moves an axis the user has already corrected — and leaves the
//! padlock showing. `apply_intake` is the patched path; the first four tests
//! are what "patched" means.

use soul_algo_trait::a0::{
    a0_all_axes_with, a0_axis_state, a0_axis_state_with, apply_intake, IntakeAnswer, IntakeSkip,
    WriteMode,
};
use soul_algo_trait::fixtures::{axis_evidence_matrix, DAY, FIXTURE_DAY_ZERO_UNIX};
use soul_algo_trait::types::{
    ApplyResult, AxisId, Band, EvidenceKind, EvidenceRef, Position, SECONDS_PER_DAY,
};

const AXIS: AxisId = AxisId::SocialEnergy;
const OTHER: AxisId = AxisId::Curiosity;

fn day(n: i64) -> i64 {
    FIXTURE_DAY_ZERO_UNIX + n * DAY
}

// ------------------------------------------------- the lock, on intake ---

#[test]
fn intake_does_not_move_an_axis_the_user_corrected() {
    let log = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, day(0)),
        EvidenceRef::correction(2, AXIS, Position::LeansLow, day(1)),
    ];

    let report = apply_intake(
        &log,
        &[IntakeAnswer::new(3, AXIS, Position::LeansHigh, day(9))],
        WriteMode::LastWriteWins,
    );

    let axis = report.axis(AXIS);
    assert_eq!(axis.position, Position::LeansLow, "the correction stands");
    assert_eq!(axis.band, Band::Strong);
    assert!(axis.locked_by_user);
    assert_eq!(axis.evidence_ids, vec![2], "the correction is still cited");
    assert!(report.applied.is_empty());
}

#[test]
fn a_skipped_answer_is_reported_rather_than_dropped() {
    let log = vec![EvidenceRef::correction(1, AXIS, Position::LeansLow, day(0))];

    let report = apply_intake(
        &log,
        &[IntakeAnswer::new(2, AXIS, Position::LeansHigh, day(4))],
        WriteMode::LastWriteWins,
    );

    assert!(report.ignored_any());
    assert_eq!(report.ignored_ids(), vec![2]);

    let ignored = report.ignored[0];
    assert_eq!(ignored.axis, AXIS);
    assert_eq!(ignored.position, Position::LeansHigh);
    assert_eq!(ignored.reason, IntakeSkip::AxisLockedByUser);
    assert_eq!(ignored.reason.as_str(), "axis_locked_by_user");
}

#[test]
fn the_answer_row_is_still_written_even_when_the_axis_will_not_move() {
    let log = vec![EvidenceRef::correction(1, AXIS, Position::LeansLow, day(0))];

    let report = apply_intake(
        &log,
        &[IntakeAnswer::new(2, AXIS, Position::LeansHigh, day(4))],
        WriteMode::LastWriteWins,
    );

    assert_eq!(report.log_after.len(), 2, "the evidence table still grows");
    let written = &report.log_after[1];
    assert_eq!(written.evidence_id, 2);
    assert_eq!(written.kind, EvidenceKind::Questionnaire);
    assert!(!written.locked_by_user);
}

#[test]
fn a_lock_on_one_axis_does_not_freeze_the_others() {
    let log = vec![EvidenceRef::correction(1, AXIS, Position::LeansLow, day(0))];

    let report = apply_intake(
        &log,
        &[
            IntakeAnswer::new(2, AXIS, Position::LeansHigh, day(4)),
            IntakeAnswer::new(3, OTHER, Position::LeansHigh, day(4)),
        ],
        WriteMode::LastWriteWins,
    );

    assert_eq!(report.ignored_ids(), vec![2]);
    assert_eq!(report.applied, vec![3]);
    assert_eq!(report.axis(OTHER).position, Position::LeansHigh);
    assert_eq!(report.axis(OTHER).band, Band::Moderate);
    assert_eq!(report.axis(AXIS).position, Position::LeansLow);
}

#[test]
fn intake_on_a_clean_profile_still_works() {
    let report = apply_intake(
        &[],
        &[
            IntakeAnswer::new(1, AXIS, Position::LeansHigh, day(0)),
            IntakeAnswer::new(2, OTHER, Position::LeansLow, day(0)),
        ],
        WriteMode::LastWriteWins,
    );

    assert_eq!(report.applied, vec![1, 2]);
    assert!(!report.ignored_any());
    assert_eq!(report.axis(AXIS).band, Band::Moderate);
    assert_eq!(report.axis(AXIS).evidence_ids, vec![1]);
    assert!(!report.axis(AXIS).locked_by_user);
    assert_eq!(report.axes.len(), 5);
}

#[test]
fn a_later_correction_still_moves_a_locked_axis() {
    // The lock stops the machine and the questionnaire. It does not stop the
    // user changing their mind.
    let log = vec![
        EvidenceRef::correction(1, AXIS, Position::LeansLow, day(0)),
        EvidenceRef::correction(2, AXIS, Position::LeansHigh, day(30)),
    ];

    let state = a0_axis_state(AXIS, &log).state;
    assert_eq!(state.position, Position::LeansHigh);
    assert_eq!(state.band, Band::Strong);
    assert!(state.locked_by_user);
    assert_eq!(state.evidence_ids, vec![2]);
}

#[test]
fn intake_matches_replay_on_every_fixture_and_mode() {
    // The reason `apply_intake` can be trusted: it is the same fold as the
    // replay, so a store that applies answers imperatively and a store that
    // replays its log cannot disagree about where an axis ended up. This is the
    // property Goal 1's `intake` breaks.
    let answers = [
        IntakeAnswer::new(9001, AXIS, Position::LeansHigh, day(200)),
        IntakeAnswer::new(9002, OTHER, Position::LeansLow, day(200)),
        IntakeAnswer::new(9003, AXIS, Position::LeansLow, day(200)),
    ];

    for mode in [WriteMode::LastWriteWins, WriteMode::NoDowngrade] {
        for (name, log) in axis_evidence_matrix() {
            let report = apply_intake(&log, &answers, mode);
            let replayed: Vec<_> = a0_all_axes_with(&report.log_after, mode)
                .into_iter()
                .map(|outcome| outcome.state)
                .collect();
            assert_eq!(report.axes, replayed, "{name} under {mode:?}");
        }
    }
}

// ------------------------------------------- the lock, on the replay path ---

#[test]
fn an_inference_never_moves_a_locked_axis_but_is_still_returned() {
    let log = vec![
        EvidenceRef::correction(1, AXIS, Position::LeansLow, day(0)),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansHigh,
            Band::Strong,
            EvidenceKind::Message,
            day(1),
        ),
    ];

    let outcome = a0_axis_state(AXIS, &log);
    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.evidence_ids, vec![1]);
    assert_eq!(outcome.refused_by_lock_ids(), vec![2]);
    assert_eq!(outcome.shadow[1].result, ApplyResult::RefusedLocked);
    assert!(!outcome.shadow[1].cited);
}

#[test]
fn a_user_may_correct_an_axis_to_unknown() {
    let log = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, day(0)),
        EvidenceRef::correction(2, AXIS, Position::Unknown, day(1)),
    ];

    let state = a0_axis_state(AXIS, &log).state;
    assert_eq!(state.position, Position::Unknown);
    assert_eq!(state.band, Band::None, "unknown claims nothing");
    assert!(state.locked_by_user, "\"stop guessing\" still locks");
}

// ----------------------------------------------------------- forgetting ---

#[test]
fn forgotten_rows_are_dropped_before_anything_is_aggregated() {
    let mut log = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, day(0)),
        EvidenceRef::correction(2, AXIS, Position::LeansLow, day(1)),
    ];

    assert!(a0_axis_state(AXIS, &log).state.locked_by_user);

    log[1] = log[1].clone().into_forgotten();
    let after = a0_axis_state(AXIS, &log).state;

    assert_eq!(after.position, Position::LeansHigh, "falls back on its own");
    assert!(!after.locked_by_user, "a forgotten lock is not a lock");
    assert_eq!(after.evidence_ids, vec![1]);
    assert!(
        !after.evidence_ids.iter().any(|id| *id == 2),
        "nothing cites a forgotten row"
    );
}

#[test]
fn a_fully_forgotten_axis_returns_to_unknown() {
    for (name, log) in axis_evidence_matrix() {
        let forgotten: Vec<EvidenceRef> = log.into_iter().map(|row| row.into_forgotten()).collect();
        for mode in [WriteMode::LastWriteWins, WriteMode::NoDowngrade] {
            let state = a0_axis_state_with(AXIS, &forgotten, mode).state;
            assert_eq!(state.position, Position::Unknown, "{name}");
            assert_eq!(state.band, Band::None, "{name}");
            assert!(state.evidence_ids.is_empty(), "{name}");
            assert!(!state.locked_by_user, "{name}");
        }
    }
}

// ------------------------------------------- the last-write-wins contract ---

#[test]
fn last_write_wins_is_the_a0_contract() {
    // Documented, not endorsed. `place_axis` in Goal 1 replaces `evidence_ids`
    // rather than accumulating, so a single Weak inference erases a Moderate
    // questionnaire answer's citation and takes the axis with it. This test
    // exists so that a future change to A0 has to be a deliberate change to the
    // baseline, and so that `NoDowngrade` below has something to differ from.
    let log = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, day(0)),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansLow,
            Band::Weak,
            EvidenceKind::Message,
            day(3),
        ),
    ];

    let outcome = a0_axis_state_with(AXIS, &log, WriteMode::LastWriteWins);
    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.band, Band::Weak, "downgraded by one guess");
    assert_eq!(outcome.state.evidence_ids, vec![2]);
    assert!(
        !outcome.state.evidence_ids.contains(&1),
        "the questionnaire answer stops being cited: this is the defect, pinned"
    );
    assert!(!outcome.refused_any(), "nothing is refused under LWW");
}

#[test]
fn no_downgrade_keeps_the_better_supported_position() {
    let log = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, day(0)),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansLow,
            Band::Weak,
            EvidenceKind::Message,
            day(3),
        ),
    ];

    let outcome = a0_axis_state_with(AXIS, &log, WriteMode::NoDowngrade);
    assert_eq!(outcome.state.position, Position::LeansHigh);
    assert_eq!(outcome.state.band, Band::Moderate);
    assert_eq!(outcome.state.evidence_ids, vec![1]);
    assert_eq!(outcome.refused_ids(), vec![2], "kept in the shadow");
    assert_eq!(outcome.shadow[1].result, ApplyResult::RefusedWeaker);
    assert_eq!(
        outcome.refused_by_lock().count(),
        0,
        "refused for being weaker, not by a lock"
    );
}

#[test]
fn no_downgrade_lets_an_agreeing_row_join_the_citation_list() {
    let log = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, day(0)),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansHigh,
            Band::Weak,
            EvidenceKind::Message,
            day(3),
        ),
    ];

    let outcome = a0_axis_state_with(AXIS, &log, WriteMode::NoDowngrade);
    assert_eq!(outcome.state.evidence_ids, vec![1, 2]);
    assert_eq!(
        outcome.state.band,
        Band::Moderate,
        "the strongest single supporter, not a sum"
    );
    assert!(outcome.shadow.iter().all(|entry| entry.cited));
}

#[test]
fn no_downgrade_still_lets_an_equally_supported_disagreement_through() {
    let log = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, day(0)),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansLow,
            Band::Moderate,
            EvidenceKind::Message,
            day(3),
        ),
    ];

    let outcome = a0_axis_state_with(AXIS, &log, WriteMode::NoDowngrade);
    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.state.evidence_ids, vec![2]);
}

#[test]
fn no_downgrade_never_overrides_the_lock() {
    let log = vec![
        EvidenceRef::correction(1, AXIS, Position::LeansLow, day(0)),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansHigh,
            Band::Strong,
            EvidenceKind::Message,
            day(3),
        ),
    ];

    let outcome = a0_axis_state_with(AXIS, &log, WriteMode::NoDowngrade);
    assert_eq!(outcome.state.position, Position::LeansLow);
    assert_eq!(outcome.refused_by_lock_ids(), vec![2]);
}

// ------------------------------------------------------------ housekeeping ---

#[test]
fn evidence_for_another_axis_is_ignored() {
    let log = vec![EvidenceRef::correction(
        1,
        OTHER,
        Position::LeansLow,
        day(0),
    )];
    let outcome = a0_axis_state(AXIS, &log);
    assert_eq!(outcome.state.position, Position::Unknown);
    assert!(outcome.shadow.is_empty());
}

#[test]
fn an_empty_log_leaves_every_axis_unknown() {
    for outcome in a0_all_axes_with(&[], WriteMode::LastWriteWins) {
        assert_eq!(outcome.state.position, Position::Unknown);
        assert_eq!(outcome.state.band, Band::None);
        assert!(outcome.state.evidence_ids.is_empty());
    }
}

#[test]
fn a0_ignores_the_timestamp_entirely() {
    // A0 has no independence key and no decay. Shifting every row by a decade
    // must not change a thing; A1 is where the day starts to matter.
    let log = vec![
        EvidenceRef::questionnaire(1, AXIS, Position::LeansHigh, day(0)),
        EvidenceRef::inference(
            2,
            AXIS,
            Position::LeansHigh,
            Band::Weak,
            EvidenceKind::Message,
            day(3),
        ),
    ];
    let shifted: Vec<EvidenceRef> = log
        .iter()
        .cloned()
        .map(|mut row| {
            row.recorded_at_unix -= 3650 * SECONDS_PER_DAY;
            row
        })
        .collect();

    for mode in [WriteMode::LastWriteWins, WriteMode::NoDowngrade] {
        assert_eq!(
            a0_axis_state_with(AXIS, &log, mode).state.position,
            a0_axis_state_with(AXIS, &shifted, mode).state.position
        );
        assert_eq!(
            a0_axis_state_with(AXIS, &log, mode).state.band,
            a0_axis_state_with(AXIS, &shifted, mode).state.band
        );
    }
}
