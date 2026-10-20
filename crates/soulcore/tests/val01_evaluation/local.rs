//! Local test adapter. Returned draft text is ephemeral and never a record field.
use super::{
    briefs::{build_brief, same_template_voice, ManualVoice},
    pairs::{validate, validate_pair_slot, AssignedSlot},
    records::*,
};
use soul_draft::{
    draft::{Degradation, Draft, DraftRequest, DraftSource as ActualSource, NeverSent},
    error::DraftError,
};
use soul_policy::{hitl::RequestOrigin, redactor::KnownIdentifiers};
use soul_profile::view::ProfileView;
use soulcore::commands::{
    draft::{DraftRefusal, DraftSession},
    policy::PolicySession,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExecutionCounts {
    pub allocated_candidates: usize,
    pub brief_builds: usize,
    pub draft_calls: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    RunClosed,
    PathPaused,
    InvalidRecords,
    Registration,
    AlreadyStarted,
    WrongTask,
}

/// Shared by local and endpoint test adapters, before new allocation or construction.
/// `now` is a synthetic monotonic second, not a claim of real consent collection.
pub fn admit_slot(
    data: &Evaluation,
    slot: &RegisteredSlot,
    now: u64,
) -> Result<(), AdmissionError> {
    let run = data
        .runs
        .iter()
        .find(|run| run.evaluation_run == slot.evaluation_run)
        .ok_or(AdmissionError::InvalidRecords)?;
    if !run.can_start(now) {
        return Err(AdmissionError::RunClosed);
    }
    validate(data).map_err(|_| AdmissionError::InvalidRecords)?;
    let registered = data
        .registered_slots
        .iter()
        .find(|registered| {
            registered.evaluation_run == slot.evaluation_run
                && registered.task_slot == slot.task_slot
        })
        .ok_or(AdmissionError::Registration)?;
    if registered != slot {
        return Err(AdmissionError::Registration);
    }
    validate_pair_slot(
        &pair_for(slot),
        &AssignedSlot {
            task_slot: registered.task_slot,
            task_type: registered.task_type,
            contrast: registered.contrast,
            left_variant: registered.left_variant,
            right_variant: registered.right_variant,
            presentation_seed: registered.presentation_seed,
        },
    )
    .map_err(|_| AdmissionError::Registration)?;
    if data.pairs.iter().any(|pair| {
        pair.pair_id == slot.pair_id
            || (pair.evaluation_run == slot.evaluation_run && pair.task_slot == slot.task_slot)
    }) {
        return Err(AdmissionError::AlreadyStarted);
    }
    let paused = data
        .pairs
        .iter()
        .filter(|pair| {
            pair.evaluation_run == slot.evaluation_run
                && pair.conditions.path == slot.conditions.path
        })
        .any(|pair| {
            pair.pair_completion_status == PairCompletion::SafetyStop
                || data.candidates.iter().any(|candidate| {
                    candidate.pair_id == pair.pair_id
                        && (candidate.candidate_completion_status
                            == CandidateCompletion::SafetyStop
                            || candidate.privacy_or_safety_stop == Measured::Value(true)
                            || candidate.failure_code == FailureCode::SafetyStop)
                })
        });
    if paused {
        return Err(AdmissionError::PathPaused);
    }
    // A caller cannot collide with an earlier candidate's opaque identity.
    for variant in [slot.left_variant, slot.right_variant] {
        let id = candidate_id(slot, variant);
        if data
            .candidates
            .iter()
            .any(|candidate| candidate.candidate_id == id)
        {
            return Err(AdmissionError::InvalidRecords);
        }
    }
    Ok(())
}

pub fn run_local_pair(
    data: &mut Evaluation,
    slot: &RegisteredSlot,
    now: u64,
    snapshot: &ProfileView,
    manual: ManualVoice,
    pasted: &str,
    counts: &mut ExecutionCounts,
) -> Result<Vec<Draft>, AdmissionError> {
    if slot.task_type != TaskType::T1 {
        return Err(AdmissionError::WrongTask);
    }
    admit_slot(data, slot, now)?;
    let mut pair = pair_for(slot);
    pair.pair_started_at = Some(now);
    let mut candidates = Vec::new();
    let mut briefs = Vec::new();
    for variant in [slot.left_variant, slot.right_variant] {
        counts.allocated_candidates += 1;
        let mut candidate = Candidate::planned(
            &slot.pair_id,
            &candidate_id(slot, variant),
            variant,
            TaskType::T1,
        );
        counts.brief_builds += 1;
        match build_brief(variant, snapshot, manual) {
            Ok(brief) => {
                candidate.brief_build_status = Measured::Value(BriefBuildStatus::Built);
                candidate.actual_brief_state = Measured::Value(match variant {
                    Variant::A => ActualBrief::Neutral,
                    Variant::B => ActualBrief::FromView,
                    Variant::C => ActualBrief::ManualVoice,
                });
                briefs.push(Some(brief));
            }
            Err(_) => {
                candidate.brief_build_status = Measured::Value(BriefBuildStatus::Failed);
                candidate.actual_brief_state = Measured::Value(ActualBrief::Unknown);
                candidate.candidate_completion_status = CandidateCompletion::SystemFailure;
                candidate.failure_code = FailureCode::BriefBuildFailed;
                candidate.missing_reason = Some(MissingReason::BriefBuildFailed);
                candidate.edit_seconds = Measured::Missing(MissingReason::BriefBuildFailed);
                candidate.pause_seconds = Measured::Missing(MissingReason::BriefBuildFailed);
                candidate.help_count = Measured::Missing(MissingReason::BriefBuildFailed);
                briefs.push(None);
                pair.pair_completion_status = PairCompletion::SystemFailure;
            }
        }
        candidates.push(candidate);
    }
    pair.comparability_status = match (&briefs[0], &briefs[1]) {
        (Some(left), Some(right)) if !same_template_voice(left, right) => Comparability::Comparable,
        _ => Comparability::NotComparable,
    };
    let mut drafts = Vec::new();
    for (candidate, brief) in candidates.iter_mut().zip(briefs) {
        let Some(brief) = brief else { continue };
        let drafting = DraftSession::new("synthetic-local-template", KnownIdentifiers::new());
        let mut policy = PolicySession::closed();
        let request = DraftRequest::from_paste(brief, pasted);
        counts.draft_calls += 1;
        match drafting.draft_offline(
            &mut policy,
            &request,
            RequestOrigin::User,
            now.saturating_mul(1000),
        ) {
            Ok(drafted) => {
                record_draft(candidate, &drafted.draft);
                drafts.push(drafted.draft);
            }
            Err(refusal) => {
                let safety = matches!(refusal, DraftRefusal::Draft(DraftError::Clinical(_)));
                candidate.candidate_completion_status = if safety {
                    CandidateCompletion::SafetyStop
                } else {
                    CandidateCompletion::SystemFailure
                };
                candidate.failure_code = if safety {
                    FailureCode::SafetyStop
                } else {
                    FailureCode::ResponseRejected
                };
                candidate.refusal_code = if safety {
                    Measured::Value(RefusalCode::Safety)
                } else {
                    Measured::NotApplicable
                };
                candidate.privacy_or_safety_stop = Measured::Value(safety);
                candidate.missing_reason = Some(MissingReason::NotGenerated);
                pair.pair_completion_status = if safety {
                    PairCompletion::SafetyStop
                } else {
                    PairCompletion::SystemFailure
                };
                // Any safety failure stops this path, including the other candidate.
                if safety {
                    break;
                }
            }
        }
    }
    data.pairs.push(pair);
    data.candidates.extend(candidates);
    validate(data).map_err(|_| AdmissionError::InvalidRecords)?;
    Ok(drafts)
}

fn candidate_id(slot: &RegisteredSlot, variant: Variant) -> String {
    format!("{}-{variant:?}", slot.pair_id)
}

fn pair_for(slot: &RegisteredSlot) -> Pair {
    Pair {
        evaluation_run: slot.evaluation_run.clone(),
        task_slot: slot.task_slot,
        pair_id: slot.pair_id.clone(),
        task_type: slot.task_type,
        semantic_stratum: slot.semantic_stratum,
        contrast: slot.contrast,
        left_variant: slot.left_variant,
        right_variant: slot.right_variant,
        presentation_seed: slot.presentation_seed,
        comparability_status: Comparability::Comparable,
        pair_started_at: None,
        pair_completed_at: None,
        pair_completion_status: PairCompletion::Incomplete,
        preference: Preference::Unrated,
        blind_status: BlindStatus::Intact,
        conditions: slot.conditions.clone(),
    }
}

fn record_draft(candidate: &mut Candidate, draft: &Draft) {
    candidate.generated = true;
    candidate.draft_source = match draft.source {
        ActualSource::ToneTemplate => DraftSource::ToneTemplate,
        ActualSource::UserEndpoint => DraftSource::UserEndpoint,
    };
    candidate.degraded_reason = match draft.degraded {
        None => DegradedReason::None,
        Some(Degradation::ReplyUnreadable) => DegradedReason::ReplyUnreadable,
        Some(Degradation::ReplyEmpty) => DegradedReason::ReplyEmpty,
        Some(Degradation::ReplyClinical) => DegradedReason::ReplyClinical,
    };
    candidate.failure_code = FailureCode::None;
    candidate.first_pass_accepted = Measured::Missing(MissingReason::NotRated);
    candidate.edit_seconds = Measured::Missing(MissingReason::NotRated);
    candidate.pause_seconds = Measured::Missing(MissingReason::NotRated);
    candidate.help_count = Measured::Missing(MissingReason::NotRated);
    candidate.placeheld_turns = Measured::Value(
        u32::try_from(draft.placeheld_turns).expect("single-paste turn count fits u32"),
    );
    candidate.carries_exempted_original = Measured::Value(draft.carries_exempted_original);
    candidate.privacy_or_safety_stop = Measured::Value(false);
    candidate.sanitized_note = Measured::Missing(MissingReason::NotRated);
    candidate.missing_reason = Some(MissingReason::NotRated);
    candidate.delivery = draft.delivery != NeverSent;
}
