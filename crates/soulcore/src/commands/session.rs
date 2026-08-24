//! WP13's second slice: the one store a running Soul opens, and the small
//! file beside it that remembers what the user already agreed to.
//!
//! Everything above this module has been able to assume somebody else was
//! holding an open [`SqlCipherStore`]. Nobody was: the desktop shell carried an
//! in-memory [`Config`] and every route that needed rows was empty. This is
//! where the handle comes from, and there is exactly one of it — WP07 leftover
//! 8 is blunt about why. Two connections to the same SQLCipher database are
//! two write-ahead logs, and the second one to be opened wins arguments it was
//! never told it was having.
//!
//! ## What lives on disk, and what deliberately does not
//!
//! Next to `soul.db` sits [`CONFIG_FILE_NAME`], and it has two fields:
//! whether the first-run wizard finished, and which directories the user
//! authorized for read-only scanning. That is the whole file.
//!
//! There is no field in it for collection, for the cloud, or for a model
//! endpoint, and the absence is the point. AC-02 says those three are off
//! after the wizard; a file that could say otherwise is a file that could be
//! edited, corrupted, or written by a future version into a state the user
//! never chose. [`StoredConfig`] is `deny_unknown_fields`, so a file that
//! names one of them does not load at all — the session reports the problem
//! and runs closed. "A restart cannot reopen anything" is therefore a
//! statement about the shape of the file rather than about the care taken by
//! the code that reads it.
//!
//! No memory, no draft and no scan result is written here. The store is the
//! place for content; this file holds two answers the user gave the shell.
//!
//! ## Key material
//!
//! [`key_provider`] picks the platform one. On Windows that is
//! `DpapiKeyProvider`, which — see `docs/SECURITY.md` — protects the KEK with
//! `CryptProtectData` under the logged-in user and keeps the database key
//! wrapped under it in `keys.dpapi` beside the database. Anywhere else, which
//! for this product means a developer machine or Linux CI, it is
//! `TestKeyProvider` against a seed file in the same directory, and
//! [`KeyProtection`] carries that fact to the interface so nothing on screen
//! can claim a protection this build does not have.
//!
//! Either way the store may still fail to open — a Windows account with no
//! loaded user profile has no DPAPI master key to protect anything with — and
//! that is a state the session reports rather than works around. There is no
//! fallback path from the Windows provider to the developer one; a build that
//! quietly downgraded its key protection would be worse than one that did not
//! start.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_fileplan::{Refusal, ScanLimits};
use soul_policy::clock::{now_unix_millis, now_unix_seconds, rfc3339_utc};
use soul_policy::hitl::{RequestOrigin, TokenIssuer};
use soul_policy::ReasonCode;
use soul_store::SqlCipherStore;
use soul_store_api::forget::ForgetImpact;

use crate::commands::collect::{
    self as collect_commands, share, CollectorConfig, CollectorHandle, ConsentHandle,
    ForegroundSource, SourceError, COLLECTION_TOPIC,
};
use crate::commands::draft::{
    self, Approval, DraftRefusal, DraftSession, DraftValue, E1DraftPlan, PersonSummaryView,
};
use crate::commands::fileplan::{
    AuthorizedRootView, FilePlanSession, PlanPreview, READ_ONLY_NOTICE,
};
use crate::commands::graph::{self as graph_commands, PeopleGraphView};
use crate::commands::import::{
    self as import_commands, ImportPreview, ImportReceiptView, IMPORT_REFUSED_NOTICE,
};
use crate::commands::memory::{
    self as memory_commands, ForgetConfirmation, ForgetPreview, ForgetReceiptView, MemoryChange,
    MemoryDetail, MemoryList, NewMemory,
};
use crate::commands::policy::PolicySession;
use crate::commands::profile::{
    self as profile_commands, GivenAnswer, IntakeReceipt, ProfileScreen, QuestionView,
};
use crate::commands::shell::{self, CloudNotice, ConfigSnapshot, WizardAnswers, WizardRefused};
use crate::commands::store::{self as store_commands, AuditChainView, ResearchPreviewView};
use crate::config::Config;

/// The environment variable that names the data directory outright.
///
/// For CI and for tests, which must not write into the directory a real
/// installation uses. It is read before the platform rules rather than after
/// so that a test cannot half-escape them.
pub const DATA_DIRECTORY_OVERRIDE: &str = "SOUL_DATA_DIR";

/// The two answers that survive a restart. See the module docs.
pub const CONFIG_FILE_NAME: &str = "config.json";

/// Where the DPAPI-protected seed goes on Windows, per `docs/SECURITY.md`.
pub const KEY_BLOB_FILE_NAME: &str = "keys.dpapi";

/// `%LOCALAPPDATA%\Soul`, as PRODUCT_LOCK names it.
pub const WINDOWS_DIRECTORY_NAME: &str = "Soul";

/// The same directory anywhere else, spelled the way that platform spells
/// application data directories.
pub const UNIX_DIRECTORY_NAME: &str = "soul";

/// What the interface is told when the store did not open.
pub const STORE_UNAVAILABLE_NOTICE: &str =
    "这台机器上的加密库没有打开，所以要读库的页面暂时没有内容可显示。";

/// The profile this installation keeps.
///
/// A constant rather than a row somebody looks up, because Soul is one
/// person's copy of themselves and `ProfileStore` has no way to list profiles
/// — a generated id would have to be written down somewhere, and the two
/// places it could go are both worse. In `config.json` it would be a third
/// field on a file whose whole argument is that it has two; in the store it
/// would be unreadable exactly when the store is what failed to open. Fixing
/// it is the same choice `soul-profile` made for `axis_id`, for the same
/// reason: an identifier that changes is a history that forks.
pub const OWNER_PROFILE_ID: Uuid = Uuid::from_u128(0x0192b0c0_5001_7c01_8c01_000000000001);

/// How many rows the research preview shows. It is a sample, not an export.
pub const RESEARCH_PREVIEW_ROWS: usize = 50;

/// What the wizard is told when it hands in a questionnaire with nothing in it.
pub const NO_ANSWERS_NOTICE: &str = "这份问卷一道题都没有答，所以没有东西可以写进档案。\
    随便答一道都行，没答的那些会留成「还看不出方向」，不会被猜。";

/// What a forget is told when the preview it echoes is not the one on screen.
pub const FORGET_NOT_PREVIEWED_NOTICE: &str = "这次遗忘对不上你刚才看过的那份影响面预览。\
    什么都没有销毁：先看一遍这条记忆现在的预览，再决定。";

/// What collection is, stated once, on the page that offers it.
///
/// PRODUCT_LOCK's sentence for this slice — 仅前台应用使用时长，窗口标题不采 —
/// as something the core says rather than something the interface promises on
/// its behalf. It is here for the same reason [`READ_ONLY_NOTICE`] is in
/// `fileplan.rs`: a promise kept in TypeScript is a promise no Rust test reads.
pub const COLLECT_DURATION_ONLY_NOTICE: &str = "这一版的采集只记一样东西：\
    哪个应用在前台，以及它在前台待了多久。窗口标题不记，文件内容不记，\
    键盘和剪贴板连代码路径都没有。应用名和时长一起密封在库里，跟着这一次采集的内容密钥走。";

/// What the collection page says while the gate is shut.
pub const COLLECT_OFF_NOTICE: &str = "采集现在是关的：没有给出同意，也没有采集线程在跑。";

/// And while it is open and a collector is running.
pub const COLLECT_RUNNING_NOTICE: &str = "采集正在进行：后台线程在按秒看前台是哪个应用，\
    换了应用就把上一段的时长写成一条记录。按「停止采集」之后 1 秒内不会再有新的记录。";

/// Consent is granted and nothing is watching. On Linux and on a developer
/// machine that is the normal outcome, and saying so is the answer: a build
/// that reported "采集已打开" while sampling nothing would be the worst of the
/// three states.
pub const COLLECT_NOT_OBSERVING_NOTICE: &str = "同意已经记下来了，但这台机器上没有东西在采：\
    v0.1 只在 Windows 上看前台，别的平台上给出的同意就只是同意，不会去看任何窗口。";

/// The `source` a machine with no foreground collector reports.
pub const NO_FOREGROUND_SOURCE: &str = "unsupported";

/// What the endpoint form is told when what it was given is not an address.
///
/// [`OriginError`](soul_policy::net_guard::OriginError) names the string it
/// refused in every one of its variants, and that is the one thing this
/// sentence must not do. A user who pasted a key into the address bar by
/// mistake — or who typed the `user:password@host` form the parser rejects
/// outright — would have it read back to them on screen, and from a screen it
/// goes into a screenshot. So the refusal describes what an address has to
/// look like, and the field keeps whatever they typed for them to fix.
pub const ENDPOINT_UNPARSABLE_NOTICE: &str = "这个地址不像一个端点：要 http:// 或 https:// 开头，\
    后面跟主机名，端口不写就按 80 或 443 算，比如 http://127.0.0.1:11434/v1；\
    地址里不能带用户名和密码。这一次什么都没有保存，端点还是没有填写。";

/// A collector that would not wind down. Rare enough to be worth a sentence.
pub const COLLECT_NOT_STOPPED_NOTICE: &str = "同意已经收回，但采集线程没有正常收尾：";

/// Where this machine keeps Soul's data.
pub fn data_directory() -> Result<PathBuf, DirectoryError> {
    if let Some(named) = non_empty_var(DATA_DIRECTORY_OVERRIDE) {
        return Ok(PathBuf::from(named));
    }
    if cfg!(windows) {
        return non_empty_var("LOCALAPPDATA")
            .map(|base| PathBuf::from(base).join(WINDOWS_DIRECTORY_NAME))
            .ok_or(DirectoryError::Undiscoverable {
                variables: &[DATA_DIRECTORY_OVERRIDE, "LOCALAPPDATA"],
            });
    }
    if let Some(base) = non_empty_var("XDG_DATA_HOME") {
        return Ok(PathBuf::from(base).join(UNIX_DIRECTORY_NAME));
    }
    non_empty_var("HOME")
        .map(|home| {
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join(UNIX_DIRECTORY_NAME)
        })
        .ok_or(DirectoryError::Undiscoverable {
            variables: &[DATA_DIRECTORY_OVERRIDE, "XDG_DATA_HOME", "HOME"],
        })
}

fn non_empty_var(name: &str) -> Option<OsString> {
    std::env::var_os(name).filter(|value| !value.is_empty())
}

/// No directory could be worked out from the environment.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DirectoryError {
    #[error("none of these say where Soul's data lives: {}", variables.join(", "))]
    Undiscoverable { variables: &'static [&'static str] },
}

/// Which key store the database key came out of.
///
/// Carried to the interface so that a build using developer key material
/// cannot be mistaken for one that is protected by the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyProtection {
    /// Windows DPAPI, protecting the KEK under the logged-in user. The chain
    /// below it is `docs/SECURITY.md`'s: KEK wraps the database key and every
    /// content key.
    Dpapi,
    /// A seed file beside the database, with no platform protection at all.
    DeveloperKeyFile,
}

impl KeyProtection {
    pub const fn as_str(self) -> &'static str {
        match self {
            KeyProtection::Dpapi => "dpapi",
            KeyProtection::DeveloperKeyFile => "developer_key_file",
        }
    }

    /// What the user is told about this machine's key material.
    pub const fn notice(self) -> &'static str {
        match self {
            KeyProtection::Dpapi => "数据库密钥由 Windows 的用户级密钥保护接管。",
            KeyProtection::DeveloperKeyFile => {
                "这是开发构建：数据库密钥放在库旁边的种子文件里，没有平台密钥保护。"
            }
        }
    }
}

/// The platform's key provider, and the name of what it is.
///
/// `cfg!` rather than `#[cfg]`: both providers exist on every platform —
/// `DpapiKeyProvider` compiles anywhere and refuses everywhere it has no
/// `CryptProtectData` to call — so writing the choice as a runtime constant
/// means the Windows arm is type-checked by the Linux build too. With
/// `#[cfg]` it would only ever be compiled on the platform nobody develops on,
/// which is where a rename goes unnoticed until CI.
///
/// The branch is one-way. Nothing below it may fall back from `Dpapi` to
/// `DeveloperKeyFile`: a Windows machine whose DPAPI is unavailable has to be
/// told so, not handed an unprotected seed file with the same database
/// behind it.
fn key_provider(directory: &Path) -> (Box<dyn soul_store::KeyProvider>, KeyProtection) {
    if cfg!(windows) {
        (
            Box::new(soul_store::DpapiKeyProvider::new(
                directory.join(KEY_BLOB_FILE_NAME),
            )),
            KeyProtection::Dpapi,
        )
    } else {
        (
            Box::new(soul_store::TestKeyProvider::in_dir(directory)),
            KeyProtection::DeveloperKeyFile,
        )
    }
}

/// Everything that survives a restart.
///
/// Two fields, and neither of them can switch a capability on. See the module
/// docs for why that is the design rather than an omission.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredConfig {
    #[serde(default)]
    pub wizard_completed: bool,
    #[serde(default)]
    pub authorized_roots: Vec<PathBuf>,
}

impl StoredConfig {
    /// The running configuration this file describes.
    ///
    /// Everything else is [`Config::default`], which is to say off. The three
    /// capabilities AC-02 names are not read from disk because they are not
    /// written to it.
    pub fn to_config(&self) -> Config {
        Config {
            authorized_roots: self.authorized_roots.clone(),
            ..Config::default()
        }
    }
}

pub fn config_path(directory: impl AsRef<Path>) -> PathBuf {
    directory.as_ref().join(CONFIG_FILE_NAME)
}

/// Read the file, treating "not there yet" as a first run rather than a fault.
pub fn read_stored_config(directory: impl AsRef<Path>) -> Result<StoredConfig, ConfigFileError> {
    let path = config_path(directory);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StoredConfig::default())
        }
        Err(error) => return Err(ConfigFileError::Io(error.to_string())),
    };
    serde_json::from_str(&text).map_err(|error| ConfigFileError::Unreadable(error.to_string()))
}

/// Write the file, with the rename that keeps a half-written one from being
/// the one that is read next time.
pub fn write_stored_config(
    directory: impl AsRef<Path>,
    stored: &StoredConfig,
) -> Result<(), ConfigFileError> {
    let directory = directory.as_ref();
    std::fs::create_dir_all(directory).map_err(|error| ConfigFileError::Io(error.to_string()))?;
    let text = serde_json::to_string_pretty(stored)
        .map_err(|error| ConfigFileError::Unreadable(error.to_string()))?;
    let partial = directory.join(format!("{CONFIG_FILE_NAME}.partial"));
    std::fs::write(&partial, text).map_err(|error| ConfigFileError::Io(error.to_string()))?;
    std::fs::rename(&partial, config_path(directory))
        .map_err(|error| ConfigFileError::Io(error.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConfigFileError {
    #[error("配置文件读写失败：{0}")]
    Io(String),
    /// The file is there and is not what this build writes. Reported rather
    /// than repaired: a file naming a capability is exactly the case that must
    /// not be silently rewritten into one that looks fine.
    #[error("配置文件不是这一版认得的形状，已按全部关闭运行：{0}")]
    Unreadable(String),
}

/// The one store handle this process has, or the reason it has none.
#[derive(Debug)]
enum StoreHandle {
    Open(Arc<Mutex<SqlCipherStore>>),
    Unavailable(String),
}

/// One running Soul.
///
/// The store, the configuration, the authorized directories, the drafting
/// pair and the token ledger, in one place because they have to agree with
/// each other. The desktop shell manages exactly one of these and every
/// command borrows it; nothing else in the process opens a database.
#[derive(Debug)]
pub struct Session {
    directory: PathBuf,
    store: StoreHandle,
    key_protection: KeyProtection,
    config: Config,
    wizard_completed: bool,
    /// What went wrong with the file on disk, if anything. Shown rather than
    /// swallowed: a session whose configuration will not persist is one whose
    /// next launch asks the same questions again.
    config_problem: Option<String>,
    fileplan: FilePlanSession,
    unavailable_roots: Vec<UnavailableRootView>,
    issuer: TokenIssuer,
    draft: DraftSession,
    policy: PolicySession,
    /// The last forget impact the user was shown, and for which memory.
    ///
    /// Forgetting is irreversible and the numbers behind it are a live query,
    /// so the confirmation has to name the answer it read — WP04 left this
    /// gap open and said so. One slot, replaced by the next preview, taken by
    /// value when the forget runs: the same shape the endpoint drafting path
    /// uses to make an approval describe what is actually about to happen.
    held_forget: Option<HeldForget>,
    /// Whether collection may run, for this process and no longer.
    ///
    /// Opened closed at every launch and never read from disk. That is AC-02
    /// stated as a field rather than as care: [`StoredConfig`] has nowhere to
    /// put a granted consent, so there is no file a restart could read one out
    /// of, and [`ConsentHandle::from_ledger`] — the one constructor that could
    /// load one — is called nowhere in the product.
    consent: ConsentHandle,
    /// The collector, while one is running. Dropping the session stops it.
    collector: Option<RunningCollector>,
    /// Why nothing is being collected, when the user has said it may be.
    ///
    /// On anything that is not Windows the answer is that this build has no
    /// foreground source, which is a fact about the build rather than a fault.
    collect_problem: Option<String>,
}

/// One running collector, and the label of the source feeding it.
///
/// The label is kept beside the handle because [`ForegroundSource::describe`]
/// belongs to a value the collector took ownership of, and asking the platform
/// again would answer for a source that is not the one running.
#[derive(Debug)]
struct RunningCollector {
    handle: CollectorHandle,
    source: &'static str,
}

/// One forget the user has been quoted a price for.
#[derive(Debug, Clone)]
struct HeldForget {
    preview_id: Uuid,
    memory_id: Uuid,
    impact: ForgetImpact,
}

impl Session {
    /// Open everything, once, for this directory.
    ///
    /// Never fails. A store that will not open, a configuration file that will
    /// not parse and a directory that has gone are all states the interface has
    /// to be able to show, and a session that refused to exist would leave it
    /// with nothing to show them on.
    pub fn open(directory: impl Into<PathBuf>) -> Session {
        let directory = directory.into();
        let mut config_problem = match std::fs::create_dir_all(&directory) {
            Ok(()) => None,
            Err(error) => Some(ConfigFileError::Io(error.to_string()).to_string()),
        };

        let stored = match read_stored_config(&directory) {
            Ok(stored) => stored,
            Err(error) => {
                config_problem = Some(error.to_string());
                StoredConfig::default()
            }
        };

        let config = stored.to_config();
        let (fileplan, rejected) = FilePlanSession::from_config(&config);
        let unavailable_roots = rejected
            .into_iter()
            .map(|(path, refusal)| UnavailableRootView {
                path,
                explanation: refusal.to_string(),
            })
            .collect();

        let (keys, key_protection) = key_provider(&directory);
        let store = match store_commands::open_store(&directory, keys.as_ref()) {
            Ok(store) => StoreHandle::Open(share(store)),
            Err(error) => StoreHandle::Unavailable(error.to_string()),
        };

        let (draft, policy) = draft::closed_session();
        Session {
            directory,
            store,
            key_protection,
            config,
            wizard_completed: stored.wizard_completed,
            config_problem,
            fileplan,
            unavailable_roots,
            issuer: TokenIssuer::new(),
            draft,
            policy,
            held_forget: None,
            consent: ConsentHandle::closed(),
            collector: None,
            collect_problem: None,
        }
    }

    /// Open the directory this machine keeps Soul's data in.
    pub fn open_platform_directory() -> Result<Session, DirectoryError> {
        Ok(Session::open(data_directory()?))
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// The store, for a caller that already knows it may not be there.
    ///
    /// One `Arc`, cloned. There is no entry point on this type that opens a
    /// second connection, and `Session::open` is the only place that opens the
    /// first.
    pub fn store(&self) -> Option<Arc<Mutex<SqlCipherStore>>> {
        match &self.store {
            StoreHandle::Open(store) => Some(Arc::clone(store)),
            StoreHandle::Unavailable(_) => None,
        }
    }

    pub fn snapshot(&self) -> ConfigSnapshot {
        ConfigSnapshot::of(&self.config)
    }

    /// What the shell needs to know that is not about capabilities: whether
    /// the wizard is behind us, and whether there is a database to read.
    pub fn status(&self) -> SessionStatus {
        SessionStatus {
            wizard_completed: self.wizard_completed,
            store_opened: matches!(self.store, StoreHandle::Open(_)),
            key_protection: self.key_protection.as_str().to_owned(),
            store_notice: match &self.store {
                StoreHandle::Open(_) => self.key_protection.notice().to_owned(),
                StoreHandle::Unavailable(reason) => {
                    format!("{STORE_UNAVAILABLE_NOTICE}（{reason}）")
                }
            },
            config_problem: self.config_problem.clone(),
        }
    }

    /// Finish the first-run wizard, and write down that it is finished.
    ///
    /// The refusal is `soulcore`'s existing one: a wizard that would hand back
    /// an open configuration does not finish, so nothing is written either.
    pub fn complete_wizard(
        &mut self,
        answers: &WizardAnswers,
    ) -> Result<ConfigSnapshot, WizardRefused> {
        shell::complete_wizard(answers)?;
        self.wizard_completed = true;
        self.persist();
        Ok(self.snapshot())
    }

    pub fn cloud_toggle(&self, requested_on: bool) -> CloudNotice {
        shell::cloud_toggle(&self.config, requested_on)
    }

    /// The directories this session may read, and the ones it can no longer
    /// find.
    pub fn files(&self) -> FilesView {
        FilesView {
            read_only_notice: READ_ONLY_NOTICE.to_owned(),
            executable_in_this_version: false,
            roots: self
                .fileplan
                .roots()
                .iter()
                .map(AuthorizedRootView::of)
                .collect(),
            unavailable_roots: self.unavailable_roots.clone(),
        }
    }

    /// Record that the user authorized one directory, and remember it.
    pub fn authorize(&mut self, path: &str) -> Result<FilesView, SessionRefusal> {
        let root = self.fileplan.authorize(path)?;
        let canonical = root.canonical().to_path_buf();
        if !self.config.authorized_roots.contains(&canonical) {
            self.config.authorized_roots.push(canonical);
        }
        self.persist();
        Ok(self.files())
    }

    /// Scan one authorized directory and describe what tidying it would mean.
    ///
    /// Read-only, all the way down: `soul-fileplan` has no write API and this
    /// build has no `execute`. The origin is [`RequestOrigin::User`] because a
    /// call arriving over the IPC is a click.
    pub fn preview(&mut self, path: &str) -> Result<PlanPreview, SessionRefusal> {
        let preview = self.fileplan.preview(
            &mut self.issuer,
            path,
            RequestOrigin::User,
            ScanLimits::default(),
            now_unix_millis(),
        )?;
        Ok(PlanPreview::of(&preview))
    }

    /// The people graph, with every edge's evidence resolved.
    pub fn people(&self) -> Result<PeopleGraphView, SessionRefusal> {
        let store = self.opened_store()?;
        let store = hold(&store);
        Ok(graph_commands::people_view(&store)?)
    }

    /// Everything Soul will say about one person, and what each line rests on.
    pub fn person_summary(
        &mut self,
        contact_id: &str,
    ) -> Result<PersonSummaryView, SessionRefusal> {
        let contact_id = Uuid::parse_str(contact_id).map_err(|_| SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: "这不是一个认得出来的人的编号。".to_owned(),
        })?;
        let store = self.opened_store()?;
        let store = hold(&store);
        Ok(draft::summarize_person(
            &mut self.policy,
            &store,
            contact_id,
            RequestOrigin::User,
            now_unix_millis(),
        )?)
    }

    /// AC-17: a draft written on this machine, with no request body built.
    pub fn draft_pasted(&mut self, pasted: &str) -> Result<DraftValue, SessionRefusal> {
        Ok(draft::draft_pasted(&self.draft, &mut self.policy, pasted)?)
    }

    /// Step one of the endpoint path: describe what would be sent.
    ///
    /// `include_original` is the one-shot exemption AC-13 is about, and the
    /// session keeps no note of it: it is forwarded, spent while the body is
    /// built, and gone. Anything but `Some(true)` — which is every call the
    /// confirmation screen makes until somebody presses the second button —
    /// describes a placeheld request, and there is no field on this type or in
    /// `config.json` that could say otherwise.
    ///
    /// It is an [`Option`] because the interface may leave it out, and the
    /// answer to a caller who said nothing is decided here rather than in the
    /// shell: an absent second confirmation is not a confirmation, and that is
    /// the sort of thing `soulcore` should be the one to say.
    pub fn prepare_draft(
        &mut self,
        pasted: &str,
        include_original: Option<bool>,
    ) -> Result<E1DraftPlan, SessionRefusal> {
        Ok(draft::prepare_pasted(
            &mut self.draft,
            &mut self.policy,
            pasted,
            include_original == Some(true),
        )?)
    }

    /// Step two: the user approved the plan they were shown.
    pub fn generate_draft(&mut self, approval: &Approval) -> Result<DraftValue, SessionRefusal> {
        Ok(draft::generate_prepared(&mut self.draft, &mut self.policy, approval)?.draft)
    }

    /// The honest answer to a user who read the plan and said no.
    pub fn discard_draft(&mut self) -> bool {
        self.draft.discard()
    }

    // ------------------------------------------------- WP08: the endpoint ---

    /// The user typed in their own OpenAI-compatible endpoint.
    ///
    /// Three things happen here, and the one that does not is the load-bearing
    /// one. The guard is re-pointed, so an approved generation has somewhere to
    /// go; the in-memory [`Config`] records that there is an endpoint, so the
    /// snapshot the interface renders says 已填写; and `persist` is not
    /// called. [`StoredConfig`] has no field this could be written to and
    /// this method does not give it one, which is what makes AC-02 true of the
    /// endpoint the same way it is true of collection consent: the next launch
    /// starts from [`draft::closed_session`] because there is no file that
    /// could tell it otherwise, rather than because something remembered to
    /// clear one.
    ///
    /// Nothing is contacted. `Origin::parse` reads a string and `NetGuard`
    /// holds the answer; the first packet still waits for the plan on the
    /// drafting screen and the approval in front of it. A preparation made
    /// before the address changed would be approved against the address that
    /// is here when the user presses 生成 — the plan is counts and a model
    /// name, and never named a host — which is reachable only by leaving the
    /// drafting page mid-flight and coming back to it.
    ///
    /// What is stored in the configuration is the origin the guard ended up
    /// with rather than the string that was typed: a path, a query and a
    /// trailing slash are all dropped by the parser, and the field should say
    /// what Soul would actually reach.
    pub fn set_user_endpoint(&mut self, url: &str) -> Result<ConfigSnapshot, SessionRefusal> {
        // The error is dropped rather than forwarded. Every variant of it
        // quotes the string back, and [`ENDPOINT_UNPARSABLE_NOTICE`] says why
        // that is not something to put on a screen.
        self.policy
            .set_user_endpoint(url)
            .map_err(|_| SessionRefusal {
                reason_code: ReasonCode::EgressTargetUnparsable.as_str().to_owned(),
                explanation: ENDPOINT_UNPARSABLE_NOTICE.to_owned(),
            })?;
        self.config.llm_endpoint = self
            .policy
            .guard()
            .config()
            .e1_endpoint()
            .map(ToString::to_string);
        Ok(self.snapshot())
    }

    /// The user took the address away again.
    ///
    /// [`PolicySession::clear_user_endpoint`] puts the guard back to
    /// `NetGuard::closed()`, which refuses every origin including loopback, so
    /// what is left is the state a fresh launch is in rather than a weaker one
    /// that merely has no URL to hand. Nothing is persisted here either; there
    /// was never anything on disk to remove.
    pub fn clear_user_endpoint(&mut self) -> ConfigSnapshot {
        self.policy.clear_user_endpoint();
        self.config.llm_endpoint = None;
        self.snapshot()
    }

    // ------------------------------------------------------- WP06: import ---

    /// What a `soul-import-v1` file contains. Writes nothing.
    ///
    /// The store has to be open for a preview as well as for a commit. Parsing
    /// itself needs nothing — `soul-import` never touches a database — but a
    /// screen that counted up somebody's export and only then said there is
    /// nowhere to put it would have read the file for no reason.
    pub fn preview_soul_import_v1(&self, text: &str) -> Result<ImportPreview, SessionRefusal> {
        self.opened_store()?;
        let staged = import_commands::read_soul_import_v1(text)?;
        Ok(ImportPreview::of(&staged))
    }

    /// The same for a Telegram Desktop `result.json`. AC-05.
    ///
    /// v0.1 does not open archives: the user points at the `result.json` that
    /// Telegram's own *Export chat history → Machine-readable JSON* produced,
    /// and this reads the text of that one file.
    pub fn preview_telegram(&self, text: &str) -> Result<ImportPreview, SessionRefusal> {
        self.opened_store()?;
        let staged = import_commands::read_telegram(&telegram_document(text)?)?;
        Ok(ImportPreview::of(&staged))
    }

    /// Seal a `soul-import-v1` file into the store. AC-04.
    pub fn commit_soul_import_v1(
        &mut self,
        text: &str,
    ) -> Result<ImportReceiptView, SessionRefusal> {
        let staged = import_commands::read_soul_import_v1(text)?;
        self.commit_import(&staged)
    }

    /// Seal a Telegram export into the store. AC-05.
    pub fn commit_telegram(&mut self, text: &str) -> Result<ImportReceiptView, SessionRefusal> {
        let staged = import_commands::read_telegram(&telegram_document(text)?)?;
        self.commit_import(&staged)
    }

    /// Write one parsed import, and derive the graph it implies.
    ///
    /// The file is parsed again by whichever `commit_*` got here rather than
    /// carried over from the preview: a staged import held on the session
    /// would be a second copy of somebody's export sitting in memory between
    /// two clicks, and the text the WebView sends back is the same text it
    /// previewed. What the user approved is a set of counts, and a re-parse of
    /// the same bytes produces the same ones.
    ///
    /// The rebuild runs under the same store guard, which is what makes
    /// `/graph` show the people this file just added without a second opening
    /// of anything. `soul-graph::rebuild` is idempotent — a second import
    /// updates the ties rather than growing a parallel graph.
    fn commit_import(
        &mut self,
        staged: &soul_import::model::StagedImport,
    ) -> Result<ImportReceiptView, SessionRefusal> {
        let at = now_unix_seconds();
        let store = self.opened_store()?;
        let mut store = hold(&store);
        let receipt = import_commands::commit(&mut store, staged, at)?;
        let build = graph_commands::rebuild(&mut store, at)?;
        Ok(ImportReceiptView::of(&receipt, build.edges_written.len()))
    }

    // ------------------------------------------------------ WP03: profile ---

    /// The eleven questions, for a wizard that has to draw them.
    ///
    /// Answered from the canonical list rather than from the store, so a
    /// machine whose database will not open can still ask them; recording the
    /// answers is what needs the store, and that is the next method.
    pub fn questionnaire(&self) -> Vec<QuestionView> {
        profile_commands::question_views()
    }

    /// Record a completed questionnaire. AC-03.
    ///
    /// Blank answers are dropped rather than guessed at: an axis nobody
    /// answered for stays `unknown`. A questionnaire where every question was
    /// left blank is refused, because it would leave the profile exactly as
    /// empty as it was and reporting that as a completed intake would be a
    /// lie the wizard then repeats to the user.
    pub fn answer_questionnaire(
        &mut self,
        answers: &[GivenAnswer],
    ) -> Result<IntakeReceipt, SessionRefusal> {
        if answers.iter().all(|answer| answer.given.trim().is_empty()) {
            return Err(SessionRefusal {
                reason_code: ReasonCode::Routine.as_str().to_owned(),
                explanation: NO_ANSWERS_NOTICE.to_owned(),
            });
        }
        let at = now_unix_seconds();
        let store = self.opened_store()?;
        let mut store = hold(&store);
        Ok(profile_commands::intake_from(
            &mut store,
            OWNER_PROFILE_ID,
            answers,
            &rfc3339_utc(at),
            at,
        )?)
    }

    /// The profile screen: axes, voice, the pointers to what the user stated.
    pub fn profile(&self) -> Result<ProfileScreen, SessionRefusal> {
        let store = self.opened_store()?;
        let store = hold(&store);
        Ok(profile_commands::screen(&store, OWNER_PROFILE_ID)?)
    }

    /// The user read an axis and said it is wrong. AC-07: this pins it.
    pub fn correct_axis(
        &mut self,
        axis_id: &str,
        position: &str,
    ) -> Result<ProfileScreen, SessionRefusal> {
        let axis_id = profile_commands::axis_named(axis_id).ok_or_else(|| SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: "这不是这一版档案里的那五条轴之一。".to_owned(),
        })?;
        let position =
            profile_commands::position_named(position).ok_or_else(|| SessionRefusal {
                reason_code: ReasonCode::Routine.as_str().to_owned(),
                explanation: "一条轴只有偏向一端、偏向另一端和两端都有三种说法。".to_owned(),
            })?;
        let at = now_unix_seconds();
        let store = self.opened_store()?;
        let mut store = hold(&store);
        profile_commands::correct_axis(&mut store, OWNER_PROFILE_ID, axis_id, position, at)?;
        Ok(profile_commands::screen(&store, OWNER_PROFILE_ID)?)
    }

    /// The user set a voice field by hand. Inference must leave it alone after
    /// this, and `soul-profile` is where that is enforced.
    pub fn set_voice(
        &mut self,
        field: &str,
        option: &str,
    ) -> Result<ProfileScreen, SessionRefusal> {
        let setting =
            profile_commands::voice_setting_named(field, option).ok_or_else(|| SessionRefusal {
                reason_code: ReasonCode::Routine.as_str().to_owned(),
                explanation: "语气只有那四项，每一项只有列出来的那几个取值。".to_owned(),
            })?;
        let at = now_unix_seconds();
        let store = self.opened_store()?;
        let mut store = hold(&store);
        profile_commands::set_voice(&mut store, OWNER_PROFILE_ID, setting, at)?;
        Ok(profile_commands::screen(&store, OWNER_PROFILE_ID)?)
    }

    // ------------------------------------------------------- WP04: memory ---

    /// Every memory, prose left sealed.
    pub fn memories(&self) -> Result<MemoryList, SessionRefusal> {
        let store = self.opened_store()?;
        let store = hold(&store);
        Ok(memory_commands::rows(&store)?)
    }

    /// One memory, opened because the user asked for this one.
    pub fn memory(&self, memory_id: &str) -> Result<MemoryDetail, SessionRefusal> {
        let memory_id = parse_id(memory_id, "记忆")?;
        let store = self.opened_store()?;
        let store = hold(&store);
        Ok(memory_commands::detail(&store, memory_id)?)
    }

    /// Write a memory the user typed.
    pub fn write_memory(&mut self, new: &NewMemory) -> Result<MemoryDetail, SessionRefusal> {
        let at = now_unix_seconds();
        let store = self.opened_store()?;
        let mut store = hold(&store);
        Ok(memory_commands::write_new(&mut store, new, at)?)
    }

    /// Edit one in place. The same content key is reused, so the memory stays
    /// exactly one forget unit.
    pub fn edit_memory(
        &mut self,
        memory_id: &str,
        change: &MemoryChange,
    ) -> Result<MemoryDetail, SessionRefusal> {
        let memory_id = parse_id(memory_id, "记忆")?;
        let at = now_unix_seconds();
        let store = self.opened_store()?;
        let mut store = hold(&store);
        Ok(memory_commands::write_change(
            &mut store, memory_id, change, at,
        )?)
    }

    /// What forgetting this memory would cost. Destroys nothing.
    ///
    /// The preview is remembered so the forget that follows can be checked
    /// against it. Asking twice replaces the held one, which is why the
    /// confirmation carries an id rather than only a memory.
    pub fn preview_forget(&mut self, memory_id: &str) -> Result<ForgetPreview, SessionRefusal> {
        let memory_id = parse_id(memory_id, "记忆")?;
        let impact = {
            let store = self.opened_store()?;
            let store = hold(&store);
            memory_commands::preview_forget(&store, memory_id)?
        };
        let preview_id = Uuid::now_v7();
        self.held_forget = Some(HeldForget {
            preview_id,
            memory_id,
            impact: impact.clone(),
        });
        Ok(ForgetPreview::of(preview_id, memory_id, &impact))
    }

    /// Destroy the content key behind one memory. AC-15, and irreversible.
    ///
    /// Refused unless the confirmation names the preview the user was shown.
    /// Forgetting is allowed — it is what D15 means by deleting — and it is
    /// not a file write; what is not allowed is doing it because a second
    /// click landed on a screen nobody read.
    pub fn forget_memory(
        &mut self,
        confirmation: &ForgetConfirmation,
    ) -> Result<ForgetReceiptView, SessionRefusal> {
        let held = self
            .held_forget
            .take()
            .filter(|held| {
                held.preview_id.to_string() == confirmation.preview_id
                    && held.memory_id.to_string() == confirmation.memory_id
            })
            .ok_or_else(|| SessionRefusal {
                reason_code: ReasonCode::PlanHashMismatch.as_str().to_owned(),
                explanation: FORGET_NOT_PREVIEWED_NOTICE.to_owned(),
            })?;

        let at = now_unix_seconds();
        let store = self.opened_store()?;
        let mut store = hold(&store);
        let receipt = memory_commands::forget(&mut store, held.memory_id, at)?;
        Ok(ForgetReceiptView::of(
            held.memory_id,
            &receipt,
            &held.impact,
        ))
    }

    // ------------------------------------------------------- WP07: collect ---

    /// Whether collection may run, whether it is running, and how much has
    /// been written. Never what was collected.
    ///
    /// The two booleans are read off the consent ledger and the collector
    /// thread, not off a configuration field. `collect.rs` says why in one
    /// line: a third copy of the answer in a settings file is how a user ends
    /// up being shown "off" by a switch while something is still writing.
    pub fn collect_status(&self) -> CollectStatus {
        let running = self.running_collector();
        CollectStatus {
            consent_granted: self.consent.is_granted(COLLECTION_TOPIC),
            collector_running: running.is_some(),
            source: running
                .map(|running| running.source.to_owned())
                .unwrap_or_else(available_source),
            events_collected: self.count_collected(),
            // Consent lives in this process. There is no field in
            // `config.json` that could carry it to the next launch.
            survives_restart: false,
            duration_only_notice: COLLECT_DURATION_ONLY_NOTICE.to_owned(),
            notice: self.collect_notice(),
        }
    }

    /// The user said collection may run, and it starts if this machine has a
    /// foreground source.
    ///
    /// Granting and collecting are two things, and they can come apart: on
    /// Linux and on a developer machine `platform_source` has nothing to
    /// return, so consent is recorded and the collector is not started. The
    /// status says which of the two happened rather than reporting the grant
    /// as if something were being watched.
    pub fn grant_collect_consent(&mut self) -> Result<CollectStatus, SessionRefusal> {
        self.grant_collection(
            collect_commands::platform_source(),
            CollectorConfig::default(),
        )
    }

    /// As [`Session::grant_collect_consent`], against a source the caller has.
    ///
    /// AC-09 and AC-10 are written about a Windows desktop, and a Linux CI host
    /// has none. This is how they are proven through the session anyway: the
    /// same consent ledger, the same collector, the same encrypted store, with
    /// [`crate::commands::collect::FakeForegroundSource`] where
    /// `GetForegroundWindow` would be. Nothing in the desktop shell calls it —
    /// `apps/desktop/src-tauri/tests/command_surface.rs` reads the shell's own
    /// sources back and fails if that ever changes.
    #[doc(hidden)]
    pub fn grant_collect_consent_with_source<F>(
        &mut self,
        source: F,
        config: CollectorConfig,
    ) -> Result<CollectStatus, SessionRefusal>
    where
        F: ForegroundSource + Send + 'static,
    {
        self.grant_collection(Ok(source), config)
    }

    /// The user took it back. Revoking comes first, so the collector thread
    /// sees it whether or not the stop below gets a chance to run.
    ///
    /// `soul-collect` gives the two paths different meanings on purpose: a
    /// revocation drops the session in flight, a stop writes it. Both stop
    /// producing events inside `STOP_BUDGET`, which `soul-collect`'s own tests
    /// measure.
    pub fn revoke_collect_consent(&mut self) -> Result<CollectStatus, SessionRefusal> {
        let at = now_unix_seconds();
        let entry = self.consent.revoke(COLLECTION_TOPIC, at);

        // Stopping is what the user asked for, and it happens whether or not
        // the chain can be written to. A store that has gone away is reported
        // in the notice; it is not a reason to leave a collector running.
        self.collect_problem = match self.store() {
            Some(store) => collect_commands::record_consent_change(&mut hold(&store), entry, at)
                .err()
                .map(|error| error.to_string()),
            None => Some(STORE_UNAVAILABLE_NOTICE.to_owned()),
        };

        if let Some(running) = self.collector.take() {
            if let Err(error) = collect_commands::stop(running.handle) {
                self.collect_problem = Some(format!("{COLLECT_NOT_STOPPED_NOTICE}{error}"));
            }
        }
        Ok(self.collect_status())
    }

    /// Record the grant, then try to start. The order matters: `runner::start`
    /// refuses an ungranted ledger, and the audit entry the ledger handed back
    /// is owed to the chain before anything begins writing events.
    fn grant_collection<F>(
        &mut self,
        source: Result<F, SourceError>,
        config: CollectorConfig,
    ) -> Result<CollectStatus, SessionRefusal>
    where
        F: ForegroundSource + Send + 'static,
    {
        if self.running_collector().is_some() {
            return Ok(self.collect_status());
        }

        let store = self.opened_store()?;
        let at = now_unix_seconds();
        let entry = self.consent.grant(COLLECTION_TOPIC, at);
        collect_commands::record_consent_change(&mut hold(&store), entry, at)?;

        self.collect_problem = match source {
            Ok(source) => {
                let label = source.describe();
                match collect_commands::start(source, Arc::clone(&store), &self.consent, config) {
                    Ok(handle) => {
                        self.collector = Some(RunningCollector {
                            handle,
                            source: label,
                        });
                        None
                    }
                    Err(error) => Some(error.to_string()),
                }
            }
            Err(error) => Some(error.to_string()),
        };
        Ok(self.collect_status())
    }

    /// The collector, if there is one and its thread has not already finished.
    ///
    /// A revocation stops the thread without anyone calling `stop`, so a handle
    /// that is still held is not the same thing as a collector that is running.
    fn running_collector(&self) -> Option<&RunningCollector> {
        self.collector
            .as_ref()
            .filter(|running| running.handle.is_running())
    }

    /// Foreground events in the store, or `None` when there is no store to
    /// count them in.
    fn count_collected(&self) -> Option<usize> {
        let store = self.store()?;
        let store = hold(&store);
        collect_commands::events_collected(&store).ok()
    }

    fn collect_notice(&self) -> String {
        let state = if self.running_collector().is_some() {
            COLLECT_RUNNING_NOTICE
        } else if self.consent.is_granted(COLLECTION_TOPIC) {
            COLLECT_NOT_OBSERVING_NOTICE
        } else {
            COLLECT_OFF_NOTICE
        };
        match &self.collect_problem {
            Some(problem) => format!("{state}（{problem}）"),
            None => state.to_owned(),
        }
    }

    // --------------------------------------- WP02: research, and the chain ---

    /// What the research track would see. AC-20: no third-party row, no file.
    pub fn research(&self) -> Result<ResearchPreviewView, SessionRefusal> {
        let store = self.opened_store()?;
        let store = hold(&store);
        Ok(store_commands::research_view(
            &store,
            RESEARCH_PREVIEW_ROWS,
        )?)
    }

    /// The audit chain, played back and checked. AC-23: no prose in it.
    pub fn audit(&self) -> Result<AuditChainView, SessionRefusal> {
        let store = self.opened_store()?;
        let store = hold(&store);
        Ok(store_commands::audit_view(&store)?)
    }

    fn opened_store(&self) -> Result<Arc<Mutex<SqlCipherStore>>, SessionRefusal> {
        match &self.store {
            StoreHandle::Open(store) => Ok(Arc::clone(store)),
            StoreHandle::Unavailable(reason) => Err(SessionRefusal {
                reason_code: ReasonCode::Routine.as_str().to_owned(),
                explanation: format!("{STORE_UNAVAILABLE_NOTICE}（{reason}）"),
            }),
        }
    }

    /// Write the two answers down, recording a failure rather than hiding it.
    fn persist(&mut self) {
        let stored = StoredConfig {
            wizard_completed: self.wizard_completed,
            authorized_roots: self.config.authorized_roots.clone(),
        };
        self.config_problem = write_stored_config(&self.directory, &stored)
            .err()
            .map(|error| error.to_string());
    }
}

/// The store, with a poisoned lock recovered rather than propagated.
///
/// A panic while one command held the store must not take every other route
/// away for the rest of the session: `SqlCipherStore` writes in transactions,
/// so what a panicking caller can leave behind is a rolled-back statement
/// rather than a half-written row.
fn hold(store: &Arc<Mutex<SqlCipherStore>>) -> MutexGuard<'_, SqlCipherStore> {
    store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One Telegram export, as JSON, or a refusal that says where the file stops
/// being readable without quoting what is there.
///
/// `serde_json`'s own message is not forwarded. It names the value it choked
/// on, and that value is somebody's message; a line and a column say the same
/// thing about where to look and nothing at all about what is written there.
fn telegram_document(text: &str) -> Result<serde_json::Value, SessionRefusal> {
    serde_json::from_str(text).map_err(|error| SessionRefusal {
        reason_code: ReasonCode::Routine.as_str().to_owned(),
        explanation: format!(
            "{IMPORT_REFUSED_NOTICE}\n第 {} 行第 {} 列起，这个文件不是一段读得下去的 JSON。\
             Telegram 的「导出聊天记录」要选 Machine-readable JSON，导出目录里的 result.json \
             才是这一版认得的形状；压缩包和 HTML 导出都读不了。",
            error.line(),
            error.column(),
        ),
    })
}

/// An identifier the interface handed back, or a refusal that names the kind
/// of thing it was supposed to identify and nothing else.
fn parse_id(id: &str, kind: &str) -> Result<Uuid, SessionRefusal> {
    Uuid::parse_str(id).map_err(|_| SessionRefusal {
        reason_code: ReasonCode::Routine.as_str().to_owned(),
        explanation: format!("这不是一个认得出来的{kind}编号。"),
    })
}

/// What the shell knows about this session that is not about capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionStatus {
    /// Whether the first-run wizard has been finished before. Read from disk,
    /// which is what makes it survive a restart.
    pub wizard_completed: bool,
    pub store_opened: bool,
    /// `dpapi` or `developer_key_file`.
    pub key_protection: String,
    /// One sentence about the database: how its key is protected, or why it
    /// did not open.
    pub store_notice: String,
    /// Present when the configuration file could not be read or written.
    pub config_problem: Option<String>,
}

/// The collection screen's whole state.
///
/// Counts and booleans, and not one field that could hold an application name,
/// a window title or a path. PRODUCT_LOCK puts window titles in the 不做
/// column and `soul-collect` keeps them out of [`crate::commands::collect::AppIdentity`];
/// what this type adds is that the interface is never sent one either, so "the
/// screen does not show what you were doing" is a property of the shape rather
/// than of the component that renders it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectStatus {
    /// Whether the consent ledger has the collection topic open, right now.
    pub consent_granted: bool,
    /// Whether a collector thread is actually running. Not the same answer: a
    /// grant on a machine with no foreground source leaves this false.
    pub collector_running: bool,
    /// `windows.foreground_process`, `fake.scripted_desktop`, or
    /// [`NO_FOREGROUND_SOURCE`]. A fixed label, never a sample.
    pub source: String,
    /// Foreground events in the store, or `None` when it did not open.
    pub events_collected: Option<usize>,
    /// Always false. Consent is granted for this process, and the file beside
    /// the store has no field that could carry it to the next launch.
    pub survives_restart: bool,
    /// What collection is, in the core's own words: duration only.
    pub duration_only_notice: String,
    /// What is true right now: off, running, or granted with nothing watching.
    pub notice: String,
}

/// What this machine's foreground source is called, or that it has none.
///
/// Asked of the platform rather than remembered, because a build reporting
/// `windows.foreground_process` on a host that has no such thing would be
/// making exactly the claim `platform_source` exists to refuse to make.
fn available_source() -> String {
    match collect_commands::platform_source() {
        Ok(source) => source.describe().to_owned(),
        Err(_) => NO_FOREGROUND_SOURCE.to_owned(),
    }
}

/// The file-plan screen's whole state before a scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesView {
    pub read_only_notice: String,
    /// Always false. There is no `execute` on this surface to make it true.
    pub executable_in_this_version: bool,
    pub roots: Vec<AuthorizedRootView>,
    /// Authorized once, and no longer resolvable. Reported rather than
    /// dropped: the user believes Soul can read these.
    pub unavailable_roots: Vec<UnavailableRootView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnavailableRootView {
    pub path: String,
    pub explanation: String,
}

/// A refused command, as a code and a sentence.
///
/// One shape for every route the shell has, rather than one per work package:
/// a screen that has to branch on which crate said no is a screen deciding
/// something. The code comes from `soul-policy`'s frozen vocabulary, so an
/// audit reader and a person reading the interface see the same word, and the
/// sentence is always Soul's own — never an endpoint's and never a path an
/// operating system error happened to quote back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
#[error("{explanation}")]
pub struct SessionRefusal {
    pub reason_code: String,
    pub explanation: String,
}

impl From<Refusal> for SessionRefusal {
    fn from(refusal: Refusal) -> SessionRefusal {
        SessionRefusal {
            reason_code: refusal.reason_code().as_str().to_owned(),
            explanation: refusal.to_string(),
        }
    }
}

impl From<DraftRefusal> for SessionRefusal {
    fn from(refusal: DraftRefusal) -> SessionRefusal {
        SessionRefusal {
            reason_code: refusal.reason_code().as_str().to_owned(),
            explanation: refusal.to_string(),
        }
    }
}

/// A file that did not parse, as a sentence the user can act on.
///
/// `ImportFailure` already reads like one — a locator, a contract field name
/// and what is wrong with it, per defect — and it is `soul-import`'s job to
/// keep the file's own words out of it. What is added here is the line in
/// front: nothing was written, so there is nothing to undo.
impl From<soul_import::defect::ImportFailure> for SessionRefusal {
    fn from(failure: soul_import::defect::ImportFailure) -> SessionRefusal {
        SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: format!("{IMPORT_REFUSED_NOTICE}\n{failure}"),
        }
    }
}

/// A file that parsed and then could not be stored.
///
/// Said differently from a parse failure on purpose: a commit is not atomic in
/// v0.1 — `soul-store-api` has no entry point that lets a caller open a
/// transaction — so a failure here can leave part of the export behind.
/// Re-running the same file is safe; the people are matched by identifier
/// digest and only the events are written a second time.
impl From<soul_import::commit::ImportError> for SessionRefusal {
    fn from(error: soul_import::commit::ImportError) -> SessionRefusal {
        SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: format!(
                "这次导入没有做完：{error}。v0.1 的导入不是一个事务，中途失败可能已经写进去一部分；\
                 同一个文件再导一次是安全的，人会被认回来，事件会多一份。"
            ),
        }
    }
}

impl From<soul_graph::GraphError> for SessionRefusal {
    fn from(error: soul_graph::GraphError) -> SessionRefusal {
        SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: error.to_string(),
        }
    }
}

impl From<soul_profile::ProfileError> for SessionRefusal {
    fn from(error: soul_profile::ProfileError) -> SessionRefusal {
        SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: error.to_string(),
        }
    }
}

impl From<soul_memory::MemoryError> for SessionRefusal {
    fn from(error: soul_memory::MemoryError) -> SessionRefusal {
        SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: error.to_string(),
        }
    }
}

impl From<soul_store_api::types::StoreError> for SessionRefusal {
    fn from(error: soul_store_api::types::StoreError) -> SessionRefusal {
        SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: error.to_string(),
        }
    }
}
