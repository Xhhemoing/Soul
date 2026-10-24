#![cfg(feature = "local-validation")]

use serde_json::{json, Value};
use soul_api::ApiServer;
use soul_runtime::{Caller, Runtime, RuntimeContext};
use soul_types::{BuiltinFixtureId, SyntheticImportRequest};
use soulcore::commands::session::Session;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

fn fixture() -> (tempfile::TempDir, Runtime, String) {
    let directory = tempfile::tempdir().unwrap();
    let mut session = Session::open(directory.path());
    assert!(session.status().store_opened);
    session.set_voice("register", "plain").unwrap();
    assert!(!session.snapshot().llm_endpoint_configured);
    let runtime = Runtime::new(session);
    assert!(!runtime.is_synthetic());
    // Only a compiled-in synthetic fixture is used to seed this real-mode Runtime.
    let owner = RuntimeContext::new(Caller::desktop("synthetic-host"), None);
    let prepared = runtime
        .prepare_synthetic_import(
            &owner,
            SyntheticImportRequest {
                fixture_id: BuiltinFixtureId::Basic,
            },
        )
        .unwrap();
    runtime
        .confirm_import(
            &owner,
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .unwrap();
    let person = runtime.held_session().scoped_register_recipients().unwrap()[0]
        .person_id
        .to_string();
    (directory, runtime, person)
}

fn exchange(address: SocketAddr, raw: &str) -> (u16, Value) {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(6)))
        .unwrap();
    stream.write_all(raw.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = response.split("\r\n\r\n").nth(1).unwrap();
    (status, serde_json::from_str(body).unwrap())
}

fn request(server: &ApiServer, method: &str, path: &str, body: Value) -> (u16, Value) {
    let address = server.address();
    let auth = server.authorization();
    let body = if method == "GET" {
        String::new()
    } else {
        body.to_string()
    };
    exchange(address, &format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", auth.as_str(), body.len()))
}

fn proposal(server: &ApiServer, person: &str) -> String {
    let (status, draft) = request(
        server,
        "POST",
        "/v1/scoped/draft",
        json!({
            "pasted": "SYNTHETIC_PRIVATE_PASTE", "person_id": person, "occasion": "personal"
        }),
    );
    assert_eq!(status, 200);
    assert_eq!(draft["draft"]["carries_exempted_original"], false);
    assert!(!draft["draft"]["text"]
        .as_str()
        .unwrap()
        .contains("SYNTHETIC_PRIVATE_PASTE"));
    let (status, proposed) = request(
        server,
        "POST",
        "/v1/scoped/propose",
        json!({
            "result_id": draft["result_id"], "edit_revision": 1, "register": "formal"
        }),
    );
    assert_eq!(status, 200);
    proposed["proposal_id"].as_str().unwrap().to_owned()
}

#[test]
fn default_start_still_refuses_a_production_runtime_with_the_feature_enabled() {
    let (_directory, runtime, _person) = fixture();
    assert_eq!(
        ApiServer::start(runtime).unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
}

#[test]
fn validation_host_uses_preimported_production_session_and_only_local_scoped_flow() {
    let (directory, runtime, person) = fixture();
    let server = ApiServer::start_s1_validation(runtime.clone()).unwrap();
    assert!(server.address().ip().is_loopback());
    assert_eq!(
        request(&server, "GET", "/v1/status", Value::Null).1["store_opened"],
        true
    );
    assert!(
        request(&server, "GET", "/v1/people?offset=0&limit=100", Value::Null).1["total"]
            .as_u64()
            .unwrap()
            >= 2
    );
    assert_eq!(
        request(
            &server,
            "GET",
            &format!("/v1/scoped/status?person_id={person}&occasion=personal"),
            Value::Null
        )
        .0,
        200
    );
    assert_eq!(
        request(&server, "GET", "/v1/events/snapshot", Value::Null).0,
        200
    );
    // An allowed exact memory route reaches the core and reports an absent
    // resource as 503, rather than being hidden by the host's 404 scope refusal.
    assert_eq!(
        request(
            &server,
            "GET",
            "/v1/memories/00000000-0000-0000-0000-000000000001",
            Value::Null
        )
        .0,
        503
    );
    let id = proposal(&server, &person);
    assert_eq!(
        request(
            &server,
            "POST",
            &format!("/v1/scoped/{id}/discard"),
            json!({})
        )
        .0,
        200
    );
    assert!(runtime
        .held_session()
        .runtime_pending_slots()
        .scoped_proposal
        .is_none());
    assert!(!runtime.held_session().snapshot().llm_endpoint_configured);
    let address = server.address();
    server.shutdown();
    assert_eq!(server.in_flight_operations(), 0);
    assert!(TcpStream::connect(address).is_err());
    drop(server);
    drop(runtime);
    assert!(Session::open(directory.path()).status().store_opened);
}

#[test]
fn validation_host_forbidden_mutations_and_reads_do_not_enter_runtime() {
    let (_directory, runtime, person) = fixture();
    let server = ApiServer::start_s1_validation(runtime.clone()).unwrap();
    let proposal_id = proposal(&server, &person);
    let snapshot = serde_json::to_value(runtime.event_snapshot()).unwrap();
    let id = "00000000-0000-0000-0000-000000000001";
    let forbidden = [
        (
            "POST",
            "/v1/imports/prepare".to_owned(),
            json!({"fixture_id":"basic"}),
        ),
        (
            "POST",
            format!("/v1/imports/{id}/confirm"),
            json!({"preview_id":id}),
        ),
        ("POST", format!("/v1/imports/{id}/discard"), json!({})),
        (
            "POST",
            "/v1/scoped/confirm".to_owned(),
            json!({"proposal_id":proposal_id,"expected_revision":null}),
        ),
        (
            "PATCH",
            format!("/v1/memories/{id}"),
            json!({"summary":"SYNTHETIC"}),
        ),
        (
            "POST",
            format!("/v1/memories/{id}/forget-preview"),
            json!({}),
        ),
        (
            "POST",
            format!("/v1/memories/{id}/forget-confirm"),
            json!({"memory_id":id,"preview_id":id}),
        ),
        ("POST", "/v1/draft".to_owned(), json!({})),
        ("GET", format!("/v1/imports/{id}"), Value::Null),
        ("GET", format!("/v1/memories/{id}/state"), Value::Null),
    ];
    for (method, path, body) in forbidden {
        assert_eq!(request(&server, method, &path, body).0, 404);
        assert_eq!(
            serde_json::to_value(runtime.event_snapshot()).unwrap(),
            snapshot
        );
    }
    assert_eq!(
        runtime
            .held_session()
            .runtime_pending_slots()
            .scoped_proposal
            .map(|id| id.to_string()),
        Some(proposal_id.clone())
    );
    assert_eq!(
        request(
            &server,
            "POST",
            &format!("/v1/scoped/{proposal_id}/discard"),
            json!({})
        )
        .0,
        200
    );
}

#[test]
fn validation_host_retains_token_host_origin_and_strict_body_checks() {
    let (_directory, runtime, _person) = fixture();
    let server = ApiServer::start_s1_validation(runtime.clone()).unwrap();
    let address = server.address();
    let auth = server.authorization();
    let before = serde_json::to_value(runtime.event_snapshot()).unwrap();
    for raw in [
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\n\r\n"),
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer wrong\r\n\r\n"),
        format!("GET /v1/status HTTP/1.1\r\nHost: localhost:{}\r\nAuthorization: {}\r\n\r\n", address.port(), auth.as_str()),
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {}\r\nOrigin: https://example.invalid\r\n\r\n", auth.as_str()),
    ] {
        let (status, body) = exchange(address, &raw);
        assert!(status >= 400);
        assert!(!body.to_string().contains(auth.as_str()));
        assert_eq!(serde_json::to_value(runtime.event_snapshot()).unwrap(), before);
    }
    assert_eq!(
        request(
            &server,
            "POST",
            "/v1/scoped/draft",
            json!({"caller":"forged"})
        )
        .0,
        400
    );
    assert_eq!(
        serde_json::to_value(runtime.event_snapshot()).unwrap(),
        before
    );
}
