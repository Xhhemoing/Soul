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
use soul_policy::clock::now_unix_millis;
use soul_policy::hitl::{RequestOrigin, TokenIssuer};
use soul_policy::ReasonCode;
use soul_store::SqlCipherStore;

use crate::commands::collect::share;
use crate::commands::draft::{
    self, Approval, DraftRefusal, DraftSession, DraftValue, E1DraftPlan, PersonSummaryView,
};
use crate::commands::fileplan::{
    AuthorizedRootView, FilePlanSession, PlanPreview, READ_ONLY_NOTICE,
};
use crate::commands::graph::{self as graph_commands, PeopleGraphView};
use crate::commands::policy::PolicySession;
use crate::commands::shell::{self, CloudNotice, ConfigSnapshot, WizardAnswers, WizardRefused};
use crate::commands::store as store_commands;
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
    pub fn prepare_draft(&mut self, pasted: &str) -> Result<E1DraftPlan, SessionRefusal> {
        Ok(draft::prepare_pasted(
            &mut self.draft,
            &mut self.policy,
            pasted,
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

impl From<soul_graph::GraphError> for SessionRefusal {
    fn from(error: soul_graph::GraphError) -> SessionRefusal {
        SessionRefusal {
            reason_code: ReasonCode::Routine.as_str().to_owned(),
            explanation: error.to_string(),
        }
    }
}
