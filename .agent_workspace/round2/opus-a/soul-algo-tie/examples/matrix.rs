//! Prints the ablation table and every Chinese explanation.
//!
//! `cargo run --example matrix`. The table in `ABLATION.md` is this output, so
//! the document cannot drift away from the code.

use soul_algo_tie::testing;
use soul_algo_tie::{score_all, Detail, TieAlgo};

fn main() {
    let fixtures = testing::all();

    println!("## 消融矩阵（as_of = 2026-08-24T14:00:00Z）\n");
    println!("| 夹具 | 说明 | 次数 | 天数 | 互惠 | 私聊 | 距今 | T0 | T3 | T3R | T4 |");
    println!("|---|---|---:|---:|:---:|:---:|---:|---|---|---|---|");
    for f in &fixtures {
        let scores = score_all(f.peer_id, &f.log, f.as_of);
        let s = &scores[0];
        let bands: Vec<String> = scores
            .iter()
            .map(|score| score.band.as_str().to_string())
            .collect();
        println!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} |",
            f.name,
            f.summary,
            s.interaction_count,
            s.active_day_count,
            if s.is_reciprocal() { "是" } else { "否" },
            if s.any_direct { "是" } else { "否" },
            s.silent_days,
            bands.join(" | "),
        );
    }

    println!("\n## T3R 的折算量（四分之一为单位）\n");
    println!("| 夹具 | 桶内条数 (0-90/90-180/180-360/≥360) | 折算次数 | 折算天数 |");
    println!("|---|---|---:|---:|");
    for f in &fixtures {
        let score = TieAlgo::T3R.score(f.peer_id, &f.log, f.as_of);
        let Detail::Decayed(d) = score.detail else {
            continue;
        };
        println!(
            "| `{}` | {}/{}/{}/{} | {} | {} |",
            f.name,
            d.bucket_interactions[0],
            d.bucket_interactions[1],
            d.bucket_interactions[2],
            d.bucket_interactions[3],
            soul_algo_tie::types::zh_quarters(d.eff_count_milli),
            soul_algo_tie::types::zh_quarters(d.eff_days_milli),
        );
    }

    println!("\n## 中文解释\n");
    for f in &fixtures {
        println!("### `{}` — {}\n", f.name, f.summary);
        for algo in TieAlgo::CANDIDATES {
            let score = algo.score(f.peer_id, &f.log, f.as_of);
            println!(
                "- **{}**（{}）：{}",
                algo.id(),
                score.band.as_str(),
                algo.explain_zh(&score)
            );
        }
        println!();
    }
}
