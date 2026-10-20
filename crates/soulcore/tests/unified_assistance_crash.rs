//! Real process death, not a dropped Session or owned-work lease.
//! The two child entrypoints are no-ops in an ordinary test run. The parent
//! tests re-execute them and accept only the deliberate no-Drop exit code.
#[path = "../../../apps/desktop/src-tauri/src/assistance_runtime/driver.rs"]
mod assistance_driver;

use assistance_driver::{cycle, CycleOutcome};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use soul_store_api::assistance::AssistanceWorkClaim;
use soul_testkit::{crash, mock_llm::MockLlm};
use soulcore::commands::collect::{CollectorConfig, FakeForegroundSource};
use soulcore::commands::draft::Approval;
use soulcore::commands::memory;
use soulcore::commands::session::*;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{atomic::AtomicBool, Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

const DATA_DIR_ENV: &str = "SOUL_ASSISTANCE_CRASH_DIR";
const ENDPOINT_ENV: &str = "SOUL_ASSISTANCE_CRASH_ENDPOINT";
const CLAIM_CHILD: &str = "child_exits_with_a_live_claim_and_a_newer_pending_revision";
const PERMISSION_CHILD: &str = "child_exits_with_unconsumed_approval_and_collection_consent";
const NO_DROP_EXIT: i32 = 86;
const TITLE: &str = "SYNTHETIC_CRASH_GOAL_TITLE";
const BODY: &str = "SYNTHETIC_CRASH_GOAL_BODY";
const UPDATED_BODY: &str = "SYNTHETIC_CRASH_LATEST_GOAL_BODY";
const PASTE: &str = "SYNTHETIC_CRASH_THIRD_PARTY_PASTE";

#[derive(Serialize, Deserialize)]
struct ClaimReady {
    item_id: Uuid,
    claimed_revision: Uuid,
    latest_revision: Uuid,
    claim_id: Uuid,
}

#[derive(Serialize, Deserialize)]
struct PermissionReady {
    item_id: Uuid,
    revision: Uuid,
    context_id: Uuid,
    approval: Approval,
}

fn child_directory() -> PathBuf {
    PathBuf::from(std::env::var_os(DATA_DIR_ENV).expect("parent supplies a temporary directory"))
}

fn goal(session: &mut Session) -> AssistanceItemRow {
    session
        .create_assistance_goal(CreateAssistanceGoal {
            title: TITLE.into(),
            summary: BODY.into(),
            deadline_at: None,
            source_observation: None,
        })
        .unwrap()
        .item
}

fn pointer(item: &AssistanceItemRow) -> AssistanceItemRequest {
    AssistanceItemRequest {
        item_id: item.item_id,
        expected_revision: item.revision,
    }
}

/// Only UUIDs and approval metadata cross the child boundary; never prose.
fn ready_then_exit(directory: &Path, ready: &impl Serialize) -> ! {
    let mut file = std::fs::File::create(directory.join("ready.json")).unwrap();
    file.write_all(&serde_json::to_vec(ready).unwrap()).unwrap();
    file.sync_all().unwrap();
    // exit bypasses Rust Drop for Session, SQLCipher, work and collector owners.
    std::process::exit(NO_DROP_EXIT);
}

/// A failed assertion or timeout must not leave an owner holding the OS lease.
struct ChildOwner(Option<Child>);

impl Drop for ChildOwner {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn run_child<T: DeserializeOwned>(directory: &Path, name: &str, endpoint: Option<&str>) -> T {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg(name)
        .args(["--exact", "--nocapture", "--test-threads=1"])
        .env(crash::CHILD_MARKER_ENV, "1")
        .env(DATA_DIR_ENV, directory)
        .env_remove("FAILPOINTS")
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(endpoint) = endpoint {
        command.env(ENDPOINT_ENV, endpoint);
    } else {
        command.env_remove(ENDPOINT_ENV);
    }
    let mut owner = ChildOwner(Some(command.spawn().unwrap()));
    let until = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(status) = owner.0.as_mut().unwrap().try_wait().unwrap() {
            // A panic, abort or missing test must fail, not masquerade as our crash.
            assert_eq!(status.code(), Some(NO_DROP_EXIT), "child {name}: {status}");
            owner.0 = None;
            break;
        }
        assert!(Instant::now() < until, "child {name} did not exit in time");
        std::thread::sleep(Duration::from_millis(10));
    }
    // try_wait reaped the actual process before Session can acquire a new lease.
    serde_json::from_slice(&std::fs::read(directory.join("ready.json")).unwrap()).unwrap()
}

fn opened(directory: &Path) -> Session {
    let session = Session::open(directory);
    assert!(
        session.status().store_opened,
        "{}",
        session.status().store_notice
    );
    session
}

fn assert_no_body_or_egress(session: &Session) {
    let chain = session.audit().unwrap();
    assert!(chain.verified, "{:?}", chain.verification_problem);
    assert!(chain.entries.iter().all(|entry| entry.follows_previous));
    assert!(!chain
        .entries
        .iter()
        .any(|entry| entry.action == "egress.request" || entry.action == "draft.create"));
    let serialized = serde_json::to_string(&chain).unwrap();
    for prose in [TITLE, BODY, UPDATED_BODY, PASTE] {
        assert!(
            !serialized.contains(prose),
            "audit contains synthetic prose"
        );
    }
}

#[test]
fn child_exits_with_a_live_claim_and_a_newer_pending_revision() {
    if !crash::is_child() {
        return;
    }
    crash::abort_if_child();
    let directory = child_directory();
    let mut session = opened(&directory);
    let item = goal(&mut session);
    let shared = session.store().unwrap();
    let work = session.begin_assistance_work().unwrap().unwrap();
    let AssistanceSource::ManualGoal { memory_id, .. } = item.source else {
        panic!("manual goal owns its commitment memory")
    };
    let ready = {
        let mut store = shared.lock().unwrap();
        let claimed = store.assistance_item_work(item.item_id).unwrap();
        assert_eq!(claimed.state, AssistanceWorkState::Running);
        assert_eq!(claimed.claimed_revision, Some(item.revision));
        assert!(claimed.pending_revision.is_none());
        // An existing domain API changes the same encrypted store while the
        // owned work remains alive, just as the in-flight work regression does.
        memory::update(
            &mut store,
            memory_id,
            &soul_memory::MemoryEdit::summary(UPDATED_BODY),
            1_800_000_000,
        )
        .unwrap();
        let latest = store.assistance_item(item.item_id).unwrap();
        let current = store.assistance_item_work(item.item_id).unwrap();
        assert_ne!(latest.revision, item.revision);
        assert_eq!(current.state, AssistanceWorkState::Running);
        assert_eq!(current.claim_id, claimed.claim_id);
        assert_eq!(current.claimed_revision, Some(item.revision));
        assert_eq!(current.pending_revision, Some(latest.revision));
        assert_ne!(current.pending_revision, current.claimed_revision);
        assert!(current.result_revision.is_none());
        ClaimReady {
            item_id: item.item_id,
            claimed_revision: item.revision,
            latest_revision: latest.revision,
            claim_id: current.claim_id.unwrap(),
        }
    };
    assert!(session.assistance_work_status().unwrap().active_worker);
    std::hint::black_box(&work);
    ready_then_exit(&directory, &ready);
}

#[test]
fn process_death_recovers_the_latest_revision_once_through_the_production_driver() {
    let directory = tempfile::tempdir().unwrap();
    let ready: ClaimReady = run_child(directory.path(), CLAIM_CHILD, None);
    assert_ne!(ready.claimed_revision, ready.latest_revision);
    let session = Arc::new(Mutex::new(opened(directory.path())));
    let old_claim = AssistanceWorkClaim {
        item_id: ready.item_id,
        revision: ready.claimed_revision,
        claim_id: ready.claim_id,
        preparation_revision: None,
    };
    {
        let held = session.lock().unwrap();
        let metadata = held
            .assistance_item_work(&ready.item_id.to_string())
            .unwrap();
        assert_eq!(metadata.state, AssistanceWorkState::Pending);
        assert_eq!(metadata.item_revision, ready.latest_revision);
        assert_eq!(metadata.pending_revision, Some(ready.latest_revision));
        assert!(metadata.claim_id.is_none());
        assert!(metadata.claimed_revision.is_none());
        assert!(metadata.result_revision.is_none());
        let status = held.assistance_work_status().unwrap();
        assert_eq!(status.pending_items, Some(1));
        assert_eq!(status.claimed_items, Some(0));
        assert!(!status.active_worker);
        assert_no_body_or_egress(&held);
    }
    let stopping = AtomicBool::new(false);
    assert_eq!(cycle(&session, &stopping).unwrap(), CycleOutcome::Completed);
    let completed = {
        let held = session.lock().unwrap();
        let metadata = held
            .assistance_item_work(&ready.item_id.to_string())
            .unwrap();
        assert_eq!(metadata.state, AssistanceWorkState::Ready);
        assert_eq!(metadata.result_revision, Some(ready.latest_revision));
        assert!(metadata.claim_id.is_none());
        assert!(metadata.pending_revision.is_none());
        assert!(!held
            .store()
            .unwrap()
            .lock()
            .unwrap()
            .assistance_claim_completed(&old_claim)
            .unwrap());
        assert_no_body_or_egress(&held);
        metadata
    };
    for _ in 0..2 {
        assert_eq!(cycle(&session, &stopping).unwrap(), CycleOutcome::Idle);
        assert_eq!(
            session
                .lock()
                .unwrap()
                .assistance_item_work(&ready.item_id.to_string())
                .unwrap(),
            completed
        );
    }
    drop(session);
    let reopened = Arc::new(Mutex::new(opened(directory.path())));
    assert_eq!(cycle(&reopened, &stopping).unwrap(), CycleOutcome::Idle);
    assert_eq!(
        reopened
            .lock()
            .unwrap()
            .assistance_item_work(&ready.item_id.to_string())
            .unwrap(),
        completed
    );
    assert_no_body_or_egress(&reopened.lock().unwrap());
}

#[test]
fn child_exits_with_unconsumed_approval_and_collection_consent() {
    if !crash::is_child() {
        return;
    }
    crash::abort_if_child();
    let directory = child_directory();
    let mut session = opened(&directory);
    session
        .set_user_endpoint(&std::env::var(ENDPOINT_ENV).unwrap())
        .unwrap();
    let item = goal(&mut session);
    let source = FakeForegroundSource::new();
    let granted = session
        .grant_collect_consent_with_source(
            source,
            CollectorConfig::every(Duration::from_millis(10)),
        )
        .unwrap();
    assert!(granted.consent_granted);
    assert!(granted.collector_running);
    let AssistanceContextView::Memory { memory, .. } =
        session.prepare_assistance_context(pointer(&item)).unwrap()
    else {
        panic!("manual goal prepares a memory context")
    };
    let context_id = memory.context_id.unwrap();
    let approval = session
        .prepare_draft_with_memory(PASTE, &context_id.to_string(), &[], false)
        .unwrap()
        .prepared
        .plan
        .approval();
    assert!(!approval.preparation_id.is_empty());
    assert!(!approval.plan_hash.is_empty());
    // No begin_draft/discard/mutation follows preparation. This is a live,
    // never-consumed approval, whose foreground priority blocks local claims.
    assert!(
        session
            .assistance_work_status()
            .unwrap()
            .foreground_preparation_held
    );
    assert!(session.begin_assistance_work().unwrap().is_none());
    assert_no_body_or_egress(&session);
    assert!(session.collect_status().consent_granted);
    ready_then_exit(
        &directory,
        &PermissionReady {
            item_id: item.item_id,
            revision: item.revision,
            context_id,
            approval,
        },
    );
}

#[test]
fn process_death_does_not_restore_unconsumed_approval_context_or_collection_consent() {
    let directory = tempfile::tempdir().unwrap();
    let endpoint = MockLlm::start().unwrap();
    let ready: PermissionReady = run_child(
        directory.path(),
        PERMISSION_CHILD,
        Some(&endpoint.base_url()),
    );
    assert_eq!(endpoint.request_count(), 0);
    let mut session = opened(directory.path());
    assert!(!session.collect_status().consent_granted);
    assert!(!session.collect_status().collector_running);
    assert!(
        !session
            .assistance_work_status()
            .unwrap()
            .foreground_preparation_held
    );
    let item = session
        .assistance_item(&ready.item_id.to_string())
        .unwrap()
        .item;
    assert_eq!(
        item.revision, ready.revision,
        "the approval's item was never edited or forgotten"
    );
    // Endpoints are session-only. Explicitly configure the SAME live loopback
    // server so refusal cannot be attributed merely to an unavailable endpoint.
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    assert_eq!(
        session
            .begin_draft(&ready.approval)
            .unwrap_err()
            .reason_code,
        "PLAN_HASH_MISMATCH"
    );
    assert!(session
        .draft_pasted_with_memory(PASTE, &ready.context_id.to_string())
        .is_err());
    assert_eq!(endpoint.request_count(), 0);
    assert_no_body_or_egress(&session);
    let session = Arc::new(Mutex::new(session));
    let stopping = AtomicBool::new(false);
    assert_eq!(cycle(&session, &stopping).unwrap(), CycleOutcome::Completed);
    assert_eq!(cycle(&session, &stopping).unwrap(), CycleOutcome::Idle);
    assert_no_body_or_egress(&session.lock().unwrap());
    assert_eq!(endpoint.request_count(), 0);
}
