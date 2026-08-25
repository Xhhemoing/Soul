//! Putting a parsed import into the encrypted store.
//!
//! Four kinds of row come out of one file, and the order matters:
//!
//! 1. **contacts**, one per person the file mentions, matched against what is
//!    already there by identifier digest so a second import of an overlapping
//!    export does not clone everybody;
//! 2. **sealed bodies**, one per message with text, under the sender's content
//!    key — which is also the key behind their display label, so forgetting a
//!    contact takes what they wrote with it;
//! 3. **events**, one per message, carrying a pointer to the sealed body and
//!    never the body;
//! 4. **interaction evidence**, one per (message, person the user was talking
//!    to), which is what WP05 reads to build the graph.
//!
//! Nothing here treats the file as authority. A body arrives as
//! [`UntrustedText`], is sealed, and is pointed at. It is scanned — for the
//! audit entry, not to decide anything — and the count of lines that tried
//! something goes into an `injection.blocked` entry so the user can see that
//! their export contained an attempt and that it went nowhere.
//!
//! Committing the same file twice writes the events twice. v0.1 has no
//! external-id index to deduplicate against, and inventing one would mean a
//! column holding a platform's message id. The caller decides.

use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use soul_graph::interaction::{
    conversation_ref, interaction_evidence, Direction, InteractionRef, Venue,
};
use soul_policy::audit::{AuditContent, ReasonCode};
use soul_policy::injection;
use soul_policy::redactor::{NAME_PLACEHOLDER, THIRD_PARTY_PLACEHOLDER};
use soul_schema::audit::{AuditAction, AuditCounts};
use soul_schema::common::{
    ActorSubject, Derivation, E0Deny, E1Disposition, EgressPolicy, Privacy, Purpose,
    ResearchDisposition, Retention, SchemaVersion, SealedSubject, Sha256Hex, Subject,
};
use soul_schema::contact::{ContactClass, ContactIdentifier, SoulContact};
use soul_schema::event::{EventKind, SoulEvent};
use soul_schema::memory::ForgetState;
use soul_schema::soul_import_v1::SenderScope;
use soul_store_api::types::{SealRequest, StoreError};
use soul_store_api::{BlobStore, EventStore, GraphStore, ProfileStore};

use crate::model::{ImportSource, ParticipantHandle, StagedImport, StagedParticipant};

/// Anything that can go wrong once a file has already parsed.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("the store refused: {0}")]
    Store(#[from] StoreError),

    /// No participant is the user. Without one there is no centre to the graph
    /// and no way to say which messages are the user's own.
    #[error("the file names nobody as the account owner, so nothing can be attributed")]
    NoOwner,

    /// A message names a sender the participant pass did not see. The parsers
    /// register every sender they read, so this means they disagree.
    #[error("message {index} names a sender that is not among the file's participants")]
    UnknownSender { index: usize },
}

/// What one commit wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReceipt {
    pub source: Option<ImportSource>,
    pub self_contact_id: Option<Uuid>,
    pub contacts_created: Vec<Uuid>,
    pub contacts_matched: Vec<Uuid>,
    pub events_written: Vec<Uuid>,
    pub evidence_written: Vec<Uuid>,
    /// Messages whose body carried injection markers. They were stored like
    /// any other data and obeyed by nothing.
    pub messages_with_injection_markers: u64,
    /// The entries the caller owes the chain, built here and appended by
    /// whoever holds the open store.
    pub audit: Vec<AuditContent>,
}

impl ImportReceipt {
    pub fn contacts_touched(&self) -> Vec<Uuid> {
        let mut all = self.contacts_created.clone();
        all.extend(self.contacts_matched.iter().copied());
        all
    }
}

/// Privacy for an imported message.
///
/// `e1: placeholder` is the whole of the third-party promise on the storage
/// side: the row may be reasoned about locally, and if any of it ever reaches
/// the user's own endpoint the redactor puts a placeholder there first.
fn imported_privacy(subject: Subject) -> Privacy {
    Privacy {
        subject,
        derivation: Derivation::Raw,
        purposes: vec![Purpose::SoulProfile, Purpose::Graph],
        retention: Retention::until_forgotten(),
        egress: EgressPolicy {
            e0: E0Deny,
            e1: E1Disposition::Placeholder,
            research_export: ResearchDisposition::Deny,
        },
    }
}

/// One person, once they have a place in the store.
#[derive(Debug, Clone)]
struct ResolvedContact {
    contact_id: Uuid,
    content_key_id: Uuid,
    is_owner: bool,
}

/// Write a parsed import into the store.
pub fn commit<S>(store: &mut S, staged: &StagedImport) -> Result<ImportReceipt, ImportError>
where
    S: EventStore + ProfileStore + GraphStore + BlobStore,
{
    if staged.owner().is_none() {
        return Err(ImportError::NoOwner);
    }

    let mut receipt = ImportReceipt {
        source: Some(staged.source),
        ..ImportReceipt::default()
    };

    let by_handle = resolve_contacts(store, staged, &mut receipt)?;
    let owner = by_handle
        .values()
        .find(|resolved| resolved.is_owner)
        .cloned()
        .ok_or(ImportError::NoOwner)?;
    receipt.self_contact_id = Some(owner.contact_id);

    let speakers = speakers_by_conversation(staged, &by_handle);

    for (index, message) in staged.messages.iter().enumerate() {
        let sender = by_handle
            .get(&message.sender)
            .ok_or(ImportError::UnknownSender { index })?
            .clone();

        let subject = match message.scope {
            SenderScope::Owner => Subject::Owner,
            SenderScope::ThirdParty => Subject::ThirdParty,
        };
        let event_id = Uuid::now_v7();

        let body_ref = match message.body.is_empty() {
            true => None,
            false => {
                let sealed_subject = match message.scope {
                    SenderScope::Owner => SealedSubject::Owner,
                    SenderScope::ThirdParty => SealedSubject::ThirdParty,
                };
                let mut request = SealRequest::new(
                    sender.content_key_id,
                    event_id,
                    "body_ref",
                    sealed_subject,
                    message.body.as_str().as_bytes().to_vec(),
                );
                if sealed_subject == SealedSubject::ThirdParty {
                    request = request.with_placeholder(THIRD_PARTY_PLACEHOLDER);
                }
                Some(store.seal(request)?)
            }
        };

        store.append_event(SoulEvent {
            schema_version: SchemaVersion,
            event_id,
            ts: message.occurred_at.clone(),
            source: staged.source.event_source(),
            kind: EventKind::ImportItem,
            actor_subject: match message.scope {
                SenderScope::Owner => ActorSubject::Owner,
                SenderScope::ThirdParty => ActorSubject::ThirdParty,
            },
            consent_id: None,
            privacy: imported_privacy(subject),
            body_ref,
        })?;
        receipt.events_written.push(event_id);

        // Recorded, never obeyed: the scan feeds the audit entry below and
        // nothing else in this function looks at the result.
        if injection::looks_like_injection(&message.body) {
            receipt.messages_with_injection_markers += 1;
        }

        let peers: Vec<Uuid> = match sender.is_owner {
            // The user talking: everyone who has spoken in this conversation
            // heard it. Restricting to people who spoke keeps a large silent
            // group from generating an edge per lurker.
            true => speakers
                .get(&message.conversation_id)
                .map(|set| set.iter().copied().collect())
                .unwrap_or_default(),
            false => vec![sender.contact_id],
        };

        for peer_id in peers {
            if peer_id == owner.contact_id {
                continue;
            }
            let evidence_id = Uuid::now_v7();
            let observation = InteractionRef::new(
                event_id,
                owner.contact_id,
                peer_id,
                conversation_ref(staged.source.as_str(), &message.conversation_id),
                match sender.is_owner {
                    true => Direction::Outgoing,
                    false => Direction::Incoming,
                },
                message.occurred_at.clone(),
                match message.group {
                    true => Venue::Group,
                    false => Venue::Direct,
                },
            );
            // An outgoing message is the user's own words about someone else;
            // an incoming one is the other person's. Both are handled as
            // third-party data downstream.
            let evidence_subject = match sender.is_owner {
                true => Subject::Mixed,
                false => Subject::ThirdParty,
            };
            store.put_evidence(interaction_evidence(
                evidence_id,
                evidence_subject,
                &observation,
            ))?;
            receipt.evidence_written.push(evidence_id);
        }
    }

    receipt.audit.push(
        AuditContent::allowed(AuditAction::ImportCommit, ReasonCode::Routine)
            .about(&receipt.contacts_touched())
            .counting(AuditCounts {
                items: Some(receipt.events_written.len() as u64),
                bytes: None,
            }),
    );
    if receipt.messages_with_injection_markers > 0 {
        receipt.audit.push(
            AuditContent::denied(
                AuditAction::InjectionBlocked,
                ReasonCode::InjectionMarkersFound,
            )
            .counting(AuditCounts {
                items: Some(receipt.messages_with_injection_markers),
                bytes: None,
            }),
        );
    }

    Ok(receipt)
}

/// Give every participant a contact row, reusing one when the identifiers
/// match something already stored.
fn resolve_contacts<S>(
    store: &mut S,
    staged: &StagedImport,
    receipt: &mut ImportReceipt,
) -> Result<BTreeMap<ParticipantHandle, ResolvedContact>, ImportError>
where
    S: GraphStore + BlobStore,
{
    let existing = store.list_contacts()?;
    let mut known: BTreeMap<String, &SoulContact> = BTreeMap::new();
    for contact in &existing {
        for identifier in contact.identifiers.iter().flatten() {
            known.insert(identifier.value_hash.as_str().to_owned(), contact);
        }
    }

    let mut resolved = BTreeMap::new();
    for participant in &staged.participants {
        let hashes: Vec<Sha256Hex> = participant
            .handles
            .iter()
            .map(|handle| handle.value_hash(staged.source))
            .collect();
        let matched = hashes
            .iter()
            .find_map(|hash| known.get(hash.as_str()).copied());

        let contact_id = matched
            .map(|contact| contact.contact_id)
            .unwrap_or_else(Uuid::now_v7);
        let content_key_id = matched
            .and_then(|contact| contact.display_label_ref.as_ref())
            .map(|sealed| sealed.content_key_id)
            .unwrap_or_else(Uuid::now_v7);

        match matched.is_some() {
            true => receipt.contacts_matched.push(contact_id),
            false => receipt.contacts_created.push(contact_id),
        }

        let display_label_ref = seal_label(
            store,
            participant,
            contact_id,
            content_key_id,
            matched.and_then(|contact| contact.display_label_ref.clone()),
        )?;

        store.put_contact(SoulContact {
            schema_version: SchemaVersion,
            contact_id,
            contact_class: match participant.is_owner {
                true => ContactClass::Owner,
                false => ContactClass::ThirdParty,
            },
            display_label_ref,
            identifiers: Some(
                hashes
                    .into_iter()
                    .map(|value_hash| ContactIdentifier {
                        kind: soul_schema::contact::IdentifierKind::PlatformUid,
                        value_hash,
                    })
                    .collect(),
            ),
            forget_state: matched
                .map(|contact| contact.forget_state)
                .unwrap_or(ForgetState::Active),
        })?;

        for handle in &participant.handles {
            resolved.insert(
                handle.clone(),
                ResolvedContact {
                    contact_id,
                    content_key_id,
                    is_owner: participant.is_owner,
                },
            );
        }
    }
    Ok(resolved)
}

/// Seal a display name, or keep the one already stored.
///
/// A label is somebody's name, so a third-party one is sealed with the same
/// placeholder the redactor uses. Re-importing does not reseal an unchanged
/// name: that would leave an orphaned blob behind for no reason.
fn seal_label<S: BlobStore>(
    store: &mut S,
    participant: &StagedParticipant,
    contact_id: Uuid,
    content_key_id: Uuid,
    existing: Option<soul_schema::common::SealedText>,
) -> Result<Option<soul_schema::common::SealedText>, ImportError> {
    let Some(label) = participant.display_label.as_ref() else {
        return Ok(existing);
    };
    if let Some(existing) = existing {
        if existing.char_count == label.char_count() as u64 {
            if let Ok(previous) = store.open(&existing) {
                if previous == label.as_str().as_bytes() {
                    return Ok(Some(existing));
                }
            }
        }
    }

    let subject = match participant.is_owner {
        true => SealedSubject::Owner,
        false => SealedSubject::ThirdParty,
    };
    let mut request = SealRequest::new(
        content_key_id,
        contact_id,
        "display_label_ref",
        subject,
        label.as_str().as_bytes().to_vec(),
    );
    if subject == SealedSubject::ThirdParty {
        request = request.with_placeholder(NAME_PLACEHOLDER);
    }
    Ok(Some(store.seal(request)?))
}

/// Who actually said something in each conversation, as contact ids.
fn speakers_by_conversation(
    staged: &StagedImport,
    by_handle: &BTreeMap<ParticipantHandle, ResolvedContact>,
) -> BTreeMap<String, BTreeSet<Uuid>> {
    let mut speakers: BTreeMap<String, BTreeSet<Uuid>> = BTreeMap::new();
    for message in &staged.messages {
        let Some(sender) = by_handle.get(&message.sender) else {
            continue;
        };
        if sender.is_owner {
            continue;
        }
        speakers
            .entry(message.conversation_id.clone())
            .or_default()
            .insert(sender.contact_id);
    }
    speakers
}
