//! Telegram Desktop 4.x, *Export chat history → Machine-readable JSON*.
//!
//! The file is `result.json`. There is no schema for it — it is somebody
//! else's format — so this adapter states what it needs and refuses the file
//! when it is not there, naming the chat and the message by id. Silently
//! dropping a row would be worse than failing: the graph would come out
//! thinner than the export and nothing would say so.
//!
//! Three decisions worth knowing about.
//!
//! **Time comes from `date_unixtime`.** The neighbouring `date` field is local
//! wall-clock with no offset, so turning it into an instant means guessing a
//! time zone. Every Desktop 4.x export writes `date_unixtime`; a file without
//! it is refused rather than imported an unknown number of hours out.
//!
//! **`contacts.list` is not imported.** Telegram's phone book entries carry a
//! name and a phone number but no user id, and chat messages carry a user id
//! and no phone number. There is no join key, so merging them would produce
//! either duplicate people or wrong ones. Only people who appear in a chat
//! become contacts.
//!
//! **A chat title is never put in an error message.** `name` on a personal
//! chat is the other person's name; the locator uses ids.

use serde_json::Value;

use soul_policy::clock::rfc3339_utc;
use soul_policy::injection::UntrustedText;
use soul_schema::common::Timestamp;
use soul_schema::soul_import_v1::SenderScope;

use crate::defect::{Defect, ImportFailure, Locator};
use crate::instant::is_civil_datetime;
use crate::model::{
    ImportSource, ParticipantHandle, ParticipantIndex, StagedImport, StagedMessage,
};
use crate::redact::ContentGuard;

/// Fields of this format that hold prose, names or account handles.
const CONTENT_KEYS: &[&str] = &[
    "text",
    "text_entities",
    "from",
    "actor",
    "name",
    "title",
    "first_name",
    "last_name",
    "bio",
    "username",
    "phone_number",
];

const SOURCE: ImportSource = ImportSource::TelegramDesktop;

/// Parse a `result.json` document.
pub fn parse(document: &Value) -> Result<StagedImport, ImportFailure> {
    let guard = ContentGuard::from_document(document, CONTENT_KEYS);
    let mut defects = Vec::new();
    let mut examined = 0usize;
    let mut participants = ParticipantIndex::new();
    let mut messages: Vec<StagedMessage> = Vec::new();

    let owner = match owner_handle(document, &mut defects) {
        Some(owner) => owner,
        None => return Err(ImportFailure::new(SOURCE, defects, examined)),
    };
    participants.observe(
        owner.clone(),
        display_name(document.get("personal_information")).as_deref(),
        true,
    );
    if let Some(username) = document
        .pointer("/personal_information/username")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
    {
        participants.alias(&owner, ParticipantHandle::handle(format!("@{username}")));
    }

    let Some(chats) = document.pointer("/chats/list").and_then(Value::as_array) else {
        defects.push(Defect::field(
            Locator::Path("chats".into()),
            "list",
            "must be an array; this does not look like a Telegram Desktop result.json"
                .to_owned(),
        ));
        return Err(ImportFailure::new(SOURCE, defects, examined));
    };

    for (chat_index, chat) in chats.iter().enumerate() {
        let chat_id = chat.get("id").and_then(Value::as_i64);
        let chat_locator = Locator::Path(match chat_id {
            Some(id) => format!("chats.list[{chat_index}](chat_id={id})"),
            None => format!("chats.list[{chat_index}]"),
        });

        let Some(chat_id) = chat_id else {
            defects.push(Defect::field(
                chat_locator,
                "id",
                "is required and this chat has no numeric id".to_owned(),
            ));
            continue;
        };
        let Some(chat_messages) = chat.get("messages").and_then(Value::as_array) else {
            defects.push(Defect::field(
                chat_locator,
                "messages",
                "is required and this chat has no message array".to_owned(),
            ));
            continue;
        };

        let conversation_id = chat_id.to_string();
        let group = chat.get("type").and_then(Value::as_str) != Some("personal_chat");

        for (message_index, message) in chat_messages.iter().enumerate() {
            examined += 1;
            let message_id = message.get("id").and_then(Value::as_i64);
            let locator = Locator::Path(match message_id {
                Some(id) => format!(
                    "chats.list[{chat_index}](chat_id={chat_id}).messages[{message_index}](message_id={id})"
                ),
                None => format!(
                    "chats.list[{chat_index}](chat_id={chat_id}).messages[{message_index}]"
                ),
            });

            if message_id.is_none() {
                defects.push(Defect::field(
                    locator.clone(),
                    "id",
                    "is required and this message has no numeric id".to_owned(),
                ));
            }

            let occurred_at = read_instant(message, &locator, &mut defects);

            match message.get("type").and_then(Value::as_str) {
                // Joins, calls and title changes. They are part of the history
                // but carry no body, and v0.1 has no event kind for them.
                Some("service") => continue,
                Some("message") => {}
                Some(_) | None => {
                    defects.push(Defect::field(
                        locator.clone(),
                        "type",
                        "must be `message` or `service`; nothing else is supported".to_owned(),
                    ));
                    continue;
                }
            }

            let Some(from_id) = message.get("from_id").and_then(Value::as_str) else {
                defects.push(Defect::field(
                    locator,
                    "from_id",
                    "is required; without it there is no way to tell who wrote this message"
                        .to_owned(),
                ));
                continue;
            };

            let (Some(occurred_at), Some(message_id)) = (occurred_at, message_id) else {
                continue;
            };

            let sender = ParticipantHandle::platform_uid(from_id);
            let is_owner = sender == owner;
            participants.observe(
                sender.clone(),
                message.get("from").and_then(Value::as_str),
                is_owner,
            );

            messages.push(StagedMessage {
                external_id: format!("{chat_id}:{message_id}"),
                occurred_at,
                sender,
                conversation_id: conversation_id.clone(),
                group,
                scope: match is_owner {
                    true => SenderScope::Owner,
                    false => SenderScope::ThirdParty,
                },
                body: UntrustedText::new(flatten_text(message.get("text"))),
            });
        }
    }

    if !defects.is_empty() {
        // The guard runs over every reason on the way out rather than at each
        // construction site, so a new message added later cannot skip it.
        for defect in &mut defects {
            defect.reason = guard.guard(std::mem::take(&mut defect.reason));
        }
        return Err(ImportFailure::new(SOURCE, defects, examined));
    }

    Ok(StagedImport {
        source: SOURCE,
        exported_at: None,
        participants: participants.into_participants(),
        messages,
    })
}

/// The user's own identifier, from `personal_information.user_id`.
fn owner_handle(document: &Value, defects: &mut Vec<Defect>) -> Option<ParticipantHandle> {
    let locator = Locator::Path("personal_information".into());
    match document
        .pointer("/personal_information/user_id")
        .and_then(Value::as_i64)
    {
        Some(user_id) => Some(ParticipantHandle::platform_uid(format!("user{user_id}"))),
        None => {
            defects.push(Defect::field(
                locator,
                "user_id",
                "is required; without it your own messages cannot be told apart from \
                 everyone else's"
                    .to_owned(),
            ));
            None
        }
    }
}

fn display_name(personal: Option<&Value>) -> Option<String> {
    let personal = personal?;
    let first = personal
        .get("first_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let last = personal
        .get("last_name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let joined = format!("{first} {last}").trim().to_owned();
    match joined.is_empty() {
        true => None,
        false => Some(joined),
    }
}

/// `date_unixtime` as an RFC 3339 UTC instant, reporting what is missing.
///
/// `date` is checked too when it is there. It is not used to build the
/// instant, but a `date` that is not a real civil timestamp says the file is
/// damaged in a way the user should hear about.
fn read_instant(message: &Value, locator: &Locator, defects: &mut Vec<Defect>) -> Option<Timestamp> {
    if let Some(date) = message.get("date").and_then(Value::as_str) {
        if !is_civil_datetime(date) {
            defects.push(Defect::field(
                locator.clone(),
                "date",
                "is not a `YYYY-MM-DDTHH:MM:SS` instant".to_owned(),
            ));
        }
    } else if message.get("date").is_none() {
        defects.push(Defect::field(
            locator.clone(),
            "date",
            "is required and this message has no time".to_owned(),
        ));
    }

    let raw = message.get("date_unixtime");
    let seconds = match raw {
        Some(Value::String(text)) => text.parse::<i64>().ok(),
        Some(Value::Number(number)) => number.as_i64(),
        _ => None,
    };
    match seconds {
        Some(seconds) => Some(Timestamp::new(rfc3339_utc(seconds))),
        None => {
            defects.push(Defect::field(
                locator.clone(),
                "date_unixtime",
                "is required: the neighbouring `date` carries no time zone, so on its own \
             it can only be guessed at"
                .to_owned(),
            ));
            None
        }
    }
}

/// Telegram writes `text` either as a string or as a list of runs, where a run
/// is a bare string or `{"type": …, "text": …}`. Both flatten to the same
/// thing, which is what the user actually saw on screen.
fn flatten_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(runs)) => runs
            .iter()
            .filter_map(|run| match run {
                Value::String(text) => Some(text.as_str()),
                Value::Object(_) => run.get("text").and_then(Value::as_str),
                _ => None,
            })
            .collect(),
        _ => String::new(),
    }
}
