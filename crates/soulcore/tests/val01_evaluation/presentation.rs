//! Synthetic callback presentation, not a human GUI or a security isolation boundary.
use super::{
    endpoint::{EndpointInput, EndpointPair},
    local::{run_local_pair, ExecutionCounts},
    manual, pairs,
    records::*,
    synthetic_cards::{self, CardId, CARDS},
};
use soul_policy::redactor::KnownIdentifiers;
use soul_profile::view::ProfileView;
use soul_testkit::mock_llm::MockLlm;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PresentationError {
    State,
    RunClosed,
    PathPaused,
    Generation,
    Feedback,
    Metadata,
}

#[derive(Clone, Copy)]
pub(super) enum SimulatedApproval {
    ApproveExact,
    Deny,
}

// Deliberately no Debug/Serialize, source, variant, candidate id or plan.
pub(super) struct BlindPairView<'a> {
    pub card: &'static str,
    pub instruction: &'static str,
    pub left_text: &'a str,
    pub right_text: &'a str,
}

pub(super) struct SimulatedSideFeedback {
    pub completed: bool,
    pub first_pass: Measured<bool>,
    pub edit_seconds: Measured<u64>,
    pub pause_seconds: Measured<u64>,
    pub help_count: Measured<u32>,
    pub unsupported_claim: Measured<bool>,
    pub fact_errors: Measured<u32>,
    pub safety_stop: bool,
    pub t1_slot_filled: bool,
    pub missing_reason: Option<MissingReason>,
}

pub(super) struct SimulatedFeedback {
    pub preference: Preference,
    pub blind_status: BlindStatus,
    pub sides: [SimulatedSideFeedback; 2],
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Registered,
    Ready,
    Presented,
    Recorded,
    Revealed,
    Invalidated,
}

#[derive(Default, serde::Serialize)]
struct Counts {
    http_requests: usize,
    local_draft_calls: usize,
    presentations: usize,
    feedback_records: usize,
    reveals: usize,
}

// Ownership is a test workflow boundary, not isolation from the operator.
// No Debug or Serialize implementation can accidentally print held prose.
pub(super) struct Simulation {
    data: Evaluation,
    snapshot: ProfileView,
    phase: [Phase; 5],
    text: [Option<[String; 2]>; 5],
    counts: Counts,
}

impl Simulation {
    pub fn new() -> Self {
        Self::with_window(10, 100)
    }

    pub fn with_window(start: u64, deadline: u64) -> Self {
        Self {
            data: Evaluation {
                runs: vec![Run {
                    evaluation_run: "synthetic-five-card-run".into(),
                    run_started_at: start,
                    run_deadline_at: deadline,
                    participant_scope: ParticipantScope::Author,
                    feedback_purpose: FeedbackPurpose::ProductEvaluation,
                    consent_state: ConsentState::Granted,
                }],
                registered_slots: CARDS.into_iter().map(CardId::slot).collect(),
                pairs: vec![],
                candidates: vec![],
            },
            snapshot: synthetic_cards::snapshot(),
            phase: [Phase::Registered; 5],
            text: std::array::from_fn(|_| None),
            counts: Counts::default(),
        }
    }

    pub fn generate_all(
        &mut self,
        now: u64,
        approval: SimulatedApproval,
    ) -> Result<(), PresentationError> {
        let mut failure = None;
        for card in CARDS {
            if let Err(error) = self.generate(card, now, approval) {
                failure.get_or_insert(error);
            }
        }
        failure.map_or(Ok(()), Err)
    }

    pub fn generate(
        &mut self,
        id: CardId,
        now: u64,
        approval: SimulatedApproval,
    ) -> Result<(), PresentationError> {
        self.live(id, now)?;
        if self.phase[id.index()] != Phase::Registered {
            return Err(PresentationError::State);
        }
        let card = id.card();
        let slot = id.slot();
        let mut texts = Vec::new();
        if card.task == TaskType::T1 {
            let mut counts = ExecutionCounts::default();
            let drafts = run_local_pair(
                &mut self.data,
                &slot,
                now,
                &self.snapshot,
                synthetic_cards::manual(),
                card.paste,
                &mut counts,
            )
            .map_err(|_| PresentationError::Generation)?;
            self.counts.local_draft_calls += counts.draft_calls;
            texts.extend(drafts.into_iter().map(|draft| draft.text));
        } else {
            let endpoint = MockLlm::start().map_err(|_| PresentationError::Generation)?;
            endpoint.set_reply(card.reply);
            let url = endpoint.base_url();
            let mut pair = EndpointPair::start(
                &mut self.data,
                &slot,
                now,
                EndpointInput {
                    snapshot: &self.snapshot,
                    manual: synthetic_cards::manual(),
                    identifiers: KnownIdentifiers::new(),
                    pasted: card.paste,
                    endpoint: Some(&url),
                },
            )
            .map_err(|_| PresentationError::Generation)?;
            for side in 0..2 {
                let Ok(plan) = pair.prepare(&mut self.data, side, now) else {
                    continue;
                };
                match approval {
                    SimulatedApproval::ApproveExact => {
                        if let Ok(draft) =
                            pair.generate(&mut self.data, side, &plan.approval(), now)
                        {
                            texts.push(draft.text);
                        }
                    }
                    SimulatedApproval::Deny => {
                        // Explicit simulated rejection: do not call generate or
                        // fabricate an Approval. Dropping pair discards plans.
                        let variant = [slot.left_variant, slot.right_variant][side];
                        let candidate = self
                            .data
                            .candidates
                            .iter_mut()
                            .find(|c| c.pair_id == slot.pair_id && c.requested_variant == variant)
                            .ok_or(PresentationError::Metadata)?;
                        candidate.candidate_completion_status = CandidateCompletion::SystemFailure;
                        candidate.failure_code = FailureCode::ApprovalRejected;
                        candidate.refusal_code = Measured::Value(RefusalCode::Approval);
                        candidate.missing_reason = Some(MissingReason::NotGenerated);
                        candidate.privacy_or_safety_stop = Measured::Value(false);
                        let row = self
                            .data
                            .pairs
                            .iter_mut()
                            .find(|p| p.pair_id == slot.pair_id)
                            .ok_or(PresentationError::Metadata)?;
                        row.pair_completion_status = PairCompletion::SystemFailure;
                    }
                }
            }
            self.counts.http_requests += endpoint.request_count();
        }
        pairs::validate(&self.data).map_err(|_| PresentationError::Metadata)?;
        let generated = self
            .data
            .candidates
            .iter()
            .filter(|c| c.pair_id == slot.pair_id)
            .filter(|c| c.generated)
            .count();
        if texts.len() != 2 || generated != 2 {
            self.phase[id.index()] = Phase::Invalidated;
            return Err(PresentationError::Generation);
        }
        // local returns a Vec that skips failed sides; only the verified
        // two-generated/two-draft case establishes positional correspondence.
        let [left, right]: [String; 2] = texts
            .try_into()
            .map_err(|_| PresentationError::Generation)?;
        self.text[id.index()] = Some([left, right]);
        self.phase[id.index()] = Phase::Ready;
        Ok(())
    }

    pub fn present(
        &mut self,
        id: CardId,
        now: u64,
        renderer: impl FnOnce(BlindPairView<'_>),
    ) -> Result<(), PresentationError> {
        self.live(id, now)?;
        if self.phase[id.index()] != Phase::Ready {
            return Err(PresentationError::State);
        }
        let [left, right] = self.text[id.index()]
            .as_ref()
            .ok_or(PresentationError::State)?;
        let card = id.card();
        renderer(BlindPairView {
            card: card.label,
            instruction: card.instruction,
            left_text: left,
            right_text: right,
        });
        self.phase[id.index()] = Phase::Presented;
        self.counts.presentations += 1;
        Ok(())
    }

    pub fn submit(
        &mut self,
        id: CardId,
        now: u64,
        feedback: SimulatedFeedback,
    ) -> Result<(), PresentationError> {
        self.record_feedback(id, now, feedback, SanitizedNote::SyntheticFeedback)
    }

    pub fn submit_operator(
        &mut self,
        id: CardId,
        now: u64,
        feedback: SimulatedFeedback,
    ) -> Result<(), PresentationError> {
        self.record_feedback(id, now, feedback, SanitizedNote::SyntheticOperatorFeedback)
    }

    fn record_feedback(
        &mut self,
        id: CardId,
        now: u64,
        feedback: SimulatedFeedback,
        note: SanitizedNote,
    ) -> Result<(), PresentationError> {
        self.live(id, now)?;
        if self.phase[id.index()] != Phase::Presented {
            return Err(PresentationError::State);
        }
        let slot = id.slot();
        let mut proposed = self.data.clone();
        let mut any_safety = false;
        let mut both_completed = true;
        for (variant, side) in [slot.left_variant, slot.right_variant]
            .into_iter()
            .zip(feedback.sides)
        {
            // These fixed local drafts always contain a body slot. Filling it
            // can complete the task, but cannot rewrite the initial draft's
            // first-pass observation as accepted.
            if slot.task_type == TaskType::T1
                && ((!side.t1_slot_filled && side.completed)
                    || side.first_pass == Measured::Value(true))
            {
                return Err(PresentationError::Feedback);
            }
            let candidate = proposed
                .candidates
                .iter_mut()
                .find(|c| c.pair_id == slot.pair_id && c.requested_variant == variant)
                .ok_or(PresentationError::Metadata)?;
            candidate.candidate_completion_status = if side.safety_stop {
                CandidateCompletion::SafetyStop
            } else if side.completed {
                CandidateCompletion::Completed
            } else {
                CandidateCompletion::Incomplete
            };
            any_safety |= side.safety_stop;
            both_completed &= side.completed && !side.safety_stop;
            candidate.first_pass_accepted = side.first_pass;
            candidate.edit_seconds = side.edit_seconds;
            candidate.pause_seconds = side.pause_seconds;
            candidate.help_count = side.help_count;
            candidate.unsupported_claim = side.unsupported_claim;
            candidate.required_fact_error_count = side.fact_errors;
            candidate.privacy_or_safety_stop = Measured::Value(side.safety_stop);
            if side.safety_stop {
                candidate.failure_code = FailureCode::SafetyStop;
            }
            candidate.missing_reason = side.missing_reason;
            candidate.sanitized_note = Measured::Value(note);
        }
        let pair = proposed
            .pairs
            .iter_mut()
            .find(|p| p.pair_id == slot.pair_id)
            .ok_or(PresentationError::Metadata)?;
        pair.preference = feedback.preference;
        pair.blind_status = feedback.blind_status;
        pair.pair_completion_status = if any_safety {
            PairCompletion::SafetyStop
        } else if both_completed {
            PairCompletion::Completed
        } else {
            PairCompletion::Incomplete
        };
        pair.pair_completed_at = both_completed.then_some(now);
        pairs::validate(&proposed).map_err(|_| PresentationError::Feedback)?;
        self.data = proposed;
        self.phase[id.index()] = Phase::Recorded;
        self.text[id.index()] = None;
        self.counts.feedback_records += 1;
        if any_safety {
            self.invalidate_path(slot.conditions.path);
            // Preserve D104 callback behavior; the operator may reveal its
            // just-fixed feedback without re-enabling this paused path.
            if note == SanitizedNote::SyntheticOperatorFeedback {
                self.phase[id.index()] = Phase::Recorded;
            }
        }
        Ok(())
    }

    pub fn reveal(&mut self, id: CardId) -> Result<[Variant; 2], PresentationError> {
        if self.phase[id.index()] != Phase::Recorded {
            return Err(PresentationError::State);
        }
        self.phase[id.index()] = Phase::Revealed;
        self.counts.reveals += 1;
        let slot = id.slot();
        Ok([slot.left_variant, slot.right_variant])
    }

    pub fn operator_summary(&self) -> Result<serde_json::Value, PresentationError> {
        let data = self.export_records();
        let json = serde_json::to_string(&data).map_err(|_| PresentationError::Metadata)?;
        let summary = manual::report_json(&json).map_err(|_| PresentationError::Metadata)?;
        Ok(serde_json::json!({
            "synthetic": true, "human_evaluation": false, "summary": summary, "counts": self.counts
        }))
    }

    pub fn revealed_metadata(&self) -> Result<serde_json::Value, PresentationError> {
        if self.phase.iter().any(|phase| *phase != Phase::Revealed) {
            return Err(PresentationError::State);
        }
        serde_json::to_value(self.export_records()).map_err(|_| PresentationError::Metadata)
    }

    pub fn withdraw(&mut self) {
        self.data.runs[0].consent_state = ConsentState::Withdrawn;
        for card in CARDS {
            self.invalidate_path(card.slot().conditions.path);
        }
    }

    pub fn stop(&mut self) {
        for card in CARDS {
            self.text[card.index()] = None;
            self.phase[card.index()] = Phase::Invalidated;
        }
    }

    // A fixed adversarial synthetic snapshot, not an arbitrary profile input.
    pub fn with_invalid_b() -> Self {
        let mut simulation = Self::new();
        let denied = include_str!("../../../../fixtures/denylist/diagnostic_terms.txt")
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .expect("fixed diagnostic fixture");
        simulation.snapshot.axes[0].reading = denied.into();
        simulation
    }

    fn invalidate_path(&mut self, path: Path) {
        for card in CARDS
            .into_iter()
            .filter(|id| id.slot().conditions.path == path)
        {
            self.text[card.index()] = None;
            self.phase[card.index()] = Phase::Invalidated;
        }
    }

    fn live(&mut self, id: CardId, now: u64) -> Result<(), PresentationError> {
        if !self.data.runs[0].can_start(now) {
            for card in CARDS {
                self.invalidate_path(card.slot().conditions.path);
            }
            return Err(PresentationError::RunClosed);
        }
        let path = id.slot().conditions.path;
        if self.data.pairs.iter().any(|pair| {
            pair.conditions.path == path
                && pair.pair_completion_status == PairCompletion::SafetyStop
        }) {
            self.invalidate_path(path);
            return Err(PresentationError::PathPaused);
        }
        Ok(())
    }

    pub fn export_records(&self) -> Evaluation {
        let mut data = self.data.clone();
        for slot in &self.data.registered_slots {
            if data.pairs.iter().any(|pair| pair.pair_id == slot.pair_id) {
                continue;
            }
            // Only this private session's known unstarted slots are projected.
            // Never repairs an imported file or replaces an existing outcome.
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
                pair_started_at: None,
                pair_completed_at: None,
                pair_completion_status: PairCompletion::Incomplete,
                preference: Preference::Unrated,
                blind_status: BlindStatus::Intact,
                conditions: slot.conditions.clone(),
            });
            for variant in [slot.left_variant, slot.right_variant] {
                data.candidates.push(Candidate::planned(
                    &slot.pair_id,
                    &format!("{}-{variant:?}", slot.pair_id),
                    variant,
                    slot.task_type,
                ));
            }
        }
        data
    }
}
