//! What happens when something asks for a plan to be carried out.
//!
//! It is refused, and the interesting part is how the refusal is shaped.
//! [`refuse_execution`] returns [`ExecutionRefusal`] — not a `Result`, so there
//! is no success variant for a future caller to construct, no `Ok` arm for one
//! to fill in, and nothing to unwrap. "v0.1 does not write files" is the return
//! type rather than a branch inside it.
//!
//! AC-19 asks for three refusals and this is the file-plan surface's answer to
//! all three: an unknown action, a plan whose hash moved after approval, and a
//! replayed capability token. Each gets its own [`ReasonCode`], because an
//! audit trail that recorded them identically would not let anyone tell a user
//! editing a plan apart from a token being reused.
//!
//! The issuer is borrowed immutably, and that is load-bearing. WP08 promised
//! that v0.1 never consumes a file-write token; a function that cannot take a
//! mutable borrow of the ledger cannot spend anything in it, whatever it is
//! handed. So a presented token is *inspected* — has this one already been
//! spent? — and then refused along with everything else.

use soul_policy::audit::AuditContent;
use soul_policy::hitl::{ActionKind, ActionRequest, RequestOrigin, TokenIssuer};
use soul_policy::ReasonCode;
use soul_schema::audit::AuditAction;

/// The actions this crate answers for. Everything else, including actions
/// other work packages know about, is unknown *here*: a file-plan surface that
/// accepted `forget.execute` would be a second door into forgetting.
pub const FILEPLAN_ACTIONS: &[ActionKind] = &[ActionKind::ScanDirectory, ActionKind::PlanFiles];

/// A refusal to carry out a file plan. There is no other outcome.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{}", self.explanation())]
pub struct ExecutionRefusal {
    reason: ReasonCode,
    /// Set when the refusal was about a token rather than about the request,
    /// so the audit entry can be filed under `capability.reject`.
    about_a_token: bool,
}

impl ExecutionRefusal {
    pub fn reason_code(&self) -> ReasonCode {
        self.reason
    }

    /// What the user is told. Says what was refused and, when the answer is
    /// simply "this version does not do that", says when it will.
    pub fn explanation(&self) -> &'static str {
        match self.reason {
            ReasonCode::UnknownAction => "这不是本版本认得的动作",
            ReasonCode::ExternalContentNotAuthority => "文件里的内容不能授权任何动作",
            ReasonCode::PlanHashMismatch => "计划在你批准之后变过了，请重新看一遍再决定",
            ReasonCode::TokenReplayed => "这张一次性凭据已经用过了",
            _ => "v0.1 只做只读扫描与计划预览；文件写入是 v0.1.1 的事",
        }
    }

    /// Whether the refusal is the standing one — this build has no write path
    /// — rather than a fault in the request.
    pub fn is_write_not_implemented(&self) -> bool {
        self.reason == ReasonCode::WriteNotImplemented
    }

    pub fn audit(&self) -> AuditContent {
        let action = match self.about_a_token {
            true => AuditAction::CapabilityReject,
            false => AuditAction::FilePlan,
        };
        AuditContent::denied(action, self.reason)
    }
}

/// Refuse to carry out a plan, and say which of the reasons applies first.
///
/// The order is the order a reviewer would ask about them: is this an action
/// this surface knows, did a person ask for it, is it still the plan they
/// approved, is the token one that has already been used — and then, when
/// nothing at all is wrong with the request, the refusal that is always true.
///
/// That last case is the one worth reading twice. A perfectly formed request,
/// with a live token, against an unmodified plan, is refused. There is no
/// combination of inputs that returns anything else.
pub fn refuse_execution(issuer: &TokenIssuer, request: &ActionRequest) -> ExecutionRefusal {
    let Some(kind) = ActionKind::parse(&request.action) else {
        return ExecutionRefusal {
            reason: ReasonCode::UnknownAction,
            about_a_token: false,
        };
    };
    if !FILEPLAN_ACTIONS.contains(&kind) {
        return ExecutionRefusal {
            reason: ReasonCode::UnknownAction,
            about_a_token: false,
        };
    }
    if request.origin == RequestOrigin::ExternalContent {
        return ExecutionRefusal {
            reason: ReasonCode::ExternalContentNotAuthority,
            about_a_token: false,
        };
    }
    if let Some(approved) = request.approved_plan_hash.as_ref() {
        if approved != &request.plan_hash() {
            return ExecutionRefusal {
                reason: ReasonCode::PlanHashMismatch,
                about_a_token: false,
            };
        }
    }
    if let Some(token_id) = request.token_id {
        if issuer.is_spent(token_id) {
            return ExecutionRefusal {
                reason: ReasonCode::TokenReplayed,
                about_a_token: true,
            };
        }
    }
    ExecutionRefusal {
        reason: ReasonCode::WriteNotImplemented,
        about_a_token: request.token_id.is_some(),
    }
}
