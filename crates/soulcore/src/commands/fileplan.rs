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

use std::path::Path;

use soul_fileplan::{AuthorizedRoots, FilePlanError, FilePlanPreview, ScanReport};
use soul_policy::audit::{append_or_store_error, AuditContent};
use soul_policy::hitl::{ActionKind, ActionRequest, HitlDenial, PlanHash, RequestOrigin};
use soul_policy::ReasonCode;
use soul_schema::audit::{AuditAction, AuditCounts};
use soul_store::SqlCipherStore;
use soul_store_api::types::StoreError;
use uuid::Uuid;

use crate::commands::policy::PolicySession;

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
