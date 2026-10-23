//! Tracks admitted reads and writes independently of HTTP connection future lifetimes.
use std::sync::{Arc, Condvar, Mutex};

#[derive(Default)]
pub(crate) struct ReadActivity {
    count: Mutex<usize>,
    settled: Condvar,
}

impl ReadActivity {
    pub(crate) fn begin(self: &Arc<Self>) -> ReadGuard {
        *self.count.lock().unwrap_or_else(|error| error.into_inner()) += 1;
        ReadGuard {
            activity: Arc::clone(self),
        }
    }

    pub(crate) fn count(&self) -> usize {
        *self.count.lock().unwrap_or_else(|error| error.into_inner())
    }

    pub(crate) fn wait_empty(&self) {
        let mut count = self.count.lock().unwrap_or_else(|error| error.into_inner());
        while *count != 0 {
            count = self
                .settled
                .wait(count)
                .unwrap_or_else(|error| error.into_inner());
        }
    }
}

pub(crate) struct ReadGuard {
    activity: Arc<ReadActivity>,
}

impl Drop for ReadGuard {
    fn drop(&mut self) {
        let mut count = self
            .activity
            .count
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *count -= 1;
        if *count == 0 {
            self.activity.settled.notify_all();
        }
    }
}
