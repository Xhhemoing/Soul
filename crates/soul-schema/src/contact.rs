//! Model for `docs/schemas/contact.schema.json`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::common::{SchemaVersion, SealedText, Sha256Hex};
use crate::memory::ForgetState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactClass {
    #[serde(rename = "self")]
    Owner,
    ThirdParty,
    Group,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentifierKind {
    Phone,
    Email,
    Handle,
    PlatformUid,
}

/// Identifiers are stored hashed; the raw handle never lands in this row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactIdentifier {
    pub kind: IdentifierKind,
    pub value_hash: Sha256Hex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulContact {
    pub schema_version: SchemaVersion,
    pub contact_id: Uuid,
    pub contact_class: ContactClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_label_ref: Option<SealedText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifiers: Option<Vec<ContactIdentifier>>,
    pub forget_state: ForgetState,
}
