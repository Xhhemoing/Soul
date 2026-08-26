//! COPY_ZH.md 第 5 节验收断言 × soul-algo-trait 实际渲染输出。
//!
//! 只读探针：以 path 依赖引用 crate，不改 crate 一行。跑法见报告。

use soul_algo_trait::a2::{a2_render, PersonnelSummary};
use soul_algo_trait::fixtures::tie_score_matrix;
use soul_algo_trait::types::Band;
use soul_algo_trait::{axis_vocabulary, denylist};

/// COPY_ZH.md 第 0.3 节禁词表，逐字抄自冻结话术，不含「诊断词表全表」（那条
/// 指向 crate 自己的 DIAGNOSTIC_DENYLIST，已单独覆盖）。
const COPY_ZH_FORBIDDEN: [&str; 12] = [
    "分数", "评分", "得分", "打分", "百分比", "百分位", "排名", "指数", "权重", "系数", "衰减",
    "半衰期",
];

/// COPY_ZH.md 第 5 节断言 4：任何模板不出现「今天」「现在」字样。
const COPY_ZH_TIME_WORDS: [&str; 2] = ["今天", "现在"];

/// COPY_ZH.md 第 0.1 节词汇单源：档位词只许这三个。
const COPY_ZH_BAND_WORDS: [&str; 3] = ["强", "中等", "弱"];

fn every_a2_sentence() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, mut score) in tie_score_matrix() {
        for band in [Band::None, Band::Weak, Band::Moderate, Band::Strong] {
            score.band = band;
            for bullet in a2_render(&score).bullets {
                out.push((
                    format!("{name}/{band:?}/{}", bullet.statement_key),
                    bullet.text_zh,
                ));
            }
        }
    }
    out.push((
        "nothing_to_say".to_owned(),
        PersonnelSummary::nothing_to_say_zh().to_owned(),
    ));
    out
}

fn section(title: &str) {
    println!("\n## {title}");
}

fn main() {
    let sentences = every_a2_sentence();
    println!(
        "A2 渲染句去重前 {} 条（{} 条唯一）。",
        sentences.len(),
        {
            let mut uniq: Vec<&str> = sentences.iter().map(|(_, t)| t.as_str()).collect();
            uniq.sort_unstable();
            uniq.dedup();
            uniq.len()
        }
    );

    // ---------------------------------------------------------- 断言 5 ---
    section("COPY_ZH §0.1 / §5.5 词汇单源：moderate → 「中等」");
    for band in [Band::Weak, Band::Moderate, Band::Strong] {
        let word = band.label_zh();
        let ok = COPY_ZH_BAND_WORDS.contains(&word);
        println!(
            "  Band::{band:?}.label_zh() = 「{word}」  {}",
            if ok { "符合" } else { "!! 违反（冻结词表无此词）" }
        );
    }
    let moderate_lines: Vec<&(String, String)> = sentences
        .iter()
        .filter(|(_, t)| t.contains("归在「中」一档"))
        .collect();
    println!(
        "  渲染出「归在「中」一档」的句子：{} 条（COPY_ZH P5 要求「中等」）",
        moderate_lines.len()
    );

    // ---------------------------------------------------------- 断言 4 ---
    section("COPY_ZH §5 断言 4：任何模板不出现「今天」「现在」");
    // 「出现在」里含「现在」两字，是断言按字面substring读时的假阳性，与
    // 「不读墙钟」无关。两个数分开报，免得把措辞问题算成纪律问题。
    let mut literal = 0_usize;
    let mut genuine: Vec<&str> = Vec::new();
    for (_, text) in &sentences {
        for word in COPY_ZH_TIME_WORDS {
            if text.contains(word) {
                literal += 1;
            }
        }
        let stripped = text.replace("出现在", "");
        if (stripped.contains("现在") || stripped.contains("今天"))
            && !genuine.contains(&text.as_str())
        {
            genuine.push(text.as_str());
        }
    }
    println!("  按字面 substring 命中：{literal} 次（含「出现在」的假阳性）");
    println!("  剔除「出现在」后的真命中：{} 条唯一句", genuine.len());
    for text in &genuine {
        println!("    !! 「{text}」");
    }
    let has_chuxianzai = sentences
        .iter()
        .filter(|(_, t)| t.contains("出现在"))
        .count();
    println!("  含「出现在」的渲染句：{has_chuxianzai} 条（断言若按字面执行会误伤）");

    // ---------------------------------------------------------- 断言 1 ---
    section("COPY_ZH §5 断言 1：crate 的 assert_publishable 是否覆盖 §0.3 禁词表");
    for word in COPY_ZH_FORBIDDEN {
        let probe = format!("按上面的计数，这条往来的{word}偏高。");
        let caught = denylist::assert_publishable(&probe).is_err();
        println!(
            "  「{word}」{} {}",
            if caught { "被拦截" } else { "未被拦截" },
            if caught { "" } else { "  <-- 缺口" }
        );
    }

    section("COPY_ZH §5 断言 1：任何拉丁字母");
    let latin: Vec<&(String, String)> = sentences
        .iter()
        .filter(|(_, t)| t.chars().any(|c| c.is_ascii_alphabetic()))
        .collect();
    println!(
        "  A2 渲染句含拉丁字母：{} 条；crate 无对应筛子（denylist 只查固定词表）",
        latin.len()
    );
    let axis_latin = axis_vocabulary()
        .iter()
        .filter(|t| t.chars().any(|c| c.is_ascii_alphabetic()))
        .count();
    println!("  axis_vocabulary() 含拉丁字母：{axis_latin} 条（含 statement_key，非用户可见句）");

    // ------------------------------------------------------- 模板对照 ---
    section("COPY_ZH §4 六类句 vs crate 实际模板");
    let render = |name: &str| {
        let score = tie_score_matrix()
            .into_iter()
            .find(|(k, _)| *k == name)
            .expect("fixture")
            .1;
        a2_render(&score)
    };

    let split = render("group_heavy_plus_one_direct_each_way");
    for (label, key, frozen) in [
        (
            "P1 总量",
            "personnel.activity.counts",
            "一共 {次数} 次往来，分布在 {天数} 个自然日、{会话数} 个会话里。",
        ),
        (
            "P1b 分列",
            "personnel.venue.direct_and_group_counts",
            "其中一对一往来 {一对一次数} 次，群里同场 {群聊次数} 次。",
        ),
        (
            "P2 方向",
            "personnel.tie.two_way",
            "（互斥五分支，阈值 2:1）多数时候是你先开口。/ 两边说得差不多。",
        ),
        (
            "P4 近因",
            "personnel.recency.days_since_last",
            "最近一次是 {最后日期}。",
        ),
        (
            "P5 归档",
            "personnel.tie.filed_band",
            "按上面的计数，这段关系归在『{强/中等/弱}』一档；这是工作假设，不是对这个人的判断。",
        ),
    ] {
        let actual = split
            .bullets
            .iter()
            .find(|b| b.statement_key == key)
            .map(|b| b.text_zh.clone())
            .unwrap_or_else(|| "（该夹具未渲染此句）".to_owned());
        println!("  {label}\n    冻结：{frozen}\n    实际：{actual}");
    }

    let group_only = render("group_only_reciprocal");
    let p3 = group_only
        .bullets
        .iter()
        .find(|b| b.statement_key == "personnel.venue.group_only")
        .map(|b| b.text_zh.as_str())
        .unwrap_or("（无）");
    println!("  P3 场合（仅群聊）\n    冻结：只在群聊里见过。\n    实际：{p3}");
    let direct = render("direct_reciprocal_thick");
    println!(
        "  P3 场合（有一对一）\n    冻结：有过一对一交流。\n    实际：{}",
        if direct.has("personnel.venue.group_only") {
            "（有 group_only 句）"
        } else {
            "（crate 刻意不渲染正面分支，见 a2.rs venue_bullet 注释）"
        }
    );

    let dormant = render("dormant_but_strong");
    let p4b = dormant
        .bullets
        .iter()
        .find(|b| b.statement_key == "personnel.recency.dormant")
        .map(|b| b.text_zh.as_str())
        .unwrap_or("（无）");
    println!("  P4 沉寂追加句\n    冻结：你们最近半年没有往来。\n    实际：{p4b}");

    section("逐字相同的句子");
    let exact = split
        .bullets
        .iter()
        .find(|b| b.statement_key == "personnel.venue.direct_and_group_counts")
        .map(|b| b.text_zh.clone())
        .unwrap();
    println!("  P1b：{exact}");
    println!("  与 COPY_ZH 模板结构逐字一致（只有占位符被填成数字）。");
}
