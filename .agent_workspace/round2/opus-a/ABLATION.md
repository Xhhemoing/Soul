MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 2 / opus-a — C8 消融：T3 vs T3R vs T4

参考实现：`.agent_workspace/round2/opus-a/soul-algo-tie/`（edition 2021 / rust-version 1.83 / 零依赖 / `forbid(unsafe_code)` / 产品路径无浮点）。
消融套件：`tests/ablation.rs`（19 测）。表格由 `cargo run --example matrix` 生成，`the_observed_table_is_exactly_what_the_rules_say` 钉住，文档与代码不会走偏。

**`as_of` 纪律**：全部夹具 `as_of = 2026-08-24T14:00:00Z = 1787580000`（= `2026-01-01T00:00:00Z` + 235 天 + 14 小时，`the_anchor_is_the_instant_the_brief_names` 验算）。
`age_days = max(0, as_of − occurred_at) / 86400`，整日下取整。包内不存在 `SystemTime`。
另一条合法取法 `as_of = max(occurred_at)`（全库）在 `tests/as_of_discipline.rs` 里单独跑过，见下面第 6 节。

## 1. 消融矩阵

| # | 夹具 | 次数 | 天数 | 互惠 | 私聊 | 距今 | 常识判卷 | T0 | T3 | T3R | T4 |
|---|---|---:|---:|:---:|:---:|---:|---|---|---|---|---|
| 1 | `empty` | 0 | 0 | 否 | 否 | 0 | weak | weak | weak | weak | weak |
| 2 | `single_inbound` | 1 | 1 | 否 | 是 | 1 | weak | weak | weak | weak | weak |
| 3 | `lilei_12` | 12 | 6 | 是 | 是 | 3 | **strong** | strong | strong | strong | strong |
| 4 | `afternoon_20` | 20 | 1 | 是 | 是 | 1 | **moderate** | moderate | moderate | moderate | moderate |
| 5 | `group_only_50` | 50 | 50 | 是 | 否 | 1 | 非 strong | **strong** ✗ | moderate | moderate | moderate |
| 6 | `one_sided_100` | 100 | 100 | 否 | 是 | 1 | weak | weak | weak | weak | weak |
| 7 | `flood_1000_in_one_day` | 1000 | 1 | 是 | 是 | 1 | 非 strong | moderate | moderate | moderate | moderate |
| 8 | `steady_16_over_8_weeks` | 16 | 8 | 是 | 是 | 7 | **strong** | strong | strong | strong | strong |
| 9 | `quiet_179_days` | 12 | 6 | 是 | 是 | 179 | 未判 | strong | strong | moderate | strong |
| 10 | `quiet_180_days` | 12 | 6 | 是 | 是 | 180 | 非 strong | **strong** ✗ | **strong** ✗ | moderate | moderate |
| 11 | `quiet_200_days` | 12 | 6 | 是 | 是 | 200 | 非 strong | **strong** ✗ | **strong** ✗ | moderate | moderate |
| 12 | `revived_after_gap` | 32 | 12 | 是 | 是 | 4 | **strong**（判卷有争议，见 §4） | strong | strong | **moderate** ✗ | strong |
| 13 | `group_only_quiet_200` | 20 | 10 | 是 | 否 | 200 | 非 strong | **strong** ✗ | moderate | moderate | weak |
| 14 | `dormant_359_days` | 12 | 6 | 是 | 是 | 359 | 非 strong | **strong** ✗ | **strong** ✗ | weak | moderate |
| 15 | `dormant_360_days` | 12 | 6 | 是 | 是 | 360 | 非 strong | **strong** ✗ | **strong** ✗ | weak | weak |
| 16 | `dormant_2019`（F04） | 20 | 10 | 是 | 是 | 2632 | 非 strong | **strong** ✗ | **strong** ✗ | weak | weak |

`✗` = 违反判卷。判卷（`TRUTH` 表）先于实现写死在 `tests/ablation.rs` 顶部，来源逐条注明：EVAL_MATRIX F01–F05、R1-SYNTHESIS P0-3 / P1-4、Goal 1 验收夹具。凡本轮没有共识的格子写 `未判`，不臆造答案。

### 判卷违规计数（C8 结果）

| 规则 | 违规夹具 | 数量 |
|---|---|---:|
| T0（基线，已退役为 oracle） | 5, 10, 11, 13, 14, 15, 16 | **7** |
| T3 | 10, 11, 14, 15, 16 | **5** |
| T3R | 12 | **1** |
| T4 | — | **0** |

`the_c8_violation_tables_are_exactly_these` 钉住这四行。

## 2. T3R 的折算量（四分之一为单位，`eff/4` 即用户看到的次数）

| 夹具 | 桶内条数 0–90 / 90–180 / 180–360 / ≥360 | 折算次数 | 折算天数 | 强档需要 |
|---|---|---:|---:|---|
| `lilei_12` | 12/0/0/0 | 12 | 6 | ≥10 次且 ≥3 天 |
| `afternoon_20` | 20/0/0/0 | 20 | **1** | 天数不够 |
| `group_only_50` | 50/0/0/0 | 50 | 50 | 无私聊，封顶 |
| `steady_16_over_8_weeks` | 16/0/0/0 | 16 | 8 | 过 |
| `quiet_179_days` | 0/2/10/0 | **3.5** | 1.75 | 次数不够 |
| `quiet_180_days` | 0/0/12/0 | **3** | 1.5 | 次数不够 |
| `quiet_200_days` | 0/0/12/0 | **3** | 1.5 | 次数不够 |
| `revived_after_gap` | 2/0/30/0 | **9.5** | 4.5 | **差半次** |
| `group_only_quiet_200` | 0/0/20/0 | 5 | 2.5 | 无私聊，封顶 |
| `dormant_359_days` | 0/0/2/10 | **0.5** | 0.25 | 连中档 3 次都不到 |
| `dormant_360_days` / `dormant_2019` | 0/0/0/12（20） | **0** | 0 | 全部归零 |

整数实现：`eff_count_milli` 与 `eff_days_milli` 以「四分之一次」为单位，阈值 = 冻结常量 ×4（强 40 / 12，中 12）。判档全程整数比较，无浮点、无边界抖动（`thresholds_are_the_frozen_bars_times_four`、`the_strong_corner_is_exact_in_quarter_units`）。

**「一天的权重」歧义的裁决**：取该日**最新一条**互动的桶权重。它与 CANDIDATE_SPEC 的「每个自然日取该日内最大 w」等价（权重随年龄单调不增，最新即最大），而且比「用当天日期对 as_of 分桶」更保守——后者会给一个当天并没有发生任何事的午夜记权重。跨桶边界的自然日是唯一能区分两种读法的情形，`day_weight_comes_from_the_latest_exchange_of_that_day` 钉住了这个差别。

## 3. 净胜夹具（C8 门）

**T3 相对 T0**：净胜 2（#5 `group_only_50`、#13 `group_only_quiet_200`），零反向回归。群聊封顶是结构修复，不是调参。

**T3R 相对 T3**：净胜 5（#10、#11、#14、#15、#16），零判卷回归。
**T4 相对 T3**：净胜 5（同样 5 条），零判卷回归。

两条净胜路线覆盖同一组夹具，这正是「衰减是否值得」的答案：**值得，但两种实现方式在判卷上打成平手**。

- 头号净胜 **#16 `dormant_2019`**：2019 年的 20 次私聊互惠，2026-08-24 判档。T3 判 strong（它没有时间概念）；T3R 全部 20 条落进 ≥360 天桶、折算 0 次 → weak；T4 距今 2632 天 ≥ 360 → weak。三者的原始计数完全一致（20 次 / 10 天），只是「对现在的主张」被撤回。
- 第二净胜 **#11 `quiet_200_days`**：T3 strong；T4 降一档 → moderate；T3R 折算 3 次 → moderate。**这是 T4 相对 T3 的第二条独立净胜。**

## 4. T3R 的唯一反向回归：#12 `revived_after_gap`

夹具：30 次私聊互惠发生在 191–200 天前，另有 2 次发生在 4–5 天前。共 32 次 / 12 个自然日 / 双向 / 私聊。

- T3 / T4：strong。T4 的理由是「最近一次距今 4 天，不用降档」。
- T3R：`2×1 + 30×¼ = 9.5` 折算次数，强档要 10 —— **差半次**，判 moderate。

判卷取 strong，理由写在 `tests/ablation.rs` 的 `TRUTH` 注释里：一个用户四天前刚说过话、总共 32 次一对一往来横跨 12 天的人，如果人脉图说这是「中等」，这张图描述的是档案而不是生活。

**这一格是本表唯一有争议的判卷**，必须显式记录：

- 若 Round 3 判 moderate 正确（理由可以是「主体证据确实旧了」），T3R 就变成 0 违规、T4 变成 1 违规，推荐结论反转。
- 我判 strong 的额外依据：T3R 在这里的失败模式是**不可向用户解释的**——用户会看到「你们一共 32 次往来……折算下来相当于 9.5 次」，然后问「为什么上周聊过的人算中等」。答案是「因为 30 条旧消息每条只算四分之一」，这是规则在为自己辩护，不是在描述关系。
- 反过来，T4 在这里的失败模式（如果它错了）是「一条最近的消息把整段历史救活」。这个失败可以被一句话防住，而且它在方向上是保守的：它不会把陌生人升档，只会把老朋友留在原档。

## 5. T3R 与 T4 的全部分歧（4 格，逐条记录）

`the_two_recency_rules_disagree_on_exactly_these_fixtures` 钉住这四条，多一条少一条都红。

| # | 夹具 | T3R | T4 | 分歧的根源 | 判卷 |
|---|---|---|---|---|---|
| 9 | `quiet_179_days` | moderate | strong | T4 的闸门还没开（179 < 180）；T3R 已经把 6 个活跃日里的 5 个打了四分之一折 | 未判 |
| 12 | `revived_after_gap` | moderate | strong | T3R 看整段历史的折算总量，T4 只看最后一条 | strong（T4 对） |
| 13 | `group_only_quiet_200` | moderate | weak | T3R 的群聊天花板把它按在 moderate；T4 从天花板再降一档 | 非 strong（两者都过） |
| 14 | `dormant_359_days` | weak | moderate | T3R 只剩最新一天的四分之一；T4 只降一档 | 非 strong（两者都过） |

结构性观察：**T3R 永远不会比 T4 更宽松**，因为衰减同时打击次数和天数两个门；而 T4 的降档最多走一格（≥360 直接封 weak 除外）。四条分歧里三条是「T3R 更严」，一条（#14）是 T3R 更严到跨了两档。

## 6. `as_of` 取法的影响（零回归的另一面）

`tests/as_of_discipline.rs` 用第二种合法 `as_of`（全库 `max(occurred_at)`，落在 2026-08-23T13:52:30Z，比锚点早约 10 小时）重跑全部夹具。全部 16 × 4 = 64 格里只有两格变化：

| 夹具 | 规则 | 锚点 as_of | 全库 max as_of | 原因 |
|---|---|---|---|---|
| `quiet_180_days` | T4 | moderate | strong | 距今变成 179 天，闸门没开 |
| `dormant_360_days` | T4 | weak | moderate | 距今变成 359 天，只降一档 |

两格都恰好坐在阈值上，是阈值锋利而不是规则不稳。**结论：`as_of` 必须在一次 rebuild 内全库统一取一个值**，并写进规范。另有两条纪律测试：

- `a_peer_local_as_of_would_hide_every_dormant_tie`：若按每个 peer 自己的最后一条取 `as_of`，`dormant_2019` 在 T3R 和 T4 下都会回到 strong。**逐 peer 取 as_of 是错的**，这是一个显式陷阱记录。
- `scoring_is_a_pure_function_of_the_evidence_and_the_as_of`：两次运行逐字节一致（F15 红线）。

## 7. 零回归证明（所有候选，所有夹具）

| 性质 | 测试 | 覆盖 |
|---|---|---|
| Goal 1 验收夹具仍是 strong | `lilei_12_is_strong_under_every_candidate` | #3 |
| 单向永远 weak | `one_sided_ties_are_weak_under_every_rule` | 全夹具扫描 |
| 群聊永远非 strong | `group_only_ties_are_never_strong_under_any_candidate` | 全夹具扫描 |
| 计数与规则无关 | `every_rule_agrees_about_the_counts_whatever_it_decides_about_the_band` | 全夹具 × 全规则 |
| 遗忘只降不升（orphan） | `forgetting_evidence_can_only_lower_a_band` | 全夹具 × 全规则 × 4 种截断 |
| 输入顺序无关 | `input_order_does_not_change_any_score` | 全夹具 |
| 产品路径无浮点 | `the_product_path_contains_no_floating_point` | 读源码断言 |
| O(n) 单遍 | `a_large_ego_network_is_scored_in_one_pass` | 10 万互动 / 1000 peer |
| T0 仍等价于 Goal 1 | `tests/goal1_fidelity.rs`（6500 组随机 + 跨纪元 + 活跃日集合） | oracle |

## 8. 中文解释对照（同一条夹具，三种说法）

`quiet_200_days`（T3 strong / T3R moderate / T4 moderate）：

- **T3**：「……你们私下一对一聊过，双方都发过消息，往来 12 次不少于 10 次，分布的 6 天也不少于 3 天，所以算强联系。」——**它说不出问题在哪，因为它看不见时间。**
- **T3R**：「……距今不到 90 天的往来按 1 次计，满 90 天不满 180 天的按半次计，满 180 天不满 360 天的按四分之一计，再早的不计。你们的 12 次往来里，0 次按 1 次计、0 次按半次计、12 次按四分之一计、0 次不计，折算下来相当于 3 次；……但折算后只有 3 次，不到 10 次，所以算中等联系。另外提醒一句：你们最近 200 天没有往来。」
- **T4**：「……往来 12 次不少于 10 次，分布的 6 天也不少于 3 天，本来可以算强联系；不过你们最近一次往来距今 200 天，已经不少于 180 天没有联系，所以往下降到中等联系。要是你们又聊起来，这一档会自己涨回去。」

三段都不含拉丁字母、不含分数/百分比/诊断词（`tests/explain_zh.rs` 14 测）。T3/T4 的输出**一个小数点都没有**（`the_integer_rules_never_print_a_fraction`）；T3R 的小数只出现在四分之一边界上（`.25` / `.5` / `.75`），且桶内条数全部列出，用户可以自己把 `2×1 + 30×¼` 算一遍。

## 9. 结论（详细论证见 REPORT.md）

1. 衰减/降档是真需求：T3 在 5 条夹具上错档，全部是「沉寂关系仍判强」。**T3 不能作为唯一保留算法。**
2. T3R 与 T4 在判卷上并列修好这 5 条，T3R 另在 #12 上错一格，T4 零错。
3. 触发本轮任务书写死的裁决条件：「若数字显示 T4 与 T3R 在夹具上打平且 T4 用整数解释，选 T4」。**推荐 T4。**
4. T3R 不删除，降为规范附录里的备选度量（如果 Round 3 判 #12 的 moderate 才是对的，一行 `TieAlgo` 切换即可换回）。
