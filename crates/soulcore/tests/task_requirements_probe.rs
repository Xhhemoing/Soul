//! Offline synthetic research: required information, diagnostic gaps, and whole-group context.
#![allow(dead_code)]
#[path = "task_retrieval_probe/adapter.rs"]
mod adapter;
#[path = "task_requirements_probe/data.rs"]
mod data;
#[path = "task_requirements_probe/diagnosis.rs"]
mod diagnosis;
#[path = "task_requirements_probe/execution.rs"]
mod execution;
#[path = "task_requirements_probe/files.rs"]
mod files;
#[path = "evidence_stopping_probe/policy.rs"]
mod policy;
#[path = "retrieval_controls_probe/ranking.rs"]
mod ranking;
#[path = "task_requirements_probe/replay.rs"]
mod replay;
#[path = "task_requirements_probe/scoring.rs"]
mod scoring;
#[path = "context_selection_probe/selector.rs"]
mod selector;
#[path = "task_retrieval_probe/semantic.rs"]
mod semantic;
#[path = "task_requirements_probe/tests.rs"]
mod tests;
#[path = "task_requirements_probe/timing.rs"]
mod timing;
#[path = "task_retrieval_probe/types.rs"]
mod types;

#[test]
#[ignore = "explicit input-only synthetic export"]
fn export_inputs() {
    files::export_inputs();
}

#[test]
#[ignore = "explicit offline prediction replay with synthetic stores"]
fn replay_predictions() {
    replay::run();
}

#[test]
#[ignore = "explicit synthetic local-stage timing, no model or network calls"]
fn measure_local_stages() {
    timing::run();
}
