//! Bounded research bundles are prepared in memory by the caller. This crate
//! publishes one file without replacing an existing target.
//!
//! Before publication, an exclusive handle is marked for deletion. Closing
//! that handle (including process termination) removes the unfinished file.
//! Publication clears that mark after writing and syncing the complete bytes.
//! This is a process-failure boundary, not a power-loss durability guarantee.
//! If marking a newly created file fails, an empty file may remain; no bytes
//! have been written, and cleanup never deletes a potentially replaced path.

#![deny(unsafe_code)]
#![deny(missing_debug_implementations)]

use std::{io, path::Path, sync::atomic::AtomicBool};

#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
mod windows {
    use super::*;

    #[derive(Debug)]
    pub struct Destination;
    #[derive(Debug)]
    pub struct PendingFile<'a>(pub &'a AtomicBool);

    impl Destination {
        pub fn prepare(_path: &Path) -> io::Result<Self> {
            Err(unsupported())
        }

        pub fn write<'a>(
            &self,
            _bytes: &[u8],
            _cancelled: &'a AtomicBool,
        ) -> io::Result<PendingFile<'a>> {
            Err(unsupported())
        }
    }

    impl PendingFile<'_> {
        pub fn publish(self) -> io::Result<()> {
            let _ = self.0;
            Err(unsupported())
        }
    }

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "research file publication requires Windows",
        )
    }
}

/// A local absolute destination whose existing ancestors are pinned in place.
#[derive(Debug)]
pub struct Destination(windows::Destination);

/// An exclusive, fully written file that will be deleted unless published.
#[derive(Debug)]
pub struct PendingFile<'a> {
    inner: windows::PendingFile<'a>,
}

impl Destination {
    pub fn prepare(path: &Path) -> io::Result<Self> {
        windows::Destination::prepare(path).map(Self)
    }

    pub fn write<'a>(
        &self,
        bytes: &[u8],
        cancelled: &'a AtomicBool,
    ) -> io::Result<PendingFile<'a>> {
        self.0
            .write(bytes, cancelled)
            .map(|inner| PendingFile { inner })
    }
}

impl PendingFile<'_> {
    pub fn publish(self) -> io::Result<()> {
        self.inner.publish()
    }
}
