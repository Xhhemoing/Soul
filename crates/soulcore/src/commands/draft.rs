//! WP10's command surface: drafting, which never goes out.
//!
//! This is the only layer that holds all three things a draft needs — the open
//! store, the [`PolicySession`], and the caller's clock — so it is the only
//! layer where a generation request can happen. `soul-draft` underneath is
//! pure: it turns pastes into turns, a voice into a template, and graph edges
//! into claims, and it has no way to reach the network. Everything on the wire
//! goes through [`PolicySession::e1_generate`], which is the single E1 path
//! WP08 built.
//!
//! The order in [`draft_reply`] is the policy, and it is fixed:
//!
//! 1. read the voice from the profile, every time, so a value the user just
//!    set is in effect on this draft (AC-07);
//! 2. assemble the turns and record what the injection scan saw — recording
//!    only: a hit changes no branch;
//! 3. ask [`soul_policy::hitl::check_action`] whether `draft.reply` may
//!    proceed, with a plan made of counts;
//! 4. redact, either the default way or with the one-shot exemption the caller
//!    minted for this call and nothing else;
//! 5. with an endpoint: hash the plan, mint a token, spend it on one request.
//!    Without one: render the template, which is the shipped shape of a draft
//!    and not a fallback;
//! 6. refuse text that makes a medical claim, and append the audit entries.
//!
//! Two things this module deliberately does not do. It does not remember an
//! exemption: [`OneShotExemption`] arrives inside [`DraftRequest`], is consumed
//! by value, and there is no field anywhere that could hold it for the next
//! draft. And it does not store the paste. Pasted prose is material for one
//! draft; keeping it is WP04's job, through WP04's entry point, with WP04's
//! consent story.
//!
//! A refused request is a refusal the caller can read. A cross-origin redirect
//! comes back as [`DraftCommandError::Generation`] with
//! [`ReasonCode::E1CrossOriginRedirect`] on it, not as a template draft that
//! quietly pretends nothing happened.
//!
//! [`draft_view`] and [`DraftView`] are the same path with a screen at the end
//! of it. Nothing about the policy changes: the view calls [`draft_reply`] and
//! appends no audit entry of its own. What it adds is the shape a WebView may
//! receive — counts, four sentences this module owns, and the drafted text —
//! and the two facts that are true by construction rather than by field:
//! `never_sent`, because there is no code path in this product that sends one,
//! and an exemption that is always absent, because forwarding one message's
//! original prose needs a per-turn consent screen this round does not have.

use serde::Serialize;
use uuid::Uuid;

use soul_policy::audit::{append_or_store_error, AuditContent};
use soul_policy::hitl::{
    ActionKind, ActionRequest, CapabilityScope, HitlDenial, PlanHash, RequestOrigin,
};
use soul_policy::injection::UntrustedText;
use soul_policy::net_guard::OriginError;
use soul_policy::redactor::{KnownIdentifiers, OneShotExemption, Turn};
use soul_policy::ReasonCode;
use soul_schema::audit::{AuditAction, AuditCounts};
use soul_store::SqlCipherStore;
use soul_store_api::types::{StoreError, StoreResult};
use soul_store_api::{BlobStore, GraphStore};

use crate::commands::graph as graph_commands;
use crate::commands::policy::{e1_plan, E1Refusal, PolicySession, TokenRefused};
use crate::commands::profile as profile_commands;
use crate::commands::shell::{self, Session, ViewRefused, ENDPOINT_UNUSABLE_EXPLANATION};
use crate::commands::store::StoreSlot;
use crate::Config;

/// The WP10 types a caller names, re-exported so a host does not have to add
/// `soul-draft` to its own manifest to use this surface.
pub use soul_draft::{
    DraftOutcome, DraftRoute, DraftStats, PastedTurn, PeopleSummary, SummaryClaim,
};

/// Everything one draft needs, with the exemption inside it.
///
/// The exemption is a field rather than a parameter so it is moved in with the
/// rest of the request and dropped with it. `OneShotExemption` is neither
/// `Clone` nor `Copy`, which makes this struct neither, which is the point: a
/// request cannot be replayed and a caller cannot keep one around.
#[derive(Debug)]
pub struct DraftRequest<'a> {
    /// Whose voice to draft in.
    pub profile_id: Uuid,
    /// What the user pasted, in the order they pasted it.
    pub pasted: Vec<PastedTurn>,
    /// The model name to ask the endpoint for. Part of the approved plan.
    pub model: &'a str,
    /// Permission to include one turn's original prose in this one request,
    /// minted here and now by `ExemptionRequest::for_turn(id).confirm(true)`.
    pub exemption: Option<OneShotExemption>,
    pub now_ms: u64,
    pub at_unix_seconds: i64,
}

impl<'a> DraftRequest<'a> {
    pub fn new(
        profile_id: Uuid,
        pasted: Vec<PastedTurn>,
        model: &'a str,
        now_ms: u64,
        at_unix_seconds: i64,
    ) -> DraftRequest<'a> {
        DraftRequest {
            profile_id,
            pasted,
            model,
            exemption: None,
            now_ms,
            at_unix_seconds,
        }
    }

    /// Carry one turn's original prose this once.
    pub fn including(mut self, exemption: OneShotExemption) -> DraftRequest<'a> {
        self.exemption = Some(exemption);
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DraftCommandError {
    #[error(transparent)]
    Hitl(#[from] HitlDenial),

    /// v0.1 will not mint the capability this route needs.
    #[error(transparent)]
    TokenRefused(#[from] TokenRefused),

    /// The one request did not happen. The reason code is on it, so a caller
    /// can tell a refused redirect from an endpoint that is simply down.
    #[error("the generation request did not go through: {0}")]
    Generation(#[source] E1Refusal),

    #[error(transparent)]
    Draft(#[from] soul_draft::DraftError),

    #[error(transparent)]
    Profile(#[from] soul_profile::ProfileError),

    #[error(transparent)]
    Graph(#[from] soul_graph::GraphError),

    #[error(transparent)]
    Store(#[from] StoreError),
}

impl DraftCommandError {
    /// The code the audit entry carries, where the refusal had one.
    ///
    /// Public because a refused draft has to be explainable in the UI without
    /// showing the user an error string, and because a test asserting "the
    /// user got a readable failure" needs something to assert on.
    pub fn reason_code(&self) -> Option<ReasonCode> {
        match self {
            DraftCommandError::Hitl(denial) => Some(denial.reason_code()),
            DraftCommandError::TokenRefused(refused) => Some(refused.reason),
            DraftCommandError::Generation(refusal) => Some(refusal.reason_code()),
            _ => None,
        }
    }
}

/// The session a draft runs under, decided by the configuration.
///
/// `Some(url)` is the endpoint the user typed in; `None` is the shipped
/// default, and [`PolicySession::closed`] reaches nothing at all — not the
/// vendor, and not a model somebody left running on loopback.
pub fn session_for(
    config: &Config,
    identifiers: KnownIdentifiers,
) -> Result<PolicySession, OriginError> {
    match config.llm_endpoint.as_deref() {
        Some(url) => PolicySession::with_user_endpoint(url, identifiers),
        None => Ok(PolicySession::new(
            soul_policy::net_guard::EgressConfig::closed(),
            identifiers,
        )),
    }
}

/// The names the redactor must placehold, read out of the contact graph.
///
/// A contact row has no name on it: the label is sealed, and this opens it.
/// The plaintext goes into the placeholder dictionary and nowhere else — never
/// into a turn, never into a plan, never into anything that could end up in a
/// request body. A label whose content key has been destroyed is skipped,
/// because a name nobody can recover is also a name that cannot leak.
///
/// An empty result is worth noticing: it means the second redaction rule, the
/// one that catches a two-character name, has nothing to match on.
pub fn known_identifiers(store: &SqlCipherStore) -> StoreResult<KnownIdentifiers> {
    let mut identifiers = KnownIdentifiers::new();
    for contact in store.list_contacts()? {
        let Some(sealed) = contact.display_label_ref.as_ref() else {
            continue;
        };
        let Ok(bytes) = store.open(sealed) else {
            continue;
        };
        if let Ok(label) = String::from_utf8(bytes) {
            identifiers.add_name(label.trim());
        }
    }
    Ok(identifiers)
}

/// Draft one reply. Returns the draft; nothing sends it.
pub fn draft_reply(
    store: &mut SqlCipherStore,
    session: &mut PolicySession,
    request: DraftRequest<'_>,
) -> Result<DraftOutcome, DraftCommandError> {
    let at_unix_seconds = request.at_unix_seconds;
    let mut audit: Vec<AuditContent> = Vec::new();
    let drafted = draft_reply_inner(store, session, request, &mut audit);
    finish(store, audit, at_unix_seconds, drafted)
}

fn draft_reply_inner(
    store: &mut SqlCipherStore,
    session: &mut PolicySession,
    request: DraftRequest<'_>,
    audit: &mut Vec<AuditContent>,
) -> Result<DraftOutcome, DraftCommandError> {
    // 1. The voice as it stands now. Read on every draft, never cached: AC-07
    //    is the difference between the value the user just set and the value
    //    this process happened to start with.
    let voice = profile_commands::voice(store, request.profile_id)?;

    // 2. The paste becomes turns, and the scan writes down what it saw. It
    //    does not filter: the turns below are the turns above.
    let pasted = soul_draft::turns_from(request.pasted);
    audit.extend(soul_draft::injection_audit(&pasted));

    let route = match session.guard().config().e1_endpoint() {
        Some(_) => DraftRoute::E1,
        None => DraftRoute::Template,
    };

    let mut turns: Vec<Turn> = Vec::with_capacity(pasted.len() + 1);
    if route == DraftRoute::E1 {
        // The voice reaches the wire as a turn of the user's own. The
        // instruction slot is a constant in `soul_policy::e1` and stays one.
        turns.push(soul_draft::tone_turn(&voice));
    }
    turns.extend(pasted);

    // 3. Counts, a model name and four enum words. No prose in the plan, and
    //    `draft.reply` needs no capability token — the paste is material, and
    //    material does not ask for anything.
    let plan = serde_json::json!({
        "action": ActionKind::DraftReply.as_str(),
        "turns": turns.len(),
        "third_party_turns": turns.iter().filter(|turn| turn.is_third_party()).count(),
        "voice": soul_draft::voice_plan(&voice),
        "route": route.as_str(),
    });
    let approved = match session.check_action(
        &ActionRequest::new(ActionKind::DraftReply.as_str(), RequestOrigin::User).with_plan(plan),
        request.now_ms,
    ) {
        Ok(approved) => approved,
        Err(denial) => {
            audit.push(AuditContent::denied(
                AuditAction::HitlDeny,
                denial.reason_code(),
            ));
            return Err(DraftCommandError::Hitl(denial));
        }
    };

    // 4. The only two ways to a request body, and the exemption is eaten here.
    let body = match request.exemption {
        Some(exemption) => session.redact_with_exemption(&turns, exemption),
        None => session.redact(&turns),
    };
    let stats = DraftStats::of(&body, turns.len());

    // 5. One request, or none at all.
    let text = match route {
        DraftRoute::Template => soul_draft::template_draft(&voice, &body),
        DraftRoute::E1 => {
            let exempted = body.exempted_turn();
            let plan_hash = PlanHash::of(&e1_plan(request.model, &body));
            let token =
                match session.issue_token(CapabilityScope::E1Generate, plan_hash, request.now_ms) {
                    Ok(token) => token,
                    Err(refused) => {
                        audit.push(AuditContent::denied(
                            AuditAction::CapabilityReject,
                            refused.reason,
                        ));
                        return Err(DraftCommandError::TokenRefused(refused));
                    }
                };
            match session.e1_generate(request.model, body, token.token_id(), request.now_ms) {
                Ok(outcome) => {
                    let mut entry = outcome.audit();
                    if let Some(turn_id) = exempted {
                        entry = entry.about(&[turn_id]);
                    }
                    audit.push(entry);
                    soul_draft::answer_text(&outcome.body)?.as_str().to_owned()
                }
                Err(refusal) => {
                    audit.push(refusal.audit());
                    return Err(DraftCommandError::Generation(refusal));
                }
            }
        }
    };

    // 6. The assertion, then the entry: what happened, over how many turns,
    //    under which plan. Not a word of what was drafted.
    let outcome = DraftOutcome::new(text, route, stats)?;
    audit.push(
        AuditContent::allowed(AuditAction::DraftCreate, ReasonCode::Routine)
            .counting(AuditCounts {
                items: Some(stats.turns() as u64),
                bytes: None,
            })
            .for_plan(approved.plan_hash.as_str()),
    );
    Ok(outcome)
}

/// What is supported about one person, from what is stored on this machine.
///
/// Counts and evidence ids, assembled locally. There is no endpoint branch
/// here at all: the statistical reading is the summary, not a degraded version
/// of one, so a configured endpoint changes nothing about what this returns.
///
/// No audit entry either. The chain records egress and decisions; reading the
/// user's own graph back is neither, and an `inference.write` entry for a
/// summary that writes no inference would be a false statement in a log whose
/// value is that it does not make any.
pub fn people_summary(
    store: &SqlCipherStore,
    session: &mut PolicySession,
    contact_id: Uuid,
    now_ms: u64,
) -> Result<PeopleSummary, DraftCommandError> {
    let plan = serde_json::json!({
        "action": ActionKind::AnalysePeople.as_str(),
        "contact_id": contact_id.to_string(),
    });
    session.check_action(
        &ActionRequest::new(ActionKind::AnalysePeople.as_str(), RequestOrigin::User)
            .with_plan(plan),
        now_ms,
    )?;

    let graph = graph_commands::load(store)?;
    let edges: Vec<soul_graph::model::TieEdge> =
        graph.edges_for(contact_id).into_iter().cloned().collect();

    // Resolved one edge at a time, and a failure to resolve is the answer:
    // an edge whose evidence has gone is a claim nothing supports, so the
    // caller gets an error rather than a shorter list.
    let mut resolved = Vec::with_capacity(edges.len());
    for edge in &edges {
        resolved.push(graph_commands::edge_evidence(store, edge)?);
    }
    let ties: Vec<soul_draft::ResolvedTie<'_>> = edges
        .iter()
        .zip(resolved.iter())
        .map(|(edge, evidence)| soul_draft::ResolvedTie::new(edge, evidence))
        .collect();

    Ok(soul_draft::summarize(contact_id, &ties)?)
}

/// Append what the draft owes the chain, then return what happened.
///
/// The entries go in on both paths: a refused draft is a thing that happened.
/// A failure to write one is only reported when the draft itself succeeded,
/// because replacing a refusal with a storage error would hide the reason the
/// user actually needs.
fn finish(
    store: &mut SqlCipherStore,
    audit: Vec<AuditContent>,
    at_unix_seconds: i64,
    drafted: Result<DraftOutcome, DraftCommandError>,
) -> Result<DraftOutcome, DraftCommandError> {
    let mut write_failure: Option<StoreError> = None;
    for content in audit {
        if let Err(error) = append_or_store_error(store, content, at_unix_seconds) {
            write_failure.get_or_insert(error);
        }
    }
    match (drafted, write_failure) {
        (Ok(outcome), None) => Ok(outcome),
        (Ok(_), Some(error)) => Err(DraftCommandError::Store(error)),
        (Err(error), _) => Err(error),
    }
}

// --------------------------------------------------------------- the view

/// The model name the view path asks for.
///
/// It reaches nothing in this build. A view draft runs under the session
/// [`session_for`] returns for a configuration with no endpoint, which is
/// every configuration the shell can produce — there is no way to write
/// `Config::llm_endpoint` from the desktop — so the route is always
/// [`DraftRoute::Template`] and the E1 branch that would use this name is
/// unreachable. It is a constant rather than a caller's argument so that
/// remains a fact about the code instead of a habit of the shell.
pub const VIEW_MODEL: &str = "local-model";

/// What the draft page says under the result, whatever the result is.
///
/// The claim is structural, not aspirational: `soul-draft` has no transport,
/// this crate's only outbound path is `PolicySession::e1_generate`, and
/// nothing anywhere takes a finished draft as an argument. The last clause is
/// there because the honest version of "we will not send it" also has to say
/// who decides.
pub const DRAFT_NEVER_SENT_EXPLANATION: &str =
    "这是一份草稿。本产品没有把它发出去的代码路径：要不要用、发给谁、什么时候发，都由你自己决定。";

/// How the template route describes itself, in one line under the draft.
pub const TEMPLATE_ROUTE_LABEL: &str = "本机模板写成，没有任何字节出网。";

/// How the E1 route would describe itself. No shipped configuration reaches
/// it; the line exists so the label is visibly chosen per route rather than
/// being a constant that happens to be true today.
pub const E1_ROUTE_LABEL: &str = "由你自己配置的模型端点写成，只发出了那一次请求。";

/// What the draft page says when the paste box was empty.
pub const EMPTY_PASTE_EXPLANATION: &str = "粘贴框是空的。先把要回复的那段话贴进来，再让它起草。";

/// One draft, in the shape a WebView may hold.
///
/// Private fields and one constructor, which is what makes `never_sent`
/// worth reading: there is no second way to build this value and no setter, so
/// the field cannot be written `false` by any caller — the way
/// `FilePlanPreview::written_to_disk` is structurally false in
/// `soul-fileplan`.
///
/// `Serialize` and not `Deserialize`, for the same reason: a value that could
/// be parsed back could be parsed out of anything, and then "this draft was
/// never sent" would be a claim about a JSON document rather than about this
/// build.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DraftView {
    text: String,
    /// `"template"` or `"e1"`, from [`DraftRoute::as_str`].
    route: String,
    /// The same route as a sentence the shell renders verbatim.
    route_label: String,
    turns: usize,
    third_party_turns: usize,
    placeheld_turns: usize,
    /// Always false on this path: the view never mints an exemption.
    carries_exempted_original: bool,
    /// Always true; see the type note.
    never_sent: bool,
    notice: String,
}

impl DraftView {
    pub fn of(outcome: &DraftOutcome) -> DraftView {
        let stats = outcome.stats();
        DraftView {
            text: outcome.text().to_owned(),
            route: outcome.route().as_str().to_owned(),
            route_label: route_label(outcome.route()).to_owned(),
            turns: stats.turns(),
            third_party_turns: stats.third_party_turns(),
            placeheld_turns: stats.placeheld_turns(),
            carries_exempted_original: stats.carries_exempted_original(),
            never_sent: true,
            notice: DRAFT_NEVER_SENT_EXPLANATION.to_owned(),
        }
    }
}

const fn route_label(route: DraftRoute) -> &'static str {
    match route {
        DraftRoute::Template => TEMPLATE_ROUTE_LABEL,
        DraftRoute::E1 => E1_ROUTE_LABEL,
    }
}

/// Draft a reply from what the user pasted, for the draft page.
///
/// The order is the policy and it is fixed. The store is looked for first,
/// because everything after it records something; the paste is checked next,
/// so an empty box costs no audit entry; and only then does anything happen
/// that the chain has to remember.
///
/// Every pasted item becomes a [`PastedTurn::unattributed`] turn. The page has
/// no "who said this" control and inventing one here would be worse than not
/// having it: `Mixed` is counted as somebody else's by the redactor, so an
/// unattributed line is placeheld rather than quoted, and a mistaken `Owner`
/// would put a third party's prose in front of a model.
pub fn draft_view(
    slot: &StoreSlot,
    session: &Session,
    pasted: Vec<String>,
) -> Result<DraftView, ViewRefused> {
    let Some(mut store) = slot.lock() else {
        return Err(ViewRefused::no_store_opened());
    };

    let turns: Vec<PastedTurn> = pasted
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| PastedTurn::unattributed(UntrustedText::new(line.to_owned())))
        .collect();
    if turns.is_empty() {
        return Err(ViewRefused::empty_paste(EMPTY_PASTE_EXPLANATION));
    }

    let config = session.config();
    let identifiers =
        known_identifiers(&store).map_err(|error| ViewRefused::refused(None, error.to_string()))?;
    let mut policy = session_for(&config, identifiers)
        .map_err(|_| ViewRefused::refused(None, ENDPOINT_UNUSABLE_EXPLANATION))?;

    let (now_ms, at_unix_seconds) = shell::wall_clock();
    let request = DraftRequest::new(
        session.draft_profile_id(),
        turns,
        VIEW_MODEL,
        now_ms,
        at_unix_seconds,
    );

    let outcome = draft_reply(&mut store, &mut policy, request)
        .map_err(|error| ViewRefused::refused(error.reason_code(), error.to_string()))?;
    Ok(DraftView::of(&outcome))
}
