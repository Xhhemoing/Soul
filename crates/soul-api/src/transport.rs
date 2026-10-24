use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Query, Request, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::Response;
use axum::{routing::any, Router};
use hyper::body::Body as _;
use hyper::server::conn::http1;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::service::TowerToHyperService;
use serde::Serialize;
use soul_runtime::Runtime;
use soul_types::{
    MemoryIdRequest, PeoplePageRequest, ReadError, ReadErrorCode, ScopedStatusRequest,
};
use tokio::sync::{watch, Semaphore};
use tokio::task::JoinSet;

use crate::{Secret, HEADER_TIMEOUT, MAX_CONNECTIONS, MAX_HEADER_BYTES, MAX_RESPONSE_BYTES};

#[derive(Clone, Copy)]
pub(crate) enum HostScope {
    Synthetic,
    #[cfg(feature = "local-validation")]
    S1Validation,
}

impl HostScope {
    fn permits(self, _method: &Method, _path: &str) -> bool {
        match self {
            Self::Synthetic => true,
            #[cfg(feature = "local-validation")]
            Self::S1Validation => {
                if _method == Method::GET {
                    matches!(
                        _path,
                        "/v1/status" | "/v1/people" | "/v1/events/snapshot" | "/v1/scoped/status"
                    ) || _path
                        .strip_prefix("/v1/memories/")
                        .is_some_and(crate::writes::exact_id)
                } else {
                    crate::writes::s1_validation_route(_method, _path)
                }
            }
        }
    }
}

#[derive(Clone)]
struct Context {
    runtime: Runtime,
    authority: String,
    origin: String,
    secret: Arc<Secret>,
    read_slots: Arc<Semaphore>,
    reads: Arc<crate::read_activity::ReadActivity>,
    scope: HostScope,
}

pub(crate) async fn serve(
    listener: tokio::net::TcpListener,
    address: SocketAddr,
    runtime: Runtime,
    secret: Arc<Secret>,
    mut stop: watch::Receiver<bool>,
    reads: Arc<crate::read_activity::ReadActivity>,
    scope: HostScope,
) {
    let app = Router::new().fallback(any(handle)).with_state(Context {
        runtime,
        authority: address.to_string(),
        origin: format!("http://{address}"),
        secret,
        read_slots: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        reads,
        scope,
    });
    let semaphore = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let mut connections = JoinSet::new();
    loop {
        if *stop.borrow() {
            break;
        }
        // Do not accept more connections than can make progress. A slow header
        // occupies a permit for at most HEADER_TIMEOUT, including on shutdown.
        let permit = tokio::select! {
            biased;
            _ = stop.changed() => break,
            permit = semaphore.clone().acquire_owned() => match permit { Ok(permit) => permit, Err(_) => break },
        };
        let incoming = tokio::select! {
            biased;
            _ = stop.changed() => break,
            result = listener.accept() => result,
        };
        let Ok((stream, peer)) = incoming else {
            break;
        };
        if !peer.ip().is_loopback() {
            continue;
        }
        let service = TowerToHyperService::new(app.clone());
        connections.spawn(async move {
            let _permit = permit;
            let mut builder = http1::Builder::new();
            builder
                .keep_alive(false)
                .max_headers(32)
                .max_buf_size(MAX_HEADER_BYTES)
                .timer(TokioTimer::new())
                .header_read_timeout(HEADER_TIMEOUT);
            // No upgrades and no HTTP/2. Every connection has one request.
            // Parser errors are intentionally not logged or echoed.
            let _ = builder
                .serve_connection(
                    TokioIo::new(crate::deadline_io::DeadlineIo::new(stream)),
                    service,
                )
                .await;
        });
        while connections.try_join_next().is_some() {}
    }
    // Closing the listener precedes settling read workers. Do not abort a
    // dispatched blocking read or drop its Session ownership early.
    drop(listener);
    while connections.join_next().await.is_some() {}
}

pub(crate) fn unique_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let first = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Some(first)
}

fn equal_secret(provided: &[u8], expected: &[u8]) -> bool {
    if provided.len() != expected.len() {
        return false;
    }
    provided
        .iter()
        .zip(expected)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

async fn handle(State(context): State<Context>, request: Request) -> Response {
    let headers = request.headers();
    if request.uri().scheme().is_some()
        || request.uri().authority().is_some()
        || unique_header(headers, "host") != Some(context.authority.as_str())
    {
        return reject(StatusCode::BAD_REQUEST);
    }
    if headers.contains_key("origin")
        && unique_header(headers, "origin") != Some(context.origin.as_str())
    {
        return reject(StatusCode::FORBIDDEN);
    }
    let Some(authorization) = unique_header(headers, "authorization") else {
        return reject(StatusCode::UNAUTHORIZED);
    };
    let Some(token) = authorization.strip_prefix("Bearer ") else {
        return reject(StatusCode::UNAUTHORIZED);
    };
    if !equal_secret(token.as_bytes(), context.secret.0.as_bytes()) {
        return reject(StatusCode::UNAUTHORIZED);
    }
    // Trusted host selection is not a request field. Refuse out-of-scope
    // routes before body decoding, queue admission, events or Session work.
    if !context
        .scope
        .permits(request.method(), request.uri().path())
    {
        return reject(StatusCode::NOT_FOUND);
    }
    let is_write = request.method() != Method::GET;
    let operation = if is_write {
        match crate::writes::parse(request).await {
            Ok(operation) => Operation::Write(operation),
            Err(response) => return response,
        }
    } else {
        if headers.contains_key(header::TRANSFER_ENCODING)
            || (headers.contains_key(header::CONTENT_LENGTH)
                && unique_header(headers, "content-length") != Some("0"))
            || request.body().size_hint().lower() != 0
        {
            return reject(StatusCode::BAD_REQUEST);
        }
        let path = request.uri().path();
        if path == "/v1/status" && request.uri().query().is_none() {
            Operation::Status
        } else if path == "/v1/people" {
            let Ok(Query(page)) = Query::<PeoplePageRequest>::try_from_uri(request.uri()) else {
                return reject(StatusCode::BAD_REQUEST);
            };
            Operation::People(page)
        } else if path == "/v1/events/snapshot" && request.uri().query().is_none() {
            Operation::EventSnapshot
        } else if path == "/v1/scoped/status" {
            let Ok(Query(scope)) = Query::<ScopedStatusRequest>::try_from_uri(request.uri()) else {
                return reject(StatusCode::BAD_REQUEST);
            };
            Operation::ScopedStatus(scope)
        } else if let Some(id) = path.strip_prefix("/v1/imports/") {
            if !crate::writes::exact_id(id) || request.uri().query().is_some() {
                return reject(StatusCode::BAD_REQUEST);
            }
            Operation::ImportState(id.to_owned())
        } else if let Some(id) = path
            .strip_prefix("/v1/memories/")
            .and_then(|path| path.strip_suffix("/state"))
        {
            if !crate::writes::exact_id(id) || request.uri().query().is_some() {
                return reject(StatusCode::BAD_REQUEST);
            }
            Operation::MemoryState(id.to_owned())
        } else if let Some(id) = path.strip_prefix("/v1/memories/") {
            if id.len() != 36 || request.uri().query().is_some() {
                return reject(StatusCode::BAD_REQUEST);
            }
            Operation::Memory(id.to_owned())
        } else {
            return reject(StatusCode::NOT_FOUND);
        }
    };
    let runtime = context.runtime.clone();
    let Ok(read_permit) = context.read_slots.acquire_owned().await else {
        return reject(StatusCode::SERVICE_UNAVAILABLE);
    };
    let read_guard = context.reads.begin();
    // A disconnect cannot cancel a read already dispatched to the owner. The
    // connection and executor both wait for its actual completion on shutdown.
    match tokio::task::spawn_blocking(move || {
        // This permit lives inside the worker. A disconnected HTTP future
        // cannot release it and create an unbounded queue of abandoned reads.
        let _read_permit = read_permit;
        let _read_guard = read_guard;
        match operation {
            Operation::Status => json(StatusCode::OK, &runtime.status()),
            Operation::People(page) => match runtime.people(page) {
                Ok(page) => json(StatusCode::OK, &page),
                Err(error) => read_error(error),
            },
            Operation::Memory(memory_id) => match runtime.memory(MemoryIdRequest { memory_id }) {
                Ok(memory) => json(StatusCode::OK, &memory),
                Err(error) => read_error(error),
            },
            Operation::EventSnapshot => json(StatusCode::OK, &runtime.event_snapshot()),
            Operation::ImportState(id) => match runtime.safe_import_operation(&id) {
                Ok(value) => json(StatusCode::OK, &value),
                Err(error) => crate::writes::error_response(error),
            },
            Operation::MemoryState(id) => match runtime.safe_memory_state(&id) {
                Ok(value) => json(StatusCode::OK, &value),
                Err(error) => crate::writes::error_response(error),
            },
            Operation::ScopedStatus(scope) => {
                match runtime.safe_scoped_register_status(&scope.person_id, scope.occasion) {
                    Ok(value) => json(StatusCode::OK, &value),
                    Err(error) => crate::writes::error_response(error),
                }
            }
            Operation::Write(operation) => crate::writes::execute(&runtime, operation),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(_) if is_write => crate::writes::refuse(StatusCode::SERVICE_UNAVAILABLE),
        Err(_) => reject(StatusCode::SERVICE_UNAVAILABLE),
    }
}

enum Operation {
    Status,
    People(PeoplePageRequest),
    Memory(String),
    EventSnapshot,
    ImportState(String),
    MemoryState(String),
    ScopedStatus(ScopedStatusRequest),
    Write(crate::writes::WriteOperation),
}

pub(crate) fn json(status: StatusCode, value: &impl Serialize) -> Response {
    let mut bytes = LimitedBytes(Vec::new());
    match serde_json::to_writer(&mut bytes, value) {
        Ok(()) => Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CACHE_CONTROL, "no-store")
            .header("x-content-type-options", "nosniff")
            .header(header::CONNECTION, "close")
            .body(Body::from(bytes.0))
            .expect("static response headers"),
        _ => reject(StatusCode::PAYLOAD_TOO_LARGE),
    }
}

struct LimitedBytes(Vec<u8>);

impl std::io::Write for LimitedBytes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_RESPONSE_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "response limit",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn read_error(error: ReadError) -> Response {
    let status = match error.code {
        ReadErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
        ReadErrorCode::ReadFailed => StatusCode::SERVICE_UNAVAILABLE,
    };
    json(status, &error)
}

fn reject(status: StatusCode) -> Response {
    // Closed static codes/messages only; never include request/header/paths or
    // lower-level error text. Reuse the shared closed error representation.
    json(
        status,
        &ReadError {
            code: if status.is_server_error() {
                ReadErrorCode::ReadFailed
            } else {
                ReadErrorCode::InvalidRequest
            },
            message: "Local API request refused".into(),
            details: None,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_serialization_stops_at_the_transport_limit() {
        let response = json(StatusCode::OK, &"x".repeat(MAX_RESPONSE_BYTES + 1));
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
