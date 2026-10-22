use super::*;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
};

const LIMIT: usize = 16 * 1024;
const READ_LIMIT: Duration = Duration::from_secs(2);

struct Request {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

pub(super) fn connection(
    mut stream: TcpStream,
    host: &str,
    token: &str,
    operator: &mut Operator,
    elapsed: Duration,
) {
    let began = Instant::now();
    let budget = READ_LIMIT.min(operator.deadline.saturating_sub(elapsed));
    // The polling listener is nonblocking; accepted sockets can inherit that
    // mode. Normalize this connection so read/write timeouts also cover valid
    // requests whose headers and body arrive in separate TCP reads.
    let response = stream
        .set_nonblocking(false)
        .map_err(|_| "transport")
        .and_then(|()| read_request(&mut stream, began, budget))
        .and_then(|request| route(request, host, token, operator, elapsed + began.elapsed()));
    let (status, content_type, body) = match response {
        Ok((content_type, body)) => ("200 OK", content_type, body),
        Err(code) => (
            "400 Bad Request",
            "application/json",
            json!({"error":code}).to_string(),
        ),
    };
    // Do not reflect request targets, headers, parser errors or body text.
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\
         Connection: close\r\nCache-Control: no-store\r\n\
         Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n\
         Referrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        body.len()
    );
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    let _ = stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(body.as_bytes()));
}

fn read_request(
    stream: &mut TcpStream,
    began: Instant,
    budget: Duration,
) -> Result<Request, &'static str> {
    let mut bytes = Vec::new();
    let header_end = loop {
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        read_more(stream, &mut bytes, began, budget)?;
    };
    let header = std::str::from_utf8(&bytes[..header_end - 4]).map_err(|_| "headers")?;
    if !header.is_ascii() {
        return Err("headers");
    }
    let mut lines = header.split("\r\n");
    let mut request_line = lines.next().ok_or("headers")?.split(' ');
    let method = request_line.next().ok_or("headers")?.to_owned();
    let path = request_line.next().ok_or("headers")?.to_owned();
    if request_line.next() != Some("HTTP/1.1") || request_line.next().is_some() {
        return Err("headers");
    }
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or("headers")?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            || value.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err("headers");
        }
        if headers
            .insert(name.to_ascii_lowercase(), value.trim().to_owned())
            .is_some()
        {
            return Err("duplicate_header");
        }
    }
    if headers.contains_key("transfer-encoding") || headers.contains_key("expect") {
        return Err("framing");
    }
    let content_length = match headers.get("content-length") {
        Some(value) if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            value.parse::<usize>().map_err(|_| "framing")?
        }
        Some(_) => return Err("framing"),
        None if method == "GET" => 0,
        None => return Err("framing"),
    };
    let total = header_end
        .checked_add(content_length)
        .filter(|total| *total <= LIMIT)
        .ok_or("size")?;
    if method == "GET" && content_length != 0 {
        return Err("framing");
    }
    while bytes.len() < total {
        read_more(stream, &mut bytes, began, budget)?;
    }
    if bytes.len() != total {
        return Err("framing");
    }
    Ok(Request {
        method,
        path,
        headers,
        body: bytes[header_end..].to_vec(),
    })
}

fn read_more(
    stream: &mut TcpStream,
    bytes: &mut Vec<u8>,
    began: Instant,
    budget: Duration,
) -> Result<(), &'static str> {
    let remaining = budget
        .checked_sub(began.elapsed())
        .filter(|left| !left.is_zero())
        .ok_or("timeout")?;
    stream
        .set_read_timeout(Some(remaining))
        .map_err(|_| "transport")?;
    let mut chunk = [0_u8; 2048];
    let count = stream.read(&mut chunk).map_err(|_| "timeout")?;
    if count == 0 {
        return Err("truncated");
    }
    if bytes.len() + count > LIMIT {
        return Err("size");
    }
    bytes.extend_from_slice(&chunk[..count]);
    Ok(())
}

fn route(
    request: Request,
    host: &str,
    token: &str,
    operator: &mut Operator,
    elapsed: Duration,
) -> Result<(&'static str, String), &'static str> {
    let began = Instant::now();
    if request.headers.get("host").map(String::as_str) != Some(host) {
        return Err("host");
    }
    let prefix = format!("/{token}/");
    let path = request.path.strip_prefix(&prefix).ok_or("route")?;
    match (request.method.as_str(), path) {
        ("GET", "") => Ok((
            "text/html; charset=utf-8",
            include_str!("../operator_ui/index.html").into(),
        )),
        ("GET", "app.js") => Ok((
            "text/javascript; charset=utf-8",
            include_str!("../operator_ui/app.js").into(),
        )),
        ("GET", "timer.js") => Ok((
            "text/javascript; charset=utf-8",
            include_str!("../operator_ui/timer.js").into(),
        )),
        ("GET", "styles.css") => Ok((
            "text/css; charset=utf-8",
            include_str!("../operator_ui/styles.css").into(),
        )),
        ("POST", "api") => {
            if request.headers.get("origin") != Some(&format!("http://{host}")) {
                return Err("origin");
            }
            if request.headers.get("x-soul-operator").map(String::as_str) != Some(token) {
                return Err("token");
            }
            if request.headers.get("content-type").map(String::as_str) != Some("application/json") {
                return Err("content_type");
            }
            let mut result = operator.handle(&request.body, elapsed)?;
            let response_elapsed = elapsed + began.elapsed();
            if response_elapsed >= operator.deadline {
                operator.finish(false);
                result = operator.state()?;
            }
            if operator.stage != Stage::Finished {
                result["remaining_seconds"] =
                    json!(operator.deadline.saturating_sub(response_elapsed).as_secs());
            }
            Ok(("application/json", result.to_string()))
        }
        _ => Err("route"),
    }
}
