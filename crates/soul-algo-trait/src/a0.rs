//! **A0 — questionnaire + correction lock.** The Goal 1 baseline, restated as
//! a pure function, and patched where Round 1 found it leaking.
//!
//! Goal 1 spreads this rule across `intake`, `correct_axis` and
//! `record_axis_inference` in `crates/soul-profile/src/service.rs`, where each
//! call mutates the stored profile in place. Replaying the evidence log through
//! [`a0_axis_state`] produces the axis the store should be holding, which is
//! what makes the baseline testable without a store, and what makes a forget
//! recomputable: drop the row, replay, and the axis falls back on its own.
//!
//! # The lock defect Round 1 found, and what fixes it
//!
//! `record_axis_inference` checks `axis_is_locked` before it calls
//! `place_axis`. **`intake` does not.** It walks the answers and calls
//! `place_axis` for each one with `locked_by_user: None`, which leaves the lock
//! flag set while overwriting the position, the band and the citation list
//! underneath it. Re-running the questionnaire therefore moves an axis the user
//! has already corrected, and leaves it looking locked afterwards — the worst
//! of the two possible bugs, because the UI still shows the padlock.
//!
//! The replay in this module never had that defect: a questionnaire row is not
//! a correction, so the same branch that refuses a machine inference refuses
//! it. [`apply_intake`] is the imperative path restated so that it agrees with
//! the replay: it consults the standing lock per axis, skips the answers that
//! would move a locked axis, and reports them in
//! [`IntakeReport::ignored`] instead of silently dropping them.
//! `intake_matches_replay` in `tests/a0_lock.rs` pins the agreement.
//!
//! **The answer row is still written.** The user answered the question; that is
//! a fact about the user whatever the axis does with it. What the lock stops is
//! the axis moving, not the evidence table growing — the same shape
//! `record_axis_inference` already has, where a refused inference is stored and
//! then not applied.
//!
//! # Round 3: frozen as keeper #2
//!
//! `R2-SYNTHESIS.md` retains two algorithms for v0.1: one tie-strength
//! algorithm, and **A0**. Round 3 freezes A0 in the shape below and changes no
//! behaviour: the intake patch from Round 2 stands, [`A0_ALGORITHM_ID`] does
//! not move, and the default write mode stays [`WriteMode::LastWriteWins`].
//! `tests/frozen_defaults.rs` pins all three, so a later edit to any of them
//! has to be a deliberate unfreeze rather than a drift.
//!
//! What "frozen" rules out, specifically: A0 does not gain a band-upgrade rule.
//! [`crate::a1`] implements one and is **not** wired in — `a0_axis_state` never
//! calls it, and the only way to reach `Strong` on this path remains a user
//! correction. That is the intended v0.1 semantics: `Strong` means the user
//! looked at the claim and said so.
//!
//! # Two write modes, and why the frozen default is the clobbering one
//!
//! [`WriteMode::LastWriteWins`] is A0 exactly as Goal 1 behaves, and it is the
//! **regression contract**: `place_axis` replaces `evidence_ids` rather than
//! accumulating, so the newest row standing is the whole of the support. One
//! `Weak` inference lands on top of a `Moderate` questionnaire answer and the
//! questionnaire stops being cited.  `last_write_wins_is_the_a0_contract`
//! documents that this is intended baseline behaviour and not an accident.
//!
//! Round 3 keeps it as the default because **on the v0.1 data plane the
//! downgrade it allows is unreachable.** The clobber needs a non-correction row
//! that disagrees with what is standing, and the only two producers a v0.1
//! install has against an axis are the questionnaire and a correction (see
//! [`crate::types::EvidenceKind::reachable_for_axis_in_v01`]). A correction
//! locks; a second questionnaire answer replacing the first is the questionnaire
//! working as intended, not a downgrade. Direction, band and lock therefore
//! come out the same under both modes for every v0.1-reachable log, which
//! `the_frozen_write_mode_decides_nothing_a_v01_user_is_told` runs rather than
//! asserts. Switching the default would change the baseline the Goal 1 store is
//! compared against and change nothing the profile displays — and a freeze is
//! for holding the baseline still.
//!
//! **One thing does differ, and it is not nothing.** The two modes disagree
//! about [`AxisState::evidence_ids`]: five re-fills of one questionnaire leave
//! last-write-wins citing the fifth row and `NoDowngrade` citing all five.
//! 「用户能复核计数」 is a criterion in its own right, so
//! `the_two_write_modes_do_differ_in_what_they_cite` states the divergence
//! plainly instead of leaving it inside a passing test. It is a Round X
//! question, not a reason to unfreeze.
//!
//! [`WriteMode::NoDowngrade`] therefore stays as a named, tested opt-in, ready
//! for the version that starts writing behavioural rows against an axis. That
//! is the version that needs it — and see
//! `a0_trusts_the_band_on_a_row_it_is_handed` for the other thing that version
//! must decide: A0 reads [`EvidenceRef::band`] and never caps it, so
//! "only a correction reaches `Strong`" is a property of the v0.1 data plane
//! rather than of the code in this module.
//!
//! [`WriteMode::NoDowngrade`] is the smallest change that stops the clobber
//! without turning A0 into a tally:
//!
//! - A row that **agrees** with the standing position joins the citation list,
//!   and the band becomes the strongest single supporting row.
//! - A row that **disagrees** replaces the axis only if it is worth at least as
//!   much as what is standing; otherwise it is refused as
//!   [`ApplyResult::RefusedWeaker`] and kept in the shadow.
//! - The lock still outranks both. Only the user moves a locked axis.
//!
//! Neither mode reads a clock, and neither is numeric.

use crate::types::{
    ApplyResult, AxisId, AxisOutcome, AxisState, Band, EvidenceRef, Position, ShadowInference,
};

/// Identifier A0 stamps on the states it produces.
///
/// Unchanged from Round 2, and unchanged on purpose: Round 3 froze A0 without
/// altering a single decision it makes, so a state stamped by the Round 2
/// package and a state stamped by this one mean the same thing.
pub const A0_ALGORITHM_ID: &str = "a0.questionnaire_correction_lock.v2";

/// The write mode A0 uses unless a caller names another one.
///
/// [`WriteMode::LastWriteWins`], frozen. See the module docs for why the
/// clobbering mode is the safe default on the v0.1 data plane.
pub const A0_DEFAULT_WRITE_MODE: WriteMode = WriteMode::LastWriteWins;

/// How a new row that is not a correction interacts with what is already
/// standing on the axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WriteMode {
    /// Goal 1's `place_axis`: the newest row replaces position, band and
    /// citations. The A0 regression contract.
    #[default]
    LastWriteWins,
    /// A weaker row may not clobber a better supported disagreeing position; a
    /// row that agrees joins the support instead of replacing it.
    NoDowngrade,
}

/// Replay one axis under A0 with the baseline write mode.
///
/// Evidence for other axes is ignored, so a caller can hand over the whole log.
/// Input order is taken to be the order the rows were recorded.
pub fn a0_axis_state(axis: AxisId, evidence: &[EvidenceRef]) -> AxisOutcome {
    a0_axis_state_with(axis, evidence, A0_DEFAULT_WRITE_MODE)
}

/// Replay all five axes under A0 with the baseline write mode.
pub fn a0_all_axes(evidence: &[EvidenceRef]) -> Vec<AxisOutcome> {
    a0_all_axes_with(evidence, A0_DEFAULT_WRITE_MODE)
}

/// Replay one axis under A0, choosing the write mode.
pub fn a0_axis_state_with(axis: AxisId, evidence: &[EvidenceRef], mode: WriteMode) -> AxisOutcome {
    let mut state = AxisState::unknown(axis, A0_ALGORITHM_ID);
    let mut shadow = Vec::new();

    for row in evidence
        .iter()
        .filter(|row| row.axis == axis && !row.forgotten)
    {
        let result = place(&mut state, row, mode);
        shadow.push(ShadowInference {
            evidence_id: row.evidence_id,
            axis,
            position: row.position,
            band: row.effective_band(),
            result,
            cited: false,
        });
    }

    for entry in &mut shadow {
        entry.cited = state.evidence_ids.contains(&entry.evidence_id);
    }

    AxisOutcome { state, shadow }
}

/// Replay all five axes under A0, in questionnaire order, choosing the mode.
pub fn a0_all_axes_with(evidence: &[EvidenceRef], mode: WriteMode) -> Vec<AxisOutcome> {
    AxisId::ALL
        .iter()
        .map(|axis| a0_axis_state_with(*axis, evidence, mode))
        .collect()
}

// ---------------------------------------------------------------- intake ---

/// One questionnaire answer, before it has been written anywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntakeAnswer {
    /// Id the answer's evidence row will carry.
    pub evidence_id: u64,
    /// Which axis the question is about.
    pub axis: AxisId,
    /// What the user answered.
    pub position: Position,
    /// When the questionnaire was answered. Recorded on the row; A0 does not
    /// read it, A1 does.
    pub recorded_at_unix: i64,
}

impl IntakeAnswer {
    /// One answer.
    pub fn new(evidence_id: u64, axis: AxisId, position: Position, recorded_at_unix: i64) -> Self {
        IntakeAnswer {
            evidence_id,
            axis,
            position,
            recorded_at_unix,
        }
    }

    /// The evidence row this answer becomes. Written whether or not the axis
    /// moves.
    pub fn to_evidence(self) -> EvidenceRef {
        EvidenceRef::questionnaire(
            self.evidence_id,
            self.axis,
            self.position,
            self.recorded_at_unix,
        )
    }
}

/// Why an answer did not move its axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntakeSkip {
    /// The user has corrected this axis. Only the user moves it now.
    AxisLockedByUser,
}

impl IntakeSkip {
    /// A stable string for the audit row, holding no answer content.
    pub fn as_str(self) -> &'static str {
        match self {
            IntakeSkip::AxisLockedByUser => "axis_locked_by_user",
        }
    }
}

/// One answer the intake read and did not apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IgnoredAnswer {
    /// Which row it was. It exists in the evidence table.
    pub evidence_id: u64,
    /// Which axis it was about.
    pub axis: AxisId,
    /// What it claimed.
    pub position: Position,
    /// Why it was not applied.
    pub reason: IntakeSkip,
}

/// What one run of the questionnaire did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IntakeReport {
    /// The five axes afterwards, in questionnaire order.
    pub axes: Vec<AxisState>,
    /// Ids of the answers that moved their axis, in input order.
    pub applied: Vec<u64>,
    /// The answers that did not move their axis, and why. Never merged into
    /// `applied`: a re-fill that hit a locked axis has to be visible, both to
    /// the user ("we kept your correction") and to whoever reads the audit.
    pub ignored: Vec<IgnoredAnswer>,
    /// The evidence log afterwards: everything that was there, plus one row per
    /// answer including the ignored ones.
    pub log_after: Vec<EvidenceRef>,
}

impl IntakeReport {
    /// Ids of the answers that were read and not applied.
    pub fn ignored_ids(&self) -> Vec<u64> {
        self.ignored
            .iter()
            .map(|answer| answer.evidence_id)
            .collect()
    }

    /// Whether any answer was refused by a lock.
    pub fn ignored_any(&self) -> bool {
        !self.ignored.is_empty()
    }

    /// The state of one axis afterwards.
    pub fn axis(&self, axis: AxisId) -> &AxisState {
        self.axes
            .iter()
            .find(|state| state.axis == axis)
            .expect("apply_intake always returns all five axes")
    }
}

/// Run the questionnaire against a standing evidence log.
///
/// This is the patched `intake`. The rule it adds to Goal 1 is one line long
/// and is the whole point of the function: **an answer never moves an axis the
/// user has corrected.** Every answer still becomes a row in
/// [`IntakeReport::log_after`]; the ones that could not move their axis are
/// listed in [`IntakeReport::ignored`].
///
/// Answers are applied in input order, so two answers about the same axis in
/// one run behave exactly as two rows in the log would.
pub fn apply_intake(
    prior_log: &[EvidenceRef],
    answers: &[IntakeAnswer],
    mode: WriteMode,
) -> IntakeReport {
    let mut axes: Vec<AxisState> = a0_all_axes_with(prior_log, mode)
        .into_iter()
        .map(|outcome| outcome.state)
        .collect();

    let mut applied = Vec::new();
    let mut ignored = Vec::new();
    let mut log_after = prior_log.to_vec();

    for answer in answers {
        let row = answer.to_evidence();
        log_after.push(row.clone());

        let Some(state) = axes.iter_mut().find(|state| state.axis == answer.axis) else {
            continue;
        };

        match place(state, &row, mode) {
            ApplyResult::Applied => applied.push(answer.evidence_id),
            ApplyResult::RefusedLocked => ignored.push(IgnoredAnswer {
                evidence_id: answer.evidence_id,
                axis: answer.axis,
                position: answer.position,
                reason: IntakeSkip::AxisLockedByUser,
            }),
            // A questionnaire answer is never a correction, so `place` can only
            // refuse it via the lock. `NoDowngrade` cannot reject it either: a
            // questionnaire row is Moderate, and the only thing that outranks
            // Moderate is a correction, which has already locked the axis.
            ApplyResult::RefusedWeaker => ignored.push(IgnoredAnswer {
                evidence_id: answer.evidence_id,
                axis: answer.axis,
                position: answer.position,
                reason: IntakeSkip::AxisLockedByUser,
            }),
        }
    }

    IntakeReport {
        axes,
        applied,
        ignored,
        log_after,
    }
}

// ------------------------------------------------------------- internals ---

/// Apply one row to one axis. The single place the lock and the write mode are
/// decided, so the replay and the intake path cannot drift apart.
fn place(state: &mut AxisState, row: &EvidenceRef, mode: WriteMode) -> ApplyResult {
    let band = row.effective_band();

    if state.locked_by_user && !row.locked_by_user {
        return ApplyResult::RefusedLocked;
    }

    if row.locked_by_user {
        replace(state, row, band);
        state.locked_by_user = true;
        return ApplyResult::Applied;
    }

    match mode {
        WriteMode::LastWriteWins => {
            replace(state, row, band);
            ApplyResult::Applied
        }
        WriteMode::NoDowngrade => {
            if state.position == Position::Unknown && state.evidence_ids.is_empty() {
                replace(state, row, band);
                ApplyResult::Applied
            } else if row.position == state.position {
                if !state.evidence_ids.contains(&row.evidence_id) {
                    state.evidence_ids.push(row.evidence_id);
                }
                state.band = Band::max_of(state.band, band);
                ApplyResult::Applied
            } else if band.at_least(state.band) {
                replace(state, row, band);
                ApplyResult::Applied
            } else {
                ApplyResult::RefusedWeaker
            }
        }
    }
}

fn replace(state: &mut AxisState, row: &EvidenceRef, band: Band) {
    state.position = row.position;
    state.band = band;
    state.evidence_ids = vec![row.evidence_id];
}
