pub use super::execution::{execute, Outcome};
use super::{adapter::Harness, data::Action, files, scoring};
use std::{fs::OpenOptions, io::Write};

pub fn run() {
    let dir = files::directory();
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("results.jsonl"))
        .expect("new replay output only");
    let mut count = 0;
    let mut available = 0;
    let cases = files::fixtures();
    let case_count = cases.len();
    for fixture in cases {
        let input = &fixture.input;
        let input_check = files::check_exported_input(&dir, input);
        let prediction = files::prediction(&dir, input);
        let (rank, rank_source) = files::rank(&dir, input);
        let case = input.case();
        let mut harness = Harness::new(&case);
        let snapshot = harness
            .snapshot()
            .expect("all source snapshots before prediction replay");
        for method in ["all_options", "predicted_required", "fixed4", "fixed8"] {
            let mut result = match input_check {
                Err(reason) => {
                    let mut result = Outcome::unavailable(reason);
                    result.diagnosis_method =
                        matches!(method, "all_options" | "predicted_required");
                    result
                }
                Ok(()) => execute(
                    input,
                    prediction.prediction.as_ref().map_err(|reason| *reason),
                    rank.as_ref().map(Vec::as_slice).map_err(|reason| *reason),
                    method,
                    &mut harness,
                    &snapshot,
                ),
            };
            if let Err(reason) = harness.validate(&snapshot) {
                result.available = false;
                result.action = Action::Unknown;
                result.reason = reason;
            }
            // Gold and split metadata enter only after the runtime result exists.
            let score = scoring::score(&fixture.gold, prediction.prediction.as_ref().ok(), &result);
            let row = serde_json::json!({"case_id":input.case_id,"family_id":fixture.family_id,
                "split":fixture.split,"variant":fixture.variant,"method":method,
                "ranking_source":rank_source,"ranking_error":rank.as_ref().err(),
                "model_failure":prediction.failure,"prediction_available":prediction.prediction.is_ok(),
                "prediction_error":prediction.prediction.as_ref().err(),"outcome":result,"evaluation":score,
                "new_model_requests":0,"new_model_ms":null,"new_model_tokens":null,"new_model_cost":null});
            count += 1;
            available += usize::from(result.available);
            writeln!(output, "{}", serde_json::to_string(&row).unwrap())
                .expect("result JSONL write");
            println!("{row}");
        }
    }
    assert_eq!(count, case_count * 4, "all cases and methods preserved");
    println!(
        "{}",
        serde_json::json!({"phase":"complete","case_count":case_count,"conditions":count,
        "available":available,"unavailable":count-available,"network_requests":0,"new_model_requests":0})
    );
}
