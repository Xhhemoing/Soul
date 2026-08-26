MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 3 / opus-a — T4 定稿与 T4D 直连计数门

产出目录：`.agent_workspace/round3/opus-a/`

```
soul-algo-tie/                 独立 Cargo 包，edition 2021 / rust-version 1.83 / 零依赖 / forbid(unsafe_code)
  src/constants.rs             唯一常量表（G3 单源阈值）：3 / 10 / 3 与 180 / 360
  src/types.rs                 Band / Interaction / Tally / TieScore（**四项拆分计数 + 合计**）/ Detail
  src/gate.rs                  Granovetter 梯子，只管「互惠 ∧ 次数 ∧ 天数」，不管场地；T4 与 T4D 共用同一函数体
  src/recency.rs               唯一的降档步骤（≥180 降一档，≥360 封 weak），两条规则共用
  src/t4.rs                    全场地计数 + 群聊天花板 + 降档（Round 2 胜者，**保留为回退**）
  src/t4d.rs                   一对一计数 + 降档（**默认**）
  src/testing/mod.rs           21 条确定性夹具 + `private_only()` + 合并 store
  src/testing/oracle.rs        T0，**仅测试用 oracle**，不是 TieAlgo 变体
  src/tombstones.rs            #[cfg(test)] T1 / T2 / T3 / T3R 墓碑（可执行的否决理由与复活条件）
  examples/matrix.rs           生成本文件的消融表
  tests/ablation.rs            判卷表 / 观测表 / C8 违规表（20 测）
  tests/direct_gate.rs         T4D 的性质测试，生成式（8 测，共 2 万余条随机日志）
  tests/as_of_discipline.rs    as_of 纪律（10 测）
  tests/explain_zh.rs          中文话术冻结，含「一对一 vs 群里」要求（17 测）
  tests/goal1_fidelity.rs      oracle ↔ Goal 1 等价 + 拆分计数不改总数（5 测）
TEST_LOG.txt                   版本、cargo tree、fmt、clippy -D warnings、debug/release 双跑、matrix 全量输出
REPORT.md                      本文件
make_test_log.sh               重新生成 TEST_LOG.txt（日志里除 5 行抬头外无一行是手写的）
```

**测试：121 全绿**（60 单测 + 20 消融 + 8 直连门 + 10 `as_of` + 17 解释 + 5 保真 + 1 doc-test），debug 与 release 各跑一遍。`cargo fmt --check` 无 diff，`cargo clippy --all-targets -- -D warnings` 干净，`cargo tree` 只有自己一行。

`as_of` 由调用方传入，全程不读墙钟；本包锚点 = 2026-08-24T14:00:00Z = **1787580000**。

---

# 硬结论：**是**，T4D 应当替换 T4，成为唯一保留的人脉图算法

任务书给的三条替换判据，逐条对上（三条都有测试名，不是叙述）：

| 判据 | 结果 | 测试 |
|---|---|---|
| `lilei_12` 仍是 Strong | ✅ strong | `lilei_12_is_strong_under_both_rules` |
| `group_heavy_plus_one_direct_each_way` 不是 Strong | ✅ weak（且 T4 在这里是 strong） | `group_heavy_plus_one_direct_each_way_must_not_be_strong_and_is_weak` |
| 私聊-only 夹具上相对 T4 无其它回归 | ✅ 15 条私聊-only 夹具逐条同档；另有 4000 条随机私聊日志上两规则**恒等** | `the_two_rules_agree_on_every_fixture_with_no_group_rows`、`the_two_rules_are_the_same_rule_on_evidence_with_no_group_rows` |

再加一条不在判据里、但让这次替换在工程上变成**单向安全**的性质：在 4000 条随机混合日志上，`rank(T4D) ≤ rank(T4)` 恒成立（`t4d_never_says_more_than_t4`）。T4D 数的行是 T4 的子集、天是子集、一对一互惠蕴含总体互惠，梯子对两个量单调，降档步骤共用 —— 所以 T4D 只可能比 T4 更保守，**不可能在任何输入上凭空升档**。换句话说：换上去只会少标强关系，不会多标。

T4 **不删除**，作为回退单体留在同一个包里（`TieAlgo::T4`），回退是一行 `TieAlgo::DEFAULT` 的改动，不是考古。

---

## 1. 两条规则，写全

```text
# 共用：一次 tally，四项计数 + 两个日集合
direct_out / direct_in / group_out / group_in
days（任一场地的活跃日） ⊇ direct_days（一对一活跃日）
silence = max(0, as_of - last_contact_任一场地) / 86400        # 整数天，闭区间

# T4（回退）：全场地计数
band = Strong    if 互惠 ∧ 有过一对一 ∧ 次数 ≥10 ∧ 活跃日 ≥3
     | Moderate  if 互惠 ∧ 次数 ≥3
     | Weak
群聊-only 封顶 Moderate

# T4D（默认）：一对一计数
direct_reciprocal = direct_out ≥1 ∧ direct_in ≥1
band = Strong    if direct_reciprocal ∧ direct_count ≥10 ∧ direct_active_days ≥3
     | Moderate  if direct_reciprocal ∧ direct_count ≥3
     | Weak
（群聊-only ⇒ direct_count = 0 ⇒ 不互惠 ⇒ Weak，无需天花板）

# 共用降档：silence ≥360 → Weak；≥180 → 降一档
```

三个 Round 3 的结构性改动，都不许改档（有测试兜底）：梯子搬进 `gate.rs`、降档搬进 `recency.rs`、计数改为拆分后再求和。T4 的档位与 Round 2 逐条一致（消融表 T4 列与 Round 2 `ABLATION.md` 的 T4 列在 16 条旧夹具上完全相同）。

**阈值一个没动**。R2-SYNTHESIS §冻结边界的 3 / 10 / 3 与 180 / 360 闭区间原样保留在 `constants.rs`；T4D 改的不是数字，是**数哪些行**。

## 2. 消融矩阵（as_of = 1787580000，由 `cargo run --example matrix` 生成）

| 夹具 | 总次数 | 一对一 | 群里 | 总天数 | 一对一天数 | 一对一互惠 | 距今 | T0(oracle) | T4 | **T4D** |
|---|---:|---:|---:|---:|---:|:---:|---:|---|---|---|
| `empty` | 0 | 0 | 0 | 0 | 0 | 否 | 0 | weak | weak | weak |
| `single_inbound` | 1 | 1 | 0 | 1 | 1 | 否 | 1 | weak | weak | weak |
| `lilei_12` | 12 | 12 | 0 | 6 | 6 | 是 | 3 | strong | strong | **strong** |
| `afternoon_20` | 20 | 20 | 0 | 1 | 1 | 是 | 1 | moderate | moderate | moderate |
| `group_only_50` | 50 | 0 | 50 | 50 | 0 | 否 | 1 | strong | moderate | **weak** |
| `one_sided_100` | 100 | 100 | 0 | 100 | 100 | 否 | 1 | weak | weak | weak |
| `flood_1000_in_one_day` | 1000 | 1000 | 0 | 1 | 1 | 是 | 1 | moderate | moderate | moderate |
| `steady_16_over_8_weeks` | 16 | 16 | 0 | 8 | 8 | 是 | 7 | strong | strong | strong |
| `quiet_179_days` | 12 | 12 | 0 | 6 | 6 | 是 | 179 | strong | strong | strong |
| `quiet_180_days` | 12 | 12 | 0 | 6 | 6 | 是 | 180 | strong | moderate | moderate |
| `quiet_200_days` | 12 | 12 | 0 | 6 | 6 | 是 | 200 | strong | moderate | moderate |
| `revived_after_gap` | 32 | 32 | 0 | 12 | 12 | 是 | 4 | strong | strong | strong |
| `group_only_quiet_200` | 20 | 0 | 20 | 10 | 0 | 否 | 200 | strong | weak | weak |
| `dormant_359_days` | 12 | 12 | 0 | 6 | 6 | 是 | 359 | strong | moderate | moderate |
| `dormant_360_days` | 12 | 12 | 0 | 6 | 6 | 是 | 360 | strong | weak | weak |
| `dormant_2019` | 20 | 20 | 0 | 10 | 10 | 是 | 2632 | strong | weak | weak |
| `group_heavy_plus_one_direct_each_way` | 32 | 2 | 30 | 10 | 1 | 是 | 2 | strong | strong | **weak** |
| `group_heavy_plus_three_directs` | 33 | 3 | 30 | 10 | 2 | 是 | 2 | strong | strong | **moderate** |
| `group_heavy_plus_directs_one_way` | 38 | 8 | 30 | 10 | 3 | 否 | 2 | strong | strong | **weak** |
| `dormant_direct_group_ping_yesterday` | 13 | 12 | 1 | 7 | 6 | 是 | 1 | strong | strong | strong |
| `dormant_direct_no_ping` | 12 | 12 | 0 | 6 | 6 | 是 | 300 | strong | moderate | moderate |

C8 判卷违规（判卷标准写在 `tests/ablation.rs::TRUTH`，先于观测写死）：

| 规则 | 违规条数 | 是哪些 |
|---|---:|---|
| T0（oracle，仅作标尺） | 11 | 群聊 3 条 + 沉寂 5 条 + 群重度 3 条 |
| T4（回退） | 3 | `group_heavy_plus_one_direct_each_way` / `group_heavy_plus_three_directs` / `group_heavy_plus_directs_one_way` |
| **T4D（默认）** | **0** | — |

T4 的 3 条违规就是 R2-SYNTHESIS §潜在边界风险 2 挂的那笔账，本轮把它从「fable-b 的一段话」变成了三条会红的夹具。

## 3. 任务书点名的两条夹具

**`group_heavy_plus_one_direct_each_way`**：群里互惠 30 次 / 10 天，外加一对一各 1 次。
一对一 2 次，`2 < 3`，所以 **Weak**（不是 Moderate）。注意它**通过**了一对一互惠这一关 —— 拦住它的是次数门，不是互惠门，这一点在测试里单独断言，免得日后有人以为是互惠救的场。同一份证据 T4 判 **strong**。

配对夹具 `group_heavy_plus_three_directs` 只多一条私聊（3 次），T4D 升到 moderate。两条一起证明 T4D 是**把门搬到直连计数上**，而不是「见群重度就杀」。

**`lilei_12`**：全私聊 12 次 / 6 天 / 3 天前 → **strong**，与 Goal 1 验收口径一致。任务书里「若 T4D 打挂 lilei_12 则放弃 T4D」的分支没有触发。

## 4. 代价：群聊-only 从 moderate 掉到 weak（唯一的行为倒退，必须签字）

T4 给「只在群里见过」的人留了 `GROUP_ONLY_CEILING = Moderate` 的天花板，理由是 Round 1 §3.1：天天在同一个项目群里的同事不是陌生人。T4D **不可能保留这条天花板** —— 天花板作用的那个量（全场地次数）在 T4D 里根本不参与判档，`direct_count = 0` 先在互惠门上就挂了。于是 `group_only_50` 由 moderate 变 weak。

三点说明，供上层裁决：

1. 这不算判据意义上的回归：`group_only_50` 不是私聊-only 夹具，且判卷真值是 `NotStrong`，weak 与 moderate 都不违规。两条规则的**全部**分歧只有 4 条，全部含群聊行（`the_two_rules_disagree_on_exactly_these_fixtures`）。
2. 证据没有丢，只是不再抬档：`TieScore` 仍然带着 `group_out_count / group_in_count`，中文解释仍然会说「群里 50 次」并明说这 50 次不算进这一档。用户能看到全部数字并自己判断。
3. 如果产品坚持「同事 ≠ 陌生人」，正确的落点是**渲染层**（A2 那类「常在同一个群」的标注），不是在 T4D 里补第二套阈值 —— R2-SYNTHESIS 已经在源码级禁止第二套阈值。本包没有留这个口子。

## 5. 风险：降档用「任一场地」的近因（F04c 被扩大了）

任务书要求：降档看 `last_contact` 的任一场地，群里一响就不算沉寂。已按此实现，并做成一对只差一行证据的夹具：

- `dormant_direct_group_ping_yesterday`：私聊 12 次 / 6 天全在 300 天前，昨天对方在群里说了一句 → **strong**。
- `dormant_direct_no_ping`：同样的私聊史，没有那句 → **moderate**。

这是 R2-SYNTHESIS §潜在边界风险 1（F04c「复燃一响」）的扩大版：原来需要一句私聊才能复活，现在群里被 @ 一次也够。我把这条夹具的判卷真值标成 **`Unjudged`** 而不是 `Exactly(Strong)`：任务书指定了机制，但没有哪一轮同意过「300 天没单独说过话的人是强联系」这个结论。两种读法都成立，取决于产品把「近因」当作**关系强度的一部分**还是**关系是否仍然存在的证据**；本包实现的是后者。

真要收紧，最小改动是把降档时钟从 `last_contact` 换成 `last_direct_contact`（`TieScore` 已经带着这个字段，`recency::demote` 是纯函数），代价是「群里天天见、私聊半年没有」的同事会掉档。这是 Round X 的题，本轮不擅自加第三道门 —— R2-SYNTHESIS 明令禁止静默加门。

## 6. 中文解释：一对一次数与群里次数分开说

T4D 的第一句话永远同时给出两个数（哪怕其中一个是 0），因为它判档用的是前者：

> **`group_heavy_plus_one_direct_each_way` / T4D**：你们一共有 32 次往来：一对一 2 次（你发出 1 次，对方发来 1 次），群里 30 次（你发出 15 次，对方发来 15 次）；这些往来分布在 10 天、2 个会话里，最近一次是 2026 年 8 月 22 日，距今 2 天。你们一对一聊过 2 次，双方都发过，但不到 3 次，还只是打过招呼；你们在群里还有 30 次往来，那只说明你们常在同一个场合，不算进这一档，最近一次往来距今 2 天，还不到 180 天，不用往下降，所以算弱联系。

> **`group_only_50` / T4D**：……一对一 0 次（你发出 0 次，对方发来 0 次），群里 50 次……你们从来没有单独聊过，一对一 0 次，看不出你们私下有来往；你们在群里还有 50 次往来，那只说明你们常在同一个场合，不算进这一档……所以算弱联系。

冻结成性质测试而不是字符串比对：`the_default_rule_names_the_one_to_one_count_apart_from_the_group_count` 对每条非空夹具断言「一对一 N 次」「群里 M 次」「你发出 K 次」三个片段都在；`a_private_only_tie_is_not_told_about_a_group_it_was_never_in` 保证纯私聊的人不会收到一句关于群的废话。原有约束全部继续生效：无拉丁字母、无小数点、无「分数/权重/百分」、必须点名 次数 / 天数 / 距今 / 日期 / 最终档位。T4 的话术一字未改，且与 T4D 在每条夹具上都不同字符串（`the_rollback_rule_keeps_reporting_one_total`）。

## 7. 公开 API

```rust
pub enum TieAlgo { T4, T4D }              // #[default] = T4D
pub const TieAlgo::DEFAULT = TieAlgo::T4D;

pub fn score(peer_id, &[Interaction], as_of_unix) -> TieScore;          // 默认规则
pub fn score_ego_network(&[Interaction], as_of_unix) -> Vec<(u64, TieScore)>;
pub fn explain_zh(&TieScore) -> String;   // 按 score.algorithm_id 派发，不会用 T4D 的话术解释 T4 的档
pub fn score_both(peer_id, &[Interaction], as_of_unix) -> [TieScore; 2];  // 对照用，一次 tally
```

`TieScore` 新增：`direct_out_count / direct_in_count / group_out_count / group_in_count`、`direct_active_day_count`、`last_direct_contact_unix`，以及派生的 `direct_count()` / `group_count()` / `is_direct_reciprocal()` / `any_direct()`。旧的合计字段（`interaction_count / outgoing_count / incoming_count / active_day_count`）语义不变，且被测试钉死为拆分项之和。

T0 只在 `testing::oracle` 里，`TieAlgo::from_id("T0") == None`，产品路径选不到它（`the_oracle_is_not_reachable_as_a_product_rule`）。T1 / T2 / T3 / T3R 在 `#[cfg(test)] tombstones.rs`，每条都带杀死它的夹具；T3R 另带复活条件与复活代价（`t3rs_revival_condition_costs_the_revived_friendship`：复活它就要接受把四天前刚聊过的 32 次私聊判成 moderate），并且钉住了「T3R 也修不了场地闩」（`t3r_would_not_have_fixed_the_venue_latch_either`）。

## 8. 与 R2-SYNTHESIS 冻结边界的一致性

| 冻结项 | 本包 |
|---|---|
| `as_of` 全库一个值、调用方传入、禁止 per-peer max | ✅ `tests/as_of_discipline.rs`，含「按 peer 取 max 会让 2019 年的关系变 strong」的反例 |
| 降档 ≥180 一档、≥360 封 weak、闭区间 | ✅ `recency.rs` + `the_frozen_recency_edges_are_closed_intervals_at_180_and_360` |
| 阈值 3 / 10 / 3，删除 count ≥8 | ✅ `constants.rs` 单源 |
| 废弃 T3R 的 milliscale | ✅ T3R 只在墓碑里，四分之一单位 |
| 群聊不能 Strong | ✅ 两条规则都不能，且 T4D 连 Moderate 也不给 |
| 禁止第二套阈值 | ✅ 两条规则共用同一张常量表与同一个梯子函数 |
| 不引入浮点 | ✅ `the_product_path_contains_no_floating_point` 逐文件扫源码，唯一的浮点在 T1 墓碑里 |

## 9. 没做的事

- **统一 `crates/soul-algo`**：本包仍是 `.agent_workspace` 下的独立包，没有动主仓 `crates/`。合并是 Round X 的落地动作，`TieAlgo` 的 API 面已经按「只有两条规则、默认一条」收敛，搬过去不需要再改形状。
- **`ALGO_FROZEN` 声明与 `docs/algorithms/REJECTED.md`**：本包只提供可执行的墓碑，没有写文档；墓碑测试里的判词可以直接搬进去。
- **A0 / A1 / A2**：本轮 opus-a 只做人脉侧。
- **预测准确率**：按 SOTA 验收口径，不验收。
