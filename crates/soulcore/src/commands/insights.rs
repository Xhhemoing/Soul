//! Local feedback on a committed import. Wording is rebuilt from A2, never stored.
//!
//! The persisted dependency record contains only UUIDs, integers and booleans.
//! A contact is included whole or omitted whole: a resource limit must never
//! turn a partial evidence set into a claim about the complete relationship.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use soul_graph::interaction::{self, Direction, Venue};
use soul_graph::model::{SoulGraph, TieEdge, TieType};
use soul_schema::common::{
    field_aad, ActorSubject, Derivation, E1Disposition, Privacy, Purpose, ResearchDisposition,
    RetentionMode, SealedSubject, Subject, SupportedBand, Timestamp,
};
use soul_schema::contact::ContactClass;
use soul_schema::event::{EventKind, EventSource};
use soul_schema::evidence::{EvidenceKind, EvidenceMethod, SoulEvidence};
use soul_schema::memory::ForgetState;
use soul_store::SqlCipherStore;
use soul_store_api::import::{
    ImportCounts, ImportIndexStore, ImportNamespaceState, ImportOperation, ImportOperationState,
};
use soul_store_api::insight::{InsightValidity, LocalInsightBundle};
use soul_store_api::{BlobStore, EventStore, ProfileStore};

use super::draft::PersonSummaryView;
use super::session::SessionRefusal;

const SNAPSHOT_VERSION: i64 = 1;
const MAX_CONTACTS: usize = 16;
const MAX_UUIDS: usize = 4096;
const MAX_VALUES: usize = 65_536;
const MAX_FLAGS: usize = 16_384;
const RECEIPT_ONLY: &str = "本次导入已完成；目前没有可用的本机人物摘要。这里仅保留历史操作计数。";
const CURRENT: &str = "以下内容来自本机现有记录，工作假设，非临床结论。";
const INVALIDATED: &str = "摘要依据已变化或来源已遗忘；旧摘要不再展示，历史操作计数仍保留。";
const IGNORED: &str = "这份本机摘要已忽略；忽略不会修改事实或用于训练。";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalInsightView {
    pub bundle_id: Uuid,
    pub operation_id: Uuid,
    pub namespace_id: Uuid,
    pub validity: InsightValidity,
    pub ignored: bool,
    pub counts: ImportCounts,
    pub summaries: Vec<PersonSummaryView>,
    pub notice: String,
}

/// Called inside the same domain transaction that commits the import receipt.
/// This path has no policy session, endpoint or transport capability.
pub fn create_for_import(
    store: &mut SqlCipherStore,
    operation: &ImportOperation,
    contacts_touched: &[Uuid],
    at: i64,
) -> Result<LocalInsightBundle, SessionRefusal> {
    ensure_operation(store, operation)?;
    let built = build(store, contacts_touched)?;
    let bundle = LocalInsightBundle {
        bundle_id: Uuid::now_v7(),
        operation_id: operation.operation_id,
        namespace_id: operation.namespace_id,
        created_at: at,
        validity: InsightValidity::Active,
        ignored: false,
        counts: operation.receipt.clone().ok_or_else(unavailable)?,
        dependency_uuids: built.dependencies.uuids,
        dependency_values: built.dependencies.values,
        dependency_flags: built.dependencies.flags,
    };
    store.put_local_insight(&bundle)?;
    Ok(bundle)
}

/// Revalidation also runs after restart; no persisted wording is trusted.
pub fn views(store: &mut SqlCipherStore) -> Result<Vec<LocalInsightView>, SessionRefusal> {
    store
        .recent_local_insights(100)?
        .into_iter()
        .map(|bundle| view(store, bundle))
        .collect()
}

/// A new bundle gets a new identity. The old bundle, including its ignored
/// flag, is retained as a historical operation record.
pub fn recompute(
    store: &mut SqlCipherStore,
    bundle_id: Uuid,
    at: i64,
) -> Result<LocalInsightView, SessionRefusal> {
    let old = store.local_insight(bundle_id)?;
    let contacts = candidates(&old).ok_or_else(unavailable)?;
    let operation = store.import_operation(old.operation_id)?;
    ensure_operation(store, &operation)?;
    let graph = soul_graph::load(store).map_err(|_| unavailable())?;
    if contacts.iter().any(|id| {
        graph
            .node(*id)
            .is_none_or(|node| node.forget_state != ForgetState::Active || node.is_owner())
    }) {
        return Err(unavailable());
    }
    let bundle = store.transact(|store| {
        let bundle = create_for_import(store, &operation, &contacts, at)?;
        // A missing/unreadable dependency is not a successful recomputation
        // with fewer facts. Roll back the new bundle in that case.
        if candidates(&bundle).as_ref() != Some(&contacts) {
            return Err(unavailable());
        }
        Ok(bundle)
    })?;
    view(store, bundle)
}

pub fn ignore(
    store: &mut SqlCipherStore,
    bundle_id: Uuid,
) -> Result<LocalInsightView, SessionRefusal> {
    let bundle = store.set_local_insight_ignored(bundle_id, true)?;
    view(store, bundle)
}

fn ensure_operation(
    store: &SqlCipherStore,
    operation: &ImportOperation,
) -> Result<(), SessionRefusal> {
    let namespace = store.import_namespace(operation.namespace_id)?;
    if namespace.state != ImportNamespaceState::Active
        || operation.state != ImportOperationState::Committed
        || operation.receipt.is_none()
        || store.import_operation(operation.operation_id)? != *operation
    {
        return Err(unavailable());
    }
    Ok(())
}

fn view(
    store: &mut SqlCipherStore,
    mut bundle: LocalInsightBundle,
) -> Result<LocalInsightView, SessionRefusal> {
    let mut summaries = Vec::new();
    if bundle.validity == InsightValidity::Active {
        let current = (|| {
            let operation = store.import_operation(bundle.operation_id).ok()?;
            ensure_operation(store, &operation).ok()?;
            if operation.namespace_id != bundle.namespace_id
                || operation.receipt.as_ref() != Some(&bundle.counts)
            {
                return None;
            }
            let built = build(store, &candidates(&bundle)?).ok()?;
            (built.dependencies.uuids == bundle.dependency_uuids
                && built.dependencies.values == bundle.dependency_values
                && built.dependencies.flags == bundle.dependency_flags)
                .then_some(built.summaries)
        })();
        match current {
            Some(current) => summaries = current,
            None => bundle = store.invalidate_local_insight(bundle.bundle_id)?,
        }
    }
    let notice = if bundle.validity != InsightValidity::Active {
        INVALIDATED
    } else if bundle.ignored {
        summaries.clear();
        IGNORED
    } else if summaries.is_empty() {
        RECEIPT_ONLY
    } else {
        CURRENT
    };
    Ok(LocalInsightView {
        bundle_id: bundle.bundle_id,
        operation_id: bundle.operation_id,
        namespace_id: bundle.namespace_id,
        validity: bundle.validity,
        ignored: bundle.ignored,
        counts: bundle.counts,
        summaries,
        notice: notice.to_owned(),
    })
}

fn unavailable() -> SessionRefusal {
    SessionRefusal {
        reason_code: "LOCAL_INSIGHT_UNAVAILABLE".to_owned(),
        explanation: "这份摘要的来源或依据已不可用，请查看历史操作回执。".to_owned(),
    }
}

#[derive(Default)]
struct Dependencies {
    uuids: Vec<Uuid>,
    values: Vec<i64>,
    flags: Vec<bool>,
}

impl Dependencies {
    fn fits(&self) -> bool {
        self.uuids.len() <= MAX_UUIDS
            && self.values.len() <= MAX_VALUES
            && self.flags.len() <= MAX_FLAGS
    }

    // Every variable-length group has all three lengths, so equal concatenated
    // vectors cannot hide a changed boundary between contacts or evidence rows.
    fn append(&mut self, other: Dependencies) {
        self.values.extend([
            other.uuids.len() as i64,
            other.values.len() as i64,
            other.flags.len() as i64,
        ]);
        self.uuids.extend(other.uuids);
        self.values.extend(other.values);
        self.flags.extend(other.flags);
    }

    fn count(&mut self, count: u64) -> Option<()> {
        self.values.push(i64::try_from(count).ok()?);
        Some(())
    }

    fn time(&mut self, at: Option<&Timestamp>) -> Option<()> {
        self.flags.push(at.is_some());
        if let Some(at) = at {
            self.values
                .push(soul_graph::t4d_adapt::rfc3339_to_unix(at.as_str()).ok()?);
        }
        Some(())
    }
}

struct Built {
    dependencies: Dependencies,
    summaries: Vec<PersonSummaryView>,
}

/// The header is version, contact-count; its first N UUIDs are exactly the
/// selected contacts. All later records are framed and rebuilt by this module.
fn candidates(bundle: &LocalInsightBundle) -> Option<Vec<Uuid>> {
    if bundle.dependency_values.first() != Some(&SNAPSHOT_VERSION) {
        return None;
    }
    let count = usize::try_from(*bundle.dependency_values.get(1)?).ok()?;
    if count > MAX_CONTACTS || bundle.dependency_uuids.len() < count {
        return None;
    }
    let selected = bundle.dependency_uuids[..count].to_vec();
    if selected.windows(2).any(|pair| pair[0] >= pair[1]) {
        return None;
    }
    Some(selected)
}

fn build(store: &SqlCipherStore, contacts: &[Uuid]) -> Result<Built, SessionRefusal> {
    let graph = soul_graph::load(store).map_err(|_| unavailable())?;
    let mut global_as_of = None;
    for at in graph
        .edges
        .iter()
        .filter_map(|edge| edge.tie_strength.as_of_utc.as_ref())
    {
        let at = soul_graph::t4d_adapt::rfc3339_to_unix(at.as_str()).map_err(|_| unavailable())?;
        global_as_of = Some(global_as_of.map_or(at, |previous: i64| previous.max(at)));
    }
    let mut groups = Vec::new();
    let mut summaries = Vec::new();
    let mut selected = Vec::new();
    let mut used = Dependencies::default();
    for id in contacts.iter().copied().collect::<BTreeSet<_>>() {
        if selected.len() == MAX_CONTACTS {
            break;
        }
        let Some((group, summary)) = contact(store, &graph, id) else {
            continue;
        };
        if used.uuids.len() + group.uuids.len() + MAX_CONTACTS + 1 > MAX_UUIDS
            || used.values.len() + group.values.len() + 3 * MAX_CONTACTS + 4 > MAX_VALUES
            || used.flags.len() + group.flags.len() + 2 > MAX_FLAGS
        {
            continue;
        }
        used.uuids.extend(&group.uuids);
        used.values.extend(&group.values);
        used.flags.extend(&group.flags);
        selected.push(id);
        groups.push(group);
        summaries.push(summary);
    }
    let mut dependencies = Dependencies {
        uuids: selected,
        values: vec![SNAPSHOT_VERSION, summaries.len() as i64],
        flags: vec![graph.self_contact_id.is_some(), global_as_of.is_some()],
    };
    if let Some(id) = graph.self_contact_id {
        dependencies.uuids.push(id);
    }
    if let Some(at) = global_as_of {
        dependencies.values.push(at);
    }
    for group in groups {
        dependencies.append(group);
    }
    Ok(Built {
        dependencies,
        summaries,
    })
}

fn contact(
    store: &SqlCipherStore,
    graph: &SoulGraph,
    id: Uuid,
) -> Option<(Dependencies, PersonSummaryView)> {
    let node = graph.node(id)?;
    if node.is_owner() || node.forget_state != ForgetState::Active {
        return None;
    }
    let mut d = Dependencies::default();
    d.uuids.push(id);
    d.values.push(match node.contact_class {
        ContactClass::Owner => return None,
        ContactClass::ThirdParty => 1,
        ContactClass::Group => 2,
        ContactClass::Service => 3,
    });
    d.count(node.interaction_count)?;
    d.time(node.last_contact_utc.as_ref())?;
    let mut edges = graph.edges_for(id);
    edges.sort_by_key(|edge| edge.relationship_id);
    if edges.is_empty() || edges.len() > MAX_UUIDS / 3 {
        return None;
    }
    d.values.push(edges.len() as i64);
    let mut evidence_ids = BTreeSet::new();
    for edge in edges {
        d.append(edge_dependencies(edge)?);
        evidence_ids.extend(edge.evidence_ids.iter().copied());
        if !d.fits() || evidence_ids.len() > MAX_UUIDS / 4 {
            return None;
        }
    }
    d.values.push(evidence_ids.len() as i64);
    let mut resolved = Vec::new();
    for id in evidence_ids {
        let evidence = store.get_evidence(id).ok()?;
        d.append(evidence_dependencies(store, &evidence)?);
        if !d.fits() {
            return None;
        }
        resolved.push(evidence);
    }
    let summary = soul_draft::analysis::summarize_person(graph, id, &resolved).ok()?;
    if summary.points.is_empty() {
        return None;
    }
    // A2 chooses an edge and individual citations. Record those boundaries too,
    // including its existing treatment of timestamps with equivalent instants.
    d.uuids.push(summary.relationship_id);
    d.values.push(summary.points.len() as i64);
    for point in &summary.points {
        d.values
            .extend([band(point.band()), point.evidence_ids().len() as i64]);
        d.uuids.extend(point.evidence_ids());
    }
    if !d.fits() {
        return None;
    }
    Some((d, PersonSummaryView::of(&summary).ok()?))
}

fn edge_dependencies(edge: &TieEdge) -> Option<Dependencies> {
    if edge.evidence_ids.len() > MAX_UUIDS || edge.types.len() > MAX_VALUES {
        return None;
    }
    let mut d = Dependencies::default();
    d.uuids.extend([
        edge.relationship_id,
        edge.from_contact_id,
        edge.to_contact_id,
    ]);
    let s = &edge.tie_strength;
    d.values.extend([
        band(s.band),
        optional_band(s.user_band),
        optional_band(s.machine_band),
        match s.locked_by_user {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        },
        match s.algorithm_id.as_str() {
            "" => 0,
            "T4D" => 1,
            _ => return None,
        },
        s.silent_days,
    ]);
    for count in [
        s.interaction_count,
        s.outgoing_count,
        s.incoming_count,
        s.conversation_count,
        s.active_day_count,
        s.direct_out_count,
        s.direct_in_count,
        s.group_out_count,
        s.group_in_count,
        s.direct_active_day_count,
    ] {
        d.count(count)?;
    }
    d.time(Some(&s.first_contact_utc))?;
    d.time(Some(&s.last_contact_utc))?;
    d.time(s.last_direct_contact_utc.as_ref())?;
    d.time(s.as_of_utc.as_ref())?;
    d.values.push(edge.types.len() as i64);
    for kind in &edge.types {
        d.values.push(match kind {
            TieType::Direct => 0,
            TieType::GroupOnly => 1,
            TieType::Reciprocal => 2,
            TieType::OneSided => 3,
        });
    }
    // This is the complete cited set, not the subset a particular A2 bullet uses.
    d.values.push(edge.evidence_ids.len() as i64);
    d.uuids.extend(&edge.evidence_ids);
    Some(d)
}

fn evidence_dependencies(store: &SqlCipherStore, row: &SoulEvidence) -> Option<Dependencies> {
    if row.source_refs.len() > MAX_UUIDS / 4 {
        return None;
    }
    let mut d = Dependencies::default();
    d.uuids.push(row.evidence_id);
    d.values.extend([
        match row.kind {
            EvidenceKind::Message => 0,
            EvidenceKind::UserCorrection => 1,
            _ => return None,
        },
        subject(row.subject),
        band(row.strength),
        match row.method {
            None => 0,
            Some(EvidenceMethod::UserStated) => 1,
            Some(EvidenceMethod::Manual) => 2,
            Some(EvidenceMethod::Heuristic) => 3,
            Some(EvidenceMethod::Statistical) => 4,
            Some(EvidenceMethod::Llm) => 5,
        },
        match row.exportable_to_research {
            None => 0,
            Some(false) => 1,
            Some(true) => 2,
        },
    ]);
    privacy(&mut d, row.privacy.as_ref());
    if row.kind == EvidenceKind::UserCorrection {
        if row.source_refs.len() != 1 {
            return None;
        }
        d.uuids.push(soul_graph::corrected_relationship(row)?);
        return Some(d);
    }
    let interactions = interaction::interactions_in(row);
    if interactions.is_empty()
        || interactions.len() != row.source_refs.len()
        || interactions.len() > MAX_UUIDS / 4
    {
        return None;
    }
    d.values.push(interactions.len() as i64);
    for observation in interactions {
        let mut entry = Dependencies::default();
        entry.uuids.extend([
            observation.event_id,
            observation.self_contact_id,
            observation.peer_contact_id,
        ]);
        entry.values.extend([
            match observation.direction {
                Direction::Outgoing => 0,
                Direction::Incoming => 1,
            },
            match observation.venue {
                Venue::Direct => 0,
                Venue::Group => 1,
            },
        ]);
        entry.time(Some(&observation.occurred_at))?;
        // Existing fixed-size conversation handle, not a new content digest or
        // a serialization of opaque source_refs. Canonical 32-byte boundary.
        let conversation = observation.conversation_ref.as_str().as_bytes();
        if conversation.len() != 64 {
            return None;
        }
        for pair in conversation.chunks_exact(2) {
            let byte = |b: u8| match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                _ => None,
            };
            entry
                .values
                .push(i64::from(byte(pair[0])? * 16 + byte(pair[1])?));
        }
        let event = store.get_event(observation.event_id).ok()?;
        entry.time(Some(&event.ts))?;
        entry.values.extend([
            match event.source {
                EventSource::CollectorForegroundApp => 0,
                EventSource::ImportSoulImportV1 => 1,
                EventSource::ImportTelegramDesktop => 2,
                EventSource::UiQuestionnaire => 3,
                EventSource::UiPaste => 4,
                EventSource::CoreMemory => 5,
                EventSource::CoreForget => 6,
            },
            match event.kind {
                EventKind::AppForeground => 0,
                EventKind::MessageObserved => 1,
                EventKind::ImportItem => 2,
                EventKind::QuestionnaireAnswer => 3,
                EventKind::MemoryWrite => 4,
                EventKind::ForgetExecute => 5,
            },
            match event.actor_subject {
                ActorSubject::Owner => 0,
                ActorSubject::ThirdParty => 1,
                ActorSubject::System => 2,
            },
        ]);
        entry.flags.push(event.consent_id.is_some());
        if let Some(id) = event.consent_id {
            entry.uuids.push(id);
        }
        privacy(&mut entry, Some(&event.privacy));
        entry.flags.push(event.body_ref.is_some());
        if let Some(sealed) = event.body_ref {
            if sealed.aad.as_deref() != Some(field_aad(&event.event_id, "body_ref").as_str())
                || !store.has_content_key(sealed.content_key_id)
                || store.import_key_destroyed(sealed.content_key_id).ok()?
            {
                return None;
            }
            // Reading is only a liveness check. No text, content hash, length
            // derived from plaintext, or reversible body fingerprint is saved.
            drop(store.open(&sealed).ok()?);
            entry.uuids.extend([sealed.content_key_id, sealed.blob_id]);
            entry.values.push(match sealed.subject {
                SealedSubject::Owner => 0,
                SealedSubject::ThirdParty => 1,
                SealedSubject::Mixed => 2,
            });
            entry.count(sealed.char_count)?;
            entry.flags.push(true);
        }
        d.append(entry);
        if !d.fits() {
            return None;
        }
    }
    Some(d)
}

fn band(value: SupportedBand) -> i64 {
    match value {
        SupportedBand::Weak => 1,
        SupportedBand::Moderate => 2,
        SupportedBand::Strong => 3,
    }
}

fn optional_band(value: Option<SupportedBand>) -> i64 {
    value.map(band).unwrap_or(0)
}

fn subject(value: Subject) -> i64 {
    match value {
        Subject::Owner => 0,
        Subject::ThirdParty => 1,
        Subject::Mixed => 2,
        Subject::System => 3,
    }
}

fn privacy(d: &mut Dependencies, value: Option<&Privacy>) {
    d.flags.push(value.is_some());
    let Some(p) = value else {
        return;
    };
    d.values.extend([
        subject(p.subject),
        match p.derivation {
            Derivation::Raw => 0,
            Derivation::Derived => 1,
            Derivation::Aggregate => 2,
        },
        match p.retention.mode {
            RetentionMode::UntilForgotten => 0,
            RetentionMode::Days => 1,
            RetentionMode::Session => 2,
        },
        match p.egress.e1 {
            E1Disposition::Deny => 0,
            E1Disposition::Placeholder => 1,
            E1Disposition::Allow => 2,
        },
        match p.egress.research_export {
            ResearchDisposition::Deny => 0,
            ResearchDisposition::Hash => 1,
            ResearchDisposition::Bucket => 2,
            ResearchDisposition::Allow => 3,
        },
        p.purposes.len() as i64,
    ]);
    d.flags.push(p.retention.days.is_some());
    if let Some(days) = p.retention.days {
        d.values.push(i64::from(days));
    }
    for purpose in &p.purposes {
        d.values.push(match purpose {
            Purpose::SoulProfile => 0,
            Purpose::AssistantTask => 1,
            Purpose::Graph => 2,
            Purpose::Memory => 3,
            Purpose::Research => 4,
            Purpose::Audit => 5,
            Purpose::Diagnostics => 6,
        });
    }
}
