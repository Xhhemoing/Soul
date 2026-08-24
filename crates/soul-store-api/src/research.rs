//! The read-only research surface.
//!
//! WP01 shipped no research API at all, which left WP02 unable to express
//! AC-20 against the storage boundary: the manifest has to come out of the same
//! backend that holds the events, but building it must not be reachable from
//! anything that writes. So it is a separate trait rather than a method on
//! [`crate::SoulStore`], and every backend that implements it is read-only by
//! signature — `&self`, no interior mutability required.
//!
//! Two invariants are enforced here rather than left to each backend:
//!
//! * `written_to_disk` is `false` for a `research_preview`, because v0.1
//!   research never lands in a file (PRODUCT_LOCK, DECISIONS D7/D18);
//! * `third_party_rows` may only be set from a number the backend counted.
//!   [`zero_third_party_rows`] is the only constructor, and it refuses anything
//!   but zero. A backend that hard-codes the constant is not passing a count
//!   through this function, and a backend whose exclusion filter regresses will
//!   fail here instead of publishing the row.

use uuid::Uuid;

use soul_schema::export_manifest::{
    ExportField, ExportKind, ExportRow, RedactionProfile, SoulExportManifest, ZeroThirdPartyRows,
};

use crate::types::{StoreError, StoreResult};

/// What the caller wants previewed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchPreviewRequest {
    /// Upper bound on rows shown. The preview is a sample, not an export.
    pub max_rows: usize,
    /// Identity of this preview. Callers that want a reproducible manifest,
    /// such as tests, supply their own.
    pub manifest_id: Option<Uuid>,
}

impl Default for ResearchPreviewRequest {
    fn default() -> Self {
        ResearchPreviewRequest {
            max_rows: 50,
            manifest_id: None,
        }
    }
}

impl ResearchPreviewRequest {
    pub fn with_max_rows(mut self, max_rows: usize) -> Self {
        self.max_rows = max_rows;
        self
    }

    pub fn with_manifest_id(mut self, manifest_id: Uuid) -> Self {
        self.manifest_id = Some(manifest_id);
        self
    }
}

/// A preview, plus the counts that show the exclusion actually ran.
///
/// The extra counts are outside [`SoulExportManifest`] on purpose: the frozen
/// contract is `additionalProperties: false`, and these numbers are evidence
/// for the caller and for CI, not part of the published shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchPreviewReport {
    pub manifest: SoulExportManifest,
    /// Candidate rows the query produced before anything was dropped.
    pub candidate_rows_total: u64,
    /// Candidate rows dropped because they were about someone else. A fixture
    /// with third-party data in it must make this non-zero, or the exclusion is
    /// untested.
    pub third_party_rows_excluded: u64,
}

impl ResearchPreviewReport {
    /// Rows that reached the manifest.
    pub fn row_count(&self) -> usize {
        self.manifest.rows.len()
    }
}

/// The only way to populate `third_party_rows`.
///
/// Takes the number a backend counted in its own output and refuses anything
/// but zero, so the field can never be a literal that nobody checked.
pub fn zero_third_party_rows(counted_in_output: u64) -> StoreResult<ZeroThirdPartyRows> {
    if counted_in_output == 0 {
        Ok(ZeroThirdPartyRows)
    } else {
        Err(StoreError::ContractViolation(format!(
            "{counted_in_output} third-party row(s) reached the research preview; \
             PRODUCT_LOCK allows none"
        )))
    }
}

/// Assemble a preview manifest with the invariants already applied.
pub fn preview_manifest(
    manifest_id: Uuid,
    fields: Vec<ExportField>,
    rows: Vec<ExportRow>,
    third_party_rows_in_output: u64,
) -> StoreResult<SoulExportManifest> {
    Ok(SoulExportManifest {
        schema_version: soul_schema::common::SchemaVersion,
        manifest_id,
        export_kind: ExportKind::ResearchPreview,
        fields: Some(fields),
        rows,
        third_party_rows: zero_third_party_rows(third_party_rows_in_output)?,
        // v0.1 research is preview-only. Nothing here has been, or may be,
        // written to a file.
        written_to_disk: false,
        redaction_profile: RedactionProfile::default(),
    })
}

/// A backend that can describe what research would see, without exporting it.
pub trait ResearchPreview {
    /// Read-only by signature. Implementations must not write a file and must
    /// not mutate the store.
    fn research_preview(
        &self,
        request: &ResearchPreviewRequest,
    ) -> StoreResult<ResearchPreviewReport>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_zero_count_is_refused_rather_than_rounded_down() {
        assert!(zero_third_party_rows(0).is_ok());
        assert!(matches!(
            zero_third_party_rows(1),
            Err(StoreError::ContractViolation(_))
        ));
    }

    #[test]
    fn a_preview_manifest_never_claims_to_have_been_written() {
        let manifest = preview_manifest(
            "0192a1b2-c3d4-7e5f-8a9b-0c1d2e3f4001".parse().expect("uuid"),
            vec![ExportField::EventKind],
            vec![ExportRow {
                event_kind: Some("app.foreground".into()),
                aggregate_count: Some(3),
                ..ExportRow::default()
            }],
            0,
        )
        .expect("manifest");
        assert!(!manifest.written_to_disk);
        assert_eq!(manifest.export_kind, ExportKind::ResearchPreview);
    }
}
