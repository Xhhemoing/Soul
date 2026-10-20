//! Public IPC trajectory: contact forgetting remains a separate core API.
use super::*;

fn create_goal(shell: &Shell, source: Value, summary: &str) -> Value {
    shell
        .invoke(
            "create_assistance_goal",
            json!({"request": {
                "title": "Synthetic IPC task", "summary": summary,
                "deadline_at": null, "source_observation": source
            }}),
        )
        .unwrap()
}

fn prepare(shell: &Shell, item: &Value) -> Value {
    let context = shell
        .invoke(
            "prepare_assistance_context",
            json!({"request": {
                "item_id": item["item_id"], "expected_revision": item["revision"]
            }}),
        )
        .unwrap();
    assert_eq!(context["kind"], "insight");
    let plan = shell
        .invoke(
            "prepare_draft_with_insight",
            json!({
                "pasted": "SYNTHETIC_IPC_PRIVATE_PASTE", "contextId": context["insight"]["context_id"],
                "includeCounts": false, "includeOriginal": false
            }),
        )
        .unwrap();
    json!({"approval": {
        "preparation_id": plan["prepared"]["plan"]["preparation_id"],
        "plan_hash": plan["prepared"]["plan"]["plan_hash"]
    }})
}

#[test]
fn imported_goal_forget_rejects_pending_approval_and_preserves_independent_data_over_ipc() {
    const GOAL: &str = "SYNTHETIC_IPC_LINKED_GOAL_PRIVATE";
    const OTHER: &str = "SYNTHETIC_IPC_INDEPENDENT_GOAL_PRIVATE";
    const REPLY: &str = "Synthetic endpoint response for review only";
    let endpoint = MockLlm::start().unwrap();
    endpoint.set_reply(REPLY);
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
    let observation = shell
        .invoke(
            "link_assistance_observation",
            json!({"request": {
                "bundle_id": bundle["bundle_id"],
                "contact_id": bundle["summaries"][0]["contact_id"]
            }}),
        )
        .unwrap();
    let linked = create_goal(
        &shell,
        json!({
            "item_id": observation["item"]["item_id"],
            "expected_revision": observation["item"]["revision"]
        }),
        GOAL,
    );
    let independent = create_goal(&shell, Value::Null, OTHER);
    let item = &linked["item"];
    let id = &item["item_id"];
    let memory_id = &item["source"]["memory_id"];
    let positive = prepare(&shell, item);
    assert_eq!(endpoint.request_count(), 0);
    let draft = shell.invoke("generate_draft", positive).unwrap();
    assert_eq!(draft["source"], "user_endpoint");
    assert_eq!(draft["delivery"], false);
    assert_eq!(endpoint.request_count(), 1);
    shell
        .invoke(
            "set_assistance_feedback",
            json!({"request": {
                "item_id": id, "expected_revision": item["revision"],
                "expected_feedback_revision": null, "value": "helpful"
            }}),
        )
        .unwrap();
    assert_eq!(
        shell
            .invoke("assistance_item", json!({"itemId": id}))
            .unwrap()["item"],
        *item,
        "generating and feedback are not completion"
    );

    let unused = prepare(&shell, item);
    let forget = shell
        .invoke("preview_forget", json!({"memoryId": memory_id}))
        .unwrap();
    assert_eq!(forget["destroys_anything"], false);
    shell
        .invoke(
            "forget_memory",
            json!({"confirmation": {
                "preview_id": forget["preview_id"], "memory_id": memory_id
            }}),
        )
        .unwrap();
    let denied = shell.invoke("generate_draft", unused.clone()).unwrap_err();
    assert_eq!(denied["reason_code"], "PLAN_HASH_MISMATCH");
    assert_eq!(endpoint.request_count(), 1);
    let forgotten = shell
        .invoke("assistance_item", json!({"itemId": id}))
        .unwrap();
    assert_eq!(forgotten["item"]["validity"], "forgotten");
    assert!(forgotten["goal"].is_null());
    assert!(shell
        .invoke("memory_detail", json!({"memoryId": memory_id}))
        .is_err());

    let restarted = shell.restart();
    restarted
        .invoke("set_user_endpoint", json!({"url": endpoint.base_url()}))
        .unwrap();
    assert_eq!(
        restarted.invoke("generate_draft", unused).unwrap_err()["reason_code"],
        "PLAN_HASH_MISMATCH"
    );
    let after_restart = restarted
        .invoke("assistance_item", json!({"itemId": id}))
        .unwrap();
    assert_eq!(after_restart["item"], forgotten["item"]);
    assert!(after_restart["goal"].is_null());
    assert!(restarted
        .invoke("memory_detail", json!({"memoryId": memory_id}))
        .is_err());
    let other = restarted
        .invoke(
            "assistance_item",
            json!({"itemId": independent["item"]["item_id"]}),
        )
        .unwrap();
    assert_eq!(other["item"], independent["item"]);
    assert_eq!(other["goal"]["summary"], OTHER);
    let retained = restarted.invoke("local_insights", json!({})).unwrap();
    assert_eq!(retained[0]["bundle_id"], bundle["bundle_id"]);
    assert_eq!(retained[0]["validity"], "active");
    assert_eq!(retained[0]["summaries"], bundle["summaries"]);
    assert_eq!(endpoint.request_count(), 1);
    let chain = restarted.invoke("audit_chain", json!({})).unwrap();
    assert_eq!(chain["verified"], true);
    for prose in [GOAL, OTHER, "SYNTHETIC_IPC_PRIVATE_PASTE", REPLY] {
        assert!(!chain.to_string().contains(prose));
    }
}
