//! Prints the verification matrix. The table in REPORT.md is this output,
//! machine-generated per ABLATION_PROTOCOL §5.3 — never hand-copied.

use t4d_verify::{all, score_t4, score_t4d};

fn main() {
    println!("MODEL_SLUG: claude-fable-5-thinking-xhigh");
    println!("as_of = 2026-08-24T14:00:00Z (store-level, caller-supplied)");
    println!();
    println!("| Fixture | count | days | direct | direct days | silent | T4 | T4D |");
    println!("|---|---:|---:|---:|---:|---:|---|---|");
    for f in all() {
        let t4 = score_t4(&f.log, f.as_of);
        let t4d = score_t4d(&f.log, f.as_of);
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            f.name,
            t4.interaction_count,
            t4.active_day_count,
            t4.direct_count,
            t4.direct_day_count,
            t4.silent_days,
            t4.band.as_str(),
            t4d.band.as_str(),
        );
    }
}
