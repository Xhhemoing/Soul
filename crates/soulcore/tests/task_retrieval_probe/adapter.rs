//! Test-only adapter; does not install a production AssistanceBinding.
use super::types::{Candidate, TaskInput};
use soul_memory::{ContextSelection, MemorySelectionRequest, MemoryTimeScope, SelectionLimits};
use soul_schema::memory::MemoryType;
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::memory::NewMemory;
use soulcore::commands::session::{
    AssistanceHandling, AssistanceItemRow, AssistanceValidity, MemoryContextRequest,
};
use soulcore::commands::session::{
    AssistanceItemDetail, AssistanceSource, CreateAssistanceGoal, Session,
};
use std::collections::BTreeSet;
use std::time::Instant;
use uuid::Uuid;

use super::types::InputCase;

pub struct Harness {
    _dir: tempfile::TempDir,
    pub session: Session,
    pub goal: AssistanceItemDetail,
    pub ids: Vec<Uuid>,
    pub alias_ids: Vec<Uuid>,
    aliases: Vec<Candidate>,
    case: InputCase,
    endpoint: MockLlm,
}

pub struct Snapshot {
    item: AssistanceItemRow,
    goal: ContextSelection,
    candidates: Vec<ContextSelection>,
    aliases: Vec<ContextSelection>,
    pub input: InputCase,
}

pub struct Observation {
    pub selected: Vec<String>,
    pub retained: Vec<String>,
    pub metrics: serde_json::Value,
}

impl Harness {
    pub fn new(case: &InputCase) -> Self {
        Self::new_with_aliases(case, &[])
    }

    pub fn new_with_aliases(case: &InputCase, aliases: &[Candidate]) -> Self {
        super::types::case_bytes(case).expect("bounded visible input");
        assert!(case.candidates.len() + aliases.len() <= SelectionLimits::default().max_candidates);
        let all_ids: BTreeSet<_> = case
            .candidates
            .iter()
            .chain(aliases)
            .map(|entry| &entry.id)
            .collect();
        assert_eq!(all_ids.len(), case.candidates.len() + aliases.len());
        assert!(aliases.iter().all(|alias| case
            .candidates
            .iter()
            .any(|entry| entry.title == alias.title && entry.summary == alias.summary)));
        let dir = tempfile::tempdir().expect("synthetic temporary store");
        let mut session = Session::open(dir.path());
        assert!(session.status().store_opened);
        let endpoint = MockLlm::start().expect("controlled unused loopback");
        session
            .set_user_endpoint(&endpoint.base_url())
            .expect("explicit endpoint");
        let goal = session
            .create_assistance_goal(CreateAssistanceGoal {
                title: case.task.scope.clone(),
                summary: serde_json::to_string(&case.task).unwrap(),
                deadline_at: None,
                source_observation: None,
            })
            .expect("save current task as an explicit manual goal");
        let mut ids: Vec<_> = case
            .candidates
            .iter()
            .chain(aliases)
            .map(|candidate| {
                let record = session
                    .write_memory(&NewMemory {
                        memory_type: "commitment".into(),
                        title: candidate.title.clone(),
                        summary: candidate.summary.clone(),
                    })
                    .expect("save synthetic candidate");
                Uuid::parse_str(&record.memory_id).unwrap()
            })
            .collect();
        let alias_ids = ids.split_off(case.candidates.len());
        Self {
            _dir: dir,
            session,
            goal,
            ids,
            alias_ids,
            aliases: aliases.to_vec(),
            case: case.clone(),
            endpoint,
        }
    }

    pub fn goal_memory_id(&self) -> Uuid {
        match self.goal.item.source {
            AssistanceSource::ManualGoal { memory_id, .. } => memory_id,
            _ => panic!("manual goal"),
        }
    }
    pub fn snapshot(&self) -> Result<Snapshot, &'static str> {
        let detail = self
            .session
            .assistance_item(&self.goal.item.item_id.to_string())
            .map_err(|_| "goal_unavailable")?;
        if detail.item != self.goal.item
            || detail.item.handling != AssistanceHandling::Open
            || detail.item.validity != AssistanceValidity::Current
        {
            return Err("goal_changed");
        }
        let task: TaskInput = serde_json::from_str(&detail.goal.ok_or("goal_missing")?.summary)
            .map_err(|_| "task_shape")?;
        if task != self.case.task {
            return Err("task_changed");
        }
        let shared = self.session.store().ok_or("store_unavailable")?;
        let store = shared.lock().map_err(|_| "store_unavailable")?;
        let select = |id| -> Result<ContextSelection, &'static str> {
            let selection = soul_memory::select_for_draft(
                &*store,
                &MemorySelectionRequest {
                    candidate_ids: vec![id],
                    query: None,
                    memory_type: Some(MemoryType::Commitment),
                    time_scope: MemoryTimeScope::Unspecified,
                },
                SelectionLimits::default(),
            )
            .map_err(|_| "snapshot_failed")?;
            if selection.selected().len() != 1 || selection.selected()[0].memory_id() != id {
                return Err("snapshot_unavailable");
            }
            Ok(selection)
        };
        let goal = select(self.goal_memory_id())?;
        // Snapshot every explicit candidate, including those outside top eight.
        let candidates = self
            .ids
            .iter()
            .map(|id| select(*id))
            .collect::<Result<Vec<_>, _>>()?;
        let aliases = self
            .alias_ids
            .iter()
            .map(|id| select(*id))
            .collect::<Result<Vec<_>, _>>()?;
        for (selection, expected) in aliases.iter().zip(&self.aliases) {
            let preview = selection.preview();
            if preview.selected[0].title != expected.title
                || preview.selected[0].summary != expected.summary
            {
                return Err("alias_changed");
            }
        }
        let input = InputCase {
            case_id: self.case.case_id.clone(),
            task,
            candidates: candidates
                .iter()
                .zip(&self.case.candidates)
                .map(|(selected, original)| {
                    let preview = selected.preview();
                    let entry = &preview.selected[0];
                    Candidate {
                        id: original.id.clone(),
                        title: entry.title.clone(),
                        summary: entry.summary.clone(),
                    }
                })
                .collect(),
        };
        if input != self.case {
            return Err("input_changed");
        }
        Ok(Snapshot {
            item: detail.item,
            goal,
            candidates,
            aliases,
            input,
        })
    }

    pub fn validate(&self, snapshot: &Snapshot) -> Result<(), &'static str> {
        let current = self
            .session
            .assistance_item(&snapshot.item.item_id.to_string())
            .map_err(|_| "goal_unavailable")?;
        if current.item != snapshot.item
            || current.item.handling != AssistanceHandling::Open
            || current.item.validity != AssistanceValidity::Current
        {
            return Err("goal_changed");
        }
        let shared = self.session.store().ok_or("store_unavailable")?;
        let store = shared.lock().map_err(|_| "store_unavailable")?;
        for selected in std::iter::once(&snapshot.goal)
            .chain(&snapshot.candidates)
            .chain(&snapshot.aliases)
        {
            soul_memory::revalidate_selection(&*store, selected).map_err(|_| "snapshot_changed")?;
        }
        Ok(())
    }

    pub fn observe(
        &mut self,
        snapshot: &Snapshot,
        ranked: &[String],
        allow_owner: bool,
    ) -> Result<Observation, &'static str> {
        self.observe_selected_prefix(snapshot, ranked, None, allow_owner, true)
    }

    // Shared by several integration targets; only the stopping probe uses this entry.
    #[allow(dead_code)]
    pub fn observe_prefix(
        &mut self,
        snapshot: &Snapshot,
        full_rank: &[String],
        k: usize,
        allow_owner: bool,
    ) -> Result<Observation, &'static str> {
        self.observe_selected_prefix(snapshot, full_rank, Some(k), allow_owner, true)
    }

    /// Research observation of normal budget losses; invariant violations still panic.
    /// Existing entry points retain their original full-retention assertions.
    #[allow(dead_code)]
    pub fn observe_prefix_allow_loss(
        &mut self,
        snapshot: &Snapshot,
        full_rank: &[String],
        k: usize,
        allow_owner: bool,
    ) -> Result<Observation, &'static str> {
        self.observe_selected_prefix(snapshot, full_rank, Some(k), allow_owner, false)
    }

    fn observe_selected_prefix(
        &mut self,
        snapshot: &Snapshot,
        ranked: &[String],
        prefix: Option<usize>,
        allow_owner: bool,
        require_full_retention: bool,
    ) -> Result<Observation, &'static str> {
        self.validate(snapshot)?;
        let expected: BTreeSet<_> = self.case.candidates.iter().map(|entry| &entry.id).collect();
        if ranked.len() != self.case.candidates.len()
            || ranked.iter().collect::<BTreeSet<_>>() != expected
        {
            return Err("ranking_not_permutation");
        }
        if let Some(k) = prefix {
            if k == 0 || k > SelectionLimits::default().max_selected || k > ranked.len() {
                return Err("prefix_count");
            }
        }
        let submitted = &ranked[..prefix.unwrap_or(ranked.len())];
        let ordered: Vec<_> = submitted
            .iter()
            .map(|id| {
                self.case
                    .candidates
                    .iter()
                    .position(|entry| &entry.id == id)
                    .ok_or("unknown_id")
                    .map(|index| self.ids[index])
            })
            .collect::<Result<_, _>>()?;
        let started = Instant::now();
        let context = self
            .session
            .prepare_memory_context(MemoryContextRequest {
                candidate_ids: ordered,
                query: None,
                memory_type: Some(MemoryType::Commitment),
                time_specific: false,
            })
            .map_err(|_| {
                assert_eq!(self.endpoint.request_count(), 0);
                "selection_failed"
            })?;
        let selection_ms = started.elapsed().as_secs_f64() * 1000.0;
        assert!(context.preview.selected.len() <= SelectionLimits::default().max_selected);
        assert!(
            serde_json::to_vec(&context.preview.selected).unwrap().len()
                <= SelectionLimits::default().max_context_bytes
        );
        let mut selected = Vec::new();
        let mut selected_ids = Vec::new();
        for entry in &context.preview.selected {
            let index = self
                .ids
                .iter()
                .position(|id| *id == entry.memory_id)
                .ok_or("selected_outside_pool")?;
            assert!(entry.citation.memory_id == entry.memory_id);
            let original = snapshot.candidates[index].preview();
            assert!(entry.citation == original.selected[0].citation);
            assert!(
                entry.title == self.case.candidates[index].title
                    && entry.summary == self.case.candidates[index].summary
            );
            selected.push(self.case.candidates[index].id.clone());
            selected_ids.push(entry.memory_id);
        }
        assert_eq!(
            selected_ids.iter().collect::<BTreeSet<_>>().len(),
            selected_ids.len()
        );
        assert!(selected.iter().all(|id| submitted.contains(id)));
        let owner_ids = if allow_owner {
            selected_ids.clone()
        } else {
            Vec::new()
        };
        let started = Instant::now();
        let prepared = self
            .session
            .prepare_draft_with_memory(
                "合成消息等待答复",
                &context.context_id.ok_or("no_context")?.to_string(),
                &owner_ids,
                false,
            )
            .map_err(|_| {
                assert_eq!(self.endpoint.request_count(), 0);
                "preparation_failed"
            })?;
        let preparation_ms = started.elapsed().as_secs_f64() * 1000.0;
        let retained: BTreeSet<_> = prepared
            .prepared
            .retained_memory_ids
            .iter()
            .copied()
            .collect();
        if require_full_retention {
            assert!(retained == selected_ids.iter().copied().collect());
        } else {
            assert!(retained.is_subset(&selected_ids.iter().copied().collect()));
        }
        assert_eq!(retained.len(), prepared.prepared.retained_memory_ids.len());
        let allowed: BTreeSet<_> = prepared
            .prepared
            .endpoint_owner_ids
            .iter()
            .copied()
            .collect();
        if require_full_retention {
            assert!(allowed == owner_ids.iter().copied().collect());
        } else {
            let retained_owners: BTreeSet<_> = owner_ids
                .iter()
                .copied()
                .filter(|id| retained.contains(id))
                .collect();
            assert_eq!(allowed, retained_owners);
        }
        assert_eq!(allowed.len(), prepared.prepared.endpoint_owner_ids.len());
        assert!(self
            .alias_ids
            .iter()
            .all(|id| !allowed.contains(id) && !retained.contains(id)));
        if allow_owner && (require_full_retention || !retained.is_empty()) {
            let shared = self.session.store().ok_or("store_unavailable")?;
            let store = shared.lock().map_err(|_| "store_unavailable")?;
            let identifiers = soulcore::commands::draft::known_identifiers(&store)
                .map_err(|_| "identifiers_unavailable")?;
            let redactor = soul_policy::redactor::Redactor::new(identifiers);
            let (_, data) = prepared
                .prepared
                .body_preview
                .split_once("本次明确提供的记忆参考（保存的记录，未经事实或当前性核验）：")
                .expect("memory data section");
            assert!(context
                .preview
                .selected
                .iter()
                .filter(|entry| require_full_retention || retained.contains(&entry.memory_id))
                .all(|entry| data.contains(&redactor.scrub_identifiers(&entry.summary))));
        } else if !allow_owner {
            assert!(context
                .preview
                .selected
                .iter()
                .all(|entry| !prepared.prepared.body_preview.contains(&entry.summary)));
            assert!(!prepared
                .prepared
                .body_preview
                .contains(&snapshot.input.task.intent));
        }
        assert!(!prepared
            .prepared
            .retained_memory_ids
            .contains(&self.goal_memory_id()));
        assert!(!prepared
            .prepared
            .body_preview
            .contains(&serde_json::to_string(&snapshot.input.task).unwrap()));
        assert!(
            prepared.prepared.encoded_bytes <= soulcore::commands::draft::MAX_MEMORY_REQUEST_BYTES
        );
        assert_eq!(self.endpoint.request_count(), 0);
        let retained_ids = prepared
            .prepared
            .retained_memory_ids
            .iter()
            .map(|id| {
                let index = self
                    .ids
                    .iter()
                    .position(|candidate| candidate == id)
                    .expect("retained identity in pool");
                self.case.candidates[index].id.clone()
            })
            .collect();
        Ok(Observation {
            retained: retained_ids,
            selected,
            metrics: serde_json::json!({
                "requested_k": prefix, "submitted_candidate_count": submitted.len(),
                "selected_count": selected_ids.len(), "retained_count": retained.len(),
                "owner_excerpts_explicitly_allowed": allow_owner, "goal_memory_in_final_request": false,
                "encoded_bytes": prepared.prepared.encoded_bytes, "selection_ms": selection_ms, "preparation_ms": preparation_ms,
                "approved": false, "executed": false, "network_requests": self.endpoint.request_count()
            }),
        })
    }
}
