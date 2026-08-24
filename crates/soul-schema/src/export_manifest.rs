//! Model for `docs/schemas/export-manifest.schema.json`.
//!
//! v0.1 research output is preview-only: `third_party_rows` is `0` by
//! contract and a `research_preview` may not claim to have been written.

use serde::de::{Error as DeError, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::common::{SchemaVersion, SupportedBand};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportKind {
    ResearchPreview,
    SelfFullExport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportField {
    EventKind,
    TimeBucketUtc,
    DurationBucket,
    SelfTraitAxis,
    SelfTraitBand,
    AggregateCount,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_bucket_utc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_trait_axis: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_trait_band: Option<SupportedBand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate_count: Option<u64>,
}

/// Always zero. Third-party rows have no legal path into an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ZeroThirdPartyRows;

impl Serialize for ZeroThirdPartyRows {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(0)
    }
}

impl<'de> Deserialize<'de> for ZeroThirdPartyRows {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = u64::deserialize(d)?;
        if raw == 0 {
            Ok(ZeroThirdPartyRows)
        } else {
            Err(D::Error::invalid_value(Unexpected::Unsigned(raw), &"0"))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ThirdPartyBodyExcluded;

impl Serialize for ThirdPartyBodyExcluded {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str("excluded")
    }
}

impl<'de> Deserialize<'de> for ThirdPartyBodyExcluded {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        if raw == "excluded" {
            Ok(ThirdPartyBodyExcluded)
        } else {
            Err(D::Error::invalid_value(Unexpected::Str(&raw), &"excluded"))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoOneTimeOverride;

impl Serialize for NoOneTimeOverride {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bool(false)
    }
}

impl<'de> Deserialize<'de> for NoOneTimeOverride {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = bool::deserialize(d)?;
        if raw {
            Err(D::Error::invalid_value(Unexpected::Bool(raw), &"false"))
        } else {
            Ok(NoOneTimeOverride)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RedactionProfile {
    pub third_party_body: ThirdPartyBodyExcluded,
    /// The single-message override that E1 drafting allows has no counterpart
    /// here; research never sees third-party prose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub one_time_override_allowed: Option<NoOneTimeOverride>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulExportManifest {
    pub schema_version: SchemaVersion,
    pub manifest_id: Uuid,
    pub export_kind: ExportKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<ExportField>>,
    pub rows: Vec<ExportRow>,
    pub third_party_rows: ZeroThirdPartyRows,
    pub written_to_disk: bool,
    pub redaction_profile: RedactionProfile,
}
