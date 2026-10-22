use std::time::Instant;

use serde_json::{json, Value};
use soul_policy::e1::{DRAFTING_INSTRUCTION, QUOTE_CLOSE, QUOTE_OPEN};
use soul_store_api::import::{ImportSourceKind, LegacyImportChoice};
use soul_testkit::{fixtures, mock_llm::MockLlm};
use soulcore::commands::draft::E1DraftPlan;
use soulcore::commands::session::{
    AssistanceContextView, AssistanceItemDetail, AssistanceItemRequest, AssistanceSource,
    BindAssistanceGoalSource, CreateAssistanceGoal, LinkAssistanceObservation, Session,
};
use uuid::Uuid;

use super::cards::Scene;
use super::support;

fn pointer(item: &AssistanceItemDetail) -> AssistanceItemRequest {
    AssistanceItemRequest {
        item_id: item.item.item_id,
        expected_revision: item.item.revision,
    }
}

fn observation(session: &mut Session) -> AssistanceItemDetail {
    // Existing synthetic import fixture, never a user export.
    let text = fixtures::read_text("import/soul-import-v1/valid_basic.jsonl")
        .expect("repository synthetic fixture");
    let work = session
        .begin_import_prepare(
            text,
            ImportSourceKind::SoulImportV1,
            None,
            LegacyImportChoice::Reject,
        )
        .expect("prepare synthetic import");
    let prepared = session
        .complete_import_prepare(work.execute())
        .expect("prepared import");
    let work = session
        .begin_import(
            &prepared.operation.operation_id.to_string(),
            &prepared.operation.preview_id.to_string(),
        )
        .expect("confirm synthetic import");
    session
        .complete_import(work.execute())
        .expect("commit synthetic import");
    let bundle = session
        .local_insights()
        .expect("local candidates")
        .remove(0);
    session
        .link_assistance_observation(LinkAssistanceObservation {
            bundle_id: bundle.bundle_id,
            contact_id: Uuid::parse_str(&bundle.summaries[0].contact_id).expect("fixture contact"),
        })
        .expect("explicitly link fixture observation")
}

fn capture(session: &mut Session, endpoint: &MockLlm, plan: &E1DraftPlan, preview: &str) -> String {
    let before = endpoint.request_count();
    let approval = plan.approval();
    let work = session.begin_draft(&approval).expect("exact-plan approval");
    session
        .complete_draft(work.execute())
        .expect("fixed loopback completion");
    assert_eq!(endpoint.request_count(), before + 1);
    assert!(session.begin_draft(&approval).is_err());
    assert_eq!(endpoint.request_count(), before + 1);
    let requests = endpoint.requests();
    let body: Value = serde_json::from_str(&requests[before].body).expect("captured JSON");
    let messages = body["messages"].as_array().expect("actual messages");
    assert_eq!(messages.len(), 2);
    assert!(messages[0]["role"] == "system");
    assert!(messages[0]["content"] == DRAFTING_INSTRUCTION);
    assert!(messages[1]["role"] == "user");
    let material = messages[1]["content"]
        .as_str()
        .expect("actual user material");
    assert!(material == format!("{QUOTE_OPEN}\n{preview}\n{QUOTE_CLOSE}"));
    material.to_owned()
}

pub fn observe(scene: &Scene) -> (Value, String) {
    let (_dir, mut session) = support::setup();
    let observation = observation(&mut session);
    let endpoint = MockLlm::start().expect("controlled loopback mock");
    endpoint.set_reply("合成固定回执，不作语义评价。");
    session
        .set_user_endpoint(&endpoint.base_url())
        .expect("explicit loopback endpoint");
    let profile = session
        .set_voice("register", "formal")
        .expect("explicit voice");
    let goal_text = format!("{}\n{}", scene.goal, scene.forbidden);
    let goal = session
        .create_assistance_goal(CreateAssistanceGoal {
            title: "合成沟通目标".into(),
            summary: goal_text.clone(),
            deadline_at: None,
            source_observation: None,
        })
        .expect("explicit synthetic goal");
    let goal_id = match &goal.item.source {
        AssistanceSource::ManualGoal { memory_id, .. } => *memory_id,
        _ => panic!("explicit goal must own a commitment memory"),
    };

    // This is the ordinary local draft path, NOT automatic local preparation.
    let plain = session
        .draft_pasted(scene.paste)
        .expect("ordinary local template");
    let AssistanceContextView::Memory { memory, .. } = session
        .prepare_assistance_context(pointer(&goal))
        .expect("goal memory context")
    else {
        panic!("unbound goal is a memory context");
    };
    assert_eq!(memory.preview.selected.len(), 1);
    assert!(memory.preview.selected[0].memory_id == goal_id);
    assert!(memory.preview.selected[0].summary == goal_text);
    let local_started = Instant::now();
    let local = session
        .draft_pasted_with_memory(
            scene.paste,
            &memory.context_id.expect("local grant").to_string(),
        )
        .expect("local draft with goal reference");
    let local_ms = local_started.elapsed().as_secs_f64() * 1000.0;
    assert!(local.draft.text == plain.text);
    assert!(local.context.selected[0].memory_id == goal_id);
    assert_eq!(endpoint.request_count(), 0);

    let memory_started = Instant::now();
    let AssistanceContextView::Memory { memory, .. } = session
        .prepare_assistance_context(pointer(&goal))
        .expect("fresh goal context")
    else {
        panic!("unbound goal is a memory context");
    };
    let memory_plan = session
        .prepare_draft_with_memory(
            scene.paste,
            &memory.context_id.expect("memory grant").to_string(),
            &[goal_id],
            false,
        )
        .expect("explicitly permit only the synthetic owner goal excerpt");
    let memory_prepare_ms = memory_started.elapsed().as_secs_f64() * 1000.0;
    assert!(memory_plan.prepared.endpoint_owner_ids == [goal_id]);
    assert!(memory_plan.prepared.retained_memory_ids == [goal_id]);
    assert_eq!(endpoint.request_count(), 0);
    let material = capture(
        &mut session,
        &endpoint,
        &memory_plan.prepared.plan,
        &memory_plan.prepared.body_preview,
    );
    let (profile_section, memory_section) = material
        .split_once("本次明确提供的记忆参考（保存的记录，未经事实或当前性核验）：")
        .expect("labelled owner memory section");
    assert!(!profile_section.contains(scene.goal));
    assert!(!profile_section.contains(scene.forbidden));
    assert!(memory_section.contains(scene.goal));
    assert!(memory_section.contains(scene.forbidden));
    assert!(!material.contains(scene.paste));

    // Bind the same goal to an already-existing observation; don't edit its text.
    let bound = session
        .bind_assistance_goal_source(BindAssistanceGoalSource {
            item_id: goal.item.item_id,
            expected_revision: goal.item.revision,
            source_observation: Some(pointer(&observation)),
        })
        .expect("bind existing synthetic observation");
    assert!(bound.goal.as_ref().expect("goal remains").summary == goal_text);
    assert!(bound.item.item_id == goal.item.item_id);
    assert!(session.profile().expect("same profile").voice == profile.voice);
    let insight_started = Instant::now();
    let AssistanceContextView::Insight { insight, .. } = session
        .prepare_assistance_context(pointer(&bound))
        .expect("bound observation context")
    else {
        panic!("bound goal selects insight context");
    };
    let insight_plan = session
        .prepare_draft_with_insight(scene.paste, &insight.context_id.to_string(), true, false)
        .expect("explicitly permit current derived counts, not historical text");
    let insight_prepare_ms = insight_started.elapsed().as_secs_f64() * 1000.0;
    assert!(insight_plan.prepared.includes_counts);
    assert_eq!(endpoint.request_count(), 1);
    let material = capture(
        &mut session,
        &endpoint,
        &insight_plan.prepared.plan,
        &insight_plan.prepared.body_preview,
    );
    assert!(!material.contains(scene.goal));
    assert!(!material.contains(scene.forbidden));
    assert!(!material.contains(scene.paste));
    assert!(!insight.reference.summary.points.is_empty());
    for point in &insight.reference.summary.points {
        assert!(material.contains(&point.statement));
    }
    for source in &insight.reference.sources {
        assert!(!material.contains(&source.evidence_id.to_string()));
    }
    assert_eq!(endpoint.request_count(), 2);
    assert!(session.profile().expect("unchanged profile").voice == profile.voice);
    assert!(
        session
            .assistance_item(&bound.item.item_id.to_string())
            .expect("current goal")
            .goal
            .expect("goal retained locally")
            .summary
            == goal_text
    );

    (
        json!({
            "scene": scene.label,
            "same_goal_paste_and_voice": true,
            "paste_body_placeheld_in_both": true,
            "unbound": {
                "context_kind": "memory",
                "permission": "explicit_owner_goal_excerpt",
                "goal_and_constraints_in_memory_data_section": true,
                "prepare_ms": memory_prepare_ms
            },
            "bound": {
                "context_kind": "insight",
                "permission": "explicit_derived_counts",
                "goal_and_constraints_in_request": false,
                "all_selected_statements_in_request": true,
                "goal_still_present_locally": true,
                "prepare_ms": insight_prepare_ms
            },
            "loopback_requests": 2,
            "replayed_approval_requests": 0,
            "permission_shapes_equal": false,
            "fair_generation_quality_comparison": false,
            "ordinary_local_memory_draft": {
                "goal_reference_retained": true,
                "text_equals_plain_template": true,
                "automatic_preparation_exercised": false,
                "elapsed_ms": local_ms
            },
            "model_intent_preservation": null,
            "relationship_understanding": null
        }),
        plain.text,
    )
}
