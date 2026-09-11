//! Drafting a reply, which is not the same thing as sending one.
//!
//! PRODUCT_LOCK: *出站消息 v0.1 只起草，不发送*. Nothing in this crate has a
//! recipient, an address book entry, or a verb that means "deliver"; the one
//! thing that can go on a socket is a generation request, and it goes to the
//! origin an [`EgressPermit`](soul_policy::EgressPermit) names. [`Draft`] says
//! so in its type: `delivery` is a [`NeverSent`], which serializes to `false`
//! and has no other inhabitant.
//!
//! ## Where the prompt goes
//!
//! `soul-policy` defines the request as exactly two slots: a constant
//! instruction, and one slot of quoted external material that the instruction
//! tells the model not to obey. WP10 does not widen that. So the profile
//! brief travels *in the material slot*, above the conversation, and the
//! instruction stays the constant it was. That ordering is deliberate:
//!
//! * the instruction position is unreachable from anything that varies at
//!   runtime, which is the whole of AC-25 on the drafting side — an injection
//!   string cannot become an instruction because no runtime value can;
//! * the brief is descriptive, never imperative, so nothing is lost by it
//!   being labelled as material rather than as an order.
//!
//! The cost is written down in `docs/STATUS.md`: a build that wants the voice
//! in the system message has to change `soul-policy::e1`, and that is a change
//! to the permission surface, which is where the review attention belongs.
//!
//! ## Where the redaction goes
//!
//! [`Drafter::redact`] is the only way a body is built, and it feeds every
//! turn — the brief included — through [`Redactor`]. A third-party turn
//! becomes a placeholder unless the user has confirmed twice for that one
//! turn, and the exemption is consumed by value, so the next draft places it
//! back behind a placeholder without anyone having to remember to.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use soul_policy::audit::{AuditContent, ReasonCode};
use soul_policy::injection::{self, InjectionSignal};
use soul_policy::redactor::{OneShotExemption, RedactedBody, Redactor, Turn};
use soul_schema::audit::{AuditAction, AuditCounts, AuditDecision};
use soul_schema::common::SealedSubject;

use crate::brief::ProfileBrief;
use crate::error::{DraftResult, GenerationRefused};
use crate::reply::{self, ReplyDefect};
use crate::template::{self, TemplateContext};

/// The sentence every draft carries, whichever path produced it.
pub const NOT_SENT_NOTICE: &str = "这是草稿。Soul 不会替你发送，检查和修改之后由你自己发出去。";

/// Why a draft came out of the local template.
pub const TEMPLATE_NOTICE: &str =
    "本次没有用模型：草稿由本机确定性语气模板生成，没有任何内容离开本机。";

/// Why a draft that was supposed to come from the endpoint came out of the
/// template anyway.
pub const DEGRADED_NOTICE: &str = "端点的回复不能用，已退回本机确定性语气模板。";

/// Why a draft came from the user's own endpoint.
pub const ENDPOINT_NOTICE: &str =
    "草稿由你配置的模型端点生成。送出去的请求里，第三人正文默认已占位。";

/// The turn id the profile brief travels under.
///
/// Fixed rather than fresh, and never a third-party subject, so a one-shot
/// exemption — which names exactly one turn — can never be pointed at it.
pub const BRIEF_TURN_ID: Uuid = Uuid::from_u128(0x0192b0c0_5010_7a10_8b10_000000000010);

/// Serializes to `false`. There is no other value, because v0.1 does not send.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NeverSent;

impl Serialize for NeverSent {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bool(false)
    }
}

impl<'de> Deserialize<'de> for NeverSent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match bool::deserialize(d)? {
            false => Ok(NeverSent),
            true => Err(serde::de::Error::invalid_value(
                serde::de::Unexpected::Bool(true),
                &"false",
            )),
        }
    }
}

/// Which path produced the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftSource {
    /// The deterministic tone template. No key, no network, no model.
    ToneTemplate,
    /// The OpenAI-compatible endpoint the user configured.
    UserEndpoint,
}

/// Why the endpoint path fell back to the template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Degradation {
    /// The answer was not JSON, or had no assistant message in it.
    ReplyUnreadable,
    /// The assistant message was empty.
    ReplyEmpty,
    /// The answer carried vocabulary this product must not say.
    ReplyClinical,
}

impl Degradation {
    fn of(defect: &ReplyDefect) -> Degradation {
        match defect {
            ReplyDefect::NotJson | ReplyDefect::NoMessage => Degradation::ReplyUnreadable,
            ReplyDefect::Empty => Degradation::ReplyEmpty,
            ReplyDefect::Clinical(_) => Degradation::ReplyClinical,
            // Drafting asks for prose rather than for something made out of
            // material Soul supplied, so it never calls
            // `reply::read_grounded_in` and neither of these can arrive here.
            // A multi-line draft is in particular a fine draft: it is prose
            // the user copies elsewhere, not a line shown under a label. They
            // are mapped rather than left unreachable: a panic in a
            // degradation label would be a crash an endpoint could ask for.
            ReplyDefect::Ungrounded | ReplyDefect::NotOneLine => Degradation::ReplyUnreadable,
        }
    }
}

/// The conversation a draft is being written against, plus who the user is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftRequest {
    brief: ProfileBrief,
    turns: Vec<Turn>,
}

impl DraftRequest {
    pub fn new(brief: ProfileBrief, turns: Vec<Turn>) -> DraftRequest {
        DraftRequest { brief, turns }
    }

    /// The paste box: whatever the user pasted is one turn belonging to
    /// somebody else.
    ///
    /// Third-party by default and not by inspection. Soul cannot tell from the
    /// text whose words they are, and the safe reading of "I pasted a message
    /// I received" is the one that keeps it off the wire.
    pub fn from_paste(brief: ProfileBrief, pasted: &str) -> DraftRequest {
        DraftRequest::new(
            brief,
            vec![Turn::new(Uuid::now_v7(), SealedSubject::ThirdParty, pasted)],
        )
    }

    pub fn brief(&self) -> &ProfileBrief {
        &self.brief
    }

    pub fn brief_mut(&mut self) -> &mut ProfileBrief {
        &mut self.brief
    }

    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }

    /// Ids of the third-party turns, which are the ones an exemption may name.
    pub fn third_party_turn_ids(&self) -> Vec<Uuid> {
        self.turns
            .iter()
            .filter(|turn| turn.is_third_party())
            .map(|turn| turn.turn_id)
            .collect()
    }

    /// The counts the deterministic template works from.
    pub fn context(&self) -> TemplateContext {
        TemplateContext::replying_to(
            self.turns
                .iter()
                .filter(|turn| turn.is_third_party())
                .count(),
        )
    }

    /// What the turns tried to do, per turn. For the audit entry only.
    pub fn injection_signals(&self) -> Vec<InjectionSignal> {
        let mut signals: Vec<InjectionSignal> = self
            .turns
            .iter()
            .flat_map(|turn| injection::scan(&turn.body))
            .collect();
        signals.sort();
        signals.dedup();
        signals
    }
}

/// One draft. Never sent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    /// The text the user may copy. Everything else on this struct is about
    /// where it came from.
    pub text: String,
    pub source: DraftSource,
    /// Always `false`.
    pub delivery: NeverSent,
    pub third_party_turns: usize,
    pub placeheld_turns: usize,
    /// True only for the one request the user confirmed twice for.
    pub carries_exempted_original: bool,
    pub degraded: Option<Degradation>,
    /// What the material — pasted or returned — tried to do. Recorded so the
    /// audit entry can be written; nothing branches on it.
    pub injection_signals: Vec<String>,
    pub not_sent_notice: String,
    pub source_notice: String,
}

impl Draft {
    /// The audit entries this draft owes the chain.
    ///
    /// Built here and appended by whoever holds the open store, which is how
    /// `soul-graph` and `soul-policy` already do it. Counts and codes only:
    /// there is no field on an `AuditContent` that could hold the draft.
    pub fn audit(&self) -> Vec<AuditContent> {
        let reason = match (self.source, self.carries_exempted_original) {
            (DraftSource::ToneTemplate, _) => ReasonCode::Routine,
            (DraftSource::UserEndpoint, true) => ReasonCode::ThirdPartyBodyIncluded,
            (DraftSource::UserEndpoint, false) => ReasonCode::ThirdPartyBodyPlaceheld,
        };
        let mut entries = vec![
            AuditContent::new(AuditAction::DraftCreate, AuditDecision::Allowed)
                .because(reason)
                .counting(AuditCounts {
                    items: Some(self.third_party_turns as u64),
                    bytes: None,
                }),
        ];

        if !self.injection_signals.is_empty() {
            entries.push(
                AuditContent::denied(
                    AuditAction::InjectionBlocked,
                    ReasonCode::InjectionMarkersFound,
                )
                .counting(AuditCounts {
                    items: Some(self.injection_signals.len() as u64),
                    bytes: None,
                }),
            );
        }
        entries
    }
}

/// What a body that went out was carrying, once the body itself is gone.
///
/// Read off a [`RedactedBody`] before it is handed to the network, because
/// the body is consumed by the call and these three numbers are what the
/// draft and its audit entry have to report. Counts only: there is no field
/// here that could hold a word of what was in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BodyFacts {
    pub third_party_turns: usize,
    pub placeheld_turns: usize,
    pub carries_exempted_original: bool,
}

impl BodyFacts {
    pub fn of(body: &RedactedBody) -> BodyFacts {
        BodyFacts {
            third_party_turns: body.third_party_turns(),
            placeheld_turns: body.placeheld_turns(),
            carries_exempted_original: body.carries_exempted_original(),
        }
    }
}

/// Turns a prepared request body into model text.
///
/// The seam between this crate and the network. An implementation may put
/// exactly one thing on the wire — a generation request to the origin an
/// `EgressPermit` names — and it is handed a [`RedactedBody`], which only
/// `soul-policy`'s redactor can produce. There is no method here that takes a
/// URL, and none that takes raw prose.
///
/// The body is taken by value: one body, one request. A signature that lent it
/// out would let an implementation keep a copy, and a copy of a body carrying
/// an exempted original is the one thing AC-13 says must not survive the call.
pub trait ReplyGenerator {
    fn generate(&mut self, body: RedactedBody) -> Result<String, GenerationRefused>;
}

/// Composes a draft out of a profile, a conversation and a redactor.
#[derive(Debug, Clone)]
pub struct Drafter {
    redactor: Redactor,
    model: String,
}

impl Drafter {
    pub fn new(redactor: Redactor, model: impl Into<String>) -> Drafter {
        Drafter {
            redactor,
            model: model.into(),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn redactor(&self) -> &Redactor {
        &self.redactor
    }

    /// The request body, exactly as it would go on the wire.
    ///
    /// Public because the caller needs it to describe the request in the plan
    /// it asks the user to approve, and because a test has to be able to look
    /// at the bytes. `exemption` is taken by value and dropped here: there is
    /// nowhere for it to be remembered, which is what AC-13 means.
    pub fn redact(
        &self,
        request: &DraftRequest,
        exemption: Option<OneShotExemption>,
    ) -> DraftResult<RedactedBody> {
        let mut turns = Vec::with_capacity(request.turns.len() + 1);
        turns.push(Turn::new(
            BRIEF_TURN_ID,
            SealedSubject::Owner,
            request.brief.render()?,
        ));
        turns.extend(request.turns.iter().cloned());

        Ok(match exemption {
            Some(exemption) => self
                .redactor
                .redact_for_e1_with_exemption(&turns, exemption),
            None => self.redactor.redact_for_e1(&turns),
        })
    }

    /// AC-17: no endpoint, no model, no socket.
    ///
    /// The body is not built at all on this path — there is nothing to build
    /// it for — so a draft with no key cannot leak by construction rather than
    /// by a check that happens to pass.
    pub fn draft_offline(&self, request: &DraftRequest) -> DraftResult<Draft> {
        let text = template::render(&request.brief, request.context())?;
        Ok(self.finish(
            request,
            text,
            DraftSource::ToneTemplate,
            None,
            BodyFacts::default(),
            Vec::new(),
        ))
    }

    /// The endpoint path: redact, generate, read the answer as data.
    ///
    /// A refusal to reach the endpoint is returned as an error, because AC-11
    /// wants a refused cross-origin redirect to be visible rather than papered
    /// over. An answer that arrives but cannot be used falls back to the
    /// template and says so in [`Draft::degraded`] — that is a content
    /// problem, not a permission one, and the user still gets a draft.
    pub fn draft_with<G: ReplyGenerator>(
        &self,
        request: &DraftRequest,
        exemption: Option<OneShotExemption>,
        generator: &mut G,
    ) -> DraftResult<Draft> {
        let body = self.redact(request, exemption)?;
        let facts = BodyFacts::of(&body);
        let raw = generator.generate(body)?;
        self.finish_reply(request, facts, &raw)
    }

    /// Read an answer whose request was made elsewhere.
    ///
    /// `soulcore` needs this because its E1 path runs through
    /// `PolicySession::e1_generate`, which is where the capability token is
    /// spent — the body leaves in one call and the answer arrives from
    /// another. `facts` is what that body was carrying, taken before it was
    /// handed over, because the counts a draft reports have to describe the
    /// bytes that actually went out rather than a body rebuilt afterwards.
    /// Rebuilding would be wrong twice over: the exemption is gone by then.
    pub fn finish_reply(
        &self,
        request: &DraftRequest,
        facts: BodyFacts,
        raw: &str,
    ) -> DraftResult<Draft> {
        match reply::read(raw) {
            Ok(answer) => Ok(self.finish(
                request,
                answer.text,
                DraftSource::UserEndpoint,
                None,
                facts,
                answer.signals,
            )),
            Err(defect) => {
                let text = template::render(&request.brief, request.context())?;
                Ok(self.finish(
                    request,
                    text,
                    DraftSource::ToneTemplate,
                    Some(Degradation::of(&defect)),
                    facts,
                    Vec::new(),
                ))
            }
        }
    }

    fn finish(
        &self,
        request: &DraftRequest,
        text: String,
        source: DraftSource,
        degraded: Option<Degradation>,
        facts: BodyFacts,
        reply_signals: Vec<InjectionSignal>,
    ) -> Draft {
        let mut signals = request.injection_signals();
        signals.extend(reply_signals);
        signals.sort();
        signals.dedup();

        let source_notice = match (source, degraded.is_some()) {
            (DraftSource::UserEndpoint, _) => ENDPOINT_NOTICE,
            (DraftSource::ToneTemplate, true) => DEGRADED_NOTICE,
            (DraftSource::ToneTemplate, false) => TEMPLATE_NOTICE,
        };

        Draft {
            text,
            source,
            delivery: NeverSent,
            third_party_turns: facts.third_party_turns,
            placeheld_turns: facts.placeheld_turns,
            carries_exempted_original: facts.carries_exempted_original,
            degraded,
            injection_signals: signals
                .into_iter()
                .map(|signal| signal.as_str().to_owned())
                .collect(),
            not_sent_notice: NOT_SENT_NOTICE.to_owned(),
            source_notice: source_notice.to_owned(),
        }
    }
}

/// Every key a serialized [`Draft`] may carry.
///
/// Read by `tests/never_sends.rs`, which asserts there is no field on the
/// value the shell receives that could name a recipient or express an action.
/// `deny_unknown_fields` catches the reverse — a field added here and not
/// there fails to deserialize.
pub const DRAFT_FIELDS: &[&str] = &[
    "text",
    "source",
    "delivery",
    "third_party_turns",
    "placeheld_turns",
    "carries_exempted_original",
    "degraded",
    "injection_signals",
    "not_sent_notice",
    "source_notice",
];

/// The serialized draft, for a caller crossing a process boundary with it.
pub fn to_value(draft: &Draft) -> Value {
    serde_json::to_value(draft).expect("a Draft is a plain record and always serializes")
}
