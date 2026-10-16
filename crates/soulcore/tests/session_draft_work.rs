//! Real Session/SQLCipher and loopback HTTP tests of the owned draft lifecycle.
//! Synthetic prose only. The gate server holds a response until explicitly released.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::draft::Approval;
use soulcore::commands::memory::{ForgetConfirmation, MemoryChange, NewMemory};
use soulcore::commands::profile::GivenAnswer;
use soulcore::commands::session::{Session, SessionRefusal};

const WAIT: Duration = Duration::from_secs(8);
const PASTE: &str = "这是合成测试粘贴文本，不应进入审计。";
const REPLY: &str = "收到，周五见。";

struct GateEndpoint {
    url: String,
    received: mpsc::Receiver<()>,
    release: mpsc::SyncSender<()>,
    worker: Option<thread::JoinHandle<()>>,
}

impl GateEndpoint {
    fn start() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let (seen_tx, received) = mpsc::channel();
        let (release, go) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let until = Instant::now() + WAIT;
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() >= until || go.try_recv().is_ok() {
                            return;
                        }
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("test listener: {error}"),
                }
            };
            stream.set_read_timeout(Some(WAIT)).unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut buf = [0; 4096];
                let count = stream.read(&mut buf).unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buf[..count]);
                if let Some(at) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&bytes[..at]).to_ascii_lowercase();
                    let length: usize = head
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if bytes.len() >= at + 4 + length {
                        break;
                    }
                }
            }
            seen_tx.send(()).unwrap();
            let _ = go.recv();
            let body = serde_json::json!({"choices":[{"message":{"content":REPLY}}]}).to_string();
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            let _ = stream.write_all(response.as_bytes());
        });
        Self {
            url,
            received,
            release,
            worker: Some(worker),
        }
    }
    fn wait_for_request(&self) {
        self.received
            .recv_timeout(WAIT)
            .expect("request reached endpoint");
    }
    fn respond(&self) {
        let _ = self.release.try_send(());
    }
}
impl Drop for GateEndpoint {
    fn drop(&mut self) {
        self.respond();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn session(url: &str) -> (tempfile::TempDir, Session) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    session.set_user_endpoint(url).unwrap();
    (directory, session)
}
fn approval(session: &mut Session) -> Approval {
    session.prepare_draft(PASTE, None).unwrap().approval()
}
fn no_accepted_draft(session: &Session) {
    let audit = session.audit().unwrap();
    assert!(audit.verified);
    assert!(!audit.entries.iter().any(|e| e.action == "draft.create"));
    let text = serde_json::to_string(&audit).unwrap();
    assert!(!text.contains(PASTE));
    assert!(!text.contains(REPLY));
}

#[test]
fn withdrawal_finishes_while_http_is_blocked_and_the_reply_is_rejected() {
    let endpoint = GateEndpoint::start();
    let (_dir, mut session) = session(&endpoint.url);
    let approval = approval(&mut session);
    let shared = Arc::new(Mutex::new(session));
    let worker_session = shared.clone();
    let worker = thread::spawn(move || {
        let work = worker_session
            .lock()
            .unwrap()
            .begin_draft(&approval)
            .unwrap();
        let completed = work.execute();
        let result = worker_session.lock().unwrap().complete_draft(completed);
        result
    });
    endpoint.wait_for_request();
    let (tx, rx) = mpsc::channel();
    let controls = shared.clone();
    let control = thread::spawn(move || {
        let snapshot = controls.lock().unwrap().clear_user_endpoint();
        tx.send(snapshot).unwrap();
    });
    assert!(
        !rx.recv_timeout(WAIT)
            .expect("withdrawal does not wait for HTTP")
            .llm_endpoint_configured
    );
    control.join().unwrap();
    endpoint.respond();
    let refusal = worker.join().unwrap().unwrap_err();
    assert_eq!(refusal.reason_code, "PLAN_HASH_MISMATCH");
    let session = shared.lock().unwrap();
    no_accepted_draft(&session);
    let audit = session.audit().unwrap();
    assert_eq!(
        audit
            .entries
            .iter()
            .filter(|e| e.action == "egress.request" && e.decision == "allowed")
            .count(),
        1
    );
}

#[test]
fn issued_permit_can_run_after_withdrawal_but_cannot_be_accepted_or_replayed() {
    let endpoint = MockLlm::start().unwrap();
    let (_dir, mut session) = session(&endpoint.base_url());
    let approved = approval(&mut session);
    let work = session.begin_draft(&approved).unwrap();
    session.clear_user_endpoint();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    assert!(session.begin_draft(&approved).is_err());
    assert!(session.prepare_draft("new", None).is_err());
    let completed = work.execute();
    // Even a physically completed response occupies the slot until accounted for.
    assert!(session.prepare_draft("new", None).is_err());
    assert!(session.complete_draft(completed).is_err());
    assert_eq!(endpoint.request_count(), 1);
    no_accepted_draft(&session);
    let next = approval(&mut session);
    session.generate_draft(&next).unwrap();
    assert_eq!(endpoint.request_count(), 2);
}

#[test]
fn one_preparation_admits_one_worker_and_matching_completion_restores_capacity() {
    let endpoint = MockLlm::start().unwrap();
    let (_dir, mut session) = session(&endpoint.base_url());
    let approved = approval(&mut session);
    let work = session.begin_draft(&approved).unwrap();
    assert!(session.begin_draft(&approved).is_err());
    assert!(session.prepare_draft("other", None).is_err());
    session.complete_draft(work.execute()).unwrap();
    assert!(session.begin_draft(&approved).is_err());
    let next = approval(&mut session);
    session.generate_draft(&next).unwrap();
    assert_eq!(endpoint.request_count(), 2);
}

#[test]
fn discard_invalidates_the_reply_without_releasing_running_capacity() {
    let endpoint = MockLlm::start().unwrap();
    let (_dir, mut session) = session(&endpoint.base_url());
    let approved = approval(&mut session);
    let work = session.begin_draft(&approved).unwrap();
    assert!(session.discard_draft());
    assert!(session.prepare_draft("other", None).is_err());
    assert!(session.complete_draft(work.execute()).is_err());
    no_accepted_draft(&session);
    assert_eq!(endpoint.request_count(), 1);
    assert!(!session.discard_draft());
    session.prepare_draft("fresh", None).unwrap();
}

#[test]
fn dropped_unexecuted_work_reclaims_capacity_without_reusing_its_approval() {
    let endpoint = MockLlm::start().unwrap();
    let (_dir, mut session) = session(&endpoint.base_url());
    let approved = approval(&mut session);
    drop(session.begin_draft(&approved).unwrap());
    assert!(session.begin_draft(&approved).is_err());
    let next = approval(&mut session);
    session.generate_draft(&next).unwrap();
    assert_eq!(endpoint.request_count(), 1);
}

#[test]
fn completion_from_another_session_cannot_release_this_sessions_worker() {
    let endpoint = MockLlm::start().unwrap();
    let (_dir_a, mut first) = session(&endpoint.base_url());
    let (_dir_b, mut second) = session(&endpoint.base_url());
    let a = approval(&mut first);
    let b = approval(&mut second);
    let own = first.begin_draft(&a).unwrap();
    let foreign = second.begin_draft(&b).unwrap().execute();
    assert!(first.complete_draft(foreign).is_err());
    assert!(first.prepare_draft("cannot steal slot", None).is_err());
    first.complete_draft(own.execute()).unwrap();
    assert_eq!(endpoint.request_count(), 2);
}

#[test]
fn a_rejected_endpoint_parse_preserves_a_valid_preparation() {
    let endpoint = MockLlm::start().unwrap();
    let (_dir, mut session) = session(&endpoint.base_url());
    let approved = approval(&mut session);
    assert!(session
        .set_user_endpoint("https://private:secret@example.com")
        .is_err());
    session.generate_draft(&approved).unwrap();
    assert_eq!(endpoint.request_count(), 1);
}

fn memory() -> NewMemory {
    NewMemory {
        memory_type: "episodic".into(),
        title: "synthetic".into(),
        summary: "synthetic context".into(),
    }
}

fn mutate(session: &mut Session, kind: &str, endpoint: &str, memory_id: &str) {
    match kind {
        "same_endpoint" => {
            session.set_user_endpoint(endpoint).unwrap();
        }
        "endpoint_aba" => {
            session.set_user_endpoint("http://127.0.0.1:9").unwrap();
            session.set_user_endpoint(endpoint).unwrap();
        }
        "clear" => {
            session.clear_user_endpoint();
        }
        "voice" => {
            session.set_voice("register", "formal").unwrap();
        }
        "axis" => {
            let axis = session.profile().unwrap().axes[0].axis_id.clone();
            session.correct_axis(&axis, "leans_high").unwrap();
        }
        "intake" => {
            session
                .answer_questionnaire(&[GivenAnswer {
                    question_id: "q.axis.curiosity".into(),
                    given: "leans_high".into(),
                }])
                .unwrap();
        }
        "import" => {
            session
                .commit_soul_import_v1(
                    &fixtures::read_text("import/soul-import-v1/three_partners.jsonl").unwrap(),
                )
                .unwrap();
        }
        "failed_import" => {
            assert!(session.commit_telegram("not an export").is_err());
        }
        "failed_voice" => {
            assert!(session.set_voice("not-a-field", "not-a-value").is_err());
        }
        "memory_write" => {
            session.write_memory(&memory()).unwrap();
        }
        "memory_edit" => {
            session
                .edit_memory(
                    memory_id,
                    &MemoryChange {
                        title: Some("new synthetic".into()),
                        summary: Some("new synthetic context".into()),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        "forget" => {
            let preview = session.preview_forget(memory_id).unwrap();
            session
                .forget_memory(&ForgetConfirmation {
                    preview_id: preview.preview_id,
                    memory_id: memory_id.into(),
                })
                .unwrap();
        }
        "failed_graph" => {
            assert!(session.correct_tie("invalid-id", "strong").is_err());
        }
        "failed_release" => {
            assert!(session.release_tie("invalid-id").is_err());
        }
        _ => panic!("unknown synthetic mutation"),
    }
}
const MUTATIONS: &[&str] = &[
    "same_endpoint",
    "endpoint_aba",
    "clear",
    "voice",
    "axis",
    "intake",
    "import",
    "failed_import",
    "failed_voice",
    "memory_write",
    "memory_edit",
    "forget",
    "failed_graph",
    "failed_release",
];

#[test]
fn every_source_mutation_rejects_an_old_preparation_before_any_request() {
    let endpoint = MockLlm::start().unwrap();
    for kind in MUTATIONS {
        let (_dir, mut session) = session(&endpoint.base_url());
        let memory_id = session.write_memory(&memory()).unwrap().memory_id;
        let approved = approval(&mut session);
        mutate(&mut session, kind, &endpoint.base_url(), &memory_id);
        assert!(session.begin_draft(&approved).is_err(), "{kind}");
    }
    assert_eq!(endpoint.request_count(), 0);
}

#[test]
fn every_source_mutation_rejects_an_already_dispatched_reply() {
    let endpoint = MockLlm::start().unwrap();
    for kind in MUTATIONS {
        let (_dir, mut session) = session(&endpoint.base_url());
        let memory_id = session.write_memory(&memory()).unwrap().memory_id;
        let approved = approval(&mut session);
        let work = session.begin_draft(&approved).unwrap();
        mutate(&mut session, kind, &endpoint.base_url(), &memory_id);
        assert!(session.complete_draft(work.execute()).is_err(), "{kind}");
        no_accepted_draft(&session);
    }
    assert_eq!(endpoint.request_count(), MUTATIONS.len());
}

#[test]
fn failed_audit_never_returns_a_draft_and_does_not_permanently_fill_the_slot() {
    use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
    use soulcore::commands::{session::KEY_BLOB_FILE_NAME, store::database_path};
    let endpoint = MockLlm::start().unwrap();
    let (directory, mut session) = session(&endpoint.base_url());
    let keys: Box<dyn KeyProvider> = if cfg!(windows) {
        Box::new(DpapiKeyProvider::new(
            directory.path().join(KEY_BLOB_FILE_NAME),
        ))
    } else {
        Box::new(TestKeyProvider::in_dir(directory.path()))
    };
    let conn = rusqlite::Connection::open(database_path(directory.path())).unwrap();
    conn.execute_batch(&format!(
        "PRAGMA key = \"x'{}'\";",
        keys.database_key().unwrap().to_hex()
    ))
    .unwrap();
    let approved = approval(&mut session);
    let work = session.begin_draft(&approved).unwrap();
    conn.execute_batch("CREATE TRIGGER refuse_draft_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT, 'synthetic audit failure'); END;").unwrap();
    let refusal: SessionRefusal = session.complete_draft(work.execute()).unwrap_err();
    assert!(refusal.explanation.contains("synthetic audit failure"));
    conn.execute_batch("DROP TRIGGER refuse_draft_audit")
        .unwrap();
    no_accepted_draft(&session);
    let next = approval(&mut session);
    session.generate_draft(&next).unwrap();
    assert_eq!(endpoint.request_count(), 2);
}
