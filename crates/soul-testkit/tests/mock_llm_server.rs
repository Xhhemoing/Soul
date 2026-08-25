//! The mock endpoint must bind loopback, record verbatim, redirect on demand,
//! and release the port on shutdown.
//!
//! Requests are written straight onto a `TcpStream`. Pulling in an HTTP client
//! to test the test double would be the one dependency this repository most
//! wants to keep out.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use soul_testkit::mock_llm::MockLlm;

fn post(addr: SocketAddr, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(addr).expect("connect to the mock endpoint");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set read timeout");
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        addr.port(),
        body.as_bytes().len(),
    );
    stream.write_all(request.as_bytes()).expect("write request");
    stream.flush().expect("flush");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read the response");
    response
}

#[test]
fn records_the_raw_request_body_and_then_shuts_down() {
    let server = MockLlm::start().expect("start the mock endpoint");
    let addr = server.addr();
    assert!(addr.ip().is_loopback(), "the mock must bind loopback only");
    assert_eq!(
        server.base_url(),
        format!("http://127.0.0.1:{}", addr.port())
    );
    assert_eq!(server.request_count(), 0);

    let body = r#"{"model":"local","messages":[{"role":"user","content":"[第三人正文已占位]"}]}"#;
    let response = post(addr, "/v1/chat/completions", body);
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "expected 200, got: {response}",
    );

    let recorded = server.requests();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].method, "POST");
    assert_eq!(recorded[0].path, "/v1/chat/completions");
    assert_eq!(
        recorded[0].body, body,
        "the body must be kept byte-for-byte; a re-serialized copy would hide whitespace and escaping",
    );
    assert!(
        recorded[0]
            .headers
            .iter()
            .any(|(name, _)| name == "content-type"),
        "headers are captured too",
    );

    server.shutdown();

    let mut refused = false;
    for _ in 0..100 {
        if TcpStream::connect(addr).is_err() {
            refused = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(refused, "the port must be released after shutdown");
}

#[test]
fn can_answer_with_a_cross_origin_redirect() {
    let server = MockLlm::start().expect("start the mock endpoint");
    let elsewhere = format!("http://127.0.0.1:{}/v1/chat/completions", server.port() + 1);
    server.set_redirect(&elsewhere);

    let response = post(server.addr(), "/v1/chat/completions", "{}");
    assert!(
        response.starts_with("HTTP/1.1 302"),
        "expected 302, got: {response}",
    );
    assert!(
        response.to_ascii_lowercase().contains("location:"),
        "the redirect must carry a Location header so the egress guard has something to refuse",
    );
    assert!(response.contains(&elsewhere));

    server.clear_redirect();
    let response = post(server.addr(), "/v1/chat/completions", "{}");
    assert!(response.starts_with("HTTP/1.1 200"));

    assert_eq!(server.request_count(), 2, "both attempts are recorded");
}

#[test]
fn two_servers_get_distinct_ports() {
    let first = MockLlm::start().expect("first");
    let second = MockLlm::start().expect("second");
    assert_ne!(first.port(), second.port());
}
