use super::{fixtures, manual};

#[test]
fn manual_metadata_uses_existing_validation_and_summary() {
    let input = serde_json::to_string(&fixtures::fixture()).unwrap();
    let report = manual::report_json(&input).expect("offline metadata must summarize");
    assert_eq!(report["funnel"]["assigned"], 8);
    assert_eq!(report["funnel"]["completed"], 5);
    assert_eq!(report["withdrawn_runs"], 1);
    assert!(report["pairs"].is_array());
    assert!(report["candidates"].is_array());
}

#[test]
fn manual_file_is_read_only_and_returns_same_report() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.json");
    let input = serde_json::to_string(&fixtures::fixture()).unwrap();
    std::fs::write(&path, &input).unwrap();
    let report = manual::summarize_path(&path).expect("explicit metadata file must summarize");
    assert_eq!(report["funnel"]["assigned"], 8);
    assert_eq!(std::fs::read_to_string(path).unwrap(), input);
}

fn error_value(input: &str) -> serde_json::Value {
    serde_json::to_value(manual::report_json(input).unwrap_err()).unwrap()
}

#[test]
fn malformed_unknown_and_missing_fields_are_rejected_without_echoing_input() {
    const SENTINEL: &str = "PRIVATE_TEXT_MUST_NOT_APPEAR";
    let bad_syntax = format!("{{\"{SENTINEL}\":");
    let error = error_value(&bad_syntax);
    assert_eq!(error["category"], "json");
    assert!(!error.to_string().contains(SENTINEL));
    for position in ["root", "candidate", "conditions"] {
        let mut data = serde_json::to_value(fixtures::fixture()).unwrap();
        let object = match position {
            "root" => &mut data,
            "candidate" => &mut data["candidates"][0],
            _ => &mut data["pairs"][0]["conditions"],
        };
        object[SENTINEL] = serde_json::json!(SENTINEL);
        let error = error_value(&data.to_string());
        assert_eq!(error["category"], "json");
        assert_eq!(error["kind"], "data");
        assert!(!error.to_string().contains(SENTINEL));
    }
    for field in [
        "missing_reason",
        "edit_seconds",
        "pause_seconds",
        "delivery",
    ] {
        let mut data = serde_json::to_value(fixtures::fixture()).unwrap();
        data["candidates"][0].as_object_mut().unwrap().remove(field);
        assert_eq!(error_value(&data.to_string())["category"], "json");
    }
    let mut data = serde_json::to_value(fixtures::fixture()).unwrap();
    data["pairs"][0]
        .as_object_mut()
        .unwrap()
        .remove("pair_completed_at");
    assert_eq!(error_value(&data.to_string())["category"], "json");
}

#[test]
fn relationship_errors_do_not_echo_record_ids() {
    let mut data = fixtures::fixture();
    data.candidates[0].pair_id = "PRIVATE_RECORD_ID".into();
    data.candidates[0].candidate_id = "PRIVATE_RECORD_ID".into();
    let error = error_value(&serde_json::to_string(&data).unwrap());
    assert_eq!(error["category"], "validation");
    assert!(error["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|kind| kind == "missing_pair"));
    assert!(!error.to_string().contains("PRIVATE_RECORD_ID"));
}

#[test]
fn non_utf8_and_unavailable_file_report_only_read_category() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("PRIVATE_PATH.json");
    let error = serde_json::to_value(manual::summarize_path(&path).unwrap_err()).unwrap();
    assert_eq!(error, serde_json::json!({"category":"read"}));
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    let error = serde_json::to_value(manual::summarize_path(&path).unwrap_err()).unwrap();
    assert_eq!(error, serde_json::json!({"category":"read"}));
}

#[test]
fn synthetic_manual_example_has_hand_checked_timing_denominators() {
    let report = manual::report_json(include_str!("synthetic_manual.json")).unwrap();
    assert_eq!(report["funnel"]["assigned"], 3);
    assert_eq!(report["funnel"]["completed"], 3);
    let pair = &report["pairs"][0]["summary"];
    assert_eq!(pair["preference"]["target_wins"], 1);
    assert_eq!(pair["preference"]["ties"], 1);
    assert_eq!(pair["preference"]["denominator"], 3);
    assert_eq!(pair["edit_missing_pairs"], 1);
    assert_eq!(pair["edit_valid_pairs"], 2);
    assert_eq!(pair["edits"][0]["control_seconds"], 0);
    assert_eq!(pair["median_difference"], -55.0);
    assert_eq!(pair["median_ratio"], -0.25);
    let rows = report["candidates"].as_array().unwrap();
    let a = rows
        .iter()
        .find(|r| r["stratum"]["variant"] == "a")
        .unwrap();
    assert_eq!(a["summary"]["edit_missing"]["not_measured"], 1);
    assert_eq!(a["summary"]["pause_missing"]["not_measured"], 1);
}

#[test]
fn offline_report_refuses_missing_registered_pair_instead_of_shrinking_assigned() {
    let mut data: serde_json::Value =
        serde_json::from_str(include_str!("synthetic_manual.json")).unwrap();
    data["pairs"]
        .as_array_mut()
        .unwrap()
        .retain(|pair| pair["pair_id"] != "synthetic-M2");
    data["candidates"]
        .as_array_mut()
        .unwrap()
        .retain(|candidate| candidate["pair_id"] != "synthetic-M2");
    assert_eq!(data["registered_slots"].as_array().unwrap().len(), 3);
    let error = error_value(&data.to_string());
    assert_eq!(error["category"], "validation");
    assert_eq!(error["kinds"], serde_json::json!(["registration"]));
}
