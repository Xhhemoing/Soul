//! The `soul-import-v1` JSONL reader.
//!
//! One JSON object per line: a header, then messages. Each line is validated
//! against `docs/schemas/soul-import-v1.schema.json` on its own, which is what
//! makes a line number a useful thing to report.
//!
//! The frozen contract puts a `oneOf` at the top level, so a schema failure
//! against a message line is reported by the validator as "matches neither
//! branch" — true, and useless to somebody trying to fix their file. So this
//! module states the same requirements a second time, in [`shape_defects`],
//! and that is where the sentence comes from. The schema stays the authority
//! on whether a line is accepted; the restatement only has to explain a line
//! the schema has already refused.
//!
//! Restating a contract usually means two versions of it to keep in step. The
//! alternative here was to forward the deserializer's message, and that turns
//! out to be worse: `serde`'s text quotes the value it choked on, which is the
//! one thing a refusal must not do.

use serde_json::{Map, Value};

use soul_policy::injection::UntrustedText;
use soul_schema::common::Timestamp;
use soul_schema::soul_import_v1::{ImportLine, ImportMessage, SenderScope};
use soul_schema::validate::SchemaId;
use soul_schema::SchemaSet;

use crate::defect::{schema_defects, Defect, ImportFailure, Locator};
use crate::instant;
use crate::model::{
    ImportSource, ParticipantHandle, ParticipantIndex, StagedImport, StagedMessage,
};
use crate::redact::{summarize_names, ContentGuard};

/// Fields of this format that hold what someone wrote.
const CONTENT_KEYS: &[&str] = &["text"];

/// The two line shapes, in the order the contract lists their properties.
const HEADER_FIELDS: &[&str] = &["type", "format", "version", "exported_at"];
const MESSAGE_FIELDS: &[&str] = &[
    "type",
    "id",
    "occurred_at",
    "sender_scope",
    "conversation_id",
    "sender_id",
    "text",
];

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
        let mut line_defects = match validator.is_valid(&value) {
            true => Vec::new(),
            // Say which field is wrong if this module can tell, and fall back
            // to the validator's own verdict if it cannot.
            false => match shape_defects(&value, &locator, &guard) {
                found if found.is_empty() => schema_defects(validator, &value, &locator, &guard),
                found => found,
            },
        };

        if !line_defects.is_empty() {
            // One place where every reason meets the guard, so a message added
            // later cannot skip it.
            for defect in &mut line_defects {
                defect.reason = guard.guard(std::mem::take(&mut defect.reason));
            }
            defects.append(&mut line_defects);
            continue;
        }

        match serde_json::from_value::<ImportLine>(value) {
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
            Err(_) => {
                // The schema accepted the line and the model did not, which
                // means the two have drifted apart. The deserializer's own
                // sentence is not repeated, because it quotes the value.
                defects.push(Defect::at(
                    locator,
                    "satisfies the contract but not this build's reader, which means the two \
                     have drifted apart"
                        .to_owned(),
                ));
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

/// What the contract asks of one line, said in the contract's own words.
///
/// Every sentence below is built from a property name or an enumerated value
/// out of `soul-import-v1.schema.json`. The only thing that comes from the
/// file is an unexpected property name, which is why that one goes through the
/// guard as a fragment. Returns empty when this module has nothing specific to
/// add.
fn shape_defects(value: &Value, locator: &Locator, guard: &ContentGuard) -> Vec<Defect> {
    let Some(object) = value.as_object() else {
        return vec![Defect::at(locator.clone(), "must be a JSON object")];
    };
    let expected: &[&str] = match object.get("type").and_then(Value::as_str) {
        Some("header") => HEADER_FIELDS,
        Some("message") => MESSAGE_FIELDS,
        _ => {
            return vec![Defect::field(
                locator.clone(),
                "type",
                "must be `header` or `message`",
            )]
        }
    };

    let mut faults: Vec<(&str, &'static str)> = Vec::new();
    for field in expected {
        if !object.contains_key(*field) {
            faults.push((field, "is required and this line has no value for it"));
        }
    }
    match expected == HEADER_FIELDS {
        true => check_header(object, &mut faults),
        false => check_message(object, &mut faults),
    }

    let mut defects: Vec<Defect> = faults
        .into_iter()
        .map(|(field, reason)| Defect::field(locator.clone(), field, reason))
        .collect();

    let undefined: Vec<String> = object
        .keys()
        .filter(|field| !expected.contains(&field.as_str()))
        .cloned()
        .collect();
    if !undefined.is_empty() {
        defects.push(Defect::at(
            locator.clone(),
            format!(
                "carries {}, which the contract does not define",
                summarize_names(&undefined, guard),
            ),
        ));
    }
    defects
}

/// A field whose value is present but wrong, and why. Both halves are fixed
/// vocabulary, which is what makes them safe to show.
type Fault<'a> = (&'a str, &'static str);

fn check_header<'a>(object: &Map<String, Value>, faults: &mut Vec<Fault<'a>>) {
    if object.get("format").is_some_and(|f| f != "soul-import-v1") {
        faults.push(("format", "must equal `soul-import-v1`"));
    }
    if object.get("version").is_some_and(|v| v != 1) {
        faults.push(("version", "must equal 1"));
    }
    check_instant(object, "exported_at", faults);
}

fn check_message<'a>(object: &Map<String, Value>, faults: &mut Vec<Fault<'a>>) {
    if object
        .get("id")
        .is_some_and(|id| id.as_str().is_none_or(str::is_empty))
    {
        faults.push(("id", "must be a non-empty string"));
    }
    check_instant(object, "occurred_at", faults);
    if object
        .get("sender_scope")
        .is_some_and(|scope| !matches!(scope.as_str(), Some("self" | "third_party")))
    {
        faults.push(("sender_scope", "must be `self` or `third_party`"));
    }
    for field in ["conversation_id", "sender_id", "text"] {
        if object.get(field).is_some_and(|value| !value.is_string()) {
            faults.push((field, "must be a JSON string"));
        }
    }
}

fn check_instant<'a>(object: &Map<String, Value>, field: &'a str, faults: &mut Vec<Fault<'a>>) {
    if object
        .get(field)
        .is_some_and(|value| !value.as_str().is_some_and(instant::is_date_time))
    {
        faults.push((
            field,
            "must be an RFC 3339 instant, such as `2026-08-24T08:00:00Z`",
        ));
    }
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
