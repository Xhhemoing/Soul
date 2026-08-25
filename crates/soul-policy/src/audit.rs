//! Building audit entries that carry no prose, and appending them through the
//! storage boundary.
//!
//! WP02 settled the division of labour and this module takes the other half of
//! it: the store owns `seq`, `prev_hash` and `entry_hash`, and refuses to let a
//! caller supply its own links; everything else about an entry is decided here.
//! What that leaves to enforce is the promise in `docs/SECURITY.md` — the chain
//! records that something happened, never what was said.
//!
//! Two independent nets, because the contract alone is not enough:
//!
//! 1. `docs/schemas/audit.schema.json` is `additionalProperties: false`, so a
//!    stray field fails validation inside the store;
//! 2. [`AuditContent::check`] refuses the prose-shaped field names outright and
//!    constrains `reason_code` to the closed [`ReasonCode`] vocabulary, so a
//!    future schema change cannot quietly open a channel.
//!
//! The second net is what AC-23 leans on: the serialized chain is fed to
//! `soul_testkit::LeakageChecker` in `tests/audit_chain.rs`, and the checker
//! only has something to find if a builder somewhere put it there.

use std::fmt;

use uuid::Uuid;

use soul_schema::audit::{
    AuditAction, AuditCounts, AuditDecision, EgressClass as AuditEgressClass, SoulAuditEntry,
};
use soul_schema::common::{SchemaVersion, Sha256Hex, Timestamp};
use soul_store_api::types::StoreResult;
use soul_store_api::AuditLog;

use crate::clock::rfc3339_utc;

/// Field names that would carry prose. None of them may appear in a
/// serialized audit entry, at any depth.
///
/// PRODUCT_LOCK names `body`, `text`, `content`, `quote`, `summary` and
/// `prompt`; the rest are the obvious spellings of the same thing.
pub const FORBIDDEN_FIELDS: &[&str] = &[
    "body",
    "text",
    "content",
    "quote",
    "summary",
    "prompt",
    "message",
    "excerpt",
    "snippet",
    "plaintext",
    "display_name",
    "title",
];

/// The closed vocabulary of `reason_code`.
///
/// `audit.schema.json` only constrains the shape (`^[A-Z][A-Z0-9_]{2,63}$`),
/// which a sentence in shouting case would satisfy. An enum is what actually
/// keeps prose out of the field, and it doubles as the list of reasons the
/// product is prepared to explain in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReasonCode {
    // ---- egress ----
    /// Neither loopback nor the configured endpoint: business egress.
    E0NoCodePath,
    /// No endpoint has been configured, so nothing may leave.
    E1NotConfigured,
    /// An endpoint exists, but this is a different origin.
    E1OriginMismatch,
    /// A redirect tried to move the request to another origin.
    E1CrossOriginRedirect,
    /// The request target could not be parsed into an origin.
    EgressTargetUnparsable,
    /// The request body left the machine with third-party prose placeheld.
    ThirdPartyBodyPlaceheld,
    /// The user confirmed twice that this one request may carry the original.
    ThirdPartyBodyIncluded,

    // ---- human in the loop ----
    /// The requested action is not in the known set.
    UnknownAction,
    /// The plan changed after it was approved.
    PlanHashMismatch,
    /// The capability token has already been spent.
    TokenReplayed,
    /// The capability token is past its time to live.
    TokenExpired,
    /// The capability token was issued for a different scope.
    TokenScopeMismatch,
    /// No such token was ever issued.
    TokenUnknown,
    /// v0.1 refuses file writes even with a valid token.
    WriteNotImplemented,
    /// The action arrived without a token that it requires.
    TokenRequired,
    /// The path is outside every root the user has authorized.
    PathNotAuthorized,

    // ---- provenance ----
    /// The request originated in external content, which is never authority.
    ExternalContentNotAuthority,
    /// Injection markers were found in imported or pasted content.
    InjectionMarkersFound,

    // ---- consent ----
    /// The user turned a capability on.
    ConsentGranted,
    /// The user turned a capability off.
    ConsentRevoked,
    /// The capability has never been consented to.
    ConsentMissing,

    // ---- generic ----
    /// Nothing unusual; recorded so a decision always has a reason.
    Routine,
}

impl ReasonCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            ReasonCode::E0NoCodePath => "E0_NO_CODE_PATH",
            ReasonCode::E1NotConfigured => "E1_NOT_CONFIGURED",
            ReasonCode::E1OriginMismatch => "E1_ORIGIN_MISMATCH",
            ReasonCode::E1CrossOriginRedirect => "E1_CROSS_ORIGIN_REDIRECT",
            ReasonCode::EgressTargetUnparsable => "EGRESS_TARGET_UNPARSABLE",
            ReasonCode::ThirdPartyBodyPlaceheld => "THIRD_PARTY_BODY_PLACEHELD",
            ReasonCode::ThirdPartyBodyIncluded => "THIRD_PARTY_BODY_INCLUDED",
            ReasonCode::UnknownAction => "UNKNOWN_ACTION",
            ReasonCode::PlanHashMismatch => "PLAN_HASH_MISMATCH",
            ReasonCode::TokenReplayed => "TOKEN_REPLAYED",
            ReasonCode::TokenExpired => "TOKEN_EXPIRED",
            ReasonCode::TokenScopeMismatch => "TOKEN_SCOPE_MISMATCH",
            ReasonCode::TokenUnknown => "TOKEN_UNKNOWN",
            ReasonCode::WriteNotImplemented => "WRITE_NOT_IMPLEMENTED",
            ReasonCode::TokenRequired => "TOKEN_REQUIRED",
            ReasonCode::PathNotAuthorized => "PATH_NOT_AUTHORIZED",
            ReasonCode::ExternalContentNotAuthority => "EXTERNAL_CONTENT_NOT_AUTHORITY",
            ReasonCode::InjectionMarkersFound => "INJECTION_MARKERS_FOUND",
            ReasonCode::ConsentGranted => "CONSENT_GRANTED",
            ReasonCode::ConsentRevoked => "CONSENT_REVOKED",
            ReasonCode::ConsentMissing => "CONSENT_MISSING",
            ReasonCode::Routine => "ROUTINE",
        }
    }

    /// Every code, for the test that checks each one satisfies the pattern in
    /// the frozen contract.
    pub const ALL: &'static [ReasonCode] = &[
        ReasonCode::E0NoCodePath,
        ReasonCode::E1NotConfigured,
        ReasonCode::E1OriginMismatch,
        ReasonCode::E1CrossOriginRedirect,
        ReasonCode::EgressTargetUnparsable,
        ReasonCode::ThirdPartyBodyPlaceheld,
        ReasonCode::ThirdPartyBodyIncluded,
        ReasonCode::UnknownAction,
        ReasonCode::PlanHashMismatch,
        ReasonCode::TokenReplayed,
        ReasonCode::TokenExpired,
        ReasonCode::TokenScopeMismatch,
        ReasonCode::TokenUnknown,
        ReasonCode::WriteNotImplemented,
        ReasonCode::TokenRequired,
        ReasonCode::PathNotAuthorized,
        ReasonCode::ExternalContentNotAuthority,
        ReasonCode::InjectionMarkersFound,
        ReasonCode::ConsentGranted,
        ReasonCode::ConsentRevoked,
        ReasonCode::ConsentMissing,
        ReasonCode::Routine,
    ];
}

impl fmt::Display for ReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditContentError {
    #[error("audit entries must not carry a `{field}` field; the chain records that something happened, not what was said")]
    ForbiddenField { field: String },

    #[error("`{0}` is not one of the reason codes the product knows how to explain")]
    UnknownReasonCode(String),

    #[error("an audit entry must serialize to a JSON object")]
    NotAnObject,

    #[error("the entry could not be serialized: {0}")]
    NotSerializable(String),
}

/// Everything about an audit entry that the caller decides.
///
/// `seq`, `prev_hash` and `entry_hash` are absent on purpose: the store writes
/// them, and a builder that offered them would invite a caller to hand in a
/// chain that verifies against nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditContent {
    pub action: AuditAction,
    pub decision: AuditDecision,
    pub reason_code: Option<ReasonCode>,
    /// Bare UUIDs. They may outlive the rows they name; forgetting must never
    /// be blocked by the audit chain.
    pub subject_refs: Vec<Uuid>,
    pub counts: Option<AuditCounts>,
    pub plan_hash: Option<String>,
    pub capability_token_id: Option<Uuid>,
    pub egress_class: Option<AuditEgressClass>,
}

impl AuditContent {
    pub fn new(action: AuditAction, decision: AuditDecision) -> AuditContent {
        AuditContent {
            action,
            decision,
            reason_code: None,
            subject_refs: Vec::new(),
            counts: None,
            plan_hash: None,
            capability_token_id: None,
            egress_class: None,
        }
    }

    pub fn allowed(action: AuditAction, reason: ReasonCode) -> AuditContent {
        AuditContent::new(action, AuditDecision::Allowed).because(reason)
    }

    pub fn denied(action: AuditAction, reason: ReasonCode) -> AuditContent {
        AuditContent::new(action, AuditDecision::Denied).because(reason)
    }

    pub fn because(mut self, reason: ReasonCode) -> AuditContent {
        self.reason_code = Some(reason);
        self
    }

    pub fn about(mut self, subjects: &[Uuid]) -> AuditContent {
        self.subject_refs = subjects.to_vec();
        self
    }

    pub fn counting(mut self, counts: AuditCounts) -> AuditContent {
        self.counts = Some(counts);
        self
    }

    pub fn for_plan(mut self, plan_hash: impl Into<String>) -> AuditContent {
        self.plan_hash = Some(plan_hash.into());
        self
    }

    pub fn with_token(mut self, token_id: Uuid) -> AuditContent {
        self.capability_token_id = Some(token_id);
        self
    }

    pub fn over(mut self, class: AuditEgressClass) -> AuditContent {
        self.egress_class = Some(class);
        self
    }

    /// Turn the content into a contract entry, checking it first.
    ///
    /// `at_unix_seconds` is passed in rather than read from the clock so that
    /// a caller replaying a batch, and a test, both get deterministic output.
    pub fn into_entry(
        self,
        entry_id: Uuid,
        at_unix_seconds: i64,
    ) -> Result<SoulAuditEntry, AuditContentError> {
        let entry = SoulAuditEntry {
            schema_version: SchemaVersion,
            // Placeholders. The store overwrites all three; see WP02's note in
            // docs/STATUS.md.
            seq: 0,
            entry_id,
            ts: Timestamp::new(rfc3339_utc(at_unix_seconds)),
            prev_hash: Sha256Hex::new("0".repeat(64)),
            entry_hash: Sha256Hex::new("0".repeat(64)),
            action: self.action,
            decision: self.decision,
            reason_code: self.reason_code.map(|code| code.as_str().to_owned()),
            subject_refs: match self.subject_refs.is_empty() {
                true => None,
                false => Some(self.subject_refs),
            },
            counts: self.counts,
            plan_hash: self.plan_hash.map(Sha256Hex::new),
            capability_token_id: self.capability_token_id,
            egress_class: self.egress_class,
        };
        check(&entry)?;
        Ok(entry)
    }
}

/// Reject an entry that carries prose-shaped fields or an unknown reason code.
///
/// Runs over the serialized form rather than the struct so that it keeps
/// working if `SoulAuditEntry` grows a field: this is the net that has to hold
/// when the contract changes, not the one that changes with it.
pub fn check(entry: &SoulAuditEntry) -> Result<(), AuditContentError> {
    let value = serde_json::to_value(entry)
        .map_err(|error| AuditContentError::NotSerializable(error.to_string()))?;
    if !value.is_object() {
        return Err(AuditContentError::NotAnObject);
    }
    check_value(&value)?;

    if let Some(code) = entry.reason_code.as_deref() {
        if !ReasonCode::ALL.iter().any(|known| known.as_str() == code) {
            return Err(AuditContentError::UnknownReasonCode(code.to_owned()));
        }
    }
    Ok(())
}

fn check_value(value: &serde_json::Value) -> Result<(), AuditContentError> {
    match value {
        serde_json::Value::Object(map) => {
            for (key, nested) in map {
                let lowered = key.to_ascii_lowercase();
                if FORBIDDEN_FIELDS.contains(&lowered.as_str()) {
                    return Err(AuditContentError::ForbiddenField { field: key.clone() });
                }
                check_value(nested)?;
            }
            Ok(())
        }
        serde_json::Value::Array(items) => items.iter().try_for_each(check_value),
        _ => Ok(()),
    }
}

/// Append one checked entry to the chain.
///
/// The store assigns the links and validates against the frozen contract; this
/// wrapper only guarantees that nothing which failed [`check`] gets that far.
pub fn append<L: AuditLog>(
    log: &mut L,
    content: AuditContent,
    at_unix_seconds: i64,
) -> Result<Uuid, AuditWriteError> {
    let entry = content.into_entry(Uuid::now_v7(), at_unix_seconds)?;
    Ok(log.append_audit(entry)?)
}

#[derive(Debug, thiserror::Error)]
pub enum AuditWriteError {
    #[error(transparent)]
    Content(#[from] AuditContentError),
    #[error("the audit chain could not be appended to: {0}")]
    Store(#[from] soul_store_api::types::StoreError),
}

/// Convenience for callers that only care whether the write landed.
pub fn append_or_store_error<L: AuditLog>(
    log: &mut L,
    content: AuditContent,
    at_unix_seconds: i64,
) -> StoreResult<Uuid> {
    match append(log, content, at_unix_seconds) {
        Ok(id) => Ok(id),
        Err(AuditWriteError::Store(error)) => Err(error),
        Err(AuditWriteError::Content(error)) => Err(
            soul_store_api::types::StoreError::ContractViolation(error.to_string()),
        ),
    }
}
