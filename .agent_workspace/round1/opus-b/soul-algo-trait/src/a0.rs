//! **A0 — questionnaire + correction lock.** The Goal 1 baseline, restated as
//! a pure function.
//!
//! Goal 1 spreads this rule across `intake`, `correct_axis` and
//! `record_axis_inference` in `crates/soul-profile/src/service.rs`, where each
//! call mutates the stored profile in place. Replaying the evidence log through
//! [`a0_axis_state`] produces the same axis the store would be holding, which
//! is what makes the baseline testable without a store, and what makes a
//! forget recomputable: drop the row, replay, and the axis falls back on its
//! own.
//!
//! The rules, in the order they bite:
//!
//! - Forgotten rows are dropped before anything is aggregated.
//! - Each remaining row replaces the axis, exactly as `place_axis` does:
//!   `evidence_ids` says what supports the position the axis is in *now*, not
//!   everything ever seen. Superseded rows keep their place in the evidence
//!   table and the audit chain.
//! - Once the user corrects the axis it is locked. Later non-lock inferences
//!   are still returned, in [`AxisOutcome::shadow`] marked
//!   [`ApplyResult::RefusedLocked`], but they do not move the axis. A later
//!   correction still applies: the lock stops the machine, not the user.
//! - No evidence at all leaves the axis at `Unknown` / `Band::None`.
//! - A row that claims `Position::Unknown` claims no direction, so it carries
//!   `Band::None` however it was labelled. A user is allowed to correct an axis
//!   *to* `Unknown` — "stop guessing" is a legitimate correction, and it locks
//!   the axis like any other.
//!
//! Nothing here reads a clock and nothing here is numeric.

use crate::types::{ApplyResult, AxisId, AxisOutcome, AxisState, EvidenceRef, ShadowInference};

/// Identifier A0 stamps on the states it produces.
pub const A0_ALGORITHM_ID: &str = "a0.questionnaire_correction_lock.v1";

/// Replay one axis under A0.
///
/// Evidence for other axes is ignored, so a caller can hand over the whole log.
/// Input order is taken to be the order the rows were recorded.
pub fn a0_axis_state(axis: AxisId, evidence: &[EvidenceRef]) -> AxisOutcome {
    let mut state = AxisState::unknown(axis, A0_ALGORITHM_ID);
    let mut shadow = Vec::new();

    for row in evidence
        .iter()
        .filter(|row| row.axis == axis && !row.forgotten)
    {
        let band = row.effective_band();
        let result = if state.locked_by_user && !row.locked_by_user {
            ApplyResult::RefusedLocked
        } else {
            state.position = row.position;
            state.band = band;
            state.evidence_ids = vec![row.evidence_id];
            state.locked_by_user = state.locked_by_user || row.locked_by_user;
            ApplyResult::Applied
        };

        shadow.push(ShadowInference {
            evidence_id: row.evidence_id,
            axis,
            position: row.position,
            band,
            result,
            cited: false,
        });
    }

    for entry in &mut shadow {
        entry.cited = state.evidence_ids.contains(&entry.evidence_id);
    }

    AxisOutcome { state, shadow }
}

/// Replay all five axes under A0, in questionnaire order.
pub fn a0_all_axes(evidence: &[EvidenceRef]) -> Vec<AxisOutcome> {
    AxisId::ALL
        .iter()
        .map(|axis| a0_axis_state(*axis, evidence))
        .collect()
}
