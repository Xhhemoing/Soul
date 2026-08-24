//! The one thing WP11 actually does: look, and say what tidying would mean.
//!
//! The order of the gates is the argument. Permission first, because whether a
//! request may be made at all does not depend on what is on the disk — and
//! because a request that came out of a file's own contents has to be turned
//! away before that file's contents get to choose a directory. Authorization
//! second, so the walk never starts outside a root the user named. The disk
//! last, and only read.
//!
//! Two HITL checks rather than one, because they answer different questions:
//! `scan.directory` asks whether this build may look at a directory at all, and
//! `plan.files` fixes the plan hash the user is about to be shown. Neither
//! action needs a capability token — `ActionKind::needs_capability_token` says
//! so — which is how "v0.1 issues and consumes no file-write token" survives a
//! preview happening. Nothing on this path can reach a ledger entry.

use soul_policy::audit::AuditContent;
use soul_policy::hitl::{
    check_action, ActionKind, ActionRequest, ApprovedAction, PlanHash, RequestOrigin, TokenIssuer,
};
use soul_policy::ReasonCode;
use soul_schema::audit::{AuditAction, AuditCounts, AuditDecision};

use crate::authorize::Authorization;
use crate::plan::{self, OrganizePlan};
use crate::refusal::Refusal;
use crate::scan::{self, DirectoryScan, ScanLimits};

/// What one look at one directory produced.
#[derive(Debug, Clone)]
pub struct Preview {
    scan: DirectoryScan,
    plan: OrganizePlan,
    plan_hash: PlanHash,
    approved: ApprovedAction,
}

impl Preview {
    pub fn scan(&self) -> &DirectoryScan {
        &self.scan
    }

    pub fn plan(&self) -> &OrganizePlan {
        &self.plan
    }

    /// The hash the user is approving. Hand it back with any later request
    /// about this plan; a rescan that produced different moves produces a
    /// different hash, and the request is refused.
    pub fn plan_hash(&self) -> &PlanHash {
        &self.plan_hash
    }

    pub fn approved(&self) -> &ApprovedAction {
        &self.approved
    }

    /// Whether the directory is exactly as it was before the preview ran.
    pub fn disk_unchanged(&self) -> bool {
        self.scan.disk_unchanged()
    }

    /// A plan was previewed: how many moves it proposes, and its hash. Not one
    /// file name, and not the directory.
    pub fn audit(&self) -> AuditContent {
        AuditContent::new(AuditAction::FilePlan, AuditDecision::Allowed)
            .because(ReasonCode::Routine)
            .for_plan(self.plan_hash.as_str())
            .counting(AuditCounts {
                items: Some(self.plan.moves().len() as u64),
                bytes: None,
            })
    }
}

/// Scan an authorized directory and preview the plan for it.
pub fn preview(
    authorization: &Authorization,
    issuer: &mut TokenIssuer,
    raw: &str,
    origin: RequestOrigin,
    limits: ScanLimits,
    now_ms: u64,
) -> Result<Preview, Refusal> {
    // The value hashed for this gate carries no path: whether Soul may look at
    // a directory is not a question about which one, and this hash goes nowhere
    // the user can compare it against.
    let scan_request = ActionRequest::new(ActionKind::ScanDirectory.as_str(), origin).with_plan(
        serde_json::json!({
            "action": ActionKind::ScanDirectory.as_str(),
            "read_only": true,
            "max_depth": limits.max_depth,
            "max_entries": limits.max_entries,
        }),
    );
    check_action(issuer, &scan_request, now_ms)?;

    let scan = scan::scan(authorization, raw, limits)?;
    let plan = plan::build(&scan);

    let plan_request =
        ActionRequest::new(ActionKind::PlanFiles.as_str(), origin).with_plan(plan.to_json());
    let approved = check_action(issuer, &plan_request, now_ms)?;
    let plan_hash = approved.plan_hash.clone();

    Ok(Preview {
        scan,
        plan,
        plan_hash,
        approved,
    })
}
