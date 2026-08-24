//! WP10's command surface: draft a reply, summarize a person, and send
//! neither.
//!
//! Thin, like the rest of this module. `soul-draft` decides what a draft says
//! and `soul-policy` decides whether anything may leave; what is here is the
//! wiring, plus the one piece of state that has to outlive a single call.
//!
//! ## Why there are two steps and not one
//!
//! [`DraftSession::prepare`] builds the request body and describes it.
//! [`DraftSession::generate`] sends it. They are separate because the thing
//! between them is a person: the plan the user approves has to be the plan
//! that executes, and `PolicySession::e1_generate` compares the two by hash.
//! A single call would hash a plan nobody had seen.
//!
//! The body is held between the steps rather than rebuilt, for a reason that
//! is easy to miss. A one-shot exemption is consumed when the body is built,
//! so a rebuilt body would be a *different* body — placeheld where the user
//! had confirmed — and the counts the user approved would no longer describe
//! what went out. Holding it also bounds the blast radius: there is room for
//! exactly one prepared body, a second `prepare` replaces it, and `generate`
//! takes it by value. One body, one request, no way to replay it.
//!
//! ## What is not here
//!
//! No `send`, and no argument that means one. The one socket this file can
//! reach is `PolicySession::e1_generate`, which goes to the origin the user
//! configured and returns text. Whether the user then sends the draft to
//! anybody is between them and their messaging app.
//!
//! `summarize_person` writes no audit entry, and that is deliberate rather
//! than an omission: it reads the graph and the evidence rows the graph
//! already cites, changes nothing, and `docs/schemas/audit.schema.json` is
//! frozen with no action for it. An entry invented here would be a claim the
//! contract does not make.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_draft::analysis::{self, PersonSummary, SummarySource};
use soul_draft::brief::ProfileBrief;
use soul_draft::draft::{BodyFacts, Draft, DraftRequest, Drafter};
use soul_draft::error::DraftError;
use soul_graph::GraphError;
use soul_policy::audit::AuditContent;
use soul_policy::hitl::{
    ActionKind, ActionRequest, CapabilityScope, HitlDenial, PlanHash, RequestOrigin,
};
use soul_policy::redactor::{KnownIdentifiers, OneShotExemption, RedactedBody, Redactor};
use soul_policy::ReasonCode;
use soul_profile::ProfileError;
use soul_schema::common::SupportedBand;
use soul_store::SqlCipherStore;

use crate::commands::policy::{e1_plan, E1Refusal, PolicySession, TokenRefused};

pub use soul_draft::draft::{
    DraftSource, NeverSent, DEGRADED_NOTICE, ENDPOINT_NOTICE, NOT_SENT_NOTICE, TEMPLATE_NOTICE,
};
pub use soul_draft::template::BODY_SLOT;

// Re-exported so the desktop shell can name what it receives without taking
// `soul-draft` and `soul-policy` as dependencies of its own. A forwarder that
// had them would be a forwarder that could assemble a request body.
pub use soul_draft::draft::Draft as DraftValue;
pub use soul_policy::redactor::KnownIdentifiers as DraftIdentifiers;

/// What the user is told before a generation request goes out.
///
/// A constant rather than a sentence assembled in the interface, for the same
/// reason WP09 made the cloud notice one: it is a promise about what the
/// build does, and a promise kept in TypeScript is one the Rust tests cannot
/// check.
pub const E1_PLAN_NOTICE: &str = "确认之后，只有下面这些内容会发到你自己配置的模型端点，\
     用来生成草稿。第三人正文默认已占位。草稿生成之后仍然由你自己决定要不要发出去，\
     Soul 不会替你发送。";

/// The model name a session uses until the user names one.
///
/// It only ever reaches anything once an endpoint is configured — the
/// deterministic template does not consult it, and with no endpoint there is
/// nobody to send a model name to. Naming it plainly beats an empty string
/// that would look like a bug in a request log.
pub const UNNAMED_MODEL: &str = "unnamed-model";

/// The drafting state one Soul session carries.
///
/// `identifiers` has to be the same set the [`PolicySession`] was built with:
/// both hold a [`Redactor`], and two redactors that disagree about who exists
/// would placehold different things. The shell builds one
/// [`KnownIdentifiers`] from the contact graph and hands it to both.
#[derive(Debug)]
pub struct DraftSession {
    drafter: Drafter,
    /// Room for exactly one. See the module docs.
    pending: Option<Pending>,
}

/// A body that has been built and described, waiting for a person.
#[derive(Debug)]
struct Pending {
    id: Uuid,
    request: DraftRequest,
    body: RedactedBody,
    facts: BodyFacts,
    plan_hash: PlanHash,
}

impl DraftSession {
    pub fn new(model: impl Into<String>, identifiers: KnownIdentifiers) -> DraftSession {
        DraftSession {
            drafter: Drafter::new(Redactor::new(identifiers), model),
            pending: None,
        }
    }

    pub fn model(&self) -> &str {
        self.drafter.model()
    }

    /// The plan hash the user is currently being asked about, if any.
    pub fn pending_plan_hash(&self) -> Option<&PlanHash> {
        self.pending.as_ref().map(|pending| &pending.plan_hash)
    }

    /// Throw away a prepared body without sending it.
    ///
    /// The honest answer to a user who read the plan and said no. Returns
    /// whether there was anything to throw away.
    pub fn discard(&mut self) -> bool {
        self.pending.take().is_some()
    }

    /// AC-17: draft with no key, no model and no socket.
    ///
    /// No body is built on this path, so there is nothing for a bug to leak.
    /// `origin` is a parameter rather than a default: a request the WebView
    /// made because somebody clicked is [`RequestOrigin::User`], and a caller
    /// that derived one from imported text has to say so and be refused.
    pub fn draft_offline(
        &self,
        policy: &mut PolicySession,
        request: &DraftRequest,
        origin: RequestOrigin,
        now_ms: u64,
    ) -> Result<Drafted, DraftRefusal> {
        allow(policy, ActionKind::DraftReply, origin, now_ms)?;
        let draft = self.drafter.draft_offline(request)?;
        Ok(Drafted {
            audit: draft.audit(),
            draft,
        })
    }

    /// Step one of the endpoint path: redact, describe, and stop.
    ///
    /// Nothing leaves here. What comes back is what the user is being asked
    /// to approve, and its hash is what [`DraftSession::generate`] will be
    /// held to.
    pub fn prepare(
        &mut self,
        policy: &mut PolicySession,
        request: DraftRequest,
        exemption: Option<OneShotExemption>,
        origin: RequestOrigin,
        now_ms: u64,
    ) -> Result<E1DraftPlan, DraftRefusal> {
        allow(policy, ActionKind::GenerateWithUserEndpoint, origin, now_ms)?;

        let body = self.drafter.redact(&request, exemption)?;
        let facts = BodyFacts::of(&body);
        let plan_hash = PlanHash::of(&e1_plan(self.drafter.model(), &body));
        let id = Uuid::now_v7();

        // A second prepare replaces the first, so an unapproved body cannot
        // sit around waiting for a token that was issued for another one.
        self.pending = Some(Pending {
            id,
            request,
            body,
            facts,
            plan_hash: plan_hash.clone(),
        });

        Ok(E1DraftPlan {
            preparation_id: id.to_string(),
            plan_hash: plan_hash.as_str().to_owned(),
            model: self.drafter.model().to_owned(),
            third_party_turns: facts.third_party_turns,
            placeheld_turns: facts.placeheld_turns,
            carries_exempted_original: facts.carries_exempted_original,
            notice: E1_PLAN_NOTICE.to_owned(),
            not_sent_notice: NOT_SENT_NOTICE.to_owned(),
        })
    }

    /// Step two: the user approved this exact preparation, so run it.
    ///
    /// Both halves of the approval are checked, and they catch different
    /// mistakes:
    ///
    /// * the plan hash catches a body whose *shape* is not the one described
    ///   — a different model, a turn that is no longer placeheld — and it is
    ///   checked again inside `PolicySession::e1_generate` against the plan
    ///   the token was minted for;
    /// * the preparation id catches a body that is a different body of the
    ///   same shape. The plan carries counts and no prose, on purpose, so two
    ///   pastes with one third-party turn each hash identically. Without the
    ///   id, an approval the user gave for one message would send another.
    ///
    /// The token is minted here because this call *is* the approval: the
    /// ledger's job is to make sure one click buys one request.
    pub fn generate(
        &mut self,
        policy: &mut PolicySession,
        approval: &Approval,
        now_ms: u64,
    ) -> Result<Drafted, DraftRefusal> {
        let Some(pending) = self.pending.take() else {
            return Err(DraftRefusal::NothingPrepared);
        };
        if approval.preparation_id != pending.id.to_string() {
            return Err(DraftRefusal::NotThePreparedRequest);
        }
        if pending.plan_hash.as_str() != approval.plan_hash {
            return Err(HitlDenial::PlanHashMismatch {
                approved: approval.plan_hash.clone(),
                current: pending.plan_hash.as_str().to_owned(),
            }
            .into());
        }

        let token = policy
            .issue_token(
                CapabilityScope::E1Generate,
                pending.plan_hash.clone(),
                now_ms,
            )
            .map_err(DraftRefusal::Token)?;
        let outcome =
            policy.e1_generate(self.drafter.model(), pending.body, token.token_id(), now_ms)?;

        let draft = self
            .drafter
            .finish_reply(&pending.request, pending.facts, &outcome.body)?;
        let mut audit = vec![outcome.audit()];
        audit.extend(draft.audit());
        Ok(Drafted { draft, audit })
    }
}

/// One draft, and what the chain is owed for it.
///
/// The entries are handed back rather than written, exactly as `policy.rs`
/// and `fileplan.rs` do it: the caller holds the open store, and an entry
/// written from here would be written twice by a caller that also audits.
#[derive(Debug, Clone, PartialEq)]
pub struct Drafted {
    /// The value the shell receives. `tests/never_sends.rs` in `soul-draft`
    /// pins its shape, which is why this is the draft itself and not a view
    /// assembled here — one surface, one test.
    pub draft: Draft,
    pub audit: Vec<AuditContent>,
}

/// What the user reads before deciding whether anything may leave.
///
/// Counts, a model name and two sentences. There is no field here that could
/// hold the text: the whole point of the step is that the user approves the
/// *shape* of a request, and the shape is what the hash is taken over.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct E1DraftPlan {
    /// Which preparation this is. Echoed back on approval; see
    /// [`DraftSession::generate`] for why the hash alone is not enough.
    pub preparation_id: String,
    pub plan_hash: String,
    pub model: String,
    pub third_party_turns: usize,
    pub placeheld_turns: usize,
    /// True only when the user has already confirmed twice for one turn.
    pub carries_exempted_original: bool,
    pub notice: String,
    pub not_sent_notice: String,
}

impl E1DraftPlan {
    /// The answer to press-approve, built from the plan the user was shown.
    pub fn approval(&self) -> Approval {
        Approval {
            preparation_id: self.preparation_id.clone(),
            plan_hash: self.plan_hash.clone(),
        }
    }
}

/// What the shell sends back when the user says yes.
///
/// Both fields come from the [`E1DraftPlan`] that was on screen. A shell that
/// invented either one is a shell approving something nobody read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub preparation_id: String,
    pub plan_hash: String,
}

/// The pair of sessions a shell with no store yet starts from.
///
/// Both are built here rather than separately because they have to agree:
/// each holds a [`Redactor`], and two redactors that knew about different
/// names would placehold different things. Nothing is configured, so nothing
/// can leave; and there are no known identifiers, because the contact graph
/// that supplies them needs an open store, which is WP13's question. The
/// shape-based scrub still catches addresses, handles and long digit runs,
/// and a third-party turn is placeheld whole regardless of who is in it.
pub fn closed_session() -> (DraftSession, PolicySession) {
    (
        DraftSession::new(UNNAMED_MODEL, KnownIdentifiers::new()),
        PolicySession::closed(),
    )
}

/// Draft a reply to something a person pasted into the interface.
///
/// The one-call entry point a desktop shell binds, and the two small
/// decisions in it are the ones a thin forwarder should not be making:
///
/// * the origin is [`RequestOrigin::User`], because a call arriving over the
///   IPC is a click. A caller that derived the text from imported content has
///   to go through [`DraftSession::draft_offline`] and say so;
/// * the clock is read here. `soul-policy`'s clock module says nothing reads
///   it except at the outermost edge, and a shell entry point is that edge.
///
/// The paste is one third-party turn by construction. Soul cannot tell from
/// the text whose words they are, and the safe reading of "I pasted a message
/// I received" is the one that keeps it off the wire.
///
/// This is the local path: no request body is built, so there is nothing for
/// a bug to leak. The endpoint path is [`DraftSession::prepare`] and
/// [`DraftSession::generate`], which need a screen for the person in between.
pub fn draft_pasted(
    drafting: &DraftSession,
    policy: &mut PolicySession,
    pasted: &str,
) -> Result<Draft, DraftRefusal> {
    let request = DraftRequest::from_paste(ProfileBrief::neutral(), pasted);
    Ok(drafting
        .draft_offline(
            policy,
            &request,
            RequestOrigin::User,
            soul_policy::clock::now_unix_millis(),
        )?
        .draft)
}

/// The profile a draft prompt is allowed to know about.
///
/// Blank profiles resolve to the neutral voice rather than failing, because
/// drafting before an import is a thing a user may reasonably do.
pub fn brief(store: &SqlCipherStore, profile_id: Uuid) -> Result<ProfileBrief, DraftRefusal> {
    Ok(ProfileBrief::from_view(&soul_profile::profile_view(
        store, profile_id,
    )?)?)
}

/// Everything Soul will say about one person, and what each line rests on.
///
/// AC-16. The evidence is resolved out of the store before the summary is
/// built, so a point cannot cite a row that has been forgotten; `soul-draft`
/// refuses the whole summary rather than returning a shorter one.
pub fn summarize_person(
    policy: &mut PolicySession,
    store: &SqlCipherStore,
    contact_id: Uuid,
    origin: RequestOrigin,
    now_ms: u64,
) -> Result<PersonSummaryView, DraftRefusal> {
    allow(policy, ActionKind::AnalysePeople, origin, now_ms)?;

    let graph = soul_graph::load(store)?;
    let edge = graph
        .edges_for(contact_id)
        .into_iter()
        .max_by_key(|edge| edge.tie_strength.interaction_count)
        .ok_or(DraftError::NoSuchTie { contact_id })?
        .clone();
    let resolved = soul_graph::resolve_evidence(store, &edge)?;

    let summary = analysis::summarize_person(&graph, contact_id, &resolved)?;
    PersonSummaryView::of(&summary)
}

/// What the desktop shell may display about one person.
///
/// Not [`PersonSummary`] itself — it carries a `relationship_id` the shell
/// has no use for — and the rendered text is here rather than built in
/// TypeScript, so the sentences the user reads are the ones that went through
/// the denylist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonSummaryView {
    pub contact_id: String,
    /// `counts` or `user_endpoint`.
    pub source: String,
    /// Every line, each naming how many rows are behind it.
    pub text: String,
    /// Non-empty, and every point cites at least one evidence row.
    pub points: Vec<SummaryPointView>,
    /// `工作假设，非临床结论`.
    pub notice: String,
    /// Always false, and there is no code path that sets it.
    pub clinical_claim: bool,
}

impl PersonSummaryView {
    pub fn of(summary: &PersonSummary) -> Result<PersonSummaryView, DraftRefusal> {
        Ok(PersonSummaryView {
            contact_id: summary.contact_id.to_string(),
            source: match summary.source {
                SummarySource::Counts => "counts",
                SummarySource::UserEndpoint => "user_endpoint",
            }
            .to_owned(),
            text: analysis::render(summary)?,
            points: summary
                .points
                .iter()
                .map(|point| SummaryPointView {
                    statement: point.statement().to_owned(),
                    evidence_ids: point.evidence_ids().iter().map(Uuid::to_string).collect(),
                    band: band_word(point.band()).to_owned(),
                })
                .collect(),
            notice: summary.notice.clone(),
            clinical_claim: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryPointView {
    pub statement: String,
    /// Never empty. `SummaryPoint::new` refuses to build a point without one.
    pub evidence_ids: Vec<String>,
    pub band: String,
}

fn band_word(band: SupportedBand) -> &'static str {
    match band {
        SupportedBand::Weak => "weak",
        SupportedBand::Moderate => "moderate",
        SupportedBand::Strong => "strong",
    }
}

/// The gate every command here goes through first.
///
/// The plan is the action name alone: nothing on this surface is approved in
/// advance, so there is no earlier hash to compare against, and what the
/// check is doing is refusing an unknown action and refusing external content
/// as authority. `GenerateWithUserEndpoint` needs a token, which is why
/// `prepare` — which sends nothing — is the only place it appears, and why
/// the token check is skipped by asking about the action rather than
/// executing it.
fn allow(
    policy: &mut PolicySession,
    kind: ActionKind,
    origin: RequestOrigin,
    now_ms: u64,
) -> Result<(), DraftRefusal> {
    if origin == RequestOrigin::ExternalContent {
        return Err(HitlDenial::ExternalContentNotAuthority.into());
    }
    if kind.needs_capability_token() {
        // Preparing is not generating. The token is checked in `generate`,
        // where a request actually leaves.
        return Ok(());
    }
    policy.check_action(&ActionRequest::new(kind.as_str(), origin), now_ms)?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DraftRefusal {
    #[error(transparent)]
    Hitl(#[from] HitlDenial),
    #[error(transparent)]
    Draft(#[from] DraftError),
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error(transparent)]
    Graph(#[from] GraphError),
    #[error(transparent)]
    E1(#[from] E1Refusal),
    #[error(transparent)]
    Token(TokenRefused),
    #[error("nothing has been prepared, so there is no request to approve")]
    NothingPrepared,
    #[error("the approval names a request that is no longer the prepared one")]
    NotThePreparedRequest,
}

impl DraftRefusal {
    /// The code the audit entry carries. Never any prose.
    pub fn reason_code(&self) -> ReasonCode {
        match self {
            DraftRefusal::Hitl(denial) => denial.reason_code(),
            DraftRefusal::E1(refusal) => refusal.reason_code(),
            DraftRefusal::Token(refused) => refused.reason,
            DraftRefusal::Draft(error) => error.reason_code().unwrap_or(ReasonCode::Routine),
            DraftRefusal::NothingPrepared | DraftRefusal::NotThePreparedRequest => {
                ReasonCode::PlanHashMismatch
            }
            DraftRefusal::Profile(_) | DraftRefusal::Graph(_) => ReasonCode::Routine,
        }
    }
}

/// What the WebView is told when drafting is refused.
///
/// A code and a sentence. The sentence is Soul's own — a refusal that quoted
/// an endpoint would carry that endpoint's words into the interface, and
/// `soul-egress` already redacts its error strings for the same reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRefusalView {
    pub reason_code: String,
    pub explanation: String,
}

impl DraftRefusalView {
    pub fn of(refusal: &DraftRefusal) -> DraftRefusalView {
        DraftRefusalView {
            reason_code: refusal.reason_code().as_str().to_owned(),
            explanation: refusal.to_string(),
        }
    }
}
