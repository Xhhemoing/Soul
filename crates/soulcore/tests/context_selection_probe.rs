//! Synthetic, offline comparison of support-group selection and ranked prefixes.
#![allow(dead_code)]
#[path = "task_retrieval_probe/adapter.rs"]
mod adapter;
#[path = "evidence_stopping_probe/artifacts.rs"]
mod artifacts;
#[path = "retrieval_controls_probe/fixtures.rs"]
mod fixtures;
#[path = "evidence_stopping_probe/policy.rs"]
mod policy;
#[path = "retrieval_controls_probe/ranking.rs"]
mod ranking;
#[path = "context_selection_probe/replay.rs"]
mod replay;
#[path = "context_selection_probe/selector.rs"]
mod selector;
#[path = "task_retrieval_probe/semantic.rs"]
mod semantic;
#[path = "context_selection_probe/tests.rs"]
mod tests;
#[path = "task_retrieval_probe/types.rs"]
mod types;

#[test]
#[ignore = "explicit offline synthetic context selection replay"]
fn replay_predictions() {
    replay::run();
}
