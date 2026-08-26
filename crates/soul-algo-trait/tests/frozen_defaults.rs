//! What Round 3 froze, pinned so that unfreezing it has to be deliberate.
//!
//! `R2-SYNTHESIS.md` retains one tie-strength algorithm and **A0**. This file
//! is the A0 side of that decision written as assertions: the identifier, the
//! default write mode, the meaning of `Strong`, and the fact that A2 renders
//! whatever the tie algorithm decided without forming a second opinion.
//!
//! Every test here is a tripwire rather than a discovery. If one of them fails,
//! the question to ask is not "what broke" but "was this change meant to be a
//! change to the frozen v0.1 specification".

use soul_algo_trait::a0::{
    a0_all_axes, a0_all_axes_with, a0_axis_state, apply_intake, IntakeAnswer, WriteMode,
    A0_ALGORITHM_ID, A0_DEFAULT_WRITE_MODE,
};
use soul_algo_trait::a1::{
    a1_can_fire_on_v01_data, A1_ALGORITHM_ID, A1_DEFAULT_INDEPENDENCE, A1_EMPTY_ON_V01,
};
use soul_algo_trait::a2::{a2_render, A2_ALGORITHM_ID};
use soul_algo_trait::a3::A3_ALGORITHM_ID;
use soul_algo_trait::fixtures::{
    axis_evidence_matrix, tie_score_matrix, DAY, FIXTURE_DAY_ZERO_UNIX,
};
use soul_algo_trait::types::{AxisId, Band, EvidenceKind, EvidenceRef, Position};
use soul_algo_trait::A1Independence;

const AXIS: AxisId = AxisId::SocialEnergy;

fn day(n: i64) -> i64 {
    FIXTURE_DAY_ZERO_UNIX + n * DAY
}

// ------------------------------------------------------ A0 is keeper #2 ---

#[test]
fn the_a0_identifier_did_not_move() {
    // Round 3 froze A0 without changing a decision it makes, so a state stamped
    // by the Round 2 package and a state stamped by this one mean the same
    // thing. Bumping this string would be a claim that they do not.
    assert_eq!(A0_ALGORITHM_ID, "a0.questionnaire_correction_lock.v2");

    for (name, log) in axis_evidence_matrix() {
        for outcome in a0_all_axes(&log) {
            assert_eq!(outcome.state.algorithm_id, A0_ALGORITHM_ID, "{name}");
        }
    }
}

#[test]
fn the_frozen_write_mode_is_last_write_wins() {
    assert_eq!(A0_DEFAULT_WRITE_MODE, WriteMode::LastWriteWins);
    assert_eq!(WriteMode::default(), WriteMode::LastWriteWins);

    for (name, log) in axis_evidence_matrix() {
        for axis in AxisId::ALL {
            assert_eq!(
                a0_axis_state(axis, &log).state,
                soul_algo_trait::a0::a0_axis_state_with(axis, &log, A0_DEFAULT_WRITE_MODE).state,
                "{name}/{}",
                axis.key()
            );
        }
    }
}

#[test]
fn the_frozen_write_mode_decides_nothing_a_v01_user_is_told() {
    // The argument for keeping the clobbering mode as the frozen default. The
    // clobber needs a non-correction row that disagrees with what is standing,
    // and v0.1 has no producer that can make one: the questionnaire and a
    // correction are the whole data plane, and a correction locks. So on every
    // fixture restricted to v0.1-reachable rows, the two modes agree about the
    // three things the profile shows — direction, band, lock.
    //
    // They do *not* agree about the citation list; see the next test. That is
    // the entire observable difference, and it is why this test names three
    // fields instead of comparing the whole state.
    for (name, log) in axis_evidence_matrix() {
        let v01: Vec<EvidenceRef> = log
            .into_iter()
            .filter(|row| row.kind.reachable_for_axis_in_v01())
            .collect();

        let lww = a0_all_axes_with(&v01, WriteMode::LastWriteWins);
        let no_downgrade = a0_all_axes_with(&v01, WriteMode::NoDowngrade);

        for (left, right) in lww.iter().zip(no_downgrade.iter()) {
            let axis = left.state.axis.key();
            assert_eq!(left.state.position, right.state.position, "{name}/{axis}");
            assert_eq!(left.state.band, right.state.band, "{name}/{axis}");
            assert_eq!(
                left.state.locked_by_user, right.state.locked_by_user,
                "{name}/{axis}"
            );
        }
    }
}

#[test]
fn the_two_write_modes_do_differ_in_what_they_cite() {
    // Found by the test above, and kept as its own assertion rather than
    // smoothed over. Five re-fills of one questionnaire on one afternoon:
    // last-write-wins cites the last row and drops the other four, NoDowngrade
    // cites all five. The axis reads the same either way, so this is not a
    // reason to unfreeze — but 「用户能复核计数」 is a criterion in its own
    // right, and a user asking "what is this based on" gets a shorter answer
    // under the frozen mode. Round X should decide whether that matters;
    // Round 3's job is to make sure it is not a surprise.
    let log: Vec<EvidenceRef> = (1..=5)
        .map(|n| EvidenceRef::questionnaire(n, AXIS, Position::LeansHigh, day(0) + n as i64 * 600))
        .collect();

    let lww = soul_algo_trait::a0::a0_axis_state_with(AXIS, &log, WriteMode::LastWriteWins).state;
    let no_downgrade =
        soul_algo_trait::a0::a0_axis_state_with(AXIS, &log, WriteMode::NoDowngrade).state;

    assert_eq!(lww.band, no_downgrade.band);
    assert_eq!(lww.position, no_downgrade.position);
    assert_eq!(
        lww.evidence_ids,
        vec![5],
        "the frozen mode cites the newest"
    );
    assert_eq!(no_downgrade.evidence_ids, vec![1, 2, 3, 4, 5]);
}

#[test]
fn strong_on_the_retained_path_means_the_user_said_so() {
    // The one-sentence semantics of the retained algorithm, scoped to where it
    // actually holds: the v0.1 data plane. A0 is not wired to A1 and has no
    // upgrade rule of its own, so across every fixture — restricted to the rows
    // a v0.1 install can produce — nothing but a correction reaches Strong.
    for (name, log) in axis_evidence_matrix() {
        let v01: Vec<EvidenceRef> = log
            .into_iter()
            .filter(|row| row.kind.reachable_for_axis_in_v01())
            .collect();

        for mode in [WriteMode::LastWriteWins, WriteMode::NoDowngrade] {
            for outcome in a0_all_axes_with(&v01, mode) {
                let state = outcome.state;
                if state.band == Band::Strong {
                    assert!(
                        state.locked_by_user,
                        "{name}/{} under {mode:?}",
                        state.axis.key()
                    );
                }
            }
        }
    }
}

#[test]
fn a0_trusts_the_band_on_a_row_it_is_handed() {
    // Documented, not endorsed — the same shape as
    // `last_write_wins_is_the_a0_contract`.
    //
    // The guarantee above is a property of the v0.1 *data plane*, not of A0's
    // code: A0 reads `EvidenceRef::band` and never caps it, so a producer that
    // writes a row labelled `Strong` gets `Strong`, no correction involved.
    // Under NoDowngrade a later agreeing questionnaire answer does not even
    // pull it back down, because the band of a supported position is the
    // strongest single supporter.
    //
    // Nothing in v0.1 can do this — `record_axis_inference` has no production
    // caller, which is what `only_two_evidence_kinds_can_reach_an_axis_in_v01`
    // pins. The version that starts writing behavioural rows against an axis
    // has to either cap non-correction rows at Moderate or adopt A1, and this
    // test is here so that it is a decision somebody makes rather than a
    // regression somebody ships.
    let overconfident = vec![
        EvidenceRef::inference(
            1,
            AXIS,
            Position::LeansHigh,
            Band::Strong,
            EvidenceKind::Message,
            day(0),
        ),
        EvidenceRef::questionnaire(2, AXIS, Position::LeansHigh, day(2)),
    ];

    let lww =
        soul_algo_trait::a0::a0_axis_state_with(AXIS, &overconfident, WriteMode::LastWriteWins)
            .state;
    assert_eq!(
        lww.band,
        Band::Moderate,
        "last write wins, and the last write is the questionnaire"
    );

    let no_downgrade =
        soul_algo_trait::a0::a0_axis_state_with(AXIS, &overconfident, WriteMode::NoDowngrade).state;
    assert_eq!(
        no_downgrade.band,
        Band::Strong,
        "the gap: a self-declared Strong row survives an agreeing answer"
    );
    assert!(
        !no_downgrade.locked_by_user,
        "and it is Strong without being locked, which is the part that matters"
    );

    // A correction is still the only thing that locks.
    let corrected = [
        overconfident.as_slice(),
        &[EvidenceRef::correction(3, AXIS, Position::LeansLow, day(3))],
    ]
    .concat();
    let state = a0_axis_state(AXIS, &corrected).state;
    assert_eq!(state.position, Position::LeansLow);
    assert!(state.locked_by_user);
}

#[test]
fn the_intake_patch_is_part_of_the_freeze() {
    // The Round 1 defect must not come back with the freeze. Re-running the
    // questionnaire over a corrected axis leaves the axis alone, reports the
    // answer, and still writes the row.
    let log = vec![EvidenceRef::correction(1, AXIS, Position::LeansLow, day(0))];
    let report = apply_intake(
        &log,
        &[IntakeAnswer::new(2, AXIS, Position::LeansHigh, day(30))],
        A0_DEFAULT_WRITE_MODE,
    );

    assert_eq!(report.axis(AXIS).position, Position::LeansLow);
    assert_eq!(report.axis(AXIS).evidence_ids, vec![1]);
    assert_eq!(report.ignored_ids(), vec![2]);
    assert!(report.applied.is_empty());
    assert_eq!(report.log_after.len(), 2);
}

// -------------------------------------------- A2 is frozen as a renderer ---

#[test]
fn the_a2_source_still_holds_no_threshold() {
    // The same source-level guard `a2_render.rs` runs, restated here because it
    // is a freeze condition and not only an A2 property: the retained pair has
    // exactly one place where a band is decided, and it is not this crate.
    let source = include_str!("../src/a2.rs");
    for forbidden in [
        "STRONG_MIN",
        "MODERATE_MIN",
        "MIN_INTERACTIONS",
        "MIN_ACTIVE_DAYS",
        ">= 10",
        ">=10",
    ] {
        assert!(
            !source.contains(forbidden),
            "src/a2.rs contains {forbidden:?}: A2 has started deciding bands again"
        );
    }
}

#[test]
fn a2_is_a_function_of_the_band_it_is_handed() {
    // Freezing a renderer means pinning that it stays one. Every fixture, every
    // band: the bullets that come out carry the band that went in, and the
    // added venue-split field does not change that.
    for (name, mut base) in tie_score_matrix() {
        for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
            base.band = band;
            for bullet in a2_render(&base).bullets {
                assert_eq!(bullet.band, band, "{name}: {}", bullet.statement_key);
                assert!(
                    !bullet.evidence_ids.is_empty(),
                    "{name}: {} cites nothing",
                    bullet.statement_key
                );
            }
        }
    }
}

// ---------------------------------------------- A1 is present, not kept ---

#[test]
fn a1_is_not_on_the_retained_path() {
    // A0 must not depend on A1. Checked at the source rather than by behaviour,
    // because the failure this guards against is somebody wiring the upgrade in
    // "just for the strong case" and the ablation quietly becoming untrue.
    let a0_source = include_str!("../src/a0.rs");
    for line in a0_source.lines() {
        let code = line.trim_start();
        if code.starts_with("//") {
            continue;
        }
        assert!(
            !code.contains("a1::") && !code.contains("crate::a1"),
            "a0.rs reaches into a1: {line}"
        );
    }
}

#[test]
fn the_a1_identifier_moved_because_its_default_did() {
    assert_eq!(A1_ALGORITHM_ID, "a1.independent_group_upgrade.v3");
    assert_eq!(A1_DEFAULT_INDEPENDENCE, A1Independence::TwoKindsAcrossDays);
    assert_eq!(
        A1_EMPTY_ON_V01,
        !a1_can_fire_on_v01_data(A1_DEFAULT_INDEPENDENCE)
    );
}

#[test]
fn the_four_algorithm_identifiers_are_distinct_and_self_describing() {
    let ids = [
        A0_ALGORITHM_ID,
        A1_ALGORITHM_ID,
        A2_ALGORITHM_ID,
        A3_ALGORITHM_ID,
    ];
    for (index, id) in ids.iter().enumerate() {
        assert!(!ids[index + 1..].contains(id), "duplicate id {id}");
    }
    assert!(A3_ALGORITHM_ID.ends_with(".rejected"));
}
