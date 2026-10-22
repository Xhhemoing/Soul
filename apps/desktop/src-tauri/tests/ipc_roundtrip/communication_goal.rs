//! Synthetic goal + evidence over the production Tauri handler, loopback only.
use super::*;

const GOAL: &str = "SYNTHETIC_JOINT_GOAL_FRIDAY_ONLY";

fn setup() -> (Shell, MockLlm, Value, Value) {
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply("Synthetic draft for review");
    let shell = Shell::on(scratch());
    shell
        .invoke("set_user_endpoint", json!({"url": endpoint.base_url()}))
        .unwrap();
    shell
        .import(
            "soul-import-v1",
            &fixture("import/soul-import-v1/valid_basic.jsonl"),
        )
        .unwrap();
    let bundles = shell.invoke("local_insights", json!({})).unwrap();
    let bundle = &bundles[0];
    let request = json!({"bundle_id": bundle["bundle_id"],
        "contact_id": bundle["summaries"][0]["contact_id"]});
    let observation = shell
        .invoke("link_assistance_observation", json!({"request": request}))
        .unwrap();
    let goal = shell
        .invoke(
            "create_assistance_goal",
            json!({"request": {"title": "Synthetic coordination goal", "summary": GOAL,
            "deadline_at": null, "source_observation": {
                "item_id": observation["item"]["item_id"],
                "expected_revision": observation["item"]["revision"]
            }}}),
        )
        .unwrap();
    (shell, endpoint, goal["item"].clone(), request)
}

fn context(shell: &Shell, item: &Value) -> Value {
    let value = shell
        .invoke(
            "prepare_assistance_context",
            json!({"request": {"item_id": item["item_id"],
                "expected_revision": item["revision"]}}),
        )
        .unwrap();
    assert_eq!(value["kind"], "insight");
    assert_eq!(value["goal_preview"]["selected"][0]["summary"], GOAL);
    value
}

fn approval(plan: &Value) -> Value {
    json!({"approval": {"preparation_id": plan["prepared"]["plan"]["preparation_id"],
        "plan_hash": plan["prepared"]["plan"]["plan_hash"]}})
}

#[test]
fn communication_goal_optional_wire_and_independent_permissions_match_actual_body() {
    let (shell, endpoint, item, _) = setup();
    // Omitted and explicit null both preserve compatibility and default denial.
    for (index, (goal_flag, counts, expected_goal)) in [
        (None, false, false),
        (Some(Value::Null), false, false),
        (Some(json!(false)), true, false),
        (Some(json!(true)), false, true),
        (Some(json!(true)), true, true),
    ]
    .into_iter()
    .enumerate()
    {
        let selected = context(&shell, &item);
        let mut body = json!({"pasted": "SYNTHETIC_PRIVATE_PASTE", "contextId": selected["insight"]["context_id"],
            "includeCounts": counts, "includeOriginal": false});
        if let Some(flag) = goal_flag {
            body["includeGoal"] = flag;
        }
        let plan = shell.invoke("prepare_draft_with_insight", body).unwrap();
        assert_eq!(plan["prepared"]["includes_goal"], expected_goal);
        assert_eq!(plan["prepared"]["includes_counts"], counts);
        assert_eq!(plan["goal_preview"].is_null(), !expected_goal);
        assert_eq!(endpoint.request_count(), index);
        let draft = shell.invoke("generate_draft", approval(&plan)).unwrap();
        assert_eq!(draft["delivery"], false);
        assert_eq!(endpoint.request_count(), index + 1);
        let recorded = endpoint.requests().pop().unwrap();
        let wire: Value = serde_json::from_str(&recorded.body).unwrap();
        assert_eq!(
            wire["messages"][1]["content"],
            format!(
                "{}\n{}\n{}",
                soul_policy::e1::QUOTE_OPEN,
                plan["prepared"]["body_preview"].as_str().unwrap(),
                soul_policy::e1::QUOTE_CLOSE
            )
        );
        assert_eq!(recorded.body.contains(GOAL), expected_goal);
        assert_eq!(recorded.body.contains("本机统计"), counts);
        assert!(!recorded.body.contains("SYNTHETIC_PRIVATE_PASTE"));
        assert_eq!(
            recorded.body.len(),
            plan["prepared"]["encoded_bytes"].as_u64().unwrap() as usize
        );
        assert!(shell.invoke("generate_draft", approval(&plan)).is_err());
        assert_eq!(endpoint.request_count(), index + 1);
    }
}

#[test]
fn communication_goal_forged_selection_on_plain_insight_refuses_without_request() {
    let (shell, endpoint, _, request) = setup();
    let selected = shell
        .invoke("prepare_insight_context", json!({"request": request}))
        .unwrap();
    assert!(shell
        .invoke(
            "prepare_draft_with_insight",
            json!({"pasted": "synthetic", "contextId": selected["context_id"],
                "includeCounts": true, "includeOriginal": true, "includeGoal": true}),
        )
        .is_err());
    assert_eq!(endpoint.request_count(), 0);
}

#[test]
fn communication_goal_edit_invalidates_joint_approval_on_real_handler() {
    let (shell, endpoint, item, _) = setup();
    let selected = context(&shell, &item);
    let plan = shell
        .invoke(
            "prepare_draft_with_insight",
            json!({"pasted": "synthetic", "contextId": selected["insight"]["context_id"],
                "includeCounts": true, "includeOriginal": false, "includeGoal": true}),
        )
        .unwrap();
    shell
        .invoke(
            "edit_assistance_goal",
            json!({"request": {"item_id": item["item_id"], "expected_revision": item["revision"],
                "title": "Synthetic revised goal", "summary": "SYNTHETIC_NEW_CONSTRAINT", "deadline_at": null}}),
        )
        .unwrap();
    assert!(shell.invoke("generate_draft", approval(&plan)).is_err());
    assert_eq!(endpoint.request_count(), 0);
}
