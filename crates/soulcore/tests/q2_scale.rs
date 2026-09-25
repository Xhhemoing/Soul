//! Q2-03 adds the scale/peer-count cross-product that the small import fixtures
//! and the 400-row source-index query-plan test do not cover. Only public Session
//! import and read surfaces are exercised; each fresh store is imported once.
//!
//! The ordinary test catches dropped/duplicated messages, merged peers, missing
//! direct edges, leaked research rows, and a broken audit chain at 100/10. The
//! ignored test runs the same assertions at 1k/10k messages and 10/100 peers.
//! It is descriptive measurement, with no elapsed-time pass/fail threshold.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use soul_store::{SqlCipherStore, TestKeyProvider};
use soulcore::commands::{
    graph as graph_commands, import as import_commands,
    session::{Session, SessionRefusal},
};

const STAGES: [&str; 9] = [
    "open",
    "preview",
    "commit_including_rebuild",
    "people",
    "profile",
    "memories",
    "research",
    "audit",
    "total",
];
const DECOMPOSITION_AT_UNIX_SECONDS: i64 = 1_800_000_000;
const DECOMPOSITION_KEY_SEED: &str = "q2 performance decomposition";

/// Every peer has its own two-person conversation, with half the messages in
/// each direction. All ids, text, timestamps, ordering, and LF bytes are fixed.
/// At most 10,000 seconds are used, so all times fit within the specified day.
fn synthetic_jsonl(messages: usize, peers: usize) -> String {
    assert!(peers > 0 && messages <= 10_000);
    assert_eq!(messages % (2 * peers), 0);
    let mut text = String::from(
        "{\"type\":\"header\",\"format\":\"soul-import-v1\",\"version\":1,\"exported_at\":\"2026-08-19T00:00:00Z\"}\n",
    );
    for index in 0..messages {
        let peer = index % peers;
        let outgoing = (index / peers) % 2 == 0;
        let row = json!({
            "type": "message",
            "id": format!("q2-message-{index:05}"),
            "occurred_at": format!(
                "2026-08-18T{:02}:{:02}:{:02}Z",
                index / 3600, (index / 60) % 60, index % 60,
            ),
            "sender_scope": if outgoing { "self" } else { "third_party" },
            "conversation_id": format!("q2-conversation-{peer:03}"),
            "sender_id": if outgoing { "q2-self".to_owned() } else { format!("q2-peer-{peer:03}") },
            "text": format!("Synthetic scale observation {index:05}."),
        });
        text.push_str(&serde_json::to_string(&row).expect("serialize synthetic message"));
        text.push('\n');
    }
    text
}

fn timed<T>(operation: impl FnOnce() -> T) -> (T, u64) {
    let start = Instant::now();
    let value = operation();
    let ns = start
        .elapsed()
        .as_nanos()
        .try_into()
        .expect("duration fits u64");
    (value, ns)
}

struct Observation {
    durations_ns: BTreeMap<&'static str, u64>,
    counts: Value,
}

struct DecomposedObservation {
    durations_ns: BTreeMap<&'static str, u64>,
    counts: Value,
}

/// `total` begins after opening, immediately before preview, and ends when
/// audit returns. It includes the seven calls and their tiny call/clock gaps,
/// but excludes open, input generation, metadata/hash collection, assertions,
/// report serialization, dropping the Session, and temporary-directory cleanup.
fn observe(text: &str, messages: usize, peers: usize) -> Observation {
    let keep = tempfile::tempdir().expect("fresh data directory per observation");
    let directory = std::fs::canonicalize(keep.path()).expect("canonical data directory");
    let (mut session, open) = timed(|| Session::open(&directory));
    let total_start = Instant::now();
    let (preview, preview_ns) = timed(|| session.preview_soul_import_v1(text));
    let (receipt, commit_ns) = timed(|| session.commit_soul_import_v1(text));
    let (people, people_ns) = timed(|| session.people());
    let (profile, profile_ns) = timed(|| session.profile());
    let (memories, memories_ns) = timed(|| session.memories());
    let (research, research_ns) = timed(|| session.research());
    let (audit, audit_ns) = timed(|| session.audit());
    let total = total_start
        .elapsed()
        .as_nanos()
        .try_into()
        .expect("duration fits u64");

    // All correctness work is outside the measured interval. The expectations
    // come from two-way, equal-sized, one-to-one conversations, not product helpers.
    let preview = preview.expect("valid synthetic preview");
    let receipt = receipt.expect("valid synthetic commit");
    let people = people.expect("read graph and resolve its evidence");
    let profile = profile.expect("read profile");
    let memories = memories.expect("read memories");
    let research = research.expect("read research preview");
    let audit = audit.expect("read and verify audit");
    assert_eq!(preview.messages, messages);
    assert_eq!(preview.participants, peers + 1);
    assert_eq!(preview.conversations, peers);
    assert!(preview.owner_identified);
    assert_eq!(preview.messages_with_injection_markers, 0);
    assert!(!preview.writes_anything);
    assert_eq!(receipt.events_written, messages);
    assert_eq!(receipt.evidence_written, messages);
    assert_eq!(receipt.contacts_created, peers + 1);
    assert_eq!(receipt.contacts_matched, 0);
    assert_eq!(receipt.ties_rebuilt, peers);
    assert_eq!(people.people.len(), peers + 1);
    assert_eq!(
        people.people.iter().filter(|person| person.is_you).count(),
        1
    );
    assert_eq!(people.ties.len(), peers);
    assert!(people.third_party_data_is_local_only);
    let per_peer = (messages / peers) as u64;
    for tie in &people.ties {
        assert_eq!(tie.interaction_count, per_peer);
        assert_eq!(tie.outgoing_count, per_peer / 2);
        assert_eq!(tie.incoming_count, per_peer / 2);
        assert_eq!(tie.direct_count, per_peer);
        assert_eq!(tie.group_count, 0);
        assert_eq!(tie.conversation_count, 1);
        assert_eq!(tie.evidence.len(), messages / peers);
        assert!(tie.local_only);
    }
    assert!(profile.stated.is_empty());
    assert!(profile.axes.iter().all(|axis| axis.position == "unknown"));
    assert!(
        memories.memories.is_empty(),
        "an import does not author memories"
    );
    assert_eq!(research.third_party_rows, 0);
    assert!(research.third_party_rows_excluded > 0);
    assert!(research.deny_rows_excluded > 0);
    assert!(!research.written_to_disk);
    assert!(
        research.rows.is_empty(),
        "imported prose is denied for research"
    );
    assert!(audit.verified, "{:?}", audit.verification_problem);
    assert!(audit.entries.iter().all(|entry| entry.follows_previous));
    let import = audit
        .entries
        .iter()
        .find(|entry| entry.action == "import.commit")
        .expect("the import appears in the chain");
    assert_eq!(import.items, Some(messages as u64));
    assert!(audit
        .entries
        .iter()
        .any(|entry| entry.action == "inference.write"));

    Observation {
        durations_ns: BTreeMap::from([
            ("open", open),
            ("preview", preview_ns),
            ("commit_including_rebuild", commit_ns),
            ("people", people_ns),
            ("profile", profile_ns),
            ("memories", memories_ns),
            ("research", research_ns),
            ("audit", audit_ns),
            ("total", total),
        ]),
        counts: json!({
            "expected_messages": messages, "expected_peers": peers,
            "expected_self_messages": messages / 2, "expected_third_party_messages": messages / 2,
            "preview_messages": preview.messages, "preview_participants": preview.participants,
            "preview_conversations": preview.conversations, "preview_writes_anything": preview.writes_anything,
            "events_written": receipt.events_written, "evidence_written": receipt.evidence_written,
            "contacts_created": receipt.contacts_created, "contacts_matched": receipt.contacts_matched,
            "ties_rebuilt": receipt.ties_rebuilt, "people": people.people.len(), "edges": people.ties.len(),
            "profile_axes": profile.axes.len(), "profile_stated": profile.stated.len(),
            "memories": memories.memories.len(), "research_rows": research.rows.len(),
            "research_candidate_rows": research.candidate_rows_total,
            "research_third_party_rows": research.third_party_rows,
            "research_third_party_rows_excluded": research.third_party_rows_excluded,
            "research_deny_rows_excluded": research.deny_rows_excluded,
            "research_written_to_disk": research.written_to_disk,
            "audit_entries": audit.entries.len(), "audit_verified": audit.verified,
        }),
    }
}

/// A diagnostic companion to observe. It uses the same parser, encrypted
/// store, command-layer writes, graph rebuild, and audit append path, but keeps
/// the parsed value outside the transaction so the two expensive inner stages
/// can be timed separately. It is not the Session path: identifier syncing
/// happens after that private transaction and remains represented only by
/// observe's end-to-end commit timing.
fn observe_decomposed(text: &str, messages: usize, peers: usize) -> DecomposedObservation {
    let keep = tempfile::tempdir().expect("fresh data directory per decomposition");
    let database = keep.path().join("soul.db");
    let keys = TestKeyProvider::from_seed(DECOMPOSITION_KEY_SEED);
    let mut store = SqlCipherStore::open(&database, &keys).expect("open encrypted store");
    let (staged, parse_ns) = timed(|| import_commands::read_soul_import_v1(text));
    let staged = staged.expect("valid synthetic import");

    let mut encrypted_import_write_ns = 0;
    let mut graph_rebuild_ns = 0;
    let (transaction, transaction_total_ns) = timed(|| {
        store.transact(|store| -> Result<_, SessionRefusal> {
            let (receipt, elapsed) =
                timed(|| import_commands::commit(store, &staged, DECOMPOSITION_AT_UNIX_SECONDS));
            encrypted_import_write_ns = elapsed;
            let receipt = receipt?;

            let (build, elapsed) =
                timed(|| graph_commands::rebuild(store, DECOMPOSITION_AT_UNIX_SECONDS));
            graph_rebuild_ns = elapsed;
            Ok((receipt, build?))
        })
    });
    let (receipt, build) = transaction.expect("commit and rebuild in one transaction");
    let measured_operations_ns = encrypted_import_write_ns
        .checked_add(graph_rebuild_ns)
        .expect("measured operation durations fit u64");
    let transaction_overhead_residual_ns = transaction_total_ns
        .checked_sub(measured_operations_ns)
        .expect("transaction contains both measured operations");

    assert_eq!(staged.messages.len(), messages);
    assert_eq!(staged.participants.len(), peers + 1);
    assert_eq!(receipt.events_written.len(), messages);
    assert_eq!(receipt.evidence_written.len(), messages);
    assert_eq!(receipt.contacts_created.len(), peers + 1);
    assert!(receipt.contacts_matched.is_empty());
    assert_eq!(build.edges_written.len(), peers);

    DecomposedObservation {
        durations_ns: BTreeMap::from([
            ("parse", parse_ns),
            ("encrypted_import_write", encrypted_import_write_ns),
            ("graph_rebuild", graph_rebuild_ns),
            ("transaction_total", transaction_total_ns),
            (
                "transaction_overhead_residual",
                transaction_overhead_residual_ns,
            ),
        ]),
        counts: json!({
            "expected_messages": messages,
            "expected_peers": peers,
            "events_written": receipt.events_written.len(),
            "evidence_written": receipt.evidence_written.len(),
            "contacts_created": receipt.contacts_created.len(),
            "contacts_matched": receipt.contacts_matched.len(),
            "ties_rebuilt": build.edges_written.len(),
        }),
    }
}

#[test]
fn a_hundred_messages_across_ten_peers_preserve_counts_privacy_and_audit() {
    let text = synthetic_jsonl(100, 10);
    observe(&text, 100, 10);
}

#[test]
fn a_hundred_messages_decomposition_reports_real_transaction_stages() {
    let text = synthetic_jsonl(100, 10);
    let observation = observe_decomposed(&text, 100, 10);

    for stage in [
        "parse",
        "encrypted_import_write",
        "graph_rebuild",
        "transaction_total",
        "transaction_overhead_residual",
    ] {
        assert!(
            observation.durations_ns.contains_key(stage),
            "missing measured stage {stage}"
        );
    }
    assert_eq!(observation.durations_ns.len(), 5);
    assert_eq!(observation.counts["events_written"], 100);
    assert_eq!(observation.counts["evidence_written"], 100);
    assert_eq!(observation.counts["contacts_created"], 11);
    assert_eq!(observation.counts["ties_rebuilt"], 10);

    let write = observation.durations_ns["encrypted_import_write"];
    let rebuild = observation.durations_ns["graph_rebuild"];
    let transaction = observation.durations_ns["transaction_total"];
    assert!(
        transaction >= write + rebuild,
        "transaction time must contain both measured operations"
    );
    assert_eq!(
        observation.durations_ns["transaction_overhead_residual"],
        transaction - write - rebuild
    );
}

/// Run ancillary commands only through pwsh. Nothing this helper does is timed.
/// Fixture bytes go to stdin, never to an argument or a source-controlled file.
fn powershell_json(script: &str, input: &[u8]) -> Value {
    let mut child = Command::new("pwsh")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("pwsh is required for explicit Windows scale measurement");
    child
        .stdin
        .take()
        .expect("stdin pipe")
        .write_all(input)
        .expect("write fixture bytes");
    let output = child
        .wait_with_output()
        .expect("wait for measurement metadata");
    assert!(
        output.status.success(),
        "metadata command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("metadata command emits JSON only")
}

fn source_sha() -> String {
    let sha = std::env::var("SOUL_Q2_SOURCE_SHA")
        .expect("set SOUL_Q2_SOURCE_SHA to the frozen clean 40-hex git HEAD before measurement");
    assert!(
        sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "SOUL_Q2_SOURCE_SHA must be exactly 40 hexadecimal characters"
    );
    sha.to_ascii_lowercase()
}

/// Metadata comes from the actual local host and toolchain, not a hand-entered
/// machine label. Source identity and cleanliness are checked before any trial.
fn measurement_context(sha: &str) -> Value {
    let metadata = powershell_json(
        r#"$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$head = (& git rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0) { throw 'git rev-parse failed' }
$dirty = @(& git status --porcelain --untracked-files=normal)
if ($LASTEXITCODE -ne 0) { throw 'git status failed' }
if ($dirty.Count -ne 0) { throw 'measurement requires a clean worktree' }
$rustc = & rustc --version
if ($LASTEXITCODE -ne 0) { throw 'rustc --version failed' }
$cargo = & cargo --version
if ($LASTEXITCODE -ne 0) { throw 'cargo --version failed' }
$os = Get-CimInstance Win32_OperatingSystem
$cpu = @(Get-CimInstance Win32_Processor | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors)
[ordered]@{
    source_sha = $head
    machine = [ordered]@{ name = [Environment]::MachineName; os = $os.Caption; os_version = $os.Version; architecture = $os.OSArchitecture; total_visible_memory_kib = $os.TotalVisibleMemorySize; cpu = $cpu }
    toolchain = [ordered]@{ rustc = $rustc; cargo = $cargo; pwsh = $PSVersionTable.PSVersion.ToString() }
    build_environment = [ordered]@{ cargo_target_dir = $env:CARGO_TARGET_DIR; cargo_profile_dev_debug = $env:CARGO_PROFILE_DEV_DEBUG; cargo_profile_test_debug = $env:CARGO_PROFILE_TEST_DEBUG; cargo_incremental = $env:CARGO_INCREMENTAL; cargo_build_jobs = $env:CARGO_BUILD_JOBS; rustflags = $env:RUSTFLAGS }
} | ConvertTo-Json -Depth 8 -Compress
"#,
        &[],
    );
    assert_eq!(
        metadata["source_sha"].as_str(),
        Some(sha),
        "source SHA must equal the clean checkout HEAD"
    );
    json!({
        "host": metadata,
        "build_profile": "cargo test --locked; workspace profile.test opt-level=1",
        "debug_assertions": cfg!(debug_assertions),
        "executable": std::env::current_exe().expect("test executable").to_string_lossy(),
        "cache": {
            "database": "fresh independent temporary directory per warmup and trial; one import per database",
            "sequence": "open, preview, commit including rebuild, people, profile, memories, research, audit",
            "os_page_cache": "not flushed; warm/cold uncontrolled; one discarded warmup before five trials per case",
            "build_cache": "excluded from measured durations; actual CARGO_TARGET_DIR recorded",
        },
        "timing_unit": "nanoseconds",
        "total_definition": "wall time from immediately before preview through audit return; excludes open, fixture generation, metadata/hash collection, assertions, JSON output, Session drop, and temporary directory cleanup",
        "open_definition": "Session::open only; tempdir creation and canonicalization excluded",
        "scope": "synthetic import/read-only screens only; profile and memories unpopulated; no real data, endpoint, repeated import, UI, installer, or Linux measurement",
    })
}

fn fixture_sha256(text: &str) -> String {
    let value = powershell_json(
        r#"$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$inputStream = [Console]::OpenStandardInput()
$hash = [Security.Cryptography.SHA256]::HashData($inputStream)
[Convert]::ToHexString($hash).ToLowerInvariant() | ConvertTo-Json -Compress
"#,
        text.as_bytes(),
    );
    let hash = value.as_str().expect("SHA-256 string");
    assert!(hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    hash.to_owned()
}

fn emit(report: &mut std::fs::File, record: Value) {
    let line = serde_json::to_string(&record).expect("serialize measurement record");
    writeln!(report, "{line}").expect("write pure JSONL report");
    report
        .flush()
        .expect("flush each record for partial-run diagnostics");
    // The initial newline keeps a JSON value separate from libtest's prefix.
    println!("\n{line}");
}

/// Explicit only: run after this file is committed, with the same clean SHA
/// for every case. The JSONL file is independent of libtest's console wrapper.
#[test]
#[ignore = "Q2-03 explicit synthetic measurement: requires frozen clean SHA and Windows pwsh"]
fn measure_four_synthetic_scales_with_five_independent_trials() {
    let sha = source_sha(); // Missing or malformed identity refuses before any database opens.
    let context = measurement_context(&sha);
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/q2");
    std::fs::create_dir_all(&output_dir).expect("measurement report directory");
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let report_path = output_dir.join(format!("q2-scale-{run_id}.jsonl"));
    let mut report = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&report_path)
        .expect("new measurement report without overwriting a prior run");
    emit(
        &mut report,
        json!({
            "record": "run", "schema": "soul-q2-scale-v1", "source_sha": sha,
            "report_path": report_path.to_string_lossy(), "context": context,
            "cases": [[1000, 10], [1000, 100], [10000, 10], [10000, 100]],
            "warmups_per_case": 1, "trials_per_case": 5,
        }),
    );
    for (messages, peers) in [(1000, 10), (1000, 100), (10000, 10), (10000, 100)] {
        let text = synthetic_jsonl(messages, peers);
        let hash = fixture_sha256(&text);
        let mut trials = Vec::with_capacity(5);
        for trial in 0..=5 {
            let observation = observe(&text, messages, peers);
            emit(
                &mut report,
                json!({
                    "record": "sample", "source_sha": sha, "fixture_sha256": hash,
                    "messages": messages, "peers": peers, "fixture_bytes": text.len(),
                    "warmup": trial == 0, "trial": trial,
                    "durations_ns": observation.durations_ns, "counts": observation.counts,
                }),
            );
            if trial != 0 {
                trials.push(observation.durations_ns);
            }
        }
        let mut summary = BTreeMap::new();
        for stage in STAGES {
            let mut values: Vec<u64> = trials.iter().map(|trial| trial[stage]).collect();
            values.sort_unstable();
            summary.insert(
                stage,
                json!({ "min": values[0], "median": values[2], "max": values[4] }),
            );
        }
        emit(
            &mut report,
            json!({
                "record": "summary", "source_sha": sha, "fixture_sha256": hash,
                "messages": messages, "peers": peers, "sample_count": 5,
                "warmup_excluded": true, "summary_ns": summary,
            }),
        );
    }
    emit(
        &mut report,
        json!({ "record": "complete", "source_sha": sha, "cases": 4, "warmups": 4, "trials": 20 }),
    );
}
