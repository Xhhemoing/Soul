//! Every way this crate says no, and what the audit chain records when it does.
//!
//! One note about vocabulary, because it is a compromise rather than a design.
//! `soul_policy::ReasonCode` is a closed list and WP08 froze it; there is no
//! `PATH_NOT_AUTHORIZED` in it and WP11 is not the work package that widens a
//! permission vocabulary. `CONSENT_MISSING` is used instead, and it is a true
//! statement about every authorization refusal here: the user never consented
//! to that directory, which is the whole of why the answer is no. The two
//! refusals that are not about authorization — the path is unreadable, or it is
//! a file where a directory was expected — record as `ROUTINE`, because the
//! product refused nothing; the disk did.
//!
//! [`Refusal::audit`] never carries the path. `AuditContent` would reject a
//! prose field outright, and a path is prose: it is the user's directory names,
//! and on a shared machine it is somebody's real name.

use soul_policy::audit::AuditContent;
use soul_policy::hitl::HitlDenial;
use soul_policy::ReasonCode;
use soul_schema::audit::AuditAction;

use crate::screen::PathDefect;

/// Why a read-only file operation did not happen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("还没有授权任何目录，所以没有可读的地方")]
    NothingAuthorized,

    #[error("这不是本版本会接受的路径写法：{0}")]
    Malformed(#[from] PathDefect),

    #[error("`{shown}` 不在任何已授权目录里")]
    OutsideAuthorizedRoot { shown: String },

    #[error("`{shown}` 经由符号链接离开了授权目录")]
    SymlinkEscapesRoot { shown: String },

    #[error("`{shown}` 经过符号链接，本版本不跟随链接")]
    SymlinkNotFollowed { shown: String },

    #[error("`{shown}` 读不到：可能不存在，也可能当前用户看不见它")]
    Unreadable { shown: String },

    #[error("`{shown}` 不是目录")]
    NotADirectory { shown: String },

    #[error(transparent)]
    Hitl(#[from] HitlDenial),
}

impl Refusal {
    /// Whether the answer was no because nobody authorized that place.
    ///
    /// AC-18's "one hundred percent" is counted over these: a test that asked
    /// for something under B and got `Unreadable` back would have proved that B
    /// was missing, not that it was refused.
    pub fn is_authorization_refusal(&self) -> bool {
        matches!(
            self,
            Refusal::NothingAuthorized
                | Refusal::Malformed(_)
                | Refusal::OutsideAuthorizedRoot { .. }
                | Refusal::SymlinkEscapesRoot { .. }
                | Refusal::SymlinkNotFollowed { .. }
        )
    }

    pub fn reason_code(&self) -> ReasonCode {
        match self {
            Refusal::Hitl(denial) => denial.reason_code(),
            Refusal::Unreadable { .. } | Refusal::NotADirectory { .. } => ReasonCode::Routine,
            _ => ReasonCode::ConsentMissing,
        }
    }

    /// The audit entry this refusal deserves: that a file plan was asked for,
    /// that it was denied, and under which code. No path, no file name.
    pub fn audit(&self) -> AuditContent {
        let action = match self {
            Refusal::Hitl(HitlDenial::Token(_)) => AuditAction::CapabilityReject,
            Refusal::Hitl(_) => AuditAction::HitlDeny,
            _ => AuditAction::FilePlan,
        };
        AuditContent::denied(action, self.reason_code())
    }
}
