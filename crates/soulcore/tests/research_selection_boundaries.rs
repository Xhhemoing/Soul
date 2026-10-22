//! Synthetic Session selection experiments and the official-import time
//! normalization control (soul-graph itself does not depend on soul-import).
//! No endpoint, collector, real user corpus, or claim of semantic model quality.

use soul_memory::{ExclusionReason, SelectionLimits};
use soul_store_api::{BlobStore, MemoryStore};
use soulcore::commands::memory::{ForgetConfirmation, NewMemory};
use soulcore::commands::session::{MemoryContextRequest, Session};
use uuid::Uuid;

fn request(ids: &[Uuid]) -> MemoryContextRequest {
    MemoryContextRequest {
        candidate_ids: ids.to_vec(),
        query: None,
        memory_type: None,
        time_specific: false,
    }
}

#[test]
fn default_selection_limit_order_and_forgetting_change_counterevidence_availability() {
    let dir = tempfile::tempdir().unwrap();
    let mut session = Session::open(dir.path());
    let limit = SelectionLimits::default().max_selected;
    let mut ids = Vec::new();
    for n in 0..=limit {
        let summary = if n == limit {
            "合成反证：场所已经关闭。"
        } else {
            "合成支持材料：场所开放。"
        };
        let saved = session
            .write_memory(&NewMemory {
                memory_type: "preference".into(),
                title: format!("合成记录 {n}"),
                summary: summary.into(),
            })
            .unwrap();
        ids.push(Uuid::parse_str(&saved.memory_id).unwrap());
    }
    let counter = ids[limit];
    let first = session.prepare_memory_context(request(&ids)).unwrap();
    assert_eq!(
        first
            .preview
            .selected
            .iter()
            .map(|row| row.memory_id)
            .collect::<Vec<_>>(),
        ids[..limit]
    );
    assert_eq!(first.preview.excluded.len(), 1);
    assert_eq!(first.preview.excluded[0].memory_id, counter);
    assert_eq!(
        first.preview.excluded[0].reason,
        ExclusionReason::SelectionLimit
    );

    let mut reordered = vec![counter];
    reordered.extend_from_slice(&ids[..limit]);
    let moved = session.prepare_memory_context(request(&reordered)).unwrap();
    assert_eq!(
        moved
            .preview
            .selected
            .iter()
            .map(|row| row.memory_id)
            .collect::<Vec<_>>(),
        reordered[..limit]
    );
    assert_eq!(moved.preview.excluded.len(), 1);
    assert_eq!(moved.preview.excluded[0].memory_id, ids[limit - 1]);
    assert_eq!(
        moved.preview.excluded[0].reason,
        ExclusionReason::SelectionLimit
    );

    // Obtain a fresh original-order context immediately before forgetting;
    // rejection below must not merely be caused by the preceding re-prepare.
    let before_forget = session.prepare_memory_context(request(&ids)).unwrap();
    let old_context = before_forget.context_id.unwrap().to_string();
    let shared = session.store().unwrap();
    let forgotten = shared.lock().unwrap().get_memory(ids[0]).unwrap();
    let old_title = forgotten.title_ref.unwrap();
    let old_summary = forgotten.summary_ref.unwrap();
    assert!(shared.lock().unwrap().open(&old_summary).is_ok());
    let forget = session.preview_forget(&ids[0].to_string()).unwrap();
    session
        .forget_memory(&ForgetConfirmation {
            preview_id: forget.preview_id,
            memory_id: ids[0].to_string(),
        })
        .unwrap();
    assert!(session
        .draft_pasted_with_memory("合成请求", &old_context)
        .is_err());
    {
        let store = shared.lock().unwrap();
        assert!(!store.has_content_key(old_summary.content_key_id));
        assert!(store.open(&old_title).is_err());
        assert!(store.open(&old_summary).is_err());
    }

    let replacement = session.prepare_memory_context(request(&ids)).unwrap();
    assert_eq!(
        replacement
            .preview
            .selected
            .iter()
            .map(|row| row.memory_id)
            .collect::<Vec<_>>(),
        ids[1..]
    );
    assert_eq!(replacement.preview.excluded.len(), 1);
    assert_eq!(replacement.preview.excluded[0].memory_id, ids[0]);
    assert_eq!(
        replacement.preview.excluded[0].reason,
        ExclusionReason::Inactive
    );
    {
        let store = shared.lock().unwrap();
        for selected in &replacement.preview.selected {
            let persisted = store.get_memory(selected.memory_id).unwrap();
            let title = persisted.title_ref.unwrap();
            let summary = persisted.summary_ref.unwrap();
            assert_eq!(selected.citation.memory_id, selected.memory_id);
            assert_eq!(selected.citation.title_blob_id, title.blob_id);
            assert_eq!(selected.citation.summary_blob_id, summary.blob_id);
            assert_eq!(
                String::from_utf8(store.open(&title).unwrap()).unwrap(),
                selected.title
            );
            assert_eq!(
                String::from_utf8(store.open(&summary).unwrap()).unwrap(),
                selected.summary
            );
        }
    }
    let new_context = replacement.context_id.unwrap().to_string();
    let drafted = session
        .draft_pasted_with_memory("合成请求", &new_context)
        .unwrap();
    assert!(drafted
        .context
        .selected
        .iter()
        .any(|row| row.memory_id == counter));
    assert!(drafted
        .context
        .selected
        .iter()
        .all(|row| row.memory_id != ids[0]));
}

#[test]
fn official_import_normalizes_offset_cross_day_evidence_before_graph_rebuild() {
    let canonical = [
        "2024-02-29T23:30:00Z",
        "2024-03-01T00:00:00Z",
        "2024-03-01T00:15:00Z",
    ];
    let mixed = [
        "2024-03-01T00:30:00+01:00",
        "2024-03-01T00:00:00Z",
        "2024-03-01T00:15:00Z",
    ];
    let mut outcomes = Vec::new();
    for times in [canonical, mixed] {
        for reverse in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let mut lines = vec![serde_json::json!({
                "type": "header", "format": "soul-import-v1", "version": 1,
                "exported_at": "2024-03-02T00:00:00Z",
            })
            .to_string()];
            let indices = if reverse {
                vec![2, 1, 0]
            } else {
                vec![0, 1, 2]
            };
            for n in indices {
                lines.push(
                    serde_json::json!({
                        "type": "message", "id": format!("synthetic-{n}"), "occurred_at": times[n],
                        "sender_scope": if n == 0 { "self" } else { "third_party" },
                        "conversation_id": "synthetic-conversation",
                        "sender_id": if n == 0 { "synthetic-owner" } else { "synthetic-peer" },
                        "text": "合成消息",
                    })
                    .to_string(),
                );
            }
            let staged = soul_import::soul_import_v1::parse(&lines.join("\n")).unwrap();
            let mut normalized = staged
                .messages
                .iter()
                .map(|message| message.occurred_at.as_str())
                .collect::<Vec<_>>();
            normalized.sort();
            assert_eq!(normalized, canonical);
            let mut store = soul_store::SqlCipherStore::open(
                dir.path().join("normalized.db"),
                &soul_store::TestKeyProvider::from_seed("synthetic normalized time"),
            )
            .unwrap();
            soul_import::commit::commit(&mut store, &staged).unwrap();
            soul_graph::rebuild(&mut store).unwrap();
            let graph = soul_graph::load(&store).unwrap();
            assert_eq!(graph.edges.len(), 1);
            let edge = &graph.edges[0];
            let strength = &edge.tie_strength;
            assert_eq!(strength.interaction_count, 3);
            assert_eq!(strength.direct_active_day_count, 2);
            assert_eq!(strength.first_contact_utc.as_str(), canonical[0]);
            assert_eq!(strength.last_contact_utc.as_str(), canonical[2]);
            assert_eq!(strength.as_of_utc.as_ref().unwrap().as_str(), canonical[2]);
            assert_eq!(soul_graph::resolve_evidence(&store, edge).unwrap().len(), 3);
            outcomes.push(strength.clone());
        }
    }
    assert!(outcomes.windows(2).all(|pair| pair[0] == pair[1]));
}
