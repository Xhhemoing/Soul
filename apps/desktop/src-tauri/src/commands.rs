//! The IPC surface, and nothing else.
//!
//! Every function here is one line long by design: it takes what the WebView
//! sent, hands it to `soulcore::commands::shell`, and returns what came back.
//! No decision is made in this file, so there is nothing in it for a test to
//! catch — which is the point. The decisions have tests, in `soulcore`.

use soulcore::commands::shell::{
    CloudNotice, ConfigSnapshot, RootRefused, Session, WizardAnswers, WizardRefused,
};
use tauri::State;

/// The configuration this session is running under.
///
/// A [`Session`] rather than a bare `Config`, because authorising a directory
/// is a write and managed state in Tauri is shared. The lock lives inside
/// `soulcore`, so this file still holds nothing but a name: where a real
/// installation *reads* the configuration from — and therefore what happens on
/// the second launch — is still WP13's question.
#[derive(Debug, Default)]
pub struct SessionConfig(pub Session);

#[tauri::command]
pub fn config_snapshot(session: State<'_, SessionConfig>) -> ConfigSnapshot {
    session.0.snapshot()
}

#[tauri::command]
pub fn complete_wizard(
    session: State<'_, SessionConfig>,
    answers: WizardAnswers,
) -> Result<ConfigSnapshot, WizardRefused> {
    session.0.complete_wizard(&answers)
}

#[tauri::command]
pub fn cloud_toggle(session: State<'_, SessionConfig>, requested_on: bool) -> CloudNotice {
    session.0.cloud_toggle(requested_on)
}

#[tauri::command]
pub fn authorize_root(
    session: State<'_, SessionConfig>,
    path: String,
) -> Result<ConfigSnapshot, RootRefused> {
    session.0.authorize_root(&path)
}

#[tauri::command]
pub fn authorized_roots(session: State<'_, SessionConfig>) -> Vec<String> {
    session.0.authorized_roots()
}

/// The command names the WebView is allowed to call.
///
/// Spelled out so `tests/command_surface.rs` can compare this list against
/// `apps/desktop/src/core.ts`, which is the only place the other side names
/// them. A command added to one and not the other is a failing test.
pub const COMMAND_NAMES: &[&str] = &[
    "config_snapshot",
    "complete_wizard",
    "cloud_toggle",
    "authorize_root",
    "authorized_roots",
];
