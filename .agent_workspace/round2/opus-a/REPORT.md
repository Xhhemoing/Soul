MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 2 / opus-a — T3 / T3R / T4 实现与 C8 消融

产出目录：`.agent_workspace/round2/opus-a/`

```
soul-algo-tie/                 独立 Cargo 包，edition 2021 / rust-version 1.83 / 零依赖 / forbid(unsafe_code)
  src/constants.rs             唯一常量表（G3 单源阈值）
  src/types.rs                 Band / Interaction / TieScore（含 any_direct）/ Detail / Tally / 日期与整数工具
  src/gate.rs                  Granovetter 门闩，T3 与 T3R 共用同一函数体
  src/t0.rs                    基线，**仅作回归 oracle**
  src/t3.rs                    门闩 × 原始计数
  src/t3r.rs                   门闩 × 分桶衰减计数（四分之一为单位，整数）
  src/t4.rs                    门闩 × 原始计数 + 整数降档
  src/tombstones.rs            #[cfg(test)] T1/T2 墓碑（可执行的否决理由）
  src/testing.rs               16 条确定性夹具 + 合并 store
  examples/matrix.rs           生成 ABLATION.md 的表
  tests/ablation.rs            C8 消融（19 测）
  tests/as_of_discipline.rs    as_of 纪律（9 测）
  tests/explain_zh.rs          中文话术冻结（14 测）
  tests/goal1_fidelity.rs      T0 ↔ Goal 1 等价（5 测）
ABLATION.md                    消融表 + 净胜/回归/分歧逐条
TEST_LOG.txt                   版本、cargo tree、fmt、clippy -D warnings、cargo test、matrix 全量输出
REPORT.md                      本文件
```

**测试：97 全绿**（49 单测 + 19 消融 + 9 `as_of` + 14 解释 + 5 保真 + 1 doc-test）。`cargo fmt --check` 无 diff，`cargo clippy --all-targets -- -D warnings` 干净，`cargo tree` 只有自己一行。

---

# 硬推荐：保留 **T4**，作为唯一的人脉图算法

**T4 = Granovetter 门闩（互惠 ∧ 私聊 ∧ 次数 ≥10 ∧ 活跃日 ≥3）+ 整数降档（沉寂 ≥180 天降一档，≥360 天封 weak）。**

三条理由，都有夹具支撑：

1. **它是唯一零判卷违规的规则。** T0 违规 7 条，T3 违规 5 条，T3R 违规 1 条，T4 违规 0 条（`the_c8_violation_tables_are_exactly_these`）。
2. **它与 T3R 在「衰减到底修好了什么」上完全打平。** 五条沉寂类夹具（`quiet_180` / `quiet_200` / `dormant_359` / `dormant_360` / `dormant_2019`）T3 全错，T3R 与 T4 全对。衰减带来的净胜，降档一条不落地全拿到了。
3. **它用整数解释，T3R 不能。** 触发本轮任务书写死的裁决条件：数字打平 + T4 整数可解释 → 选 T4。下面 §3 用 fable-a 自己的验收条款把这一条展开。

T3R **不删除**，写进规范附录作为备选度量；T3 **不单独保留**（它在 5 条夹具上错档，见 §2）；T0 **只保留为回归 oracle**，禁止回退（R1-SYNTHESIS 已明令）。

---

## 1. 常量对齐（本轮的规范性改动，必须留痕）

Round 1 的 opus-a T3 用的是 `count ≥ 8`。fable-a 冻结的 `CANDIDATE_SPEC` 与 Goal 1 `graph_build.rs` 都是 `count ≥ 10`。**Round 2 把所有存活候选对齐到 Goal 1 的 3 / 10 / 3**，理由是：如果两条规则的阈值不同，消融测到的就不是「规则的差别」，而是「阈值的差别」，C8 的结论会被污染。

| 项 | Round 1 opus-a T3 | Round 2（本包，全族统一） | 来源 |
|---|---|---|---|
| `MODERATE_MIN_INTERACTIONS` | 3（但私聊互惠 2 条即 moderate） | 3，**无例外** | Goal 1 |
| `STRONG_MIN_INTERACTIONS` | **8** | **10** | Goal 1 / CANDIDATE_SPEC |
| `STRONG_MIN_ACTIVE_DAYS` | 3 | 3 | Goal 1 |
| 群聊另设中档梯子 | 有（≥5 次 / ≥2 天） | **删除**，与所有人同用 ≥3 次 | 简化 |
| `GROUP_ONLY_CEILING` | Moderate | Moderate（保留） | Round 1 §3.1 的裁决 |
| `STRONG_REQUIRES_DIRECT` | 是 | 是 | Granovetter |

两处行为变化都有测试记录：`a_single_direct_exchange_is_weak_like_the_shipped_rule`（原来 moderate，现在 weak）、`ten_over_three_days_is_the_strong_boundary`（9 次现在是 moderate，Round 1 的 8 会判 strong）。`round_one_t3s_count_of_eight_is_gone` 直接钉住这次对齐。

所有阈值集中在 `src/constants.rs` 一个模块，T3R 的门槛是**推导出来的**（`STRONG_MIN_INTERACTIONS * MILLI_PER_INTERACTION` = 40），不是重新抄的数字 —— 这是 fable-a 验收门 G3「单源阈值」的实现形式。

## 2. 为什么 T3 单体不够（它错在哪）

T3 在 5 条夹具上判卷违规，全部同一个失效面：**它没有时间概念**。

| 夹具 | T3 | 应为 | 用户会看到什么 |
|---|---|---|---|
| `quiet_180_days` / `quiet_200_days` | strong | 非 strong | 半年多没说过话的人标成「强联系」 |
| `dormant_359_days` / `dormant_360_days` | strong | 非 strong | 快一年没联系的人标成「强联系」 |
| `dormant_2019` | strong | 非 strong | **七年前**停掉的关系标成「强联系」 |

R1-SYNTHESIS P1-4 已经把它列为遗留缺陷。它不是罕见路径：任何跑满一年的导入库都会有一堆这样的边，用户看到一次就不再信这张图。

T3 仍然值得留在规范附录里做**回退单体**（它是 T4 去掉降档以后的东西，`T4::band_of` 的第一行就是 `T3::band_of`），但不能作为唯一保留算法。

## 3. T4 vs T3R：用 fable-a 自己的门来判

R1-SYNTHESIS 把 T3R 记为 PROVISIONAL 胜者，并写明消杀条件。逐条对照：

### 3.1 C8 消融门：「T3R 相对 T3 若无任何净胜夹具 → 杀 T3R，留 T3（简单者胜）」

T3R **有** 5 条净胜夹具，所以这一条不杀 T3R。但同一条门也说明了本轮的裁决原则：**打平时选简单者**。T4 与 T3R 在这 5 条上给出的都是「非 strong」，判卷层面完全打平，于是原则生效。

### 3.2 零回归门：「T3R 在任何 T3 通过的夹具上翻车 → 杀」

T3R 在 `revived_after_gap`（#12）上翻车：T3 判 strong（判卷同意），T3R 判 moderate。**按字面读，这一条已经触发。**

我不主张按字面执行「杀」，因为这一格的判卷是我下的判断而不是本轮共识（ABLATION.md §4 明确标注了争议）。但它足以把 T3R 从「provisional 胜者」降到「与 T4 平级的候选」，而平级之后 §3.3 决定胜负。

### 3.3 解释复核门：「若中文话术被判『数字不可自行复核』→ 退回 T3」

这是决定性的一条。同一条边，两种说法：

- **T4**：「往来 12 次不少于 10 次，分布的 6 天也不少于 3 天，本来可以算强联系；不过你们最近一次往来距今 200 天，已经不少于 180 天没有联系，所以往下降到中等联系。」
  用户要复核的量：**12、6、200**。三个都能直接从证据列表数出来，第三个甚至就是屏幕上那个日期减今天。
- **T3R**：「你们的 12 次往来里，0 次按 1 次计、0 次按半次计、12 次按四分之一计、0 次不计，折算下来相当于 3 次；有往来的 6 天里……折算下来相当于 1.5 天。」
  用户要复核的量：**四个桶的条数、四个桶的权重、两个加权和、以及「天的权重取该日最新一条」这条规则**。每个数字确实可算，但复核一条边要做两次分组求和。

两者都通过了「不含小数点/百分比/诊断词/拉丁字母」的机械检查。差别不在合规，在**成本**：T4 的解释是一句话，T3R 的解释是一张表。产品锁要求的是「用户能复核」，不是「理论上可复核」。

另一个不对称：**T3R 的分档理由会随时间自己漂移，而漂移量用户看不见**。一条边今天折算 10 次、明天 9.75 次，档位跳变的那一天没有任何新证据发生。T4 也会因时间跳变，但跳变点是「第 180 天」「第 360 天」这两个用户能记住的数字，而且解释里直接写出来。

### 3.4 简化门（fable-a ACCEPTANCE §1「单一规范函数 + 单源阈值」）

| 项 | T3 | T3R | T4 |
|---|---|---|---|
| 判档需要的量 | 2（次数、天数） | 6（4 桶条数 → 2 个加权和） | 3（次数、天数、距今天数） |
| 新增常量 | 0 | 桶边界 3 个 + 权重 4 个 + 单位 1 个 | 2 个（180、360） |
| 需要向用户解释的规则条数 | 3 | 3 + 4 条折算规则 + 1 条「天怎么折」 | 3 + 2 条降档规则 |
| 边界歧义 | 无 | 「一天的权重」有两种读法（本包裁决并测试） | 无 |
| 判档算术 | 整数 | 整数（四分之一单位；若用有理数实现则是浮点） | 整数 |

T4 增加两个常量和一条「降一档」的规则，就拿到了 T3R 用一整套折算体系拿到的同一批净胜。

### 3.5 一个反方论点，写出来免得被当成没想过

T3R 有一条 T4 没有的能力：**它能看见「历史很旧但仍在滴水」的关系**（#12 那种）。T4 对 `revived_after_gap` 判 strong，等于说「最后一条消息可以复活整段历史」。如果 Round 3 认为这是缺陷，最小的补丁不是换成 T3R，而是给 T4 加一条**同样是整数**的门：「强档还要求最近 180 天内至少有 N 次往来」。那仍然只需要用户数一个数字。这条补丁本轮**没有实现**——它没有夹具支持，而且 EVAL_MATRIX 的判卷纪律禁止对着实现出题。

## 4. 三个候选的评分（1–5，附证据）

| 维 | T3 | T3R | T4 | 说明 |
|---|---|---|---|---|
| C1 产品锁契合 | 4 | 5 | **5** | T3 扣分：「最近接触」这个产品字段存在，但不影响档位，图会自相矛盾 |
| C2 可测性 | **5** | 4 | **5** | 三者都无墙钟、无浮点。T3R 扣分在「一天的权重」有裁决空间，且阈值边界随 `as_of` 移动更频繁 |
| C3 可解释性 | **5** | 3 | **5** | §3.3。T3R 的解释是一张表；T4 的是一句话，且一个小数点都没有 |
| C4 隐私 | **5** | **5** | **5** | 三者输入完全相同：id / 方向 / 时间戳 / 场合布尔 / 会话 id。无正文可达 |
| C5 稳健性 | 3 | 4 | **5** | 判卷违规 5 / 1 / 0 |
| C6 计算成本 | **5** | **5** | **5** | 单遍 O(n)，10 万互动 / 1000 peer 通过（`a_large_ego_network_is_scored_in_one_pass`） |
| C7 SOTA 对齐 | 4 | **5** | **5** | T3 = Granovetter；T3R = Granovetter + Burt/Hawkes 衰减的量化版；T4 = Granovetter + 「最近接触」这条 RFM 里唯一没有争议的维度，做成阶梯而不是分数 |
| C8 创新必要 | — | 4 | **5** | 净胜 5 / 5，反向回归 1 / 0 |

C7 需要说明白：**T4 不是发明**。它是 recency-as-a-gate，在 tie-strength 文献里就是 Marsden-Campbell 的「最近接触」指标（他们的三个 predictor：closeness、duration、frequency，加上 recency 作为 decay proxy），只不过用阶梯而不是连续函数实现。把连续量离散成用户能复述的档，是产品锁的要求，不是理论上的退让。

## 5. 实现里被我裁决的歧义（逐条）

1. **T3R 的「一天的权重」**：取该日最新一条互动的桶权重。与 CANDIDATE_SPEC 的「取该日内最大 w」等价（权重随年龄单调不增），比「按当天日期对 as_of 分桶」保守。测试：`day_weight_comes_from_the_latest_exchange_of_that_day`。
2. **`GROUP_ONLY_CEILING = Moderate`**：沿用 Round 1 的裁决。严格读法（Weak）仍然只差一个常量，`the_strict_reading_of_the_group_ceiling_is_one_constant_away` 记录后果。注意在 `STRONG_REQUIRES_DIRECT = true` 下这个天花板是冗余的（群聊本来就过不了强档门），保留它是为了「有人把开关关掉」的那天。
3. **T4 的降档次序**：先算 T3 档（含群聊封顶），再降。所以群聊边在任何时刻都不可能是 strong，降档只会让它更低（`a_group_only_tie_is_never_strong_before_or_after_demotion`）。
4. **未来时间戳**：`age_days` 钳到 0。导入日志的时钟经常跑快，不钳的话衰减权重会超过最新桶。
5. **空证据的时间戳**：`interaction_count == 0` 时 `first/last_contact_unix = 0`，是「没有往来」的哨兵而不是 1970 年；`explain_zh` 走独立分支，不会输出「1970」（`an_empty_tie_says_so_plainly`）。若并入 `soul-graph`，建议改成 `Option<Timestamp>`。
6. **`TieScore` 新增字段**：`any_direct`（本轮任务书要求）、`silent_days`、`as_of_unix`、`detail`。`detail` 是一个三变体枚举（原始计数 / 折算量 / 降档前的档），让 `explain_zh(&TieScore)` 能说出真正的判据，而不必回证据库再查一次 —— 这是 Round 1 §3.5 记录的那个洞。四个字段都不含正文、姓名或会话内容。

## 6. T1 / T2 墓碑（可执行）

`src/tombstones.rs` 是 `#[cfg(test)]`，产品路径不可达，也不参加消融。它存在是因为 fable-a 验收门 G2 要求否决可追溯到具体夹具，而文档会腐烂、测试不会：

| 墓碑测试 | 钉住的否决理由 |
|---|---|
| `t1_calls_one_afternoon_a_strong_tie_which_is_why_it_is_not_a_product_path` | F02：T1 无跨日门，一下午 20 条 mass ≈ 19.85 → strong。三个存活候选都判 moderate |
| `t1s_group_weight_does_not_stop_a_group_chat_reaching_strong` | P0-3：0.4 的群权重挡不住 50 条近期群聊。**降权解决不了定义问题** |
| `t2_lets_one_axis_buy_back_another` | T2 的补偿性求和：10 次挤在 2 天 → 3+3+2 = 8 → strong |
| `t2_moves_a_band_that_no_new_evidence_touched` | 同一份证据、没有任何新计数，档位随时间跳变，且解释必然是比较性的 |

这也是本包里唯一出现浮点的地方 —— 它是展品。产品路径无浮点由 `the_product_path_contains_no_floating_point` 读源码断言。

## 7. 硬约束自检

| 约束 | 状态 | 证据 |
|---|---|---|
| 推断可解引用证据 | 输入即证据行，输出只含对这些行的计数 | `every_rule_agrees_about_the_counts_whatever_it_decides_about_the_band` |
| 禁 score / percentile / 诊断词 | 输出只有三档 + 计数；解释无拉丁字母、无量化词、无诊断词 | `tests/explain_zh.rs`（14 测） |
| 第三人数据不出本机 | 输入无正文无姓名，只有 id / 方向 / 时间 / 场合 | `Interaction` 定义 |
| 可用中文解释强/中/弱 | 每档每规则都有句子，复述可核对的次数、天数、日期、距今天数 | `every_explanation_names_the_numbers_the_user_can_recount` |
| 可 orphan | 纯函数无状态；删证据重跑只会降档 | `forgetting_evidence_can_only_lower_a_band`（全夹具 × 全规则 × 4 种截断） |
| 禁读墙钟 | `as_of` 是参数；包内不存在 `SystemTime` | `tests/as_of_discipline.rs`（9 测） |
| 无 unsafe / 无网络 / 零依赖 | `forbid(unsafe_code)`；`cargo tree` 一行 | `TEST_LOG.txt` |
| O(n) | 单遍扫描 + 有序集合；全图一次分组 | `a_large_ego_network_is_scored_in_one_pass` |
| Goal 1 保真 | T0 与字符串版 `Tally::band` 6500 组随机比对全等；重构后的活跃日集合仍与 Goal 1 逐条相同 | `tests/goal1_fidelity.rs` |

## 8. 交给 Round 3 的三件事

1. **仲裁 `revived_after_gap` 的判卷**（ABLATION.md §4）。这是本轮唯一能反转推荐的格子。若判 moderate 正确，胜者换成 T3R，且需要给 T3R 补一句「为什么上周聊过的人只算中等」的话术。
2. **把 `as_of` 的取法写进规范**：全库统一一个值，禁止逐 peer 取。`a_peer_local_as_of_would_hide_every_dormant_tie` 已经把错法的后果钉死了。
3. **降档常量 180 / 360 需要一次产品侧确认**。本轮它们是从 T3R 的桶边界借来的（复用同一组数字是刻意的：两个候选的时间观应当一致，才能公平比较）。若产品认为「半年不联系就降档」太急，改的是 `constants.rs` 的两行，不是规则。

另有两处不属于算法胜者、但必须有人接的债，本轮只记录不实现：R1-SYNTHESIS P0-1（Goal 1 导入把群聊一条发言扇出成对每个成员一条 outgoing —— 算法层已经用 venue 门闩封住了「群聊升强档」，但**扇出仍然会让计数虚高**，那是管道债）与 P0-2（人脉图不可纠正）。
