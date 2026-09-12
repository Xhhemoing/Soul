//! WP08's command surface: ask permission, get a one-time token, redact a
//! draft, and run one generation against the user's own endpoint.
//!
//! Thin on purpose. Every decision is `soul-policy`'s and every socket is
//! `soul-egress`'s; what is here is the small amount of wiring between them,
//! plus the [`PolicySession`] that has to outlive a single call because a
//! capability token issued by one command is spent by the next.
//!
//! No UI, and no store handle. Each function returns the [`AuditContent`] its
//! outcome deserves rather than writing it, because the caller is the one
//! holding the open store — and because an audit entry written from here would
//! be written twice by a caller that also audits.

use soul_policy::audit::AuditContent;
use soul_policy::e1::{E1Purpose, E1RequestPlan};
use soul_policy::hitl::{
    check_action, ActionKind, ActionRequest, ApprovedAction, CapabilityScope, CapabilityToken,
    HitlDenial, PlanHash, RequestOrigin, TokenIssuer,
};
use soul_policy::net_guard::{EgressConfig, EgressDenied, NetGuard, Origin, OriginError};
use soul_policy::redactor::{KnownIdentifiers, OneShotExemption, RedactedBody, Redactor, Turn};
use soul_policy::ReasonCode;
use soul_schema::audit::{AuditAction, AuditDecision};
use uuid::Uuid;

/// The permission state one Soul session carries.
///
/// The token ledger is the reason this is a struct rather than four free
/// functions: "a token is good for one use" is only true if the thing that
/// issued it is still around to remember that it was spent.
#[derive(Debug)]
pub struct PolicySession {
    guard: NetGuard,
    issuer: TokenIssuer,
    redactor: Redactor,
}

impl PolicySession {
    /// A session that can reach nothing. The default until the user configures
    /// an endpoint: `NetGuard::closed()` refuses every origin, loopback
    /// included.
    pub fn closed() -> PolicySession {
        PolicySession::new(EgressConfig::closed(), KnownIdentifiers::new())
    }

    pub fn new(egress: EgressConfig, identifiers: KnownIdentifiers) -> PolicySession {
        PolicySession {
            guard: NetGuard::new(egress),
            issuer: TokenIssuer::new(),
            redactor: Redactor::new(identifiers),
        }
    }

    /// Point the session at the endpoint the user configured.
    pub fn with_user_endpoint(
        url: &str,
        identifiers: KnownIdentifiers,
    ) -> Result<PolicySession, OriginError> {
        Ok(PolicySession::new(
            EgressConfig::with_user_endpoint(url)?,
            identifiers,
        ))
    }

    /// Re-point an existing session at an endpoint the user has just entered.
    ///
    /// The difference from [`PolicySession::with_user_endpoint`] is what is
    /// *kept*, and it is the reason this exists rather than the caller
    /// building a second session. The redactor is built from a
    /// [`KnownIdentifiers`] set that has to be the same one the drafter holds
    /// — `draft.rs` says why: two redactors that disagree about who exists
    /// placehold different things — and a caller that replaced the whole
    /// session would have to know that set in order to hand it over again.
    /// Today the shell's set is empty, so replacing would look identical;
    /// the day it is not, the endpoint form would silently be the thing that
    /// emptied it.
    ///
    /// The guard is what changes, and only the guard. Nothing is contacted:
    /// [`Origin::parse`](soul_policy::net_guard::Origin::parse) reads a string
    /// and `NetGuard` holds the answer, so the first packet still waits for
    /// [`PolicySession::e1_generate`] and the approval in front of it.
    pub fn set_user_endpoint(&mut self, url: &str) -> Result<(), OriginError> {
        self.guard = NetGuard::new(EgressConfig::with_user_endpoint(url)?);
        Ok(())
    }

    /// Re-point the redactor at a set of identifiers the shell has just read.
    ///
    /// The twin of [`PolicySession::set_user_endpoint`], and the same argument
    /// the other way round: that one swaps the guard and keeps the redactor,
    /// this one swaps the redactor and keeps the guard — and the token ledger
    /// with it, so a set that arrives while a plan is approved does not void
    /// the token that plan was minted against. A caller that rebuilt the whole
    /// session to change one of the two would take the address the user typed
    /// away with it, which is the bug the endpoint form's own docs describe.
    ///
    /// The set has to be the one the drafter was given.
    /// [`crate::commands::draft::known_identifiers`] builds it once and both
    /// sessions are handed the same answer; two redactors that disagree about
    /// who exists placehold different things, and the one that decides what
    /// goes on the wire would not be the one the plan was described from.
    pub fn set_identifiers(&mut self, identifiers: KnownIdentifiers) {
        self.redactor = Redactor::new(identifiers);
    }

    /// Take the endpoint away again, leaving the session where it started.
    ///
    /// `NetGuard::closed()` refuses every origin, loopback included, so this
    /// is the same state [`PolicySession::closed`] is in rather than a weaker
    /// one that merely has no URL to hand.
    pub fn clear_user_endpoint(&mut self) {
        self.guard = NetGuard::closed();
    }

    pub fn guard(&self) -> &NetGuard {
        &self.guard
    }

    /// The redactor every body this session sends is built through.
    ///
    /// Borrowed rather than handed out by value so there is only ever one set
    /// of identifiers in play. A caller that needs to build a body while also
    /// holding this session mutably — `draft.rs`'s summary rephrasing is the
    /// one — clones it, which is a copy of the same set rather than a second
    /// opinion about who exists.
    pub fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// May this action proceed?
    ///
    /// Straight through to [`soul_policy::hitl::check_action`]. Wrapping it
    /// adds nothing except the session's token ledger, which is the point.
    pub fn check_action(
        &mut self,
        request: &ActionRequest,
        now_ms: u64,
    ) -> Result<ApprovedAction, HitlDenial> {
        check_action(&mut self.issuer, request, now_ms)
    }

    /// Mint a one-time token for a plan the user has approved.
    ///
    /// v0.1 refuses to issue a file-write token at all. `TokenIssuer::consume`
    /// would refuse it on presentation regardless, but a token that exists and
    /// never works is a thing someone will eventually try to make work.
    pub fn issue_token(
        &mut self,
        scope: CapabilityScope,
        plan_hash: PlanHash,
        now_ms: u64,
    ) -> Result<CapabilityToken, TokenRefused> {
        if !scope.is_implemented_in_v0_1() {
            return Err(TokenRefused {
                scope,
                reason: ReasonCode::WriteNotImplemented,
            });
        }
        Ok(self.issuer.issue(scope, plan_hash, now_ms))
    }

    /// Replace third-party prose with a placeholder and scrub identifiers.
    ///
    /// The default and the only path most callers need. There is no argument
    /// that turns it off.
    pub fn redact(&self, turns: &[Turn]) -> RedactedBody {
        self.redactor.redact_for_e1(turns)
    }

    /// Redact with one turn left intact, because the user confirmed twice.
    ///
    /// The exemption is consumed by value and nothing here remembers it: the
    /// next call to either method places the same turn back behind a
    /// placeholder.
    pub fn redact_with_exemption(
        &self,
        turns: &[Turn],
        exemption: OneShotExemption,
    ) -> RedactedBody {
        self.redactor.redact_for_e1_with_exemption(turns, exemption)
    }

    /// Send one generation request to the user's own endpoint.
    ///
    /// The order is the policy: the body is redacted before it is described,
    /// the description is what the token was issued against, the action is
    /// checked, and only then does the guard mint the permit that
    /// `soul-egress` needs. A caller cannot reorder these, because each step
    /// produces the value the next one requires.
    pub fn e1_generate(
        &mut self,
        model: &str,
        body: RedactedBody,
        token_id: Uuid,
        now_ms: u64,
    ) -> Result<E1Outcome, E1Refusal> {
        self.e1_generate_for(E1Purpose::Draft, model, body, token_id, now_ms)
    }

    /// The same, for a request that is asking for something other than a
    /// draft.
    ///
    /// The purpose decides which of `soul-policy`'s instruction constants goes
    /// in the system position and nothing else: the guard, the token and the
    /// redacted body are the same on every purpose, so a new one cannot buy a
    /// caller a socket it did not already have. It is a separate entry point
    /// rather than a field on the session because the purpose belongs to one
    /// request — a session that remembered it would send the summary
    /// instruction on the next draft.
    pub fn e1_generate_for(
        &mut self,
        purpose: E1Purpose,
        model: &str,
        body: RedactedBody,
        token_id: Uuid,
        now_ms: u64,
    ) -> Result<E1Outcome, E1Refusal> {
        let plan = e1_plan(model, self.guard.config().e1_endpoint(), &body);
        let request = ActionRequest::new(
            ActionKind::GenerateWithUserEndpoint.as_str(),
            RequestOrigin::User,
        )
        .with_plan(plan)
        .with_token(token_id);
        let approved = self.check_action(&request, now_ms)?;

        // There is no URL to ask the guard about until the user has entered
        // one, so this refusal is decided here rather than in `authorize_e1`.
        // It carries the same reason code the guard would have used.
        let Some(endpoint) = self.guard.config().e1_endpoint() else {
            return Err(E1Refusal::NotConfigured);
        };
        let url = format!("{endpoint}/v1/chat/completions");
        let permit = self.guard.authorize_e1(&url)?;
        let class = permit.class();

        let carries_original = body.carries_exempted_original();
        let response = soul_egress::send(&E1RequestPlan::chat_completions_for(
            purpose, permit, model, body,
        ))?;

        Ok(E1Outcome {
            status: response.status,
            body: response.body,
            plan_hash: approved.plan_hash,
            token_id,
            egress_class: class,
            carries_exempted_original: carries_original,
        })
    }
}

/// What the user is approving when they approve a generation request.
///
/// Counts, a model name and a destination, never the text. This is the value
/// the plan hash is taken over, so an edit between approval and execution
/// changes the hash and [`check_action`] refuses — which is only meaningful if
/// the plan describes the request faithfully, hence the redaction counts being
/// in it.
///
/// `target` is the origin the request would reach, and it is in here because
/// `docs/SECURITY.md` says an E1 configuration change invalidates the plan
/// hash. Without it a plan prepared against one address stays approvable after
/// the 设置 page has been pointed at another, and the body the user read a
/// description of goes somewhere they never approved. The hash is what leaves
/// this function, so naming an origin here puts no address in a plan, an audit
/// entry or a screen.
pub fn e1_plan(model: &str, target: Option<&Origin>, body: &RedactedBody) -> serde_json::Value {
    serde_json::json!({
        "action": ActionKind::GenerateWithUserEndpoint.as_str(),
        "model": model,
        "target": target.map(ToString::to_string),
        "third_party_turns": body.third_party_turns(),
        "placeheld_turns": body.placeheld_turns(),
        "carries_exempted_original": body.carries_exempted_original(),
    })
}

/// A token this build will not issue.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("v0.1 does not issue {} tokens", scope.as_str())]
pub struct TokenRefused {
    pub scope: CapabilityScope,
    pub reason: ReasonCode,
}

#[derive(Debug, thiserror::Error)]
pub enum E1Refusal {
    #[error(transparent)]
    Hitl(#[from] HitlDenial),
    #[error("no endpoint is configured, so there is nowhere to send a request")]
    NotConfigured,
    #[error(transparent)]
    Egress(#[from] EgressDenied),
    #[error(transparent)]
    Transport(#[from] soul_egress::EgressError),
}

impl E1Refusal {
    /// The code the audit entry carries. A transport failure that is not a
    /// refused redirect has no policy reason, so it records as routine: the
    /// request was allowed and the network did not cooperate.
    pub fn reason_code(&self) -> ReasonCode {
        match self {
            E1Refusal::Hitl(denial) => denial.reason_code(),
            E1Refusal::NotConfigured => ReasonCode::E1NotConfigured,
            E1Refusal::Egress(denied) => denied.reason_code(),
            E1Refusal::Transport(error) => error.reason_code().unwrap_or(ReasonCode::Routine),
        }
    }

    /// The audit entry for a refusal, with the action it was refused under.
    pub fn audit(&self) -> AuditContent {
        let action = match self {
            E1Refusal::Hitl(HitlDenial::Token(_)) => AuditAction::CapabilityReject,
            E1Refusal::Hitl(_) => AuditAction::HitlDeny,
            _ => AuditAction::EgressRequest,
        };
        AuditContent::denied(action, self.reason_code())
    }
}

/// One generation that happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E1Outcome {
    pub status: u16,
    /// What the endpoint returned. Data, never instruction: no caller may act
    /// on a tool plan found in here.
    pub body: String,
    pub plan_hash: PlanHash,
    pub token_id: Uuid,
    pub egress_class: soul_policy::EgressClass,
    pub carries_exempted_original: bool,
}

impl E1Outcome {
    /// The audit entry for the request. Records that one request left, under
    /// which plan and token, and whether the user's double confirmation was in
    /// play — never a word of what was sent or returned.
    pub fn audit(&self) -> AuditContent {
        let reason = match self.carries_exempted_original {
            true => ReasonCode::ThirdPartyBodyIncluded,
            false => ReasonCode::ThirdPartyBodyPlaceheld,
        };
        AuditContent::new(AuditAction::EgressRequest, AuditDecision::Allowed)
            .because(reason)
            .for_plan(self.plan_hash.as_str())
            .with_token(self.token_id)
            .over(self.egress_class.as_audit_class())
    }
}
