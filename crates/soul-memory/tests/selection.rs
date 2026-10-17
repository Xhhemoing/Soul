//! Selection is a one-draft local capability, never a whole-store search or
//! an egress grant. All prose here is synthetic.

mod common;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use soul_memory::{
    create, forget, revalidate_selection, select_for_draft, update, ExclusionReason, MemoryDraft,
    MemoryEdit, MemoryPrivacyClass, MemorySelectionRequest, MemoryTimeScope, SelectionError,
    SelectionLimits, SelectionOutcome,
};
use soul_schema::common::{SealedSubject, SealedText};
use soul_schema::memory::{ForgetState, MemoryType, SoulMemory};
use soul_store_api::fake::FakeStore;
use soul_store_api::types::{SealRequest, StoreError, StoreResult};
use soul_store_api::{BlobStore, MemoryStore, SoulStore};
use uuid::Uuid;

#[derive(Debug)]
struct ScopedStore {
    inner: FakeStore,
    rows: BTreeMap<Uuid, SoulMemory>,
    unreadable: BTreeSet<Uuid>,
    raw: BTreeMap<Uuid, Vec<u8>>,
    reads: RefCell<Vec<Uuid>>,
    opens: RefCell<Vec<Uuid>>,
}

impl ScopedStore {
    fn new() -> Self {
        Self {
            inner: FakeStore::new(),
            rows: BTreeMap::new(),
            unreadable: BTreeSet::new(),
            raw: BTreeMap::new(),
            reads: RefCell::new(Vec::new()),
            opens: RefCell::new(Vec::new()),
        }
    }

    fn add(&mut self, title: &str, summary: &str) -> SoulMemory {
        create(
            &mut self.inner,
            &MemoryDraft::own(MemoryType::Preference, title, summary),
            common::NOW,
        )
        .expect("create synthetic memory")
    }
}

impl MemoryStore for ScopedStore {
    fn put_memory(&mut self, memory: SoulMemory) -> StoreResult<Uuid> {
        self.inner.put_memory(memory)
    }

    fn get_memory(&self, id: Uuid) -> StoreResult<SoulMemory> {
        self.reads.borrow_mut().push(id);
        self.rows
            .get(&id)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.inner.get_memory(id))
    }

    fn list_memories(&self) -> StoreResult<Vec<SoulMemory>> {
        panic!("selection must never enumerate the whole store")
    }
}

impl BlobStore for ScopedStore {
    fn seal(&mut self, request: SealRequest) -> StoreResult<SealedText> {
        self.inner.seal(request)
    }

    fn open(&self, sealed: &SealedText) -> StoreResult<Vec<u8>> {
        self.opens.borrow_mut().push(sealed.blob_id);
        if self.unreadable.contains(&sealed.blob_id) {
            return Err(StoreError::BlobMissing(sealed.blob_id));
        }
        if let Some(bytes) = self.raw.get(&sealed.blob_id) {
            return Ok(bytes.clone());
        }
        self.inner.open(sealed)
    }

    fn has_content_key(&self, id: Uuid) -> bool {
        self.inner.has_content_key(id)
    }
}

fn request(ids: &[Uuid]) -> MemorySelectionRequest {
    MemorySelectionRequest {
        candidate_ids: ids.to_vec(),
        ..MemorySelectionRequest::default()
    }
}

#[test]
fn only_explicit_ids_are_opened_and_the_citation_names_the_current_sealed_fields() {
    let mut store = ScopedStore::new();
    let visible = store.add("周末安排", "这次想去图书馆。");
    let secret = store.add("不在范围", "范围外的合成秘密。");
    let selection = select_for_draft(
        &store,
        &request(&[visible.memory_id, visible.memory_id]),
        SelectionLimits::default(),
    )
    .unwrap();

    assert_eq!(*store.reads.borrow(), vec![visible.memory_id]);
    assert_eq!(store.opens.borrow().len(), 2);
    assert!(!store
        .opens
        .borrow()
        .contains(&secret.summary_ref.unwrap().blob_id));
    assert_eq!(selection.selected().len(), 1);
    let entry = &selection.selected()[0];
    assert_eq!(entry.title(), "周末安排");
    assert_eq!(entry.privacy_class(), MemoryPrivacyClass::Owner);
    assert_eq!(entry.citation().memory_id, visible.memory_id);
    assert_eq!(
        entry.citation().title_blob_id,
        visible.title_ref.unwrap().blob_id
    );
    assert_eq!(selection.excluded()[0].reason, ExclusionReason::DuplicateId);
    assert_eq!(selection.outcome(), SelectionOutcome::Context);
    assert!(!format!("{selection:?}").contains("图书馆"));
    assert!(!format!("{:?}", selection.preview()).contains("图书馆"));
    assert!(serde_json::to_string(&selection.preview())
        .unwrap()
        .contains("图书馆"));
}

#[test]
fn empty_scope_and_time_specific_queries_do_not_read_or_decrypt_anything() {
    let store = ScopedStore::new();
    let empty = select_for_draft(&store, &request(&[]), SelectionLimits::default()).unwrap();
    assert_eq!(empty.outcome(), SelectionOutcome::Abstain);
    let mut timed = request(&[Uuid::now_v7()]);
    timed.time_scope = MemoryTimeScope::AtTime;
    let selection = select_for_draft(&store, &timed, SelectionLimits::default()).unwrap();
    assert_eq!(selection.outcome(), SelectionOutcome::Clarify);
    assert_eq!(
        selection.excluded()[0].reason,
        ExclusionReason::TemporalScopeUnavailable
    );
    assert!(store.reads.borrow().is_empty());
    assert!(store.opens.borrow().is_empty());
}

#[test]
fn candidate_query_and_invalid_limits_fail_before_any_decryption() {
    let store = ScopedStore::new();
    let mut scoped = request(&vec![Uuid::now_v7(); 33]);
    assert_eq!(
        select_for_draft(&store, &scoped, SelectionLimits::default()),
        Err(SelectionError::TooManyCandidates)
    );
    scoped.candidate_ids.truncate(1);
    scoped.query = Some("秘".repeat(257));
    assert_eq!(
        select_for_draft(&store, &scoped, SelectionLimits::default()),
        Err(SelectionError::QueryTooLong)
    );
    scoped.query = None;
    for limits in [
        SelectionLimits {
            max_selected: 0,
            ..SelectionLimits::default()
        },
        SelectionLimits {
            max_candidates: 1,
            ..SelectionLimits::default()
        },
        SelectionLimits {
            max_context_bytes: 1,
            ..SelectionLimits::default()
        },
    ] {
        assert_eq!(
            select_for_draft(&store, &scoped, limits),
            Err(SelectionError::InvalidLimits)
        );
    }
    assert!(store.reads.borrow().is_empty());
    assert!(store.opens.borrow().is_empty());
    let mut sensitive_query = request(&[]);
    sensitive_query.query = Some("合成秘密查询".to_owned());
    assert!(!format!("{sensitive_query:?}").contains("合成秘密"));
}

#[test]
fn inactive_and_linked_source_records_are_excluded_without_opening_prose() {
    let mut store = ScopedStore::new();
    let mut ids = Vec::new();
    for state in [ForgetState::PendingForget, ForgetState::Forgotten] {
        let mut memory = store.add("旧记录", "这条不应打开。");
        memory.forget_state = state;
        ids.push(memory.memory_id);
        store.rows.insert(memory.memory_id, memory);
    }
    for linked_evidence in [true, false] {
        let mut memory = store.add("有外部链接", "不能猜测 opaque source refs。");
        if linked_evidence {
            memory.evidence_ids = Some(vec![Uuid::now_v7()]);
        } else {
            memory.source_event_ids = Some(vec![Uuid::now_v7()]);
        }
        ids.push(memory.memory_id);
        store.rows.insert(memory.memory_id, memory);
    }
    ids.push(Uuid::now_v7());
    let selection = select_for_draft(&store, &request(&ids), SelectionLimits::default()).unwrap();
    assert!(selection.selected().is_empty());
    assert_eq!(selection.excluded().len(), 5);
    assert_eq!(selection.excluded()[0].reason, ExclusionReason::Inactive);
    assert_eq!(
        selection.excluded()[2].reason,
        ExclusionReason::UnsupportedLinkedSources
    );
    assert_eq!(selection.excluded()[4].reason, ExclusionReason::NotFound);
    assert!(store.opens.borrow().is_empty());
}

#[test]
fn invalid_reference_unreadable_missing_and_bad_utf8_are_distinct_exclusions() {
    let mut store = ScopedStore::new();
    let mut wrong_key = store.add("错密钥", "不可引用。");
    wrong_key.title_ref.as_mut().unwrap().content_key_id = Uuid::now_v7();
    store.rows.insert(wrong_key.memory_id, wrong_key.clone());
    let mut missing = store.add("缺字段", "不可引用。");
    missing.summary_ref = None;
    store.rows.insert(missing.memory_id, missing.clone());
    let unreadable = store.add("缺blob", "不可引用。");
    store
        .unreadable
        .insert(unreadable.summary_ref.as_ref().unwrap().blob_id);
    let invalid_utf8 = store.add("坏编码", "不可引用。");
    store.raw.insert(
        invalid_utf8.summary_ref.as_ref().unwrap().blob_id,
        vec![0xff],
    );
    let selection = select_for_draft(
        &store,
        &request(&[
            wrong_key.memory_id,
            missing.memory_id,
            unreadable.memory_id,
            invalid_utf8.memory_id,
        ]),
        SelectionLimits::default(),
    )
    .unwrap();
    let reasons: Vec<_> = selection
        .excluded()
        .iter()
        .map(|item| item.reason)
        .collect();
    assert_eq!(
        reasons,
        vec![
            ExclusionReason::InvalidSealedReference,
            ExclusionReason::MissingSealedFields,
            ExclusionReason::Unreadable,
            ExclusionReason::NotUtf8,
        ]
    );
    assert!(selection.selected().is_empty());
}

#[test]
fn missing_classification_is_unknown_and_mixed_fields_never_become_owner() {
    let mut store = ScopedStore::new();
    let mut unknown = store.add("未知主体", "不自动允许出网。");
    unknown.third_party_content_present = None;
    store.rows.insert(unknown.memory_id, unknown.clone());
    let mixed = create(
        &mut store.inner,
        &MemoryDraft::own(MemoryType::Episodic, "混合", "第三人内容。").about(SealedSubject::Mixed),
        common::NOW,
    )
    .unwrap();
    let third = create(
        &mut store.inner,
        &MemoryDraft::own(MemoryType::Episodic, "第三人", "第三人内容。")
            .about(SealedSubject::ThirdParty),
        common::NOW,
    )
    .unwrap();
    let selection = select_for_draft(
        &store,
        &request(&[unknown.memory_id, mixed.memory_id, third.memory_id]),
        SelectionLimits::default(),
    )
    .unwrap();
    assert_eq!(
        selection.selected()[0].privacy_class(),
        MemoryPrivacyClass::Unknown
    );
    assert_eq!(
        selection.selected()[1].privacy_class(),
        MemoryPrivacyClass::Mixed
    );
    assert_eq!(
        selection.selected()[2].privacy_class(),
        MemoryPrivacyClass::ThirdParty
    );
}

#[test]
fn keywords_match_only_inside_the_scope_with_stable_title_first_ordering() {
    let mut store = ScopedStore::new();
    let summary_hit = store.add("日程", "Library 图书馆 安排。");
    let title_hit = store.add("LIBRARY 图书馆", "安静学习。");
    let unrelated = store.add("其它", "运动安排。");
    let _outside_scope = store.add("Library 图书馆", "本次不允许读取的反例。");
    let mut scoped = request(&[
        summary_hit.memory_id,
        unrelated.memory_id,
        title_hit.memory_id,
    ]);
    scoped.query = Some("library 图书馆".to_owned());
    let selection = select_for_draft(&store, &scoped, SelectionLimits::default()).unwrap();
    assert_eq!(selection.selected()[0].memory_id(), title_hit.memory_id);
    assert_eq!(selection.selected()[1].memory_id(), summary_hit.memory_id);
    assert_eq!(
        selection.excluded()[0].reason,
        ExclusionReason::KeywordMismatch
    );
    assert_eq!(store.reads.borrow().len(), 3);
    scoped.memory_type = Some(MemoryType::Commitment);
    store.opens.borrow_mut().clear();
    let selection = select_for_draft(&store, &scoped, SelectionLimits::default()).unwrap();
    assert!(selection.selected().is_empty());
    assert!(store.opens.borrow().is_empty());
    assert!(selection
        .excluded()
        .iter()
        .all(|item| item.reason == ExclusionReason::TypeMismatch));
}

#[test]
fn exact_serialized_budget_accounts_for_chinese_escaping_and_skips_whole_entries() {
    let mut store = ScopedStore::new();
    let large = store.add("较长记录", &"中文\n\"\\".repeat(22));
    let small = store.add("短记忆", "完整否定：我不接受这个安排。");
    let only_small = select_for_draft(
        &store,
        &request(&[small.memory_id]),
        SelectionLimits::default(),
    )
    .unwrap();
    let budget = only_small.serialized_context_bytes().unwrap();
    assert_eq!(
        budget,
        serde_json::to_vec(&only_small.preview().selected)
            .unwrap()
            .len()
    );
    let limits = SelectionLimits {
        max_context_bytes: budget,
        max_entry_bytes: budget,
        ..SelectionLimits::default()
    };
    let selection = select_for_draft(
        &store,
        &request(&[large.memory_id, small.memory_id]),
        limits,
    )
    .unwrap();
    assert_eq!(selection.selected().len(), 1);
    assert_eq!(selection.selected()[0].memory_id(), small.memory_id);
    assert_eq!(
        selection.selected()[0].summary(),
        "完整否定：我不接受这个安排。"
    );
    assert!(selection.serialized_context_bytes().unwrap() <= budget);
    assert_eq!(selection.preview().selected.len(), 1);
    assert_eq!(
        selection.excluded()[0].reason,
        ExclusionReason::ContextBudget
    );
}

#[test]
fn per_entry_and_selection_count_limits_preserve_manual_order() {
    let mut store = ScopedStore::new();
    let too_large = store.add("大记录", &"x".repeat(300));
    let first = store.add("第一", "完整记录一。");
    let second = store.add("第二", "完整记录二。");
    let selection = select_for_draft(
        &store,
        &request(&[too_large.memory_id, second.memory_id, first.memory_id]),
        SelectionLimits {
            max_selected: 1,
            max_entry_bytes: 100,
            ..SelectionLimits::default()
        },
    )
    .unwrap();
    assert_eq!(selection.selected()[0].memory_id(), second.memory_id);
    assert_eq!(selection.excluded()[0].reason, ExclusionReason::EntryBudget);
    assert_eq!(
        selection.excluded()[1].reason,
        ExclusionReason::SelectionLimit
    );
    assert!(!store
        .opens
        .borrow()
        .contains(&too_large.title_ref.unwrap().blob_id));
}

#[test]
fn revalidation_rejects_edits_and_key_destruction_in_a_real_reopened_store() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory-selection.db");
    let mut store = common::open(&path);
    let memory = create(
        &mut store,
        &MemoryDraft::own(MemoryType::Preference, "本次安排", "我想去合成图书馆。"),
        common::NOW,
    )
    .unwrap();
    let selection = select_for_draft(
        &store,
        &request(&[memory.memory_id]),
        SelectionLimits::default(),
    )
    .unwrap();
    revalidate_selection(&store, &selection).unwrap();
    store.flush().unwrap();
    store.close().unwrap();
    let mut store = common::open(&path);
    revalidate_selection(&store, &selection).unwrap();
    update(
        &mut store,
        memory.memory_id,
        &MemoryEdit::summary("这次改为在家。"),
        common::NOW,
    )
    .unwrap();
    assert_eq!(
        revalidate_selection(&store, &selection),
        Err(SelectionError::SourceChanged)
    );
    let updated = select_for_draft(
        &store,
        &request(&[memory.memory_id]),
        SelectionLimits::default(),
    )
    .unwrap();
    forget(&mut store, memory.memory_id, common::NOW).unwrap();
    assert_eq!(
        revalidate_selection(&store, &updated),
        Err(SelectionError::SourceChanged)
    );
    let forgotten = select_for_draft(
        &store,
        &request(&[memory.memory_id]),
        SelectionLimits::default(),
    )
    .unwrap();
    assert!(forgotten.selected().is_empty());
    assert_eq!(forgotten.excluded()[0].reason, ExclusionReason::Inactive);
}

#[test]
fn final_request_trimming_updates_the_same_snapshot_and_rejects_scope_expansion() {
    let mut store = ScopedStore::new();
    let first = store.add("一", "第一条。");
    let second = store.add("二", "第二条。");
    let mut selection = select_for_draft(
        &store,
        &request(&[first.memory_id, second.memory_id]),
        SelectionLimits::default(),
    )
    .unwrap();
    let before = selection.clone();
    assert_eq!(
        selection.retain_ids(&[Uuid::now_v7()]),
        Err(SelectionError::InvalidRetainedIds)
    );
    assert_eq!(selection, before);
    selection.retain_ids(&[second.memory_id]).unwrap();
    assert_eq!(selection.selected().len(), 1);
    assert_eq!(selection.preview().selected[0].memory_id, second.memory_id);
    assert_eq!(selection.excluded()[0].memory_id, first.memory_id);
    assert_eq!(
        selection.excluded()[0].reason,
        ExclusionReason::FinalRequestBudget
    );
    selection.retain_ids(&[]).unwrap();
    assert_eq!(selection.outcome(), SelectionOutcome::Abstain);
}
