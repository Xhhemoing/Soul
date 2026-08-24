//! Foreground application duration, and nothing else about the screen.
//!
//! PRODUCT_LOCK is unusually specific about this work package: "电脑采集 v0.1
//! 仅前台应用使用时长。窗口标题不采。文件元数据推迟 v0.1.1。无键盘记录、无
//! 密码框", and "采集默认关". Everything in this crate follows from those two
//! sentences.
//!
//! The shape:
//!
//! ```text
//!   ForegroundSource  ->  ConsentHandle  ->  Collector  ->  EventStore
//!   (Windows / fake)      (soul-policy)      (sessions)     (soul-store)
//! ```
//!
//! * [`ForegroundSource`] is the seam. Windows implements it against the real
//!   desktop in `src/windows.rs`; CI implements it with
//!   [`FakeForegroundSource`]. There is no third path into the collector, so a
//!   test on a Linux host exercises the same gate and the same writes as a
//!   session on a real machine.
//! * [`ConsentHandle`] is the gate, and it is checked twice per written event:
//!   once before the sample and once in the instant before the store is
//!   touched. Nothing here can be switched on except through
//!   [`soul_policy::consent`], which grants nothing by default.
//! * [`Collector`] turns samples into sessions and sessions into events, with
//!   the application name and the duration sealed under one content key so
//!   forgetting can reach them.
//! * [`runner::start`] runs all of that on a background thread that stops
//!   inside [`runner::STOP_BUDGET`].
//!
//! What is deliberately absent: any call that reads a window caption, any
//! keyboard or mouse hook, any file metadata, any network. The window-title
//! promise is checked by a test that reads these sources back, because a
//! promise about what code does *not* do cannot be demonstrated by running it.

#![cfg_attr(not(windows), forbid(unsafe_code))]
#![cfg_attr(windows, deny(unsafe_code))]
#![deny(missing_debug_implementations)]

pub mod collector;
pub mod consent;
pub mod error;
pub mod fake;
mod lock;
pub mod runner;
pub mod session;
pub mod source;

#[cfg(windows)]
pub mod windows;

pub use collector::{CollectSink, Collector, Poll, Tally};
pub use consent::{ConsentHandle, COLLECTION_TOPIC};
pub use error::{CollectError, CollectResult};
pub use fake::FakeForegroundSource;
pub use runner::{
    start, CollectorConfig, CollectorHandle, CollectorReport, StopReason, DEFAULT_POLL_INTERVAL,
    STOP_BUDGET,
};
pub use session::{ForegroundSession, SealedSessionBody, BODY_FIELD};
pub use source::{platform_source, AppIdentity, ForegroundSource, SourceError};
