//! Best-effort native notifications; authoritative reconciliation is a Runtime snapshot.
//!
//! Keep this RAII owner outside managed App state. Its worker holds an AppHandle,
//! which would otherwise form App -> managed relay -> App and prevent cleanup.
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tauri::Emitter;

pub const EVENT_NAME: &str = "soul-runtime-change";

pub struct RuntimeEventRelay {
    stop: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl RuntimeEventRelay {
    pub fn start<R: tauri::Runtime>(
        runtime: &soul_runtime::Runtime,
        handle: tauri::AppHandle<R>,
    ) -> io::Result<Self> {
        let receiver = runtime.subscribe_events(128);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("soul-runtime-events".into())
            .spawn(move || {
                while !stopping.load(Ordering::Acquire) {
                    match receiver.recv_timeout(Duration::from_millis(50)) {
                        Ok(event) => {
                            let _ = handle.emit(EVENT_NAME, event);
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self {
            stop,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::Release);
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

impl Drop for RuntimeEventRelay {
    fn drop(&mut self) {
        self.shutdown();
    }
}
