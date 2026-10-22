use super::{
    adapter::Harness,
    data::Action,
    execution::{self, Outcome},
    ranking,
    schema::*,
};
use serde::Serialize;
use std::{collections::BTreeSet, fs::OpenOptions, io::Write, path::Path, time::Instant};

#[derive(Serialize)]
pub struct Condition {
    pub method: &'static str,
    pub outcome: Outcome,
    pub projection_ms: f64,
    pub ranking_ms: f64,
}

pub fn conditions(
    raw: &RawInput,
    prediction: Result<&OpenPrediction, &'static str>,
) -> Vec<Condition> {
    let raw_check = validate_raw(raw);
    let prediction = prediction.and_then(|p| validate_prediction(raw, p).map(|()| p));
    let mut rows = Vec::new();
    for method in ["predicted_required", "fixed4", "fixed8"] {
        let predicted = method == "predicted_required";
        let started = Instant::now();
        let projected = project(raw, if predicted { prediction.ok() } else { None });
        let projection_ms = started.elapsed().as_secs_f64() * 1000.0;
        let mut ranking_ms = 0.0;
        let mut outcome = if let Err(reason) = raw_check {
            Outcome::unavailable(reason)
        } else if predicted && prediction.is_err() {
            Outcome::unavailable(prediction.err().unwrap())
        } else if predicted && prediction.unwrap().needs.is_empty() {
            let mut out = Outcome::unavailable("no_required_information");
            out.available = true;
            out.action = Action::Ready;
            out.predicted_action = Some(Action::Ready);
            out.required_ids = Some(Vec::new());
            out.assessments = Some(Vec::new());
            out.selected_ids = Some(Vec::new());
            out.retained_ids = Some(Vec::new());
            out.chosen_groups = Some(Vec::new());
            out
        } else {
            match projected {
                Err(reason) => Outcome::unavailable(reason),
                Ok(input) => {
                    let case = input.case();
                    let started = Instant::now();
                    let raw_case = project(raw, None).expect("validated raw input").case();
                    let rank = ranking::rank(&raw_case, true, true);
                    ranking_ms = started.elapsed().as_secs_f64() * 1000.0;
                    let mut harness = Harness::new(&case);
                    match harness.snapshot() {
                        Err(reason) => Outcome::unavailable(reason),
                        Ok(snapshot) => {
                            let legacy = prediction.map(legacy_prediction);
                            execution::execute(
                                &input,
                                legacy.as_ref().map_err(|e| *e),
                                Ok(&rank),
                                method,
                                &mut harness,
                                &snapshot,
                            )
                        }
                    }
                }
            }
        };
        outcome.diagnosis_method = predicted;
        rows.push(Condition {
            method,
            outcome,
            projection_ms,
            ranking_ms,
        });
    }
    rows
}

pub fn run() {
    let dir = std::env::var_os("SOUL_OPEN_TASK_DIR").expect("explicit synthetic directory");
    run_directory(Path::new(&dir)).expect("open-task replay artifacts");
}

pub fn run_directory(dir: &Path) -> Result<usize, &'static str> {
    let lines = super::files::input_lines(&dir.join("input.jsonl"))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("results.jsonl"))
        .map_err(|_| "results_create_new")?;
    let mut seen = BTreeSet::new();
    let mut count = 0;
    for (line_index, line) in lines.into_iter().enumerate() {
        let started = Instant::now();
        let raw = line.and_then(|bytes| {
            serde_json::from_slice::<RawInput>(&bytes).map_err(|_| "input_shape")
        });
        let case_id = raw
            .as_ref()
            .map(|r| r.case_id.clone())
            .unwrap_or_else(|_| format!("invalid_line_{}", line_index + 1));
        let input_check = raw.as_ref().map_err(|reason| *reason).and_then(|raw| {
            validate_raw(raw)?;
            if !seen.insert(raw.case_id.clone()) {
                return Err("duplicate_case_id");
            }
            super::files::check_exported_input(dir, raw)
        });
        let artifacts = raw
            .as_ref()
            .ok()
            .filter(|raw| validate_raw(raw).is_ok())
            .map(|raw| super::files::prediction(dir, raw));
        let parse_ms = started.elapsed().as_secs_f64() * 1000.0;
        let rows = match (&raw, input_check) {
            (Ok(raw), Ok(())) => conditions(
                raw,
                artifacts
                    .as_ref()
                    .ok_or("prediction_unavailable")
                    .and_then(|a| a.prediction.as_ref().map_err(|e| *e)),
            ),
            (_, Err(reason)) => ["predicted_required", "fixed4", "fixed8"]
                .into_iter()
                .map(|method| {
                    let mut outcome = Outcome::unavailable(reason);
                    outcome.diagnosis_method = method == "predicted_required";
                    Condition {
                        method,
                        outcome,
                        projection_ms: 0.0,
                        ranking_ms: 0.0,
                    }
                })
                .collect(),
            _ => unreachable!(),
        };
        for row in rows {
            let projected_input = if row.method == "predicted_required" && input_check.is_ok() {
                artifacts
                    .as_ref()
                    .and_then(|artifact| artifact.prediction.as_ref().ok())
                    .and_then(|prediction| {
                        raw.as_ref()
                            .ok()
                            .and_then(|raw| project(raw, Some(prediction)).ok())
                    })
            } else {
                None
            };
            let value = serde_json::json!({
                "case_id":case_id, "input_line":line_index + 1, "method":row.method,
                "ranking_source":"bm25_group_mmr_original_text",
                "input_error":input_check.err(),
                "prediction_error":artifacts.as_ref().and_then(|a| a.prediction.as_ref().err()),
                "prediction_parse_error":artifacts.as_ref().and_then(|a| a.prediction_parse_error),
                "raw_prediction":artifacts.as_ref().and_then(|a| a.raw_prediction.as_ref()),
                "open_prediction":artifacts.as_ref().and_then(|a| a.prediction.as_ref().ok()),
                "projected_input":projected_input,
                "model_failure":artifacts.as_ref().and_then(|a| a.failure.as_ref()),
                "failure_artifact_error":artifacts.as_ref().and_then(|a| a.failure_error),
                "parse_ms":parse_ms, "projection_ms":row.projection_ms, "ranking_ms":row.ranking_ms,
                "outcome":row.outcome, "new_model_requests":0, "final_generation_executed":false,
                "scope":"synthetic preparation only; no semantic correctness score"
            });
            serde_json::to_writer(&mut output, &value).map_err(|_| "results_encoding")?;
            output.write_all(b"\n").map_err(|_| "results_write")?;
            output.flush().map_err(|_| "results_flush")?;
            count += 1;
        }
    }
    Ok(count)
}
