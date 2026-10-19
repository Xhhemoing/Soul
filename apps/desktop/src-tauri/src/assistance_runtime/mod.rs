//! Process-owned local work driver. This module is intentionally independent
//! of Tauri so the shipped thread and scheduling turn can use real-store tests.

pub mod driver;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use soulcore::commands::session::{AssistanceRuntimeState, Session};

pub struct AssistanceRuntime {
    stopping: Arc<AtomicBool>,
    wake: mpsc::Sender<()>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl AssistanceRuntime {
    pub fn start(session: Arc<Mutex<Session>>) -> Self {
        let stopping = Arc::new(AtomicBool::new(false));
        let (wake, receiver) = mpsc::channel();
        let worker_session = Arc::clone(&session);
        let worker_stopping = Arc::clone(&stopping);
        let worker = thread::Builder::new()
            .name("soul-local-assistance".into())
            .spawn(move || {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    record(&worker_session, AssistanceRuntimeState::Running, None);
                    loop {
                        match driver::cycle(&worker_session, &worker_stopping) {
                            Ok(driver::CycleOutcome::Stopped) => break,
                            Ok(_) => {
                                record(&worker_session, AssistanceRuntimeState::Running, None);
                            }
                            Err(_) => record(
                                &worker_session,
                                AssistanceRuntimeState::Running,
                                Some("ASSISTANCE_RUNTIME_COORDINATION_FAILED"),
                            ),
                        }
                        if worker_stopping.load(Ordering::Acquire) {
                            break;
                        }
                        // No timer runs in a mounted route. A disconnected owner
                        // is also a stop request; it must not create a hot loop.
                        match receiver.recv_timeout(Duration::from_secs(1)) {
                            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                        }
                    }
                }));
                if result.is_err() {
                    record(
                        &worker_session,
                        AssistanceRuntimeState::Failed,
                        Some("ASSISTANCE_RUNTIME_PANICKED"),
                    );
                } else {
                    record(&worker_session, AssistanceRuntimeState::Stopped, None);
                }
            });
        let worker = match worker {
            Ok(worker) => Some(worker),
            Err(_) => {
                record(
                    &session,
                    AssistanceRuntimeState::Failed,
                    Some("ASSISTANCE_RUNTIME_START_FAILED"),
                );
                None
            }
        };
        Self {
            stopping,
            wake,
            worker: Mutex::new(worker),
        }
    }

    /// Called on process exit and on Drop. Never called with a Session/store
    /// guard: an already executing worker retains its physical slot until join.
    pub fn request_stop(&self) {
        self.stopping.store(true, Ordering::Release);
        let _ = self.wake.send(());
    }

    pub fn shutdown(&self) {
        self.request_stop();
        if let Some(worker) = self
            .worker
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take()
        {
            let _ = worker.join();
        }
    }
}

impl Drop for AssistanceRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn record(session: &Arc<Mutex<Session>>, state: AssistanceRuntimeState, error: Option<&str>) {
    session
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .record_assistance_runtime_state(state, error);
}
