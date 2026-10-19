//! One production scheduling turn, shared with the real-Session regressions.
//! No UI, endpoint, approval, or content is carried by this scheduler.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use soulcore::commands::session::{Session, SessionRefusal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    Stopped,
    Idle,
    Completed,
}

/// The Session guard protects ownership transitions, never the owned work.
/// Shutdown prevents a new claim, but a claim already made is still completed.
pub fn cycle(
    session: &Arc<Mutex<Session>>,
    stopping: &AtomicBool,
) -> Result<CycleOutcome, SessionRefusal> {
    let work = {
        let mut session = session.lock().unwrap_or_else(|error| error.into_inner());
        // The stop request may have arrived while waiting for the mutex.
        if stopping.load(Ordering::Acquire) {
            return Ok(CycleOutcome::Stopped);
        }
        session.reconcile_assistance_work()?;
        if stopping.load(Ordering::Acquire) {
            return Ok(CycleOutcome::Stopped);
        }
        session.begin_assistance_work()?
    };
    let Some(work) = work else {
        return Ok(CycleOutcome::Idle);
    };
    let completion = work.execute();
    session
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .complete_assistance_work(completion)?;
    Ok(CycleOutcome::Completed)
}
