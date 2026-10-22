//! Relationship and stage validation. Only validated records enter arithmetic.
use super::records::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    DuplicateId,
    MissingRun,
    MissingPair,
    CandidateCount,
    Contrast,
    Stage,
    Brief,
    Source,
    Task,
    Time,
    Privacy,
    Registration,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordError {
    pub record_id: String,
    pub kind: ErrorKind,
}
pub struct ValidatedEvaluation<'a> {
    pub(super) data: &'a Evaluation,
}

fn error(errors: &mut Vec<RecordError>, id: &str, kind: ErrorKind) {
    errors.push(RecordError {
        record_id: id.into(),
        kind,
    });
}

pub fn validate(data: &Evaluation) -> Result<ValidatedEvaluation<'_>, Vec<RecordError>> {
    let mut errors = Vec::new();
    let mut run_ids = BTreeSet::new();
    let mut pair_ids = BTreeSet::new();
    let mut candidate_ids = BTreeSet::new();
    let mut task_slots = BTreeSet::new();
    for run in &data.runs {
        if run.evaluation_run.is_empty() || !run_ids.insert(&run.evaluation_run) {
            error(&mut errors, &run.evaluation_run, ErrorKind::DuplicateId);
        }
        if run.run_started_at >= run.run_deadline_at {
            error(&mut errors, &run.evaluation_run, ErrorKind::Time);
        }
    }
    let mut registered_keys = BTreeSet::new();
    let mut registered_pair_ids = BTreeSet::new();
    for slot in &data.registered_slots {
        let sides = slot.contrast.variants();
        let valid_sides = (slot.left_variant, slot.right_variant) == sides
            || (slot.right_variant, slot.left_variant) == sides;
        if !run_ids.contains(&slot.evaluation_run)
            || slot.pair_id.is_empty()
            || slot.task_slot == 0
            || !registered_keys.insert((&slot.evaluation_run, slot.task_slot))
            || !registered_pair_ids.insert(&slot.pair_id)
            || !valid_sides
            || !valid_task_shape(slot.task_type, slot.semantic_stratum, &slot.conditions)
        {
            error(&mut errors, &slot.pair_id, ErrorKind::Registration);
        }
    }
    for pair in &data.pairs {
        if pair.pair_id.is_empty()
            || !pair_ids.insert(&pair.pair_id)
            || !task_slots.insert((&pair.evaluation_run, pair.task_slot))
        {
            error(&mut errors, &pair.pair_id, ErrorKind::DuplicateId);
        }
        let run = data
            .runs
            .iter()
            .find(|run| run.evaluation_run == pair.evaluation_run);
        if run.is_none() {
            error(&mut errors, &pair.pair_id, ErrorKind::MissingRun);
        }
        let registered = data.registered_slots.iter().find(|slot| {
            slot.evaluation_run == pair.evaluation_run && slot.task_slot == pair.task_slot
        });
        if registered != Some(&RegisteredSlot::from_pair(pair)) {
            error(&mut errors, &pair.pair_id, ErrorKind::Registration);
        }
        let expected = pair.contrast.variants();
        if (pair.left_variant, pair.right_variant) != expected
            && (pair.right_variant, pair.left_variant) != expected
        {
            error(&mut errors, &pair.pair_id, ErrorKind::Contrast);
        }
        if !valid_task_shape(pair.task_type, pair.semantic_stratum, &pair.conditions) {
            error(&mut errors, &pair.pair_id, ErrorKind::Task);
        }
        if let Some(run) = run {
            if pair
                .pair_started_at
                .is_some_and(|start| start < run.run_started_at || start >= run.run_deadline_at)
                || pair.pair_completed_at.is_some_and(|end| {
                    pair.pair_started_at.is_none_or(|start| end < start)
                        || end > run.run_deadline_at
                })
            {
                error(&mut errors, &pair.pair_id, ErrorKind::Time);
            }
        }
        let candidates: Vec<_> = data
            .candidates
            .iter()
            .filter(|c| c.pair_id == pair.pair_id)
            .collect();
        if candidates.len() != 2 {
            error(&mut errors, &pair.pair_id, ErrorKind::CandidateCount);
        }
        let variants: BTreeSet<_> = candidates.iter().map(|c| c.requested_variant).collect();
        if candidates.len() == 2
            && variants != BTreeSet::from([pair.left_variant, pair.right_variant])
        {
            error(&mut errors, &pair.pair_id, ErrorKind::Contrast);
        }
        let both_completed = candidates.len() == 2
            && candidates
                .iter()
                .all(|c| c.candidate_completion_status == CandidateCompletion::Completed);
        if (pair.pair_completion_status == PairCompletion::Completed) != both_completed
            || (pair.pair_completed_at.is_some()
                != (pair.pair_completion_status == PairCompletion::Completed))
            || (pair.pair_started_at.is_none()
                && candidates
                    .iter()
                    .any(|c| c.generated || matches!(c.brief_build_status, Measured::Value(_))))
            || (pair.preference != Preference::Unrated && !candidates.iter().all(|c| c.generated))
        {
            error(&mut errors, &pair.pair_id, ErrorKind::Stage);
        }
    }
    for candidate in &data.candidates {
        if candidate.candidate_id.is_empty() || !candidate_ids.insert(&candidate.candidate_id) {
            error(&mut errors, &candidate.candidate_id, ErrorKind::DuplicateId);
        }
        let Some(pair) = data
            .pairs
            .iter()
            .find(|pair| pair.pair_id == candidate.pair_id)
        else {
            error(&mut errors, &candidate.candidate_id, ErrorKind::MissingPair);
            continue;
        };
        validate_candidate(candidate, pair, &mut errors);
    }
    if errors.is_empty() {
        Ok(ValidatedEvaluation { data })
    } else {
        Err(errors)
    }
}

fn valid_task_shape(task: TaskType, semantic: SemanticStratum, conditions: &Conditions) -> bool {
    let expected = match task {
        TaskType::T1 => (Path::LocalTemplate, RedactionCondition::LocalOnly),
        TaskType::T2 => (Path::EndpointPlaceholder, RedactionCondition::Placeholder),
        TaskType::T3 => (
            Path::EndpointOriginal,
            RedactionCondition::SingleUseOriginal,
        ),
    };
    (conditions.path, conditions.redaction_condition) == expected
        && ((task == TaskType::T3) != (semantic == SemanticStratum::NotApplicable))
}

fn validate_candidate(c: &Candidate, pair: &Pair, errors: &mut Vec<RecordError>) {
    let id = &c.candidate_id;
    let completed = c.candidate_completion_status == CandidateCompletion::Completed;
    let timing_missing = matches!(
        c.edit_seconds,
        Measured::Missing(MissingReason::NotMeasured)
    ) || matches!(
        c.pause_seconds,
        Measured::Missing(MissingReason::NotMeasured)
    );
    // NotMeasured means a completed timing observation was not made. Existing
    // stage reasons are preserved; it cannot excuse a missing non-timing fact.
    let other_not_measured = [
        matches!(
            c.actual_brief_state,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(
            c.brief_build_status,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(
            c.first_pass_accepted,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(c.help_count, Measured::Missing(MissingReason::NotMeasured)),
        matches!(
            c.unsupported_claim,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(
            c.required_fact_error_count,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(
            c.privacy_or_safety_stop,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(
            c.placeheld_turns,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(
            c.carries_exempted_original,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(
            c.refusal_code,
            Measured::Missing(MissingReason::NotMeasured)
        ),
        matches!(
            c.sanitized_note,
            Measured::Missing(MissingReason::NotMeasured)
        ),
    ]
    .into_iter()
    .any(|missing| missing);
    if (timing_missing && !completed)
        || (timing_missing != (c.missing_reason == Some(MissingReason::NotMeasured)))
        || other_not_measured
    {
        error(errors, id, ErrorKind::Stage);
    }
    let safety_stop = c.privacy_or_safety_stop == Measured::Value(true)
        || c.candidate_completion_status == CandidateCompletion::SafetyStop
        || c.failure_code == FailureCode::SafetyStop;
    if safety_stop
        && (c.privacy_or_safety_stop != Measured::Value(true)
            || c.candidate_completion_status != CandidateCompletion::SafetyStop
            || c.failure_code != FailureCode::SafetyStop)
    {
        error(errors, id, ErrorKind::Stage);
    }
    if c.generated
        && (matches!(c.refusal_code, Measured::Value(_))
            || (c.failure_code != FailureCode::None
                && !(safety_stop && c.failure_code == FailureCode::SafetyStop)
                && !(c.draft_source == DraftSource::ToneTemplate
                    && matches!(
                        c.degraded_reason,
                        DegradedReason::ReplyUnreadable
                            | DegradedReason::ReplyEmpty
                            | DegradedReason::ReplyClinical
                    )
                    && c.failure_code == FailureCode::ResponseRejected)))
    {
        error(errors, id, ErrorKind::Stage);
    }
    if completed
        && (c.failure_code != FailureCode::None || matches!(c.refusal_code, Measured::Value(_)))
    {
        error(errors, id, ErrorKind::Stage);
    }
    let expected_brief = match c.requested_variant {
        Variant::A => ActualBrief::Neutral,
        Variant::B => ActualBrief::FromView,
        Variant::C => ActualBrief::ManualVoice,
    };
    match c.brief_build_status {
        Measured::Value(BriefBuildStatus::Built)
            if c.actual_brief_state != Measured::Value(expected_brief) =>
        {
            error(errors, id, ErrorKind::Brief)
        }
        Measured::Value(BriefBuildStatus::Failed)
            if c.generated
                || c.actual_brief_state != Measured::Value(ActualBrief::Unknown)
                || c.failure_code != FailureCode::BriefBuildFailed =>
        {
            error(errors, id, ErrorKind::Brief)
        }
        Measured::Missing(_) if c.generated => error(errors, id, ErrorKind::Brief),
        Measured::NotApplicable => error(errors, id, ErrorKind::Brief),
        _ => {}
    }
    if c.generated {
        if c.brief_build_status != Measured::Value(BriefBuildStatus::Built) {
            error(errors, id, ErrorKind::Brief);
        }
        let valid_source = match pair.task_type {
            TaskType::T1 => {
                c.draft_source == DraftSource::ToneTemplate
                    && c.degraded_reason == DegradedReason::None
            }
            TaskType::T2 | TaskType::T3 => {
                (c.draft_source == DraftSource::UserEndpoint
                    && c.degraded_reason == DegradedReason::None)
                    || (c.draft_source == DraftSource::ToneTemplate
                        && matches!(
                            c.degraded_reason,
                            DegradedReason::ReplyUnreadable
                                | DegradedReason::ReplyEmpty
                                | DegradedReason::ReplyClinical
                        ))
            }
        };
        if !valid_source {
            error(errors, id, ErrorKind::Source);
        }
        if c.placeheld_turns.value().is_none()
            || c.carries_exempted_original != Measured::Value(pair.task_type == TaskType::T3)
            || c.privacy_or_safety_stop.value().is_none()
        {
            error(errors, id, ErrorKind::Privacy);
        }
        if matches!(c.first_pass_accepted, Measured::NotApplicable) {
            error(errors, id, ErrorKind::Stage);
        }
    } else {
        if c.draft_source != DraftSource::None
            || c.degraded_reason != DegradedReason::NotGenerated
            || c.failure_code == FailureCode::None
        {
            error(errors, id, ErrorKind::Source);
        }
        if c.first_pass_accepted != Measured::NotApplicable
            || c.edit_seconds.value().is_some()
            || completed
        {
            error(errors, id, ErrorKind::Stage);
        }
    }
    if completed
        && (!c.generated
            || !matches!(
                c.edit_seconds,
                Measured::Value(_) | Measured::Missing(MissingReason::NotMeasured)
            )
            || !matches!(
                c.pause_seconds,
                Measured::Value(_) | Measured::Missing(MissingReason::NotMeasured)
            )
            || c.help_count.value().is_none()
            || c.first_pass_accepted.value().is_none()
            || c.privacy_or_safety_stop != Measured::Value(false)
            || (pair.task_type != TaskType::T1 && c.degraded_reason != DegradedReason::None)
            || (pair.task_type == TaskType::T2 && c.unsupported_claim != Measured::Value(false)))
    {
        error(errors, id, ErrorKind::Stage);
    }
    if !completed && c.edit_seconds.value().is_some() {
        error(errors, id, ErrorKind::Stage);
    }
    match pair.task_type {
        TaskType::T1
            if c.unsupported_claim != Measured::NotApplicable
                || c.required_fact_error_count != Measured::NotApplicable =>
        {
            error(errors, id, ErrorKind::Task)
        }
        TaskType::T2
            if c.required_fact_error_count != Measured::NotApplicable
                || (c.generated
                    && (c.unsupported_claim == Measured::NotApplicable
                        || (completed && c.unsupported_claim.value().is_none()))) =>
        {
            error(errors, id, ErrorKind::Task)
        }
        TaskType::T3
            if c.generated
                && (c.unsupported_claim == Measured::NotApplicable
                    || c.required_fact_error_count == Measured::NotApplicable
                    || (completed
                        && (c.unsupported_claim.value().is_none()
                            || c.required_fact_error_count.value().is_none()))) =>
        {
            error(errors, id, ErrorKind::Task)
        }
        _ => {}
    }
    if c.delivery {
        error(errors, id, ErrorKind::Privacy);
    }
    if !c.generated && c.missing_reason.is_none() {
        error(errors, id, ErrorKind::Stage);
    }
}

/// Predeclared author schedule; distinct from the eight-pair arithmetic fixture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssignedSlot {
    pub task_slot: u32,
    pub task_type: TaskType,
    pub contrast: Contrast,
    pub left_variant: Variant,
    pub right_variant: Variant,
    pub presentation_seed: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotError {
    Count,
    DuplicateSlot,
    ContrastBalance,
    TaskBalance,
    SideMapping,
    SideBalance,
    RegistrationMismatch,
}
pub fn validate_pair_slot(pair: &Pair, slot: &AssignedSlot) -> Result<(), SlotError> {
    if (
        pair.task_slot,
        pair.task_type,
        pair.contrast,
        pair.left_variant,
        pair.right_variant,
        pair.presentation_seed,
    ) != (
        slot.task_slot,
        slot.task_type,
        slot.contrast,
        slot.left_variant,
        slot.right_variant,
        slot.presentation_seed,
    ) {
        return Err(SlotError::RegistrationMismatch);
    }
    Ok(())
}
pub fn author_slots(seed: u64) -> Vec<AssignedSlot> {
    let mut contrast_counts = [0_u64; 3];
    (0..30_u32)
        .map(|index| {
            let contrast_index = if index % 2 == 0 {
                0
            } else if index % 4 == 1 {
                1
            } else {
                2
            };
            let contrast = [Contrast::AB, Contrast::AC, Contrast::BC][contrast_index];
            let (mut left_variant, mut right_variant) = contrast.variants();
            if seed.wrapping_add(contrast_counts[contrast_index]) % 2 == 1 {
                std::mem::swap(&mut left_variant, &mut right_variant);
            }
            contrast_counts[contrast_index] += 1;
            AssignedSlot {
                task_slot: index + 1,
                task_type: [TaskType::T1, TaskType::T2, TaskType::T3][index as usize % 3],
                contrast,
                left_variant,
                right_variant,
                presentation_seed: seed.wrapping_add(u64::from(index) + 1),
            }
        })
        .collect()
}
pub fn validate_author_slots(slots: &[AssignedSlot]) -> Result<(), SlotError> {
    if slots.len() != 30 {
        return Err(SlotError::Count);
    }
    let ids: BTreeSet<_> = slots.iter().map(|slot| slot.task_slot).collect();
    if ids.len() != slots.len() {
        return Err(SlotError::DuplicateSlot);
    }
    for slot in slots {
        let expected = slot.contrast.variants();
        if (slot.left_variant, slot.right_variant) != expected
            && (slot.right_variant, slot.left_variant) != expected
        {
            return Err(SlotError::SideMapping);
        }
    }
    let counts = [Contrast::AB, Contrast::AC, Contrast::BC].map(|contrast| {
        slots
            .iter()
            .filter(|slot| slot.contrast == contrast)
            .count()
    });
    if counts[0] < 15 || counts[1].abs_diff(counts[2]) > 1 {
        return Err(SlotError::ContrastBalance);
    }
    let tasks = [TaskType::T1, TaskType::T2, TaskType::T3]
        .map(|task| slots.iter().filter(|slot| slot.task_type == task).count());
    if tasks != [10, 10, 10] {
        return Err(SlotError::TaskBalance);
    }
    for contrast in [Contrast::AB, Contrast::AC, Contrast::BC] {
        let rows: Vec<_> = slots
            .iter()
            .filter(|slot| slot.contrast == contrast)
            .collect();
        let normal = rows
            .iter()
            .filter(|slot| (slot.left_variant, slot.right_variant) == contrast.variants())
            .count();
        if normal.abs_diff(rows.len() - normal) > 1 {
            return Err(SlotError::SideBalance);
        }
    }
    Ok(())
}
