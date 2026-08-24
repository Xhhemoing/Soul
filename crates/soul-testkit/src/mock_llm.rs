//! A loopback OpenAI-compatible endpoint that records what it was sent.
//!
//! AC-11 and AC-12 are about what leaves the machine, so the check has to
//! happen on the wire. This server keeps every raw request body verbatim,
//! which is what [`crate::leakage::LeakageChecker`] is then pointed at, and it
//! can answer with a cross-origin `302` so the egress guard's redirect refusal
//! has something real to refuse.
//!
//! It binds `127.0.0.1` on an ephemeral port and nothing else.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

/// One request as the server received it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    /// Raw body bytes decoded lossily. Nothing is parsed or normalized: a
    /// leakage check against a re-serialized body would be checking the wrong
    /// string.
    pub body: String,
}

#[derive(Debug, Default)]
struct MockState {
    requests: Mutex<Vec<RecordedRequest>>,
    /// When set, every request answers `302` pointing here.
    redirect_to: Mutex<Option<String>>,
}

/// A running mock endpoint. Dropping it stops the server.
#[derive(Debug)]
pub struct MockLlm {
    addr: SocketAddr,
    state: Arc<MockState>,
    shutdown: Option<oneshot::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl MockLlm {
    /// Bind an ephemeral loopback port and start serving.
    pub fn start() -> Result<MockLlm> {
        let state = Arc::new(MockState::default());
        let (addr_tx, addr_rx) = std::sync::mpsc::channel::<Result<SocketAddr, String>>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let server_state = Arc::clone(&state);
        let worker = std::thread::Builder::new()
            .name("soul-mock-llm".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = addr_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                runtime.block_on(async move {
                    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", 0)).await {
                        Ok(listener) => listener,
                        Err(error) => {
                            let _ = addr_tx.send(Err(error.to_string()));
                            return;
                        }
                    };
                    let bound = match listener.local_addr() {
                        Ok(bound) => bound,
                        Err(error) => {
                            let _ = addr_tx.send(Err(error.to_string()));
                            return;
                        }
                    };
                    if addr_tx.send(Ok(bound)).is_err() {
                        return;
                    }
                    let app = router(server_state);
                    let _ = axum::serve(listener, app)
                        .with_graceful_shutdown(async {
                            let _ = shutdown_rx.await;
                        })
                        .await;
                });
            })
            .context("spawning the mock endpoint thread")?;

        let addr = addr_rx
            .recv()
            .context("mock endpoint thread exited before binding")?
            .map_err(anyhow::Error::msg)
            .context("binding 127.0.0.1")?;

        Ok(MockLlm {
            addr,
            state,
            shutdown: Some(shutdown_tx),
            worker: Some(worker),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// The exact origin an E1 configuration would be pointed at.
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }

    pub fn chat_completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.base_url())
    }

    /// Everything received so far, in arrival order.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.state
            .requests
            .lock()
            .expect("no panics while holding the lock")
            .clone()
    }

    pub fn request_count(&self) -> usize {
        self.requests().len()
    }

    /// Answer every subsequent request with `302` to `target`. Used to prove
    /// the egress guard refuses to follow a redirect off the configured origin.
    pub fn set_redirect(&self, target: impl Into<String>) {
        *self
            .state
            .redirect_to
            .lock()
            .expect("no panics while holding the lock") = Some(target.into());
    }

    pub fn clear_redirect(&self) {
        *self
            .state
            .redirect_to
            .lock()
            .expect("no panics while holding the lock") = None;
    }

    /// Stop the server and wait for the thread to finish.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for MockLlm {
    fn drop(&mut self) {
        self.stop();
    }
}

fn router(state: Arc<MockState>) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handle))
        .route("/v1/models", any(handle))
        .fallback(any(handle))
        .with_state(state)
}

async fn handle(State(state): State<Arc<MockState>>, request: Request) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_owned();
    let headers = collect_headers(request.headers());

    let body = match axum::body::to_bytes(request.into_body(), 4 * 1024 * 1024).await {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => {
            return (StatusCode::BAD_REQUEST, format!("unreadable body: {error}")).into_response()
        }
    };

    state
        .requests
        .lock()
        .expect("no panics while holding the lock")
        .push(RecordedRequest {
            method,
            path,
            headers,
            body,
        });

    let redirect = state
        .redirect_to
        .lock()
        .expect("no panics while holding the lock")
        .clone();
    if let Some(target) = redirect {
        return (StatusCode::FOUND, [(axum::http::header::LOCATION, target)]).into_response();
    }

    axum::Json(serde_json::json!({
        "id": "chatcmpl-soul-mock",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "" },
            "finish_reason": "stop"
        }]
    }))
    .into_response()
}

fn collect_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_owned(),
                value.to_str().unwrap_or("<non-utf8>").to_owned(),
            )
        })
        .collect()
}
