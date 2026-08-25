//! Model for `docs/schemas/soul-import-v1.schema.json`.
//!
//! JSONL wire format for manual import. This is a transport contract, not a
//! storage format: nothing in Soul persists these lines as-is.

use serde::de::{Error as DeError, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::common::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SenderScope {
    #[serde(rename = "self")]
    Owner,
    ThirdParty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImportLine {
    Header(ImportHeader),
    Message(ImportMessage),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportHeader {
    pub format: ImportFormatTag,
    pub version: ImportVersionTag,
    pub exported_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportMessage {
    pub id: String,
    pub occurred_at: Timestamp,
    pub sender_scope: SenderScope,
    pub conversation_id: String,
    pub sender_id: String,
    /// Untrusted external text. It is data, never an instruction.
    pub text: String,
}

pub const IMPORT_FORMAT: &str = "soul-import-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportFormatTag;

impl Serialize for ImportFormatTag {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(IMPORT_FORMAT)
    }
}

impl<'de> Deserialize<'de> for ImportFormatTag {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        if raw == IMPORT_FORMAT {
            Ok(ImportFormatTag)
        } else {
            Err(D::Error::invalid_value(
                Unexpected::Str(&raw),
                &IMPORT_FORMAT,
            ))
        }
    }
}

/// The schema pins `version` to the JSON number `1`, not the string `"1"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportVersionTag;

impl Serialize for ImportVersionTag {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(1)
    }
}

impl<'de> Deserialize<'de> for ImportVersionTag {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = u64::deserialize(d)?;
        if raw == 1 {
            Ok(ImportVersionTag)
        } else {
            Err(D::Error::invalid_value(Unexpected::Unsigned(raw), &"1"))
        }
    }
}
