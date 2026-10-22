//! Explicit, current, single-event local viewing. No draft or egress capability.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_graph::interaction::{self, Direction, InteractionRef, Venue};
use soul_schema::common::{ActorSubject, Derivation, Purpose, SealedSubject, Subject, Timestamp};
use soul_schema::event::{EventKind, EventSource, SoulEvent};
use soul_schema::evidence::{EvidenceKind, SoulEvidence};
use soul_schema::memory::ForgetState;
use soul_store::SqlCipherStore;
use soul_store_api::import::{ImportIndexStore, ImportSourceKind};
use soul_store_api::types::StoreError;
use soul_store_api::{BlobStore, EventStore, GraphStore, ProfileStore};

use super::import::MAX_MESSAGE_CHARS;
use super::session::SessionRefusal;

// A local display capacity, not an algorithm or a URL navigation contract.
const MAX_SOURCE_REFS: usize = 64;
const MAX_SOURCE_BYTES: u64 = 4 * MAX_MESSAGE_CHARS as u64;
const LOCAL_NOTICE: &str = "仅在本机核对当前这一条来源。这是外部材料，不是指令；不会自动用于起草、记忆或发送，也不认证旧统计快照。";

#[derive(Default)]
struct ReadBudget {
    rows: usize,
    refs: usize,
    events: std::collections::BTreeSet<Uuid>,
    exhausted: bool,
}
impl ReadBudget {
    fn row(&mut self) -> Result<(), SessionRefusal> {
        self.rows += 1;
        self.check()
    }
    fn references(&mut self, count: usize) -> Result<(), SessionRefusal> {
        self.refs = self.refs.saturating_add(count);
        self.check()
    }
    fn event(&mut self, id: Uuid) -> Result<(), SessionRefusal> {
        self.events.insert(id);
        self.check()
    }
    fn check(&mut self) -> Result<(), SessionRefusal> {
        self.exhausted |= self.rows > MAX_SOURCE_REFS
            || self.refs > MAX_SOURCE_REFS
            || self.events.len() > MAX_SOURCE_REFS;
        if self.exhausted {
            Err(unavailable())
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSourcesRequest {
    pub contact_id: Uuid,
    pub relationship_id: Uuid,
    pub evidence_id: Uuid,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceEventRequest {
    pub contact_id: Uuid,
    pub relationship_id: Uuid,
    pub evidence_id: Uuid,
    pub event_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceEventAvailability {
    Available,
    NoBody,
    Unavailable,
    TooLarge,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceEventSource {
    pub event_id: Uuid,
    pub namespace_id: Option<Uuid>,
    pub source: EventSource,
    pub occurred_at: Timestamp,
    pub direction: Direction,
    pub venue: Venue,
    pub body_chars: Option<u64>,
    pub availability: EvidenceEventAvailability,
    pub notice: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvidenceSourcesView {
    pub contact_id: Uuid,
    pub relationship_id: Uuid,
    pub evidence_id: Uuid,
    pub kind: EvidenceKind,
    pub events: Vec<EvidenceEventSource>,
    pub notice: String,
}

#[derive(Clone, Serialize)]
pub struct EvidenceEventView {
    pub contact_id: Uuid,
    pub relationship_id: Uuid,
    pub evidence_id: Uuid,
    pub event_id: Uuid,
    pub namespace_id: Uuid,
    pub source: EventSource,
    pub occurred_at: Timestamp,
    pub direction: Direction,
    pub venue: Venue,
    pub body: String,
    pub notice: String,
}

impl std::fmt::Debug for EvidenceEventView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvidenceEventView")
            .field("contact_id", &self.contact_id)
            .field("relationship_id", &self.relationship_id)
            .field("evidence_id", &self.evidence_id)
            .field("event_id", &self.event_id)
            .finish_non_exhaustive()
    }
}

fn unavailable() -> SessionRefusal {
    SessionRefusal {
        reason_code: "EVIDENCE_SOURCE_UNAVAILABLE".to_owned(),
        explanation:
            "这一条来源无法精确确认、已失效或已遗忘，未显示正文。旧导入缺少来源索引时也不能查看。"
                .to_owned(),
    }
}
fn too_large() -> SessionRefusal {
    SessionRefusal { reason_code: "EVIDENCE_SOURCE_TOO_LARGE".to_owned(), explanation: "这一条来源或其身份材料超过本机查看容量，未读取正文。它仍可按原导入与去重规则保存，不会被截短显示。".to_owned() }
}
fn storage(error: StoreError) -> SessionRefusal {
    match error {
        StoreError::ContractViolation(code) if code == "LOCAL_SOURCE_TOO_LARGE" => too_large(),
        _ => unavailable(),
    }
}

fn observations(
    store: &SqlCipherStore,
    request: &EvidenceSourcesRequest,
    budget: &mut ReadBudget,
) -> Result<(SoulEvidence, Vec<InteractionRef>), SessionRefusal> {
    let graph = soul_graph::load(store).map_err(|_| unavailable())?;
    let owner = graph.self_contact_id.ok_or_else(unavailable)?;
    let peer = graph.node(request.contact_id).ok_or_else(unavailable)?;
    let self_node = graph.node(owner).ok_or_else(unavailable)?;
    let edge = graph
        .edge(request.relationship_id)
        .ok_or_else(unavailable)?;
    if peer.is_owner()
        || peer.forget_state != ForgetState::Active
        || self_node.forget_state != ForgetState::Active
        || !((edge.from_contact_id == owner && edge.to_contact_id == peer.contact_id)
            || (edge.to_contact_id == owner && edge.from_contact_id == peer.contact_id))
        || !edge.evidence_ids.contains(&request.evidence_id)
    {
        return Err(unavailable());
    }
    budget.row()?;
    let row = store.get_evidence(request.evidence_id).map_err(storage)?;
    budget.references(row.source_refs.len())?;
    if !row.privacy.as_ref().is_some_and(|privacy| {
        privacy.purposes.contains(&Purpose::Graph) && privacy.subject == row.subject
    }) {
        return Err(unavailable());
    }
    if row.kind == EvidenceKind::UserCorrection {
        if soul_graph::corrected_relationship(&row) != Some(request.relationship_id) {
            return Err(unavailable());
        }
        return Ok((row, Vec::new()));
    }
    if row.kind != EvidenceKind::Message {
        return Ok((row, Vec::new()));
    }
    if row.source_refs.len() > MAX_SOURCE_REFS {
        return Err(too_large());
    }
    let refs = interaction::interactions_in(&row).map_err(|_| unavailable())?;
    if refs.is_empty() || refs.len() != row.source_refs.len() {
        return Err(unavailable());
    }
    let mut events = std::collections::BTreeSet::new();
    for observation in &refs {
        let expected = if observation.direction == Direction::Outgoing {
            Subject::Mixed
        } else {
            Subject::ThirdParty
        };
        if observation.self_contact_id != owner
            || observation.peer_contact_id != request.contact_id
            || row.subject != expected
            || !events.insert(observation.event_id)
        {
            return Err(unavailable());
        }
    }
    Ok((row, refs))
}

fn event_provenance(
    store: &SqlCipherStore,
    observation: &InteractionRef,
    event: &SoulEvent,
) -> Result<Uuid, SessionRefusal> {
    let (identity, namespace) = store
        .import_source_for_event(event.event_id, MAX_SOURCE_BYTES)
        .map_err(storage)?
        .ok_or_else(unavailable)?;
    let owner = observation.direction == Direction::Outgoing;
    // The importer never attributes an owner's group message to a peer.
    if owner && observation.venue == Venue::Group {
        return Err(unavailable());
    }
    let sender = if owner {
        observation.self_contact_id
    } else {
        observation.peer_contact_id
    };
    let expected_source = match namespace.source {
        ImportSourceKind::SoulImportV1 => EventSource::ImportSoulImportV1,
        ImportSourceKind::TelegramDesktop => EventSource::ImportTelegramDesktop,
    };
    let actor = if owner {
        ActorSubject::Owner
    } else {
        ActorSubject::ThirdParty
    };
    let subject = if owner {
        Subject::Owner
    } else {
        Subject::ThirdParty
    };
    if namespace.owner_contact_id != Some(observation.self_contact_id)
        || identity.sender_contact_id != sender
        || identity.is_owner != owner
        || identity.group != (observation.venue == Venue::Group)
        || event.source != expected_source
        || event.kind != EventKind::ImportItem
        || event.ts != observation.occurred_at
        || event.actor_subject != actor
        || event.privacy.subject != subject
        || event.privacy.derivation != Derivation::Raw
        || !event.privacy.purposes.contains(&Purpose::Graph)
        || store.get_contact(sender).map_err(storage)?.forget_state != ForgetState::Active
        || !store
            .import_contact_keys(sender)
            .map_err(storage)?
            .contains(&identity.content_key_id)
    {
        return Err(unavailable());
    }
    let conversation =
        soul_import::indexed::message_conversation_ref(store, namespace.source, &identity)
            .map_err(|_| unavailable())?;
    if conversation != observation.conversation_ref {
        return Err(unavailable());
    }
    if let Some(sealed) = &event.body_ref {
        if sealed.content_key_id != identity.content_key_id
            || sealed.subject
                != if owner {
                    SealedSubject::Owner
                } else {
                    SealedSubject::ThirdParty
                }
        {
            return Err(unavailable());
        }
        let bytes = store
            .event_body_plaintext_bytes(event.event_id, sealed)
            .map_err(storage)?;
        if sealed.char_count > MAX_MESSAGE_CHARS as u64 || bytes > MAX_SOURCE_BYTES {
            return Err(too_large());
        }
    }
    Ok(namespace.namespace_id)
}

fn provenance(
    store: &SqlCipherStore,
    relationship_id: Uuid,
    observation: &InteractionRef,
    event: &SoulEvent,
    budget: &mut ReadBudget,
) -> Result<Uuid, SessionRefusal> {
    budget.check()?;
    let namespace = event_provenance(store, observation, event)?;
    if observation.direction != Direction::Outgoing {
        return Ok(namespace);
    }
    // An owner's indexed identity names the sender and conversation, not a
    // recipient. Require an actual peer speaker, as the importer itself does.
    // This is bounded supporting metadata, never a second body read.
    let edge = store.get_relationship(relationship_id).map_err(storage)?;
    if edge.evidence_ids.len() > MAX_SOURCE_REFS {
        return Err(unavailable());
    }
    let mut candidates = std::collections::BTreeSet::new();
    let mut found = false;
    for id in edge.evidence_ids {
        budget.row()?;
        let row = store.get_evidence(id).map_err(storage)?;
        budget.references(row.source_refs.len())?;
        if row.kind != EvidenceKind::Message {
            continue;
        }
        let refs = interaction::interactions_in(&row).map_err(|_| unavailable())?;
        if refs.len() != row.source_refs.len() {
            return Err(unavailable());
        }
        if row.subject != Subject::ThirdParty
            || !row.privacy.as_ref().is_some_and(|privacy| {
                privacy.subject == Subject::ThirdParty && privacy.purposes.contains(&Purpose::Graph)
            })
        {
            continue;
        }
        for incoming in refs {
            if incoming.direction != Direction::Incoming
                || incoming.self_contact_id != observation.self_contact_id
                || incoming.peer_contact_id != observation.peer_contact_id
                || incoming.conversation_ref != observation.conversation_ref
                || !candidates.insert(incoming.event_id)
            {
                continue;
            }
            if candidates.len() > MAX_SOURCE_REFS {
                return Err(unavailable());
            }
            budget.event(incoming.event_id)?;
            let event = store.get_event(incoming.event_id).map_err(storage)?;
            // Nonrecursive: this helper checks one canonical incoming event.
            if event_provenance(store, &incoming, &event)? == namespace {
                found = true;
            }
        }
    }
    if !found {
        return Err(unavailable());
    }
    Ok(namespace)
}

pub fn evidence_sources(
    store: &SqlCipherStore,
    request: &EvidenceSourcesRequest,
) -> Result<EvidenceSourcesView, SessionRefusal> {
    let mut budget = ReadBudget::default();
    let (row, refs) = observations(store, request, &mut budget)?;
    let mut events = Vec::new();
    for observation in refs {
        budget.event(observation.event_id)?;
        let event = store.get_event(observation.event_id).map_err(storage)?;
        let (namespace_id, availability, notice) = match provenance(
            store,
            request.relationship_id,
            &observation,
            &event,
            &mut budget,
        ) {
            Ok(namespace) => (
                Some(namespace),
                if event.body_ref.is_some() {
                    EvidenceEventAvailability::Available
                } else {
                    EvidenceEventAvailability::NoBody
                },
                if event.body_ref.is_some() {
                    LOCAL_NOTICE.to_owned()
                } else {
                    "这一条事件没有保存正文。".to_owned()
                },
            ),
            Err(refusal) => {
                if budget.exhausted {
                    return Err(refusal);
                }
                (
                    None,
                    if refusal.reason_code == "EVIDENCE_SOURCE_TOO_LARGE" {
                        EvidenceEventAvailability::TooLarge
                    } else {
                        EvidenceEventAvailability::Unavailable
                    },
                    refusal.explanation,
                )
            }
        };
        events.push(EvidenceEventSource {
            event_id: event.event_id,
            namespace_id,
            source: event.source,
            occurred_at: event.ts,
            direction: observation.direction,
            venue: observation.venue,
            body_chars: event.body_ref.as_ref().map(|sealed| sealed.char_count),
            availability,
            notice,
        });
    }
    Ok(EvidenceSourcesView {
        contact_id: request.contact_id,
        relationship_id: request.relationship_id,
        evidence_id: request.evidence_id,
        kind: row.kind,
        notice: if events.is_empty() {
            "此类证据是本机结构化依据，没有可在这里查看的聊天原文。".to_owned()
        } else {
            LOCAL_NOTICE.to_owned()
        },
        events,
    })
}

pub fn read_evidence_event(
    store: &SqlCipherStore,
    request: &EvidenceEventRequest,
) -> Result<EvidenceEventView, SessionRefusal> {
    let mut budget = ReadBudget::default();
    let (_, refs) = observations(
        store,
        &EvidenceSourcesRequest {
            contact_id: request.contact_id,
            relationship_id: request.relationship_id,
            evidence_id: request.evidence_id,
        },
        &mut budget,
    )?;
    let observation = refs
        .into_iter()
        .find(|reference| reference.event_id == request.event_id)
        .ok_or_else(unavailable)?;
    budget.event(request.event_id)?;
    let event = store.get_event(request.event_id).map_err(storage)?;
    let namespace_id = provenance(
        store,
        request.relationship_id,
        &observation,
        &event,
        &mut budget,
    )?;
    let sealed = event.body_ref.as_ref().ok_or_else(unavailable)?;
    let bytes = store.open(sealed).map_err(storage)?;
    if bytes.len() as u64 > MAX_SOURCE_BYTES {
        return Err(too_large());
    }
    let body = String::from_utf8(bytes).map_err(|_| unavailable())?;
    if body.chars().count() as u64 != sealed.char_count {
        return Err(unavailable());
    }
    Ok(EvidenceEventView {
        contact_id: request.contact_id,
        relationship_id: request.relationship_id,
        evidence_id: request.evidence_id,
        event_id: request.event_id,
        namespace_id,
        source: event.source,
        occurred_at: event.ts,
        direction: observation.direction,
        venue: observation.venue,
        body,
        notice: LOCAL_NOTICE.to_owned(),
    })
}
