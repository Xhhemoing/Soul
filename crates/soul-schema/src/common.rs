//! The shared vocabulary defined by `docs/schemas/_defs.schema.json`.

use serde::de::{Error as DeError, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// Literal value of every `schema_version` field in the frozen contracts.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Serializes to the frozen `"1.0.0"` literal and refuses to deserialize
/// anything else, so a model instance can never drift from the `const` in the
/// JSON Schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SchemaVersion;

impl Serialize for SchemaVersion {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(SCHEMA_VERSION)
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        if raw == SCHEMA_VERSION {
            Ok(SchemaVersion)
        } else {
            Err(D::Error::invalid_value(
                Unexpected::Str(&raw),
                &SCHEMA_VERSION,
            ))
        }
    }
}

/// RFC 3339 instant, matching `_defs#/$defs/timestamp`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub String);

impl Timestamp {
    pub fn new(value: impl Into<String>) -> Self {
        Timestamp(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Lowercase hex SHA-256 digest, matching `_defs#/$defs/sha256`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Sha256Hex(pub String);

impl Sha256Hex {
    pub fn new(value: impl Into<String>) -> Self {
        Sha256Hex(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `_defs#/$defs/evidenceBand`. Deliberately not numeric: PRODUCT_LOCK forbids
/// scales and numeric ratings on trait axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceBand {
    Weak,
    Moderate,
    Strong,
    None,
}

/// The subset of [`EvidenceBand`] usable where evidence is mandatory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportedBand {
    Weak,
    Moderate,
    Strong,
}

/// Who the data is about. Four-valued form used by `privacy.subject`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subject {
    #[serde(rename = "self")]
    Owner,
    ThirdParty,
    Mixed,
    System,
}

/// Subject of a sealed blob. `system` is not meaningful for stored prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SealedSubject {
    #[serde(rename = "self")]
    Owner,
    ThirdParty,
    Mixed,
}

/// Who caused an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorSubject {
    #[serde(rename = "self")]
    Owner,
    ThirdParty,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Derivation {
    Raw,
    Derived,
    Aggregate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    SoulProfile,
    AssistantTask,
    Graph,
    Memory,
    Research,
    Audit,
    Diagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetentionMode {
    UntilForgotten,
    Days,
    Session,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Retention {
    pub mode: RetentionMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days: Option<u32>,
}

impl Retention {
    pub fn until_forgotten() -> Self {
        Retention {
            mode: RetentionMode::UntilForgotten,
            days: None,
        }
    }
}

/// E0 is a closed set with exactly one legal value in v0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct E0Deny;

impl Serialize for E0Deny {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("deny")
    }
}

impl<'de> Deserialize<'de> for E0Deny {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        if raw == "deny" {
            Ok(E0Deny)
        } else {
            Err(D::Error::invalid_value(Unexpected::Str(&raw), &"deny"))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum E1Disposition {
    Deny,
    Placeholder,
    Allow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchDisposition {
    Deny,
    Hash,
    Bucket,
    Allow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressPolicy {
    pub e0: E0Deny,
    pub e1: E1Disposition,
    pub research_export: ResearchDisposition,
}

impl Default for EgressPolicy {
    fn default() -> Self {
        EgressPolicy {
            e0: E0Deny,
            e1: E1Disposition::Deny,
            research_export: ResearchDisposition::Deny,
        }
    }
}

/// `_defs#/$defs/privacy`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Privacy {
    pub subject: Subject,
    pub derivation: Derivation,
    pub purposes: Vec<Purpose>,
    pub retention: Retention,
    pub egress: EgressPolicy,
}

impl Privacy {
    /// Most restrictive shape that still validates: own raw data, kept until
    /// forgotten, nothing leaves the machine.
    pub fn local_only(subject: Subject, purposes: Vec<Purpose>) -> Self {
        Privacy {
            subject,
            derivation: Derivation::Raw,
            purposes,
            retention: Retention::until_forgotten(),
            egress: EgressPolicy::default(),
        }
    }
}

/// `_defs#/$defs/sealedText`: a pointer to AEAD-sealed prose, never the prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedText {
    pub content_key_id: Uuid,
    pub blob_id: Uuid,
    pub alg: SealAlg,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aad: Option<String>,
    pub subject: SealedSubject,
    pub char_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

/// The only AEAD construction the contracts admit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SealAlg;

pub const SEAL_ALG: &str = "xchacha20poly1305-ietf";

impl Serialize for SealAlg {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(SEAL_ALG)
    }
}

impl<'de> Deserialize<'de> for SealAlg {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        if raw == SEAL_ALG {
            Ok(SealAlg)
        } else {
            Err(D::Error::invalid_value(Unexpected::Str(&raw), &SEAL_ALG))
        }
    }
}

/// Canonical additional-authenticated-data string for a sealed field.
///
/// Binding the row identity and the column name into the AEAD tag means a
/// sealed blob cannot be replayed into a different row or a different column.
pub fn field_aad(row_id: &Uuid, field_name: &str) -> String {
    format!("{row_id}|{field_name}")
}

/// `clinical_claim` is `false` everywhere; Soul makes no diagnostic claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NotAClinicalClaim;

impl Serialize for NotAClinicalClaim {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bool(false)
    }
}

impl<'de> Deserialize<'de> for NotAClinicalClaim {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = bool::deserialize(d)?;
        if raw {
            Err(D::Error::invalid_value(Unexpected::Bool(raw), &"false"))
        } else {
            Ok(NotAClinicalClaim)
        }
    }
}
