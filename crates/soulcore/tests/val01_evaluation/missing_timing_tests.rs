//! Completed observations retain their facts when only timing was not measured.
use super::{aggregate, fixtures, pairs, records::*};

#[test]
fn completed_missing_edit_keeps_completion_and_other_metrics() {
    let mut data = fixtures::fixture();
    let candidate = data
        .candidates
        .iter_mut()
        .find(|c| c.candidate_id == "P1-A")
        .unwrap();
    candidate.edit_seconds = Measured::Missing(MissingReason::NotMeasured);
    candidate.missing_reason = Some(MissingReason::NotMeasured);
    let validated =
        pairs::validate(&data).unwrap_or_else(|_| panic!("completed missing edit must validate"));
    let report = aggregate::summarize(&validated);
    assert_eq!(report.funnel.completed, 5);
    let group = report
        .pairs
        .iter()
        .find(|(key, _)| {
            key.evaluation_run == "synthetic-run-1"
                && key.task_type == TaskType::T2
                && key.contrast == Contrast::AB
        })
        .unwrap()
        .1;
    assert_eq!(group.edits.len(), 2);
    assert_eq!(group.preference.denominator, 3);
    assert_eq!(group.preference.target_wins, 1);
}

#[test]
fn completed_missing_pause_keeps_independently_measured_zero_edit() {
    let mut data = fixtures::fixture();
    let candidate = data
        .candidates
        .iter_mut()
        .find(|c| c.candidate_id == "P1-A")
        .unwrap();
    candidate.pause_seconds = Measured::Missing(MissingReason::NotMeasured);
    candidate.missing_reason = Some(MissingReason::NotMeasured);
    let validated =
        pairs::validate(&data).unwrap_or_else(|_| panic!("completed missing pause must validate"));
    let report = aggregate::summarize(&validated);
    let group = report
        .pairs
        .iter()
        .find(|(key, _)| {
            key.evaluation_run == "synthetic-run-1"
                && key.task_type == TaskType::T2
                && key.contrast == Contrast::AB
        })
        .unwrap()
        .1;
    assert_eq!(group.edits.len(), 3);
    assert_eq!(group.edits[0].control_seconds, 0);
    assert_eq!(group.median_difference, Some(-10.0));
}

fn group(report: &aggregate::Summary) -> &aggregate::PairSummary {
    report
        .pairs
        .iter()
        .find(|(key, _)| {
            key.evaluation_run == "synthetic-run-1"
                && key.task_type == TaskType::T2
                && key.contrast == Contrast::AB
        })
        .unwrap()
        .1
}

#[test]
fn zero_one_and_both_sides_missing_keep_literal_metric_denominators() {
    for missing in [vec![], vec!["P1-A"], vec!["P1-A", "P1-B"]] {
        let mut data = fixtures::fixture();
        for id in &missing {
            let row = data
                .candidates
                .iter_mut()
                .find(|c| c.candidate_id == *id)
                .unwrap();
            row.edit_seconds = Measured::Missing(MissingReason::NotMeasured);
            row.missing_reason = Some(MissingReason::NotMeasured);
        }
        let report = aggregate::summarize(&pairs::validate(&data).unwrap());
        let ab = group(&report);
        assert_eq!(ab.edit_missing_pairs, usize::from(!missing.is_empty()));
        assert_eq!(ab.edit_valid_pairs, if missing.is_empty() { 3 } else { 2 });
        assert_eq!(ab.edit_valid_pairs, ab.edits.len());
        assert_eq!(ab.preference.denominator, 3);
        assert_eq!(ab.funnel.completed, 3);
        assert_eq!(report.funnel.completed, 5);
        assert_eq!(
            report
                .candidates
                .values()
                .map(|r| r
                    .edit_missing
                    .get(&MissingReason::NotMeasured)
                    .copied()
                    .unwrap_or(0))
                .sum::<usize>(),
            missing.len()
        );
        let a = report
            .candidates
            .iter()
            .find(|(key, _)| {
                key.pair.evaluation_run == "synthetic-run-1"
                    && key.pair.task_type == TaskType::T2
                    && key.variant == Variant::A
            })
            .unwrap()
            .1;
        assert_eq!(
            a.first_pass,
            aggregate::Fraction {
                numerator: 3,
                denominator: 4
            }
        );
    }
}

#[test]
fn completed_timing_rejects_inapplicability_stage_reasons_and_total_reason_mismatch() {
    for pause in [false, true] {
        for invalid in [
            Measured::NotApplicable,
            Measured::Missing(MissingReason::NotStarted),
            Measured::Missing(MissingReason::NotCompleted),
            Measured::Missing(MissingReason::NotRated),
            Measured::Missing(MissingReason::DeadlineReached),
        ] {
            let mut data = fixtures::fixture();
            let row = &mut data.candidates[0];
            if pause {
                row.pause_seconds = invalid;
            } else {
                row.edit_seconds = invalid;
            }
            assert!(pairs::validate(&data).is_err());
        }
    }
    for total in [None, Some(MissingReason::NotCompleted)] {
        let mut data = fixtures::fixture();
        data.candidates[0].edit_seconds = Measured::Missing(MissingReason::NotMeasured);
        data.candidates[0].missing_reason = total;
        assert!(pairs::validate(&data).is_err());
    }
    let mut data = fixtures::fixture();
    data.candidates[0].missing_reason = Some(MissingReason::NotMeasured);
    assert!(pairs::validate(&data).is_err());
}

#[test]
fn not_measured_is_only_a_completed_timing_reason() {
    for field in [
        "first_pass",
        "help",
        "safety",
        "unsupported",
        "brief",
        "note",
    ] {
        let mut data = fixtures::fixture();
        let row = &mut data.candidates[0];
        row.edit_seconds = Measured::Missing(MissingReason::NotMeasured);
        row.missing_reason = Some(MissingReason::NotMeasured);
        match field {
            "first_pass" => row.first_pass_accepted = Measured::Missing(MissingReason::NotMeasured),
            "help" => row.help_count = Measured::Missing(MissingReason::NotMeasured),
            "safety" => row.privacy_or_safety_stop = Measured::Missing(MissingReason::NotMeasured),
            "unsupported" => row.unsupported_claim = Measured::Missing(MissingReason::NotMeasured),
            "brief" => row.brief_build_status = Measured::Missing(MissingReason::NotMeasured),
            "note" => row.sanitized_note = Measured::Missing(MissingReason::NotMeasured),
            _ => unreachable!(),
        }
        assert!(pairs::validate(&data).is_err());
    }
    let mut data = fixtures::fixture();
    let row = data
        .candidates
        .iter_mut()
        .find(|c| c.candidate_id == "P7-A")
        .unwrap();
    row.pause_seconds = Measured::Missing(MissingReason::NotMeasured);
    row.missing_reason = Some(MissingReason::NotMeasured);
    assert!(pairs::validate(&data).is_err());
}

#[test]
fn missing_timing_does_not_restore_withdrawn_or_noncomparable_utility() {
    for withdrawn in [false, true] {
        let mut data = fixtures::fixture();
        let row = &mut data.candidates[0];
        row.edit_seconds = Measured::Missing(MissingReason::NotMeasured);
        row.missing_reason = Some(MissingReason::NotMeasured);
        if withdrawn {
            data.runs[0].consent_state = ConsentState::Withdrawn;
        } else {
            data.pairs[0].comparability_status = Comparability::NotComparable;
        }
        let report = aggregate::summarize(&pairs::validate(&data).unwrap());
        let ab = group(&report);
        assert_eq!(ab.funnel.completed, 3);
        assert_eq!(ab.edit_missing_pairs, 0);
        assert_eq!(ab.edit_valid_pairs, if withdrawn { 0 } else { 2 });
        assert_eq!(ab.preference.denominator, if withdrawn { 0 } else { 2 });
    }
}

#[test]
fn pause_missing_is_counted_separately_and_t3_facts_keep_their_denominator() {
    let mut data = fixtures::fixture();
    let row = data
        .candidates
        .iter_mut()
        .find(|c| c.candidate_id == "P6-B")
        .unwrap();
    row.pause_seconds = Measured::Missing(MissingReason::NotMeasured);
    row.missing_reason = Some(MissingReason::NotMeasured);
    let report = aggregate::summarize(&pairs::validate(&data).unwrap());
    let row = report
        .candidates
        .iter()
        .find(|(key, _)| key.pair.task_type == TaskType::T3 && key.variant == Variant::B)
        .unwrap()
        .1;
    assert_eq!(
        row.fact_correct,
        aggregate::Fraction {
            numerator: 1,
            denominator: 1
        }
    );
    assert_eq!(row.pause_missing.get(&MissingReason::NotMeasured), Some(&1));
    assert!(!row.edit_missing.contains_key(&MissingReason::NotMeasured));
    let pair = report
        .pairs
        .iter()
        .find(|(key, _)| key.task_type == TaskType::T3)
        .unwrap()
        .1;
    assert_eq!(pair.edit_valid_pairs, 1);
    assert_eq!(pair.edit_missing_pairs, 0);
}

#[test]
fn missing_timing_does_not_excuse_safety_or_factual_completion_failures() {
    for safety in [false, true] {
        let mut data = fixtures::fixture();
        let row = &mut data.candidates[0];
        row.edit_seconds = Measured::Missing(MissingReason::NotMeasured);
        row.missing_reason = Some(MissingReason::NotMeasured);
        if safety {
            row.privacy_or_safety_stop = Measured::Value(true);
        } else {
            row.unsupported_claim = Measured::Value(true);
        }
        assert!(pairs::validate(&data).is_err());
    }
}
