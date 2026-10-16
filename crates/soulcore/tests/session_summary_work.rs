//! Owned person-summary work over real Session/SQLCipher and synthetic loopback HTTP.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use soul_policy::e1::{DRAFTING_INSTRUCTION, PERSON_SUMMARY_INSTRUCTION};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::memory::{ForgetConfirmation, MemoryChange, NewMemory};
use soulcore::commands::profile::GivenAnswer;
use soulcore::commands::session::Session;

const WAIT: Duration = Duration::from_secs(8);
const REPLY: &str = "你们最近往来比较稳定，多数时候是一对一说话。";

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

fn session(endpoint: Option<&str>) -> (tempfile::TempDir, Session, String) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    session
        .commit_telegram(&fixtures::read_text("import/telegram/result_basic.json").unwrap())
        .unwrap();
    if let Some(endpoint) = endpoint {
        session.set_user_endpoint(endpoint).unwrap();
    }
    let contact_id = session
        .people()
        .unwrap()
        .people
        .into_iter()
        .filter(|person| !person.is_you && person.tie_count > 0)
        .max_by_key(|person| person.interaction_count)
        .unwrap()
        .contact_id;
    (directory, session, contact_id)
}

fn allowed_requests(session: &Session) -> usize {
    let audit = session.audit().unwrap();
    assert!(audit.verified);
    let text = serde_json::to_string(&audit).unwrap();
    assert!(!text.contains(REPLY));
    audit
        .entries
        .iter()
        .filter(|entry| entry.action == "egress.request" && entry.decision == "allowed")
        .count()
}

#[test]
fn withdrawal_and_reads_finish_while_summary_http_is_blocked() {
    let endpoint = GateEndpoint::start();
    let (_directory, mut session, contact) = session(Some(&endpoint.url));
    let work = session.begin_person_summary(&contact).unwrap();
    let shared = Arc::new(Mutex::new(session));
    let worker_session = shared.clone();
    let worker = thread::spawn(move || {
        let completed = work.execute();
        worker_session
            .lock()
            .unwrap()
            .complete_person_summary(completed)
    });
    endpoint.wait_for_request();
    let (tx, rx) = mpsc::channel();
    let controls = shared.clone();
    let control = thread::spawn(move || {
        let mut session = controls.lock().unwrap();
        assert!(session.people().is_ok());
        assert!(session.audit().is_ok());
        assert!(session.store().unwrap().try_lock().is_ok());
        let snapshot = session.clear_user_endpoint();
        let counts = session.person_summary(&contact).unwrap();
        assert_eq!(counts.source, "counts");
        tx.send(snapshot).unwrap();
    });
    assert!(
        !rx.recv_timeout(WAIT)
            .expect("controls do not wait for HTTP")
            .llm_endpoint_configured
    );
    control.join().unwrap();
    endpoint.respond();
    assert_eq!(
        worker.join().unwrap().unwrap_err().reason_code,
        "PLAN_HASH_MISMATCH"
    );
    assert_eq!(allowed_requests(&shared.lock().unwrap()), 1);
}

#[test]
fn summary_and_draft_share_one_physical_worker_until_completion_is_accounted_for() {
    let endpoint = MockLlm::start().unwrap();
    let (_directory, mut session, contact) = session(Some(&endpoint.base_url()));
    let work = session.begin_person_summary(&contact).unwrap();
    assert!(session.begin_person_summary(&contact).is_err());
    assert!(session.prepare_draft("synthetic", None).is_err());
    session.clear_user_endpoint();
    assert_eq!(session.person_summary(&contact).unwrap().source, "counts");
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    assert!(session.begin_person_summary(&contact).is_err());
    let completed = work.execute();
    assert!(session.begin_person_summary(&contact).is_err());
    assert!(session.prepare_draft("synthetic", None).is_err());
    assert!(session.complete_person_summary(completed).is_err());
    assert_eq!(endpoint.request_count(), 1);
    assert_eq!(allowed_requests(&session), 1);
    let approved = session.prepare_draft("synthetic", None).unwrap().approval();
    let draft_work = session.begin_draft(&approved).unwrap();
    assert!(session.begin_person_summary(&contact).is_err());
    assert_eq!(endpoint.request_count(), 1);
    session.clear_user_endpoint();
    assert_eq!(session.person_summary(&contact).unwrap().source, "counts");
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    assert!(session.begin_person_summary(&contact).is_err());
    assert!(session.complete_draft(draft_work.execute()).is_err());
    session.person_summary(&contact).unwrap();
    assert_eq!(endpoint.request_count(), 3);
}

#[test]
fn dropping_unexecuted_summary_work_reclaims_the_slot_without_a_request() {
    let endpoint = MockLlm::start().unwrap();
    let (_directory, mut session, contact) = session(Some(&endpoint.base_url()));
    drop(session.begin_person_summary(&contact).unwrap());
    assert_eq!(endpoint.request_count(), 0);
    session.person_summary(&contact).unwrap();
    assert_eq!(endpoint.request_count(), 1);
}

#[test]
fn a_foreign_summary_completion_cannot_release_this_sessions_worker() {
    let endpoint = MockLlm::start().unwrap();
    let (_a, mut first, a) = session(Some(&endpoint.base_url()));
    let (_b, mut second, b) = session(Some(&endpoint.base_url()));
    let own = first.begin_person_summary(&a).unwrap();
    let foreign = second.begin_person_summary(&b).unwrap().execute();
    assert!(first.complete_person_summary(foreign).is_err());
    assert!(first.begin_person_summary(&a).is_err());
    assert!(first.prepare_draft("synthetic", None).is_err());
    first.complete_person_summary(own.execute()).unwrap();
    assert_eq!(endpoint.request_count(), 2);
    assert_eq!(allowed_requests(&first), 1);
}

#[test]
fn local_counts_still_require_the_same_source_revision_at_completion() {
    let endpoint = MockLlm::start().unwrap();
    let (_directory, mut session, contact) = session(None);
    let work = session.begin_person_summary(&contact).unwrap();
    session.set_voice("register", "formal").unwrap();
    assert_eq!(
        session
            .complete_person_summary(work.execute())
            .unwrap_err()
            .reason_code,
        "PLAN_HASH_MISMATCH"
    );
    assert_eq!(session.person_summary(&contact).unwrap().source, "counts");
    assert_eq!(endpoint.request_count(), 0);
    assert_eq!(allowed_requests(&session), 0);
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
                .commit_telegram(&fixtures::read_text("import/telegram/result_basic.json").unwrap())
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
                        summary: Some("changed synthetic context".into()),
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
        "tie" => {
            let tie = session.people().unwrap().ties[0].relationship_id.clone();
            session.correct_tie(&tie, "strong").unwrap();
        }
        "release" => {
            let tie = session.people().unwrap().ties[0].relationship_id.clone();
            session.release_tie(&tie).unwrap();
        }
        _ => panic!("unknown synthetic mutation"),
    }
}

#[test]
fn source_changes_reject_old_counts_and_endpoint_prose_after_recording_the_request() {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    for kind in [
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
        "tie",
        "release",
    ] {
        let (_directory, mut session, contact) = session(Some(&endpoint.base_url()));
        let memory_id = session.write_memory(&memory()).unwrap().memory_id;
        let work = session.begin_person_summary(&contact).unwrap();
        mutate(&mut session, kind, &endpoint.base_url(), &memory_id);
        assert!(
            session.complete_person_summary(work.execute()).is_err(),
            "{kind}"
        );
        assert_eq!(allowed_requests(&session), 1, "{kind}");
        // All invalidations leave capacity available only after completion.
        session.set_user_endpoint(&endpoint.base_url()).unwrap();
        drop(session.begin_person_summary(&contact).unwrap());
    }
    assert_eq!(endpoint.request_count(), 16);
}

#[test]
fn rejected_endpoint_parse_preserves_a_live_summary_snapshot() {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
    let (_directory, mut session, contact) = session(Some(&endpoint.base_url()));
    let work = session.begin_person_summary(&contact).unwrap();
    assert!(session
        .set_user_endpoint("https://private:secret@example.com")
        .is_err());
    let summary = session.complete_person_summary(work.execute()).unwrap();
    assert_eq!(summary.source, "user_endpoint");
    assert_eq!(endpoint.request_count(), 1);
}

#[test]
fn invalid_or_clinical_replies_leave_the_exact_counts_and_evidence_standing() {
    let endpoint = MockLlm::start().unwrap();
    let (_directory, mut session, contact) = session(None);
    let counts = session.person_summary(&contact).unwrap();
    session.set_user_endpoint(&endpoint.base_url()).unwrap();
    for reply in ["", "这个人最喜欢榴莲。", "这个人有抑郁症。"] {
        endpoint.set_reply(reply);
        assert_eq!(session.person_summary(&contact).unwrap(), counts);
    }
    assert_eq!(endpoint.request_count(), 3);
    assert_eq!(allowed_requests(&session), 3);
    for request in endpoint.requests() {
        assert!(request.body.contains(PERSON_SUMMARY_INSTRUCTION));
        assert!(!request.body.contains(DRAFTING_INSTRUCTION));
        assert!(request.body.contains("【本机统计，供改写参考"));
    }
}

#[test]
fn transport_failure_keeps_counts_and_records_the_attempt_without_endpoint_prose() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let (_directory, mut session, contact) = session(None);
    let counts = session.person_summary(&contact).unwrap();
    session.set_user_endpoint(&url).unwrap();
    assert_eq!(session.person_summary(&contact).unwrap(), counts);
    let audit = session.audit().unwrap();
    assert!(audit.verified);
    assert_eq!(
        audit
            .entries
            .iter()
            .filter(|entry| entry.action == "egress.request")
            .count(),
        1
    );
}

#[test]
fn failed_audit_never_returns_a_summary_and_releases_only_its_finished_slot() {
    use soul_store::{DpapiKeyProvider, KeyProvider, TestKeyProvider};
    use soulcore::commands::{session::KEY_BLOB_FILE_NAME, store::database_path};

    let endpoint = MockLlm::start().unwrap();
    let (directory, mut session, contact) = session(Some(&endpoint.base_url()));
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
    let work = session.begin_person_summary(&contact).unwrap();
    conn.execute_batch("CREATE TRIGGER refuse_summary_audit BEFORE INSERT ON audit BEGIN SELECT RAISE(ABORT, 'synthetic audit failure'); END;").unwrap();
    let refusal = session.complete_person_summary(work.execute()).unwrap_err();
    assert!(refusal.explanation.contains("synthetic audit failure"));
    conn.execute_batch("DROP TRIGGER refuse_summary_audit")
        .unwrap();
    assert_eq!(allowed_requests(&session), 0);
    session.person_summary(&contact).unwrap();
    assert_eq!(endpoint.request_count(), 2);
    assert_eq!(allowed_requests(&session), 1);
}
