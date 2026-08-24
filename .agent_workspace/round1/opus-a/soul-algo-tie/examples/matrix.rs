//! Print the band each candidate gives each fixture, as a Markdown table.
//!
//! `cargo run --example matrix`. The table in REPORT.md is this output pasted
//! in, so the report cannot drift away from the code.

use soul_algo_tie::testing;
use soul_algo_tie::{score_all, TieAlgo};

fn main() {
    let header: Vec<&str> = TieAlgo::ALL.iter().map(|algo| algo.id()).collect();
    println!(
        "| fixture | 次数 | 天数 | 互惠 | 私聊 | {} |",
        header.join(" | ")
    );
    println!("|---|---:|---:|---|---|{}", "---|".repeat(header.len()));

    for (name, peer_id, log, now) in testing::all() {
        let scores = score_all(peer_id, &log, now);
        let any_direct = log
            .iter()
            .any(|row| row.peer_id == peer_id && row.venue_direct);
        let bands: Vec<String> = scores
            .iter()
            .map(|score| score.band.as_str().to_string())
            .collect();
        println!(
            "| `{name}` | {} | {} | {} | {} | {} |",
            scores[0].interaction_count,
            scores[0].active_day_count,
            yes_no(scores[0].is_reciprocal()),
            yes_no(any_direct),
            bands.join(" | "),
        );
    }

    println!();
    for (name, peer_id, log, now) in testing::all() {
        println!("### {name}");
        for algo in TieAlgo::ALL {
            let score = algo.score(peer_id, &log, now);
            println!("- **{}**：{}", algo.id(), algo.explain_zh(&score));
        }
        println!();
    }
}

fn yes_no(flag: bool) -> &'static str {
    if flag {
        "是"
    } else {
        "否"
    }
}
