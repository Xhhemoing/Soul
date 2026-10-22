//! Small-pool timings reuse current algorithms and existing machine predictions.
//! File I/O and fixture/gold loading stay outside the measured calls.
use super::{data::Action, diagnosis, files, ranking};
use std::{fs::OpenOptions, hint::black_box, io::Write, time::Instant};

fn measure<T>(mut operation: impl FnMut() -> T) -> (f64, f64) {
    let first = Instant::now();
    black_box(operation());
    let first_us = first.elapsed().as_secs_f64() * 1_000_000.0;
    let repeated = Instant::now();
    for _ in 0..10 {
        black_box(operation());
    }
    (first_us, repeated.elapsed().as_secs_f64() * 100_000.0)
}

pub fn run() {
    let dir = files::directory();
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join("local-stage-timings.jsonl"))
        .expect("new timing artifact only");
    let mut rows = 0;
    for (index, fixture) in files::fixtures().into_iter().enumerate() {
        let input = &fixture.input;
        files::check_exported_input(&dir, input).expect("unchanged synthetic input");
        let case = input.case();
        let mut record = |stage: &str, timings: (f64, f64)| {
            let row = serde_json::json!({"case_id":input.case_id,"stage":stage,
                "candidate_count":case.candidates.len(),"first_us":timings.0,
                "warm_mean_us":timings.1,"warm_repetitions":10,
                "profile":"cargo test profile","network_requests":0});
            writeln!(output, "{row}").expect("timing artifact write");
            rows += 1;
        };
        record(
            "input_validation",
            measure(|| diagnosis::validate_input(black_box(input)).expect("valid input")),
        );
        record(
            "exact_groups",
            measure(|| ranking::groups(black_box(&case))),
        );
        // Rotate mode order across cases; each measurement includes its own tokenize/score.
        let modes = [
            ("bm25", false, false),
            ("bm25_groups", true, false),
            ("bm25_groups_mmr", true, true),
        ];
        for offset in 0..modes.len() {
            let (name, grouped, diversity) = modes[(index + offset) % modes.len()];
            record(
                name,
                measure(|| ranking::rank(black_box(&case), grouped, diversity)),
            );
        }
        let prediction = files::prediction(&dir, input)
            .prediction
            .expect("existing valid machine prediction");
        if diagnosis::action(&prediction) == Action::Ready {
            let rank = ranking::rank(&case, true, true);
            record(
                "support_selection_with_validation",
                measure(|| {
                    diagnosis::selection(black_box(input), &prediction, &rank)
                        .expect("valid machine support selection")
                }),
            );
        }
    }
    println!("local-stage timing rows: {rows}; model/network calls: 0");
}
