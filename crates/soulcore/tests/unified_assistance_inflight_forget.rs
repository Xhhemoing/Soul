//! Real Session/SQLCipher and held loopback HTTP; synthetic material only.
use soul_draft::DraftSource;
use soul_store_api::forget::ForgetOutcomeOps;
use soul_store_api::import::{ImportIndexStore, ImportSourceKind, LegacyImportChoice};
use soul_store_api::{BlobStore, EventStore, ForgetUnit, StoreError};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::draft::{Approval, DraftRefusal};
use soulcore::commands::session::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;

const WAIT: Duration = Duration::from_secs(15);
const GOAL: &str = "SYNTHETIC_INFLIGHT_SOURCED_GOAL_PRIVATE";
const INDEPENDENT: &str = "SYNTHETIC_INFLIGHT_INDEPENDENT_GOAL_PRIVATE";
const PASTE: &str = "SYNTHETIC_INFLIGHT_PASTE_PRIVATE_20261019";
const REPLY: &str = "SYNTHETIC_INFLIGHT_REPLY_PRIVATE_20261019";

struct HeldEndpoint {
    url: String,
    received: mpsc::Receiver<()>,
    release: mpsc::SyncSender<()>,
    responded: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl HeldEndpoint {
    fn start() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let (seen, received) = mpsc::sync_channel(1);
        let (release, gate) = mpsc::sync_channel(1);
        let responded = Arc::new(AtomicBool::new(false));
        let sent = responded.clone();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + WAIT;
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        // This timeout is only bounded accept polling, never
                        // synchronization of forgetting against HTTP progress.
                        if Instant::now() >= deadline
                            || !matches!(
                                gate.recv_timeout(Duration::from_millis(2)),
                                Err(mpsc::RecvTimeoutError::Timeout)
                            )
                        {
                            return;
                        }
                    }
                    Err(error) => panic!("loopback accept: {error}"),
                }
            };
            stream.set_write_timeout(Some(WAIT)).unwrap();
            let mut bytes = Vec::new();
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                assert!(!remaining.is_zero(), "complete HTTP request timed out");
                stream.set_read_timeout(Some(remaining)).unwrap();
                let mut buffer = [0; 4096];
                let count = stream.read(&mut buffer).unwrap();
                assert!(count > 0, "client closed before complete HTTP request");
                bytes.extend_from_slice(&buffer[..count]);
                assert!(bytes.len() <= 128 * 1024, "bounded synthetic request");
                if let Some(at) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..at]).to_ascii_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .expect("production client sends content-length")
                        .parse()
                        .unwrap();
                    if bytes.len() >= at + 4 + length {
                        break;
                    }
                }
            }
            seen.send(()).unwrap();
            // A timeout closes the socket without a reply and fails the test;
            // it must never silently turn a slow test into a released response.
            if gate.recv_timeout(WAIT).is_err() {
                return;
            }
            let body = serde_json::json!({"choices":[{"message":{"content":REPLY}}]}).to_string();
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            sent.store(true, Ordering::Release);
            stream.write_all(response.as_bytes()).unwrap();
        });
        Self {
            url,
            received,
            release,
            responded,
            worker: Some(worker),
        }
    }

    fn respond(&self) {
        let _ = self.release.try_send(());
    }
}

impl Drop for HeldEndpoint {
    fn drop(&mut self) {
        self.respond();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn pointer(item: &AssistanceItemRow) -> AssistanceItemRequest {
    AssistanceItemRequest {
        item_id: item.item_id,
        expected_revision: item.revision,
    }
}

fn goal(session: &mut Session, source: Option<AssistanceItemRequest>) -> AssistanceItemRow {
    session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "Synthetic confirmed task".into(),
            summary: if source.is_some() { GOAL } else { INDEPENDENT }.into(),
            deadline_at: None,
            source_observation: source,
        })
        .unwrap()
        .item
}

fn memory_id(item: &AssistanceItemRow) -> Uuid {
    match item.source {
        AssistanceSource::ManualGoal { memory_id, .. } => memory_id,
        _ => panic!("manual goal owns a memory"),
    }
}

fn prepare(session: &mut Session, item: &AssistanceItemRow) -> Approval {
    let AssistanceContextView::Insight { insight, .. } =
        session.prepare_assistance_context(pointer(item)).unwrap()
    else {
        panic!("explicitly bound imported source")
    };
    session
        .prepare_draft_with_insight(PASTE, &insight.context_id.to_string(), false, false)
        .unwrap()
        .prepared
        .plan
        .approval()
}

fn audit_count(session: &Session, action: &str) -> usize {
    session
        .audit()
        .unwrap()
        .entries
        .iter()
        .filter(|entry| entry.action == action)
        .count()
}

#[test]
fn source_forget_during_http_rejects_old_result_keeps_slot_and_survives_restart() {
    let directory = tempfile::tempdir().unwrap();
    let control_endpoint = MockLlm::start().unwrap();
    control_endpoint.set_reply(REPLY);
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    session
        .set_user_endpoint(&control_endpoint.base_url())
        .unwrap();
    let work = session
        .begin_import_prepare(
            fixtures::read_text("import/soul-import-v1/valid_basic.jsonl").unwrap(),
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .unwrap();
    let prepared = session.complete_import_prepare(work.execute()).unwrap();
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    session.complete_import(work.execute()).unwrap();
    let bundle = session.local_insights().unwrap().remove(0);
    let contact_id = Uuid::parse_str(&bundle.summaries[0].contact_id).unwrap();
    let observation = session
        .link_assistance_observation(LinkAssistanceObservation {
            bundle_id: bundle.bundle_id,
            contact_id,
        })
        .unwrap()
        .item;
    let sourced = goal(&mut session, Some(pointer(&observation)));
    let independent = goal(&mut session, None);
    let (source_body, source_text) = {
        let shared = session.store().unwrap();
        let store = shared.lock().unwrap();
        // This fixture has sender IDs but no display labels. Follow the
        // production import index to a message owned by this exact contact,
        // rather than assuming an optional contact field or any arbitrary CK.
        let message = store
            .import_message_batch(prepared.operation.namespace_id, None, 128)
            .unwrap()
            .into_iter()
            .find(|message| message.sender_contact_id == contact_id)
            .expect("selected source contact owns an indexed imported message");
        let body = store
            .get_event(message.event_id)
            .unwrap()
            .body_ref
            .expect("indexed fixture message has sealed body");
        assert_eq!(body.content_key_id, message.content_key_id);
        assert!(store.has_content_key(body.content_key_id));
        let text = String::from_utf8(store.open(&body).unwrap()).unwrap();
        assert!(!text.is_empty());
        (body, text)
    };

    // Positive control rules out a permanently unusable source/context.
    let approved = prepare(&mut session, &sourced);
    let work = session.begin_draft(&approved).unwrap();
    assert_eq!(
        session.complete_draft(work.execute()).unwrap().source,
        DraftSource::UserEndpoint
    );
    assert_eq!(control_endpoint.request_count(), 1);
    assert_eq!(audit_count(&session, "draft.create"), 1);

    let endpoint = HeldEndpoint::start();
    session.set_user_endpoint(&endpoint.url).unwrap();
    let approved = prepare(&mut session, &sourced);
    let work = session.begin_draft(&approved).unwrap();
    let (completed_tx, completed_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        let _ = completed_tx.send(work.execute());
    });
    endpoint
        .received
        .recv_timeout(WAIT)
        .expect("server received the entire HTTP request before forgetting");
    assert!(!endpoint.responded.load(Ordering::Acquire));

    // Directly mutate the SAME store without Session.invalidate_drafts.
    // A worker that holds the store over HTTP would block this bounded join.
    let shared = session.store().unwrap();
    let (forgotten_tx, forgotten_rx) = mpsc::sync_channel(1);
    let forgetting = thread::spawn(move || {
        let outcome = shared
            .lock()
            .unwrap()
            .execute_forget_outcome(ForgetUnit::Contact(contact_id));
        let _ = forgotten_tx.send(outcome);
    });
    let forgotten = forgotten_rx
        .recv_timeout(WAIT)
        .expect("source forgetting completes while HTTP response is held")
        .unwrap();
    forgetting.join().unwrap();
    assert!(forgotten
        .receipt
        .impact
        .content_key_ids
        .contains(&source_body.content_key_id));
    assert!(!endpoint.responded.load(Ordering::Acquire));
    assert!(matches!(
        completed_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    let busy = session
        .prepare_draft("Synthetic next request", None)
        .unwrap_err();
    assert_eq!(busy, SessionRefusal::from(DraftRefusal::Busy));
    assert_eq!(audit_count(&session, "draft.create"), 1);
    {
        let shared = session.store().unwrap();
        let store = shared.lock().unwrap();
        assert!(!store.has_content_key(source_body.content_key_id));
        assert!(matches!(
            store.open(&source_body),
            Err(StoreError::ContentKeyDestroyed(id)) if id == source_body.content_key_id
        ));
    }
    let current = session
        .assistance_item(&sourced.item_id.to_string())
        .unwrap();
    assert_eq!(current.item.validity, AssistanceValidity::Stale);
    assert_eq!(current.item.handling, AssistanceHandling::Open);
    assert_eq!(current.goal.unwrap().summary, GOAL);
    assert_eq!(
        session
            .memory(&memory_id(&sourced).to_string())
            .unwrap()
            .summary,
        GOAL
    );
    assert_eq!(
        session
            .assistance_item(&independent.item_id.to_string())
            .unwrap()
            .item,
        independent
    );

    endpoint.respond();
    let completed = completed_rx
        .recv_timeout(WAIT)
        .expect("worker finishes after explicit HTTP release");
    worker.join().unwrap();
    assert!(endpoint.responded.load(Ordering::Acquire));
    assert_eq!(
        session.complete_draft(completed).unwrap_err().reason_code,
        "PLAN_HASH_MISMATCH"
    );
    assert_eq!(audit_count(&session, "egress.request"), 2);
    assert_eq!(audit_count(&session, "draft.create"), 1);
    assert!(session.begin_draft(&approved).is_err());
    // Capacity returns only after the physical worker has finished.
    session
        .prepare_draft("Synthetic fresh request", None)
        .unwrap();
    assert!(session.discard_draft());

    // Keep an unspent approval for an unchanged, surviving source as a
    // restart control; a spent permit alone cannot prove restart invalidation.
    let AssistanceContextView::Memory { memory, .. } = session
        .prepare_assistance_context(pointer(&independent))
        .unwrap()
    else {
        panic!("independent goal memory")
    };
    let independent_approval = session
        .prepare_draft_with_memory(PASTE, &memory.context_id.unwrap().to_string(), &[], false)
        .unwrap()
        .prepared
        .plan
        .approval();
    drop(session);
    let mut reopened = Session::open(directory.path());
    assert!(reopened.status().store_opened);
    reopened
        .set_user_endpoint(&control_endpoint.base_url())
        .unwrap();
    for old in [&approved, &independent_approval] {
        assert_eq!(
            reopened.begin_draft(old).unwrap_err().reason_code,
            "PLAN_HASH_MISMATCH"
        );
    }
    {
        let shared = reopened.store().unwrap();
        let store = shared.lock().unwrap();
        assert!(!store.has_content_key(source_body.content_key_id));
        assert!(matches!(
            store.open(&source_body),
            Err(StoreError::ContentKeyDestroyed(id)) if id == source_body.content_key_id
        ));
    }
    assert_eq!(
        reopened
            .assistance_item(&sourced.item_id.to_string())
            .unwrap()
            .item,
        current.item
    );
    assert_eq!(
        reopened
            .assistance_item(&independent.item_id.to_string())
            .unwrap()
            .item,
        independent
    );
    assert_eq!(
        reopened
            .memory(&memory_id(&independent).to_string())
            .unwrap()
            .summary,
        INDEPENDENT
    );
    assert_eq!(control_endpoint.request_count(), 1);
    assert_eq!(audit_count(&reopened, "egress.request"), 2);
    assert_eq!(audit_count(&reopened, "draft.create"), 1);
    let audit = reopened.audit().unwrap();
    assert!(audit.verified);
    let audit = serde_json::to_string(&audit).unwrap();
    for prose in [GOAL, INDEPENDENT, PASTE, REPLY, &source_text] {
        assert!(!audit.contains(prose));
    }
}
