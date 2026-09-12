//! Human in the loop: what may be asked for, against which approved plan, and
//! with which one-time token.
//!
//! DECISIONS D24 gives three rules and AC-19 tests all three at once: an
//! unknown action is refused, a plan whose hash changed after approval is
//! refused, and a capability token is good for one use. This module adds the
//! two the rest of the lock implies — a token is scoped, and a token expires —
//! and one v0.1 gate: a file-write token is refused even when it is otherwise
//! perfect, because v0.1 has no write implementation and a token that would
//! start working the day one appears is a trap.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::audit::ReasonCode;

/// Every action the agent layer is allowed to ask for in v0.1.
///
/// A closed enum rather than a string is the whole of "unknown actions are
/// refused": there is no way to express one past [`ActionKind::parse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionKind {
    /// Produce a draft reply. Never sends it.
    DraftReply,
    /// Summarize a person or a situation from stored evidence.
    AnalysePeople,
    /// Read-only scan of an authorized directory.
    ScanDirectory,
    /// Produce a file-organisation plan. Read-only; v0.1 never executes it.
    PlanFiles,
    /// Show what forgetting something would cost.
    PreviewForget,
    /// Destroy content keys.
    ExecuteForget,
    /// Show what the research track would see. Writes nothing.
    PreviewResearch,
    /// Send one generation request to the user's own endpoint.
    GenerateWithUserEndpoint,
}

impl ActionKind {
    pub const ALL: &'static [ActionKind] = &[
        ActionKind::DraftReply,
        ActionKind::AnalysePeople,
        ActionKind::ScanDirectory,
        ActionKind::PlanFiles,
        ActionKind::PreviewForget,
        ActionKind::ExecuteForget,
        ActionKind::PreviewResearch,
        ActionKind::GenerateWithUserEndpoint,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            ActionKind::DraftReply => "draft.reply",
            ActionKind::AnalysePeople => "analyse.people",
            ActionKind::ScanDirectory => "scan.directory",
            ActionKind::PlanFiles => "plan.files",
            ActionKind::PreviewForget => "forget.preview",
            ActionKind::ExecuteForget => "forget.execute",
            ActionKind::PreviewResearch => "research.preview",
            ActionKind::GenerateWithUserEndpoint => "egress.generate",
        }
    }

    /// The gate an action name from outside this crate has to pass.
    pub fn parse(name: &str) -> Option<ActionKind> {
        ActionKind::ALL.iter().copied().find(|k| k.as_str() == name)
    }

    /// Actions that cannot proceed on the user's standing consent alone.
    pub fn needs_capability_token(self) -> bool {
        matches!(
            self,
            ActionKind::ExecuteForget | ActionKind::GenerateWithUserEndpoint
        )
    }
}

/// What a token is good for. Narrower than [`ActionKind`] on purpose: a token
/// names a capability, not a menu entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityScope {
    /// One generation request to the configured endpoint.
    E1Generate,
    /// Destroy one forget unit's content keys.
    ForgetExecute,
    /// Write to a file. v0.1 never honours this; see [`TokenError::WriteNotImplemented`].
    FileWrite,
}

impl CapabilityScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            CapabilityScope::E1Generate => "e1.generate",
            CapabilityScope::ForgetExecute => "forget.execute",
            CapabilityScope::FileWrite => "file.write",
        }
    }

    /// v0.1 has no file-write implementation, so a token for it is refused on
    /// presentation rather than at some later gate.
    pub fn is_implemented_in_v0_1(self) -> bool {
        !matches!(self, CapabilityScope::FileWrite)
    }
}

/// Where a request came from. External content is never authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestOrigin {
    /// The user clicked something.
    User,
    /// The request was derived from imported, pasted or scanned content.
    ExternalContent,
}

/// SHA-256 over the plan the user approved.
///
/// Recomputed from the plan at execution time and compared with what was
/// approved. Any edit, including one the UI made, changes it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanHash(String);

impl PlanHash {
    /// Hash the canonical JSON encoding of a plan.
    pub fn of(plan: &serde_json::Value) -> PlanHash {
        // `serde_json::Value`'s map is a BTreeMap here, so the encoding is
        // key-sorted and the digest is stable across processes.
        let canonical = serde_json::to_vec(plan).unwrap_or_default();
        PlanHash(hex::encode(Sha256::digest(canonical)))
    }

    pub fn from_hex(hex: impl Into<String>) -> PlanHash {
        PlanHash(hex.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A capability token, as handed to the caller.
///
/// The secret is the identifier: it is a UUIDv7 minted by [`TokenIssuer`] and
/// only that issuer's ledger knows it. There is no signature to verify because
/// the token never leaves the process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityToken {
    token_id: Uuid,
    scope: CapabilityScope,
    plan_hash: PlanHash,
    issued_at_ms: u64,
    expires_at_ms: u64,
}

impl CapabilityToken {
    pub fn token_id(&self) -> Uuid {
        self.token_id
    }

    pub fn scope(&self) -> CapabilityScope {
        self.scope
    }

    pub fn plan_hash(&self) -> &PlanHash {
        &self.plan_hash
    }

    pub fn issued_at_ms(&self) -> u64 {
        self.issued_at_ms
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }

    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TokenError {
    #[error("no capability token with id {0} was issued by this ledger")]
    Unknown(Uuid),
    #[error("capability token {0} has already been spent")]
    Replayed(Uuid),
    #[error("capability token {token_id} expired at {expires_at_ms} and it is now {now_ms}")]
    Expired {
        token_id: Uuid,
        expires_at_ms: u64,
        now_ms: u64,
    },
    #[error("capability token {token_id} is scoped to {issued} and was presented for {presented}")]
    ScopeMismatch {
        token_id: Uuid,
        issued: &'static str,
        presented: &'static str,
    },
    #[error("capability token {token_id} was approved for a different plan")]
    PlanChanged { token_id: Uuid },
    #[error("v0.1 has no file-write implementation, so capability token {0} is refused")]
    WriteNotImplemented(Uuid),
}

impl TokenError {
    pub fn reason_code(&self) -> ReasonCode {
        match self {
            TokenError::Unknown(_) => ReasonCode::TokenUnknown,
            TokenError::Replayed(_) => ReasonCode::TokenReplayed,
            TokenError::Expired { .. } => ReasonCode::TokenExpired,
            TokenError::ScopeMismatch { .. } => ReasonCode::TokenScopeMismatch,
            TokenError::PlanChanged { .. } => ReasonCode::PlanHashMismatch,
            TokenError::WriteNotImplemented(_) => ReasonCode::WriteNotImplemented,
        }
    }

    pub fn token_id(&self) -> Uuid {
        match self {
            TokenError::Unknown(id)
            | TokenError::Replayed(id)
            | TokenError::WriteNotImplemented(id)
            | TokenError::Expired { token_id: id, .. }
            | TokenError::ScopeMismatch { token_id: id, .. }
            | TokenError::PlanChanged { token_id: id } => *id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenState {
    Live,
    Spent,
}

/// Mints tokens and remembers exactly once that each was spent.
///
/// The ledger keeps spent tokens rather than deleting them so that a replay is
/// reported as a replay instead of as an unknown token; the two say different
/// things to the person reading the audit trail.
#[derive(Debug, Default)]
pub struct TokenIssuer {
    ledger: BTreeMap<Uuid, (CapabilityToken, TokenState)>,
}

/// How long a freshly issued token is good for, unless the caller says
/// otherwise. Long enough to read a confirmation dialog, short enough that a
/// token left in a log is worthless.
pub const DEFAULT_TOKEN_TTL_MS: u64 = 120_000;

impl TokenIssuer {
    pub fn new() -> TokenIssuer {
        TokenIssuer::default()
    }

    pub fn issue(
        &mut self,
        scope: CapabilityScope,
        plan_hash: PlanHash,
        now_ms: u64,
    ) -> CapabilityToken {
        self.issue_with_ttl(scope, plan_hash, now_ms, DEFAULT_TOKEN_TTL_MS)
    }

    pub fn issue_with_ttl(
        &mut self,
        scope: CapabilityScope,
        plan_hash: PlanHash,
        now_ms: u64,
        ttl_ms: u64,
    ) -> CapabilityToken {
        let token = CapabilityToken {
            token_id: Uuid::now_v7(),
            scope,
            plan_hash,
            issued_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
        };
        self.ledger
            .insert(token.token_id, (token.clone(), TokenState::Live));
        token
    }

    /// Spend a token for one use.
    ///
    /// Order matters: the ledger is consulted first, so a replay is reported
    /// as a replay even when the token has also expired, and the token is
    /// marked spent on every path that got as far as recognising it. A caller
    /// that presents a token and is refused does not get to try again.
    pub fn consume(
        &mut self,
        token_id: Uuid,
        scope: CapabilityScope,
        plan_hash: &PlanHash,
        now_ms: u64,
    ) -> Result<SpentToken, TokenError> {
        let Some((token, state)) = self.ledger.get_mut(&token_id) else {
            return Err(TokenError::Unknown(token_id));
        };
        if *state == TokenState::Spent {
            return Err(TokenError::Replayed(token_id));
        }
        *state = TokenState::Spent;

        let token = token.clone();
        if !token.scope.is_implemented_in_v0_1() {
            return Err(TokenError::WriteNotImplemented(token_id));
        }
        if token.scope != scope {
            return Err(TokenError::ScopeMismatch {
                token_id,
                issued: token.scope.as_str(),
                presented: scope.as_str(),
            });
        }
        if token.is_expired_at(now_ms) {
            return Err(TokenError::Expired {
                token_id,
                expires_at_ms: token.expires_at_ms,
                now_ms,
            });
        }
        if &token.plan_hash != plan_hash {
            return Err(TokenError::PlanChanged { token_id });
        }

        Ok(SpentToken {
            token_id,
            scope,
            plan_hash: plan_hash.clone(),
        })
    }

    pub fn is_spent(&self, token_id: Uuid) -> bool {
        matches!(self.ledger.get(&token_id), Some((_, TokenState::Spent)))
    }

    pub fn issued_count(&self) -> usize {
        self.ledger.len()
    }
}

/// Proof that a token was accepted and burnt. Cannot be constructed elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpentToken {
    token_id: Uuid,
    scope: CapabilityScope,
    plan_hash: PlanHash,
}

impl SpentToken {
    pub fn token_id(&self) -> Uuid {
        self.token_id
    }

    pub fn scope(&self) -> CapabilityScope {
        self.scope
    }

    pub fn plan_hash(&self) -> &PlanHash {
        &self.plan_hash
    }
}

/// One request to do something.
#[derive(Debug, Clone)]
pub struct ActionRequest {
    /// The action name as it arrived. A string on purpose: this is the point
    /// where an unknown one has to be caught.
    pub action: String,
    pub origin: RequestOrigin,
    /// The plan as it stands now. Hashed and compared with the approval.
    pub plan: serde_json::Value,
    /// The hash the user approved, if this action needed approval.
    pub approved_plan_hash: Option<PlanHash>,
    pub token_id: Option<Uuid>,
}

impl ActionRequest {
    pub fn new(action: impl Into<String>, origin: RequestOrigin) -> ActionRequest {
        ActionRequest {
            action: action.into(),
            origin,
            plan: serde_json::Value::Null,
            approved_plan_hash: None,
            token_id: None,
        }
    }

    pub fn with_plan(mut self, plan: serde_json::Value) -> ActionRequest {
        self.plan = plan;
        self
    }

    pub fn approved_as(mut self, hash: PlanHash) -> ActionRequest {
        self.approved_plan_hash = Some(hash);
        self
    }

    pub fn with_token(mut self, token_id: Uuid) -> ActionRequest {
        self.token_id = Some(token_id);
        self
    }

    pub fn plan_hash(&self) -> PlanHash {
        PlanHash::of(&self.plan)
    }
}

/// An action that passed every gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedAction {
    pub kind: ActionKind,
    pub plan_hash: PlanHash,
    pub spent_token: Option<SpentToken>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HitlDenial {
    #[error("`{0}` is not an action this build knows how to perform")]
    UnknownAction(String),
    #[error("external content cannot authorize an action")]
    ExternalContentNotAuthority,
    #[error("the plan changed after it was approved: approved {approved}, now {current}")]
    PlanHashMismatch { approved: String, current: String },
    #[error("{0} needs a capability token and none was presented")]
    TokenRequired(&'static str),
    #[error(transparent)]
    Token(#[from] TokenError),
}

impl HitlDenial {
    pub fn reason_code(&self) -> ReasonCode {
        match self {
            HitlDenial::UnknownAction(_) => ReasonCode::UnknownAction,
            HitlDenial::ExternalContentNotAuthority => ReasonCode::ExternalContentNotAuthority,
            HitlDenial::PlanHashMismatch { .. } => ReasonCode::PlanHashMismatch,
            HitlDenial::TokenRequired(_) => ReasonCode::TokenRequired,
            HitlDenial::Token(error) => error.reason_code(),
        }
    }
}

/// The gate itself.
///
/// Checks run in the order a reviewer would ask about them: is this a real
/// action, did a person ask for it, is it still the plan they approved, and is
/// the token good. Every refusal carries a [`ReasonCode`] so the audit entry
/// can be written without prose.
pub fn check_action(
    issuer: &mut TokenIssuer,
    request: &ActionRequest,
    now_ms: u64,
) -> Result<ApprovedAction, HitlDenial> {
    let Some(kind) = ActionKind::parse(&request.action) else {
        return Err(HitlDenial::UnknownAction(request.action.clone()));
    };

    if request.origin == RequestOrigin::ExternalContent {
        return Err(HitlDenial::ExternalContentNotAuthority);
    }

    let current = request.plan_hash();
    if let Some(approved) = request.approved_plan_hash.as_ref() {
        if approved != &current {
            return Err(HitlDenial::PlanHashMismatch {
                approved: approved.as_str().to_owned(),
                current: current.as_str().to_owned(),
            });
        }
    }

    if !kind.needs_capability_token() {
        return Ok(ApprovedAction {
            kind,
            plan_hash: current,
            spent_token: None,
        });
    }

    let Some(token_id) = request.token_id else {
        return Err(HitlDenial::TokenRequired(kind.as_str()));
    };
    let scope = match kind {
        ActionKind::ExecuteForget => CapabilityScope::ForgetExecute,
        _ => CapabilityScope::E1Generate,
    };
    let spent = issuer.consume(token_id, scope, &current, now_ms)?;

    Ok(ApprovedAction {
        kind,
        plan_hash: current,
        spent_token: Some(spent),
    })
}
