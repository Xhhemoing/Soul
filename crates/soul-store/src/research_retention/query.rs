use std::collections::BTreeMap;

use rusqlite::Row;
use serde::Deserialize;
use serde_json::Value;
use soul_schema::common::{field_aad, SealedSubject};
use soul_schema::event::SoulEvent;
use soul_schema::validate::SchemaId;

use super::*;
use crate::store::{backend, enum_text};

const MAX_EVENTS: usize = 100_000;
const MAX_METADATA: usize = 16 * 1024 * 1024;

// Also inspect documents claiming collector origin when their index disagrees.
// json_valid prevents corrupt unrelated documents from breaking the query.
const CANDIDATES: &str = "
 SELECT e.event_id,e.ts,e.source,e.kind,e.actor_subject,e.privacy_subject,
        e.body_blob_id,
        CASE WHEN length(CAST(e.doc AS BLOB)) <= 16777216 THEN e.doc END,
        b.row_id,b.field,b.content_key_id,b.subject,
        EXISTS(SELECT 1 FROM content_keys k WHERE k.content_key_id=b.content_key_id),
        EXISTS(SELECT 1 FROM destroyed_content_keys d WHERE d.content_key_id=b.content_key_id),
        length(b.nonce),length(b.ciphertext),
        length(CAST(e.event_id||e.ts||e.source||e.kind||e.actor_subject||e.privacy_subject||
          coalesce(e.body_blob_id,'')||e.doc||coalesce(b.row_id,'')||coalesce(b.field,'')||
          coalesce(b.content_key_id,'')||coalesce(b.subject,'') AS BLOB)),
        count(*) OVER()
 FROM events e LEFT JOIN sealed_blobs b ON b.blob_id=e.body_blob_id
 WHERE e.source='collector.foreground_app'
    OR CASE WHEN json_valid(e.doc) THEN json_extract(e.doc,'$.source') END='collector.foreground_app'
 ORDER BY e.event_id LIMIT 100001";

/// Exact metadata only, deliberately neither Debug nor Serialize. No keys or
/// body ciphertext are loaded. Equality detects same-count source replacement.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct SourceSnapshot {
    index: [String; 6],
    body_blob: Option<String>,
    doc: String,
    binding: [Option<String>; 4],
    key_exists: bool,
    key_destroyed: bool,
    nonce_bytes: Option<i64>,
    ciphertext_bytes: Option<i64>,
}

impl SourceSnapshot {
    fn read(row: &Row<'_>) -> StoreResult<Self> {
        Ok(Self {
            index: [
                row.get(0),
                row.get(1),
                row.get(2),
                row.get(3),
                row.get(4),
                row.get(5),
            ]
            .into_iter()
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(backend)?
            .try_into()
            .map_err(|_| refused("RESEARCH_SOURCE_UNAVAILABLE"))?,
            body_blob: row.get(6).map_err(backend)?,
            doc: row
                .get::<_, Option<String>>(7)
                .map_err(backend)?
                .ok_or_else(|| refused("RESEARCH_METADATA_LIMIT"))?,
            binding: [row.get(8), row.get(9), row.get(10), row.get(11)]
                .into_iter()
                .collect::<rusqlite::Result<Vec<Option<String>>>>()
                .map_err(backend)?
                .try_into()
                .map_err(|_| refused("RESEARCH_SOURCE_UNAVAILABLE"))?,
            key_exists: row.get(12).map_err(backend)?,
            key_destroyed: row.get(13).map_err(backend)?,
            nonce_bytes: row.get(14).map_err(backend)?,
            ciphertext_bytes: row.get(15).map_err(backend)?,
        })
    }

    fn consistent(&self, event: &SoulEvent) -> StoreResult<bool> {
        Ok(self.index
            == [
                event.event_id.to_string(),
                event.ts.as_str().to_owned(),
                enum_text(&event.source)?,
                enum_text(&event.kind)?,
                enum_text(&event.actor_subject)?,
                enum_text(&event.privacy.subject)?,
            ]
            && self.body_blob == event.body_ref.as_ref().map(|body| body.blob_id.to_string()))
    }

    fn available(&self, event: &SoulEvent) -> bool {
        let Some(body) = &event.body_ref else {
            return false;
        };
        body.subject == SealedSubject::Owner
            && body.aad.as_deref() == Some(field_aad(&event.event_id, "body_ref").as_str())
            && self.binding
                == [
                    Some(event.event_id.to_string()),
                    Some("body_ref".into()),
                    Some(body.content_key_id.to_string()),
                    Some("self".into()),
                ]
            && self.key_exists
            && !self.key_destroyed
            && self.nonce_bytes == Some(24)
            && self.ciphertext_bytes.is_some_and(|bytes| bytes >= 16)
    }
}

fn valid_day(day: &str) -> bool {
    let bytes = day.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, c)| i != 4 && i != 7 && !c.is_ascii_digit())
    {
        return false;
    }
    let year: u32 = day[..4].parse().unwrap_or(0);
    let month: usize = day[5..7].parse().unwrap_or(0);
    let date: u32 = day[8..].parse().unwrap_or(0);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        0,
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    year > 0 && (1..=12).contains(&month) && date > 0 && date <= days[month]
}

fn canonical_time(ts: &str) -> bool {
    let bytes = ts.as_bytes();
    if bytes.len() != 20
        || !bytes.is_ascii()
        || !valid_day(&ts[..10])
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    [11, 12, 14, 15, 17, 18]
        .into_iter()
        .all(|i| bytes[i].is_ascii_digit())
        && &ts[11..13] < "24"
        && &ts[14..16] < "60"
        && &ts[17..19] < "60"
}

fn in_scope(ts: &str, scope: &ResearchScope) -> bool {
    &ts[..10] >= scope.from_day.as_str() && &ts[..10] < scope.to_day.as_str()
}

fn policy_permits(doc: &Value) -> bool {
    doc["source"] == "collector.foreground_app"
        && doc["kind"] == "app.foreground"
        && doc["actor_subject"] == "self"
        && doc["privacy"]["subject"] == "self"
        && doc["privacy"]["egress"]["research_export"] == "bucket"
        && doc["privacy"]["purposes"]
            .as_array()
            .is_some_and(|purposes| purposes.iter().any(|purpose| purpose == "research"))
}

struct SourceScan {
    sources: Vec<SourceSnapshot>,
    candidate_events: u64,
    out_of_range_events: u64,
}

fn scan_sources(
    store: &SqlCipherStore,
    scope: &ResearchScope,
    mut visit: impl FnMut(&SourceSnapshot, Option<&Value>) -> StoreResult<()>,
) -> StoreResult<SourceScan> {
    if !valid_day(&scope.from_day) || !valid_day(&scope.to_day) || scope.from_day >= scope.to_day {
        return Err(refused("RESEARCH_SCOPE_INVALID"));
    }
    let mut scan = SourceScan {
        sources: Vec::new(),
        candidate_events: 0,
        out_of_range_events: 0,
    };
    let mut metadata = 0usize;
    let mut statement = store.conn.prepare(CANDIDATES).map_err(backend)?;
    let mut rows = statement.query([]).map_err(backend)?;
    while let Some(row) = rows.next().map_err(backend)? {
        let total: i64 = row.get(17).map_err(backend)?;
        if total > MAX_EVENTS as i64 {
            return Err(refused("RESEARCH_CANDIDATE_LIMIT"));
        }
        scan.candidate_events += 1;
        if scan.candidate_events > MAX_EVENTS as u64 {
            return Err(refused("RESEARCH_CANDIDATE_LIMIT"));
        }
        let size: i64 = row.get(16).map_err(backend)?;
        metadata = metadata
            .checked_add(usize::try_from(size).map_err(|_| refused("RESEARCH_METADATA_LIMIT"))?)
            .ok_or_else(|| refused("RESEARCH_METADATA_LIMIT"))?;
        if metadata > MAX_METADATA {
            return Err(refused("RESEARCH_METADATA_LIMIT"));
        }
        let snapshot = SourceSnapshot::read(row)?;
        let doc: Value = match serde_json::from_str(&snapshot.doc) {
            Ok(doc) => doc,
            Err(_) => {
                visit(&snapshot, None)?;
                scan.sources.push(snapshot);
                continue;
            }
        };
        let ts = doc["ts"].as_str().unwrap_or_default();
        // Only matching canonical timestamps prove this source is unrelated.
        if ts == snapshot.index[1] && canonical_time(ts) && !in_scope(ts, scope) {
            scan.out_of_range_events += 1;
            continue;
        }
        visit(&snapshot, Some(&doc))?;
        scan.sources.push(snapshot);
    }
    Ok(scan)
}

pub(super) fn source_snapshots(
    store: &SqlCipherStore,
    scope: &ResearchScope,
) -> StoreResult<Vec<SourceSnapshot>> {
    // Revalidate the exact sources without rebuilding discarded classifications
    // and aggregate rows. The shared scan retains all limits and scope checks.
    Ok(scan_sources(store, scope, |_, _| Ok(()))?.sources)
}

pub(super) fn prepare(
    store: &SqlCipherStore,
    scope: &ResearchScope,
) -> StoreResult<HeldResearchDataset> {
    let mut counts = ResearchCounts::default();
    let mut buckets = BTreeMap::<String, u64>::new();
    let scan = scan_sources(store, scope, |snapshot, doc| {
        let Some(doc) = doc else {
            counts.excluded_unavailable_events += 1;
            return Ok(());
        };
        let ts = doc["ts"].as_str().unwrap_or_default();
        if !policy_permits(doc) {
            counts.excluded_policy_events += 1;
        } else if let Ok(event) = SoulEvent::deserialize(doc) {
            if !snapshot.consistent(&event)? {
                counts.excluded_unavailable_events += 1;
            } else if !canonical_time(ts) {
                counts.excluded_time_events += 1;
            } else if !store.schemas.is_valid(SchemaId::Event, doc) || !snapshot.available(&event) {
                counts.excluded_unavailable_events += 1;
            } else {
                counts.included_events += 1;
                *buckets.entry(format!("{}:00:00Z", &ts[..13])).or_default() += 1;
            }
        } else {
            counts.excluded_unavailable_events += 1;
        }
        Ok(())
    })?;
    counts.candidate_events = scan.candidate_events;
    counts.out_of_range_events = scan.out_of_range_events;
    let rows = buckets
        .into_iter()
        .map(|(time_bucket_utc, aggregate_count)| ActivityBucket {
            event_kind: "app.foreground".into(),
            time_bucket_utc,
            aggregate_count,
        })
        .collect();
    Ok(HeldResearchDataset {
        dataset: ResearchDataset {
            scope: scope.clone(),
            rows,
            counts,
        },
        sources: scan.sources,
    })
}
