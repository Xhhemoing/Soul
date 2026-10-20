//! Synthetic V2 behavior, without fabricated completion or human ratings.
use super::{
    aggregate::summarize,
    briefs::{build_brief, same_template_voice, ManualVoice},
    local::{admit_slot, run_local_pair, AdmissionError, ExecutionCounts},
    pairs::validate,
    records::*,
};
use soul_draft::{draft::DraftSource as ActualSource, template::BODY_SLOT, ProfileBrief};
use soul_policy::clinical::WORKING_HYPOTHESIS_NOTICE;
use soul_profile::{
    view::{AxisView, ProfileView},
    voice::{EmojiUse, VoiceDirectness, VoiceProfile, VoiceRegister, VoiceWarmth},
};
use soul_schema::{common::EvidenceBand, profile::AxisPosition};
use uuid::Uuid;

fn manual() -> ManualVoice {
    ManualVoice {
        register: VoiceRegister::Formal,
        directness: VoiceDirectness::Direct,
        warmth: VoiceWarmth::Warm,
        emoji_use: EmojiUse::Never,
    }
}

fn snapshot(reading: &str) -> ProfileView {
    ProfileView {
        profile_id: Uuid::from_u128(100),
        voice: VoiceProfile::default(),
        notice: WORKING_HYPOTHESIS_NOTICE,
        axes: vec![AxisView {
            axis_id: Uuid::from_u128(101),
            label: "合成轴".into(),
            position: AxisPosition::Mixed,
            evidence_band: EvidenceBand::Weak,
            locked_by_user: false,
            evidence: vec![],
            inferences: vec![],
            reading: reading.into(),
        }],
    }
}

fn denied_reading() -> &'static str {
    include_str!("../../../../fixtures/denylist/diagnostic_terms.txt")
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .expect("existing denylist term")
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

fn evaluation(contrast: Contrast) -> (Evaluation, RegisteredSlot) {
    let (left, right) = contrast.variants();
    let slot = RegisteredSlot {
        evaluation_run: "synthetic-v2".into(),
        task_slot: 1,
        pair_id: "local-1".into(),
        task_type: TaskType::T1,
        semantic_stratum: SemanticStratum::NotApplicable,
        contrast,
        left_variant: left,
        right_variant: right,
        presentation_seed: 42,
        conditions: Conditions {
            path: Path::LocalTemplate,
            model_id: Reported::NotExposed,
            model_version: Reported::NotExposed,
            service_parameters: Reported::NotExposed,
            context_limit: Reported::NotExposed,
            redaction_condition: RedactionCondition::LocalOnly,
        },
    };
    let data = Evaluation {
        runs: vec![Run {
            evaluation_run: slot.evaluation_run.clone(),
            run_started_at: 10,
            run_deadline_at: 100,
            participant_scope: ParticipantScope::Author,
            feedback_purpose: FeedbackPurpose::ProductEvaluation,
            consent_state: ConsentState::Granted,
        }],
        registered_slots: vec![slot.clone()],
        pairs: vec![],
        candidates: vec![],
    };
    (data, slot)
}

fn execute(
    data: &mut Evaluation,
    slot: &RegisteredSlot,
    now: u64,
    view: &ProfileView,
    counts: &mut ExecutionCounts,
) -> Result<Vec<soul_draft::draft::Draft>, AdmissionError> {
    run_local_pair(
        data,
        slot,
        now,
        view,
        manual(),
        "合成内容：周五确认；不应出现在本机语气骨架。",
        counts,
    )
}

#[test]
fn explicit_briefs_preserve_neutral_snapshot_and_four_manual_enums() {
    let mut view = snapshot("倾向在充分了解之后再安排下一步");
    view.voice.warmth = VoiceWarmth::Cool;
    let a = build_brief(Variant::A, &view, manual()).expect("A");
    assert_eq!(a, ProfileBrief::neutral());
    let b = build_brief(Variant::B, &view, manual()).expect("B");
    assert_eq!(b.voice(), &view.voice);
    assert_eq!(b.readings().len(), 1);
    assert_eq!(b.readings()[0].text, "倾向在充分了解之后再安排下一步");
    let c = build_brief(Variant::C, &view, manual()).expect("C");
    assert_eq!(c.voice().register, VoiceRegister::Formal);
    assert_eq!(c.voice().directness, VoiceDirectness::Direct);
    assert_eq!(c.voice().warmth, VoiceWarmth::Warm);
    assert_eq!(c.voice().emoji_use, EmojiUse::Never);
    assert_eq!(c.locked_fields().len(), 4);
    assert!(c.readings().is_empty());
    assert_eq!(
        view.voice.warmth,
        VoiceWarmth::Cool,
        "one borrowed snapshot is unchanged"
    );
}

#[test]
fn failed_b_is_not_replaced_with_a_neutral_brief() {
    assert!(build_brief(Variant::B, &snapshot(denied_reading()), manual()).is_err());
}

#[test]
fn template_comparison_ignores_locks_and_axis_readings() {
    let a = ProfileBrief::neutral();
    let b = build_brief(Variant::B, &snapshot("合成非临床要点"), manual()).expect("B");
    let default_manual = ManualVoice {
        register: VoiceRegister::Plain,
        directness: VoiceDirectness::Balanced,
        warmth: VoiceWarmth::Even,
        emoji_use: EmojiUse::Sparing,
    };
    let c = build_brief(Variant::C, &snapshot("合成要点"), default_manual).expect("C");
    assert_ne!(a.voice(), c.voice(), "user_set metadata differs");
    assert!(same_template_voice(&a, &b));
    assert!(same_template_voice(&a, &c));
    assert!(!same_template_voice(
        &a,
        &build_brief(Variant::C, &snapshot("合成要点"), manual()).expect("C")
    ));
}

#[test]
fn real_template_observations_validate_and_summarize_without_human_scores() {
    let (mut data, slot) = evaluation(Contrast::AC);
    let mut counts = ExecutionCounts::default();
    let drafts =
        execute(&mut data, &slot, 20, &snapshot("合成要点"), &mut counts).expect("local execution");
    assert_eq!(
        counts,
        ExecutionCounts {
            allocated_candidates: 2,
            brief_builds: 2,
            draft_calls: 2
        }
    );
    assert_eq!(drafts.len(), 2);
    assert_ne!(drafts[0].text, drafts[1].text);
    for draft in &drafts {
        assert_eq!(draft.source, ActualSource::ToneTemplate);
        assert!(draft.text.contains(BODY_SLOT));
        assert!(!draft.text.contains("周五确认"));
        assert_eq!(
            serde_json::to_value(draft.delivery).expect("delivery"),
            false
        );
        // Offline rendering uses request.context(), but builds no redacted body;
        // its returned egress metadata comes from BodyFacts::default().
        assert_eq!(draft.third_party_turns, 0);
        assert_eq!(draft.placeheld_turns, 0);
        assert!(!draft.carries_exempted_original);
        assert_eq!(draft.degraded, None);
    }
    for candidate in &data.candidates {
        assert!(candidate.generated);
        assert_eq!(
            candidate.first_pass_accepted,
            Measured::Missing(MissingReason::NotRated)
        );
        assert_eq!(
            candidate.edit_seconds,
            Measured::Missing(MissingReason::NotRated)
        );
        assert_eq!(
            candidate.pause_seconds,
            Measured::Missing(MissingReason::NotRated)
        );
        assert_eq!(
            candidate.help_count,
            Measured::Missing(MissingReason::NotRated)
        );
        assert_eq!(
            candidate.candidate_completion_status,
            CandidateCompletion::Incomplete
        );
        assert_eq!(candidate.draft_source, DraftSource::ToneTemplate);
        assert_eq!(candidate.degraded_reason, DegradedReason::None);
        assert_eq!(candidate.placeheld_turns, Measured::Value(0));
        assert!(!candidate.delivery);
    }
    assert_eq!(
        data.pairs[0].comparability_status,
        Comparability::Comparable
    );
    let report = summarize(&validate(&data).expect("actual observations validate"));
    assert_eq!(
        (
            report.funnel.assigned,
            report.funnel.started,
            report.funnel.generated,
            report.funnel.rated,
            report.funnel.completed
        ),
        (1, 1, 1, 0, 0)
    );
    assert_eq!(report.generated_candidates, 2);
    assert_eq!(
        report
            .candidates
            .values()
            .map(|row| row.first_pass_missing)
            .sum::<usize>(),
        2
    );
    assert!(report.participant_preferences.is_empty());
}

#[test]
fn identical_effective_voice_marks_the_executed_t1_pair_not_comparable() {
    let (mut data, slot) = evaluation(Contrast::AB);
    let drafts = execute(
        &mut data,
        &slot,
        20,
        &snapshot("仅 B 携带的合成要点"),
        &mut ExecutionCounts::default(),
    )
    .expect("local execution");
    assert_eq!(drafts[0].text, drafts[1].text);
    assert_eq!(
        data.pairs[0].comparability_status,
        Comparability::NotComparable
    );
    assert!(validate(&data).is_ok());
}

#[test]
fn failed_b_records_unknown_and_never_calls_its_generation() {
    let (mut data, slot) = evaluation(Contrast::AB);
    let mut counts = ExecutionCounts::default();
    let drafts = execute(
        &mut data,
        &slot,
        20,
        &snapshot(denied_reading()),
        &mut counts,
    )
    .expect("failure recorded");
    assert_eq!(drafts.len(), 1);
    assert_eq!(
        counts,
        ExecutionCounts {
            allocated_candidates: 2,
            brief_builds: 2,
            draft_calls: 1
        }
    );
    let b = data
        .candidates
        .iter()
        .find(|c| c.requested_variant == Variant::B)
        .expect("planned B retained");
    assert_eq!(b.actual_brief_state, Measured::Value(ActualBrief::Unknown));
    assert_eq!(
        b.brief_build_status,
        Measured::Value(BriefBuildStatus::Failed)
    );
    assert_eq!(b.failure_code, FailureCode::BriefBuildFailed);
    assert!(!b.generated);
    assert_eq!(b.draft_source, DraftSource::None);
    assert_eq!(
        data.pairs[0].comparability_status,
        Comparability::NotComparable
    );
    let report = summarize(&validate(&data).expect("failed brief validates"));
    assert_eq!(report.generated_candidates, 1);
    assert_eq!(report.funnel.generated, 0);
}

#[test]
fn withdrawn_before_start_and_deadline_do_no_work_or_allocate_candidates() {
    for (consent, now) in [
        (ConsentState::Withdrawn, 20),
        (ConsentState::Granted, 9),
        (ConsentState::Granted, 100),
    ] {
        let (mut data, slot) = evaluation(Contrast::AB);
        data.runs[0].consent_state = consent;
        let mut counts = ExecutionCounts::default();
        assert_eq!(
            execute(
                &mut data,
                &slot,
                now,
                &snapshot(denied_reading()),
                &mut counts
            ),
            Err(AdmissionError::RunClosed)
        );
        assert_eq!(counts, ExecutionCounts::default());
        assert!(data.pairs.is_empty());
        assert!(data.candidates.is_empty());
        assert_eq!(data.registered_slots, vec![slot]);
    }
}

#[test]
fn altered_registration_and_reexecution_are_rejected_before_work() {
    let (mut data, slot) = evaluation(Contrast::AC);
    let mut altered = slot.clone();
    altered.presentation_seed += 1;
    let mut counts = ExecutionCounts::default();
    assert_eq!(
        execute(&mut data, &altered, 20, &snapshot("合成要点"), &mut counts),
        Err(AdmissionError::Registration)
    );
    assert_eq!(counts, ExecutionCounts::default());
    execute(&mut data, &slot, 20, &snapshot("合成要点"), &mut counts).expect("first execution");
    let before = counts;
    assert_eq!(
        execute(&mut data, &slot, 21, &snapshot("合成要点"), &mut counts),
        Err(AdmissionError::AlreadyStarted)
    );
    assert_eq!(counts, before);
    assert_eq!(data.pairs.len(), 1);
    assert_eq!(data.candidates.len(), 2);
}

#[test]
fn malformed_candidate_identity_is_rejected_before_next_assignment() {
    let (mut data, slot) = evaluation(Contrast::AC);
    execute(
        &mut data,
        &slot,
        20,
        &snapshot("合成要点"),
        &mut ExecutionCounts::default(),
    )
    .expect("first execution");
    data.candidates[1].candidate_id = data.candidates[0].candidate_id.clone();
    let mut next = slot.clone();
    next.task_slot = 2;
    next.pair_id = "local-2".into();
    data.registered_slots.push(next.clone());
    let mut counts = ExecutionCounts::default();
    assert_eq!(
        execute(&mut data, &next, 21, &snapshot("合成要点"), &mut counts),
        Err(AdmissionError::InvalidRecords)
    );
    assert_eq!(counts, ExecutionCounts::default());
    assert_eq!(data.pairs.len(), 1);
}

#[test]
fn safety_stop_pauses_the_same_run_path_without_globally_pausing_other_paths() {
    let (mut data, next) = evaluation(Contrast::AB);
    let mut previous = next.clone();
    previous.task_slot = 2;
    previous.pair_id = "stopped-local".into();
    data.registered_slots.push(previous.clone());
    let mut pair = pair_for(&previous);
    pair.pair_started_at = Some(15);
    pair.pair_completion_status = PairCompletion::SafetyStop;
    data.pairs.push(pair);
    for variant in [Variant::A, Variant::B] {
        let mut c = Candidate::planned(
            &previous.pair_id,
            &format!("stopped-{variant:?}"),
            variant,
            TaskType::T1,
        );
        c.candidate_completion_status = CandidateCompletion::SafetyStop;
        c.privacy_or_safety_stop = Measured::Value(true);
        c.failure_code = FailureCode::SafetyStop;
        c.missing_reason = Some(MissingReason::NotGenerated);
        data.candidates.push(c);
    }
    assert!(validate(&data).is_ok(), "valid synthetic safety history");
    let mut counts = ExecutionCounts::default();
    assert_eq!(
        execute(
            &mut data,
            &next,
            20,
            &snapshot(denied_reading()),
            &mut counts
        ),
        Err(AdmissionError::PathPaused)
    );
    assert_eq!(counts, ExecutionCounts::default());
    assert_eq!(data.pairs.len(), 1);
    let mut endpoint = next;
    endpoint.task_slot = 3;
    endpoint.pair_id = "separate-placeholder-path".into();
    endpoint.task_type = TaskType::T2;
    endpoint.conditions.path = Path::EndpointPlaceholder;
    endpoint.conditions.redaction_condition = RedactionCondition::Placeholder;
    data.registered_slots.push(endpoint.clone());
    assert_eq!(admit_slot(&data, &endpoint, 20), Ok(()));
}
