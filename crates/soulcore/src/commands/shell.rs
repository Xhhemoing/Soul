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
//!
//! Two things arrived later and follow the same rule. [`Session`] holds the
//! configuration the running shell is under, so authorising a directory is a
//! write the core performs and the WebView only asks for; and [`KeyProtection`]
//! carries where the database key actually came from, because "your key is
//! protected" is a claim about the operating system and `docs/SECURITY.md`
//! forbids making it before DPAPI exists.
//!
//! The two views WP09 grew later — drafting and the file plan — refuse in this
//! module's vocabulary rather than each inventing their own. [`ViewRefused`]
//! is the shape [`RootRefused`] already established, and [`wall_clock`] is
//! here so a host function reads the clock inside this crate: the command
//! layer above is one line per command by construction, and a line that reads
//! a clock is a line that decides what "now" means.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

/// What the settings page says when the store was opened with a plain key file.
///
/// Every clause is checkable against `crates/soul-store/src/keys.rs`:
/// `TestKeyProvider::in_dir` writes a 64-byte seed to `soul-test-keys.bin` with
/// no wrapping at all, and both `DpapiKeyProvider` accessors return
/// `KeyError::Unsupported`. The sentence lives here rather than in the WebView
/// for the same reason the cloud notice does — a promise kept in TypeScript is
/// a promise `cargo test` cannot check.
pub const KEY_FILE_NOT_PROTECTED_EXPLANATION: &str =
    "数据库的密钥现在放在数据目录里的一个明文文件 soul-test-keys.bin，没有交给 Windows 的 \
     DPAPI —— 那一段还没有实现。所以能读到这个目录的人，就能打开你的库：在 DPAPI 补上之前，\
     这台电脑的登录口令是唯一的一道门。";

/// What it says before any database has been opened.
pub const NO_STORE_OPENED_EXPLANATION: &str =
    "本次会话还没有打开本机数据库，因此还没有任何密钥落到磁盘上。";

/// What it would say if a platform key store answered.
///
/// No v0.1 build reaches this line. It is written down anyway so the fallback
/// above is visibly a fallback, and so the day DPAPI lands the change is one
/// arm of one match rather than a new sentence someone invents in a hurry.
pub const PLATFORM_KEY_STORE_EXPLANATION: &str =
    "数据库的密钥由平台密钥库保管，不以明文落在数据目录里。";

/// Where the database key came from, as observed when the store was opened.
///
/// Not a setting, not a preference, and not something a [`Config`] can change:
/// the only way to obtain anything but [`KeyProtection::NoStoreOpened`] is for
/// `soulcore::commands::store::open_store_for_session` to have watched a key
/// provider answer. That is what keeps the shell from being able to claim
/// protection it does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyProtection {
    /// No database was opened — the mock runtime, and every core-side test
    /// that is about the configuration rather than the disk.
    #[default]
    NoStoreOpened,
    /// A plain seed file sits next to the database. Nothing wraps it.
    UnprotectedKeyFile,
    /// A platform key store supplied the key. Unreachable in v0.1.
    PlatformKeyStore,
}

impl KeyProtection {
    /// Whether the key that wraps every content key is held by the platform.
    ///
    /// False in every v0.1 build, and false for every configuration: the value
    /// is derived from the provider that answered, so there is no switch to
    /// leave in the wrong position.
    pub fn kek_protected(self) -> bool {
        matches!(self, KeyProtection::PlatformKeyStore)
    }

    /// The sentence the shell renders, verbatim.
    pub fn explanation(self) -> &'static str {
        match self {
            KeyProtection::NoStoreOpened => NO_STORE_OPENED_EXPLANATION,
            KeyProtection::UnprotectedKeyFile => KEY_FILE_NOT_PROTECTED_EXPLANATION,
            KeyProtection::PlatformKeyStore => PLATFORM_KEY_STORE_EXPLANATION,
        }
    }
}

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
    /// Whether a platform key store holds the key that wraps the content keys.
    /// Always false in v0.1, and no configuration can make it true.
    pub kek_protected: bool,
    /// The core's own sentence about where the database key lives. The shell
    /// renders it and does not paraphrase it.
    pub key_protection: String,
}

impl ConfigSnapshot {
    /// For callers with no store: the mock runtime, and the tests that are
    /// about a configuration rather than about a disk.
    pub fn of(config: &Config) -> ConfigSnapshot {
        ConfigSnapshot::of_session(config, KeyProtection::NoStoreOpened)
    }

    pub fn of_session(config: &Config, keys: KeyProtection) -> ConfigSnapshot {
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
            kek_protected: keys.kek_protected(),
            key_protection: keys.explanation().to_owned(),
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
    Ok(ConfigSnapshot::of(&wizard_config(answers)?))
}

/// The configuration a finished wizard hands over.
///
/// Split out from [`complete_wizard`] so [`Session`] can adopt the same value
/// it returns. Both callers get the same two refusals, which is the point:
/// there is one wizard, and it is this function.
fn wizard_config(answers: &WizardAnswers) -> Result<Config, WizardRefused> {
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

    Ok(config)
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

/// Authorise a directory for read-only scanning.
///
/// Every check here is one the WebView must not be trusted with, and the order
/// matters: the path is resolved *before* it is compared with anything, so a
/// second spelling of a directory that is already authorised — a trailing
/// separator, a `..` hop, a symbolic link — is a duplicate rather than a
/// second entry. WP11 refuses anything outside this list, and it can only be
/// as trustworthy as the list is canonical.
///
/// Authorising is the one write in the shell's command surface. It is not
/// persisted: whether the list survives a restart is WP13's question, and the
/// settings page says so in as many words.
pub fn authorize_root(config: &mut Config, requested: &str) -> Result<PathBuf, RootRefused> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Err(RootRefused::new(
            RootRefusedReason::Empty,
            "还没有填目录。请写一个完整路径，比如 C:\\Users\\你\\Documents。",
        ));
    }

    let metadata = std::fs::metadata(Path::new(requested)).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            RootRefused::new(
                RootRefusedReason::NotFound,
                format!("这个路径不存在：{requested}"),
            )
        } else {
            RootRefused::new(
                RootRefusedReason::Unreadable,
                format!("读不到这个路径：{requested}（{error}）"),
            )
        }
    })?;

    if !metadata.is_dir() {
        return Err(RootRefused::new(
            RootRefusedReason::NotADirectory,
            format!("这是一个文件，不是目录：{requested}"),
        ));
    }

    let canonical = std::fs::canonicalize(Path::new(requested)).map_err(|error| {
        RootRefused::new(
            RootRefusedReason::Unreadable,
            format!("这个路径解析不出来：{requested}（{error}）"),
        )
    })?;

    if config.authorized_roots.contains(&canonical) {
        return Err(RootRefused::new(
            RootRefusedReason::AlreadyAuthorized,
            format!("这个目录已经授权过了：{}", canonical.display()),
        ));
    }

    config.authorized_roots.push(canonical.clone());
    Ok(canonical)
}

/// The authorised directories, in the form the user can read back.
///
/// The paths are the user's own — they typed them — so echoing them is not a
/// leak. What the shell still never receives is anything *inside* them.
pub fn authorized_roots(config: &Config) -> Vec<String> {
    config
        .authorized_roots
        .iter()
        .map(|root| root.display().to_string())
        .collect()
}

/// Why a directory was not authorised.
///
/// The sentence travels with the code. A WebView that had to compose the
/// message would be deciding what the refusal meant, and "the path does not
/// exist" versus "the path is a file" is exactly the distinction a hurried UI
/// collapses into "无效路径".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
#[error("{message}")]
pub struct RootRefused {
    pub reason: RootRefusedReason,
    pub message: String,
}

impl RootRefused {
    fn new(reason: RootRefusedReason, message: impl Into<String>) -> RootRefused {
        RootRefused {
            reason,
            message: message.into(),
        }
    }
}

/// What a view says when this process has no database open.
///
/// Every clause is checkable. Both host functions write to the audit chain
/// through the crate underneath them, and the chain lives in the store, so
/// there is nowhere to record the request and therefore nothing honest to do
/// but refuse. A real installation opens the database while it starts up; the
/// mock runtime the desktop tests use opens nothing at all, which is what
/// makes this the sentence those tests see.
pub const NO_STORE_FOR_VIEW_EXPLANATION: &str =
    "本次会话没有打开本机数据库。起草和文件计划都要把这次请求写进审计链，没有库就没有地方写，\
     所以这里只能拒绝。真机在启动时打开它；测试用的模拟运行时不开库，看到的就是这句话。";

/// What a view says when the configured endpoint cannot be parsed.
///
/// A fixed sentence rather than the parse error's own text: `OriginError`
/// repeats the URL it was handed, and the endpoint the user typed has no
/// business in a value that crosses the IPC boundary. Unreachable from the
/// desktop, which has no way to write one; it is here because the branch that
/// would need it must not be the branch that invents a sentence.
pub const ENDPOINT_UNUSABLE_EXPLANATION: &str =
    "配置里的模型端点解析不出来，所以这次没有继续。请到设置页把它改成一个完整的地址。";

/// Why a view has nothing to show.
///
/// Same shape as [`RootRefused`], for the same reason: the WebView renders
/// `message` and does not compose one. `code` is the audit chain's own reason
/// code where the refusal had one — `PATH_NOT_AUTHORIZED` and the rest — so a
/// caller can branch on the decision without parsing a sentence, and `None`
/// where the failure was not a policy decision at all.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct ViewRefused {
    pub reason: ViewRefusedReason,
    pub code: Option<String>,
    pub message: String,
}

impl ViewRefused {
    /// No database is open in this process; see [`NO_STORE_FOR_VIEW_EXPLANATION`].
    pub(crate) fn no_store_opened() -> ViewRefused {
        ViewRefused {
            reason: ViewRefusedReason::NoStoreOpened,
            code: None,
            message: NO_STORE_FOR_VIEW_EXPLANATION.to_owned(),
        }
    }

    /// The user pressed the button with nothing in the box.
    pub(crate) fn empty_paste(message: impl Into<String>) -> ViewRefused {
        ViewRefused {
            reason: ViewRefusedReason::EmptyPaste,
            code: None,
            message: message.into(),
        }
    }

    /// The core was asked and said no. `message` is the refusal's own `Display`
    /// text, never a sentence this layer assembled: the errors underneath are
    /// written to name what the user can act on and to leave out what they
    /// asked about, and re-wording one here would lose both properties.
    pub(crate) fn refused(code: Option<ReasonCode>, message: impl Into<String>) -> ViewRefused {
        ViewRefused {
            reason: ViewRefusedReason::Refused,
            code: code.map(|code| code.as_str().to_owned()),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewRefusedReason {
    /// Nothing is open, so nothing can be recorded. Not the user's doing.
    NoStoreOpened,
    /// The paste box was empty, or held only whitespace.
    EmptyPaste,
    /// A gate said no: policy, authorization, or the store itself.
    Refused,
}

/// Now, in the two units the command surface counts in.
///
/// Milliseconds for the token and action clocks, whole seconds for the audit
/// entry. Both come from one reading so the record and the decision cannot
/// describe two different moments. A clock before the epoch yields zero rather
/// than a panic: an unset system clock is a reason to record a wrong time, not
/// a reason to refuse a draft the user is waiting for.
pub fn wall_clock() -> (u64, i64) {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (
        u64::try_from(since_epoch.as_millis()).unwrap_or(u64::MAX),
        i64::try_from(since_epoch.as_secs()).unwrap_or(i64::MAX),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootRefusedReason {
    /// Nothing was typed.
    Empty,
    NotFound,
    NotADirectory,
    /// It exists, but the process cannot resolve or stat it.
    Unreadable,
    AlreadyAuthorized,
}

/// The configuration a running shell is under, plus how its key is held.
///
/// One lock over both, not two: every snapshot reads them together, and two
/// locks is how the count of authorised directories and the sentence about key
/// material start describing different moments.
///
/// Nothing here is written to disk. The shell's managed state is the whole
/// lifetime of an authorisation in v0.1, which is a limitation the settings
/// page states rather than hides.
#[derive(Debug, Default)]
pub struct Session {
    state: Mutex<SessionState>,
}

#[derive(Debug)]
struct SessionState {
    config: Config,
    keys: KeyProtection,
    draft_profile_id: Uuid,
}

/// Written out rather than derived because of the third field.
///
/// The draft view needs a profile to read a voice from, and `soul-profile`
/// answers for an unknown identifier with a blank profile — five unknown axes
/// and a neutral voice — so one minted here is enough to draft with and needs
/// no questionnaire behind it. Minting it once per session is what makes two
/// drafts in one run read the same voice; minting it per draft would silently
/// discard anything the user had set.
impl Default for SessionState {
    fn default() -> SessionState {
        SessionState {
            config: Config::default(),
            keys: KeyProtection::default(),
            draft_profile_id: Uuid::now_v7(),
        }
    }
}

impl Session {
    pub fn new() -> Session {
        Session::default()
    }

    /// Record which provider opened the store. The host calls this once, from
    /// its startup, with a value `commands::store` observed — there is no way
    /// in from outside to assert a protection the store did not report.
    pub fn opened_store_with(&self, keys: KeyProtection) {
        self.lock().keys = keys;
    }

    pub fn key_protection(&self) -> KeyProtection {
        self.lock().keys
    }

    /// The configuration itself, for the host functions that need more of it
    /// than a [`ConfigSnapshot`] carries — the authorised roots as paths, and
    /// whether an endpoint is configured.
    ///
    /// A clone taken under the one lock, so what a caller works from is one
    /// moment rather than a borrow it holds while the user changes something.
    /// It stays inside Rust: `llm_endpoint` is in here, and no DTO in this
    /// crate has a field it could reach.
    pub fn config(&self) -> Config {
        self.lock().config.clone()
    }

    /// The profile the views draft in the voice of. Minted once when the
    /// session is created and stable for as long as it lives, so two drafts in
    /// one run read the same voice.
    pub fn draft_profile_id(&self) -> Uuid {
        self.lock().draft_profile_id
    }

    pub fn snapshot(&self) -> ConfigSnapshot {
        let state = self.lock();
        ConfigSnapshot::of_session(&state.config, state.keys)
    }

    pub fn cloud_toggle(&self, requested_on: bool) -> CloudNotice {
        cloud_toggle(&self.lock().config, requested_on)
    }

    /// Finish the first-run wizard and adopt what it returned.
    ///
    /// Adopting matters: the wizard's promise is that everything is off when
    /// it is done, and a session that kept an older configuration would make
    /// the returned snapshot a description of nothing.
    pub fn complete_wizard(
        &self,
        answers: &WizardAnswers,
    ) -> Result<ConfigSnapshot, WizardRefused> {
        let mut state = self.lock();
        state.config = wizard_config(answers)?;
        Ok(ConfigSnapshot::of_session(&state.config, state.keys))
    }

    pub fn authorize_root(&self, requested: &str) -> Result<ConfigSnapshot, RootRefused> {
        let mut state = self.lock();
        authorize_root(&mut state.config, requested)?;
        Ok(ConfigSnapshot::of_session(&state.config, state.keys))
    }

    pub fn authorized_roots(&self) -> Vec<String> {
        authorized_roots(&self.lock().config)
    }

    /// A poisoned lock must not turn the settings page into a dead end. What
    /// is behind it is a configuration, not a half-written invariant, and the
    /// honest recovery is to carry on reading it.
    fn lock(&self) -> MutexGuard<'_, SessionState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
