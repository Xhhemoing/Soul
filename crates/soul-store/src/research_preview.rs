//! AC-20: show what research would see, and show it in memory only.
//!
//! v0.1 research is a preview. DECISIONS D7 and D18 say the redaction profile
//! has not been red-teamed, so nothing may land in a file yet; the manifest is
//! returned to the caller and that is the end of it. This module therefore
//! opens no path to the filesystem at all, and `tests/research_preview.rs`
//! reads this source back to prove it stayed that way.
//!
//! The other half of AC-20 is that third-party rows are absent because they
//! were excluded, not because nobody looked. So the grouping query keeps the
//! privacy subject on every candidate row, the exclusion is a filter over that
//! tagged set, and the number published as `third_party_rows` is counted from
//! the rows that survived. A fixture with third-party events in it makes
//! `third_party_rows_excluded` non-zero, which is what shows the filter ran.

use serde_json::Value;

use soul_schema::common::{EvidenceBand, SupportedBand};
use soul_schema::export_manifest::{ExportField, ExportRow};
use soul_store_api::research::{
    preview_manifest, ResearchPreview, ResearchPreviewReport, ResearchPreviewRequest,
};
use soul_store_api::types::StoreResult;
use uuid::Uuid;

use crate::store::{backend, SqlCipherStore};

/// Who a candidate row is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubjectClass {
    /// The owner. The only class research may see.
    Owner,
    /// Someone else, or a conversation that mixes the two. PRODUCT_LOCK treats
    /// `mixed` exactly like `third_party`.
    ThirdParty,
    /// Machine-generated bookkeeping, about nobody.
    Machine,
}

impl SubjectClass {
    fn parse(raw: &str) -> SubjectClass {
        match raw {
            "self" => SubjectClass::Owner,
            "third_party" | "mixed" => SubjectClass::ThirdParty,
            _ => SubjectClass::Machine,
        }
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    subject: SubjectClass,
    row: ExportRow,
}

/// Events grouped into hour buckets, with the privacy subject kept alongside.
///
/// The bucket is the hour of an RFC 3339 instant recorded in UTC. A timestamp
/// carrying an offset instead degrades to a date-level bucket rather than being
/// relabelled as UTC it is not.
const EVENT_ROLLUP_SQL: &str = "
    SELECT kind,
           CASE WHEN ts LIKE '%Z'
                THEN substr(ts, 1, 13) || ':00Z'
                ELSE substr(ts, 1, 10)
           END AS time_bucket,
           privacy_subject,
           count(*) AS aggregate_count
    FROM events
    GROUP BY kind, time_bucket, privacy_subject
    ORDER BY time_bucket, kind, privacy_subject
";

impl SqlCipherStore {
    fn event_candidates(&self) -> StoreResult<Vec<Candidate>> {
        let mut statement = self.conn.prepare(EVENT_ROLLUP_SQL).map_err(backend)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(backend)?;

        let mut candidates = Vec::new();
        for row in rows {
            let (kind, bucket, subject, count) = row.map_err(backend)?;
            candidates.push(Candidate {
                subject: SubjectClass::parse(&subject),
                row: ExportRow {
                    event_kind: Some(kind),
                    time_bucket_utc: Some(bucket),
                    aggregate_count: Some(count.max(0) as u64),
                    ..ExportRow::default()
                },
            });
        }
        Ok(candidates)
    }

    /// Trait axes are about the owner by definition, but they still go through
    /// the same tagging so a future axis with a third-party provenance cannot
    /// bypass the filter by arriving on a different code path.
    fn trait_axis_candidates(&self) -> StoreResult<Vec<Candidate>> {
        let mut statement = self
            .conn
            .prepare("SELECT doc FROM profiles ORDER BY profile_id")
            .map_err(backend)?;
        let rows = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(backend)?;

        let mut candidates = Vec::new();
        for row in rows {
            let doc: Value = serde_json::from_str(&row.map_err(backend)?).map_err(backend)?;
            let Some(axes) = doc.get("trait_axes").and_then(Value::as_array) else {
                continue;
            };
            for axis in axes {
                let Some(axis_id) = axis.get("axis_id").and_then(Value::as_str) else {
                    continue;
                };
                let band = axis
                    .get("evidence_band")
                    .cloned()
                    .map(serde_json::from_value::<EvidenceBand>)
                    .and_then(Result::ok);
                // An axis with no evidence behind it says nothing worth
                // exporting, and `none` is not a band the manifest admits.
                let band = match band {
                    Some(EvidenceBand::Weak) => SupportedBand::Weak,
                    Some(EvidenceBand::Moderate) => SupportedBand::Moderate,
                    Some(EvidenceBand::Strong) => SupportedBand::Strong,
                    Some(EvidenceBand::None) | None => continue,
                };
                candidates.push(Candidate {
                    subject: SubjectClass::Owner,
                    row: ExportRow {
                        self_trait_axis: Some(axis_id.to_owned()),
                        self_trait_band: Some(band),
                        ..ExportRow::default()
                    },
                });
            }
        }
        Ok(candidates)
    }
}

impl ResearchPreview for SqlCipherStore {
    fn research_preview(
        &self,
        request: &ResearchPreviewRequest,
    ) -> StoreResult<ResearchPreviewReport> {
        let mut candidates = self.event_candidates()?;
        candidates.extend(self.trait_axis_candidates()?);

        let candidate_rows_total = candidates.len() as u64;
        let third_party_rows_excluded = candidates
            .iter()
            .filter(|candidate| candidate.subject == SubjectClass::ThirdParty)
            .count() as u64;

        let kept: Vec<&Candidate> = candidates
            .iter()
            .filter(|candidate| candidate.subject == SubjectClass::Owner)
            .take(request.max_rows)
            .collect();

        // Counted over what is about to be published, not asserted about it.
        // If the filter above ever stops excluding, this is non-zero and the
        // manifest refuses to be built.
        let third_party_rows_in_output = kept
            .iter()
            .filter(|candidate| candidate.subject == SubjectClass::ThirdParty)
            .count() as u64;

        let rows: Vec<ExportRow> = kept
            .into_iter()
            .map(|candidate| candidate.row.clone())
            .collect();
        let fields = fields_present(&rows);
        let manifest = preview_manifest(
            request.manifest_id.unwrap_or_else(Uuid::now_v7),
            fields,
            rows,
            third_party_rows_in_output,
        )?;

        Ok(ResearchPreviewReport {
            manifest,
            candidate_rows_total,
            third_party_rows_excluded,
        })
    }
}

/// The field list describes what the preview actually contains, so a reader
/// cannot be told a column exists when no row carries it.
fn fields_present(rows: &[ExportRow]) -> Vec<ExportField> {
    let mut fields = Vec::new();
    let push = |present: bool, field: ExportField, into: &mut Vec<ExportField>| {
        if present && !into.contains(&field) {
            into.push(field);
        }
    };
    push(
        rows.iter().any(|row| row.event_kind.is_some()),
        ExportField::EventKind,
        &mut fields,
    );
    push(
        rows.iter().any(|row| row.time_bucket_utc.is_some()),
        ExportField::TimeBucketUtc,
        &mut fields,
    );
    push(
        rows.iter().any(|row| row.duration_bucket.is_some()),
        ExportField::DurationBucket,
        &mut fields,
    );
    push(
        rows.iter().any(|row| row.self_trait_axis.is_some()),
        ExportField::SelfTraitAxis,
        &mut fields,
    );
    push(
        rows.iter().any(|row| row.self_trait_band.is_some()),
        ExportField::SelfTraitBand,
        &mut fields,
    );
    push(
        rows.iter().any(|row| row.aggregate_count.is_some()),
        ExportField::AggregateCount,
        &mut fields,
    );
    fields
}
