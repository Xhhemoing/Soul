//! Soul's orchestration core.
//!
//! WP01 ships configuration defaults and nothing else. Every capability is a
//! separate work package, and each one adds its own file under [`commands`]
//! rather than growing a single god module.
//!
//! The normal dependency list is deliberately short: the data contracts and
//! the storage boundary. No HTTP stack, and no test kit.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod collect_probe;
pub mod commands;
pub mod config;
pub mod headless;
pub mod netwatch;

pub use config::{CloudState, Config};
