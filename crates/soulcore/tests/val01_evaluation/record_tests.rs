//! Literal expectations derive from the design table, not the summarizer.
use super::{
    aggregate::*,
    fixtures,
    pairs::{self, ErrorKind},
    records::*,
};

fn summary(data: &Evaluation) -> Summary {
    summarize(
        &pairs::validate(data)
            .unwrap_or_else(|errors| panic!("invalid synthetic fixture: {errors:?}")),
    )
}
fn pair_group(report: &Summary, task: TaskType, contrast: Contrast) -> &PairSummary {
    report
        .pairs
        .iter()
        .find(|(key, _)| {
            key.evaluation_run == "synthetic-run-1"
                && key.task_type == task
                && key.contrast == contrast
                && key.source != SourceLayer::DegradedTemplate
        })
        .map(|(_, value)| value)
        .expect("pair stratum")
}
fn accepted(report: &Summary, task: TaskType, variant: Variant, source: SourceLayer) -> Fraction {
    report
        .candidates
        .iter()
        .find(|(key, _)| {
            key.pair.evaluation_run == "synthetic-run-1"
                && key.pair.task_type == task
                && key.variant == variant
                && key.source == source
        })
        .map(|(_, value)| value.first_pass)
        .expect("candidate stratum")
}
fn reject(data: &Evaluation, kind: ErrorKind) {
    let errors = match pairs::validate(data) {
        Ok(_) => panic!("invalid registration accepted: {kind:?}"),
        Err(errors) => errors,
    };
    assert!(
        errors
            .iter()
            .any(|error| error.kind == kind && !error.record_id.is_empty()),
        "{errors:?}"
    );
}

#[test]
fn full_fixture_retains_failures_and_literal_pair_denominators() {
    let data = fixtures::fixture();
    let report = summary(&data);
    assert_eq!(
        report.funnel,
        Funnel {
            assigned: 8,
            started: 7,
            generated: 5,
            rated: 3,
            completed: 5
        }
    );
    assert_eq!(report.withdrawn_runs, 1);
    assert_eq!(report.generated_candidates, 11);
    assert_eq!(
        Fraction {
            numerator: report.funnel.completed,
            denominator: report.funnel.assigned
        }
        .value(),
        Some(0.625)
    );
    let ab = pair_group(&report, TaskType::T2, Contrast::AB);
    assert_eq!(ab.funnel.assigned, 5);
    assert_eq!(ab.funnel.completed, 3);
    assert_eq!(
        ab.preference,
        PreferenceStats {
            target_wins: 1,
            ties: 1,
            denominator: 3
        }
    );
    assert_eq!(ab.preference.value(), Some(0.5));
}

#[test]
fn zero_seconds_remains_observed_while_incomplete_pair_is_excluded_from_time() {
    let report = summary(&fixtures::fixture());
    let ab = pair_group(&report, TaskType::T2, Contrast::AB);
    assert_eq!(
        ab.edits,
        vec![
            EditPair {
                pair_id: "P1".into(),
                control_seconds: 0,
                target_seconds: 10
            },
            EditPair {
                pair_id: "P2".into(),
                control_seconds: 120,
                target_seconds: 90
            },
            EditPair {
                pair_id: "P3".into(),
                control_seconds: 80,
                target_seconds: 100
            },
        ]
    );
    assert_eq!(ab.median_difference, Some(-10.0));
    assert_eq!(ab.median_ratio, Some(0.0));
    assert_ne!(
        Measured::Value(0_u64),
        Measured::Missing(MissingReason::NotCompleted)
    );
}

#[test]
fn first_pass_and_fact_denominators_are_per_task_variant_and_source() {
    let report = summary(&fixtures::fixture());
    for (task, variant, source, numerator, denominator) in [
        (
            TaskType::T2,
            Variant::A,
            SourceLayer::UndegradedEndpoint,
            3,
            4,
        ),
        (
            TaskType::T2,
            Variant::B,
            SourceLayer::UndegradedEndpoint,
            1,
            3,
        ),
        (TaskType::T1, Variant::A, SourceLayer::LocalTemplate, 0, 1),
        (TaskType::T1, Variant::C, SourceLayer::LocalTemplate, 0, 1),
        (
            TaskType::T3,
            Variant::B,
            SourceLayer::UndegradedEndpoint,
            1,
            1,
        ),
        (
            TaskType::T3,
            Variant::C,
            SourceLayer::UndegradedEndpoint,
            0,
            1,
        ),
    ] {
        assert_eq!(
            accepted(&report, task, variant, source),
            Fraction {
                numerator,
                denominator
            }
        );
    }
    let facts: Vec<_> = report
        .candidates
        .iter()
        .filter(|(key, _)| key.pair.task_type == TaskType::T3)
        .map(|(_, value)| value.fact_correct)
        .collect();
    assert_eq!(
        facts,
        vec![
            Fraction {
                numerator: 1,
                denominator: 1
            },
            Fraction {
                numerator: 0,
                denominator: 1
            }
        ]
    );
    assert_eq!(
        pair_group(&report, TaskType::T3, Contrast::BC)
            .preference
            .denominator,
        0
    );
    assert_eq!(
        pair_group(&report, TaskType::T1, Contrast::AC)
            .funnel
            .completed,
        1
    );
}

#[test]
fn duplicate_and_missing_relationships_are_structured_errors() {
    let original = fixtures::fixture();
    let mut data = original.clone();
    data.runs.push(data.runs[0].clone());
    reject(&data, ErrorKind::DuplicateId);
    let mut data = original.clone();
    data.pairs.push(data.pairs[0].clone());
    reject(&data, ErrorKind::DuplicateId);
    let mut data = original.clone();
    data.candidates.push(data.candidates[0].clone());
    reject(&data, ErrorKind::DuplicateId);
    let mut data = original.clone();
    data.pairs[0].evaluation_run = "absent".into();
    reject(&data, ErrorKind::MissingRun);
    let mut data = original.clone();
    data.candidates[0].pair_id = "absent".into();
    reject(&data, ErrorKind::MissingPair);
    let mut data = original;
    data.candidates.remove(0);
    reject(&data, ErrorKind::CandidateCount);
}

#[test]
fn contrast_rejects_duplicate_or_wrong_sides_and_duplicate_variant_rows() {
    let original = fixtures::fixture();
    let mut data = original.clone();
    data.pairs[0].left_variant = Variant::B;
    reject(&data, ErrorKind::Contrast);
    let mut data = original.clone();
    data.pairs[0].right_variant = Variant::C;
    reject(&data, ErrorKind::Contrast);
    let mut data = original;
    data.candidates[0].requested_variant = Variant::B;
    reject(&data, ErrorKind::Contrast);
}

#[test]
fn preregistered_swapped_presentation_preserves_target_preference_and_edit_direction() {
    let mut data = fixtures::fixture();
    let pair = &mut data.pairs[0];
    std::mem::swap(&mut pair.left_variant, &mut pair.right_variant);
    data.pairs[0].preference = Preference::Left;
    // This is a different assignment fixed before output, not a post-hoc swap.
    data.registered_slots[0] = RegisteredSlot::from_pair(&data.pairs[0]);
    let report = summary(&data);
    let ab = pair_group(&report, TaskType::T2, Contrast::AB);
    assert_eq!(ab.preference.value(), Some(0.5));
    assert_eq!(ab.median_difference, Some(-10.0));
}

#[test]
fn completed_pair_and_candidate_stage_cannot_contradict_generation_or_feedback() {
    let original = fixtures::fixture();
    let mut data = original.clone();
    data.pairs[3].pair_completion_status = PairCompletion::Completed;
    data.pairs[3].pair_completed_at = Some(200);
    reject(&data, ErrorKind::Stage);
    let mut data = original.clone();
    data.candidates[0].generated = false;
    reject(&data, ErrorKind::Stage);
    let mut data = original.clone();
    data.candidates[0].edit_seconds = Measured::Missing(MissingReason::NotCompleted);
    reject(&data, ErrorKind::Stage);
    let mut data = original;
    data.candidates[0].unsupported_claim = Measured::Value(true);
    reject(&data, ErrorKind::Stage);
}

#[test]
fn failed_b_cannot_silently_become_neutral_or_generate() {
    let mut data = fixtures::fixture();
    data.candidates[1].actual_brief_state = Measured::Value(ActualBrief::Neutral);
    reject(&data, ErrorKind::Brief);
    data.candidates[1].actual_brief_state = Measured::Value(ActualBrief::Unknown);
    data.candidates[1].brief_build_status = Measured::Value(BriefBuildStatus::Failed);
    reject(&data, ErrorKind::Brief);
}

#[test]
fn withdrawn_after_scoring_keeps_flow_but_excludes_all_pending_utility() {
    let mut data = fixtures::fixture();
    data.runs[0].consent_state = ConsentState::Withdrawn;
    let report = summary(&data);
    assert_eq!(report.funnel.rated, 3);
    assert_eq!(report.funnel.completed, 5);
    assert_eq!(report.generated_candidates, 11);
    assert_eq!(report.withdrawn_runs, 2);
    assert!(report
        .pairs
        .values()
        .all(|row| row.preference.denominator == 0 && row.edits.is_empty()));
    assert!(report
        .candidates
        .values()
        .all(|row| row.first_pass.denominator == 0 && row.fact_correct.denominator == 0));
    assert_eq!(
        report
            .candidates
            .values()
            .map(|row| row.safety_stops.denominator)
            .sum::<usize>(),
        16
    );
}

#[test]
fn model_version_and_unknown_conditions_stay_separate_without_rejecting_fixture() {
    let mut data = fixtures::fixture();
    data.pairs[1].conditions.model_version = Reported::Known("synthetic-v2".into());
    data.pairs[2].conditions.model_version = Reported::Unknown;
    data.pairs[3].conditions.model_version = Reported::NotExposed;
    // Distinct conditions were declared for these slots before execution.
    for index in 1..=3 {
        data.registered_slots[index] = RegisteredSlot::from_pair(&data.pairs[index]);
    }
    let report = summary(&data);
    let groups: Vec<_> = report
        .pairs
        .iter()
        .filter(|(key, _)| key.evaluation_run == "synthetic-run-1" && key.task_type == TaskType::T2)
        .collect();
    assert_eq!(groups.len(), 4);
    assert_eq!(
        groups
            .iter()
            .map(|(_, row)| row.funnel.assigned)
            .sum::<usize>(),
        5
    );
    assert!(groups
        .iter()
        .all(|(_, row)| row.preference.denominator <= 1));
}

#[test]
fn degraded_generated_template_stays_out_of_normal_endpoint_utility() {
    let mut data = fixtures::fixture();
    let p = &mut data.pairs[0];
    p.pair_completion_status = PairCompletion::Incomplete;
    p.pair_completed_at = None;
    let c = &mut data.candidates[1];
    c.draft_source = DraftSource::ToneTemplate;
    c.degraded_reason = DegradedReason::ReplyEmpty;
    c.candidate_completion_status = CandidateCompletion::Incomplete;
    c.edit_seconds = Measured::Missing(MissingReason::NotCompleted);
    let report = summary(&data);
    assert_eq!(report.funnel.generated, 5);
    assert_eq!(report.funnel.rated, 3);
    assert_eq!(report.funnel.completed, 4);
    let ab = pair_group(&report, TaskType::T2, Contrast::AB);
    assert_eq!(ab.funnel.assigned, 5);
    assert_eq!(ab.funnel.completed, 2);
    assert_eq!(ab.degraded_pairs, 1);
    assert_eq!(ab.preference.denominator, 2);
    let degraded = report
        .candidates
        .iter()
        .find(|(key, _)| key.source == SourceLayer::DegradedTemplate)
        .unwrap()
        .1;
    assert_eq!(degraded.generated, 1);
    assert_eq!(
        degraded.first_pass,
        Fraction {
            numerator: 0,
            denominator: 1
        }
    );
    data.candidates[1].candidate_completion_status = CandidateCompletion::Completed;
    reject(&data, ErrorKind::Stage);
}

#[test]
fn generated_unrated_is_in_acceptance_denominator_with_explicit_missing_count() {
    let mut data = fixtures::fixture();
    data.pairs[0].pair_completion_status = PairCompletion::Incomplete;
    data.pairs[0].pair_completed_at = None;
    data.pairs[0].preference = Preference::Unrated;
    let c = &mut data.candidates[1];
    c.candidate_completion_status = CandidateCompletion::Incomplete;
    c.first_pass_accepted = Measured::Missing(MissingReason::NotRated);
    c.edit_seconds = Measured::Missing(MissingReason::NotCompleted);
    c.missing_reason = Some(MissingReason::NotRated);
    let report = summary(&data);
    let row = report
        .candidates
        .iter()
        .find(|(key, _)| {
            key.pair.task_type == TaskType::T2
                && key.variant == Variant::B
                && key.source == SourceLayer::UndegradedEndpoint
        })
        .unwrap()
        .1;
    assert_eq!(
        row.first_pass,
        Fraction {
            numerator: 1,
            denominator: 3
        }
    );
    assert_eq!(row.first_pass_missing, 1);
}

#[test]
fn participant_preference_median_does_not_pool_tasks_as_people() {
    let mut data = fixtures::fixture();
    data.runs[1].consent_state = ConsentState::Granted;
    data.runs[1].participant_scope = ParticipantScope::Author;
    let mut p = data.pairs[0].clone();
    p.pair_id = "P9".into();
    p.evaluation_run = "synthetic-run-2".into();
    p.task_slot = 9;
    data.registered_slots.push(RegisteredSlot::from_pair(&p));
    data.pairs.push(p);
    for index in 0..2 {
        let mut c = data.candidates[index].clone();
        c.pair_id = "P9".into();
        c.candidate_id = format!("P9-{:?}", c.requested_variant);
        data.candidates.push(c);
    }
    let report = summary(&data);
    let cohort = report
        .participant_preferences
        .iter()
        .find(|(key, _)| key.task_type == TaskType::T2 && key.contrast == Contrast::AB)
        .unwrap()
        .1;
    assert_eq!(cohort.per_run.len(), 2);
    assert_eq!(cohort.median, Some(0.75));
}

#[test]
fn task_privacy_and_time_boundaries_reject_inconsistent_records() {
    let original = fixtures::fixture();
    let mut data = original.clone();
    data.pairs[0].conditions.path = Path::EndpointOriginal;
    reject(&data, ErrorKind::Task);
    let mut data = original.clone();
    data.candidates[0].carries_exempted_original = Measured::Value(true);
    reject(&data, ErrorKind::Privacy);
    let mut data = original.clone();
    data.candidates[0].delivery = true;
    reject(&data, ErrorKind::Privacy);
    let mut data = original.clone();
    data.pairs[0].pair_started_at = Some(10000);
    reject(&data, ErrorKind::Time);
    let mut data = original;
    data.candidates[0].unsupported_claim = Measured::NotApplicable;
    reject(&data, ErrorKind::Task);
}

#[test]
fn preregistered_author_slots_are_fixed_and_balanced_before_any_output() {
    let slots = pairs::author_slots(42);
    assert_eq!(pairs::validate_author_slots(&slots), Ok(()));
    assert_eq!(slots.len(), 30);
    assert_eq!(slots, pairs::author_slots(42));
    assert_eq!(slots[0].presentation_seed, 43);
    for (contrast, count) in [(Contrast::AB, 15), (Contrast::AC, 8), (Contrast::BC, 7)] {
        let rows: Vec<_> = slots
            .iter()
            .filter(|slot| slot.contrast == contrast)
            .collect();
        assert_eq!(rows.len(), count);
        let normal = rows
            .iter()
            .filter(|slot| (slot.left_variant, slot.right_variant) == contrast.variants())
            .count();
        assert!(normal.abs_diff(count - normal) <= 1);
    }
    for task in [TaskType::T1, TaskType::T2, TaskType::T3] {
        assert_eq!(
            slots.iter().filter(|slot| slot.task_type == task).count(),
            10
        );
    }
    let mut invalid = slots.clone();
    invalid[1].task_slot = invalid[0].task_slot;
    assert_eq!(
        pairs::validate_author_slots(&invalid),
        Err(pairs::SlotError::DuplicateSlot)
    );
    let mut invalid = slots.clone();
    invalid[0].right_variant = invalid[0].left_variant;
    assert_eq!(
        pairs::validate_author_slots(&invalid),
        Err(pairs::SlotError::SideMapping)
    );
    let mut invalid = slots.clone();
    invalid.pop();
    assert_eq!(
        pairs::validate_author_slots(&invalid),
        Err(pairs::SlotError::Count)
    );
    let mut invalid = slots;
    for slot in &mut invalid {
        slot.task_type = TaskType::T1;
    }
    assert_eq!(
        pairs::validate_author_slots(&invalid),
        Err(pairs::SlotError::TaskBalance)
    );
}

#[test]
fn run_admission_checks_grant_start_and_deadline_without_invalidating_history() {
    let mut data = fixtures::fixture();
    let run = &mut data.runs[0];
    assert!(!run.can_start(0));
    assert!(run.can_start(1));
    assert!(run.can_start(9999));
    assert!(!run.can_start(10000));
    run.consent_state = ConsentState::Withdrawn;
    assert!(!run.can_start(10));
    assert_eq!(summary(&data).funnel.completed, 5);
}

#[test]
fn pair_seed_and_presentation_must_match_the_predeclared_slot() {
    let slot = pairs::author_slots(42).remove(0);
    let mut pair = fixtures::fixture().pairs.remove(0);
    pair.task_slot = slot.task_slot;
    pair.task_type = slot.task_type;
    pair.contrast = slot.contrast;
    pair.left_variant = slot.left_variant;
    pair.right_variant = slot.right_variant;
    pair.presentation_seed = slot.presentation_seed;
    assert_eq!(pairs::validate_pair_slot(&pair, &slot), Ok(()));
    pair.presentation_seed += 1;
    assert_eq!(
        pairs::validate_pair_slot(&pair, &slot),
        Err(pairs::SlotError::RegistrationMismatch)
    );
    pair.presentation_seed = slot.presentation_seed;
    std::mem::swap(&mut pair.left_variant, &mut pair.right_variant);
    assert_eq!(
        pairs::validate_pair_slot(&pair, &slot),
        Err(pairs::SlotError::RegistrationMismatch)
    );
}

#[test]
fn completed_generation_cannot_also_be_a_transport_failure_or_policy_refusal() {
    let mut data = fixtures::fixture();
    data.candidates[1].failure_code = FailureCode::TransportFailure;
    reject(&data, ErrorKind::Stage);
    let mut data = fixtures::fixture();
    data.candidates[1].refusal_code = Measured::Value(RefusalCode::Approval);
    reject(&data, ErrorKind::Stage);
}

fn stop_after_generation(data: &mut Evaluation, pair_index: usize, candidate_index: usize) {
    let pair = &mut data.pairs[pair_index];
    pair.pair_completion_status = PairCompletion::SafetyStop;
    pair.pair_completed_at = None;
    let candidate = &mut data.candidates[candidate_index];
    candidate.candidate_completion_status = CandidateCompletion::SafetyStop;
    candidate.privacy_or_safety_stop = Measured::Value(true);
    candidate.failure_code = FailureCode::SafetyStop;
    candidate.edit_seconds = Measured::Missing(MissingReason::NotCompleted);
    candidate.missing_reason = Some(MissingReason::NotCompleted);
}

#[test]
fn safety_stopped_pair_keeps_historical_flow_and_event_without_utility_scores() {
    let mut data = fixtures::fixture();
    stop_after_generation(&mut data, 0, 1);
    let report = summary(&data);
    assert_eq!(report.funnel.generated, 5);
    assert_eq!(report.funnel.rated, 3);
    assert_eq!(report.funnel.completed, 4);
    assert_eq!(report.generated_candidates, 11);
    let ab = pair_group(&report, TaskType::T2, Contrast::AB);
    assert_eq!(ab.funnel.assigned, 5);
    assert_eq!(ab.safety_excluded_pairs, 1);
    assert_eq!(ab.preference.denominator, 2);
    assert_eq!(ab.edits.len(), 2);
    assert_eq!(
        accepted(
            &report,
            TaskType::T2,
            Variant::A,
            SourceLayer::UndegradedEndpoint
        ),
        Fraction {
            numerator: 2,
            denominator: 3
        }
    );
    assert_eq!(
        accepted(
            &report,
            TaskType::T2,
            Variant::B,
            SourceLayer::UndegradedEndpoint
        ),
        Fraction {
            numerator: 1,
            denominator: 2
        }
    );
    assert_eq!(
        report
            .candidates
            .values()
            .map(|row| row.safety_stops.numerator)
            .sum::<usize>(),
        1
    );
    assert_eq!(
        report
            .candidates
            .values()
            .map(|row| row.safety_stops.denominator)
            .sum::<usize>(),
        16
    );
}

#[test]
fn safety_stopped_t3_pair_keeps_events_but_excludes_first_pass_and_fact_utility() {
    let mut data = fixtures::fixture();
    stop_after_generation(&mut data, 5, 10);
    let report = summary(&data);
    let rows: Vec<_> = report
        .candidates
        .iter()
        .filter(|(key, _)| key.pair.task_type == TaskType::T3)
        .map(|(_, row)| row)
        .collect();
    assert_eq!(rows.iter().map(|row| row.generated).sum::<usize>(), 2);
    assert_eq!(
        rows.iter()
            .map(|row| row.safety_stops.numerator)
            .sum::<usize>(),
        1
    );
    assert!(rows
        .iter()
        .all(|row| row.first_pass.denominator == 0 && row.fact_correct.denominator == 0));
}

#[test]
fn main_validation_consumes_unique_predeclared_slots_before_summarizing() {
    let original = fixtures::fixture();
    assert_eq!(original.registered_slots.len(), 8); // Arithmetic fixture is not the 30-slot plan.
    let mut future = original.clone();
    let mut slot = future.registered_slots[0].clone();
    slot.task_slot = 99;
    slot.pair_id = "future-unassigned-slot".into();
    future.registered_slots.push(slot);
    assert_eq!(summary(&future).funnel.assigned, 8); // Registration alone is not assignment.
    let mut data = original.clone();
    data.pairs[0].presentation_seed += 1;
    reject(&data, ErrorKind::Registration);
    let mut data = original.clone();
    data.pairs[0].conditions.model_version = Reported::Known("changed-after-assignment".into());
    reject(&data, ErrorKind::Registration);
    let mut data = original.clone();
    let pair = &mut data.pairs[0];
    std::mem::swap(&mut pair.left_variant, &mut pair.right_variant);
    pair.preference = Preference::Left;
    reject(&data, ErrorKind::Registration);
    let mut data = original.clone();
    data.registered_slots.remove(0);
    reject(&data, ErrorKind::Registration);
    let mut data = original.clone();
    data.registered_slots.push(data.registered_slots[0].clone());
    reject(&data, ErrorKind::Registration);
    let mut data = original;
    data.registered_slots[0].evaluation_run = "missing-run".into();
    reject(&data, ErrorKind::Registration);
}
