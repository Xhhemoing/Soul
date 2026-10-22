//! Test-only evidence stopping with synthetic sources and unused loopback.
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
#[path = "evidence_stopping_probe/replay.rs"]
mod replay;
#[path = "task_retrieval_probe/semantic.rs"]
mod semantic;
#[path = "evidence_stopping_probe/tests.rs"]
mod tests;
#[path = "task_retrieval_probe/types.rs"]
mod types;

#[test]
fn requested_two_record_prefix_limits_real_preparation() {
    let case = fixtures::cases().remove(1);
    let mut harness = adapter::Harness::new_with_aliases(&case.input, &case.aliases);
    let snapshot = harness.snapshot().expect("all original source snapshots");
    let full_rank = ranking::rank(&snapshot.input, true, true);
    let requested_k = 2;
    let observation = harness
        .observe_prefix(&snapshot, &full_rank, requested_k, true)
        .expect("real prefix preparation");
    assert_eq!(observation.metrics["network_requests"], 0);
    assert_eq!(observation.selected.len(), requested_k, "requested_prefix");
    assert_eq!(observation.retained.len(), requested_k);
}

#[test]
#[ignore = "explicit synthetic task-only export"]
fn export_tasks() {
    artifacts::export_tasks();
}

#[test]
#[ignore = "explicit synthetic support-mapping export"]
fn export_mapping_inputs() {
    artifacts::export_mapping_inputs();
}

#[test]
#[ignore = "explicit synthetic stopping replay"]
fn replay_predictions() {
    replay::run();
}
