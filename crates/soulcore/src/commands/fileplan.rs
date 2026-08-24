//! WP11's command surface: a read-only scan, and the plan preview it feeds.
//!
//! Thin, like the rest of this directory. `soul-fileplan` decides what is on
//! disk and refuses anything outside the roots it was handed; `soul-policy`
//! decides whether the request was allowed to be made at all. What is here is
//! the order those two run in, and the audit entry each outcome deserves.
//!
//! Two things this module is careful about:
//!
//! * **no token.** `ScanDirectory` and `PlanFiles` do not need a capability
//!   token, so none is issued, presented or consumed anywhere on these paths.
//!   A file-write token would be refused on presentation in any case; v0.1
//!   has nothing to present one to.
//! * **no path in the audit chain.** A preview may show the user their own
//!   file names — it is their disk. The audit chain may not: it outlives the
//!   rows it names, and a file name can be a person's name. What lands is the
//!   identifier of the scan and a count. Not the directory's identifier
//!   either: `soul-fileplan` names a directory with a digest of its path,
//!   which is not a path but would still let a reader of the chain confirm a
//!   guess about one, so the chain gets the scan's own UUID instead.
//!
//! [`fileplan_view`] and [`FilePlanView`] put the same two calls behind one
//! screen. The third care is theirs: a refusal is the refusal's own words. An
//! unauthorized path comes back as `FilePlanError`'s `Display`, which names
//! the roots the user did authorize and never the path they asked about —
//! repeating it would turn every refusal into an answer about what exists.

use std::path::Path;

use serde::Serialize;
use soul_fileplan::{
    AuthorizedRoots, FilePlanError, FilePlanPreview, PlanAction, PlanEntry, ScanReport,
};
use soul_policy::audit::{append_or_store_error, AuditContent};
use soul_policy::hitl::{ActionKind, ActionRequest, HitlDenial, PlanHash, RequestOrigin};
use soul_policy::ReasonCode;
use soul_schema::audit::{AuditAction, AuditCounts};
use soul_store::SqlCipherStore;
use soul_store_api::types::StoreError;
use uuid::Uuid;

use crate::commands::draft::{known_identifiers, session_for};
use crate::commands::policy::PolicySession;
use crate::commands::shell::{self, Session, ViewRefused, ENDPOINT_UNUSABLE_EXPLANATION};
use crate::commands::store::StoreSlot;

/// Read one authorized directory and record that it happened.
///
/// The action check comes first, the authorization check is inside
/// `soul_fileplan::scan`, and both failures leave the same shape of audit
/// entry: one request, one refusal, with the reason code that explains which
/// gate closed. There is no partial outcome — a refused scan returns no
/// entries rather than the ones it managed to read before noticing.
pub fn scan_directory(
    session: &mut PolicySession,
    store: &mut SqlCipherStore,
    roots: &AuthorizedRoots,
    target: &Path,
    origin: RequestOrigin,
    now_ms: u64,
    at_unix_seconds: i64,
) -> Result<ScanReport, FilePlanRefusal> {
    let request = ActionRequest::new(ActionKind::ScanDirectory.as_str(), origin);

    let refusal = match session.check_action(&request, now_ms) {
        Ok(_) => match soul_fileplan::scan(roots, target) {
            Ok(report) => {
                let counted = counted(report.file_count());
                append_or_store_error(store, allowed(report.scan_id(), counted), at_unix_seconds)?;
                return Ok(report);
            }
            Err(error) => FilePlanRefusal::Path(error),
        },
        Err(denial) => FilePlanRefusal::Hitl(denial),
    };

    append_or_store_error(store, denied(refusal.reason_code()), at_unix_seconds)?;
    Err(refusal)
}

/// Turn a scan into the plan the user is asked to approve.
///
/// `approved` is the hash the user said yes to, if they have been asked yet.
/// Passing it is what makes an edit between the preview and this call fail:
/// the plan is rebuilt from the scan, hashed again, and any difference —
/// a changed target, a reordered list — is a [`HitlDenial::PlanHashMismatch`].
///
/// The preview is built before the action check because building one reads
/// nothing: it is a pure function of the report the caller already holds, and
/// the check needs the plan in order to have something to compare.
pub fn plan_files(
    session: &mut PolicySession,
    store: &mut SqlCipherStore,
    report: &ScanReport,
    approved: Option<&PlanHash>,
    origin: RequestOrigin,
    now_ms: u64,
    at_unix_seconds: i64,
) -> Result<FilePlanPreview, FilePlanRefusal> {
    let preview = soul_fileplan::plan(report);

    let mut request = ActionRequest::new(ActionKind::PlanFiles.as_str(), origin)
        .with_plan(preview.to_plan_json());
    if let Some(hash) = approved {
        request = request.approved_as(hash.clone());
    }

    match session.check_action(&request, now_ms) {
        Ok(action) => {
            let counted = counted(preview.len());
            let content = allowed(preview.scan_id(), counted).for_plan(action.plan_hash.as_str());
            append_or_store_error(store, content, at_unix_seconds)?;
            Ok(preview)
        }
        Err(denial) => {
            let refusal = FilePlanRefusal::Hitl(denial);
            append_or_store_error(store, denied(refusal.reason_code()), at_unix_seconds)?;
            Err(refusal)
        }
    }
}

/// Why a scan or a plan did not happen.
#[derive(Debug, thiserror::Error)]
pub enum FilePlanRefusal {
    #[error(transparent)]
    Hitl(#[from] HitlDenial),
    #[error(transparent)]
    Path(#[from] FilePlanError),
    /// The work was refused correctly and the record of it could not be
    /// written. Reported rather than swallowed: an unrecorded decision is
    /// worse than a failed call.
    #[error(transparent)]
    Audit(#[from] StoreError),
}

impl FilePlanRefusal {
    pub fn reason_code(&self) -> ReasonCode {
        match self {
            FilePlanRefusal::Hitl(denial) => denial.reason_code(),
            FilePlanRefusal::Path(error) => error.reason_code(),
            FilePlanRefusal::Audit(_) => ReasonCode::Routine,
        }
    }
}

/// The entry a successful request leaves: which scan, and how many things.
/// No path, no name, no plan body.
fn allowed(scan_id: Uuid, counts: AuditCounts) -> AuditContent {
    AuditContent::allowed(AuditAction::FilePlan, ReasonCode::Routine)
        .about(&[scan_id])
        .counting(counts)
}

/// The entry a refused request leaves. One request in, one refusal recorded,
/// so the two can be counted against each other.
fn denied(reason: ReasonCode) -> AuditContent {
    AuditContent::denied(AuditAction::FilePlan, reason).counting(counted(1))
}

fn counted(items: usize) -> AuditCounts {
    AuditCounts {
        items: Some(items as u64),
        bytes: None,
    }
}

// --------------------------------------------------------------- the view

/// What the file page says under the suggestions, whatever they are.
///
/// True by construction rather than by intent: this crate has no call that
/// moves, renames or removes a file, `soul-fileplan` has no `execute`, and the
/// one capability that would authorise a write is refused on issue. Carrying
/// a plan out is v0.1.1, and the sentence says so rather than leaving the user
/// to wonder whether the button is missing or the feature is.
pub const PLAN_PREVIEW_ONLY_EXPLANATION: &str =
    "以下只是建议。本版本没有执行它的代码路径：一个文件都不会被移动、改名或删除，\
     磁盘上的东西保持原样。真正动手整理是 v0.1.1 的事。";

/// The three suggestions, in the words the page shows.
///
/// Chinese here rather than in the WebView for the reason every other notice
/// in this crate is: the words are part of what the product promises — 归类
/// and 移动 are suggestions, not actions taken — and a translation table in
/// TypeScript is one `cargo test` cannot read.
pub const GROUP_ACTION_LABEL: &str = "归类";
pub const MOVE_ACTION_LABEL: &str = "移动";
pub const RENAME_ACTION_LABEL: &str = "重命名";

/// One suggestion, in the shape a WebView may hold.
///
/// Relative paths, because that is what the plan is about and it is the user's
/// own disk. The audit chain gets neither; see the module note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FilePlanEntryView {
    source_rel: String,
    /// `"group"`, `"move"` or `"rename"`, from [`PlanAction::as_str`].
    action: String,
    /// The same action in the word the page shows.
    action_label: String,
    target_rel: Option<String>,
}

impl FilePlanEntryView {
    fn of(entry: &PlanEntry) -> FilePlanEntryView {
        FilePlanEntryView {
            source_rel: encode(entry.source_rel()),
            action: entry.action().as_str().to_owned(),
            action_label: action_label(entry.action()).to_owned(),
            target_rel: entry.target_rel().map(encode),
        }
    }
}

/// One scan and the plan it produced, in the shape a WebView may hold.
///
/// Private fields and one constructor, so `written_to_disk` is the same kind
/// of claim as `FilePlanPreview::written_to_disk`: not a field a caller sets,
/// therefore not a field a later caller can be persuaded to set. `Serialize`
/// and not `Deserialize`, so the value cannot be parsed back out of a document
/// that says something else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FilePlanView {
    scan_id: String,
    file_count: usize,
    dir_count: usize,
    skipped_escaping_links: usize,
    entry_count: usize,
    group_count: usize,
    move_count: usize,
    rename_count: usize,
    entries: Vec<FilePlanEntryView>,
    /// Always false; see the type note.
    written_to_disk: bool,
    notice: String,
}

impl FilePlanView {
    pub fn of(report: &ScanReport, preview: &FilePlanPreview) -> FilePlanView {
        FilePlanView {
            scan_id: preview.scan_id().to_string(),
            file_count: report.file_count(),
            dir_count: report.dir_count(),
            skipped_escaping_links: report.skipped_escaping_links(),
            entry_count: preview.len(),
            group_count: preview.count_of(PlanAction::Group),
            move_count: preview.count_of(PlanAction::Move),
            rename_count: preview.count_of(PlanAction::Rename),
            entries: preview
                .entries()
                .iter()
                .map(FilePlanEntryView::of)
                .collect(),
            written_to_disk: false,
            notice: PLAN_PREVIEW_ONLY_EXPLANATION.to_owned(),
        }
    }
}

const fn action_label(action: PlanAction) -> &'static str {
    match action {
        PlanAction::Group => GROUP_ACTION_LABEL,
        PlanAction::Move => MOVE_ACTION_LABEL,
        PlanAction::Rename => RENAME_ACTION_LABEL,
    }
}

/// A relative path as one string, components joined with `/`.
///
/// The same encoding `FilePlanPreview::to_plan_json` uses, so what the page
/// shows and what an approval would be taken over spell a path the same way on
/// every host. `Path::display` would not: it keeps the separator the platform
/// happens to use.
fn encode(rel_path: &Path) -> String {
    rel_path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join("/")
}

/// Scan one directory and show what tidying it would look like.
///
/// The order is fixed. The store comes first, because both calls below record
/// what they did. The roots are re-resolved from the session's configuration
/// rather than taken from the caller: the WebView asks about a directory, and
/// which directories may be asked about is a decision this crate keeps.
///
/// `approved` is `None` on this path. The user is being shown the plan for the
/// first time, so there is no earlier hash to hold it to — and there is no
/// second call that would carry it out anyway.
pub fn fileplan_view(
    slot: &StoreSlot,
    session: &Session,
    target: String,
) -> Result<FilePlanView, ViewRefused> {
    let Some(mut store) = slot.lock() else {
        return Err(ViewRefused::no_store_opened());
    };

    let config = session.config();
    let roots = AuthorizedRoots::canonicalized(&config.authorized_roots)
        .map_err(|error| as_refused(&FilePlanRefusal::Path(error)))?;
    let identifiers =
        known_identifiers(&store).map_err(|error| ViewRefused::refused(None, error.to_string()))?;
    let mut policy = session_for(&config, identifiers)
        .map_err(|_| ViewRefused::refused(None, ENDPOINT_UNUSABLE_EXPLANATION))?;

    let (now_ms, at_unix_seconds) = shell::wall_clock();
    let report = scan_directory(
        &mut policy,
        &mut store,
        &roots,
        Path::new(&target),
        RequestOrigin::User,
        now_ms,
        at_unix_seconds,
    )
    .map_err(|refusal| as_refused(&refusal))?;
    let preview = plan_files(
        &mut policy,
        &mut store,
        &report,
        None,
        RequestOrigin::User,
        now_ms,
        at_unix_seconds,
    )
    .map_err(|refusal| as_refused(&refusal))?;

    Ok(FilePlanView::of(&report, &preview))
}

/// A refusal in its own words, with the code the chain recorded beside it.
///
/// `to_string` and not a sentence assembled here: `FilePlanError` is written
/// so that the refusal names the authorized roots and not the path that was
/// asked about, and anything this layer composed would have to re-earn that.
fn as_refused(refusal: &FilePlanRefusal) -> ViewRefused {
    ViewRefused::refused(Some(refusal.reason_code()), refusal.to_string())
}
