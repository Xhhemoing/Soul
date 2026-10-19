//! One process-owned bridge for the same Session and the native transport.
//! Stopping wakes the scheduler and the OS owner before joining either thread.
pub mod driver;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use soulcore::commands::session::{
    AssistanceNotificationId, AssistanceNotificationRuntimeState as State, Session,
};

use crate::notification_transport::{Config, Failure, NativeTransport, Phase, StopOutcome};

pub struct NotificationRuntime {
    stopping: Arc<AtomicBool>,
    wake: mpsc::Sender<()>,
    transport: Option<Arc<NativeTransport>>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl NotificationRuntime {
    pub fn start(
        session: Arc<Mutex<Session>>,
        mut on_open: impl FnMut(AssistanceNotificationId) + Send + 'static,
    ) -> Self {
        let stopping = Arc::new(AtomicBool::new(false));
        let (wake, receiver) = mpsc::channel();
        let transport = NativeTransport::spawn(Config {
            live_capacity: 1,
            queue_capacity: 32,
            retention: Duration::from_secs(300),
        });
        let transport = match transport {
            Ok(value) => Arc::new(value),
            Err(_) => {
                record(&session, State::Failed, Some("NOTIFICATION_START_FAILED"));
                return Self {
                    stopping,
                    wake,
                    transport: None,
                    worker: Mutex::new(None),
                };
            }
        };
        let worker_transport = Arc::clone(&transport);
        let worker_session = Arc::clone(&session);
        let worker_stopping = Arc::clone(&stopping);
        let worker = thread::Builder::new()
            .name("soul-notification-runtime".into())
            .spawn(move || {
                let mut driver = driver::Driver::default();
                let result = catch_unwind(AssertUnwindSafe(|| {
                    worker_session
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .start_assistance_reminder_scheduler();
                    loop {
                        let progress = driver.step(
                            &worker_session,
                            &*worker_transport,
                            &worker_stopping,
                            &mut on_open,
                        );
                        if worker_stopping.load(Ordering::Acquire) {
                            break;
                        }
                        let (state, error) = match worker_transport.phase() {
                            Phase::Ready => (
                                State::Running,
                                progress
                                    .coordination_failed
                                    .then_some("NOTIFICATION_COORDINATION_FAILED"),
                            ),
                            Phase::Starting => (State::NotStarted, None),
                            Phase::Failed(Failure::Unsupported) => {
                                (State::Unsupported, Some("NOTIFICATION_UNSUPPORTED"))
                            }
                            Phase::Failed(_) | Phase::Stopped => {
                                (State::Failed, Some("NOTIFICATION_TRANSPORT_FAILED"))
                            }
                            Phase::Stopping => (State::Stopping, None),
                        };
                        record(&worker_session, state, error);
                        match receiver.recv_timeout(Duration::from_secs(1)) {
                            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                        }
                    }
                }));
                // Neither the wait nor the join carries a Session/store guard.
                worker_transport.request_stop();
                {
                    let mut session = worker_session.lock().unwrap_or_else(|e| e.into_inner());
                    session.stop_assistance_reminder_scheduler();
                    session.record_assistance_notification_runtime_state(State::Stopping, None);
                }
                while worker_transport.wait_stopped(Duration::from_secs(1)) != StopOutcome::Joined {
                    record(
                        &worker_session,
                        State::Stopping,
                        Some("NOTIFICATION_STOP_UNKNOWN"),
                    );
                }
                // Late receipts still belong to their exact old attempt/WID.
                // A failed final write stays durable claimed for next-holder
                // recovery; it is never a reason to submit again during exit.
                let final_result = catch_unwind(AssertUnwindSafe(|| {
                    driver.flush(&worker_session, &*worker_transport, &mut |_| {})
                }));
                let (state, error) = if result.is_err() || final_result.is_err() {
                    (State::Failed, Some("NOTIFICATION_RUNTIME_PANICKED"))
                } else if final_result.is_ok_and(|value| value.coordination_failed) {
                    (State::Stopped, Some("NOTIFICATION_COORDINATION_FAILED"))
                } else {
                    (State::Stopped, None)
                };
                record(&worker_session, state, error);
            });
        let worker = match worker {
            Ok(value) => Some(value),
            Err(_) => {
                transport.request_stop();
                record(&session, State::Failed, Some("NOTIFICATION_START_FAILED"));
                None
            }
        };
        Self {
            stopping,
            wake,
            transport: Some(transport),
            worker: Mutex::new(worker),
        }
    }

    /// No Session/store lock may be held by the caller. A timed-out OS owner
    /// remains owned until its real join, including during application exit.
    pub fn request_stop(&self) {
        self.stopping.store(true, Ordering::Release);
        if let Some(transport) = &self.transport {
            transport.request_stop();
        }
        let _ = self.wake.send(());
    }

    pub fn shutdown(&self) {
        self.request_stop();
        let mut owner = self.worker.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(worker) = owner.take() {
            let _ = worker.join();
        }
        if let Some(transport) = &self.transport {
            while transport.wait_stopped(Duration::from_secs(1)) != StopOutcome::Joined {}
        }
    }
}

impl Drop for NotificationRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn record(session: &Arc<Mutex<Session>>, state: State, error: Option<&str>) {
    session
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .record_assistance_notification_runtime_state(state, error);
}
