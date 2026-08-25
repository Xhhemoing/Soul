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

use soul_policy::ReasonCode;

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

/// What the endpoint form says about the address typed into it.
///
/// Three claims, and all three are about the core rather than about the page.
/// "只在这次运行里有效" is [`StoredConfig`](crate::commands::session::StoredConfig)
/// having no field an endpoint could be written to, so the next launch starts
/// from a closed guard without anything having to remember to clear one.
/// "填写的时候不会访问这个地址" is
/// [`Session::set_user_endpoint`](crate::commands::session::Session::set_user_endpoint)
/// parsing a string and handing the origin to a `NetGuard`: naming a host is
/// not asking it anything.
///
/// The third claim is *when* the first packet goes out, and it has to name
/// both of the paths a user can trigger, because E1 in PRODUCT_LOCK is
/// 仅用户触发的生成 rather than 起草页.
/// [`Session::generate_draft`](crate::commands::session::Session::generate_draft)
/// is one of them — the approval on the drafting screen after a preparation —
/// and [`Session::person_summary`](crate::commands::session::Session::person_summary)
/// is the other: 人脉图 上「看这个人的摘要」 is one click and one POST, with no
/// second screen in front of it. Naming only the drafting page would leave a
/// user who pointed Soul at a metered address surprised by the graph.
///
/// It travels in [`ConfigSnapshot`] beside the cloud switch's explanation, for
/// the reason that one is there: a promise kept in TypeScript is a promise no
/// Rust test reads.
pub const LLM_ENDPOINT_SESSION_ONLY_NOTICE: &str = "地址只在这次运行里有效，\
    退出 Soul 再打开需要重新填写。填写的时候不会访问这个地址，\
    只有你在起草页确认生成、或在人脉图上看某个人的摘要时才会。";

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
    /// What the user is told about that address: it lives in this process, and
    /// entering it contacts nothing. Fixed for the build, like the cloud
    /// switch's explanation, and carried here for the same reason.
    pub llm_endpoint_notice: String,
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
            llm_endpoint_notice: LLM_ENDPOINT_SESSION_ONLY_NOTICE.to_owned(),
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
        return Err(WizardRefused::not_acknowledged());
    }

    let config = Config::default();
    let open = config.open_capabilities();
    if !open.is_empty() {
        return Err(WizardRefused::capability_left_open(&open));
    }

    Ok(ConfigSnapshot::of(&config))
}

/// What the wizard says when the box at the bottom is not ticked.
pub const WIZARD_NOT_ACKNOWLEDGED_NOTICE: &str = "你还没勾上「我读过上面这几行」，\
    向导就没有可以结束的东西。什么都没有写下，勾上之后再点一次就行。";

/// Why a wizard did not finish, in the shape every other refused command uses.
///
/// A code out of `soul-policy`'s frozen vocabulary and one sentence in Soul's
/// own words — the same two fields as
/// [`SessionRefusal`](crate::commands::session::SessionRefusal), because the
/// shell has one renderer for a refusal and it reads exactly those two off a
/// rejected promise. This used to be an internally tagged enum, so the wizard
/// crossed the IPC as `{"reason": "not_acknowledged"}`: `asRefusal` did not
/// recognize it, and the first screen a new user sees was the one screen whose
/// refusal came out as the code `unavailable` over `[object Object]`.
///
/// Neither code is a word the frozen vocabulary coined for a first-run wizard,
/// and neither reaches the audit chain — the wizard appends nothing. They are
/// borrowed for the screen, and the borrowing is the same one WP11 wrote down:
/// a capability the user never agreed to is `CONSENT_MISSING`, and a step that
/// was simply not taken yet is `ROUTINE`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
#[error("{explanation}")]
pub struct WizardRefused {
    pub reason_code: String,
    pub explanation: String,
}

impl WizardRefused {
    /// The box was not ticked, so there is nothing to finish.
    pub fn not_acknowledged() -> WizardRefused {
        WizardRefused {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: WIZARD_NOT_ACKNOWLEDGED_NOTICE.to_owned(),
        }
    }

    /// The wizard would have handed back a configuration with a switch on.
    ///
    /// The switches are named, because a refusal a user cannot act on is a
    /// wall. They are field names rather than prose for the reason
    /// [`Config::open_capabilities`] returns field names: the sentence has to
    /// stay true when somebody adds a capability, and it is the configuration
    /// that knows what it left open.
    pub fn capability_left_open(open: &[&str]) -> WizardRefused {
        WizardRefused {
            reason_code: ReasonCode::ConsentMissing.as_str().to_owned(),
            explanation: format!(
                "向导交出去的配置必须是什么都没打开的，而这一份里还开着：{}。\
                 向导没有结束，也没有写下任何东西。",
                open.join("、"),
            ),
        }
    }
}
