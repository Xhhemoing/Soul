//! Exercise the shipped scheduler/thread with synthetic real SQLCipher Sessions.
//! These are not a replacement for native Tauri or Windows notification tests.
#[path = "../../../apps/desktop/src-tauri/src/assistance_runtime/mod.rs"]
mod assistance_runtime;

use assistance_runtime::driver::{cycle, CycleOutcome};
use assistance_runtime::AssistanceRuntime;
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::session::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn setup() -> (tempfile::TempDir, Session, AssistanceItemRow) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    let goal = session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "Synthetic goal".into(),
            summary: "Synthetic local preparation only".into(),
            deadline_at: None,
            source_observation: None,
        })
        .unwrap();
    (directory, session, goal.item)
}

#[test]
fn production_turn_publishes_latest_coalesced_revision_without_egress() {
    let (_directory, mut session, mut item) = setup();
    let endpoint = MockLlm::start().unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    for title in ["Synthetic second version", "Synthetic latest version"] {
        item = session
            .edit_assistance_goal(EditAssistanceGoal {
                item_id: item.item_id,
                expected_revision: item.revision,
                title: title.into(),
                summary: "Synthetic local preparation only".into(),
                deadline_at: None,
            })
            .unwrap()
            .item;
    }
    assert_eq!(
        session.assistance_work_status().unwrap().pending_items,
        Some(1)
    );
    let session = Arc::new(Mutex::new(session));
    let stopping = AtomicBool::new(false);
    assert_eq!(cycle(&session, &stopping).unwrap(), CycleOutcome::Completed);
    assert_eq!(cycle(&session, &stopping).unwrap(), CycleOutcome::Idle);
    let session = session.lock().unwrap();
    let metadata = session
        .assistance_item_work(&item.item_id.to_string())
        .unwrap();
    assert_eq!(metadata.state, AssistanceWorkState::Ready);
    assert_eq!(metadata.result_revision, Some(item.revision));
    assert!(metadata.claim_id.is_none());
    assert!(metadata.pending_revision.is_none());
    assert!(!session.assistance_work_status().unwrap().active_worker);
    assert_eq!(endpoint.request_count(), 0);
}

#[test]
fn production_turn_keeps_the_explicit_foreground_context_usable() {
    let (_directory, mut session, item) = setup();
    let context = session
        .prepare_assistance_context(AssistanceItemRequest {
            item_id: item.item_id,
            expected_revision: item.revision,
        })
        .unwrap();
    let AssistanceContextView::Memory { memory, .. } = context else {
        panic!("manual goal has a memory context");
    };
    let context_id = memory.context_id.unwrap();
    let session = Arc::new(Mutex::new(session));
    assert_eq!(
        cycle(&session, &AtomicBool::new(false)).unwrap(),
        CycleOutcome::Idle
    );
    let mut session = session.lock().unwrap();
    let metadata = session
        .assistance_item_work(&item.item_id.to_string())
        .unwrap();
    assert_eq!(metadata.state, AssistanceWorkState::Pending);
    assert!(metadata.claim_id.is_none());
    session
        .draft_pasted_with_memory("Synthetic explicit paste", &context_id.to_string())
        .expect("background scheduler must not consume or discard the held context");
}

#[test]
fn shutdown_while_waiting_for_session_does_not_claim_work() {
    let (_directory, session, item) = setup();
    let session = Arc::new(Mutex::new(session));
    let stopping = Arc::new(AtomicBool::new(false));
    let held = session.lock().unwrap();
    let worker_session = Arc::clone(&session);
    let worker_stop = Arc::clone(&stopping);
    let (entered, waiting) = mpsc::channel();
    let worker = thread::spawn(move || {
        entered.send(()).unwrap();
        cycle(&worker_session, &worker_stop)
    });
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    stopping.store(true, Ordering::Release);
    drop(held);
    assert_eq!(worker.join().unwrap().unwrap(), CycleOutcome::Stopped);
    let session = session.lock().unwrap();
    let work = session
        .assistance_item_work(&item.item_id.to_string())
        .unwrap();
    assert_eq!(work.state, AssistanceWorkState::Pending);
    assert!(work.claim_id.is_none());
}

#[test]
fn shipped_thread_completes_and_shutdown_joins_its_actual_owner() {
    let (_directory, session, item) = setup();
    let session = Arc::new(Mutex::new(session));
    let runtime = AssistanceRuntime::start(Arc::clone(&session));
    let until = Instant::now() + Duration::from_secs(8);
    loop {
        let held = session.lock().unwrap();
        let status = held.assistance_work_status().unwrap();
        if !status.active_worker && status.pending_items == Some(0) {
            assert_eq!(
                held.assistance_item_work(&item.item_id.to_string())
                    .unwrap()
                    .state,
                AssistanceWorkState::Ready
            );
            break;
        }
        drop(held);
        assert!(Instant::now() < until, "production worker did not complete");
        thread::sleep(Duration::from_millis(2));
    }
    runtime.shutdown();
    runtime.shutdown();
    let held = session.lock().unwrap();
    let status = held.assistance_work_status().unwrap();
    assert_eq!(status.runtime_state, AssistanceRuntimeState::Stopped);
    assert!(!status.active_worker);
    assert_eq!(status.claimed_items, Some(0));
    assert!(status.last_runtime_error.is_none());
}

#[test]
fn startup_recovers_an_abandoned_claim_once_and_publishes_through_production_turn() {
    let (directory, mut session, item) = setup();
    let abandoned = session.begin_assistance_work().unwrap().unwrap();
    drop(abandoned);
    // End the actual holder, including its store/work Arcs, before recovery.
    drop(session);
    let session = Arc::new(Mutex::new(Session::open(directory.path())));
    assert_eq!(
        cycle(&session, &AtomicBool::new(false)).unwrap(),
        CycleOutcome::Completed
    );
    assert_eq!(
        cycle(&session, &AtomicBool::new(false)).unwrap(),
        CycleOutcome::Idle
    );
    let held = session.lock().unwrap();
    assert_eq!(
        held.assistance_item_work(&item.item_id.to_string())
            .unwrap()
            .result_revision,
        Some(item.revision)
    );
    assert_eq!(
        held.assistance_work_status().unwrap().claimed_items,
        Some(0)
    );
}
