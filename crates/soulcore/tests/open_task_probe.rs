//! Test-only open synthetic tasks. Runtime inputs contain no options or gold.
#![allow(dead_code)]
#[path = "task_retrieval_probe/adapter.rs"]
mod adapter;
#[path = "task_requirements_probe/data.rs"]
mod data;
#[path = "task_requirements_probe/diagnosis.rs"]
mod diagnosis;
#[path = "task_requirements_probe/execution.rs"]
mod execution;
#[path = "open_task_probe/files.rs"]
mod files;
#[path = "evidence_stopping_probe/policy.rs"]
mod policy;
#[path = "retrieval_controls_probe/ranking.rs"]
mod ranking;
#[path = "open_task_probe/replay.rs"]
mod replay;
#[path = "open_task_probe/schema.rs"]
mod schema;
#[path = "context_selection_probe/selector.rs"]
mod selector;
#[path = "task_retrieval_probe/semantic.rs"]
mod semantic;
#[path = "open_task_probe/tests.rs"]
mod tests;
#[path = "task_retrieval_probe/types.rs"]
mod types;

#[test]
#[ignore = "explicit offline open-task replay using synthetic stores and no generation"]
fn replay_predictions() {
    replay::run();
}
