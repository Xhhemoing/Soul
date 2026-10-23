use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use soul_api::ApiServer;
use soul_runtime::Runtime;
use soulcore::commands::session::Session;

fn exchange(address: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(address).expect("synthetic loopback server");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn authenticated_status_uses_the_given_runtime_and_shutdown_releases_the_port() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = Runtime::new_synthetic(Session::open(directory.path()));
    assert!(runtime.status().store_opened);
    let server = ApiServer::start(runtime.clone()).unwrap();
    let address = server.address();
    let auth = server.authorization();
    let auth = auth.as_str();
    let response = exchange(
        address,
        &format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\n\r\n"),
    );
    assert!(response.starts_with("HTTP/1.1 200"), "status response");
    let body = response.split("\r\n\r\n").nth(1).unwrap();
    let status: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(status["store_opened"], true);
    assert!(status.get("store_notice").is_none());
    assert!(status["config_problem"].is_boolean());
    server.request_stop();
    server.shutdown();
    assert!(TcpStream::connect(address).is_err());
    drop(server);
    drop(runtime);
    assert!(Session::open(directory.path()).status().store_opened);
}

#[test]
fn untrusted_and_out_of_slice_requests_are_rejected_before_reading() {
    let directory = tempfile::tempdir().unwrap();
    let server = ApiServer::start(Runtime::new_synthetic(Session::open(directory.path()))).unwrap();
    let address = server.address();
    let auth = server.authorization();
    let auth = auth.as_str();
    let forbidden = [
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\n\r\n"),
        format!("GET /v1/status HTTP/1.1\r\nHost: localhost:{}\r\nAuthorization: {auth}\r\n\r\n", address.port()),
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\nOrigin: https://example.invalid\r\n\r\n"),
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\nAuthorization: {auth}\r\n\r\n"),
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nHost: {address}\r\nAuthorization: {auth}\r\n\r\n"),
        format!("POST /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\nContent-Length: 0\r\n\r\n"),
        format!("GET /v1/status?token=x HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\n\r\n"),
        format!("GET /v1/people?offset=0&limit=101 HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\n\r\n"),
        format!("GET /v1/people?offset=0&limit=1&unknown=1 HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\n\r\n"),
        format!("GET /v1/memories/not-a-uuid HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\n\r\n"),
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\nContent-Length: 1\r\n\r\nx"),
        format!("GET http://{address}/v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\n\r\n"),
        format!("GET /v1/write HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\n\r\n"),
    ];
    for request in forbidden {
        let response = exchange(address, &request);
        assert!(
            !response.starts_with("HTTP/1.1 200"),
            "rejection must not succeed"
        );
        assert!(
            !response.contains(auth),
            "response must not echo credentials"
        );
        assert!(!response.contains(&directory.path().display().to_string()));
    }
}

#[test]
fn a_slow_header_connection_expires_and_drop_stops_the_listener() {
    let directory = tempfile::tempdir().unwrap();
    let server = ApiServer::start(Runtime::new_synthetic(Session::open(directory.path()))).unwrap();
    let address = server.address();
    let mut slow = TcpStream::connect(address).unwrap();
    slow.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    slow.write_all(b"GET /v1/status HTTP/1.1\r\nHost:").unwrap();
    let mut byte = [0; 1];
    assert!(matches!(slow.read(&mut byte), Ok(0)));
    drop(server);
    assert!(TcpStream::connect(address).is_err());
}

#[test]
fn normal_runtime_cannot_be_exposed_by_the_http_host() {
    let directory = tempfile::tempdir().unwrap();
    assert!(ApiServer::start(Runtime::new(Session::open(directory.path()))).is_err());
}

#[test]
fn unavailable_synthetic_store_reports_false_health_and_safe_read_errors() {
    let directory = tempfile::tempdir().unwrap();
    let unavailable = directory.path().join("synthetic-unavailable-root");
    std::fs::write(&unavailable, b"synthetic directory obstruction").unwrap();
    let runtime = Runtime::new_synthetic(Session::open(&unavailable));
    assert!(!runtime.status().store_opened);
    let server = ApiServer::start(runtime).unwrap();
    let address = server.address();
    let authorization = server.authorization();
    let request = |path: &str| {
        exchange(
            address,
            &format!(
                "GET {path} HTTP/1.1\r\nHost: {address}\r\nAuthorization: {}\r\n\r\n",
                authorization.as_str()
            ),
        )
    };
    let status = request("/v1/status");
    assert!(status.starts_with("HTTP/1.1 200"));
    let body: serde_json::Value =
        serde_json::from_str(status.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["store_opened"], false);
    assert!(body.get("store_notice").is_none());
    let refusal = request("/v1/people?limit=1&offset=0");
    assert!(refusal.starts_with("HTTP/1.1 503"));
    let body: serde_json::Value =
        serde_json::from_str(refusal.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["code"], "read_failed");
    assert_eq!(body["details"], serde_json::Value::Null);
    for response in [status, refusal] {
        assert!(!response.contains("synthetic-unavailable-root"));
        assert!(!response.contains(&unavailable.display().to_string()));
        assert!(!response.contains(authorization.as_str()));
    }
}

#[test]
fn header_limits_and_slow_clients_do_not_exhaust_the_listener_forever() {
    let directory = tempfile::tempdir().unwrap();
    let server = ApiServer::start(Runtime::new_synthetic(Session::open(directory.path()))).unwrap();
    let address = server.address();
    let auth = server.authorization();
    let auth = auth.as_str();
    let mut many_headers =
        format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\n");
    for index in 0..40 {
        many_headers.push_str(&format!("X-Synthetic-{index}: x\r\n"));
    }
    many_headers.push_str("\r\n");
    let response = exchange(address, &many_headers);
    assert!(response.starts_with("HTTP/1.1 431") || response.starts_with("HTTP/1.1 400"));
    let oversized = format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\nX-Synthetic: {}\r\n\r\n", "x".repeat(soul_api::MAX_HEADER_BYTES + 1));
    let response = exchange(address, &oversized);
    assert!(response.starts_with("HTTP/1.1 431") || response.starts_with("HTTP/1.1 400"));
    let slow: Vec<_> = (0..soul_api::MAX_CONNECTIONS)
        .map(|_| {
            let mut stream = TcpStream::connect(address).unwrap();
            stream
                .write_all(b"GET /v1/status HTTP/1.1\r\nHost:")
                .unwrap();
            stream
        })
        .collect();
    let response = exchange(address, &format!("GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {auth}\r\nOrigin: http://{address}\r\n\r\n"));
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "header timeout frees occupied connection permits"
    );
    drop(slow);
    assert!(format!("{server:?}").contains("ApiServer"));
    assert!(!format!("{server:?}").contains(auth));
}

#[test]
fn shutdown_waits_for_a_dispatched_read_after_its_client_disconnects() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = Runtime::new_synthetic(Session::open(directory.path()));
    let server = std::sync::Arc::new(ApiServer::start(runtime.clone()).unwrap());
    let address = server.address();
    let authorization = server.authorization();
    let response = exchange(
        address,
        &format!(
            "GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {}\r\n\r\n",
            authorization.as_str()
        ),
    );
    assert!(response.starts_with("HTTP/1.1 200"));
    let held = runtime.held_session();
    let mut client = TcpStream::connect(address).unwrap();
    write!(
        client,
        "GET /v1/status HTTP/1.1\r\nHost: {address}\r\nAuthorization: {}\r\n\r\n",
        authorization.as_str()
    )
    .unwrap();
    let until = std::time::Instant::now() + Duration::from_secs(3);
    // Observe the real owner admission guard, which survives disconnection.
    // Framework state extraction/cloning is not evidence of worker dispatch.
    while server.in_flight_reads() != 1 {
        assert!(
            std::time::Instant::now() < until,
            "request reaches the dispatched runtime read"
        );
        std::thread::yield_now();
    }
    drop(client);
    let stopped = server.clone();
    let (sent, received) = std::sync::mpsc::channel();
    let shutdown = std::thread::spawn(move || {
        stopped.shutdown();
        sent.send(()).unwrap();
    });
    assert!(
        received.recv_timeout(Duration::from_millis(100)).is_err(),
        "shutdown must settle the real read, even after disconnect"
    );
    drop(held);
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    shutdown.join().unwrap();
    drop(server);
    drop(runtime);
    assert!(Session::open(directory.path()).status().store_opened);
}
