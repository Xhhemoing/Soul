use serde_json::{json, Value};
use soul_api::ApiServer;
use soul_runtime::Runtime;
use soulcore::commands::session::Session;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

fn request(
    address: SocketAddr,
    authorization: &str,
    method: &str,
    path: &str,
    body: &str,
) -> (u16, Value) {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(6)))
        .unwrap();
    write!(stream, "{method} {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: {authorization}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let status = response.split_whitespace().nth(1).unwrap().parse().unwrap();
    let body = response.split("\r\n\r\n").nth(1).unwrap();
    (status, serde_json::from_str(body).unwrap())
}

#[test]
fn a_builtin_fixture_import_is_prepared_and_confirmed_over_real_http() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = Runtime::new_synthetic(Session::open(directory.path()));
    let server = ApiServer::start(runtime.clone()).unwrap();
    let authorization = server.authorization();
    let (status, prepared) = request(
        server.address(),
        authorization.as_str(),
        "POST",
        "/v1/imports/prepare",
        r#"{"fixture_id":"basic"}"#,
    );
    assert_eq!(status, 200, "the allowlisted write route must execute");
    let operation = prepared["operation"]["operation_id"].as_str().unwrap();
    let preview = prepared["operation"]["preview_id"].as_str().unwrap();
    let (status, committed) = request(
        server.address(),
        authorization.as_str(),
        "POST",
        &format!("/v1/imports/{operation}/confirm"),
        &json!({"preview_id":preview}).to_string(),
    );
    assert_eq!(status, 200);
    assert_eq!(committed["state"], "committed");
    let (status, readback) = request(
        server.address(),
        authorization.as_str(),
        "GET",
        &format!("/v1/imports/{operation}"),
        "",
    );
    assert_eq!(status, 200);
    assert_eq!(readback, committed);
    let snapshot = request(
        server.address(),
        authorization.as_str(),
        "GET",
        "/v1/events/snapshot",
        "",
    );
    assert_eq!(snapshot.0, 200);
    assert!(snapshot.1["active_operations"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(snapshot.1["sequence"].as_u64().unwrap() >= 4);
}

#[test]
fn write_json_is_strict_bounded_and_received_before_any_runtime_admission() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = Runtime::new_synthetic(Session::open(directory.path()));
    let server = ApiServer::start(runtime.clone()).unwrap();
    let authorization = server.authorization();
    for body in [
        r#"{"fixture_id":"basic","caller":"desktop"}"#,
        r#"{"fixture_id":"basic","correlation_id":"00000000-0000-0000-0000-000000000000"}"#,
        r#"{"fixture_id":"basic","text":"not allowed"}"#,
        r#"{"fixture_id":"unknown"}"#,
        "{",
    ] {
        let response = request(
            server.address(),
            authorization.as_str(),
            "POST",
            "/v1/imports/prepare",
            body,
        );
        assert_eq!(response.0, 400);
    }
    let oversized = json!({"fixture_id":"basic","unknown":"x".repeat(65536)}).to_string();
    assert_eq!(
        request(
            server.address(),
            authorization.as_str(),
            "POST",
            "/v1/imports/prepare",
            &oversized
        )
        .0,
        413
    );
    let mut slow = TcpStream::connect(server.address()).unwrap();
    slow.set_read_timeout(Some(Duration::from_secs(6))).unwrap();
    write!(slow, "POST /v1/imports/prepare HTTP/1.1\r\nHost: {}\r\nAuthorization: {}\r\nContent-Type: application/json\r\nContent-Length: 16\r\n\r\n{{", server.address(), authorization.as_str()).unwrap();
    let mut response = String::new();
    slow.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 408"));
    assert_eq!(server.in_flight_reads(), 0);
    assert_eq!(runtime.event_snapshot().sequence, 0);
}

#[test]
fn explicit_discard_routes_preserve_the_existing_confirmation_slots() {
    let directory = tempfile::tempdir().unwrap();
    let server = ApiServer::start(Runtime::new_synthetic(Session::open(directory.path()))).unwrap();
    let authorization = server.authorization();
    let (status, prepared) = request(
        server.address(),
        authorization.as_str(),
        "POST",
        "/v1/imports/prepare",
        r#"{"fixture_id":"basic"}"#,
    );
    assert_eq!(status, 200);
    let id = prepared["operation"]["operation_id"].as_str().unwrap();
    let discarded = request(
        server.address(),
        authorization.as_str(),
        "POST",
        &format!("/v1/imports/{id}/discard"),
        "{}",
    );
    assert_eq!(discarded.0, 200);
    assert_eq!(discarded.1["state"], "discarded");
    let retry = request(
        server.address(),
        authorization.as_str(),
        "POST",
        &format!("/v1/imports/{id}/confirm"),
        &json!({"preview_id":prepared["operation"]["preview_id"]}).to_string(),
    );
    assert_ne!(retry.0, 200);
}

#[test]
fn disconnected_accepted_write_settles_before_shutdown_returns() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = Runtime::new_synthetic(Session::open(directory.path()));
    let server = std::sync::Arc::new(ApiServer::start(runtime.clone()).unwrap());
    let authorization = server.authorization();
    let held = runtime.held_session();
    let body = r#"{"fixture_id":"basic"}"#;
    let mut client = TcpStream::connect(server.address()).unwrap();
    write!(client, "POST /v1/imports/prepare HTTP/1.1\r\nHost: {}\r\nAuthorization: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", server.address(), authorization.as_str(), body.len()).unwrap();
    let until = std::time::Instant::now() + Duration::from_secs(3);
    while server.in_flight_reads() != 1 {
        assert!(
            std::time::Instant::now() < until,
            "write reaches owned worker"
        );
        std::thread::yield_now();
    }
    drop(client);
    let stop = server.clone();
    let (sent, received) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        stop.shutdown();
        sent.send(()).unwrap();
    });
    assert!(received.recv_timeout(Duration::from_millis(100)).is_err());
    drop(held);
    received.recv_timeout(Duration::from_secs(6)).unwrap();
    worker.join().unwrap();
    let snapshot = runtime.event_snapshot();
    assert_eq!(snapshot.sequence, 2);
    assert!(snapshot.active_operations.is_empty());
    assert_eq!(
        runtime
            .held_session()
            .recent_import_operations()
            .unwrap()
            .len(),
        1
    );
}
