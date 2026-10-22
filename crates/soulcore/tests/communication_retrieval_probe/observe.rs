use std::collections::BTreeSet;
use std::time::Instant;

use serde_json::{json, Value};
use soul_memory::{MemoryPrivacyClass, SelectionLimits};
use soul_testkit::mock_llm::MockLlm;
use soulcore::commands::draft::MAX_MEMORY_REQUEST_BYTES;
use soulcore::commands::memory::NewMemory;
use soulcore::commands::session::{MemoryContextRequest, MemoryContextView, Session};
use uuid::Uuid;

use super::cards::Card;
use super::strategy::{self, Candidate, Strategy};

pub struct Fixture {
    _dir: tempfile::TempDir,
    session: Session,
    endpoint: MockLlm,
    ids: Vec<Uuid>,
    texts: Vec<(String, String)>,
}

impl Fixture {
    pub fn new(card: &Card) -> Self {
        let dir = tempfile::tempdir().expect("temporary synthetic store");
        let mut session = Session::open(dir.path());
        assert!(session.status().store_opened);
        let endpoint = MockLlm::start().expect("unused controlled loopback");
        session
            .set_user_endpoint(&endpoint.base_url())
            .expect("explicit loopback endpoint");
        let ids = card
            .records
            .iter()
            .map(|record| {
                let saved = session
                    .write_memory(&NewMemory {
                        memory_type: "commitment".into(),
                        title: record.title.clone(),
                        summary: record.summary.clone(),
                    })
                    .expect("synthetic owner record");
                Uuid::parse_str(&saved.memory_id).expect("stored identity")
            })
            .collect();
        let texts = card
            .records
            .iter()
            .map(|record| (record.title.clone(), record.summary.clone()))
            .collect();
        Self {
            _dir: dir,
            session,
            endpoint,
            ids,
            texts,
        }
    }

    pub fn observe(&mut self, card: &Card) -> Value {
        assert_eq!(self.ids.len(), card.records.len());
        assert!(
            self.texts
                .iter()
                .zip(&card.records)
                .all(|((title, summary), record)| *title == record.title
                    && *summary == record.summary)
        );
        let pool: BTreeSet<_> = self.ids.iter().copied().collect();
        assert_eq!(pool.len(), self.ids.len());
        assert_eq!(
            card.order.iter().copied().collect::<BTreeSet<_>>(),
            (0..self.ids.len()).collect()
        );
        assert_eq!(
            card.required.iter().copied().collect::<BTreeSet<_>>().len(),
            2
        );
        assert!(card.required.iter().all(|index| *index < self.ids.len()));
        assert!(card
            .update
            .is_none_or(|index| card.required.contains(&index)));
        let candidates: Vec<_> = card
            .order
            .iter()
            .map(|index| Candidate {
                id: self.ids[*index],
                title: &card.records[*index].title,
                summary: &card.records[*index].summary,
            })
            .collect();
        let input: Vec<_> = candidates.iter().map(|candidate| candidate.id).collect();
        for strategy in [Strategy::SoftRank, Strategy::Interleave] {
            assert!(strategy::order(&candidates, None, strategy) == input);
        }
        let mut conditions = Vec::new();
        for mode in [
            "manual_required",
            "current_hard_query",
            "experimental_soft_rank",
            "experimental_interleave",
        ] {
            let started = Instant::now();
            let (ordered, query) = match mode {
                "manual_required" => (
                    card.order
                        .iter()
                        .filter(|index| card.required.contains(index))
                        .map(|index| self.ids[*index])
                        .collect(),
                    None,
                ),
                "current_hard_query" => (input.clone(), Some(card.query)),
                "experimental_soft_rank" => (
                    strategy::order(&candidates, Some(card.query), Strategy::SoftRank),
                    None,
                ),
                "experimental_interleave" => (
                    strategy::order(&candidates, Some(card.query), Strategy::Interleave),
                    None,
                ),
                _ => unreachable!(),
            };
            let ordering_ms = started.elapsed().as_secs_f64() * 1000.0;
            let ordered_set: BTreeSet<_> = ordered.iter().copied().collect();
            assert_eq!(ordered_set.len(), ordered.len());
            assert!(ordered_set.is_subset(&pool));
            if mode != "manual_required" {
                assert!(ordered_set == pool);
            }
            let started = Instant::now();
            let context = self
                .session
                .prepare_memory_context(MemoryContextRequest {
                    candidate_ids: ordered.clone(),
                    query: query.map(str::to_owned),
                    memory_type: None,
                    time_specific: false,
                })
                .expect("bounded real selection");
            let selection_ms = started.elapsed().as_secs_f64() * 1000.0;
            let selected: Vec<_> = context
                .preview
                .selected
                .iter()
                .map(|entry| {
                    let index = self
                        .ids
                        .iter()
                        .position(|id| *id == entry.memory_id)
                        .expect("selected identity belongs to pool");
                    assert!(ordered_set.contains(&entry.memory_id));
                    assert!(entry.citation.memory_id == entry.memory_id);
                    assert!(entry.title == card.records[index].title);
                    assert!(entry.summary == card.records[index].summary);
                    assert!(entry.privacy_class == MemoryPrivacyClass::Owner);
                    index
                })
                .collect();
            assert_eq!(
                selected.len(),
                selected.iter().copied().collect::<BTreeSet<_>>().len()
            );
            let selected_json_bytes = serde_json::to_vec(&context.preview.selected)
                .expect("selected preview encoding")
                .len();
            assert!(selected.len() <= SelectionLimits::default().max_selected);
            assert!(selected_json_bytes <= SelectionLimits::default().max_context_bytes);
            let required_selected = selected
                .iter()
                .filter(|index| card.required.contains(index))
                .count();
            if mode == "manual_required" {
                assert_eq!(required_selected, 2);
            }
            let preparation = self.prepare(card, &context, &selected);
            let excluded: Vec<_> = context
                .preview
                .excluded
                .iter()
                .map(|entry| {
                    let index = self
                        .ids
                        .iter()
                        .position(|id| *id == entry.memory_id)
                        .expect("excluded identity belongs to pool");
                    json!({ "pool_index": index, "reason": entry.reason })
                })
                .collect();
            conditions.push(json!({
                "mode": mode,
                "submitted_pool_indices": ordered.iter().map(|id| self.ids.iter().position(|candidate| candidate == id).expect("same pool")).collect::<Vec<_>>(),
                "selected_pool_indices": selected,
                "excluded": excluded,
                "required_available": 2,
                "required_selected": required_selected,
                "required_coverage": required_selected as f64 / 2.0,
                "update_available": usize::from(card.update.is_some()),
                "update_selected": card.update.map(|index| selected.contains(&index)),
                "noise_selected": selected.len() - required_selected,
                "noise_original_title_summary_utf8_bytes": selected.iter().filter(|index| !card.required.contains(index)).map(|index| card.records[*index].title.len() + card.records[*index].summary.len()).sum::<usize>(),
                "selected_preview_json_bytes": selected_json_bytes,
                "outcome": context.preview.outcome,
                "context_available": context.context_id.is_some(),
                "ordering_ms": ordering_ms,
                "selection_ms": selection_ms,
                "preparation": preparation,
                "network_requests": self.endpoint.request_count()
            }));
        }
        assert_eq!(self.endpoint.request_count(), 0);
        json!({
            "scene": card.scene,
            "variant": card.variant,
            "required_position": card.position,
            "pool_size": self.ids.len(),
            "input_pool_indices": card.order,
            "required_pool_indices": card.required,
            "update_pool_index": card.update,
            "same_pool_ids_and_content_across_positions": true,
            "fact_time_scope": "unspecified",
            "truth_labels": "hand_authored_fixture_only",
            "conditions": conditions
        })
    }

    fn prepare(&mut self, card: &Card, context: &MemoryContextView, selected: &[usize]) -> Value {
        assert_eq!(self.endpoint.request_count(), 0);
        let Some(context_id) = context.context_id else {
            assert!(selected.is_empty());
            return Value::Null;
        };
        let ids: Vec<_> = context
            .preview
            .selected
            .iter()
            .map(|entry| entry.memory_id)
            .collect();
        let expected: BTreeSet<_> = ids.iter().copied().collect();
        let started = Instant::now();
        let plan = self
            .session
            .prepare_draft_with_memory("合成待回复消息", &context_id.to_string(), &ids, false)
            .expect("explicit owner excerpt preparation only");
        let prepare_ms = started.elapsed().as_secs_f64() * 1000.0;
        let retained: BTreeSet<_> = plan.prepared.retained_memory_ids.iter().copied().collect();
        let permitted: BTreeSet<_> = plan.prepared.endpoint_owner_ids.iter().copied().collect();
        assert_eq!(retained.len(), plan.prepared.retained_memory_ids.len());
        assert_eq!(permitted.len(), plan.prepared.endpoint_owner_ids.len());
        assert!(retained == expected);
        assert!(permitted == expected);
        let (_, material) = plan
            .prepared
            .body_preview
            .split_once("本次明确提供的记忆参考（保存的记录，未经事实或当前性核验）：")
            .expect("memory data section");
        let all_selected_present = context
            .preview
            .selected
            .iter()
            .all(|entry| material.contains(&entry.summary));
        assert!(all_selected_present);
        assert!(!plan.prepared.body_preview.contains("合成待回复消息"));
        assert!(plan.prepared.encoded_bytes <= MAX_MEMORY_REQUEST_BYTES);
        let required_selected = selected
            .iter()
            .filter(|index| card.required.contains(index))
            .count();
        let required_retained = card
            .required
            .iter()
            .filter(|index| retained.contains(&self.ids[**index]))
            .count();
        assert_eq!(self.endpoint.request_count(), 0);
        json!({
            "retained_pool_indices": self.ids.iter().enumerate().filter(|(_, id)| retained.contains(*id)).map(|(index, _)| index).collect::<Vec<_>>(),
            "selected_count": ids.len(),
            "retained_count": retained.len(),
            "explicit_owner_permission_matches_selection": permitted == expected,
            "all_selected_summaries_in_memory_data_section": all_selected_present,
            "required_retained": required_retained,
            "required_trimmed_after_selection": required_selected - required_retained,
            "required_missing_before_preparation": 2 - required_selected,
            "encoded_bytes": plan.prepared.encoded_bytes,
            "prepare_ms": prepare_ms,
            "approved": false,
            "executed": false,
            "network_requests": self.endpoint.request_count()
        })
    }
}
