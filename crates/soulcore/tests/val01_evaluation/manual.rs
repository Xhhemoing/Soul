//! Explicit offline metadata input. No generation, network, database or scoring.
use super::{aggregate, pairs, records::Evaluation};
use std::path::Path;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub(super) enum ManualError {
    InputNotSpecified,
    Read,
    Json {
        kind: &'static str,
        line: usize,
        column: usize,
    },
    Validation {
        kinds: Vec<pairs::ErrorKind>,
    },
    Report,
}

pub(super) fn report_json(input: &str) -> Result<serde_json::Value, ManualError> {
    let data: Evaluation = serde_json::from_str(input).map_err(|error| ManualError::Json {
        kind: match error.classify() {
            serde_json::error::Category::Io => "io",
            serde_json::error::Category::Syntax => "syntax",
            serde_json::error::Category::Data => "data",
            serde_json::error::Category::Eof => "eof",
        },
        line: error.line(),
        column: error.column(),
    })?;
    let validated = pairs::validate(&data).map_err(|errors| ManualError::Validation {
        // Record ids are untrusted input. Only the finite error taxonomy leaves
        // the validator here; do not stringify errors or echo record ids.
        kinds: errors.into_iter().map(|error| error.kind).collect(),
    })?;
    // Live adapters can validate while they execute one registered slot at a
    // time. An offline report must represent every assignment, including an
    // explicit not-started pair, so omitted rows cannot shrink its denominator.
    // validate already proved unique registration and exact matching metadata.
    if data.registered_slots.iter().any(|slot| {
        !data.pairs.iter().any(|pair| {
            pair.evaluation_run == slot.evaluation_run && pair.task_slot == slot.task_slot
        })
    }) {
        return Err(ManualError::Validation {
            kinds: vec![pairs::ErrorKind::Registration],
        });
    }
    serde_json::to_value(aggregate::summarize(&validated)).map_err(|_| ManualError::Report)
}

pub(super) fn summarize_path(path: &Path) -> Result<serde_json::Value, ManualError> {
    let input = std::fs::read_to_string(path).map_err(|_| ManualError::Read)?;
    report_json(&input)
}

fn emit(value: &serde_json::Value) -> Result<(), ()> {
    let json = serde_json::to_string_pretty(value).map_err(|_| ())?;
    println!("VAL01_REPORT_BEGIN\n{json}\nVAL01_REPORT_END");
    Ok(())
}

#[test]
#[ignore = "explicit offline metadata file via SOUL_VAL01_INPUT"]
fn summarize_file() -> Result<(), ()> {
    let Some(path) = std::env::var_os("SOUL_VAL01_INPUT") else {
        emit(&serde_json::json!({"error": ManualError::InputNotSpecified}))?;
        return Err(());
    };
    match summarize_path(Path::new(&path)) {
        Ok(report) => {
            emit(&report)?;
            Ok(())
        }
        Err(error) => {
            emit(&serde_json::json!({"error": error}))?;
            Err(())
        }
    }
}
