use super::*;
use serde_json::json;
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener},
    thread,
};

fn call(operator: &mut Operator, request: Value, seconds: u64) -> Result<Value, &'static str> {
    operator.handle(
        &serde_json::to_vec(&request).unwrap(),
        Duration::from_secs(seconds),
    )
}

fn feedback(card: usize, complete: bool) -> Value {
    let side = json!({
        "completed":complete, "first_pass":false, "edit_seconds":null,
        "pause_seconds":null, "help_count":0, "unsupported_claim":if card == 0 {None} else {Some(false)},
        "fact_errors":if card >= 2 {Some(0)} else {None}, "safety_stop":false,
        "t1_slot_filled":card == 0 && complete
    });
    json!({"action":"submit","card":card,"feedback":{
        "preference":"tie","blind_status":"intact","sides":[side.clone(),side]
    }})
}

#[test]
fn no_mapping_until_feedback_and_explicit_reveal() {
    let mut op = Operator::new(90);
    assert_eq!(
        call(&mut op, json!({"action":"state"}), 0).unwrap(),
        json!({"stage":"welcome"})
    );
    let shown = call(&mut op, json!({"action":"start"}), 1).unwrap();
    assert_eq!(shown["stage"], "pair");
    assert!(shown["left_text"].is_string());
    assert!(shown.get("mapping").is_none());
    assert!(call(&mut op, json!({"action":"reveal","card":0}), 2).is_err());
    assert!(call(&mut op, json!({"action":"report"}), 2).is_err());
    assert_eq!(
        call(&mut op, feedback(0, true), 3).unwrap()["stage"],
        "recorded"
    );
    assert!(call(&mut op, feedback(0, true), 4).is_err());
    assert!(call(&mut op, json!({"action":"next","card":0}), 4).is_err());
    assert_eq!(
        call(&mut op, json!({"action":"reveal","card":0}), 4).unwrap()["mapping"],
        json!(["a", "b"])
    );
    assert_eq!(
        call(&mut op, json!({"action":"next","card":0}), 5).unwrap()["card"],
        1
    );
    assert!(call(&mut op, feedback(0, true), 6).is_err());
}

#[test]
fn completed_missing_times_export_without_body_or_false_zero() {
    let mut op = Operator::new(90);
    let shown = call(&mut op, json!({"action":"start"}), 1).unwrap();
    call(&mut op, feedback(0, true), 2).unwrap();
    let report = call(&mut op, json!({"action":"stop"}), 3).unwrap();
    let metadata = &report["report"]["metadata"];
    assert_eq!(metadata["pairs"].as_array().unwrap().len(), 5);
    assert_eq!(metadata["candidates"].as_array().unwrap().len(), 10);
    assert_eq!(
        metadata["candidates"][0]["edit_seconds"],
        json!({"missing":"not_measured"})
    );
    assert_eq!(
        metadata["candidates"][0]["sanitized_note"],
        json!({"value":"synthetic_operator_feedback"})
    );
    assert_eq!(metadata["runs"][0]["consent_state"], "granted");
    let serialized = serde_json::to_string(metadata).unwrap();
    assert!(!serialized.contains(shown["left_text"].as_str().unwrap()));
    assert!(super::super::manual::report_json(&serialized).is_ok());
    assert!(call(&mut op, feedback(0, true), 4).is_err());
}

#[test]
fn deadline_withdraw_and_stop_keep_unstarted_denominator() {
    for action in ["state", "withdraw", "stop"] {
        let mut op = Operator::new(2);
        let response = call(&mut op, json!({"action":action}), 2).unwrap();
        assert_eq!(response["stage"], "finished");
        assert_eq!(
            response["report"]["metadata"]["pairs"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert!(call(&mut op, json!({"action":"start"}), 2).is_err());
    }
    let mut op = Operator::new(90);
    call(&mut op, json!({"action":"start"}), 1).unwrap();
    let response = call(&mut op, json!({"action":"withdraw"}), 2).unwrap();
    assert_eq!(
        response["report"]["metadata"]["runs"][0]["consent_state"],
        "withdrawn"
    );
}

#[test]
fn safety_feedback_can_reveal_then_skips_same_path() {
    let mut op = Operator::new(90);
    call(&mut op, json!({"action":"start"}), 1).unwrap();
    for card in 0..3 {
        let mut request = feedback(card, false);
        if card == 2 {
            request["feedback"]["sides"][0]["safety_stop"] = json!(true);
        }
        call(&mut op, request, 2 + card as u64 * 3).unwrap();
        call(
            &mut op,
            json!({"action":"reveal","card":card}),
            3 + card as u64 * 3,
        )
        .unwrap();
        let response = call(
            &mut op,
            json!({"action":"next","card":card}),
            4 + card as u64 * 3,
        )
        .unwrap();
        if card == 2 {
            assert_eq!(response["stage"], "finished");
            assert_eq!(
                response["report"]["metadata"]["pairs"]
                    .as_array()
                    .unwrap()
                    .len(),
                5
            );
        }
    }
}

#[test]
fn feedback_rejects_unknown_fields_and_impossible_time_without_mutating() {
    let mut op = Operator::new(90);
    call(&mut op, json!({"action":"start"}), 1).unwrap();
    let mut request = feedback(0, true);
    request["feedback"]["sides"][0]["edit_seconds"] = json!(80);
    assert!(call(&mut op, request, 2).is_err());
    let mut request = feedback(0, true);
    request["feedback"]["text"] = json!("never accepted");
    assert!(call(&mut op, request, 2).is_err());
    assert_eq!(
        call(&mut op, feedback(0, true), 2).unwrap()["stage"],
        "recorded"
    );
}

#[test]
fn incomplete_observations_and_full_sequence_keep_original_task_validation() {
    let mut op = Operator::new(90);
    call(&mut op, json!({"action":"start"}), 1).unwrap();
    for card in 0..5 {
        let mut invalid = feedback(card, true);
        if card == 0 {
            invalid["feedback"]["sides"][0]["first_pass"] = json!(true);
        } else if card == 1 {
            invalid["feedback"]["sides"][0]["unsupported_claim"] = json!(true);
        } else {
            invalid["feedback"]["sides"][0]["fact_errors"] = Value::Null;
        }
        assert!(call(&mut op, invalid, 2 + card as u64 * 4).is_err());
        let mut request = feedback(card, card != 0);
        if card == 0 {
            request["feedback"]["sides"][0]["edit_seconds"] = json!(0);
            request["feedback"]["sides"][0]["pause_seconds"] = json!(0);
        }
        call(&mut op, request, 2 + card as u64 * 4).unwrap();
        call(
            &mut op,
            json!({"action":"reveal","card":card}),
            3 + card as u64 * 4,
        )
        .unwrap();
        let response = call(
            &mut op,
            json!({"action":"next","card":card}),
            4 + card as u64 * 4,
        )
        .unwrap();
        if card == 4 {
            assert_eq!(response["stage"], "finished");
            assert_eq!(
                response["report"]["metadata"]["candidates"][0]["edit_seconds"],
                json!({"missing":"not_completed"})
            );
            assert_eq!(response["report"]["summary"]["funnel"]["assigned"], 5);
            assert_eq!(response["report"]["summary"]["funnel"]["completed"], 4);
            assert_eq!(
                response,
                call(&mut op, json!({"action":"report"}), 21).unwrap()
            );
        }
    }
}

#[test]
fn request_shape_cannot_inject_text_or_omit_explicit_missing_fields() {
    let mut op = Operator::new(90);
    for request in [
        json!({"action":"state","text":"arbitrary"}),
        json!({"action":"start","endpoint":"http://external.invalid"}),
        json!({"action":"reveal","card":99}),
    ] {
        assert!(call(&mut op, request, 0).is_err());
    }
    call(&mut op, json!({"action":"start"}), 1).unwrap();
    let mut request = feedback(0, true);
    request["feedback"]["sides"][0]
        .as_object_mut()
        .unwrap()
        .remove("edit_seconds");
    assert!(call(&mut op, request, 2).is_err());
    assert!(op
        .handle(
            br#"{"action":"state","action":"stop"}"#,
            Duration::from_secs(2)
        )
        .is_err());
    assert_eq!(
        call(&mut op, json!({"action":"state"}), 2).unwrap()["stage"],
        "pair"
    );
}

#[test]
fn unrated_incomplete_first_pass_stays_missing_with_generated_denominator() {
    let mut op = Operator::new(90);
    call(&mut op, json!({"action":"start"}), 1).unwrap();
    let mut request = feedback(0, false);
    request["feedback"]["preference"] = json!("unrated");
    for side in request["feedback"]["sides"].as_array_mut().unwrap() {
        side["first_pass"] = Value::Null;
    }
    let mut invalid_completed = request.clone();
    for side in invalid_completed["feedback"]["sides"]
        .as_array_mut()
        .unwrap()
    {
        side["completed"] = json!(true);
        side["t1_slot_filled"] = json!(true);
    }
    assert!(call(&mut op, invalid_completed, 2).is_err());
    assert_eq!(call(&mut op, request, 2).unwrap()["stage"], "recorded");
    let report = call(&mut op, json!({"action":"stop"}), 3).unwrap();
    assert_eq!(
        report["report"]["metadata"]["candidates"][0]["first_pass_accepted"],
        json!({"missing":"not_rated"})
    );
    assert_eq!(report["report"]["summary"]["generated_candidates"], 2);
    assert_eq!(report["report"]["summary"]["funnel"]["completed"], 0);
    let rows = report["report"]["summary"]["candidates"]
        .as_array()
        .unwrap();
    let denominator: u64 = rows
        .iter()
        .map(|row| {
            row["summary"]["first_pass"]["denominator"]
                .as_u64()
                .unwrap()
        })
        .sum();
    let missing: u64 = rows
        .iter()
        .map(|row| row["summary"]["first_pass_missing"].as_u64().unwrap())
        .sum();
    assert_eq!(denominator, 2);
    assert_eq!(missing, 2);
}

fn exchange(headers: &str, body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let host = address.to_string();
    let request = headers.replace("$HOST", &host);
    let worker = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        connection(
            stream,
            &host,
            "test-token",
            &mut Operator::new(90),
            Duration::ZERO,
        );
    });
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .write_all(format!("{request}\r\n\r\n{body}").as_bytes())
        .unwrap();
    stream.shutdown(Shutdown::Write).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    worker.join().unwrap();
    response
}

#[test]
fn real_tcp_rejects_host_origin_token_and_ambiguous_framing() {
    let valid = "POST /test-token/api HTTP/1.1\r\nHost: $HOST\r\nOrigin: http://$HOST\r\nX-Soul-Operator: test-token\r\nContent-Type: application/json\r\nContent-Length: 18";
    let response = exchange(valid, "{\"action\":\"state\"}");
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.contains("Cache-Control: no-store"));
    for headers in [
        valid.replace("Host: $HOST", "Host: evil.invalid"),
        valid.replace("Origin: http://$HOST\r\n", ""),
        valid.replace("Origin: http://$HOST", "Origin: null"),
        valid.replace("X-Soul-Operator: test-token", "X-Soul-Operator: wrong"),
        format!("{valid}\r\nHost: $HOST"),
        format!("{valid}\r\nContent-Length: 18"),
        format!("{valid}\r\nTransfer-Encoding: chunked"),
        valid.replace("Content-Length: 18", "Content-Length: 999999"),
        valid.replace("Content-Length: 18", "Content-Length: 19"),
        valid.replace("Content-Type: application/json", "Content-Type: text/plain"),
        valid.replace("/test-token/api", "/wrong/api"),
        valid.replace("Host: $HOST", "Host: $HOST\r\nHOST: $HOST"),
    ] {
        let response = exchange(&headers, "{\"action\":\"state\"}");
        assert!(response.starts_with("HTTP/1.1 4"), "{response}");
        assert!(!response.contains("evil.invalid"));
    }
}

#[test]
fn real_tcp_static_routes_are_fixed_and_api_remaining_time_is_bounded() {
    let response = exchange("GET /test-token/app.js HTTP/1.1\r\nHost: $HOST", "");
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("Content-Type: text/javascript"));
    assert!(response.contains("frame-ancestors 'none'"));
    assert!(response.contains("Referrer-Policy: no-referrer"));
    assert!(response.contains("X-Content-Type-Options: nosniff"));
    assert!(!response.contains("Access-Control-Allow-Origin"));
    for path in [
        "../records.rs",
        "api",
        "app.js?query=1",
        "%2e%2e/records.rs",
    ] {
        assert!(exchange(
            &format!("GET /test-token/{path} HTTP/1.1\r\nHost: $HOST"),
            ""
        )
        .starts_with("HTTP/1.1 4"));
    }
    let response = exchange(
        "POST /test-token/api HTTP/1.1\r\nHost: $HOST\r\nOrigin: http://$HOST\r\nX-Soul-Operator: test-token\r\nContent-Type: application/json\r\nContent-Length: 18",
        "{\"action\":\"state\"}",
    );
    let body: Value = serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert!(body["remaining_seconds"].as_u64().unwrap() <= 90);
}

#[test]
fn real_tcp_partial_request_times_out_without_waiting_for_peer_close() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let worker = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        connection(
            stream,
            &address.to_string(),
            "test-token",
            &mut Operator::new(90),
            Duration::ZERO,
        );
    });
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    stream
        .write_all(b"POST /test-token/api HTTP/1.1\r\n")
        .unwrap();
    let began = Instant::now();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    worker.join().unwrap();
    assert!(response.starts_with("HTTP/1.1 400"));
    assert!(response.contains("\"error\":\"timeout\""));
    assert!(began.elapsed() < Duration::from_secs(4));
}

fn fragmented_start_from_nonblocking_listener(explicit_stream_nonblocking: bool) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (headers_sent, headers_ready) = std::sync::mpsc::channel();
    let (entering, entered) = std::sync::mpsc::channel();
    let worker = thread::spawn(move || {
        let began = Instant::now();
        let stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(began.elapsed() < Duration::from_secs(2), "accept deadline");
                    thread::sleep(Duration::from_millis(2));
                }
                Err(_) => panic!("synthetic listener accept failed"),
            }
        };
        if explicit_stream_nonblocking {
            stream.set_nonblocking(true).unwrap();
        }
        headers_ready.recv_timeout(Duration::from_secs(2)).unwrap();
        entering.send(()).unwrap();
        connection(
            stream,
            &address.to_string(),
            "test-token",
            &mut Operator::new(90),
            Duration::ZERO,
        );
    });
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    let body = br#"{"action":"start"}"#;
    let headers = format!(
        "POST /test-token/api HTTP/1.1\r\nHost: {address}\r\nOrigin: http://{address}\r\n\
         X-Soul-Operator: test-token\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).unwrap();
    headers_sent.send(()).unwrap();
    entered.recv_timeout(Duration::from_secs(2)).unwrap();
    // A valid request need not arrive in a single TCP read. Keep the gap well
    // inside the parser's two-second total budget, with the peer still open.
    thread::sleep(Duration::from_millis(100));
    let body_written = stream.write_all(body).is_ok();
    let _ = stream.shutdown(Shutdown::Write);
    let mut response = String::new();
    let _ = stream.read_to_string(&mut response);
    worker.join().unwrap();
    // A premature rejection may close/reset before the second write. Diagnose
    // the observed HTTP result, without dumping candidate text or the token.
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "valid fragmented start was rejected; explicit_nonblocking={explicit_stream_nonblocking}, body_written={body_written}"
    );
    let payload: Value = serde_json::from_str(
        response
            .split_once("\r\n\r\n")
            .expect("HTTP response framing")
            .1,
    )
    .expect("JSON response");
    assert!(payload.get("error").is_none());
    assert!(payload["stage"] == "pair");
}

#[test]
fn real_tcp_fragmented_start_after_nonblocking_accept() {
    fragmented_start_from_nonblocking_listener(false);
}

#[test]
fn real_tcp_fragmented_start_with_explicit_nonblocking_stream() {
    fragmented_start_from_nonblocking_listener(true);
}
