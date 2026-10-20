//! P1–P8 are the design's predeclared synthetic feedback, never observed effects.
use super::records::*;

pub fn conditions(task: TaskType) -> Conditions {
    Conditions {
        path: match task {
            TaskType::T1 => Path::LocalTemplate,
            TaskType::T2 => Path::EndpointPlaceholder,
            TaskType::T3 => Path::EndpointOriginal,
        },
        model_id: Reported::Known("synthetic-model".into()),
        model_version: Reported::Known("synthetic-v1".into()),
        service_parameters: Reported::Known("synthetic-temperature=0".into()),
        context_limit: Reported::Known(4096),
        redaction_condition: match task {
            TaskType::T1 => RedactionCondition::LocalOnly,
            TaskType::T2 => RedactionCondition::Placeholder,
            TaskType::T3 => RedactionCondition::SingleUseOriginal,
        },
    }
}

pub fn fixture() -> Evaluation {
    let runs = vec![
        Run {
            evaluation_run: "synthetic-run-1".into(),
            run_started_at: 1,
            run_deadline_at: 10000,
            participant_scope: ParticipantScope::Author,
            feedback_purpose: FeedbackPurpose::ProductEvaluation,
            consent_state: ConsentState::Granted,
        },
        Run {
            evaluation_run: "synthetic-run-2".into(),
            run_started_at: 1,
            run_deadline_at: 10000,
            participant_scope: ParticipantScope::VolunteerAnonymous,
            feedback_purpose: FeedbackPurpose::ProductEvaluation,
            consent_state: ConsentState::Withdrawn,
        },
    ];
    let mut pairs = Vec::new();
    let mut registered_slots = Vec::new();
    let mut candidates = Vec::new();
    for slot in 1..=8 {
        let pair_id = format!("P{slot}");
        let task = match slot {
            5 => TaskType::T1,
            6 => TaskType::T3,
            _ => TaskType::T2,
        };
        let contrast = match slot {
            5 => Contrast::AC,
            6 => Contrast::BC,
            _ => Contrast::AB,
        };
        let (left, right) = contrast.variants();
        let completed = matches!(slot, 1 | 2 | 3 | 5 | 6);
        pairs.push(Pair {
            evaluation_run: format!("synthetic-run-{}", if slot == 8 { 2 } else { 1 }),
            task_slot: slot,
            pair_id: pair_id.clone(),
            task_type: task,
            semantic_stratum: if task == TaskType::T3 {
                SemanticStratum::Coordination
            } else {
                SemanticStratum::NotApplicable
            },
            contrast,
            left_variant: left,
            right_variant: right,
            presentation_seed: 42 + u64::from(slot),
            comparability_status: if slot == 5 {
                Comparability::NotComparable
            } else {
                Comparability::Comparable
            },
            pair_started_at: (slot != 7).then_some(10),
            pair_completed_at: completed.then_some(200),
            pair_completion_status: if completed {
                PairCompletion::Completed
            } else if slot == 8 {
                PairCompletion::Aborted
            } else if slot == 4 {
                PairCompletion::SystemFailure
            } else {
                PairCompletion::Incomplete
            },
            preference: match slot {
                1 | 5 => Preference::Right,
                2 => Preference::Tie,
                3 | 6 => Preference::Left,
                _ => Preference::Unrated,
            },
            blind_status: if slot == 6 {
                BlindStatus::Broken
            } else {
                BlindStatus::Intact
            },
            conditions: conditions(task),
        });
        registered_slots.push(RegisteredSlot::from_pair(
            pairs.last().expect("just assigned synthetic pair"),
        ));
        for variant in [left, right] {
            let id = format!("{pair_id}-{variant:?}");
            let mut c = Candidate::planned(&pair_id, &id, variant, task);
            let measured = match (slot, variant) {
                (1, Variant::A) => Some((true, 0)),
                (1, Variant::B) => Some((false, 10)),
                (2, Variant::A) => Some((false, 120)),
                (2, Variant::B) => Some((true, 90)),
                (3, Variant::A) => Some((true, 80)),
                (3, Variant::B) => Some((false, 100)),
                (4, Variant::A) => Some((true, 30)),
                (5, _) => Some((false, 60)),
                (6, Variant::B) => Some((true, 20)),
                (6, Variant::C) => Some((false, 40)),
                _ => None,
            };
            if let Some((accepted, seconds)) = measured {
                fill_completed(&mut c, task, accepted, seconds);
                if slot == 6 {
                    c.required_fact_error_count =
                        Measured::Value(if variant == Variant::B { 0 } else { 1 });
                    c.sanitized_note =
                        Measured::Value(SanitizedNote::FactCorrectedBeforeCompletion);
                }
            } else if slot == 4 {
                c.brief_build_status = Measured::Value(BriefBuildStatus::Built);
                c.actual_brief_state = Measured::Value(ActualBrief::FromView);
                c.candidate_completion_status = CandidateCompletion::SystemFailure;
                c.failure_code = FailureCode::TransportFailure;
                c.missing_reason = Some(MissingReason::NotGenerated);
                c.edit_seconds = Measured::Missing(MissingReason::NotCompleted);
                c.pause_seconds = Measured::Missing(MissingReason::NotCompleted);
                c.help_count = Measured::Missing(MissingReason::NotCompleted);
                c.unsupported_claim = Measured::Missing(MissingReason::NotGenerated);
            } else if slot == 8 {
                c.failure_code = FailureCode::ConsentWithdrawnBeforeGeneration;
                c.missing_reason = Some(MissingReason::ConsentWithdrawnBeforeGeneration);
                c.actual_brief_state =
                    Measured::Missing(MissingReason::ConsentWithdrawnBeforeGeneration);
                c.brief_build_status =
                    Measured::Missing(MissingReason::ConsentWithdrawnBeforeGeneration);
                c.unsupported_claim =
                    Measured::Missing(MissingReason::ConsentWithdrawnBeforeGeneration);
                c.edit_seconds = Measured::Missing(MissingReason::NotCompleted);
            }
            candidates.push(c);
        }
    }
    Evaluation {
        runs,
        registered_slots,
        pairs,
        candidates,
    }
}

pub fn fill_completed(c: &mut Candidate, task: TaskType, accepted: bool, seconds: u64) {
    c.actual_brief_state = Measured::Value(match c.requested_variant {
        Variant::A => ActualBrief::Neutral,
        Variant::B => ActualBrief::FromView,
        Variant::C => ActualBrief::ManualVoice,
    });
    c.brief_build_status = Measured::Value(BriefBuildStatus::Built);
    c.generated = true;
    c.draft_source = if task == TaskType::T1 {
        DraftSource::ToneTemplate
    } else {
        DraftSource::UserEndpoint
    };
    c.degraded_reason = DegradedReason::None;
    c.candidate_completion_status = CandidateCompletion::Completed;
    c.first_pass_accepted = Measured::Value(accepted);
    c.edit_seconds = Measured::Value(seconds);
    c.pause_seconds = Measured::Value(0);
    c.help_count = Measured::Value(0);
    c.unsupported_claim = if task == TaskType::T1 {
        Measured::NotApplicable
    } else {
        Measured::Value(false)
    };
    c.required_fact_error_count = if task == TaskType::T3 {
        Measured::Value(0)
    } else {
        Measured::NotApplicable
    };
    c.privacy_or_safety_stop = Measured::Value(false);
    c.failure_code = FailureCode::None;
    c.placeheld_turns = Measured::Value(if task == TaskType::T2 { 1 } else { 0 });
    c.carries_exempted_original = Measured::Value(task == TaskType::T3);
    c.refusal_code = Measured::NotApplicable;
    c.missing_reason = None;
    c.sanitized_note = Measured::Value(if task == TaskType::T1 {
        SanitizedNote::TemplateSlotFilled
    } else {
        SanitizedNote::SyntheticFeedback
    });
}
