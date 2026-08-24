//! A foreground source a test can drive.
//!
//! It ships in the crate rather than behind `cfg(test)` because the acceptance
//! tests for AC-09 and AC-10 have to run the *real* pipeline — source, consent
//! gate, encrypted store — on a Linux CI host where there is no desktop. The
//! only thing they are allowed to substitute is the one thing that cannot exist
//! there, and that substitution stops at this file: nothing downstream knows
//! whether the app name came from here or from `GetForegroundWindow`.
//!
//! Cloning is how a test keeps a grip on it. The collector owns its source, so
//! the handle the test switches applications with is a clone sharing the same
//! state.

use std::sync::{Arc, Mutex};

use crate::lock::lock;
use crate::source::{AppIdentity, ForegroundSource, SourceError};

#[derive(Debug, Default)]
struct FakeState {
    current: Option<AppIdentity>,
    samples_taken: u64,
    switches: u64,
    /// Consumed by the next `sample`, so a test can show what the collector
    /// does with a platform that failed once.
    fail_next: Option<SourceError>,
}

/// A scripted desktop.
#[derive(Debug, Clone, Default)]
pub struct FakeForegroundSource {
    state: Arc<Mutex<FakeState>>,
}

impl FakeForegroundSource {
    pub fn new() -> FakeForegroundSource {
        FakeForegroundSource::default()
    }

    /// A desktop that already has `app` in front.
    pub fn showing(app: &str) -> Result<FakeForegroundSource, SourceError> {
        let source = FakeForegroundSource::new();
        source.switch_to(app)?;
        Ok(source)
    }

    /// The user alt-tabbed to `app`.
    pub fn switch_to(&self, app: &str) -> Result<(), SourceError> {
        let app = AppIdentity::new(app)?;
        let mut state = lock(&self.state);
        state.current = Some(app);
        state.switches += 1;
        Ok(())
    }

    /// Nothing is in the foreground any more: the session locked, or focus
    /// went to the desktop.
    pub fn clear(&self) {
        let mut state = lock(&self.state);
        state.current = None;
    }

    /// Make the next sample fail.
    pub fn fail_next(&self, error: SourceError) {
        lock(&self.state).fail_next = Some(error);
    }

    /// How many times the collector has actually asked. A test that sees zero
    /// here knows the loop never ran, which is a different failure from a loop
    /// that ran and wrote nothing.
    pub fn samples_taken(&self) -> u64 {
        lock(&self.state).samples_taken
    }

    pub fn switches(&self) -> u64 {
        lock(&self.state).switches
    }
}

impl ForegroundSource for FakeForegroundSource {
    fn sample(&mut self) -> Result<Option<AppIdentity>, SourceError> {
        let mut state = lock(&self.state);
        state.samples_taken += 1;
        match state.fail_next.take() {
            Some(error) => Err(error),
            None => Ok(state.current.clone()),
        }
    }

    fn describe(&self) -> &'static str {
        "fake.scripted_desktop"
    }
}
