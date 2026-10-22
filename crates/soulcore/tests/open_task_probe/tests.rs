use super::{data::Action, replay, schema::*};

fn input() -> RawInput {
    serde_json::from_value(serde_json::json!({
        "case_id":"open_a", "task_text":"请核对本次会议时间", "sources":[
            {"id":"s1","title":"会议","text":"会议周五举行。","readable":true},
            {"id":"s2","title":"更正","text":"本次更正 s1，会议周六举行。","readable":true}
        ]
    }))
    .unwrap()
}

fn prediction() -> serde_json::Value {
    serde_json::json!({"case_id":"open_a","needs":[{
        "id":"meeting_time","description":"会议时间","task_quote":"会议时间",
        "status":"supported","support_groups":[["s2"]],"conflict_ids":[]
    }],"source_metadata":[
        {"source_id":"s1","subject":"会议","scope":null,"time":"周五","supersedes":[],"correction_quote":null},
        {"source_id":"s2","subject":null,"scope":null,"time":"周六","supersedes":["s1"],"correction_quote":"本次更正 s1"}
    ]})
}

fn parse(value: serde_json::Value, raw: &RawInput) -> Result<OpenPrediction, &'static str> {
    parse_prediction(&serde_json::to_vec(&value).unwrap(), raw)
}

#[test]
fn rejects_hallucinated_metadata_excerpt() {
    let mut value = prediction();
    value["source_metadata"][0]["subject"] = "不存在的人物".into();
    assert!(parse(value, &input()).is_err());
}

#[test]
fn rejects_wrong_self_unknown_and_duplicate_corrections() {
    for (ids, quote) in [
        (serde_json::json!(["s1"]), "没有引用编号"),
        (serde_json::json!(["s2"]), "本次更正 s1"),
        (serde_json::json!(["absent"]), "本次更正 s1"),
        (serde_json::json!(["s1", "s1"]), "本次更正 s1"),
        (serde_json::json!([]), "本次更正 s1"),
    ] {
        let mut value = prediction();
        value["source_metadata"][1]["supersedes"] = ids;
        value["source_metadata"][1]["correction_quote"] = quote.into();
        assert!(parse(value, &input()).is_err());
    }
}

#[test]
fn rejects_unreadable_support_and_missing_metadata_nulls() {
    let mut raw = input();
    raw.sources[1].readable = false;
    assert!(parse(prediction(), &raw).is_err());
    let mut value = prediction();
    value["source_metadata"][0]
        .as_object_mut()
        .unwrap()
        .remove("scope");
    assert!(parse(value, &input()).is_err());
}

#[test]
fn rejects_task_quote_outside_original_task() {
    let mut value = prediction();
    value["needs"][0]["task_quote"] = "新的额外要求".into();
    assert!(parse(value, &input()).is_err());
}

#[test]
fn zero_needs_is_ready_without_preparation_or_placeholder() {
    let raw = input();
    let mut value = prediction();
    value["needs"] = serde_json::json!([]);
    let parsed = parse(value, &raw).unwrap();
    let projected = project(&raw, Some(&parsed)).unwrap();
    assert!(projected.information_options.is_empty());
    let rows = replay::conditions(&raw, Ok(&parsed));
    assert_eq!(rows.len(), 3);
    let outcome = &rows[0].outcome;
    assert!(outcome.available);
    assert_eq!(outcome.action, Action::Ready);
    assert!(!outcome.attempted_prepare);
    assert_eq!(outcome.reason, "no_required_information");
}

#[test]
fn raw_body_and_machine_metadata_survive_real_session_preparation() {
    let raw = input();
    let parsed = parse(prediction(), &raw).unwrap();
    let projected = project(&raw, Some(&parsed)).unwrap();
    assert!(projected.source_metadata.is_empty());
    assert!(projected.candidates[1]
        .summary
        .starts_with(&raw.sources[1].text));
    assert!(projected.candidates[1].summary.contains("机器提取元数据"));
    let rows = replay::conditions(&raw, Ok(&parsed));
    assert!(rows[0].outcome.available);
    assert_eq!(rows[0].outcome.retained_ids.as_ref().unwrap(), &["s2"]);
    assert_eq!(rows[0].outcome.network_requests, 0);
    // The reused adapter asserts exact saved summary and redacted body inclusion,
    // in addition to citation identities. It never executes a generated response.
    assert_eq!(
        rows[0].outcome.preparation.as_ref().unwrap()["executed"],
        false
    );
}

#[test]
fn prediction_failure_preserves_denominator_and_raw_fixed_controls() {
    let rows = replay::conditions(&input(), Err("prediction_shape"));
    assert_eq!(
        rows.iter().map(|row| row.method).collect::<Vec<_>>(),
        ["predicted_required", "fixed4", "fixed8"]
    );
    assert!(!rows[0].outcome.available);
    assert_eq!(rows[0].outcome.reason, "prediction_shape");
    for row in &rows[1..] {
        assert!(row.outcome.available);
        assert!(row.outcome.attempted_prepare);
        assert!(!row.outcome.diagnosis_method);
        assert_eq!(row.outcome.network_requests, 0);
    }
}

#[test]
fn fixed_controls_reject_invalid_unavailable_ids() {
    let raw = input();
    let mut projected = project(&raw, None).unwrap();
    projected.unavailable_ids.push("not_in_pool".into());
    assert_eq!(
        super::diagnosis::validate_fixed_input(&projected),
        Err("input_unavailable_ids")
    );
    assert!(super::diagnosis::validate_input(&project(&raw, None).unwrap()).is_err());
}

#[test]
fn rejects_unknown_fields_duplicate_metadata_and_invalid_status_groups() {
    let raw = input();
    let mut unknown = prediction();
    unknown["gold"] = serde_json::json!({});
    assert!(parse(unknown, &raw).is_err());
    let mut duplicate = prediction();
    duplicate["source_metadata"][1] = duplicate["source_metadata"][0].clone();
    assert!(parse(duplicate, &raw).is_err());
    let mut invalid_group = prediction();
    invalid_group["needs"][0]["status"] = "missing".into();
    assert!(parse(invalid_group, &raw).is_err());
    let mut invalid_id = prediction();
    invalid_id["needs"][0]["support_groups"] = serde_json::json!([["absent"]]);
    assert!(parse(invalid_id, &raw).is_err());
}

#[test]
fn nullable_metadata_and_substring_quotes_do_not_claim_semantic_truth() {
    let mut value = prediction();
    value["source_metadata"][0]["subject"] = serde_json::Value::Null;
    value["source_metadata"][0]["time"] = serde_json::Value::Null;
    // A task substring proves origin only: this deliberately vague quote remains valid.
    value["needs"][0]["task_quote"] = "请".into();
    assert!(parse(value, &input()).is_ok());
}

fn write_artifacts(dir: &std::path::Path, raw: &RawInput) {
    let encoded = serde_json::to_vec(raw).unwrap();
    std::fs::write(dir.join("input.jsonl"), &encoded).unwrap();
    std::fs::write(dir.join(format!("input-{}.json", raw.case_id)), &encoded).unwrap();
}

fn read_results(dir: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(dir.join("results.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn artifact_replay_preserves_parse_failure_and_never_overwrites_results() {
    let dir = tempfile::tempdir().unwrap();
    let raw = input();
    write_artifacts(dir.path(), &raw);
    std::fs::write(dir.path().join("prediction-open_a.json"), b"{broken json").unwrap();
    assert_eq!(replay::run_directory(dir.path()), Ok(3));
    let rows = read_results(dir.path());
    assert_eq!(rows[0]["prediction_error"], "prediction_shape");
    assert_eq!(rows[0]["raw_prediction"], "{broken json");
    assert_eq!(rows[0]["outcome"]["available"], false);
    assert_eq!(rows[1]["outcome"]["available"], true);
    assert_eq!(rows[2]["outcome"]["available"], true);
    assert_eq!(replay::run_directory(dir.path()), Err("results_create_new"));
    assert_eq!(read_results(dir.path()).len(), 3);
}

#[test]
fn failure_artifact_keeps_original_prediction_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    let raw = input();
    std::fs::write(dir.path().join("prediction-open_a.json"), b"{broken json").unwrap();
    std::fs::write(
        dir.path().join("failure-open_a.json"),
        b"{\"error\":\"synthetic failure\"}",
    )
    .unwrap();
    let artifacts = super::files::prediction(dir.path(), &raw);
    assert!(matches!(
        artifacts.prediction,
        Err("model_failure_artifact")
    ));
    assert_eq!(artifacts.prediction_parse_error, Some("prediction_shape"));
    assert_eq!(artifacts.raw_prediction.as_deref(), Some("{broken json"));
    assert!(artifacts.failure.unwrap().contains("synthetic failure"));
}

#[test]
fn input_artifact_mismatch_blocks_all_three_conditions() {
    let dir = tempfile::tempdir().unwrap();
    let mut raw = input();
    write_artifacts(dir.path(), &raw);
    raw.task_text.push_str("额外要求");
    std::fs::write(
        dir.path().join("input-open_a.json"),
        serde_json::to_vec(&raw).unwrap(),
    )
    .unwrap();
    assert_eq!(replay::run_directory(dir.path()), Ok(3));
    let rows = read_results(dir.path());
    assert!(rows.iter().all(|row| row["outcome"]["available"] == false
        && row["input_error"] == "exported_input_changed"
        && row["outcome"]["attempted_prepare"] == false));
}

#[test]
fn control_prepared_context_is_allowed_but_no_other_input_fields() {
    let dir = tempfile::tempdir().unwrap();
    let raw = input();
    let mut value = serde_json::to_value(&raw).unwrap();
    value["prepared_context"] = "synthetic controlled context".into();
    std::fs::write(
        dir.path().join("input-open_a.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    assert_eq!(super::files::check_exported_input(dir.path(), &raw), Ok(()));
    value["information_options"] = serde_json::json!([]);
    std::fs::write(
        dir.path().join("input-open_a.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    assert_eq!(
        super::files::check_exported_input(dir.path(), &raw),
        Err("exported_input_shape")
    );
}

#[test]
fn oversized_jsonl_line_and_following_invalid_line_keep_both_denominators() {
    let dir = tempfile::tempdir().unwrap();
    let mut bytes = vec![b'x'; super::policy::MAX_PREDICTION_BYTES + 1];
    bytes.extend_from_slice(b"\n{invalid}\n");
    std::fs::write(dir.path().join("input.jsonl"), bytes).unwrap();
    assert_eq!(replay::run_directory(dir.path()), Ok(6));
    let rows = read_results(dir.path());
    assert_eq!(rows[0]["input_error"], "input_budget");
    assert_eq!(rows[3]["input_error"], "input_shape");
}
