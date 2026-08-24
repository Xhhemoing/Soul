//! The IPC surface, and nothing else.
//!
//! Every function here is one line long by design: it takes what the WebView
//! sent, hands it to `soulcore::commands::shell`, and returns what came back.
//! No decision is made in this file, so there is nothing in it for a test to
//! catch — which is the point. The decisions have tests, in `soulcore`.

use soulcore::commands::shell::{self, CloudNotice, ConfigSnapshot, WizardAnswers, WizardRefused};
use soulcore::Config;
use tauri::State;

/// The configuration this session is running under.
///
/// `Config::default()` for now. Where a real installation reads it from — and
/// therefore what happens on the second launch — is WP13's question; keeping
/// it in managed state means answering it does not touch this file.
#[derive(Debug, Default)]
pub struct SessionConfig(pub Config);

#[tauri::command]
pub fn config_snapshot(config: State<'_, SessionConfig>) -> ConfigSnapshot {
    ConfigSnapshot::of(&config.0)
}

#[tauri::command]
pub fn complete_wizard(answers: WizardAnswers) -> Result<ConfigSnapshot, WizardRefused> {
    shell::complete_wizard(&answers)
}

#[tauri::command]
pub fn cloud_toggle(config: State<'_, SessionConfig>, requested_on: bool) -> CloudNotice {
    shell::cloud_toggle(&config.0, requested_on)
}

/// The command names the WebView is allowed to call.
///
/// Spelled out so `tests/command_surface.rs` can compare this list against
/// `apps/desktop/src/core.ts`, which is the only place the other side names
/// them. A command added to one and not the other is a failing test.
pub const COMMAND_NAMES: &[&str] = &["config_snapshot", "complete_wizard", "cloud_toggle"];
