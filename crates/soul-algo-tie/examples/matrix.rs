//! Prints the ablation table and every Chinese explanation.
//!
//! `cargo run --example matrix`. The tables in `REPORT.md` are this output, so
//! the document cannot drift away from the code.

use soul_algo_tie::testing::{self, oracle::T0};
use soul_algo_tie::{score_both, TieAlgo, TieAlgorithm};

fn main() {
    let fixtures = testing::all();

    println!("## 消融矩阵（as_of = 2026-08-24T14:00:00Z = 1787580000）\n");
    println!(
        "| 夹具 | 说明 | 总次数 | 一对一 | 群里 | 总天数 | 一对一天数 | 一对一互惠 | 距今 | T0(oracle) | T4 | T4D |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|:---:|---:|---|---|---|");
    for f in &fixtures {
        let [t4, t4d] = score_both(f.peer_id, &f.log, f.as_of);
        let t0 = T0::score(f.peer_id, &f.log, f.as_of);
        println!(
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            f.name,
            f.summary,
            t4.interaction_count,
            t4.direct_count(),
            t4.group_count(),
            t4.active_day_count,
            t4.direct_active_day_count,
            if t4.is_direct_reciprocal() {
                "是"
            } else {
                "否"
            },
            t4.silent_days,
            t0.band.as_str(),
            t4.band.as_str(),
            t4d.band.as_str(),
        );
    }

    println!("\n## 两条规则的分歧\n");
    println!("| 夹具 | T4 | T4D | 为什么 |");
    println!("|---|---|---|---|");
    for f in &fixtures {
        let [t4, t4d] = score_both(f.peer_id, &f.log, f.as_of);
        if t4.band == t4d.band {
            continue;
        }
        let why = if t4d.direct_count() == 0 {
            "从未一对一".to_string()
        } else if !t4d.is_direct_reciprocal() {
            "一对一只有一头在说".to_string()
        } else {
            format!("一对一只有 {} 次", t4d.direct_count())
        };
        println!(
            "| `{}` | {} | {} | {} |",
            f.name,
            t4.band.as_str(),
            t4d.band.as_str(),
            why
        );
    }

    println!("\n## 中文解释\n");
    for f in &fixtures {
        println!("### `{}` — {}\n", f.name, f.summary);
        for algo in TieAlgo::ALL {
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
