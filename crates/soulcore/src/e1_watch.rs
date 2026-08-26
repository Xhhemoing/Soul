//! The one path that wakes the HTTP client, with the sockets watched.
//!
//! [`crate::netwatch`] is pointed at two flows today, and both of them are
//! flows in which `reqwest` is never built: the main headless smoke starts
//! from [`Config::default`](crate::Config) and configures no endpoint, and
//! `collect-probe` opens no session at all. So the watcher has never once
//! observed the code that actually opens a socket — `set_user_endpoint`, then
//! a prepared draft the user approved, then a people summary offered to the
//! same address for rephrasing.
//!
//! The E1 tests in `soulcore/tests/session_e1.rs` do drive that path, and they
//! check it the only way a mock server can: `MockLlm::request_count()`. That
//! number is a statement about the requests that *arrived*, and it is a good
//! one — but a mock cannot see a socket it is not the far end of. A build that
//! resolved a hostname on the way, consulted a proxy, or phoned somewhere else
//! entirely while also answering the mock correctly would leave every one of
//! those assertions green. Counting arrivals and watching departures are two
//! different measurements, and only this module takes the second one on the
//! path where there is something to depart.
//!
//! So this is the instrument for that. Like `collect-probe` it is not a
//! product surface: no shell command reaches it, it is reachable only through
//! `soul-headless e1-watch`, and everything it writes goes to a scratch
//! directory that is deleted when it returns.
//!
//! ## What it runs
//!
//! A [`Session`] on a scratch directory, driven through the product's own
//! methods in the order the desktop shell calls them:
//!
//! 1. an import, so the graph has somebody to summarize;
//! 2. `set_user_endpoint`, pointed at the loopback endpoint below;
//! 3. `prepare_draft`, which builds a body and sends nothing;
//! 4. `generate_draft` on the approval the plan hands back — the first request;
//! 5. `person_summary`, which offers the counts to the same address — the
//!    second;
//! 6. the same approval a second time, which is `session_e1.rs`'s replay cell:
//!    the preparation has been spent, so the refusal must cost no socket at
//!    all.
//!
//! ## What it asserts
//!
//! * **No non-loopback connection, ever.** The same reading AC-21 rests on,
//!   taken this time in a process that has an HTTP client running in it.
//! * **Every loopback peer accounted for.** A loopback socket is not a finding
//!   on its own — the endpoint under test is one — so the question is *which*
//!   loopback peer. Two ports are legitimate: the endpoint's own, which is
//!   where the client connected, and the ephemeral port of each connection the
//!   endpoint accepted, which is this process talking to itself from the other
//!   side. Anything else is reported by name and fails the run.
//! * **The arrival count matches the path.** [`EXPECTED_REQUESTS`] requests
//!   reached the endpoint: one approved generation and one rephrasing. A third
//!   would mean the replay bought one.
//!
//! ## The endpoint
//!
//! In-process, on `127.0.0.1` and an ephemeral port, and written here out of
//! `std::net` rather than borrowed from `soul-testkit`. That crate owns an
//! HTTP stack on purpose, and `xtask e0-audit` keeps it reachable only through
//! a dev edge: a shipped `soulcore` that named it as a normal dependency would
//! put `hyper` back into the graph the audit exists to keep clear. Sixty lines
//! of blocking accept-and-answer is the cheaper side of that trade, and it is
//! all one OpenAI-compatible round trip needs.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::commands::session::Session;
// The same embedded corpus the main smoke imports: `fixtures/` is a repository
// directory and an installed Soul has no repository.
use crate::headless::{HeadlessError, StepReport, IMPORT_CORPUS};
use crate::netwatch::{self, Peer, WatchReport};

/// How many requests the product path above puts on the wire.
///
/// One approved generation and one summary rephrasing. The replayed approval
/// is refused before a body exists, so it adds nothing.
pub const EXPECTED_REQUESTS: usize = 2;

/// What the user pastes into the drafting panel.
const PASTED_MESSAGE: &str = "周五的场地我已经订好了，你直接过来就行";

/// What the endpoint answers with.
///
/// It has to survive `soul_draft::reply::read_grounded_in`, which refuses an
/// answer stating a figure the material does not and an answer sharing almost
/// nothing with it. The material is the counts body, and its first line is the
/// statistical header — so a sentence built out of the header's own words is
/// grounded whatever this corpus's counts turn out to be. An instrument whose
/// endpoint's answer was accepted on one fixture and dropped on the next would
/// be measuring the fixture.
const REPHRASING: &str = "这些都是本机统计出来的往来计数，不是判断。";

/// How long the endpoint waits for a request to finish arriving.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// How long the endpoint sits on a request before answering it.
///
/// Deliberate, and the reason is the whole point of the module. A loopback
/// chat completion answered immediately is open for well under a millisecond,
/// and [`netwatch::SAMPLE_INTERVAL`] is five: a run that reported zero
/// non-loopback peers would be reporting zero peers of any kind, which is the
/// reading an observer that could not see sockets at all also produces. Held
/// open for [`HOLD_OPEN`] the connection spans a couple of dozen samples, so
/// [`SocketFindings::loopback_peers`] carries the endpoint's own port and the
/// zero beside it is a measurement rather than a silence.
///
/// Twenty-odd rather than two: missing the connection has to stay impossible
/// on a loaded runner, where the sampling thread can be descheduled for
/// longer than one interval, and a quarter of a second is not a price anybody
/// pays twice.
const HOLD_OPEN: Duration = Duration::from_millis(120);

/// How often the accept loop looks, so that a shutdown is noticed promptly
/// without a second socket to wake it up with.
const ACCEPT_POLL: Duration = Duration::from_millis(2);

/// One request as the loopback endpoint received it.
///
/// Not the body. This instrument is about sockets, and `session_e1.rs` already
/// reads the bytes: keeping a redacted conversation in a report that gets
/// pasted into an issue is a cost with nothing on the other side of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArrivedRequest {
    pub method: String,
    pub path: String,
    /// The ephemeral loopback port the client connected from. Kept because the
    /// socket at this end of that connection is one the watcher will see.
    pub from_port: u16,
    /// This build has no key to send, so an `authorization` header would be a
    /// finding.
    pub carried_authorization: bool,
}

/// What arrived at the endpoint the session was pointed at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointFindings {
    pub port: u16,
    pub requests: Vec<ArrivedRequest>,
}

/// What the kernel said this process was connected to while that happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SocketFindings {
    /// True when the platform let the sockets be read. False is not a pass;
    /// see [`crate::netwatch`].
    pub observed: bool,
    pub observation_note: Option<String>,
    pub samples: u64,
    /// Zero, or the run failed.
    pub non_loopback_connections: usize,
    pub non_loopback_peers: Vec<String>,
    pub loopback_connections: usize,
    pub loopback_peers: Vec<String>,
    /// The endpoint's port, and the ephemeral port of every connection it
    /// accepted. See the module docs.
    pub expected_loopback_ports: Vec<u16>,
    /// Empty, or the run failed.
    pub unexpected_loopback_peers: Vec<String>,
    /// Whether the watcher caught the connection to the endpoint. The control
    /// on the zero above: see [`HOLD_OPEN`]. Only meaningful where the
    /// platform let the sockets be read at all.
    pub saw_the_endpoint: bool,
}

/// Everything one `e1-watch` run found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct E1WatchReport {
    /// True only if every step passed. The process exit code says the same
    /// thing; this is here so a captured log is readable on its own.
    pub ok: bool,
    pub steps: Vec<StepReport>,
    pub endpoint: EndpointFindings,
    pub sockets: SocketFindings,
    pub audit_entries: usize,
    pub audit_chain_verified: bool,
    pub scratch_removed: bool,
}

/// Run the whole thing in a scratch directory of its own, and clean up.
///
/// The directory is removed whether the run passed or failed, for the reason
/// [`crate::headless::run`] removes its own: on a real machine this sits
/// beside an actual library, and an instrument that left an encrypted database
/// in the temp folder is one nobody runs twice.
pub fn run() -> Result<E1WatchReport, HeadlessError> {
    let scratch = crate::headless::scratch_dir_named("soul-e1-watch")?;
    let outcome = run_in(&scratch);
    let _ = std::fs::remove_dir_all(&scratch);

    let mut report = outcome?;
    if scratch.exists() {
        return Err(fail(format!(
            "the scratch directory {} was still there afterwards",
            scratch.display(),
        )));
    }
    report.scratch_removed = true;
    Ok(report)
}

/// The run, against a directory the caller owns.
///
/// The endpoint is bound before the watcher starts. A listening socket has no
/// peer and is invisible to [`netwatch`] either way, but binding it first
/// means a port that could not be taken fails as itself rather than as a
/// mysteriously empty observation.
pub fn run_in(scratch: &Path) -> Result<E1WatchReport, HeadlessError> {
    let endpoint = LoopbackEndpoint::start()?;
    let port = endpoint.port();

    let watch = netwatch::Watch::start();
    let outcome = flow(scratch, &endpoint);
    let observed = watch.stop();

    let arrived = endpoint.arrived();
    endpoint.shutdown();

    let passed = outcome?;
    let sockets = socket_findings(observed, port, &arrived)?;
    require(
        arrived.len() == EXPECTED_REQUESTS,
        format!(
            "the endpoint received {} request(s), and this path makes {EXPECTED_REQUESTS}",
            arrived.len(),
        ),
    )?;
    require(
        !arrived.iter().any(|request| request.carried_authorization),
        "a request carried an authorization header, and this build has no key",
    )?;

    Ok(E1WatchReport {
        ok: true,
        steps: passed.steps,
        endpoint: EndpointFindings {
            port,
            requests: arrived,
        },
        sockets,
        audit_entries: passed.audit_entries,
        audit_chain_verified: true,
        // Set by `run`, which is what owns the directory.
        scratch_removed: false,
    })
}

/// What the flow proved, before the observations are folded in.
#[derive(Debug)]
struct FlowOutcome {
    steps: Vec<StepReport>,
    audit_entries: usize,
}

fn fail(detail: impl Into<String>) -> HeadlessError {
    HeadlessError {
        step: "e1-watch",
        detail: detail.into(),
    }
}

fn at<T, E: std::fmt::Display>(result: Result<T, E>) -> Result<T, HeadlessError> {
    result.map_err(|error| fail(error.to_string()))
}

fn require(holds: bool, detail: impl Into<String>) -> Result<(), HeadlessError> {
    match holds {
        true => Ok(()),
        false => Err(fail(detail)),
    }
}

/// A session refusal, as a failure that names the code rather than the prose.
fn refused(step: &str, refusal: crate::commands::session::SessionRefusal) -> HeadlessError {
    fail(format!("{step} was refused as {}", refusal.reason_code))
}

fn flow(scratch: &Path, endpoint: &LoopbackEndpoint) -> Result<FlowOutcome, HeadlessError> {
    let mut steps = Vec::new();
    let mut step = |name: &str, detail: String| {
        steps.push(StepReport {
            name: name.to_owned(),
            detail,
        });
    };

    let mut session = Session::open(scratch);
    let status = session.status();
    require(
        status.store_opened,
        format!("the scratch store did not open: {}", status.store_notice),
    )?;

    // --- Somebody to summarize ---------------------------------------------
    let receipt = session
        .commit_soul_import_v1(IMPORT_CORPUS)
        .map_err(|refusal| refused("the import", refusal))?;
    require(
        receipt.contacts_created >= 3 && receipt.ties_rebuilt >= 1,
        format!(
            "the corpus left {} contact(s) and {} tie(s)",
            receipt.contacts_created, receipt.ties_rebuilt,
        ),
    )?;
    let contact_id = someone_to_summarize(&session)?;
    step(
        "import",
        format!(
            "{} event(s), {} contact(s), {} tie(s)",
            receipt.events_written, receipt.contacts_created, receipt.ties_rebuilt,
        ),
    );

    // --- The address, which is not a request -------------------------------
    let snapshot = session
        .set_user_endpoint(&endpoint.base_url())
        .map_err(|refusal| refused("the endpoint address", refusal))?;
    require(
        snapshot.llm_endpoint_configured && !snapshot.fully_closed,
        "the session does not think it has an endpoint",
    )?;
    require(
        endpoint.arrived().is_empty(),
        "filling in the address contacted it",
    )?;
    step(
        "endpoint",
        "saved; no request was made to save it".to_owned(),
    );

    // --- A plan, which is also not a request -------------------------------
    let plan = session
        .prepare_draft(PASTED_MESSAGE, None)
        .map_err(|refusal| refused("the preparation", refusal))?;
    let approval = plan.approval();
    require(
        endpoint.arrived().is_empty(),
        "describing a request sent it",
    )?;
    step(
        "prepare",
        format!(
            "{} third-party turn(s), {} placeheld; nothing sent",
            plan.third_party_turns, plan.placeheld_turns,
        ),
    );

    // --- The approval, which is ---------------------------------------------
    let draft = session
        .generate_draft(&approval)
        .map_err(|refusal| refused("the approved generation", refusal))?;
    require(
        !draft.text.trim().is_empty(),
        "the generation produced nothing to copy",
    )?;
    require(
        endpoint.arrived().len() == 1,
        format!(
            "one approval left {} request(s) at the endpoint",
            endpoint.arrived().len(),
        ),
    )?;
    step(
        "generate",
        format!(
            "{} character(s) back, source {:?}, over 1 request",
            draft.text.chars().count(),
            draft.source,
        ),
    );

    // --- The graph click, which is the path's second half -------------------
    let summary = session
        .person_summary(&contact_id)
        .map_err(|refusal| refused("the people summary", refusal))?;
    require(
        !summary.points.is_empty() && summary.points.iter().all(|p| !p.evidence_ids.is_empty()),
        "a summary point reached the surface with nothing behind it",
    )?;
    require(
        endpoint.arrived().len() == 2,
        format!(
            "one summary left {} request(s) at the endpoint in total",
            endpoint.arrived().len(),
        ),
    )?;
    step(
        "summary",
        format!(
            "{} point(s), source {}, over 1 more request",
            summary.points.len(),
            summary.source,
        ),
    );

    // --- The replay, which must cost nothing --------------------------------
    let replayed = session
        .generate_draft(&approval)
        .err()
        .ok_or_else(|| fail("the spent approval was accepted a second time"))?;
    require(
        replayed.reason_code == "PLAN_HASH_MISMATCH",
        format!("the replay was refused as {}", replayed.reason_code),
    )?;
    require(
        endpoint.arrived().len() == EXPECTED_REQUESTS,
        format!(
            "a replayed approval opened a socket: {} request(s) in total",
            endpoint.arrived().len(),
        ),
    )?;
    step(
        "replay",
        format!(
            "the same approval again, refused as {}, no request",
            replayed.reason_code,
        ),
    );

    // --- What the chain heard about all of it -------------------------------
    let chain = session
        .audit()
        .map_err(|refusal| refused("reading the chain", refusal))?;
    require(
        chain.verified,
        match &chain.verification_problem {
            Some(problem) => format!("the audit chain did not verify: {problem}"),
            None => "the audit chain did not verify".to_owned(),
        },
    )?;
    let requests_recorded = chain
        .entries
        .iter()
        .filter(|entry| entry.action == "egress.request")
        .count();
    require(
        requests_recorded == EXPECTED_REQUESTS,
        format!("{requests_recorded} request(s) are in the chain and {EXPECTED_REQUESTS} went out",),
    )?;
    step(
        "audit",
        format!(
            "{} entries, chain verified, {requests_recorded} of them egress.request",
            chain.entries.len(),
        ),
    );

    let audit_entries = chain.entries.len();
    Ok(FlowOutcome {
        steps,
        audit_entries,
    })
}

/// The person this graph has the most exchanges for, off the same list the
/// Graph page draws.
fn someone_to_summarize(session: &Session) -> Result<String, HeadlessError> {
    session
        .people()
        .map_err(|refusal| refused("reading the graph", refusal))?
        .people
        .into_iter()
        .filter(|person| !person.is_you && person.tie_count > 0)
        .max_by_key(|person| person.interaction_count)
        .map(|person| person.contact_id)
        .ok_or_else(|| fail("the corpus left nobody with a tie to summarize"))
}

/// Sort the observed peers into the ones this run explains and the ones it
/// does not.
fn socket_findings(
    observed: WatchReport,
    endpoint_port: u16,
    arrived: &[ArrivedRequest],
) -> Result<SocketFindings, HeadlessError> {
    let mut expected: BTreeSet<u16> = arrived.iter().map(|request| request.from_port).collect();
    expected.insert(endpoint_port);

    let unexpected: Vec<String> = observed
        .loopback
        .iter()
        .filter(|peer| !expected.contains(&peer.port))
        .map(ToString::to_string)
        .collect();

    let note = match &observed.support {
        netwatch::Support::Observed => None,
        netwatch::Support::Unsupported(reason) => Some(reason.clone()),
    };
    let findings = SocketFindings {
        observed: observed.support.is_observed(),
        observation_note: note,
        samples: observed.samples,
        non_loopback_connections: observed.non_loopback.len(),
        non_loopback_peers: described(&observed.non_loopback),
        loopback_connections: observed.loopback.len(),
        loopback_peers: described(&observed.loopback),
        expected_loopback_ports: expected.into_iter().collect(),
        unexpected_loopback_peers: unexpected,
        saw_the_endpoint: observed
            .loopback
            .iter()
            .any(|peer| peer.port == endpoint_port),
    };

    require(
        findings.non_loopback_connections == 0,
        format!(
            "this process held {} non-loopback connection(s) while the endpoint path ran: {}",
            findings.non_loopback_connections,
            findings.non_loopback_peers.join(", "),
        ),
    )?;
    require(
        findings.unexpected_loopback_peers.is_empty(),
        format!(
            "this process talked to a loopback peer the endpoint path does not explain: {}",
            findings.unexpected_loopback_peers.join(", "),
        ),
    )?;
    // The control, and only where there was something to control. On a
    // platform with no `/proc` the observation did not happen at all, and
    // `observed: false` is how the report says so rather than this.
    require(
        !findings.observed || findings.saw_the_endpoint,
        format!(
            "the watcher never caught the connection to 127.0.0.1:{endpoint_port}, \
             so the zero beside it is not evidence: {:?}",
            findings.loopback_peers,
        ),
    )?;
    Ok(findings)
}

fn described(peers: &[Peer]) -> Vec<String> {
    peers.iter().map(ToString::to_string).collect()
}

// ------------------------------------------------- the loopback endpoint ---

/// An OpenAI-compatible endpoint on `127.0.0.1`, and nowhere else.
///
/// One connection at a time, answered on the accept loop's own thread. The
/// product path is sequential — a draft, then a summary — so a thread per
/// connection would buy nothing but a second place for this file to be wrong.
#[derive(Debug)]
struct LoopbackEndpoint {
    addr: SocketAddr,
    arrived: Arc<Mutex<Vec<ArrivedRequest>>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl LoopbackEndpoint {
    fn start() -> Result<LoopbackEndpoint, HeadlessError> {
        let listener = at(TcpListener::bind(("127.0.0.1", 0)))?;
        let addr = at(listener.local_addr())?;
        at(listener.set_nonblocking(true))?;

        let arrived = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let worker = {
            let arrived = Arc::clone(&arrived);
            let stop = Arc::clone(&stop);
            at(std::thread::Builder::new()
                .name("soul-e1-watch-endpoint".to_owned())
                .spawn(move || serve(listener, &arrived, &stop)))?
        };

        Ok(LoopbackEndpoint {
            addr,
            arrived,
            stop,
            worker: Some(worker),
        })
    }

    fn port(&self) -> u16 {
        self.addr.port()
    }

    /// The address the settings page would be given.
    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }

    fn arrived(&self) -> Vec<ArrivedRequest> {
        self.arrived
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn shutdown(mut self) {
        self.stop_serving();
    }

    fn stop_serving(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for LoopbackEndpoint {
    fn drop(&mut self) {
        self.stop_serving();
    }
}

fn serve(listener: TcpListener, arrived: &Arc<Mutex<Vec<ArrivedRequest>>>, stop: &AtomicBool) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                let _ = answer(stream, peer, arrived);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(ACCEPT_POLL)
            }
            Err(_) => return,
        }
    }
}

/// Read one request, write it down, and answer it.
///
/// The whole request is read before anything is written: a server that
/// answered while the body was still arriving would leave the client holding a
/// reset, and the failure would look like an egress defect rather than like
/// this function.
fn answer(
    mut stream: TcpStream,
    peer: SocketAddr,
    arrived: &Arc<Mutex<Vec<ArrivedRequest>>>,
) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(READ_TIMEOUT))?;

    let mut reader = BufReader::new(stream.try_clone()?);
    let mut start_line = String::new();
    reader.read_line(&mut start_line)?;
    let mut fields = start_line.split_whitespace();
    let method = fields.next().unwrap_or_default().to_owned();
    let path = fields.next().unwrap_or_default().to_owned();

    let mut content_length = 0usize;
    let mut carried_authorization = false;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value.trim().parse().unwrap_or(0);
        }
        if name.eq_ignore_ascii_case("authorization") {
            carried_authorization = true;
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;

    arrived
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(ArrivedRequest {
            method,
            path,
            from_port: peer.port(),
            carried_authorization,
        });

    // Answered late on purpose; see `HOLD_OPEN`.
    std::thread::sleep(HOLD_OPEN);

    let answer = serde_json::json!({
        "id": "soul-e1-watch",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": REPHRASING },
            "finish_reason": "stop"
        }]
    })
    .to_string();
    write!(
        stream,
        "HTTP/1.1 200 OK\r\n\
         content-type: application/json\r\n\
         content-length: {}\r\n\
         connection: close\r\n\
         \r\n",
        answer.len(),
    )?;
    stream.write_all(answer.as_bytes())?;
    stream.flush()?;
    let _ = stream.shutdown(Shutdown::Write);
    Ok(())
}
