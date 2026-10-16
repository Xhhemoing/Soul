//! Indexed imports use natural source ids, sealed under the existing forget keys.
//! The caller wraps commit, graph, audit and the operation receipt in one transaction.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

use soul_schema::common::{field_aad, ActorSubject, SchemaVersion, SealedSubject, SealedText};
use soul_schema::contact::{ContactClass, ContactIdentifier, SoulContact};
use soul_schema::memory::ForgetState;
use soul_schema::soul_import_v1::SenderScope;
use soul_store_api::import::{
    ImportCoverage, ImportIndexStore, ImportMessageIdentity, ImportNamespace, ImportNamespaceState,
    ImportParticipantIdentity, ImportSourceKind, LegacyImportChoice,
};
use soul_store_api::types::{SealRequest, StoreError};
use soul_store_api::{BlobStore, EventStore, GraphStore, ProfileStore};

use crate::commit::{self, ImportError, ImportReceipt, ResolvedContact};
use crate::model::{ImportSource, ParticipantHandle, StagedImport};

const BATCH: usize = 256;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct HandleKey(String, String);

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct MessageKey(String, String);

fn handle_key(handle: &ParticipantHandle) -> HandleKey {
    HandleKey(handle.kind_str().to_owned(), handle.value.clone())
}

fn source_kind(source: ImportSource) -> ImportSourceKind {
    match source {
        ImportSource::SoulImportV1 => ImportSourceKind::SoulImportV1,
        ImportSource::TelegramDesktop => ImportSourceKind::TelegramDesktop,
    }
}

#[derive(Clone)]
struct PlannedContact {
    existing: Option<SoulContact>,
    id: Uuid,
    key: Uuid,
}

/// Owned preparation dependencies; no raw id or body appears in Debug or IPC.
pub struct IndexedImportPlan {
    namespace: ImportNamespace,
    stored_namespace: Option<ImportNamespace>,
    legacy_choice: LegacyImportChoice,
    legacy_events: u64,
    contacts: Vec<PlannedContact>,
    participant_matches: BTreeMap<HandleKey, ImportParticipantIdentity>,
    message_matches: BTreeMap<usize, ImportMessageIdentity>,
    selected: BTreeSet<usize>,
    duplicates: u64,
}

impl std::fmt::Debug for IndexedImportPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexedImportPlan")
            .field("namespace_id", &self.namespace.namespace_id)
            .field("new_messages", &self.selected.len())
            .field("duplicates", &self.duplicates)
            .finish_non_exhaustive()
    }
}

impl IndexedImportPlan {
    pub fn namespace(&self) -> &ImportNamespace {
        &self.namespace
    }
    pub fn new_messages(&self) -> usize {
        self.selected.len()
    }
    pub fn new_evidence(&self, staged: &StagedImport) -> usize {
        let resolved: BTreeMap<_, _> = staged
            .participants
            .iter()
            .zip(&self.contacts)
            .flat_map(|(participant, contact)| {
                participant.handles.iter().map(move |handle| {
                    (
                        handle.clone(),
                        ResolvedContact {
                            contact_id: contact.id,
                            content_key_id: contact.key,
                            is_owner: participant.is_owner,
                        },
                    )
                })
            })
            .collect();
        let Some(owner) = resolved.values().find(|contact| contact.is_owner) else {
            return 0;
        };
        let speakers = commit::speakers_by_conversation(staged, &resolved);
        self.selected
            .iter()
            .map(|index| {
                let message = &staged.messages[*index];
                resolved
                    .get(&message.sender)
                    .map(|sender| {
                        commit::interaction_peers(message, sender, &speakers, owner.contact_id)
                            .len()
                    })
                    .unwrap_or(0)
            })
            .sum()
    }
    pub fn duplicate_messages(&self) -> u64 {
        self.duplicates
    }
    pub fn legacy_events(&self) -> u64 {
        self.legacy_events
    }
    pub fn contacts_created(&self) -> usize {
        self.contacts
            .iter()
            .filter(|c| c.existing.is_none())
            .count()
    }
    pub fn contacts_matched(&self) -> usize {
        self.contacts.len() - self.contacts_created()
    }
}

/// A new namespace is a local identity, never a filename or content digest.
pub fn new_namespace(source: ImportSource, coverage: ImportCoverage) -> ImportNamespace {
    ImportNamespace {
        namespace_id: Uuid::now_v7(),
        source: source_kind(source),
        state: ImportNamespaceState::Active,
        coverage,
        owner_contact_id: None,
        account_ref: None,
    }
}

/// Telegram supplies a stable account id; it cannot acquire a second namespace
/// merely because a caller asks for another local UUID.
pub fn telegram_namespace<S>(
    store: &S,
    staged: &StagedImport,
) -> Result<Option<ImportNamespace>, ImportError>
where
    S: ImportIndexStore + BlobStore,
{
    if staged.source != ImportSource::TelegramDesktop {
        return Err(ImportError::NamespaceRequired);
    }
    let owner = staged.owner().ok_or(ImportError::NoOwner)?;
    let account = handle_key(owner.handles.first().ok_or(ImportError::NoOwner)?);
    let mut matched = None;
    for namespace in store
        .import_namespaces()?
        .into_iter()
        .filter(|ns| ns.source == ImportSourceKind::TelegramDesktop)
    {
        let Some(sealed) = namespace.account_ref.as_ref() else {
            continue;
        };
        let account_owner = namespace
            .owner_contact_id
            .ok_or(ImportError::IdentityUnavailable)?;
        if read_identity::<HandleKey, _>(store, sealed, account_owner, "import_account_identity")?
            == account
        {
            if namespace.state == ImportNamespaceState::Revoked {
                return Err(ImportError::NamespaceRevoked);
            }
            if matched.replace(namespace).is_some() {
                return Err(ImportError::IdentityConflict);
            }
        }
    }
    Ok(matched)
}

fn read_identity<T: serde::de::DeserializeOwned, S: BlobStore>(
    store: &S,
    sealed: &SealedText,
    row_id: Uuid,
    field: &str,
) -> Result<T, ImportError> {
    if sealed.aad.as_deref() != Some(field_aad(&row_id, field).as_str()) {
        return Err(ImportError::IdentityUnavailable);
    }
    let raw = store
        .open(sealed)
        .map_err(|_| ImportError::IdentityUnavailable)?;
    serde_json::from_slice(&raw).map_err(|_| ImportError::IdentityUnavailable)
}

fn identity_bytes(value: &impl Serialize) -> Result<Vec<u8>, ImportError> {
    serde_json::to_vec(value).map_err(|_| ImportError::IdentityUnavailable)
}

fn body_equal<S: BlobStore>(
    store: &S,
    sealed: Option<&SealedText>,
    body: &str,
) -> Result<bool, ImportError> {
    match sealed {
        Some(sealed) => Ok(store
            .open(sealed)
            .map_err(|_| ImportError::IdentityUnavailable)?
            == body.as_bytes()),
        None => Ok(body.is_empty()),
    }
}

pub fn prepare_indexed<S>(
    store: &S,
    staged: &StagedImport,
    namespace: ImportNamespace,
    legacy_choice: LegacyImportChoice,
) -> Result<IndexedImportPlan, ImportError>
where
    S: ImportIndexStore + EventStore + GraphStore + BlobStore,
{
    if namespace.source != source_kind(staged.source) {
        return Err(ImportError::IdentityConflict);
    }
    if namespace.state != ImportNamespaceState::Active {
        return Err(ImportError::NamespaceRevoked);
    }
    let stored_namespace = match store.import_namespace(namespace.namespace_id) {
        Ok(stored) => {
            if stored != namespace {
                return Err(ImportError::SourceChanged);
            }
            Some(stored)
        }
        Err(StoreError::NotFound { .. }) => None,
        Err(_) => return Err(ImportError::IdentityUnavailable),
    };
    let legacy_events = store.import_legacy_events(namespace.source)?;
    if legacy_events > 0
        && (legacy_choice != LegacyImportChoice::ForwardOnly
            || namespace.coverage != ImportCoverage::ForwardOnly)
    {
        return Err(ImportError::LegacyChoiceRequired);
    }
    let owners = staged.participants.iter().filter(|p| p.is_owner).count();
    if owners == 0 {
        return Err(ImportError::NoOwner);
    }
    if owners > 1 {
        return Err(ImportError::AmbiguousOwner { count: owners });
    }
    let owner = staged.owner().ok_or(ImportError::NoOwner)?;
    if staged.source == ImportSource::TelegramDesktop {
        if telegram_namespace(store, staged)?
            .is_some_and(|existing| existing.namespace_id != namespace.namespace_id)
        {
            return Err(ImportError::IdentityConflict);
        }
        let account = handle_key(owner.handles.first().ok_or(ImportError::NoOwner)?);
        if let Some(sealed) = namespace.account_ref.as_ref() {
            let account_owner = namespace
                .owner_contact_id
                .ok_or(ImportError::IdentityUnavailable)?;
            if read_identity::<HandleKey, _>(
                store,
                sealed,
                account_owner,
                "import_account_identity",
            )? != account
            {
                return Err(ImportError::IdentityConflict);
            }
        }
    }
    let handles: BTreeSet<HandleKey> = staged
        .participants
        .iter()
        .flat_map(|p| p.handles.iter().map(handle_key))
        .collect();
    let mut participant_matches = BTreeMap::new();
    let mut after = None;
    loop {
        let batch = store.import_participant_batch(namespace.namespace_id, after, BATCH)?;
        if batch.is_empty() {
            break;
        }
        for identity in &batch {
            if identity.content_key_id != identity.identity_ref.content_key_id
                || store.import_key_destroyed(identity.content_key_id)?
            {
                return Err(ImportError::IdentityUnavailable);
            }
            if identity.namespace_id != namespace.namespace_id {
                return Err(ImportError::IdentityUnavailable);
            }
            let key: HandleKey = read_identity(
                store,
                &identity.identity_ref,
                identity.contact_id,
                "import_participant_identity",
            )?;
            if handles.contains(&key) && participant_matches.insert(key, identity.clone()).is_some()
            {
                return Err(ImportError::IdentityConflict);
            }
        }
        after = batch.last().map(|row| row.identity_id);
    }
    let existing = store.list_contacts()?;
    let mut contacts = Vec::with_capacity(staged.participants.len());
    for participant in &staged.participants {
        let hashes: Vec<_> = participant
            .handles
            .iter()
            .map(|h| h.value_hash(staged.source))
            .collect();
        // Recognizable forgotten identities cannot be bypassed by a new namespace.
        if existing.iter().any(|c| {
            c.forget_state == ForgetState::Forgotten
                && c.identifiers
                    .iter()
                    .flatten()
                    .any(|id| hashes.contains(&id.value_hash))
        }) {
            return Err(ImportError::NamespaceRevoked);
        }
        let ids: BTreeSet<_> = participant
            .handles
            .iter()
            .filter_map(|h| {
                participant_matches
                    .get(&handle_key(h))
                    .map(|i| i.contact_id)
            })
            .collect();
        if ids.len() > 1 {
            return Err(ImportError::IdentityConflict);
        }
        let mut found = ids.first().map(|id| store.get_contact(*id)).transpose()?;
        if found.is_none()
            && (staged.source == ImportSource::TelegramDesktop || participant.is_owner)
        {
            let candidates: Vec<_> = existing
                .iter()
                .filter(|c| {
                    c.identifiers
                        .iter()
                        .flatten()
                        .any(|id| hashes.contains(&id.value_hash))
                })
                .collect();
            if candidates.len() > 1 {
                return Err(ImportError::IdentityConflict);
            }
            found = candidates.first().map(|c| (*c).clone());
        }
        if let Some(contact) = &found {
            if contact.forget_state != ForgetState::Active {
                return Err(ImportError::NamespaceRevoked);
            }
            if (contact.contact_class == ContactClass::Owner) != participant.is_owner {
                return Err(ImportError::IdentityConflict);
            }
        } else if participant.is_owner
            && existing
                .iter()
                .any(|c| c.contact_class == ContactClass::Owner)
        {
            return Err(ImportError::IdentityConflict);
        }
        let id = found
            .as_ref()
            .map(|c| c.contact_id)
            .unwrap_or_else(Uuid::now_v7);
        let key = if let Some(contact) = &found {
            contact
                .display_label_ref
                .as_ref()
                .map(|s| s.content_key_id)
                .or(store
                    .import_contact_keys(contact.contact_id)?
                    .first()
                    .copied())
                .unwrap_or_else(Uuid::now_v7)
        } else {
            Uuid::now_v7()
        };
        if store.import_key_destroyed(key)? {
            return Err(ImportError::NamespaceRevoked);
        }
        contacts.push(PlannedContact {
            existing: found,
            id,
            key,
        });
    }
    if let Some(expected) = namespace.owner_contact_id {
        if !staged
            .participants
            .iter()
            .zip(&contacts)
            .any(|(p, c)| p.is_owner && c.id == expected)
        {
            return Err(ImportError::IdentityConflict);
        }
    }
    let by_handle: BTreeMap<_, _> = staged
        .participants
        .iter()
        .zip(&contacts)
        .flat_map(|(p, c)| p.handles.iter().map(move |h| (h.clone(), c.id)))
        .collect();
    let mut input: BTreeMap<MessageKey, usize> = BTreeMap::new();
    let mut selected = BTreeSet::new();
    let mut duplicates = 0;
    for (index, message) in staged.messages.iter().enumerate() {
        if !by_handle.contains_key(&message.sender) {
            return Err(ImportError::UnknownSender { index });
        }
        let key = MessageKey(message.conversation_id.clone(), message.external_id.clone());
        if let Some(&prior) = input.get(&key) {
            if staged.messages[prior] != *message {
                return Err(ImportError::IdentityConflict);
            }
            duplicates += 1;
        } else {
            input.insert(key, index);
            selected.insert(index);
        }
    }
    let mut message_matches = BTreeMap::new();
    let mut after = None;
    loop {
        let batch = store.import_message_batch(namespace.namespace_id, after, BATCH)?;
        if batch.is_empty() {
            break;
        }
        for identity in &batch {
            if identity.content_key_id != identity.identity_ref.content_key_id
                || store.import_key_destroyed(identity.content_key_id)?
            {
                return Err(ImportError::IdentityUnavailable);
            }
            if identity.namespace_id != namespace.namespace_id {
                return Err(ImportError::IdentityUnavailable);
            }
            let key: MessageKey = read_identity(
                store,
                &identity.identity_ref,
                identity.event_id,
                "import_message_identity",
            )?;
            if let Some(&index) = input.get(&key) {
                let message = &staged.messages[index];
                let event = store
                    .get_event(identity.event_id)
                    .map_err(|_| ImportError::IdentityUnavailable)?;
                let sender = by_handle
                    .get(&message.sender)
                    .ok_or(ImportError::UnknownSender { index })?;
                if identity.sender_contact_id != *sender
                    || identity.group != message.group
                    || identity.is_owner != (message.scope == SenderScope::Owner)
                    || identity.is_owner != (event.actor_subject == ActorSubject::Owner)
                    || event.ts != message.occurred_at
                    || event.source != staged.source.event_source()
                    || !body_equal(store, event.body_ref.as_ref(), message.body.as_str())?
                {
                    return Err(ImportError::IdentityConflict);
                }
                if message_matches.insert(index, identity.clone()).is_some() {
                    return Err(ImportError::IdentityConflict);
                }
                selected.remove(&index);
                duplicates += 1;
            }
        }
        after = batch.last().map(|row| row.identity_id);
    }
    Ok(IndexedImportPlan {
        namespace,
        stored_namespace,
        legacy_choice,
        legacy_events,
        contacts,
        participant_matches,
        message_matches,
        selected,
        duplicates,
    })
}

/// Reclassify inside the domain transaction, using ordinary row identities.
pub fn validate_indexed<S>(
    store: &S,
    staged: &StagedImport,
    plan: &IndexedImportPlan,
) -> Result<(), ImportError>
where
    S: ImportIndexStore + EventStore + GraphStore + BlobStore,
{
    let fresh = prepare_indexed(store, staged, plan.namespace.clone(), plan.legacy_choice)?;
    if fresh.stored_namespace != plan.stored_namespace
        || fresh.legacy_events != plan.legacy_events
        || fresh.participant_matches != plan.participant_matches
        || fresh.message_matches != plan.message_matches
        || fresh.selected != plan.selected
        || fresh.duplicates != plan.duplicates
        || fresh.contacts.iter().zip(&plan.contacts).any(|(a, b)| {
            a.existing != b.existing || (a.existing.is_some() && (a.id != b.id || a.key != b.key))
        })
    {
        return Err(ImportError::SourceChanged);
    }
    Ok(())
}

pub fn commit_indexed<S>(
    store: &mut S,
    staged: &StagedImport,
    plan: &IndexedImportPlan,
) -> Result<ImportReceipt, ImportError>
where
    S: ImportIndexStore + EventStore + GraphStore + ProfileStore + BlobStore,
{
    validate_indexed(store, staged, plan)?;
    let mut receipt = ImportReceipt {
        source: Some(staged.source),
        ..ImportReceipt::default()
    };
    let mut resolved = BTreeMap::new();
    let mut namespace = plan.namespace.clone();
    for (participant, contact) in staged.participants.iter().zip(&plan.contacts) {
        if contact.existing.is_some() {
            receipt.contacts_matched.push(contact.id);
        } else {
            receipt.contacts_created.push(contact.id);
        }
        let display_label_ref = commit::seal_label(
            store,
            participant,
            contact.id,
            contact.key,
            contact
                .existing
                .as_ref()
                .and_then(|c| c.display_label_ref.clone()),
        )?;
        if display_label_ref.is_none() && !store.has_content_key(contact.key) {
            commit::anchor_content_key(store, participant, contact.id, contact.key)?;
        }
        let mut identifiers = contact
            .existing
            .as_ref()
            .and_then(|c| c.identifiers.clone())
            .unwrap_or_default();
        for handle in &participant.handles {
            let id = ContactIdentifier {
                kind: handle.kind,
                value_hash: handle.value_hash(staged.source),
            };
            if !identifiers.contains(&id) {
                identifiers.push(id);
            }
        }
        store.put_contact(SoulContact {
            schema_version: SchemaVersion,
            contact_id: contact.id,
            contact_class: if participant.is_owner {
                ContactClass::Owner
            } else {
                ContactClass::ThirdParty
            },
            display_label_ref,
            identifiers: Some(identifiers),
            forget_state: ForgetState::Active,
        })?;
        let subject = if participant.is_owner {
            SealedSubject::Owner
        } else {
            SealedSubject::ThirdParty
        };
        for handle in &participant.handles {
            let key = handle_key(handle);
            if !plan.participant_matches.contains_key(&key) {
                let identity_id = Uuid::now_v7();
                let identity_ref = store.seal(SealRequest::new(
                    contact.key,
                    contact.id,
                    "import_participant_identity",
                    subject,
                    identity_bytes(&key)?,
                ))?;
                store.put_import_participant(&ImportParticipantIdentity {
                    identity_id,
                    namespace_id: namespace.namespace_id,
                    contact_id: contact.id,
                    content_key_id: contact.key,
                    identity_ref,
                })?;
            }
            resolved.insert(
                handle.clone(),
                ResolvedContact {
                    contact_id: contact.id,
                    content_key_id: contact.key,
                    is_owner: participant.is_owner,
                },
            );
        }
        if participant.is_owner {
            namespace.owner_contact_id = Some(contact.id);
            if namespace.account_ref.is_none() {
                let account = handle_key(participant.handles.first().ok_or(ImportError::NoOwner)?);
                namespace.account_ref = Some(store.seal(SealRequest::new(
                    contact.key,
                    contact.id,
                    "import_account_identity",
                    SealedSubject::Owner,
                    identity_bytes(&account)?,
                ))?);
            }
        }
    }
    store.put_import_namespace(&namespace)?;
    commit::commit_resolved(
        store,
        staged,
        receipt,
        resolved,
        Some(&plan.selected),
        |store, index, event_id, sender| {
            let message = &staged.messages[index];
            let identity_id = Uuid::now_v7();
            let key = MessageKey(message.conversation_id.clone(), message.external_id.clone());
            let identity_ref = store.seal(SealRequest::new(
                sender.content_key_id,
                event_id,
                "import_message_identity",
                if sender.is_owner {
                    SealedSubject::Owner
                } else {
                    SealedSubject::ThirdParty
                },
                identity_bytes(&key)?,
            ))?;
            store.put_import_message(&ImportMessageIdentity {
                identity_id,
                namespace_id: namespace.namespace_id,
                event_id,
                sender_contact_id: sender.contact_id,
                content_key_id: sender.content_key_id,
                identity_ref,
                group: message.group,
                is_owner: sender.is_owner,
            })?;
            Ok(())
        },
    )
}
