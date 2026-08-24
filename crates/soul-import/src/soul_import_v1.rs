//! The `soul-import-v1` JSONL reader.
//!
//! One JSON object per line: a header, then messages. Each line is validated
//! against `docs/schemas/soul-import-v1.schema.json` on its own, which is what
//! makes a line number a useful thing to report.
//!
//! The frozen contract puts a `oneOf` at the top level, so a schema failure
//! against a message line is reported by the validator as "matches neither
//! branch" — true, and useless to somebody trying to fix their file. The line
//! is therefore also deserialized into [`ImportLine`], whose failure names the
//! field. Both readings run; the schema decides whether the line is accepted,
//! the deserializer supplies the sentence.

use serde_json::Value;

use soul_policy::injection::UntrustedText;
use soul_schema::common::Timestamp;
use soul_schema::soul_import_v1::{ImportLine, ImportMessage, SenderScope};
use soul_schema::validate::SchemaId;
use soul_schema::SchemaSet;

use crate::defect::{schema_defects, Defect, ImportFailure, Locator};
use crate::model::{
    ImportSource, ParticipantHandle, ParticipantIndex, StagedImport, StagedMessage,
};
use crate::redact::ContentGuard;

/// Fields of this format that hold what someone wrote.
const CONTENT_KEYS: &[&str] = &["text"];

const SOURCE: ImportSource = ImportSource::SoulImportV1;

/// Parse a whole file. Either every line is good or nothing is imported.
pub fn parse(text: &str) -> Result<StagedImport, ImportFailure> {
    let schemas = SchemaSet::load().expect("the frozen contracts compile");
    parse_with(&schemas, text)
}

/// As [`parse`], reusing a validator set the caller already built.
pub fn parse_with(schemas: &SchemaSet, text: &str) -> Result<StagedImport, ImportFailure> {
    let validator = schemas.validator(SchemaId::SoulImportV1);
    let mut defects = Vec::new();
    let mut examined = 0usize;
    let mut exported_at = None;
    let mut participants = ParticipantIndex::new();
    let mut raw_messages: Vec<(usize, ImportMessage)> = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        if line.trim().is_empty() {
            continue;
        }
        examined += 1;
        let locator = Locator::Line(number);

        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(error) => {
                // A parse error names a byte offset, not the bytes, so it is
                // safe to pass through — but it goes through the guard anyway,
                // because "safe today" is how echoes get in.
                defects.push(Defect::at(
                    locator,
                    ContentGuard::new().guard(format!(
                        "is not valid JSON (column {})",
                        error.column().max(1)
                    )),
                ));
                continue;
            }
        };

        let guard = ContentGuard::from_document(&value, CONTENT_KEYS);
        let schema_verdict = schema_defects(validator, &value, &locator, &guard);
        let typed = serde_json::from_value::<ImportLine>(value.clone());

        if !schema_verdict.is_empty() {
            // The deserializer's complaint is the specific one; the schema's is
            // the authority. Prefer the first and fall back to the second.
            match &typed {
                Err(error) => defects.push(Defect::at(
                    locator.clone(),
                    guard.guard(error.to_string()),
                )),
                Ok(_) => {}
            }
            defects.extend(schema_verdict);
            continue;
        }

        match typed {
            Ok(ImportLine::Header(header)) => {
                if number != 1 && exported_at.is_none() {
                    defects.push(Defect::at(
                        locator,
                        "the header has to be the first line of the file".to_owned(),
                    ));
                    continue;
                }
                exported_at = Some(header.exported_at);
            }
            Ok(ImportLine::Message(message)) => raw_messages.push((number, message)),
            Err(error) => {
                // The schema accepted it and the model did not, which means the
                // two have drifted apart. Worth reporting loudly.
                defects.push(Defect::at(locator, guard.guard(error.to_string())));
            }
        }
    }

    if exported_at.is_none() && defects.is_empty() {
        defects.push(Defect::at(
            Locator::Line(1),
            "the file has no soul-import-v1 header line".to_owned(),
        ));
    }

    if !defects.is_empty() {
        return Err(ImportFailure::new(SOURCE, defects, examined));
    }

    // Every sender that ever wrote with `sender_scope: self` is the user. A
    // file may use more than one identifier for them, and all of them belong
    // to the same contact.
    let mut owner_handle: Option<ParticipantHandle> = None;
    for (_, message) in &raw_messages {
        let handle = ParticipantHandle::platform_uid(message.sender_id.clone());
        let is_owner = message.sender_scope == SenderScope::Owner;
        participants.observe(handle.clone(), None, is_owner);
        if is_owner {
            match &owner_handle {
                None => owner_handle = Some(handle),
                Some(primary) if primary != &handle => {
                    participants.alias(primary, handle);
                }
                Some(_) => {}
            }
        }
    }

    let group_conversations = crowded_conversations(&raw_messages, &participants);
    let messages = raw_messages
        .into_iter()
        .map(|(_, message)| StagedMessage {
            external_id: message.id,
            occurred_at: message.occurred_at,
            sender: ParticipantHandle::platform_uid(message.sender_id),
            group: group_conversations.contains(&message.conversation_id),
            conversation_id: message.conversation_id,
            scope: message.sender_scope,
            body: UntrustedText::new(message.text),
        })
        .collect();

    Ok(StagedImport {
        source: SOURCE,
        exported_at: exported_at.map(|value| Timestamp::new(value.as_str().to_owned())),
        participants: participants.into_participants(),
        messages,
    })
}

/// Conversations with more than one other person in them.
///
/// The distinction matters to the graph: a message the user sent in a group is
/// evidence of contact with everyone who was there, and a tie only ever seen
/// in a group is a weaker thing than a tie built one to one.
fn crowded_conversations(
    messages: &[(usize, ImportMessage)],
    participants: &ParticipantIndex,
) -> std::collections::BTreeSet<String> {
    let mut peers_by_conversation: std::collections::BTreeMap<
        String,
        std::collections::BTreeSet<String>,
    > = std::collections::BTreeMap::new();
    for (_, message) in messages {
        if participants.is_owner(&ParticipantHandle::platform_uid(message.sender_id.clone())) {
            continue;
        }
        peers_by_conversation
            .entry(message.conversation_id.clone())
            .or_default()
            .insert(message.sender_id.clone());
    }
    peers_by_conversation
        .into_iter()
        .filter(|(_, peers)| peers.len() > 1)
        .map(|(conversation, _)| conversation)
        .collect()
}
