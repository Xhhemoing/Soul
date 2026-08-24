//! WP09's command surface: what the desktop shell is allowed to know.
//!
//! The shell renders three things in this work package — the first-run wizard,
//! a configuration summary, and the cloud switch — and none of the three may
//! decide anything. So the decisions are here, in Rust, where `cargo test` can
//! see them, and the WebView only ever receives a value it renders verbatim.
//!
//! Two invariants are worth naming, because they are the ones a UI would
//! erode first:
//!
//! * [`complete_wizard`] refuses to hand back a configuration that has a
//!   capability switched on. AC-02 says collection, the cloud and the model
//!   endpoint are off once the wizard is done; a wizard that merely *happens*
//!   to leave them off satisfies the test today and stops satisfying it the
//!   first time someone adds a checkbox.
//! * [`cloud_toggle`] takes the user's request and returns the same notice for
//!   either answer. v0.1 has no non-loopback code path, so the honest response
//!   to "turn the cloud on" is a sentence, not a state change.

use serde::{Deserialize, Serialize};

use crate::config::{CloudState, Config};

/// The executable's file stem, from PRODUCT_LOCK: the process is `soul.exe`
/// on Windows, and the orchestration crate underneath it is `soulcore`.
///
/// `apps/desktop/src-tauri` asserts its own `CARGO_BIN_NAME` against this, so
/// renaming the binary breaks a test rather than quietly shipping a process
/// the lock document does not name.
pub const DESKTOP_BINARY_NAME: &str = "soul";

/// The word the cloud switch shows, in every state it has.
pub const CLOUD_NOT_YET_AVAILABLE_LABEL: &str = "尚未启用";

/// Why the switch says that, for the line underneath it.
pub const CLOUD_NOT_YET_AVAILABLE_EXPLANATION: &str =
    "v0.1 没有云端出网的代码路径。开关留在这里是为了让你看见它默认是关的，\
     点它不会发出任何请求，也不会把任何内容送出本机。";

/// What the shell may display about the current configuration.
///
/// Not [`Config`] itself: the endpoint the user typed is their own business
/// and the shell has no view in v0.1 that needs it, so this carries whether
/// one is configured rather than what it is. Everything else is a count or a
/// flag, which is also what makes it safe to hand to a WebView.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSnapshot {
    pub collect_enabled: bool,
    pub cloud: CloudNotice,
    /// True once the user has entered their own OpenAI-compatible endpoint.
    /// The URL itself is not part of this value.
    pub llm_endpoint_configured: bool,
    pub authorized_root_count: usize,
    /// Nothing is switched on. A fresh install and a finished wizard must both
    /// be able to say this.
    pub fully_closed: bool,
    /// Named switches that are on, so a refusal reads as a sentence.
    pub open_capabilities: Vec<String>,
}

impl ConfigSnapshot {
    pub fn of(config: &Config) -> ConfigSnapshot {
        ConfigSnapshot {
            collect_enabled: config.collect_enabled,
            cloud: CloudNotice::of(config),
            llm_endpoint_configured: config.llm_endpoint.is_some(),
            authorized_root_count: config.authorized_roots.len(),
            fully_closed: config.is_fully_closed(),
            open_capabilities: config
                .open_capabilities()
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }
}

/// Everything the cloud switch renders.
///
/// The label and the explanation travel with the state instead of living in
/// the WebView, because "尚未启用" is a promise about what the build does, and
/// a promise kept in TypeScript is a promise the Rust tests cannot check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudNotice {
    pub state: CloudState,
    pub label: String,
    /// Always false in v0.1, and there is no code path that sets it.
    pub enabled: bool,
    /// Always false: acting on this notice sends nothing anywhere.
    pub performs_network_request: bool,
    pub explanation: String,
}

impl CloudNotice {
    fn of(config: &Config) -> CloudNotice {
        CloudNotice {
            state: config.cloud_state,
            enabled: config.cloud_enabled,
            label: CLOUD_NOT_YET_AVAILABLE_LABEL.to_owned(),
            performs_network_request: false,
            explanation: CLOUD_NOT_YET_AVAILABLE_EXPLANATION.to_owned(),
        }
    }
}

/// The user pressed the cloud switch.
///
/// `requested_on` is recorded in the signature rather than ignored silently,
/// so the caller can see that the answer does not depend on it. Nothing is
/// written, nothing is opened, and no socket exists to be reached from here:
/// this function's whole body is a description of the build.
pub fn cloud_toggle(config: &Config, requested_on: bool) -> CloudNotice {
    let _ = requested_on;
    CloudNotice::of(config)
}

/// Read the current configuration for display.
///
/// v0.1 has no configuration file yet — WP13 decides where it lives — so this
/// is the default. The shell calls it rather than constructing a snapshot of
/// its own, which is the whole point: the WebView never assembles state.
pub fn config_snapshot() -> ConfigSnapshot {
    ConfigSnapshot::of(&Config::default())
}

/// What the first-run wizard asks for.
///
/// One field, and it is not a permission. PRODUCT_LOCK says the wizard must
/// not open with a wall of prompts, and every capability in v0.1 defaults off,
/// so there is nothing here for the user to switch on — only a statement that
/// they read what the defaults are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WizardAnswers {
    pub acknowledged_defaults_are_off: bool,
}

/// Finish the first-run wizard.
///
/// Returns the configuration the user now has, which must be the fully closed
/// one. The second check looks redundant next to [`Config::default`] — it is
/// not: it is the assertion that survives someone adding a capability to the
/// wizard later.
pub fn complete_wizard(answers: &WizardAnswers) -> Result<ConfigSnapshot, WizardRefused> {
    if !answers.acknowledged_defaults_are_off {
        return Err(WizardRefused::NotAcknowledged);
    }

    let config = Config::default();
    let open = config.open_capabilities();
    if !open.is_empty() {
        return Err(WizardRefused::CapabilityLeftOpen {
            open: open.into_iter().map(str::to_owned).collect(),
        });
    }

    Ok(ConfigSnapshot::of(&config))
}

/// Why a wizard did not finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case", tag = "reason")]
pub enum WizardRefused {
    #[error("the wizard was not acknowledged, so there is nothing to finish")]
    NotAcknowledged,
    #[error("the wizard would have left these on, and AC-02 says they are off: {}", open.join(", "))]
    CapabilityLeftOpen { open: Vec<String> },
}
