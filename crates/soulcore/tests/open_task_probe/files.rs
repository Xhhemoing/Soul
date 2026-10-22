//! Explicit caller-provided synthetic artifacts only. No fixture or scoring loader.
use super::{policy::MAX_PREDICTION_BYTES, schema::*};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

pub fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, &'static str> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("artifact_unreadable"),
    };
    let mut bytes = Vec::new();
    file.take(MAX_PREDICTION_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "artifact_unreadable")?;
    if bytes.len() > MAX_PREDICTION_BYTES {
        return Err("artifact_budget");
    }
    Ok(Some(bytes))
}

pub fn check_exported_input(dir: &Path, raw: &RawInput) -> Result<(), &'static str> {
    let bytes = read_optional(&dir.join(format!("input-{}.json", raw.case_id)))?
        .ok_or("exported_input_missing")?;
    let mut value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "exported_input_shape")?;
    value
        .as_object_mut()
        .ok_or("exported_input_shape")?
        .remove("prepared_context");
    let actual: RawInput = serde_json::from_value(value).map_err(|_| "exported_input_shape")?;
    if actual != *raw {
        return Err("exported_input_changed");
    }
    Ok(())
}

pub struct Artifacts {
    pub prediction: Result<OpenPrediction, &'static str>,
    pub prediction_parse_error: Option<&'static str>,
    pub raw_prediction: Option<String>,
    pub failure: Option<String>,
    pub failure_error: Option<&'static str>,
}

pub fn prediction(dir: &Path, raw: &RawInput) -> Artifacts {
    let prediction_file = read_optional(&dir.join(format!("prediction-{}.json", raw.case_id)));
    let failure_file = read_optional(&dir.join(format!("failure-{}.json", raw.case_id)));
    let (failure, failure_error) = match failure_file {
        Ok(bytes) => (
            bytes.map(|b| String::from_utf8_lossy(&b).into_owned()),
            None,
        ),
        Err(error) => (None, Some(error)),
    };
    let (prediction, raw_prediction) = match prediction_file {
        Ok(Some(bytes)) => (
            parse_prediction(&bytes, raw),
            Some(String::from_utf8_lossy(&bytes).into_owned()),
        ),
        Ok(None) => (Err("prediction_missing"), None),
        Err(error) => (Err(error), None),
    };
    let prediction_parse_error = prediction.as_ref().err().copied();
    // Conflicting success/failure artifacts must not silently become success.
    let prediction = if failure.is_some() || failure_error.is_some() {
        Err("model_failure_artifact")
    } else {
        prediction
    };
    Artifacts {
        prediction,
        prediction_parse_error,
        raw_prediction,
        failure,
        failure_error,
    }
}

/// One bounded line at a time; oversized lines remain one failed condition set.
pub fn input_lines(path: &Path) -> Result<Vec<Result<Vec<u8>, &'static str>>, &'static str> {
    let file = File::open(path).map_err(|_| "input_jsonl_unreadable")?;
    let mut reader = BufReader::new(file);
    let mut lines = Vec::new();
    loop {
        let mut line = Vec::new();
        let read = (&mut reader)
            .take(MAX_PREDICTION_BYTES as u64 + 1)
            .read_until(b'\n', &mut line)
            .map_err(|_| "input_jsonl_unreadable")?;
        if read == 0 {
            break;
        }
        if line.len() > MAX_PREDICTION_BYTES {
            // Drain exactly the remainder of this line, keeping allocation bounded.
            if line.last() != Some(&b'\n') {
                loop {
                    let buffer = reader.fill_buf().map_err(|_| "input_jsonl_unreadable")?;
                    if buffer.is_empty() {
                        break;
                    }
                    let end = buffer.iter().position(|byte| *byte == b'\n');
                    let consumed = end.map_or(buffer.len(), |index| index + 1);
                    reader.consume(consumed);
                    if end.is_some() {
                        break;
                    }
                }
            }
            lines.push(Err("input_budget"));
        } else {
            lines.push(Ok(line));
        }
    }
    Ok(lines)
}
