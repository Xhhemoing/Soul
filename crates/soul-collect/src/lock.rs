//! Locking that survives a poisoned mutex.
//!
//! The collector runs on a background thread and shares its store, its consent
//! ledger and its stop flag with the caller. If any thread panics while holding
//! one of those locks, the standard library hands every later caller a
//! `PoisonError` — and the collector's answer to that must not be a second
//! panic on the stop path, because stopping is the one thing the user is
//! promised will work.
//!
//! The data behind these locks is a store handle, a consent ledger and a bool.
//! None of them can be left half-updated by a panic in a way a later reader
//! would misread, so recovering the guard is the honest response.

use std::sync::{Mutex, MutexGuard};

pub fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
