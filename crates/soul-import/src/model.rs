//! What both importers reduce their format to before anything is stored.
//!
//! The point of the intermediate form is that everything after it is written
//! once. Contact matching, sealing, event and evidence construction, the
//! injection scan and the audit entry all read [`StagedImport`], so adding a
//! third format later means writing a parser and nothing else.
//!
//! The one thing to notice is the type of a message body: [`UntrustedText`],
//! which has no `Display` and cannot be interpolated into a string by
//! accident. That is DECISIONS D25 expressed as a type rather than as a rule
//! someone has to remember — the body can be sealed and stored, and it cannot
//! become an instruction on the way.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use soul_policy::injection::UntrustedText;
use soul_schema::common::{Sha256Hex, Timestamp};
use soul_schema::contact::IdentifierKind;
use soul_schema::event::EventSource;
use soul_schema::soul_import_v1::SenderScope;

/// Which of the two v0.1 formats a staged import came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportSource {
    SoulImportV1,
    TelegramDesktop,
}

impl ImportSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            ImportSource::SoulImportV1 => "soul-import-v1",
            ImportSource::TelegramDesktop => "telegram-desktop",
        }
    }

    pub const fn event_source(self) -> EventSource {
        match self {
            ImportSource::SoulImportV1 => EventSource::ImportSoulImportV1,
            ImportSource::TelegramDesktop => EventSource::ImportTelegramDesktop,
        }
    }
}

/// How the source names one person.
///
/// The raw value never reaches the database; [`ParticipantHandle::value_hash`]
/// is what a contact row stores, which is why `contact.identifiers` is a list
/// of digests in the frozen contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantHandle {
    pub kind: IdentifierKind,
    pub value: String,
}

/// Ordered by the wire spelling of the kind and then the value, because
/// [`IdentifierKind`] is a contract enum with no ordering of its own and this
/// type is used as a map key.
impl Ord for ParticipantHandle {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.kind_str(), &self.value).cmp(&(other.kind_str(), &other.value))
    }
}

impl PartialOrd for ParticipantHandle {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ParticipantHandle {
    /// The spelling `contact.schema.json` uses for this identifier kind.
    pub const fn kind_str(&self) -> &'static str {
        match self.kind {
            IdentifierKind::Phone => "phone",
            IdentifierKind::Email => "email",
            IdentifierKind::Handle => "handle",
            IdentifierKind::PlatformUid => "platform_uid",
        }
    }

    pub fn platform_uid(value: impl Into<String>) -> ParticipantHandle {
        ParticipantHandle {
            kind: IdentifierKind::PlatformUid,
            value: value.into(),
        }
    }

    pub fn handle(value: impl Into<String>) -> ParticipantHandle {
        ParticipantHandle {
            kind: IdentifierKind::Handle,
            value: value.into(),
        }
    }

    /// Digest of this identifier, scoped to the format it came from.
    ///
    /// Scoping means user `42` in a Telegram export and user `42` in a
    /// `soul-import-v1` file are two people until something joins them. Two
    /// nodes for one person is a visible mistake the user can merge; one node
    /// for two people is a wrong graph that looks right.
    pub fn value_hash(&self, source: ImportSource) -> Sha256Hex {
        let kind = match self.kind {
            IdentifierKind::Phone => "phone",
            IdentifierKind::Email => "email",
            IdentifierKind::Handle => "handle",
            IdentifierKind::PlatformUid => "platform_uid",
        };
        let mut hasher = Sha256::new();
        hasher.update(b"soul.import.identifier.v1|");
        hasher.update(source.as_str().as_bytes());
        hasher.update(b"|");
        hasher.update(kind.as_bytes());
        hasher.update(b"|");
        hasher.update(self.value.as_bytes());
        Sha256Hex::new(hex::encode(hasher.finalize()))
    }
}

/// One person seen in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedParticipant {
    /// Every identifier the file used for them. More than one when a format
    /// names the same person two ways.
    pub handles: Vec<ParticipantHandle>,
    /// Their display name, as written in the export. External text.
    pub display_label: Option<UntrustedText>,
    pub is_owner: bool,
}

/// One message, reduced to what Soul keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedMessage {
    /// The source's own id, used to report defects. Not stored.
    pub external_id: String,
    /// RFC 3339 in UTC. Both parsers normalize before constructing this.
    pub occurred_at: Timestamp,
    pub sender: ParticipantHandle,
    /// The source's conversation id, hashed before it reaches the database.
    pub conversation_id: String,
    /// True when the conversation had more than two people in it.
    pub group: bool,
    pub scope: SenderScope,
    pub body: UntrustedText,
}

/// A file that parsed, ready to be committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedImport {
    pub source: ImportSource,
    pub exported_at: Option<Timestamp>,
    pub participants: Vec<StagedParticipant>,
    pub messages: Vec<StagedMessage>,
}

impl StagedImport {
    pub fn owner(&self) -> Option<&StagedParticipant> {
        self.participants
            .iter()
            .find(|participant| participant.is_owner)
    }

    pub fn peers(&self) -> impl Iterator<Item = &StagedParticipant> {
        self.participants
            .iter()
            .filter(|participant| !participant.is_owner)
    }

    /// Distinct conversations the file covers.
    pub fn conversation_count(&self) -> usize {
        self.messages
            .iter()
            .map(|message| message.conversation_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    }
}

/// Accumulates participants while a parser walks the file.
///
/// Keyed on the handle so that the same person mentioned in twenty messages
/// becomes one participant, and so a display name seen once is kept even if
/// later mentions omit it.
#[derive(Debug, Default)]
pub struct ParticipantIndex {
    by_handle: BTreeMap<ParticipantHandle, usize>,
    participants: Vec<StagedParticipant>,
}

impl ParticipantIndex {
    pub fn new() -> ParticipantIndex {
        ParticipantIndex::default()
    }

    /// Record one sighting. Ownership and a display name are sticky: once the
    /// file has said either, a later mention that omits it does not undo it.
    pub fn observe(
        &mut self,
        handle: ParticipantHandle,
        display_label: Option<&str>,
        is_owner: bool,
    ) {
        let index = match self.by_handle.get(&handle) {
            Some(index) => *index,
            None => {
                self.participants.push(StagedParticipant {
                    handles: vec![handle.clone()],
                    display_label: None,
                    is_owner: false,
                });
                let index = self.participants.len() - 1;
                self.by_handle.insert(handle, index);
                index
            }
        };
        let participant = &mut self.participants[index];
        participant.is_owner |= is_owner;
        if participant.display_label.is_none() {
            if let Some(label) = display_label.filter(|label| !label.trim().is_empty()) {
                participant.display_label = Some(UntrustedText::new(label));
            }
        }
    }

    /// Fold `extra` into the participant already known under `primary`.
    ///
    /// Telegram names the user twice — once in `personal_information`, once as
    /// a `from_id` on their own messages — and those have to be one contact.
    /// There the second name has usually not been seen yet, so folding it is
    /// one insertion.
    ///
    /// `soul-import-v1` reaches this the other way round: a file may write the
    /// user's own messages under two `sender_id`s, and both are read before
    /// anything knows they are the same person. Refusing to fold a handle that
    /// is already known would leave two participants marked as the user, two
    /// contacts of class `self` in the store, and every graph rebuild after
    /// that import failing on `AmbiguousOwner` — for good, because the rows are
    /// already written. So an `extra` that already has a participant of its own
    /// is merged into `primary` rather than ignored.
    pub fn alias(&mut self, primary: &ParticipantHandle, extra: ParticipantHandle) {
        let Some(index) = self.by_handle.get(primary).copied() else {
            return;
        };
        match self.by_handle.get(&extra).copied() {
            None => {
                self.participants[index].handles.push(extra.clone());
                self.by_handle.insert(extra, index);
            }
            // Already the same person; saying so twice changes nothing.
            Some(duplicate) if duplicate == index => {}
            Some(duplicate) => self.fold(index, duplicate),
        }
    }

    /// Move everything held at `duplicate` onto `keep` and drop the empty row.
    ///
    /// Removing from the middle of the vector shifts every later participant
    /// down one, so the index map is rewritten in the same pass: nothing may
    /// keep pointing at a slot that has moved.
    fn fold(&mut self, keep: usize, duplicate: usize) {
        let folded = self.participants.remove(duplicate);
        let keep = match keep > duplicate {
            true => keep - 1,
            false => keep,
        };
        let target = &mut self.participants[keep];
        for handle in folded.handles {
            if !target.handles.contains(&handle) {
                target.handles.push(handle);
            }
        }
        target.is_owner |= folded.is_owner;
        if target.display_label.is_none() {
            target.display_label = folded.display_label;
        }
        for index in self.by_handle.values_mut() {
            match (*index).cmp(&duplicate) {
                std::cmp::Ordering::Equal => *index = keep,
                std::cmp::Ordering::Greater => *index -= 1,
                std::cmp::Ordering::Less => {}
            }
        }
    }

    pub fn contains(&self, handle: &ParticipantHandle) -> bool {
        self.by_handle.contains_key(handle)
    }

    pub fn is_owner(&self, handle: &ParticipantHandle) -> bool {
        self.by_handle
            .get(handle)
            .is_some_and(|index| self.participants[*index].is_owner)
    }

    pub fn into_participants(self) -> Vec<StagedParticipant> {
        self.participants
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Telegram's path: the second name for the user has not been seen before.
    #[test]
    fn a_name_nobody_has_used_yet_joins_the_participant_it_names() {
        let mut index = ParticipantIndex::new();
        let owner = ParticipantHandle::platform_uid("user42");
        index.observe(owner.clone(), Some("Lin Wei"), true);
        index.alias(&owner, ParticipantHandle::handle("@linwei"));

        let participants = index.into_participants();
        assert_eq!(participants.len(), 1);
        assert_eq!(
            participants[0].handles,
            vec![owner, ParticipantHandle::handle("@linwei")],
        );
        assert!(participants[0].is_owner);
        assert_eq!(
            participants[0]
                .display_label
                .as_ref()
                .map(|label| label.as_str()),
            Some("Lin Wei"),
        );
    }

    /// The v1 path: both names have already been observed as people of their
    /// own, and the fold has to leave one participant, not two.
    #[test]
    fn two_participants_that_turn_out_to_be_one_person_become_one() {
        let mut index = ParticipantIndex::new();
        let first = ParticipantHandle::platform_uid("u-self");
        let peer = ParticipantHandle::platform_uid("u-a");
        let second = ParticipantHandle::platform_uid("u-self-old");
        index.observe(first.clone(), None, true);
        index.observe(peer.clone(), Some("A"), false);
        index.observe(second.clone(), Some("me"), true);
        index.alias(&first, second.clone());

        assert!(index.is_owner(&first));
        assert!(index.is_owner(&second));
        assert!(!index.is_owner(&peer));

        let participants = index.into_participants();
        assert_eq!(participants.len(), 2);
        assert_eq!(
            participants
                .iter()
                .filter(|participant| participant.is_owner)
                .count(),
            1,
            "one person, one participant",
        );
        let owner = participants
            .iter()
            .find(|participant| participant.is_owner)
            .expect("the owner survives the fold");
        assert_eq!(owner.handles, vec![first, second]);
        assert_eq!(
            owner.display_label.as_ref().map(|label| label.as_str()),
            Some("me"),
            "a name the folded row carried is not thrown away",
        );
        // The peer's row moved down one when the duplicate was removed, and
        // the index has to have followed it.
        let peer_row = participants
            .iter()
            .find(|participant| participant.handles.contains(&peer))
            .expect("the peer is still there");
        assert_eq!(
            peer_row.display_label.as_ref().map(|label| label.as_str()),
            Some("A"),
        );
    }

    /// Folding the same pair twice, or a handle into itself, is a no-op.
    #[test]
    fn folding_a_participant_into_itself_changes_nothing() {
        let mut index = ParticipantIndex::new();
        let owner = ParticipantHandle::platform_uid("u-self");
        let extra = ParticipantHandle::platform_uid("u-self-2");
        index.observe(owner.clone(), None, true);
        index.observe(extra.clone(), None, true);
        index.alias(&owner, owner.clone());
        index.alias(&owner, extra.clone());
        index.alias(&owner, extra.clone());
        index.alias(&extra, owner.clone());

        let participants = index.into_participants();
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].handles, vec![owner, extra]);
    }

    /// A handle nothing has observed cannot be a fold target: there is no
    /// participant to fold into, and inventing one would invent a person.
    #[test]
    fn an_unknown_primary_is_not_a_place_to_fold_into() {
        let mut index = ParticipantIndex::new();
        let stranger = ParticipantHandle::platform_uid("u-never-seen");
        index.alias(&stranger, ParticipantHandle::platform_uid("u-a"));
        assert!(index.into_participants().is_empty());
    }
}
