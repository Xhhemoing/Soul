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
//! `summarize_person` invents no audit action of its own, and that is
//! deliberate rather than an omission: reading the graph and the evidence rows
//! the graph already cites changes nothing, and
//! `docs/schemas/audit.schema.json` is frozen with no action for it. The
//! entries it can hand back are the ones the *rephrasing* owes — `egress.request`
//! for a request that left, and whatever the refusal already names when one
//! did not. Nothing new is claimed on behalf of the summary itself.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_draft::analysis::{self, PersonSummary, SummarySource};
use soul_draft::brief::ProfileBrief;
use soul_draft::draft::{BodyFacts, Draft, DraftRequest, Drafter, ReplyGenerator};
use soul_draft::error::{DraftError, GenerationRefused};
use soul_graph::GraphError;
use soul_policy::audit::AuditContent;
use soul_policy::hitl::{
    ActionKind, ActionRequest, CapabilityScope, HitlDenial, PlanHash, RequestOrigin,
};
use soul_policy::redactor::{
    ExemptionRequest, KnownIdentifiers, OneShotExemption, RedactedBody, Redactor,
};
use soul_policy::ReasonCode;
use soul_profile::ProfileError;
use soul_schema::audit::AuditAction;
use soul_schema::contact::ContactClass;
use soul_schema::memory::ForgetState;
use soul_store::SqlCipherStore;
use soul_store_api::types::StoreResult;
use soul_store_api::{BlobStore, GraphStore};

use crate::commands::policy::{e1_plan, E1Refusal, PolicySession, TokenRefused};

pub use soul_draft::draft::{
    DraftSource, NeverSent, DEGRADED_NOTICE, ENDPOINT_NOTICE, NOT_SENT_NOTICE, TEMPLATE_NOTICE,
};
pub use soul_draft::template::BODY_SLOT;

// Re-exported so the desktop shell can name what it receives without taking
// `soul-draft` and `soul-policy` as dependencies of its own. A forwarder that
// had them would be a forwarder that could assemble a request body.
pub use soul_draft::brief::ProfileBrief as DraftBrief;
pub use soul_draft::draft::Draft as DraftValue;
pub use soul_policy::redactor::KnownIdentifiers as DraftIdentifiers;

/// What the user is told before a generation request goes out.
///
/// A constant rather than a sentence assembled in the interface, for the same
/// reason WP09 made the cloud notice one: it is a promise about what the
/// build does, and a promise kept in TypeScript is one the Rust tests cannot
/// check.
///
/// It names the request body rather than the panel. The earlier wording said
/// 只有下面这些内容 while the list underneath it is counts, a plan hash and a
/// preparation id — none of which are in the JSON — and said nothing about the
/// owner's profile brief, which is. `soul_policy::e1::E1RequestPlan::json_body`
/// and `soul_draft::draft` are the two places to check this against: a model
/// name, one fixed system instruction, and one user-material message holding
/// the rendered [`ProfileBrief`] and the pasted turn.
pub const E1_PLAN_NOTICE: &str = "确认之后，会发到你自己配置的模型端点的是这些：模型名、\
     一段固定的系统指令，以及一段引用材料——里面是你自己的档案摘要（口吻、口吻来源、\
     有证据支持的要点，和「工作假设，非临床结论」那句），加上你粘贴的这一段。\
     第三人正文默认已占位，只有你二次确认「这一条按原文带上」时才按原文发出，而且只这一次；\
     姓名与账号两种情况下都占位。下面的段数、计划哈希与准备编号是给你核对用的，\
     不在发出去的内容里。草稿生成之后仍然由你自己决定要不要发出去，Soul 不会替你发送。";

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
/// [`KnownIdentifiers`] with [`known_identifiers`] and hands it to both, which
/// is what [`DraftSession::set_identifiers`] and
/// [`PolicySession::set_identifiers`] are for.
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

    /// Teach this session's redactor who exists, keeping the model name.
    ///
    /// `Redactor` holds its set by value and `Drafter` holds the redactor, so
    /// the pair is rebuilt rather than reached into. Only the set changes: a
    /// shell that has just read the contact rows has not changed which model
    /// it is asking.
    ///
    /// Any prepared body is dropped. It was redacted under the previous set,
    /// and a body built before Soul knew a name is exactly the one that must
    /// not be what a later approval sends — the counts the user read would
    /// still describe it, because a name placeheld inside a turn changes
    /// neither `third_party_turns` nor `placeheld_turns`.
    pub fn set_identifiers(&mut self, identifiers: KnownIdentifiers) {
        let model = self.drafter.model().to_owned();
        self.drafter = Drafter::new(Redactor::new(identifiers), model);
        self.pending = None;
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
        let plan_hash = PlanHash::of(&e1_plan(
            self.drafter.model(),
            policy.guard().config().e1_endpoint(),
            &body,
        ));
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
    ///   id, an approval the user gave for one message would send another;
    /// * re-deriving the hash from the session as it is *now* catches a
    ///   destination that changed while the plan was on screen. The origin is
    ///   part of what was hashed, which is `SECURITY.md`'s 配置变更会使计划哈希
    ///   失效 rather than a second rule beside it.
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

        // The plan names the origin it would reach, so a 设置 page pointed
        // somewhere else between the two steps makes the hash the user
        // approved stop describing this session. `SECURITY.md` asks for
        // exactly that, and it is checked here rather than only inside
        // `e1_generate` so that no token is minted and no address is opened:
        // neither the endpoint the plan was described against nor the one that
        // replaced it hears anything.
        let current = PlanHash::of(&e1_plan(
            self.drafter.model(),
            policy.guard().config().e1_endpoint(),
            &pending.body,
        ));
        if current != pending.plan_hash {
            return Err(HitlDenial::PlanHashMismatch {
                approved: pending.plan_hash.as_str().to_owned(),
                current: current.as_str().to_owned(),
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
/// can leave; and the set is empty, because the contact rows that fill it need
/// an open store and this is the constructor for a caller that has none. A
/// caller that does have one calls [`known_identifiers`] and hands the answer
/// to both sessions — `Session::sync_identifiers` is the one place in the
/// product that does. The shape-based scrub still catches addresses, handles
/// and long digit runs, and a third-party turn is placeheld whole regardless
/// of who is in it.
pub fn closed_session() -> (DraftSession, PolicySession) {
    (
        DraftSession::new(UNNAMED_MODEL, KnownIdentifiers::new()),
        PolicySession::closed(),
    )
}

/// The names in this store, as the one set both redactors are given.
///
/// The other end of what [`closed_session`] cannot do. Shape matching catches
/// what looks like an identifier — an address, an `@handle`, a run of digits
/// long enough to be a phone number — and nothing can make it catch 李雷,
/// which is two characters that also occur in ordinary sentences. Only a list
/// of the names this machine actually holds can, and the display labels on the
/// contact rows are that list.
///
/// Four kinds of row are left out, and each omission is a decision:
///
/// * [`ContactClass::Owner`]. PRODUCT_LOCK's placeholder is for 第三人姓名;
///   registering the user's own name would redact them out of their own
///   drafts, and the brief that travels with every request is written in it.
/// * anybody whose [`ForgetState`] is no longer `Active`. A name Soul has been
///   told to forget is not a name it may keep in a set in memory.
/// * a label whose content key has been destroyed. [`BlobStore::open`] answers
///   `ContentKeyDestroyed`, and one unreadable label leaves that person out
///   rather than costing the other names their placeholder.
/// * a contact with no label at all, which is most of a `soul-import-v1`
///   corpus: those lines carry a `sender_id` and no display name.
///
/// `identifiers` is not read. Those are digests — `soul-import` hashes a
/// handle before it is written — and there is nothing here that tries to turn
/// one back into a number. Numbers and handles are the shapes the scrub
/// already covers.
///
/// Labels are opened, which is the reason this is here rather than in
/// `graph.rs`: that module draws people and states that it opens no seal, and
/// the view it builds still carries no name. What is opened here goes into a
/// redactor and nowhere else — not to the interface, not into an audit entry,
/// and not into `config.json`.
pub fn known_identifiers(store: &SqlCipherStore) -> StoreResult<KnownIdentifiers> {
    let mut identifiers = KnownIdentifiers::new();
    for contact in store.list_contacts()? {
        if contact.contact_class == ContactClass::Owner
            || contact.forget_state != ForgetState::Active
        {
            continue;
        }
        let Some(sealed) = contact.display_label_ref.as_ref() else {
            continue;
        };
        let Ok(opened) = store.open(sealed) else {
            continue;
        };
        if let Ok(label) = String::from_utf8(opened) {
            identifiers.add_name(label.trim());
        }
    }
    Ok(identifiers)
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
/// The brief is a parameter rather than [`ProfileBrief::neutral`], because
/// AC-07 is about the voice the user pinned reaching the thing that writes:
/// a caller with a store open builds one with [`brief`], and a caller with no
/// store — the headless smoke, a session whose database did not open — passes
/// the neutral one and gets the voice a blank profile would have given anyway.
///
/// What comes back is [`Drafted`], not the draft alone. The entries are the
/// caller's to append for the reason the type says: whoever holds the open
/// store is the one that can write them, and a draft whose `draft.create`
/// entry was dropped on the floor is a draft `/audit` never heard about.
///
/// This is the local path: no request body is built, so there is nothing for
/// a bug to leak. The endpoint path is [`DraftSession::prepare`] and
/// [`DraftSession::generate`], which need a screen for the person in between.
pub fn draft_pasted(
    drafting: &DraftSession,
    policy: &mut PolicySession,
    brief: ProfileBrief,
    pasted: &str,
) -> Result<Drafted, DraftRefusal> {
    let request = DraftRequest::from_paste(brief, pasted);
    drafting.draft_offline(
        policy,
        &request,
        RequestOrigin::User,
        soul_policy::clock::now_unix_millis(),
    )
}

/// Step one of the endpoint path, for something a person pasted.
///
/// The sibling of [`draft_pasted`], and the two make the same two decisions:
/// the origin is [`RequestOrigin::User`] because a call arriving over the IPC
/// is a click, and the clock is read here because a shell entry point is the
/// outermost edge.
///
/// What comes back is a description, not a draft. Nothing has left: the body
/// is built and held so that the counts the user is about to read are the
/// counts of the request that would go out, and [`generate_prepared`] is the
/// only thing that can spend it.
///
/// `include_original` is the second confirmation PRODUCT_LOCK asks for, and
/// the screen it comes from is the confirmation panel: the user has already
/// read a plan saying every third-party turn is placeheld, and pressing
/// 「这一条按原文带上」 prepares the same paste again with this set. It is an
/// argument rather than state for the reason [`OneShotExemption`] is consumed
/// by value — a later call that does not pass `true` is placeheld again,
/// because there is nowhere for the permission to have been kept.
///
/// A paste is one third-party turn by construction, so there is exactly one
/// id an exemption could name.
///
/// The brief travels in the body's material slot, so the voice the user
/// pinned is part of what the plan's counts describe and part of what the
/// endpoint is asked to write like. See [`draft_pasted`] for why it is a
/// parameter.
pub fn prepare_pasted(
    drafting: &mut DraftSession,
    policy: &mut PolicySession,
    brief: ProfileBrief,
    pasted: &str,
    include_original: bool,
) -> Result<E1DraftPlan, DraftRefusal> {
    let request = DraftRequest::from_paste(brief, pasted);
    let exemption = match include_original {
        true => request
            .third_party_turn_ids()
            .first()
            .and_then(|turn_id| ExemptionRequest::for_turn(*turn_id).confirm(true)),
        false => None,
    };
    drafting.prepare(
        policy,
        request,
        exemption,
        RequestOrigin::User,
        soul_policy::clock::now_unix_millis(),
    )
}

/// Step two: the user read the plan and approved this exact preparation.
///
/// The approval has to echo both halves of what was on screen. An approval
/// that names another preparation, or another shape of the same preparation,
/// reaches no endpoint — [`DraftSession::generate`] takes the prepared body by
/// value before it checks, so a refusal also leaves nothing to approve twice.
pub fn generate_prepared(
    drafting: &mut DraftSession,
    policy: &mut PolicySession,
    approval: &Approval,
) -> Result<Drafted, DraftRefusal> {
    drafting.generate(policy, approval, soul_policy::clock::now_unix_millis())
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

/// One person summary, and what the chain is owed for it.
///
/// The same shape [`Drafted`] has, and for the same reason: the caller holds
/// the open store. A summary that never left the machine owes nothing, so the
/// list is empty on the counts path — which is every session with no endpoint
/// configured.
#[derive(Debug, Clone, PartialEq)]
pub struct Summarized {
    pub view: PersonSummaryView,
    pub audit: Vec<AuditContent>,
}

/// Everything Soul will say about one person, and what each line rests on.
///
/// AC-16. The evidence is resolved out of the store before the summary is
/// built, so a point cannot cite a row that has been forgotten; `soul-draft`
/// refuses the whole summary rather than returning a shorter one.
///
/// With an endpoint configured the counts are then offered to it for
/// rephrasing, which is the half PRODUCT_LOCK's 无 key 时统计降级 needs in
/// order to be a degradation rather than the only path there is. What goes out
/// is [`analysis::summary_body`] — the statements this machine derived, no
/// third-party prose — and the Graph click is the user trigger the token is
/// minted against, so there is no second screen and no second command.
///
/// AC-17 is the other half: an endpoint that refuses, times out, or answers
/// with something the clinical check drops leaves the counts summary exactly as
/// it was. The chain still hears about the attempt.
pub fn summarize_person(
    drafting: &DraftSession,
    policy: &mut PolicySession,
    store: &SqlCipherStore,
    contact_id: Uuid,
    origin: RequestOrigin,
    now_ms: u64,
) -> Result<Summarized, DraftRefusal> {
    allow(policy, ActionKind::AnalysePeople, origin, now_ms)?;

    let graph = soul_graph::load(store)?;
    let edge = graph
        .edges_for(contact_id)
        .into_iter()
        .max_by_key(|edge| edge.tie_strength.interaction_count)
        .ok_or(DraftError::NoSuchTie { contact_id })?
        .clone();
    let resolved = soul_graph::resolve_evidence(store, &edge)?;

    let counts = analysis::summarize_person(&graph, contact_id, &resolved)?;
    if policy.guard().config().e1_endpoint().is_none() {
        return Ok(Summarized {
            view: PersonSummaryView::of(&counts)?,
            audit: Vec::new(),
        });
    }

    // Cloned because `phrase_with` needs the redactor while the generator
    // below holds the session mutably. It is the same set either way.
    let redactor = policy.redactor().clone();
    let mut rephraser = Rephraser {
        model: drafting.model().to_owned(),
        policy,
        now_ms,
        audit: Vec::new(),
    };
    let phrased = analysis::phrase_with(&counts, &redactor, &mut rephraser).ok();
    let audit = std::mem::take(&mut rephraser.audit);

    Ok(Summarized {
        view: PersonSummaryView::of(phrased.as_ref().unwrap_or(&counts))?,
        audit,
    })
}

/// What the person summary's rephrasing is told when the endpoint did not
/// produce one. Never the endpoint's own words: a hostile endpoint controls
/// those, and this string is only ever read by the fallback above anyway.
const REPHRASING_REFUSED: &str = "the summary rephrasing did not come back";

/// One rephrasing request against the endpoint the user configured.
///
/// A [`ReplyGenerator`] rather than a free function because
/// [`analysis::phrase_with`] is what builds the body, and the capability token
/// has to be minted against the body that is actually going out rather than
/// against one assembled a second time beside it.
///
/// The audit entries are collected rather than returned, because the trait's
/// error type carries no room for one and because the entries are owed either
/// way: a request that left owes `egress.request`, and one that was refused
/// owes whatever [`DraftRefusal::audit`] already names.
struct Rephraser<'a> {
    policy: &'a mut PolicySession,
    model: String,
    now_ms: u64,
    audit: Vec<AuditContent>,
}

impl Rephraser<'_> {
    /// Record what the chain is owed, and say no without quoting anything.
    fn refused(&mut self, refusal: DraftRefusal) -> GenerationRefused {
        self.audit.extend(refusal.audit());
        GenerationRefused::new(REPHRASING_REFUSED).because(refusal.reason_code())
    }
}

impl ReplyGenerator for Rephraser<'_> {
    fn generate(&mut self, body: RedactedBody) -> Result<String, GenerationRefused> {
        let plan = PlanHash::of(&e1_plan(
            &self.model,
            self.policy.guard().config().e1_endpoint(),
            &body,
        ));
        let scope = CapabilityScope::E1Generate;
        let token_id = match self.policy.issue_token(scope, plan, self.now_ms) {
            Ok(token) => token.token_id(),
            Err(refused) => return Err(self.refused(DraftRefusal::Token(refused))),
        };
        match self
            .policy
            .e1_generate(&self.model, body, token_id, self.now_ms)
        {
            Ok(outcome) => {
                self.audit.push(outcome.audit());
                Ok(outcome.body)
            }
            Err(refusal) => Err(self.refused(refusal.into())),
        }
    }
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
                    band: crate::commands::graph::band_word(point.band()).to_owned(),
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

    /// The audit entry this refusal owes the chain, where it owes one.
    ///
    /// No action is invented here: `hitl.deny`, `capability.reject` and
    /// `egress.request` are the three `audit.schema.json` already has for a
    /// refused generation, and each variant is mapped onto the one it is.
    /// [`E1Refusal::audit`] decides for itself, because a refusal that never
    /// reached the guard and one the guard turned away are different entries.
    ///
    /// `None` for the variants that describe a value rather than a decision —
    /// a profile that will not resolve, a graph that has no such tie. There is
    /// no denial there for the chain to record, and a `hitl.deny` written for
    /// one would say the user was refused something they never asked for.
    pub fn audit(&self) -> Option<AuditContent> {
        match self {
            DraftRefusal::E1(refusal) => Some(refusal.audit()),
            DraftRefusal::Hitl(HitlDenial::Token(_)) => Some(AuditContent::denied(
                AuditAction::CapabilityReject,
                self.reason_code(),
            )),
            DraftRefusal::Hitl(_)
            | DraftRefusal::NothingPrepared
            | DraftRefusal::NotThePreparedRequest => Some(AuditContent::denied(
                AuditAction::HitlDeny,
                self.reason_code(),
            )),
            DraftRefusal::Token(refused) => Some(AuditContent::denied(
                AuditAction::CapabilityReject,
                refused.reason,
            )),
            DraftRefusal::Draft(_) | DraftRefusal::Profile(_) | DraftRefusal::Graph(_) => None,
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
