//! Instruments the acceptance matrix needs, kept out of the shipped crates.
//!
//! Nothing here is a normal dependency of `soulcore`, `soul-schema` or
//! `soul-store-api`. That is what lets this crate own an HTTP stack: AC-11 and
//! AC-12 need a real loopback endpoint that records exactly what a draft
//! request would have contained, and a fake in-process function would not
//! prove anything about the wire.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod crash;
pub mod fixtures;
pub mod leakage;
pub mod mock_llm;

pub use crash::{failpoints, CrashOutcome, CrashScenario};
pub use leakage::{LeakageChecker, LeakageFinding, LeakageKind};
pub use mock_llm::{MockLlm, RecordedRequest};
