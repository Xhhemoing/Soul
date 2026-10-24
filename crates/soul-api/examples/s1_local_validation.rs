//! Explicit isolated S1 host; no platform directory, external model or collector.
//! The summary deliberately cannot serialize input, identifiers or credentials.

use serde::Serialize;
use serde_json::{json, Value};
use soul_api::ApiServer;
use soul_runtime::{Caller, Runtime, RuntimeContext};
use soul_types::PeoplePageRequest;
use soulcore::commands::memory::NewMemory;
use soulcore::commands::session::{
    ContextMemoryId, ImportPrepareRequest, MemoryContextRequest, Session,
};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

type Checked<T> = Result<T, &'static str>;
const ENGINEERING_PASTE: &str = "Synthetic S1 routing check; no sender association is asserted.";

struct Args {
    input: Option<PathBuf>,
    root: PathBuf,
    report: PathBuf,
    synthetic: bool,
}

impl Args {
    fn parse(values: impl IntoIterator<Item = OsString>) -> Checked<Self> {
        let mut values = values.into_iter();
        let (mut input, mut root, mut report, mut synthetic) = (None, None, None, false);
        while let Some(flag) = values.next() {
            match flag.to_str() {
                Some("--input") if input.is_none() => {
                    input = Some(PathBuf::from(values.next().ok_or("CLI_VALUE")?))
                }
                Some("--root") if root.is_none() => {
                    root = Some(PathBuf::from(values.next().ok_or("CLI_VALUE")?))
                }
                Some("--report") if report.is_none() => {
                    report = Some(PathBuf::from(values.next().ok_or("CLI_VALUE")?))
                }
                Some("--synthetic") if !synthetic => synthetic = true,
                _ => return Err("CLI_ARGUMENT"),
            }
        }
        let result = Self {
            input,
            root: root.ok_or("CLI_ROOT")?,
            report: report.ok_or("CLI_REPORT")?,
            synthetic,
        };
        if result.synthetic == result.input.is_some() {
            return Err("CLI_INPUT_MODE");
        }
        if !clean_absolute(&result.root)
            || !clean_absolute(&result.report)
            || result.report.parent() != Some(result.root.as_path())
            || result.report.file_name().is_none()
            || result.input.as_ref().is_some_and(|p| !clean_absolute(p))
        {
            return Err("CLI_ABSOLUTE_PATHS");
        }
        Ok(result)
    }
}

fn clean_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
}

struct Input {
    messages: u64,
    owner_rows: u64,
    owners: u64,
    peers: u64,
    participants: u64,
    conversations: u64,
    inferred_group_conversations: u64,
    unique_messages: u64,
    owner_sample: String,
    peer_sample: String,
}

fn field<'a>(value: &'a Value, key: &str) -> Checked<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("INPUT_FIELD")
}

fn inspect_input(text: &str) -> Checked<Input> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header: Value =
        serde_json::from_str(lines.next().ok_or("INPUT_EMPTY")?).map_err(|_| "INPUT_JSON")?;
    if header["type"] != "header" || header["format"] != "soul-import-v1" || header["version"] != 1
    {
        return Err("INPUT_HEADER");
    }
    let mut owners = BTreeSet::new();
    let mut senders = BTreeSet::new();
    let mut conversations: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut keys = BTreeSet::new();
    let (mut messages, mut owner_rows) = (0, 0);
    let (mut owner_sample, mut peer_sample) = (None, None);
    for line in lines {
        let row: Value = serde_json::from_str(line).map_err(|_| "INPUT_JSON")?;
        if row["type"] != "message" {
            return Err("INPUT_ROW_TYPE");
        }
        let sender = field(&row, "sender_id")?;
        let conversation = field(&row, "conversation_id")?;
        let id = field(&row, "id")?;
        let body = row
            .get("text")
            .and_then(Value::as_str)
            .ok_or("INPUT_FIELD")?;
        field(&row, "occurred_at")?;
        match field(&row, "sender_scope")? {
            "self" => {
                owners.insert(sender.to_owned());
                owner_rows += 1;
                if owner_sample.is_none() && !body.trim().is_empty() && body.len() <= 4096 {
                    owner_sample = Some(body.to_owned());
                }
            }
            "third_party" => {
                if peer_sample.is_none() && !body.trim().is_empty() {
                    peer_sample = Some(body.to_owned());
                }
            }
            _ => return Err("INPUT_SCOPE"),
        }
        senders.insert(sender.to_owned());
        conversations
            .entry(conversation.to_owned())
            .or_default()
            .insert(sender.to_owned());
        keys.insert((conversation.to_owned(), id.to_owned()));
        messages += 1;
    }
    if owners.is_empty() {
        return Err("INPUT_OWNER_MISSING");
    }
    let peers = senders.difference(&owners).count() as u64;
    if peers == 0 {
        return Err("INPUT_PEER_MISSING");
    }
    let group_count = conversations
        .values()
        .filter(|members| members.difference(&owners).count() > 1)
        .count();
    Ok(Input {
        messages,
        owner_rows,
        owners: owners.len() as u64,
        peers,
        participants: peers + 1,
        conversations: conversations.len() as u64,
        inferred_group_conversations: group_count as u64,
        unique_messages: keys.len() as u64,
        owner_sample: owner_sample.ok_or("INPUT_OWNER_SAMPLE_UNAVAILABLE")?,
        peer_sample: peer_sample.ok_or("INPUT_PEER_SAMPLE_UNAVAILABLE")?,
    })
}

#[derive(Serialize)]
struct Report {
    status: &'static str,
    stage: &'static str,
    code: &'static str,
    mode: &'static str,
    messages: Option<u64>,
    owner_rows: Option<u64>,
    owner_identifiers: Option<u64>,
    peers: Option<u64>,
    participants: Option<u64>,
    conversations: Option<u64>,
    inferred_group_conversations: Option<u64>,
    duplicates: Option<u64>,
    events_written: Option<u64>,
    evidence_written: Option<u64>,
    import_transport: &'static str,
    real_material_transport: &'static str,
    http_scoped_input: &'static str,
    voice_setting: &'static str,
    core_template: Option<bool>,
    core_memory_source: Option<bool>,
    core_memory_cancel: Option<bool>,
    core_context_one_use: Option<bool>,
    ordinary_t1_cancel: &'static str,
    session_reopened: Option<bool>,
    receipt_persisted: Option<bool>,
    people_persisted: Option<bool>,
    http_status: Option<bool>,
    http_people: Option<bool>,
    http_scoped_template: Option<bool>,
    http_proposal_discard: Option<bool>,
    proposal_confirmation_rejected_by_core: Option<bool>,
    http_unauthorized_rejected: Option<bool>,
    http_foreign_origin_rejected: Option<bool>,
    http_import_rejected: Option<bool>,
    http_unneeded_write_rejected: Option<bool>,
    unauthorized_runtime_admissions: Option<u64>,
    egress_audit_entries: Option<u64>,
    capabilities_closed: Option<bool>,
    native_ui_validation: bool,
    human_utility_evaluation: bool,
}

impl Report {
    fn new(synthetic: bool) -> Self {
        Self {
            status: "incomplete",
            stage: "initialize",
            code: "NOT_FINISHED",
            mode: if synthetic {
                "synthetic_fixture"
            } else {
                "authorized_real_file"
            },
            messages: None,
            owner_rows: None,
            owner_identifiers: None,
            peers: None,
            participants: None,
            conversations: None,
            inferred_group_conversations: None,
            duplicates: None,
            events_written: None,
            evidence_written: None,
            import_transport: "trusted_production_runtime_prepare_confirm",
            real_material_transport: "trusted_core_local_t1_and_explicit_owner_memory_excerpt",
            http_scoped_input: "synthetic_engineering_text_with_imported_contact_not_sender_mapped",
            voice_setting: "plain_neutral_validation_setting_not_inferred_preference",
            core_template: None,
            core_memory_source: None,
            core_memory_cancel: None,
            core_context_one_use: None,
            ordinary_t1_cancel: "no_endpoint_preparation_created_no_t3_cancel_claim",
            session_reopened: None,
            receipt_persisted: None,
            people_persisted: None,
            http_status: None,
            http_people: None,
            http_scoped_template: None,
            http_proposal_discard: None,
            proposal_confirmation_rejected_by_core: None,
            http_unauthorized_rejected: None,
            http_foreign_origin_rejected: None,
            http_import_rejected: None,
            http_unneeded_write_rejected: None,
            unauthorized_runtime_admissions: None,
            egress_audit_entries: None,
            capabilities_closed: None,
            native_ui_validation: false,
            human_utility_evaluation: false,
        }
    }
}

fn require(condition: bool, code: &'static str) -> Checked<()> {
    if condition {
        Ok(())
    } else {
        Err(code)
    }
}

fn record_check(result: &mut Option<bool>, condition: bool, code: &'static str) -> Checked<()> {
    *result = Some(condition);
    require(condition, code)
}

fn closed(session: &Session) -> Checked<()> {
    let config = session.snapshot();
    let collection = session.collect_status();
    require(
        config.fully_closed
            && !config.llm_endpoint_configured
            && !config.collect_enabled
            && !collection.consent_granted
            && !collection.collector_running,
        "CAPABILITIES_NOT_CLOSED",
    )
}

fn check_closed(session: &Session, report: &mut Report) -> Checked<()> {
    let result = closed(session);
    report.capabilities_closed = Some(result.is_ok());
    result
}

fn people_ids(runtime: &Runtime) -> Checked<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    let mut offset = 0;
    loop {
        let page = runtime
            .people(PeoplePageRequest { limit: 100, offset })
            .map_err(|_| "PEOPLE_READ")?;
        let received = page.people.len();
        for person in page.people {
            result.insert(person.contact_id);
        }
        if !page.has_more {
            break;
        }
        require(received > 0, "PEOPLE_PAGINATION")?;
        offset += received as u32;
    }
    Ok(result)
}

fn template(session: &mut Session, pasted: &str) -> Checked<()> {
    let drafted = session.draft_pasted(pasted).map_err(|_| "CORE_DRAFT")?;
    require(
        !drafted.text.trim().is_empty()
            && serde_json::to_value(&drafted.delivery).map_err(|_| "DRAFT_DELIVERY")? == false
            && serde_json::to_value(&drafted.source).map_err(|_| "DRAFT_SOURCE")?
                == "tone_template",
        "NOT_LOCAL_TEMPLATE",
    )
}

fn core_material(runtime: &Runtime, input: &Input, report: &mut Report) -> Checked<String> {
    let mut session = runtime.held_session();
    check_closed(&session, report)?;
    session
        .set_voice("register", "plain")
        .map_err(|_| "VOICE_SETUP")?;
    let result = template(&mut session, &input.peer_sample);
    report.core_template = Some(result.is_ok());
    result?;
    // Explicit validation excerpt, not an inferred or confirmed preference.
    let memory = session
        .write_memory(&NewMemory {
            memory_type: "episodic".into(),
            title: "S1 validation owner source excerpt".into(),
            summary: input.owner_sample.clone(),
        })
        .map_err(|_| "MEMORY_SOURCE_WRITE")?;
    let id = ContextMemoryId::parse_str(&memory.memory_id).map_err(|_| "MEMORY_ID")?;
    let request = || MemoryContextRequest {
        candidate_ids: vec![id],
        query: None,
        memory_type: None,
        time_specific: false,
    };
    let selected = session
        .prepare_memory_context(request())
        .map_err(|_| "MEMORY_SELECT")?;
    let context = selected
        .context_id
        .ok_or("MEMORY_CONTEXT_UNAVAILABLE")?
        .to_string();
    let drafted = session
        .draft_pasted_with_memory(&input.peer_sample, &context)
        .map_err(|_| "MEMORY_LOCAL_DRAFT")?;
    record_check(
        &mut report.core_memory_source,
        drafted
            .context
            .selected
            .iter()
            .any(|entry| entry.citation.memory_id == id)
            && serde_json::to_value(&drafted.draft.delivery).map_err(|_| "DRAFT_DELIVERY")?
                == false
            && serde_json::to_value(&drafted.draft.source).map_err(|_| "DRAFT_SOURCE")?
                == "tone_template",
        "MEMORY_SOURCE_NOT_SELECTED",
    )?;
    record_check(
        &mut report.core_context_one_use,
        session
            .draft_pasted_with_memory(&input.peer_sample, &context)
            .is_err(),
        "MEMORY_CONTEXT_REPLAY",
    )?;
    let pending = session
        .prepare_memory_context(request())
        .map_err(|_| "MEMORY_SELECT_CANCEL")?
        .context_id
        .ok_or("MEMORY_CONTEXT_UNAVAILABLE")?
        .to_string();
    session.discard_draft();
    record_check(
        &mut report.core_memory_cancel,
        session
            .draft_pasted_with_memory(&input.peer_sample, &pending)
            .is_err(),
        "MEMORY_CANCEL_NOT_INVALIDATED",
    )?;
    check_closed(&session, report)?;
    Ok(memory.memory_id)
}

struct Response {
    status: u16,
    value: Value,
}

fn request(
    address: SocketAddr,
    auth: Option<&str>,
    origin: Option<&str>,
    method: &str,
    path: &str,
    body: &Value,
) -> Checked<Response> {
    require(address.ip().is_loopback(), "HTTP_NOT_LOOPBACK")?;
    let body = if method == "GET" {
        String::new()
    } else {
        serde_json::to_string(body).map_err(|_| "HTTP_ENCODING")?
    };
    let mut stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(5)).map_err(|_| "HTTP_CONNECT")?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|_| "HTTP_TIMEOUT_SETUP")?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| "HTTP_TIMEOUT_SETUP")?;
    let authorization = auth
        .map(|v| format!("Authorization: {v}\r\n"))
        .unwrap_or_default();
    let origin = origin
        .map(|v| format!("Origin: {v}\r\n"))
        .unwrap_or_default();
    write!(stream, "{method} {path} HTTP/1.1\r\nHost: {address}\r\n{authorization}{origin}Connection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).map_err(|_| "HTTP_WRITE")?;
    let mut bytes = Vec::new();
    (&mut stream)
        .take((soul_api::MAX_RESPONSE_BYTES + soul_api::MAX_HEADER_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "HTTP_READ")?;
    require(
        bytes.len() <= soul_api::MAX_RESPONSE_BYTES + soul_api::MAX_HEADER_BYTES,
        "HTTP_RESPONSE_BOUND",
    )?;
    let response = std::str::from_utf8(&bytes).map_err(|_| "HTTP_UTF8")?;
    let (head, body) = response.split_once("\r\n\r\n").ok_or("HTTP_FRAME")?;
    let status = head
        .lines()
        .next()
        .and_then(|v| v.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .ok_or("HTTP_STATUS")?;
    let value = serde_json::from_str(body).map_err(|_| "HTTP_JSON")?;
    Ok(Response { status, value })
}

fn http_checks(
    runtime: &Runtime,
    memory_id: &str,
    expected_people: usize,
    report: &mut Report,
) -> Checked<()> {
    let server = ApiServer::start_s1_validation(runtime.clone()).map_err(|_| "HTTP_HOST_START")?;
    let result = (|| {
        let address = server.address();
        let authorization = server.authorization();
        let auth = Some(authorization.as_str());
        let call =
            |method, path: &str, body: &Value| request(address, auth, None, method, path, body);
        record_check(
            &mut report.http_status,
            call("GET", "/v1/status", &Value::Null)?.value["store_opened"] == true,
            "HTTP_STATUS_READ",
        )?;
        let mut offset = 0;
        let mut person = None;
        loop {
            let page = call(
                "GET",
                &format!("/v1/people?limit=100&offset={offset}"),
                &Value::Null,
            )?;
            if page.status != 200 || page.value["total"].as_u64() != Some(expected_people as u64) {
                report.http_people = Some(false);
                return Err("HTTP_PEOPLE_COUNT");
            }
            let rows = page.value["people"].as_array().ok_or("HTTP_PEOPLE_SHAPE")?;
            if person.is_none() {
                person = rows
                    .iter()
                    .find(|p| p["is_you"] == false && p["forgotten"] == false)
                    .and_then(|p| p["contact_id"].as_str())
                    .map(str::to_owned);
            }
            offset += rows.len();
            if page.value["has_more"] == false {
                break;
            }
            require(!rows.is_empty(), "HTTP_PEOPLE_PAGINATION")?;
        }
        record_check(
            &mut report.http_people,
            offset == expected_people,
            "HTTP_PEOPLE_PAGINATION",
        )?;
        let person = person.ok_or("HTTP_NO_ENGINEERING_CONTACT")?;
        let scope_path = format!("/v1/scoped/status?person_id={person}&occasion=personal");
        let before = call("GET", &scope_path, &Value::Null)?;
        require(
            before.status == 200 && before.value["state"] == "absent",
            "SCOPE_NOT_ABSENT",
        )?;
        let drafted = call(
            "POST",
            "/v1/scoped/draft",
            &json!({"pasted":ENGINEERING_PASTE,"person_id":person,"occasion":"personal"}),
        )?;
        record_check(
            &mut report.http_scoped_template,
            drafted.status == 200
                && drafted.value["draft"]["text"]
                    .as_str()
                    .is_some_and(|v| !v.is_empty())
                && drafted.value["applied_preference"].is_null(),
            "HTTP_SCOPED_DRAFT",
        )?;
        // This route is the production local-only T1 method, not an endpoint route.
        let result_id = drafted.value["result_id"]
            .as_str()
            .ok_or("HTTP_SCOPED_SHAPE")?;
        let proposal = call(
            "POST",
            "/v1/scoped/propose",
            &json!({"result_id":result_id,"edit_revision":1,"register":"plain"}),
        )?;
        require(
            proposal.status == 200 && proposal.value["expected_revision"].is_null(),
            "HTTP_PROPOSAL",
        )?;
        let id = proposal.value["proposal_id"]
            .as_str()
            .ok_or("HTTP_PROPOSAL_SHAPE")?;
        record_check(
            &mut report.http_proposal_discard,
            call("POST", &format!("/v1/scoped/{id}/discard"), &json!({}))?.status == 200,
            "HTTP_PROPOSAL_DISCARD",
        )?;
        // HTTP confirm is intentionally outside this host. Verify the discarded
        // core slot itself with the same HTTP caller rather than mislabeling a 403.
        let ctx = RuntimeContext::new(Caller::http(), None);
        record_check(
            &mut report.proposal_confirmation_rejected_by_core,
            runtime.confirm_scoped_register(&ctx, id, None).is_err(),
            "CORE_DISCARDED_PROPOSAL_CONFIRMED",
        )?;
        let after = call("GET", &scope_path, &Value::Null)?;
        require(
            after.status == 200 && after.value["state"] == "absent",
            "CANCEL_SAVED_PREFERENCE",
        )?;
        let sequence = runtime.event_snapshot().sequence;
        record_check(
            &mut report.http_unauthorized_rejected,
            request(address, None, None, "GET", "/v1/status", &Value::Null)?.status == 401,
            "HTTP_UNAUTHENTICATED_ACCEPTED",
        )?;
        record_check(
            &mut report.http_foreign_origin_rejected,
            request(
                address,
                auth,
                Some("https://foreign.invalid"),
                "GET",
                "/v1/status",
                &Value::Null,
            )?
            .status
                == 403,
            "HTTP_FOREIGN_ORIGIN_ACCEPTED",
        )?;
        record_check(
            &mut report.http_import_rejected,
            call(
                "POST",
                "/v1/imports/prepare",
                &json!({"fixture_id":"basic"}),
            )?
            .status
                == 404,
            "HTTP_IMPORT_NOT_DENIED",
        )?;
        record_check(
            &mut report.http_unneeded_write_rejected,
            call(
                "PATCH",
                &format!("/v1/memories/{memory_id}"),
                &json!({"summary":"Synthetic forbidden write"}),
            )?
            .status
                == 404,
            "HTTP_UNNEEDED_WRITE_NOT_DENIED",
        )?;
        let admissions = runtime.event_snapshot().sequence.saturating_sub(sequence);
        report.unauthorized_runtime_admissions = Some(admissions);
        require(admissions == 0, "DENIED_REQUEST_ADMITTED")?;
        require(
            runtime.event_snapshot().active_operations.is_empty(),
            "HTTP_UNSETTLED_OPERATION",
        )?;
        Ok(())
    })();
    server.shutdown();
    result
}

fn run(args: &Args, root: &Path, report: &mut Report) -> Checked<()> {
    report.stage = "input";
    let text = if args.synthetic {
        include_str!("../../../fixtures/import/soul-import-v1/valid_basic.jsonl").to_owned()
    } else {
        fs::read_to_string(args.input.as_ref().ok_or("CLI_INPUT")?).map_err(|_| "INPUT_READ")?
    };
    let input = inspect_input(&text)?;
    report.messages = Some(input.messages);
    report.owner_rows = Some(input.owner_rows);
    report.owner_identifiers = Some(input.owners);
    report.peers = Some(input.peers);
    report.participants = Some(input.participants);
    report.conversations = Some(input.conversations);
    report.inferred_group_conversations = Some(input.inferred_group_conversations);
    report.duplicates = Some(input.messages - input.unique_messages);
    report.stage = "session_open";
    if !cfg!(windows) && !args.synthetic {
        return Err("WINDOWS_DPAPI_REQUIRED");
    }
    let data = root.join("data");
    let session = Session::open(&data);
    require(session.status().store_opened, "STORE_NOT_OPENED")?;
    if cfg!(windows) {
        require(
            session.status().key_protection == "dpapi",
            "DPAPI_NOT_ACTIVE",
        )?;
    }
    check_closed(&session, report)?;
    let runtime = Runtime::new(session);
    require(!runtime.is_synthetic(), "RUNTIME_MODE")?;
    report.stage = "import_prepare";
    let import: ImportPrepareRequest = serde_json::from_value(
        json!({"text":text,"source":"soul-import-v1","namespace_id":null,"legacy_choice":"reject"}),
    )
    .map_err(|_| "IMPORT_REQUEST")?;
    let ctx = RuntimeContext::new(Caller::desktop("s1-validation"), None);
    let prepared = runtime
        .prepare_import(&ctx, import)
        .map_err(|_| "IMPORT_PREPARE")?;
    require(
        prepared.preview.owner_identified
            && !prepared.preview.writes_anything
            && prepared.preview.messages as u64 == input.messages
            && prepared.preview.participants as u64 == input.participants
            && prepared.preview.conversations as u64 == input.conversations
            && prepared.operation.preview.duplicates == input.messages - input.unique_messages
            && prepared.operation.preview.events_written == input.unique_messages,
        "IMPORT_PREVIEW_COUNTS",
    )?;
    report.stage = "import_confirm";
    let operation = prepared.operation.operation_id.to_string();
    let preview = prepared.operation.preview_id.to_string();
    let committed = runtime
        .confirm_import(&ctx, &operation, &preview)
        .map_err(|_| "IMPORT_CONFIRM")?;
    let receipt = committed
        .operation
        .receipt
        .as_ref()
        .ok_or("IMPORT_RECEIPT_MISSING")?;
    report.events_written = Some(receipt.events_written);
    report.evidence_written = Some(receipt.evidence_written);
    require(
        receipt.messages == input.messages
            && receipt.participants == input.participants
            && receipt.duplicates == input.messages - input.unique_messages
            && receipt.events_written == input.unique_messages
            && receipt.contacts_created == input.participants
            && receipt.contacts_matched == 0,
        "IMPORT_RECEIPT_COUNTS",
    )?;
    require(
        serde_json::to_value(committed.operation.state).map_err(|_| "IMPORT_STATE")? == "committed",
        "IMPORT_NOT_COMMITTED",
    )?;
    let persisted_receipt =
        serde_json::to_value(&committed.operation).map_err(|_| "RECEIPT_ENCODING")?;
    let people = people_ids(&runtime)?;
    require(
        people.len() as u64 == input.participants,
        "IMPORTED_PEOPLE_COUNT",
    )?;
    report.stage = "trusted_core_material";
    let memory_id = core_material(&runtime, &input, report)?;
    drop(runtime);
    report.stage = "session_reopen";
    let reopened = Session::open(&data);
    record_check(
        &mut report.session_reopened,
        reopened.status().store_opened,
        "REOPEN_STORE",
    )?;
    check_closed(&reopened, report)?;
    let runtime = Runtime::new(reopened);
    let recovered = runtime
        .import_operation(&operation)
        .map_err(|_| "REOPEN_RECEIPT")?;
    record_check(
        &mut report.receipt_persisted,
        serde_json::to_value(&recovered.operation).map_err(|_| "RECEIPT_ENCODING")?
            == persisted_receipt,
        "RECEIPT_NOT_PERSISTED",
    )?;
    record_check(
        &mut report.people_persisted,
        people_ids(&runtime)? == people,
        "PEOPLE_NOT_PERSISTED",
    )?;
    report.stage = "loopback_http";
    http_checks(&runtime, &memory_id, people.len(), report)?;
    report.stage = "final_safety";
    let session = runtime.held_session();
    check_closed(&session, report)?;
    let egress = session
        .audit()
        .map_err(|_| "AUDIT_READ")?
        .entries
        .iter()
        .filter(|e| e.action == "egress.request")
        .count() as u64;
    report.egress_audit_entries = Some(egress);
    require(egress == 0, "UNEXPECTED_EGRESS_AUDIT")?;
    Ok(())
}

fn prepare_output(args: &Args) -> Checked<(PathBuf, File)> {
    if let Some(input) = &args.input {
        require(
            fs::metadata(input).map_err(|_| "INPUT_NOT_FILE")?.is_file(),
            "INPUT_NOT_FILE",
        )?;
    }
    // A fresh directory is required, including rejecting pre-existing empty
    // directories. Never reopen/overwrite someone else's candidate or database.
    fs::create_dir(&args.root).map_err(|_| "ROOT_NOT_NEW")?;
    let root = fs::canonicalize(&args.root).map_err(|_| "ROOT_RESOLVE")?;
    let name = args.report.file_name().ok_or("CLI_REPORT")?;
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(root.join(name))
        .map_err(|_| "REPORT_CREATE")?;
    Ok((root, file))
}

fn save_report(file: &mut File, report: &Report) -> Checked<()> {
    file.seek(SeekFrom::Start(0)).map_err(|_| "REPORT_WRITE")?;
    file.set_len(0).map_err(|_| "REPORT_WRITE")?;
    serde_json::to_writer_pretty(&mut *file, report).map_err(|_| "REPORT_WRITE")?;
    file.write_all(b"\n")
        .and_then(|_| file.sync_all())
        .map_err(|_| "REPORT_WRITE")
}

fn safe_panic_location(file: &str, line: u32, column: u32) -> String {
    let normalized = file.replace('\\', "/");
    let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
    let relative = if normalized.starts_with("crates/") {
        Some(normalized.as_str())
    } else {
        manifest.strip_suffix("/crates/soul-api").and_then(|root| {
            normalized
                .strip_prefix(root)
                .and_then(|path| path.strip_prefix('/'))
        })
    };
    let allowed = relative.filter(|path| {
        let known_crate = [
            "soul-api",
            "soul-runtime",
            "soulcore",
            "soul-import",
            "soul-store",
            "soul-store-api",
            "soul-policy",
            "soul-schema",
            "soul-types",
            "soul-draft",
            "soul-memory",
            "soul-graph",
            "soul-profile",
        ]
        .iter()
        .any(|name| path.starts_with(format!("crates/{name}/src/").as_str()));
        (known_crate || *path == "crates/soul-api/examples/s1_local_validation.rs")
            && path.ends_with(".rs")
            && path
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
            && path
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"/_-.".contains(&byte))
    });
    match allowed {
        Some(path) => format!("S1_VALIDATION_PANIC file={path} line={line} column={column}"),
        None => "S1_VALIDATION_PANIC file=Unknown line=0 column=0".to_owned(),
    }
}

fn safe_panic_metadata(info: &std::panic::PanicHookInfo<'_>) -> String {
    match info.location() {
        Some(location) => safe_panic_location(location.file(), location.line(), location.column()),
        None => "S1_VALIDATION_PANIC file=Unknown line=0 column=0".to_owned(),
    }
}

fn main() {
    std::panic::set_hook(Box::new(|info| eprintln!("{}", safe_panic_metadata(info))));
    let args = match Args::parse(std::env::args_os().skip(1)) {
        Ok(v) => v,
        Err(code) => {
            eprintln!("S1_VALIDATION_FAILED {code}");
            std::process::exit(1);
        }
    };
    let (root, mut file) = match prepare_output(&args) {
        Ok(v) => v,
        Err(code) => {
            eprintln!("S1_VALIDATION_FAILED {code}");
            std::process::exit(1);
        }
    };
    let mut report = Report::new(args.synthetic);
    if save_report(&mut file, &report).is_err() {
        eprintln!("S1_VALIDATION_FAILED REPORT_WRITE");
        std::process::exit(1);
    }
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&args, &root, &mut report)
    }));
    let code = match outcome {
        Ok(Ok(())) => "VALIDATED",
        Ok(Err(code)) => code,
        Err(_) => "PANIC",
    };
    report.code = code;
    report.status = if code == "VALIDATED" {
        "passed"
    } else {
        "failed"
    };
    if save_report(&mut file, &report).is_err() {
        eprintln!("S1_VALIDATION_FAILED REPORT_WRITE");
        std::process::exit(1);
    }
    if code == "VALIDATED" {
        println!("S1_VALIDATION_PASSED");
    } else {
        eprintln!("S1_VALIDATION_FAILED {code}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_location_accepts_only_repository_source_paths() {
        let expected =
            "S1_VALIDATION_PANIC file=crates/soul-policy/src/injection.rs line=204 column=26";
        assert_eq!(
            safe_panic_location("crates/soul-policy/src/injection.rs", 204, 26),
            expected
        );
        let manifest = env!("CARGO_MANIFEST_DIR").replace('\\', "/");
        let root = manifest.strip_suffix("/crates/soul-api").unwrap();
        assert_eq!(
            safe_panic_location(
                &format!("{root}/crates/soul-policy/src/injection.rs"),
                204,
                26
            ),
            expected
        );
        assert_eq!(
            safe_panic_location(
                "C:/Users/SYNTHETIC_PRIVATE_PATH/crates/soul-import/src/parse.rs",
                123,
                4
            ),
            "S1_VALIDATION_PANIC file=Unknown line=0 column=0"
        );
        for file in [
            "crates/soul-policy/src/../private.rs",
            "crates/unknown/src/private.rs",
            "crates/soul-policy/src/SYNTHETIC_PRIVATE_PAYLOAD\n.rs",
        ] {
            assert_eq!(
                safe_panic_location(file, 1, 2),
                "S1_VALIDATION_PANIC file=Unknown line=0 column=0"
            );
        }
    }

    #[test]
    fn fixture_counts_and_exact_source_rows_are_derived_from_input() {
        let text = include_str!("../../../fixtures/import/soul-import-v1/valid_basic.jsonl");
        let input = inspect_input(text).unwrap();
        assert_eq!(
            (
                input.messages,
                input.owner_rows,
                input.owners,
                input.peers,
                input.participants
            ),
            (5, 2, 1, 2, 3)
        );
        assert_eq!(
            (
                input.conversations,
                input.inferred_group_conversations,
                input.unique_messages
            ),
            (2, 0, 5)
        );
        assert_eq!(input.owner_sample, "明天上午十点在公司门口见");
        assert_eq!(input.peer_sample, "好的没问题");
    }

    #[test]
    fn real_mode_requires_an_explicit_input_and_report_inside_new_root() {
        assert!(Args::parse(
            [
                "--root",
                "relative",
                "--report",
                "relative/report.json",
                "--synthetic"
            ]
            .map(OsString::from)
        )
        .is_err());
        let root = std::env::temp_dir().join("s1-validation-cli-only");
        let report = root.join("report.json");
        let args = vec![
            "--root".into(),
            root.into_os_string(),
            "--report".into(),
            report.into_os_string(),
        ];
        assert!(Args::parse(args).is_err());
    }

    #[test]
    fn summary_schema_cannot_include_source_text_or_identifiers() {
        let value = serde_json::to_value(Report::new(true)).unwrap();
        assert!(value
            .as_object()
            .unwrap()
            .values()
            .all(|v| v.is_null() || v.is_boolean() || v.is_u64() || v.is_string()));
        for key in [
            "text",
            "body",
            "token",
            "contact_id",
            "memory_id",
            "input",
            "root",
        ] {
            assert!(value.get(key).is_none());
        }
    }

    #[test]
    fn unmeasured_results_are_null_and_measured_zero_or_failure_are_preserved() {
        let mut report = Report::new(true);
        let initial = serde_json::to_value(&report).unwrap();
        for (name, value) in initial.as_object().unwrap() {
            if name == "native_ui_validation" || name == "human_utility_evaluation" {
                assert_eq!(value, &Value::Bool(false));
            } else if !value.is_string() {
                assert!(value.is_null());
            }
        }
        report.messages = Some(0);
        report.events_written = Some(7);
        assert!(record_check(&mut report.core_template, true, "CHECK").is_ok());
        assert!(record_check(&mut report.core_memory_cancel, false, "CHECK").is_err());
        let measured = serde_json::to_value(&report).unwrap();
        assert_eq!(measured["messages"], 0);
        assert_eq!(measured["events_written"], 7);
        assert_eq!(measured["core_template"], true);
        assert_eq!(measured["core_memory_cancel"], false);
        assert!(measured["egress_audit_entries"].is_null());
        assert!(measured["http_status"].is_null());
    }
}
