//! WP02's command surface: open the encrypted store, preview a forget, execute
//! one, and describe what research would see.
//!
//! Deliberately thin. Everything that decides anything lives in `soul-store`;
//! what is here is the shape a caller sees, so the desktop shell in WP09 has
//! something to bind to that is not a database handle. There is no UI, no
//! policy, and no formatting: WP08 owns the audit and permission decisions that
//! will wrap these calls, and each of them is a separate work package.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use soul_schema::audit::SoulAuditEntry;
use soul_schema::export_manifest::ExportRow;
use soul_store::{KeyProvider, SqlCipherStore, TestKeyProvider};
use soul_store_api::forget::{ForgetImpact, ForgetOps, ForgetReceipt, ForgetUnit};
use soul_store_api::research::{ResearchPreview, ResearchPreviewReport, ResearchPreviewRequest};
use soul_store_api::types::StoreResult;
use soul_store_api::AuditLog;

/// File name of the main database inside whichever directory holds it.
///
/// On Windows that directory is `%LOCALAPPDATA%\Soul`; resolving it is WP09's
/// job, because it is a shell concern and this crate must stay runnable
/// headless on a Linux CI host.
pub const DATABASE_FILE_NAME: &str = "soul.db";

pub fn database_path(directory: impl AsRef<Path>) -> PathBuf {
    directory.as_ref().join(DATABASE_FILE_NAME)
}

/// Open the store under a caller-supplied key provider.
pub fn open_store(
    directory: impl AsRef<Path>,
    keys: &dyn KeyProvider,
) -> StoreResult<SqlCipherStore> {
    SqlCipherStore::open(database_path(directory), keys)
}

/// Open the store with test key material.
///
/// The name says what it is for. `TestKeyProvider` touches no platform key
/// store, so this is the entry point headless tests and Linux CI use; a real
/// installation goes through [`open_store`] with the platform provider.
pub fn open_test_store(directory: impl AsRef<Path>, seed: &str) -> StoreResult<SqlCipherStore> {
    let keys = TestKeyProvider::from_seed(seed);
    open_store(directory, &keys)
}

/// What forgetting `unit` would cost. Read-only; destroys nothing.
pub fn preview_forget(store: &SqlCipherStore, unit: ForgetUnit) -> StoreResult<ForgetImpact> {
    store.preview_impact(unit)
}

/// Destroy the content keys the preview named.
///
/// Irreversible. The caller is responsible for having shown
/// [`preview_forget`] first; the receipt repeats the impact so the two can be
/// compared after the fact.
pub fn execute_forget(store: &mut SqlCipherStore, unit: ForgetUnit) -> StoreResult<ForgetReceipt> {
    store.execute_forget(unit)
}

/// Describe what the research track would see. Writes nothing, anywhere.
pub fn research_preview(
    store: &SqlCipherStore,
    request: &ResearchPreviewRequest,
) -> StoreResult<ResearchPreviewReport> {
    store.research_preview(request)
}

// ------------------------------------------------- what a screen may draw ---

/// What the research screen says about itself.
///
/// Held here rather than in the interface because it is a promise about the
/// build: v0.1 research is preview-only, and the manifest beside this sentence
/// carries `written_to_disk: false` as a value the store refused to set any
/// other way.
pub const RESEARCH_PREVIEW_NOTICE: &str = "研究预览只在这块屏幕上存在：它不落盘、不出网，\
    关掉这一页它就没有了。行是按事件类型与小时桶聚合出来的计数，别人的数据在查询里就被排除掉了。";

/// What the audit screen says about itself.
pub const AUDIT_CHAIN_NOTICE: &str = "审计链只记「发生过什么」，不记内容：一条记录里能有的\
    只有动作、结论、理由码、涉及到的编号和计数。每一条都带着上一条的哈希，所以中间被人改过或者\
    抽掉一条，回放的时候就对不上。";

/// One row of the preview.
///
/// Every field is optional because the manifest's is: a row says only what its
/// grouping produced. There is no field here that could hold a body, a name or
/// an identifier — `export-manifest.schema.json` has none either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchRowView {
    pub event_kind: Option<String>,
    pub time_bucket_utc: Option<String>,
    pub duration_bucket: Option<String>,
    pub self_trait_axis: Option<String>,
    pub self_trait_band: Option<String>,
    pub aggregate_count: Option<u64>,
}

impl ResearchRowView {
    fn of(row: &ExportRow) -> ResearchRowView {
        ResearchRowView {
            event_kind: row.event_kind.clone(),
            time_bucket_utc: row.time_bucket_utc.clone(),
            duration_bucket: row.duration_bucket.clone(),
            self_trait_axis: row.self_trait_axis.clone(),
            self_trait_band: row.self_trait_band.as_ref().map(word),
            aggregate_count: row.aggregate_count,
        }
    }
}

/// The whole research screen. AC-20 as three numbers and a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchPreviewView {
    pub manifest_id: String,
    pub export_kind: String,
    /// Always false. `preview_manifest` is the only constructor and it sets
    /// the field itself.
    pub written_to_disk: bool,
    /// Always 0. `zero_third_party_rows` refuses to build a manifest with any
    /// other number in it, so this is a count that was checked rather than a
    /// literal.
    pub third_party_rows: u64,
    /// Candidates the query produced before anything was dropped.
    pub candidate_rows_total: u64,
    /// Candidates dropped because they were about someone else. Non-zero is
    /// what shows the exclusion actually ran.
    pub third_party_rows_excluded: u64,
    pub fields: Vec<String>,
    pub rows: Vec<ResearchRowView>,
    /// Always `excluded`.
    pub third_party_body: String,
    pub notice: String,
}

impl ResearchPreviewView {
    fn of(report: &ResearchPreviewReport) -> ResearchPreviewView {
        ResearchPreviewView {
            manifest_id: report.manifest.manifest_id.to_string(),
            export_kind: word(&report.manifest.export_kind),
            written_to_disk: report.manifest.written_to_disk,
            third_party_rows: 0,
            candidate_rows_total: report.candidate_rows_total,
            third_party_rows_excluded: report.third_party_rows_excluded,
            fields: report.manifest.fields.iter().flatten().map(word).collect(),
            rows: report
                .manifest
                .rows
                .iter()
                .map(ResearchRowView::of)
                .collect(),
            third_party_body: word(&report.manifest.redaction_profile.third_party_body),
            notice: RESEARCH_PREVIEW_NOTICE.to_owned(),
        }
    }
}

/// The preview, as the screen shows it.
pub fn research_view(store: &SqlCipherStore, max_rows: usize) -> StoreResult<ResearchPreviewView> {
    let request = ResearchPreviewRequest::default().with_max_rows(max_rows);
    Ok(ResearchPreviewView::of(&research_preview(store, &request)?))
}

/// One entry, played back.
///
/// Ids, two vocabulary words, a reason code and counts. There is no field on
/// this that could hold prose, and there is none on `SoulAuditEntry` either —
/// `audit.schema.json` is `additionalProperties: false` precisely so a stray
/// `body` fails validation instead of reaching a screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEntryView {
    pub seq: u64,
    pub entry_id: String,
    pub ts: String,
    pub action: String,
    pub decision: String,
    pub reason_code: Option<String>,
    pub subject_refs: Vec<String>,
    pub items: Option<u64>,
    pub bytes: Option<u64>,
    pub plan_hash: Option<String>,
    pub capability_token_id: Option<String>,
    pub egress_class: Option<String>,
    pub prev_hash: String,
    pub entry_hash: String,
    /// Whether this entry's `prev_hash` is the previous entry's `entry_hash`.
    /// The whole chain is verified separately; this is the same fact one line
    /// at a time, so a reader can see where a break is rather than only that
    /// there is one.
    pub follows_previous: bool,
}

/// The chain, and whether it still holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditChainView {
    pub entries: Vec<AuditEntryView>,
    /// The store's own walk of the chain.
    pub verified: bool,
    /// Where it broke, in the store's words. Present only when `verified` is
    /// false.
    pub verification_problem: Option<String>,
    pub notice: String,
}

/// Read the chain back and check it, which is what "playback" means here.
pub fn audit_view(store: &SqlCipherStore) -> StoreResult<AuditChainView> {
    let entries = store.list_audit()?;
    let problem = store
        .verify_audit_chain()
        .err()
        .map(|error| error.to_string());
    let mut previous: Option<String> = None;
    let mut played = Vec::with_capacity(entries.len());
    for entry in &entries {
        played.push(audit_entry(entry, previous.as_deref()));
        previous = Some(entry.entry_hash.as_str().to_owned());
    }
    Ok(AuditChainView {
        entries: played,
        verified: problem.is_none(),
        verification_problem: problem,
        notice: AUDIT_CHAIN_NOTICE.to_owned(),
    })
}

fn audit_entry(entry: &SoulAuditEntry, previous_hash: Option<&str>) -> AuditEntryView {
    AuditEntryView {
        seq: entry.seq,
        entry_id: entry.entry_id.to_string(),
        ts: entry.ts.as_str().to_owned(),
        action: word(&entry.action),
        decision: word(&entry.decision),
        reason_code: entry.reason_code.clone(),
        subject_refs: entry
            .subject_refs
            .iter()
            .flatten()
            .map(|id| id.to_string())
            .collect(),
        items: entry.counts.and_then(|counts| counts.items),
        bytes: entry.counts.and_then(|counts| counts.bytes),
        plan_hash: entry
            .plan_hash
            .as_ref()
            .map(|hash| hash.as_str().to_owned()),
        capability_token_id: entry.capability_token_id.map(|id| id.to_string()),
        egress_class: entry.egress_class.as_ref().map(word),
        follows_previous: match previous_hash {
            Some(hash) => entry.prev_hash.as_str() == hash,
            // The first entry links to the genesis hash the store writes, and
            // whether that is right is `verify_audit_chain`'s question.
            None => true,
        },
        prev_hash: entry.prev_hash.as_str().to_owned(),
        entry_hash: entry.entry_hash.as_str().to_owned(),
    }
}

/// The contract's own spelling of a small enum, taken from serde rather than
/// written out a second time.
fn word<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}
