use super::{data::*, diagnosis, policy, ranking};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub fn directory() -> PathBuf {
    PathBuf::from(
        std::env::var_os("SOUL_TASK_REQUIREMENTS_DIR").expect("explicit synthetic directory"),
    )
}

fn read_optional(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, &'static str> {
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("artifact_unreadable"),
    };
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "artifact_unreadable")?;
    if bytes.len() > limit {
        return Err("artifact_budget");
    }
    Ok(Some(bytes))
}

pub fn fixtures() -> Vec<Fixture> {
    let (relative, expected) = match std::env::var("SOUL_TASK_REQUIREMENTS_SET").as_deref() {
        Err(_) | Ok("original") => ("../../fixtures/synthetic-task-context/cases.json", 96),
        Ok("metadata-v2") => ("../../fixtures/synthetic-task-context-v2/cases.json", 72),
        Ok(_) => panic!("unsupported synthetic fixture set"),
    };
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let bytes = read_optional(&path, 16 * 1024 * 1024)
        .expect("bounded synthetic fixture file")
        .expect("synthetic fixture file exists");
    let cases: Vec<Fixture> =
        serde_json::from_slice(&bytes).expect("strict synthetic fixture schema");
    assert_eq!(
        cases.len(),
        expected,
        "all synthetic cases remain in the denominator"
    );
    let mut ids = std::collections::BTreeSet::new();
    for case in &cases {
        diagnosis::validate_input(&case.input).expect("valid synthetic input");
        assert!(ids.insert(&case.input.case_id), "unique synthetic case IDs");
    }
    cases
}

fn write_new<T: Serialize>(path: &Path, value: &T) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .expect("new artifact only");
    serde_json::to_writer(&mut file, value).expect("synthetic artifact encoding");
    file.write_all(b"\n").expect("synthetic artifact write");
}

pub fn export_inputs() {
    let dir = directory();
    std::fs::create_dir_all(&dir).expect("explicit synthetic export directory");
    let mut inputs = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("input.jsonl"))
        .expect("new input-only output");
    let mut evaluation = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("evaluation.jsonl"))
        .expect("new evaluation-only output");
    let cases = fixtures();
    for case in &cases {
        write_new(
            &dir.join(format!("input-{}.json", case.input.case_id)),
            &case.input,
        );
        serde_json::to_writer(&mut inputs, &case.input).expect("input-only JSONL");
        inputs.write_all(b"\n").unwrap();
        let truth = serde_json::json!({"case_id":case.input.case_id,"family_id":case.family_id,
            "split":case.split,"variant":case.variant,"gold":case.gold});
        serde_json::to_writer(&mut evaluation, &truth).expect("evaluation-only JSONL");
        evaluation.write_all(b"\n").unwrap();
    }
    println!(
        "{}",
        serde_json::json!({"phase":"export_complete","cases":cases.len(),
        "input_only":"input.jsonl","evaluation_only":"evaluation.jsonl","network_requests":0})
    );
}

pub fn check_exported_input(dir: &Path, input: &Input) -> Result<(), &'static str> {
    let bytes = read_optional(
        &dir.join(format!("input-{}.json", input.case_id)),
        policy::MAX_PREDICTION_BYTES,
    )?
    .ok_or("exported_input_missing")?;
    let stored: Input = serde_json::from_slice(&bytes).map_err(|_| "exported_input_shape")?;
    if stored != *input {
        return Err("exported_input_changed");
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    reason: String,
}

pub struct PredictionFile {
    pub prediction: Result<Prediction, &'static str>,
    pub failure: Option<String>,
}

pub fn prediction(dir: &Path, input: &Input) -> PredictionFile {
    let result = read_optional(
        &dir.join(format!("prediction-{}.json", input.case_id)),
        policy::MAX_PREDICTION_BYTES,
    );
    match result {
        Ok(Some(bytes)) => PredictionFile {
            prediction: diagnosis::parse(&bytes, input),
            failure: None,
        },
        Err(reason) => PredictionFile {
            prediction: Err(reason),
            failure: None,
        },
        Ok(None) => {
            let failure = read_optional(
                &dir.join(format!("failure-{}.json", input.case_id)),
                policy::MAX_PREDICTION_BYTES,
            );
            match failure {
                Ok(Some(bytes)) => match serde_json::from_slice::<Failure>(&bytes) {
                    Ok(failure)
                        if !failure.reason.trim().is_empty()
                            && failure.reason.chars().count() <= 1024 =>
                    {
                        PredictionFile {
                            prediction: Err("model_failure"),
                            failure: Some(failure.reason),
                        }
                    }
                    _ => PredictionFile {
                        prediction: Err("failure_artifact_shape"),
                        failure: None,
                    },
                },
                Err(reason) => PredictionFile {
                    prediction: Err(reason),
                    failure: None,
                },
                Ok(None) => PredictionFile {
                    prediction: Err("prediction_missing"),
                    failure: None,
                },
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ranking {
    case_id: String,
    ranked_ids: Vec<String>,
}

pub fn rank(dir: &Path, input: &Input) -> (Result<Vec<String>, &'static str>, &'static str) {
    let result = read_optional(
        &dir.join(format!("ranking-{}.json", input.case_id)),
        policy::MAX_PREDICTION_BYTES,
    );
    match result {
        Ok(None) => (
            Ok(ranking::rank(&input.case(), true, true)),
            "bm25_groups_mmr",
        ),
        Err(reason) => (Err(reason), "provided_ranking"),
        Ok(Some(bytes)) => {
            let rank = serde_json::from_slice::<Ranking>(&bytes)
                .map_err(|_| "ranking_shape")
                .and_then(|value| {
                    if value.case_id != input.case_id {
                        return Err("ranking_case_id");
                    }
                    diagnosis::validate_rank(input, &value.ranked_ids)?;
                    Ok(value.ranked_ids)
                });
            (rank, "provided_ranking")
        }
    }
}
