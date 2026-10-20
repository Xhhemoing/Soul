//! Test-only endpoint adapter; every generated outcome belongs to registered records.
use super::{
    briefs::{build_brief, ManualVoice},
    local::{admit_slot, AdmissionError},
    pairs::validate,
    records::*,
};
use soul_draft::{
    draft::{Degradation, Draft, DraftSource as ActualSource, NeverSent},
    ProfileBrief,
};
use soul_policy::redactor::KnownIdentifiers;
use soul_profile::view::ProfileView;
use soulcore::commands::{
    draft::{prepare_pasted, Approval, DraftRefusal, DraftSession, E1DraftPlan},
    policy::{E1Refusal, PolicySession},
};

pub const MODEL: &str = "synthetic-val-model";

pub struct EndpointInput<'a> {
    pub snapshot: &'a ProfileView,
    pub manual: ManualVoice,
    pub identifiers: KnownIdentifiers,
    pub pasted: &'a str,
    pub endpoint: Option<&'a str>,
}

#[derive(Debug)]
pub enum EndpointError {
    Admission(AdmissionError),
    Refused(DraftRefusal),
}

struct Side {
    drafting: DraftSession,
    policy: PolicySession,
    brief: Option<ProfileBrief>,
    dispatched_failure: bool,
}

pub struct EndpointPair {
    slot: RegisteredSlot,
    snapshot: ProfileView,
    manual: ManualVoice,
    pasted: String,
    sides: [Side; 2],
    origin: Option<soul_policy::net_guard::Origin>,
}

impl EndpointPair {
    /// Keep the same two redactors and token ledgers across a registered task
    /// change; this makes the T3 -> T2 one-shot check observe the same sessions.
    pub fn next_slot(
        self,
        data: &mut Evaluation,
        slot: &RegisteredSlot,
        now: u64,
    ) -> Result<Self, AdmissionError> {
        let origin = self.sides[0].policy.guard().config().e1_endpoint().cloned();
        if origin.as_ref() != self.sides[1].policy.guard().config().e1_endpoint() {
            return Err(AdmissionError::Registration);
        }
        let mut next = Self::start(
            data,
            slot,
            now,
            EndpointInput {
                snapshot: &self.snapshot,
                manual: self.manual,
                identifiers: KnownIdentifiers::new(),
                pasted: &self.pasted,
                endpoint: None,
            },
        )?;
        next.sides = self.sides;
        next.origin = origin;
        for side in &mut next.sides {
            side.drafting.discard();
            side.brief = None;
            side.dispatched_failure = false;
        }
        Ok(next)
    }

    pub fn start(
        data: &mut Evaluation,
        slot: &RegisteredSlot,
        now: u64,
        input: EndpointInput<'_>,
    ) -> Result<Self, AdmissionError> {
        if !matches!(slot.task_type, TaskType::T2 | TaskType::T3) {
            return Err(AdmissionError::WrongTask);
        }
        admit_slot(data, slot, now)?;
        // Report only the model actually passed to DraftSession. The mock exposes
        // neither a model revision nor the endpoint's hidden service parameters.
        if slot.conditions.model_id != Reported::Known(MODEL.into())
            || slot.conditions.model_version != Reported::NotExposed
            || slot.conditions.service_parameters != Reported::NotExposed
            || slot.conditions.context_limit != Reported::NotExposed
        {
            return Err(AdmissionError::Registration);
        }
        if input
            .endpoint
            .is_some_and(|url| !is_controlled_loopback(url))
        {
            return Err(AdmissionError::Registration);
        }
        let make_side = || -> Result<Side, AdmissionError> {
            let policy = match input.endpoint {
                Some(url) => PolicySession::with_user_endpoint(url, input.identifiers.clone())
                    .map_err(|_| AdmissionError::Registration)?,
                None => PolicySession::new(
                    soul_policy::net_guard::EgressConfig::closed(),
                    input.identifiers.clone(),
                ),
            };
            Ok(Side {
                drafting: DraftSession::new(MODEL, input.identifiers.clone()),
                policy,
                brief: None,
                dispatched_failure: false,
            })
        };
        let sides = [make_side()?, make_side()?];
        let origin = sides[0].policy.guard().config().e1_endpoint().cloned();
        data.pairs.push(Pair {
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
            pair_started_at: Some(now),
            pair_completed_at: None,
            pair_completion_status: PairCompletion::Incomplete,
            preference: Preference::Unrated,
            blind_status: BlindStatus::Intact,
            conditions: slot.conditions.clone(),
        });
        for variant in [slot.left_variant, slot.right_variant] {
            data.candidates.push(Candidate::planned(
                &slot.pair_id,
                &candidate_id(slot, variant),
                variant,
                slot.task_type,
            ));
        }
        validate(data).map_err(|_| AdmissionError::InvalidRecords)?;
        Ok(Self {
            slot: slot.clone(),
            snapshot: input.snapshot.clone(),
            manual: input.manual,
            pasted: input.pasted.into(),
            sides,
            origin,
        })
    }

    pub fn prepare(
        &mut self,
        data: &mut Evaluation,
        side: usize,
        now: u64,
    ) -> Result<E1DraftPlan, EndpointError> {
        let index = self.admit_candidate(data, side, now)?;
        if data.candidates[index].generated {
            return Err(EndpointError::Admission(AdmissionError::AlreadyStarted));
        }
        // Each pair freezes one endpoint before either candidate is prepared.
        // A changed origin may still exercise rejection of an old approval in
        // generate, but it cannot mint a fresh plan under this registered pair.
        if self
            .sides
            .iter()
            .any(|state| state.policy.guard().config().e1_endpoint() != self.origin.as_ref())
        {
            return Err(EndpointError::Admission(AdmissionError::Registration));
        }
        let variant = data.candidates[index].requested_variant;
        let brief = match build_brief(variant, &self.snapshot, self.manual) {
            Ok(brief) => brief,
            Err(error) => {
                let c = &mut data.candidates[index];
                c.brief_build_status = Measured::Value(BriefBuildStatus::Failed);
                c.actual_brief_state = Measured::Value(ActualBrief::Unknown);
                c.failure_code = FailureCode::BriefBuildFailed;
                c.candidate_completion_status = CandidateCompletion::SystemFailure;
                c.missing_reason = Some(MissingReason::BriefBuildFailed);
                update_pair(data, &self.slot.pair_id);
                validate(data)
                    .map_err(|_| EndpointError::Admission(AdmissionError::InvalidRecords))?;
                return Err(EndpointError::Refused(DraftRefusal::Draft(error)));
            }
        };
        let c = &mut data.candidates[index];
        c.brief_build_status = Measured::Value(BriefBuildStatus::Built);
        c.actual_brief_state = Measured::Value(match variant {
            Variant::A => ActualBrief::Neutral,
            Variant::B => ActualBrief::FromView,
            Variant::C => ActualBrief::ManualVoice,
        });
        self.sides[side].brief = Some(brief.clone());
        let state = &mut self.sides[side];
        prepare_pasted(
            &mut state.drafting,
            &mut state.policy,
            brief,
            &self.pasted,
            self.slot.task_type == TaskType::T3,
        )
        .map_err(|refusal| {
            record_refusal(&mut data.candidates[index], &refusal);
            update_pair(data, &self.slot.pair_id);
            EndpointError::Refused(refusal)
        })
    }

    pub fn generate(
        &mut self,
        data: &mut Evaluation,
        side: usize,
        approval: &Approval,
        now: u64,
    ) -> Result<Draft, EndpointError> {
        let index = self.admit_candidate(data, side, now)?;
        // Brief identity is internal: the caller supplies a variant and a fixed
        // snapshot, never a hand-built ProfileBrief that could impersonate B.
        if let Some(brief) = &self.sides[side].brief {
            if !build_brief(
                data.candidates[index].requested_variant,
                &self.snapshot,
                self.manual,
            )
            .is_ok_and(|expected| expected == *brief)
            {
                return Err(EndpointError::Admission(AdmissionError::InvalidRecords));
            }
        }
        let state = &mut self.sides[side];
        let had_preparation = state.drafting.pending_plan_hash().is_some();
        // Policy time uses its actual milliseconds; synthetic run seconds above
        // are a different clock and must not mint already-expired permits.
        match state.drafting.generate(
            &mut state.policy,
            approval,
            soul_policy::clock::now_unix_millis(),
        ) {
            Ok(drafted) => {
                record_draft(&mut data.candidates[index], &drafted.draft);
                update_pair(data, &self.slot.pair_id);
                validate(data)
                    .map_err(|_| EndpointError::Admission(AdmissionError::InvalidRecords))?;
                Ok(drafted.draft)
            }
            Err(refusal) => {
                // A consumed preparation ended this attempt. Approval mismatches
                // leave it pending, so a correct approval is still accepted.
                state.dispatched_failure =
                    had_preparation && state.drafting.pending_plan_hash().is_none();
                // A replay is an attempt against a consumed preparation, not a
                // replacement for the first candidate's recorded successful draft.
                if !data.candidates[index].generated {
                    record_refusal(&mut data.candidates[index], &refusal);
                }
                update_pair(data, &self.slot.pair_id);
                validate(data)
                    .map_err(|_| EndpointError::Admission(AdmissionError::InvalidRecords))?;
                Err(EndpointError::Refused(refusal))
            }
        }
    }

    pub fn change_endpoint(&mut self, side: usize, url: &str) {
        assert!(
            is_controlled_loopback(url),
            "synthetic endpoint must be literal loopback"
        );
        self.sides[side]
            .policy
            .set_user_endpoint(url)
            .expect("controlled loopback origin");
    }

    fn admit_candidate(
        &mut self,
        data: &mut Evaluation,
        side: usize,
        now: u64,
    ) -> Result<usize, EndpointError> {
        validate(data).map_err(|_| EndpointError::Admission(AdmissionError::InvalidRecords))?;
        let variant = *[self.slot.left_variant, self.slot.right_variant]
            .get(side)
            .ok_or(EndpointError::Admission(AdmissionError::InvalidRecords))?;
        if !data.registered_slots.iter().any(|slot| slot == &self.slot) {
            return Err(EndpointError::Admission(AdmissionError::Registration));
        }
        let pair = data
            .pairs
            .iter()
            .find(|pair| pair.pair_id == self.slot.pair_id)
            .ok_or(EndpointError::Admission(AdmissionError::InvalidRecords))?;
        // validate checks pair/registered-slot agreement, this check additionally
        // binds the live adapter to the exact identity it originally allocated.
        if pair.evaluation_run != self.slot.evaluation_run || pair.task_slot != self.slot.task_slot
        {
            return Err(EndpointError::Admission(AdmissionError::Registration));
        }
        let index = data
            .candidates
            .iter()
            .position(|c| {
                c.pair_id == self.slot.pair_id
                    && c.candidate_id == candidate_id(&self.slot, variant)
                    && c.requested_variant == variant
            })
            .ok_or(EndpointError::Admission(AdmissionError::InvalidRecords))?;
        // A terminal attempt retains its original observation even when the run
        // later expires or is withdrawn. Side and candidate identity are checked
        // above; reject before admission can write a new failure into that row.
        if self.sides[side].dispatched_failure {
            return Err(EndpointError::Admission(AdmissionError::AlreadyStarted));
        }
        let run = data
            .runs
            .iter()
            .find(|run| run.evaluation_run == self.slot.evaluation_run)
            .ok_or(EndpointError::Admission(AdmissionError::InvalidRecords))?;
        let stop = if !run.can_start(now) {
            Some((
                AdmissionError::RunClosed,
                if run.consent_state == ConsentState::Withdrawn {
                    FailureCode::ConsentWithdrawnBeforeGeneration
                } else {
                    FailureCode::DeadlineReached
                },
            ))
        } else if data
            .pairs
            .iter()
            .filter(|pair| {
                pair.evaluation_run == self.slot.evaluation_run
                    && pair.conditions.path == self.slot.conditions.path
            })
            .any(|pair| {
                pair.pair_completion_status == PairCompletion::SafetyStop
                    || data.candidates.iter().any(|c| {
                        c.pair_id == pair.pair_id
                            && (c.privacy_or_safety_stop == Measured::Value(true)
                                || c.candidate_completion_status == CandidateCompletion::SafetyStop)
                    })
            })
        {
            Some((AdmissionError::PathPaused, FailureCode::SafetyStop))
        } else {
            None
        };
        if let Some((error, failure)) = stop {
            self.sides[side].drafting.discard();
            if !data.candidates[index].generated {
                let c = &mut data.candidates[index];
                let missing = if failure == FailureCode::DeadlineReached {
                    MissingReason::DeadlineReached
                } else if failure == FailureCode::ConsentWithdrawnBeforeGeneration {
                    MissingReason::ConsentWithdrawnBeforeGeneration
                } else {
                    MissingReason::NotGenerated
                };
                c.failure_code = failure;
                c.missing_reason = Some(missing);
                c.edit_seconds = Measured::Missing(MissingReason::NotCompleted);
                if !matches!(c.brief_build_status, Measured::Value(_)) {
                    c.brief_build_status = Measured::Missing(missing);
                    c.actual_brief_state = Measured::Missing(missing);
                }
                if failure == FailureCode::SafetyStop {
                    c.candidate_completion_status = CandidateCompletion::SafetyStop;
                    c.privacy_or_safety_stop = Measured::Value(true);
                    c.refusal_code = Measured::Value(RefusalCode::Safety);
                }
                update_pair(data, &self.slot.pair_id);
            }
            validate(data).map_err(|_| EndpointError::Admission(AdmissionError::InvalidRecords))?;
            return Err(EndpointError::Admission(error));
        }
        Ok(index)
    }
}

fn is_controlled_loopback(url: &str) -> bool {
    soul_policy::net_guard::Origin::parse(url).is_ok_and(|origin| {
        origin.scheme() == "http"
            && origin
                .host()
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    })
}

fn candidate_id(slot: &RegisteredSlot, variant: Variant) -> String {
    format!("{}-{variant:?}", slot.pair_id)
}

fn record_refusal(c: &mut Candidate, refusal: &DraftRefusal) {
    let (failure, code) = match refusal {
        DraftRefusal::E1(E1Refusal::NotConfigured) => (FailureCode::EndpointNotConfigured, None),
        DraftRefusal::NotThePreparedEndpoint | DraftRefusal::E1(E1Refusal::Egress(_)) => {
            (FailureCode::OriginRejected, Some(RefusalCode::Origin))
        }
        DraftRefusal::E1(E1Refusal::Transport(soul_egress::EgressError::CrossOriginRedirect {
            ..
        })) => (FailureCode::OriginRejected, Some(RefusalCode::Origin)),
        DraftRefusal::E1(E1Refusal::Transport(_)) => (FailureCode::TransportFailure, None),
        _ => (FailureCode::ApprovalRejected, Some(RefusalCode::Approval)),
    };
    c.failure_code = failure;
    c.refusal_code = code.map_or(Measured::NotApplicable, Measured::Value);
    c.candidate_completion_status = CandidateCompletion::SystemFailure;
    c.missing_reason = Some(MissingReason::NotGenerated);
    c.privacy_or_safety_stop = Measured::Value(false);
}

fn record_draft(c: &mut Candidate, draft: &Draft) {
    c.generated = true;
    c.draft_source = match draft.source {
        ActualSource::ToneTemplate => DraftSource::ToneTemplate,
        ActualSource::UserEndpoint => DraftSource::UserEndpoint,
    };
    c.degraded_reason = match draft.degraded {
        None => DegradedReason::None,
        Some(Degradation::ReplyUnreadable) => DegradedReason::ReplyUnreadable,
        Some(Degradation::ReplyEmpty) => DegradedReason::ReplyEmpty,
        Some(Degradation::ReplyClinical) => DegradedReason::ReplyClinical,
    };
    c.failure_code = if draft.degraded.is_some() {
        FailureCode::ResponseRejected
    } else {
        FailureCode::None
    };
    c.candidate_completion_status = CandidateCompletion::Incomplete;
    c.refusal_code = Measured::NotApplicable;
    c.first_pass_accepted = Measured::Missing(MissingReason::NotRated);
    c.edit_seconds = Measured::Missing(MissingReason::NotCompleted);
    c.pause_seconds = Measured::Missing(MissingReason::NotCompleted);
    c.help_count = Measured::Missing(MissingReason::NotCompleted);
    c.unsupported_claim = Measured::Missing(MissingReason::NotRated);
    if matches!(c.required_fact_error_count, Measured::Missing(_)) {
        c.required_fact_error_count = Measured::Missing(MissingReason::NotRated);
    }
    c.placeheld_turns =
        Measured::Value(u32::try_from(draft.placeheld_turns).expect("synthetic turn count"));
    c.carries_exempted_original = Measured::Value(draft.carries_exempted_original);
    c.privacy_or_safety_stop = Measured::Value(false);
    c.sanitized_note = Measured::Missing(MissingReason::NotRated);
    c.missing_reason = Some(MissingReason::NotRated);
    c.delivery = draft.delivery != NeverSent;
}

fn update_pair(data: &mut Evaluation, id: &str) {
    let rows: Vec<_> = data.candidates.iter().filter(|c| c.pair_id == id).collect();
    let pair = data
        .pairs
        .iter_mut()
        .find(|pair| pair.pair_id == id)
        .expect("allocated pair");
    pair.pair_completion_status = if rows
        .iter()
        .any(|c| c.candidate_completion_status == CandidateCompletion::SafetyStop)
    {
        PairCompletion::SafetyStop
    } else if rows.iter().any(|c| {
        c.candidate_completion_status == CandidateCompletion::SystemFailure
            || c.degraded_reason != DegradedReason::None && c.generated
    }) {
        PairCompletion::SystemFailure
    } else if rows.iter().any(|c| {
        matches!(
            c.failure_code,
            FailureCode::DeadlineReached | FailureCode::ConsentWithdrawnBeforeGeneration
        )
    }) {
        PairCompletion::Aborted
    } else {
        PairCompletion::Incomplete
    };
    if rows
        .iter()
        .any(|c| c.brief_build_status == Measured::Value(BriefBuildStatus::Failed))
    {
        pair.comparability_status = Comparability::NotComparable;
    }
}
