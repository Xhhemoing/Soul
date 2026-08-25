//! WP11's command surface: authorize a directory, preview a plan for it, and
//! refuse to carry the plan out.
//!
//! Thin, like the rest of this module. `soul-fileplan` decides what may be read
//! and what tidying would mean; what is here is the session that has to outlive
//! a single call — the set of directories the user has authorized — and the
//! view the WebView is allowed to see.
//!
//! There is deliberately no `execute`. [`refuse_execution`] is the whole of
//! this build's answer to a request to move a file, and it is re-exported here
//! rather than hidden so that the desktop shell has something honest to bind a
//! refusal to. PRODUCT_LOCK puts the write half in v0.1.1 (AC-27), and a
//! command surface with a stub named `execute` on it is how a stub becomes an
//! implementation.
//!
//! The token ledger is a parameter rather than a field. No file-plan action
//! needs a capability token — `ActionKind::needs_capability_token` says so and
//! `soul-fileplan` asserts it — so this surface has no reason to own one, and
//! a second ledger beside `PolicySession`'s would be a second opinion about
//! which tokens have been spent.

use std::path::Path;

use serde::{Deserialize, Serialize};

use soul_fileplan::{
    Authorization, AuthorizedRoot, ExecutionRefusal, FileKind, OrganizePlan, Preview, Refusal,
    ScanLimits,
};
use soul_policy::hitl::{ActionRequest, RequestOrigin, TokenIssuer};

use crate::config::Config;

pub use soul_fileplan::{refuse_execution, FILEPLAN_ACTIONS};

/// What the file-plan view says about executing, in every state it has.
///
/// A constant rather than a sentence in the interface, for the same reason
/// WP09 made the cloud notice one: the shell must not be able to soften it,
/// and a test can compare the two spellings.
pub const READ_ONLY_NOTICE: &str = "v0.1 只做只读扫描与计划预览，不会移动、重命名或删除任何文件。\
     受控目录整理的执行与撤销是 v0.1.1 的事。";

/// The directories this session may read.
///
/// Empty until the user authorizes something, which is also what `Config`
/// starts as. Nothing here is written to disk: where the authorization list
/// lives between runs is WP13's question, and until it is answered a restart
/// means authorizing again — which is the safe direction to be wrong in.
#[derive(Debug, Clone, Default)]
pub struct FilePlanSession {
    authorization: Authorization,
}

impl FilePlanSession {
    pub fn new() -> FilePlanSession {
        FilePlanSession::default()
    }

    /// Restore the roots a configuration names, reporting the ones that no
    /// longer resolve rather than dropping them.
    ///
    /// A directory that has been deleted or renamed since the user authorized
    /// it must not silently disappear from the list: the user believes Soul can
    /// read it, and the difference between "authorized" and "gone" is theirs to
    /// see.
    pub fn from_config(config: &Config) -> (FilePlanSession, Vec<(String, Refusal)>) {
        let mut session = FilePlanSession::new();
        let mut rejected = Vec::new();
        for root in &config.authorized_roots {
            let shown = root.to_string_lossy().into_owned();
            if let Err(refusal) = session.authorize(&shown) {
                rejected.push((shown, refusal));
            }
        }
        (session, rejected)
    }

    /// Record that the user authorized one directory.
    pub fn authorize(&mut self, path: &str) -> Result<AuthorizedRoot, Refusal> {
        self.authorization.authorize(path)
    }

    pub fn roots(&self) -> &[AuthorizedRoot] {
        self.authorization.roots()
    }

    pub fn authorization(&self) -> &Authorization {
        &self.authorization
    }

    /// Scan an authorized directory and preview the plan for it.
    ///
    /// `origin` is a parameter and not a default. A command reached from the
    /// WebView is a click, so the shell passes [`RequestOrigin::User`]; a
    /// caller that got the path out of a file has to say so, and be refused.
    pub fn preview(
        &self,
        issuer: &mut TokenIssuer,
        path: &str,
        origin: RequestOrigin,
        limits: ScanLimits,
        now_ms: u64,
    ) -> Result<Preview, Refusal> {
        soul_fileplan::preview(&self.authorization, issuer, path, origin, limits, now_ms)
    }
}

/// What the desktop shell may display about one plan.
///
/// Not [`OrganizePlan`] itself. The shell needs the file names — they are what
/// the user is being asked to look at — but it does not need
/// [`soul_fileplan::ScannedEntry`], the modification times, or anything else
/// the plan was derived from, and every field that crosses the IPC boundary is
/// a field somebody has to justify. The reason each file was left alone is
/// rendered here rather than in TypeScript, so the explanation the user reads
/// is the one the core decided on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanPreview {
    /// The directory, as the operating system spells it.
    pub root: String,
    /// What the user is approving, if they approve anything.
    pub plan_hash: String,
    /// The directory this plan was made from. A rescan that produces a
    /// different one is a rescan of a different directory.
    pub directory_snapshot: String,
    /// Whether the directory is exactly as the scan found it. Always true, and
    /// carried across the boundary so the interface can say so.
    pub disk_unchanged: bool,
    /// Always false. See [`READ_ONLY_NOTICE`].
    pub executable_in_this_version: bool,
    pub read_only_notice: String,
    pub scanned_entries: usize,
    pub skipped_entries: usize,
    /// Whether a scan limit cut the walk short.
    pub truncated: bool,
    pub moves: Vec<ProposedMoveView>,
    pub left_alone: Vec<LeftAloneView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposedMoveView {
    pub from: String,
    pub to: String,
    pub kind: String,
    /// The group's name in the language the product speaks.
    pub kind_label: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeftAloneView {
    pub path: String,
    pub reason: String,
    pub explanation: String,
}

impl PlanPreview {
    pub fn of(preview: &Preview) -> PlanPreview {
        PlanPreview::from_plan(
            preview.plan(),
            preview.plan_hash().as_str(),
            preview.disk_unchanged(),
        )
    }

    fn from_plan(plan: &OrganizePlan, plan_hash: &str, disk_unchanged: bool) -> PlanPreview {
        PlanPreview {
            root: plan.root_display().to_owned(),
            plan_hash: plan_hash.to_owned(),
            directory_snapshot: plan.snapshot_hash().to_owned(),
            disk_unchanged,
            executable_in_this_version: plan.executable_in_this_version(),
            read_only_notice: READ_ONLY_NOTICE.to_owned(),
            scanned_entries: plan.scanned_entries(),
            skipped_entries: plan.skipped_entries(),
            truncated: plan.truncated(),
            moves: plan
                .moves()
                .iter()
                .map(|proposed| ProposedMoveView {
                    from: proposed.from_relative().to_owned(),
                    to: proposed.to_relative().to_owned(),
                    kind: proposed.kind().as_str().to_owned(),
                    kind_label: kind_label(proposed.kind()).to_owned(),
                    size_bytes: proposed.size_bytes(),
                })
                .collect(),
            left_alone: plan
                .left_alone()
                .iter()
                .map(|left| LeftAloneView {
                    path: left.relative().to_owned(),
                    reason: left.reason().as_str().to_owned(),
                    explanation: left.reason().explanation().to_owned(),
                })
                .collect(),
        }
    }
}

/// The group's name as the interface shows it. Same words as the folder the
/// plan proposes, so the label and the destination cannot drift apart.
pub fn kind_label(kind: FileKind) -> &'static str {
    kind.folder().unwrap_or("未分类")
}

/// One authorized directory, for the list the shell shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizedRootView {
    pub path: String,
}

impl AuthorizedRootView {
    pub fn of(root: &AuthorizedRoot) -> AuthorizedRootView {
        AuthorizedRootView {
            path: root.canonical().to_string_lossy().into_owned(),
        }
    }
}

/// What the shell is told when a request to carry a plan out is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRefusalView {
    pub reason_code: String,
    pub explanation: String,
    /// True when the refusal is the standing one — this build has no write
    /// path — rather than a fault in the request.
    pub write_not_implemented: bool,
}

impl ExecutionRefusalView {
    pub fn of(refusal: &ExecutionRefusal) -> ExecutionRefusalView {
        ExecutionRefusalView {
            reason_code: refusal.reason_code().as_str().to_owned(),
            explanation: refusal.explanation().to_owned(),
            write_not_implemented: refusal.is_write_not_implemented(),
        }
    }
}

/// Refuse a request to carry a plan out, and describe the refusal.
///
/// The issuer is borrowed immutably here for the same reason it is in
/// `soul-fileplan`: this path cannot spend a capability token, and the
/// signature is what says so.
pub fn refuse_execution_view(
    issuer: &TokenIssuer,
    request: &ActionRequest,
) -> ExecutionRefusalView {
    ExecutionRefusalView::of(&refuse_execution(issuer, request))
}

/// Whether a path is one this session may read, without scanning it.
///
/// For the interface's own use: a directory picker can grey out a choice
/// instead of letting the user pick it and then explaining why not.
pub fn is_readable(session: &FilePlanSession, path: &Path) -> bool {
    session
        .authorization()
        .resolve(&path.to_string_lossy())
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    use soul_policy::hitl::{ActionKind, PlanHash};
    use soul_policy::ReasonCode;

    const NOW_MS: u64 = 1_787_529_600_000;

    fn folder() -> (tempfile::TempDir, String) {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = std::fs::canonicalize(directory.path()).expect("canonical temp dir");
        std::fs::write(root.join("photo.jpg"), "jpeg-ish").expect("write");
        std::fs::write(root.join("notes.md"), "# notes").expect("write");
        std::fs::write(root.join("mystery.qqq"), "?").expect("write");
        let shown = root.to_string_lossy().into_owned();
        (directory, shown)
    }

    #[test]
    fn a_fresh_session_has_authorized_nothing() {
        let session = FilePlanSession::new();
        assert!(session.roots().is_empty());
        assert!(session.authorization().is_empty());

        let mut issuer = TokenIssuer::new();
        let refusal = session
            .preview(
                &mut issuer,
                "/",
                RequestOrigin::User,
                ScanLimits::default(),
                NOW_MS,
            )
            .expect_err("nothing is authorized yet");
        assert_eq!(refusal, Refusal::NothingAuthorized);
        assert!(!is_readable(&session, Path::new("/")));
    }

    /// The default configuration authorizes nothing, and a root that has since
    /// been deleted comes back as a refusal rather than as a shorter list.
    #[test]
    fn restoring_from_configuration_reports_what_no_longer_resolves() {
        let (keep, root) = folder();
        let config = Config {
            authorized_roots: vec![root.clone().into(), format!("{root}/gone").into()],
            ..Config::default()
        };

        let (session, rejected) = FilePlanSession::from_config(&config);
        assert_eq!(session.roots().len(), 1);
        assert_eq!(rejected.len(), 1);
        assert!(matches!(rejected[0].1, Refusal::Unreadable { .. }));

        let (empty, none) = FilePlanSession::from_config(&Config::default());
        assert!(empty.roots().is_empty());
        assert!(none.is_empty());
        drop(keep);
    }

    /// The shape the WebView binds to, and the two fields it must never see a
    /// different value for.
    #[test]
    fn the_preview_serializes_to_the_shape_the_webview_expects() {
        let (keep, root) = folder();
        let mut session = FilePlanSession::new();
        session.authorize(&root).expect("authorize");
        let mut issuer = TokenIssuer::new();

        let preview = session
            .preview(
                &mut issuer,
                &root,
                RequestOrigin::User,
                ScanLimits::default(),
                NOW_MS,
            )
            .expect("a preview");
        let view = PlanPreview::of(&preview);
        let value = serde_json::to_value(&view).expect("serialize");

        assert_eq!(
            value["executable_in_this_version"],
            serde_json::json!(false),
        );
        assert_eq!(value["disk_unchanged"], serde_json::json!(true));
        assert_eq!(
            value["read_only_notice"],
            serde_json::json!(READ_ONLY_NOTICE),
        );
        assert_eq!(
            value["plan_hash"],
            serde_json::json!(preview.plan_hash().as_str())
        );
        assert_eq!(value["scanned_entries"], serde_json::json!(3));
        assert_eq!(value["truncated"], serde_json::json!(false));

        let moves = value["moves"].as_array().expect("a list of moves");
        assert_eq!(moves.len(), 2);
        assert_eq!(moves[0]["from"], serde_json::json!("notes.md"));
        assert_eq!(moves[0]["to"], serde_json::json!("文档/notes.md"));
        assert_eq!(moves[0]["kind_label"], serde_json::json!("文档"));

        let left = value["left_alone"].as_array().expect("a list");
        assert_eq!(left.len(), 1);
        assert_eq!(left[0]["path"], serde_json::json!("mystery.qqq"));
        assert!(!left[0]["explanation"].as_str().unwrap_or("").is_empty());

        // The view round-trips, so the interface cannot be handed a field the
        // core does not know about.
        let decoded: PlanPreview = serde_json::from_value(value).expect("round trip");
        assert_eq!(decoded, view);
        drop(keep);
    }

    /// Every label the interface can show comes from the folder the plan
    /// proposes, so the two cannot drift apart.
    #[test]
    fn a_group_label_is_the_folder_it_would_be_moved_into() {
        for kind in FileKind::ALL {
            match kind.folder() {
                Some(folder) => assert_eq!(kind_label(*kind), folder),
                None => assert_eq!(kind_label(*kind), "未分类"),
            }
        }
    }

    #[test]
    fn asking_to_carry_a_plan_out_is_refused_and_says_when_it_will_not_be() {
        let issuer = TokenIssuer::new();
        let request = ActionRequest::new(ActionKind::PlanFiles.as_str(), RequestOrigin::User)
            .with_plan(serde_json::json!({"moves": []}));

        let view = refuse_execution_view(&issuer, &request);
        assert_eq!(view.reason_code, ReasonCode::WriteNotImplemented.as_str(),);
        assert!(view.write_not_implemented);
        assert!(view.explanation.contains("v0.1.1"));

        let changed = request.clone().approved_as(PlanHash::from_hex("00"));
        let view = refuse_execution_view(&issuer, &changed);
        assert_eq!(view.reason_code, ReasonCode::PlanHashMismatch.as_str());
        assert!(!view.write_not_implemented);
    }

    /// There is no command here that carries a plan out, and the check is a
    /// search rather than a habit.
    #[test]
    fn this_command_surface_offers_no_way_to_move_a_file() {
        // Everything below the test module is stripped first: this test has to
        // spell the forbidden names out in order to look for them, and it found
        // itself the first time it ran.
        const SOURCE: &str = include_str!("fileplan.rs");
        let surface = SOURCE
            .split("#[cfg(test)]")
            .next()
            .expect("the module has a body above its tests");

        for forbidden in [
            "pub fn execute",
            "pub fn apply",
            "pub fn perform",
            "fs::rename",
            "fs::write",
            "fs::remove",
        ] {
            assert!(
                !surface.contains(forbidden),
                "`{forbidden}` appears in the file-plan command surface",
            );
        }
        assert!(surface.contains("pub fn refuse_execution_view"));
        assert!(
            surface.len() > 2_000,
            "the strip above removed more than the tests",
        );
    }
}
