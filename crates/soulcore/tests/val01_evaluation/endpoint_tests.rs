//! Synthetic endpoint observations, not human ratings or effect measurements.
use super::{
    aggregate::summarize,
    briefs::ManualVoice,
    endpoint::{EndpointError, EndpointInput, EndpointPair, MODEL},
    endpoint_transport::RawEndpoint,
    local::AdmissionError,
    pairs::validate,
    records::*,
};
use soul_policy::{
    clinical::WORKING_HYPOTHESIS_NOTICE,
    redactor::{KnownIdentifiers, ACCOUNT_PLACEHOLDER, NAME_PLACEHOLDER, THIRD_PARTY_PLACEHOLDER},
};
use soul_profile::{
    view::{AxisView, ProfileView},
    voice::{EmojiUse, VoiceDirectness, VoiceProfile, VoiceRegister, VoiceWarmth},
};
use soul_schema::{common::EvidenceBand, profile::AxisPosition};
use soul_testkit::mock_llm::MockLlm;
use uuid::Uuid;

pub(super) const PASTE: &str = "青 岚和青岚请周五确认青色会场；账号 memberalpha、@synth_handle、synthetic@example.invalid、138 0013 8000。";
const REPLY: &str = "收到，我会先核对安排，再向你确认下一步。";

pub(super) fn manual() -> ManualVoice {
    ManualVoice {
        register: VoiceRegister::Formal,
        directness: VoiceDirectness::Direct,
        warmth: VoiceWarmth::Warm,
        emoji_use: EmojiUse::Never,
    }
}

pub(super) fn snapshot() -> ProfileView {
    ProfileView {
        profile_id: Uuid::from_u128(401),
        voice: VoiceProfile::default(),
        notice: WORKING_HYPOTHESIS_NOTICE,
        axes: vec![AxisView {
            axis_id: Uuid::from_u128(402),
            label: "合成轴".into(),
            position: AxisPosition::Mixed,
            evidence_band: EvidenceBand::Weak,
            locked_by_user: false,
            evidence: vec![],
            inferences: vec![],
            reading: "倾向先核对安排再继续沟通".into(),
        }],
    }
}

pub(super) fn evaluation(task: TaskType) -> (Evaluation, RegisteredSlot) {
    let slot = RegisteredSlot {
        evaluation_run: "synthetic-v3".into(),
        task_slot: 1,
        pair_id: "endpoint-1".into(),
        task_type: task,
        semantic_stratum: if task == TaskType::T3 {
            SemanticStratum::Coordination
        } else {
            SemanticStratum::NotApplicable
        },
        contrast: Contrast::AB,
        left_variant: Variant::A,
        right_variant: Variant::B,
        presentation_seed: 73,
        conditions: Conditions {
            path: if task == TaskType::T3 {
                Path::EndpointOriginal
            } else {
                Path::EndpointPlaceholder
            },
            model_id: Reported::Known(MODEL.into()),
            model_version: Reported::NotExposed,
            service_parameters: Reported::NotExposed,
            context_limit: Reported::NotExposed,
            redaction_condition: if task == TaskType::T3 {
                RedactionCondition::SingleUseOriginal
            } else {
                RedactionCondition::Placeholder
            },
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

fn start(data: &mut Evaluation, slot: &RegisteredSlot, url: Option<&str>) -> EndpointPair {
    EndpointPair::start(
        data,
        slot,
        11,
        EndpointInput {
            snapshot: &snapshot(),
            manual: manual(),
            identifiers: KnownIdentifiers::new()
                .with_name("青 岚")
                .with_account("memberalpha"),
            pasted: PASTE,
            endpoint: url,
        },
    )
    .expect("start registered endpoint pair")
}

pub(super) fn material(body: &str) -> String {
    let json: serde_json::Value = serde_json::from_str(body).expect("actual request JSON");
    json["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .filter_map(|message| message["content"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn assert_unrated(data: &Evaluation) {
    let summary = summarize(&validate(data).expect("actual adapter rows validate"));
    assert_eq!(summary.funnel.rated, 0);
    assert_eq!(summary.funnel.completed, 0);
    for candidate in data
        .candidates
        .iter()
        .filter(|candidate| candidate.generated)
    {
        assert_eq!(
            candidate.candidate_completion_status,
            CandidateCompletion::Incomplete
        );
        assert_eq!(
            candidate.first_pass_accepted,
            Measured::Missing(MissingReason::NotRated)
        );
        assert!(candidate.edit_seconds.value().is_none());
        assert!(candidate.unsupported_claim.value().is_none());
        assert!(candidate.required_fact_error_count.value().is_none());
        assert!(!candidate.delivery);
    }
}

#[test]
fn endpoint_t2_pair_builds_actual_briefs_and_records_unrated_outputs() {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let left = pair.prepare(&mut data, 0, 12).unwrap();
    let right = pair.prepare(&mut data, 1, 13).unwrap();
    assert_eq!(
        endpoint.request_count(),
        0,
        "preparation and no approval send nothing"
    );
    assert_eq!(left.placeheld_turns, 1);
    assert_eq!(right.placeheld_turns, 1);
    for (side, plan) in [(0, left), (1, right)] {
        let draft = pair
            .generate(&mut data, side, &plan.approval(), 14)
            .unwrap();
        assert_eq!(draft.text, REPLY);
    }
    assert_eq!(endpoint.request_count(), 2);
    let requests = endpoint.requests();
    for request in &requests {
        assert!(!request.body.contains(PASTE));
        assert!(material(&request.body).contains(THIRD_PARTY_PLACEHOLDER));
    }
    assert!(!material(&requests[0].body).contains("倾向先核对安排再继续沟通"));
    assert!(material(&requests[1].body).contains("倾向先核对安排再继续沟通"));
    assert_eq!(
        data.candidates[1].actual_brief_state,
        Measured::Value(ActualBrief::FromView)
    );
    assert!(data
        .candidates
        .iter()
        .all(|c| c.draft_source == DraftSource::UserEndpoint
            && c.degraded_reason == DegradedReason::None));
    assert_unrated(&data);
}

#[test]
fn endpoint_pair_uses_one_owned_snapshot_even_if_the_external_profile_changes_between_candidates() {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let (mut data, mut slot) = evaluation(TaskType::T2);
    // Reversed BC also exercises the manual voice-only endpoint candidate.
    slot.contrast = Contrast::BC;
    slot.left_variant = Variant::C;
    slot.right_variant = Variant::B;
    data.registered_slots[0] = slot.clone();
    let mut view = snapshot();
    let expected_b = super::briefs::build_brief(Variant::B, &view, manual())
        .unwrap()
        .render()
        .unwrap();
    let expected_c = super::briefs::build_brief(Variant::C, &view, manual())
        .unwrap()
        .render()
        .unwrap();
    let mut pair = EndpointPair::start(
        &mut data,
        &slot,
        11,
        EndpointInput {
            snapshot: &view,
            manual: manual(),
            identifiers: KnownIdentifiers::new(),
            pasted: PASTE,
            endpoint: Some(&endpoint.base_url()),
        },
    )
    .unwrap();
    let left = pair.prepare(&mut data, 0, 12).unwrap();
    pair.generate(&mut data, 0, &left.approval(), 13).unwrap();
    view.voice.warmth = VoiceWarmth::Cool;
    view.axes[0].reading = "此后发生的合成档案变化不属于本次固定快照".into();
    let right = pair.prepare(&mut data, 1, 14).unwrap();
    pair.generate(&mut data, 1, &right.approval(), 15).unwrap();
    let requests = endpoint.requests();
    assert_eq!(requests.len(), 2);
    assert!(material(&requests[0].body).contains(&expected_c));
    assert!(!material(&requests[0].body).contains("倾向先核对安排再继续沟通"));
    assert!(material(&requests[1].body).contains(&expected_b));
    assert!(!material(&requests[1].body).contains(&view.axes[0].reading));
    assert_eq!(
        data.candidates[0].actual_brief_state,
        Measured::Value(ActualBrief::ManualVoice)
    );
    assert_eq!(
        data.candidates[1].actual_brief_state,
        Measured::Value(ActualBrief::FromView)
    );
    assert_unrated(&data);
}

#[test]
fn endpoint_adapter_rejects_nonliteral_or_external_origins_before_allocation() {
    for url in [
        "http://localhost:1234",
        "http://synthetic.example.invalid:1234",
    ] {
        let (mut data, slot) = evaluation(TaskType::T2);
        assert!(matches!(
            EndpointPair::start(
                &mut data,
                &slot,
                11,
                EndpointInput {
                    snapshot: &snapshot(),
                    manual: manual(),
                    identifiers: KnownIdentifiers::new(),
                    pasted: PASTE,
                    endpoint: Some(url),
                }
            ),
            Err(AdmissionError::Registration)
        ));
        assert!(data.pairs.is_empty());
        assert!(data.candidates.is_empty());
        assert_eq!(summarize(&validate(&data).unwrap()).funnel.started, 0);
    }
}

#[test]
fn endpoint_wrong_cross_candidate_approval_and_replay_reach_no_extra_request() {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let left = pair.prepare(&mut data, 0, 12).unwrap();
    let right = pair.prepare(&mut data, 1, 12).unwrap();
    assert!(matches!(
        pair.generate(&mut data, 1, &left.approval(), 13),
        Err(EndpointError::Refused(_))
    ));
    assert_eq!(endpoint.request_count(), 0);
    assert_eq!(
        data.candidates[1].failure_code,
        FailureCode::ApprovalRejected
    );
    let mut bad = left.approval();
    bad.plan_hash.push('x');
    assert!(pair.generate(&mut data, 0, &bad, 13).is_err());
    assert_eq!(endpoint.request_count(), 0);
    pair.generate(&mut data, 0, &left.approval(), 14).unwrap();
    assert!(matches!(
        pair.generate(&mut data, 0, &left.approval(), 15),
        Err(EndpointError::Refused(_))
    ));
    assert_eq!(endpoint.request_count(), 1);
    pair.generate(&mut data, 1, &right.approval(), 16).unwrap();
    assert_eq!(endpoint.request_count(), 2);
    assert_unrated(&data);
}

#[test]
fn endpoint_t3_scrubs_known_identifiers_and_a_later_t2_restores_placeholders() {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let (mut data, slot) = evaluation(TaskType::T3);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    for side in [0, 1] {
        let plan = pair.prepare(&mut data, side, 12).unwrap();
        assert!(plan.carries_exempted_original);
        pair.generate(&mut data, side, &plan.approval(), 13)
            .unwrap();
    }
    for request in endpoint.requests() {
        let text = material(&request.body);
        assert!(text.contains("请周五确认青色会场"));
        assert!(text.contains(NAME_PLACEHOLDER));
        assert!(text.contains(ACCOUNT_PLACEHOLDER));
        for identifier in [
            "青 岚",
            "青岚",
            "memberalpha",
            "@synth_handle",
            "synthetic@example.invalid",
            "138 0013 8000",
        ] {
            assert!(!text.contains(identifier));
        }
    }
    assert_unrated(&data);
    let (_, mut next) = evaluation(TaskType::T2);
    next.task_slot = 2;
    next.pair_id = "endpoint-next-t2".into();
    data.registered_slots.push(next.clone());
    let mut pair = pair.next_slot(&mut data, &next, 19).unwrap();
    let plan = pair.prepare(&mut data, 0, 20).unwrap();
    assert!(!plan.carries_exempted_original);
    pair.generate(&mut data, 0, &plan.approval(), 21).unwrap();
    let text = material(&endpoint.requests()[2].body);
    assert!(text.contains(THIRD_PARTY_PLACEHOLDER));
    assert!(!text.contains("青色会场"));
    assert_unrated(&data);
}

#[test]
fn endpoint_unconfigured_and_changed_origin_are_distinct_refusals() {
    let first = MockLlm::start().unwrap();
    let second = MockLlm::start().unwrap();
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, None);
    let plan = pair.prepare(&mut data, 0, 12).unwrap();
    assert!(pair.generate(&mut data, 0, &plan.approval(), 13).is_err());
    assert_eq!(
        data.candidates[0].failure_code,
        FailureCode::EndpointNotConfigured
    );
    assert_unrated(&data);
    // The origin-binding negative control starts a separate registered pair
    // whose two sides share the configured origin before the stale approval.
    let (mut configured, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut configured, &slot, Some(&first.base_url()));
    let plan = pair.prepare(&mut configured, 1, 12).unwrap();
    pair.change_endpoint(1, &second.base_url());
    assert!(pair
        .generate(&mut configured, 1, &plan.approval(), 13)
        .is_err());
    assert_eq!(
        configured.candidates[1].failure_code,
        FailureCode::OriginRejected
    );
    assert_eq!(first.request_count(), 0);
    assert_eq!(second.request_count(), 0);
    assert_unrated(&configured);
}

#[test]
fn endpoint_repreparing_the_other_side_at_another_origin_cannot_remain_comparable() {
    let first = MockLlm::start().unwrap();
    let second = MockLlm::start().unwrap();
    first.set_reply(REPLY);
    second.set_reply(REPLY);
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&first.base_url()));
    let left = pair.prepare(&mut data, 0, 12).unwrap();
    pair.generate(&mut data, 0, &left.approval(), 13).unwrap();
    pair.change_endpoint(1, &second.base_url());
    if let Ok(right) = pair.prepare(&mut data, 1, 14) {
        pair.generate(&mut data, 1, &right.approval(), 15).unwrap();
    }
    assert_eq!(first.request_count(), 1);
    assert!(
        second.request_count() == 0
            || data.pairs[0].comparability_status == Comparability::NotComparable,
        "two distinct configured origins cannot yield a comparable pair"
    );
    assert_unrated(&data);
}

#[test]
fn endpoint_next_slot_cannot_carry_different_origins_as_a_comparable_pair() {
    let first = MockLlm::start().unwrap();
    let second = MockLlm::start().unwrap();
    first.set_reply(REPLY);
    let (mut data, slot) = evaluation(TaskType::T3);
    let mut pair = start(&mut data, &slot, Some(&first.base_url()));
    let left = pair.prepare(&mut data, 0, 12).unwrap();
    pair.generate(&mut data, 0, &left.approval(), 13).unwrap();
    pair.change_endpoint(1, &second.base_url());
    let (_, mut next) = evaluation(TaskType::T2);
    next.task_slot = 2;
    next.pair_id = "different-origin-continuation".into();
    data.registered_slots.push(next.clone());
    let result = pair.next_slot(&mut data, &next, 14);
    assert!(
        result.is_err()
            || data.pairs.last().unwrap().comparability_status == Comparability::NotComparable,
        "a registered continuation cannot hide two different origins"
    );
    assert_eq!(first.request_count(), 1);
    assert_eq!(second.request_count(), 0);
    assert_unrated(&data);
}

#[test]
fn endpoint_cross_origin_redirect_records_origin_refusal_without_contacting_target() {
    let endpoint = MockLlm::start().unwrap();
    let forbidden = MockLlm::start().unwrap();
    endpoint.set_redirect(forbidden.chat_completions_url());
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let plan = pair.prepare(&mut data, 0, 12).unwrap();
    assert!(pair.generate(&mut data, 0, &plan.approval(), 13).is_err());
    assert_eq!(endpoint.request_count(), 1);
    assert_eq!(forbidden.request_count(), 0);
    assert_eq!(data.candidates[0].failure_code, FailureCode::OriginRejected);
    assert_eq!(
        data.candidates[0].refusal_code,
        Measured::Value(RefusalCode::Origin)
    );
    assert_unrated(&data);
}

#[test]
fn endpoint_reply_defects_keep_actual_degradation_and_transport_failure() {
    for (reply, expected) in [
        ("", DegradedReason::ReplyEmpty),
        (denied_reading(), DegradedReason::ReplyClinical),
    ] {
        let endpoint = MockLlm::start().unwrap();
        endpoint.set_reply(reply);
        let (mut data, slot) = evaluation(TaskType::T2);
        let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
        let plan = pair.prepare(&mut data, 0, 12).unwrap();
        pair.generate(&mut data, 0, &plan.approval(), 13).unwrap();
        assert_eq!(endpoint.request_count(), 1);
        assert_eq!(data.candidates[0].degraded_reason, expected);
        assert_eq!(data.candidates[0].draft_source, DraftSource::ToneTemplate);
        assert_eq!(
            data.candidates[0].failure_code,
            FailureCode::ResponseRejected
        );
        assert_unrated(&data);
    }
    for body in [Some("synthetic non-json response"), None] {
        let endpoint = RawEndpoint::start(body);
        let (mut data, slot) = evaluation(TaskType::T2);
        let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
        let plan = pair.prepare(&mut data, 0, 12).unwrap();
        let result = pair.generate(&mut data, 0, &plan.approval(), 13);
        assert_eq!(endpoint.request_count(), 1);
        if body.is_some() {
            result.unwrap();
            assert_eq!(
                data.candidates[0].degraded_reason,
                DegradedReason::ReplyUnreadable
            );
            assert_eq!(data.candidates[0].draft_source, DraftSource::ToneTemplate);
        } else {
            assert!(result.is_err());
            assert_eq!(
                data.candidates[0].failure_code,
                FailureCode::TransportFailure
            );
            assert!(!data.candidates[0].generated);
        }
        assert_unrated(&data);
    }
}

#[test]
fn endpoint_actual_transport_failure_is_terminal_and_cannot_be_overwritten_by_reprepare() {
    let endpoint = RawEndpoint::start(None);
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let plan = pair.prepare(&mut data, 0, 12).unwrap();
    assert!(pair.generate(&mut data, 0, &plan.approval(), 13).is_err());
    assert_eq!(endpoint.request_count(), 1);
    assert_eq!(
        data.candidates[0].failure_code,
        FailureCode::TransportFailure
    );
    assert!(pair.generate(&mut data, 0, &plan.approval(), 14).is_err());
    assert_eq!(
        data.candidates[0].failure_code,
        FailureCode::TransportFailure
    );
    assert!(
        pair.prepare(&mut data, 0, 14).is_err(),
        "a dispatched failure keeps its candidate; retry needs another registered slot"
    );
    assert_eq!(endpoint.request_count(), 1);
    assert_eq!(
        data.candidates[0].failure_code,
        FailureCode::TransportFailure
    );
    assert_unrated(&data);
}

#[test]
fn endpoint_terminal_failure_survives_deadline_on_reprepare() {
    assert_terminal_failure_survives_run_close(false);
}

#[test]
fn endpoint_terminal_failure_survives_withdrawal_on_generate() {
    assert_terminal_failure_survives_run_close(true);
}

fn assert_terminal_failure_survives_run_close(withdraw: bool) {
    let endpoint = RawEndpoint::start(None);
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let plan = pair.prepare(&mut data, 0, 12).unwrap();
    assert!(pair.generate(&mut data, 0, &plan.approval(), 13).is_err());
    assert_eq!(
        data.candidates[0].failure_code,
        FailureCode::TransportFailure
    );
    if withdraw {
        data.runs[0].consent_state = ConsentState::Withdrawn;
        assert!(pair.generate(&mut data, 0, &plan.approval(), 14).is_err());
    } else {
        assert!(pair.prepare(&mut data, 0, 100).is_err());
    }
    assert_eq!(
        data.candidates[0].failure_code,
        FailureCode::TransportFailure
    );
    assert_eq!(
        data.candidates[0].candidate_completion_status,
        CandidateCompletion::SystemFailure
    );
    assert_eq!(endpoint.request_count(), 1);
    assert_unrated(&data);
}

fn denied_reading() -> &'static str {
    include_str!("../../../../fixtures/denylist/diagnostic_terms.txt")
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap()
}

#[test]
fn endpoint_second_candidate_at_deadline_never_builds_or_sends() {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let plan = pair.prepare(&mut data, 0, 12).unwrap();
    pair.generate(&mut data, 0, &plan.approval(), 13).unwrap();
    assert!(matches!(
        pair.prepare(&mut data, 1, 100),
        Err(EndpointError::Admission(AdmissionError::RunClosed))
    ));
    assert_eq!(endpoint.request_count(), 1);
    assert_eq!(
        data.candidates[1].brief_build_status,
        Measured::Missing(MissingReason::DeadlineReached)
    );
    assert_eq!(
        data.candidates[1].failure_code,
        FailureCode::DeadlineReached
    );
    assert_unrated(&data);
}

#[test]
fn endpoint_withdrawal_after_prepare_prevents_the_approved_request() {
    let endpoint = MockLlm::start().unwrap();
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let plan = pair.prepare(&mut data, 0, 12).unwrap();
    data.runs[0].consent_state = ConsentState::Withdrawn;
    assert!(matches!(
        pair.generate(&mut data, 0, &plan.approval(), 13),
        Err(EndpointError::Admission(AdmissionError::RunClosed))
    ));
    assert_eq!(endpoint.request_count(), 0);
    assert_eq!(
        data.candidates[0].failure_code,
        FailureCode::ConsentWithdrawnBeforeGeneration
    );
    assert_unrated(&data);
}

#[test]
fn endpoint_deadline_after_prepare_stops_dispatch_and_missing_approval_is_a_real_refusal() {
    let endpoint = MockLlm::start().unwrap();
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let absent = soulcore::commands::draft::Approval {
        preparation_id: "nothing-prepared".into(),
        plan_hash: "no-approved-plan".into(),
    };
    let refusal = pair.generate(&mut data, 0, &absent, 12).unwrap_err();
    match refusal {
        EndpointError::Refused(refusal) => assert!(matches!(
            refusal,
            soulcore::commands::draft::DraftRefusal::NothingPrepared
        )),
        other => panic!("wrong refusal: {other:?}"),
    }
    assert_eq!(endpoint.request_count(), 0);
    let plan = pair.prepare(&mut data, 1, 12).unwrap();
    assert!(pair.generate(&mut data, 1, &plan.approval(), 100).is_err());
    assert_eq!(
        data.candidates[1].failure_code,
        FailureCode::DeadlineReached
    );
    assert_eq!(endpoint.request_count(), 0);
    assert_unrated(&data);
}

#[test]
fn endpoint_prior_safety_history_stops_new_and_already_allocated_candidates_on_its_path() {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut pair = start(&mut data, &slot, Some(&endpoint.base_url()));
    let plan = pair.prepare(&mut data, 0, 12).unwrap();
    pair.generate(&mut data, 0, &plan.approval(), 13).unwrap();
    // Predeclared synthetic history stands for a separately observed safety
    // termination. It is explicit fixture metadata, never inferred from success.
    let mut stopped = slot.clone();
    stopped.task_slot = 2;
    stopped.pair_id = "stopped-path".into();
    data.registered_slots.push(stopped.clone());
    let mut history = data.pairs[0].clone();
    history.task_slot = stopped.task_slot;
    history.pair_id = stopped.pair_id.clone();
    history.pair_completion_status = PairCompletion::SafetyStop;
    data.pairs.push(history);
    for variant in [Variant::A, Variant::B] {
        let mut c = Candidate::planned(
            &stopped.pair_id,
            &format!("{}-{variant:?}", stopped.pair_id),
            variant,
            TaskType::T2,
        );
        c.candidate_completion_status = CandidateCompletion::SafetyStop;
        c.privacy_or_safety_stop = Measured::Value(true);
        c.failure_code = FailureCode::SafetyStop;
        c.refusal_code = Measured::Value(RefusalCode::Safety);
        data.candidates.push(c);
    }
    validate(&data).unwrap();
    assert!(matches!(
        pair.prepare(&mut data, 1, 14),
        Err(EndpointError::Admission(AdmissionError::PathPaused))
    ));
    assert_eq!(endpoint.request_count(), 1);
    assert_eq!(
        data.candidates[1].brief_build_status,
        Measured::Missing(MissingReason::NotGenerated)
    );
    assert_eq!(data.candidates[1].failure_code, FailureCode::SafetyStop);
    let mut next = slot.clone();
    next.task_slot = 3;
    next.pair_id = "not-started-after-stop".into();
    data.registered_slots.push(next.clone());
    assert!(EndpointPair::start(
        &mut data,
        &next,
        15,
        EndpointInput {
            snapshot: &snapshot(),
            manual: manual(),
            identifiers: KnownIdentifiers::new(),
            pasted: PASTE,
            endpoint: Some(&endpoint.base_url())
        }
    )
    .is_err());
    assert_eq!(data.pairs.len(), 2);
    let (_, mut other_path) = evaluation(TaskType::T3);
    other_path.task_slot = 4;
    other_path.pair_id = "other-path-control".into();
    data.registered_slots.push(other_path.clone());
    let mut control = start(&mut data, &other_path, Some(&endpoint.base_url()));
    let plan = control.prepare(&mut data, 0, 16).unwrap();
    control
        .generate(&mut data, 0, &plan.approval(), 17)
        .unwrap();
    assert_eq!(endpoint.request_count(), 2);
    assert_unrated(&data);
}

#[test]
fn endpoint_failed_b_has_no_fallback_or_request_and_registered_identity_is_checked() {
    let endpoint = MockLlm::start().unwrap();
    let (mut data, slot) = evaluation(TaskType::T2);
    let mut view = snapshot();
    view.axes[0].reading = denied_reading().into();
    let mut pair = EndpointPair::start(
        &mut data,
        &slot,
        11,
        EndpointInput {
            snapshot: &view,
            manual: manual(),
            identifiers: KnownIdentifiers::new(),
            pasted: PASTE,
            endpoint: Some(&endpoint.base_url()),
        },
    )
    .unwrap();
    assert!(pair.prepare(&mut data, 1, 12).is_err());
    assert_eq!(
        data.candidates[1].brief_build_status,
        Measured::Value(BriefBuildStatus::Failed)
    );
    assert_eq!(
        data.candidates[1].actual_brief_state,
        Measured::Value(ActualBrief::Unknown)
    );
    assert_eq!(endpoint.request_count(), 0);
    assert_unrated(&data);
    data.candidates[0].requested_variant = Variant::B;
    assert!(matches!(
        pair.prepare(&mut data, 0, 13),
        Err(EndpointError::Admission(AdmissionError::InvalidRecords))
    ));
    assert_eq!(endpoint.request_count(), 0);
    let (mut fresh, slot) = evaluation(TaskType::T2);
    let mut forged = slot.clone();
    forged.presentation_seed += 1;
    assert!(EndpointPair::start(
        &mut fresh,
        &forged,
        11,
        EndpointInput {
            snapshot: &snapshot(),
            manual: manual(),
            identifiers: KnownIdentifiers::new(),
            pasted: PASTE,
            endpoint: Some(&endpoint.base_url())
        }
    )
    .is_err());
    assert!(fresh.pairs.is_empty());
}
