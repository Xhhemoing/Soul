MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 2 / opus-b — A0 补锁、A1 去留、A2 降为纯渲染器

产出：`.agent_workspace/round2/opus-b/soul-algo-trait/`（edition 2021，rust-version 1.83，**零依赖**，`unsafe_code = "forbid"`，`#![deny(missing_docs)]`）。
81 测全过，`cargo fmt --check` 与 `cargo clippy --all-targets -D warnings` 干净。逐条见 `TEST_LOG.txt`。
未做 `git commit`（按派发要求）。

## 0. 三句话结论

1. **A0 的锁现在两条路径都成立。** Goal 1 的 `intake` 绕过 `axis_is_locked` 直接调 `place_axis`，重跑问卷会挪走已纠正的轴、还留着锁标记；`apply_intake` 补上这一检查，被挡下的答案进 `IntakeReport::ignored`，证据行照写。
2. **A1：KEEP**，但不占「1–2 名额」，它是 A0 内部的升档规则。F16 / F16b 双线通过，独立性是纯函数可判定。**`EMPTY_ON_V01 = false`** —— 在 v0.1 只有问卷+纠正的数据面上它**会**触发，触发场景见 §2.4，而那个场景恰恰是我建议 Round 3 关掉的那一种。
3. **A2 现在真的是渲染器。** 它不再持有 3/10/3 的任何副本，band 直接从 `TieScore` 抄下来。代价要说清楚：**A2 因此不再是一个独立算法**，它没有任何自主判断，所以「保留 1 或 2 个」实际上落在「一个人脉算法」上（见 §4）。

## 1. A0 — 锁的补丁

### 1.1 缺陷复述（Round 1 P1-5）

`profile_service.rs::intake` 对每个答案无条件调 `place_axis(..., locked_by_user: None)`。`None` 的意思是「不动锁标记」，于是：position、band、`evidence_ids` 全被覆盖，`locked_by_user` 仍是 `true`。这比「锁失效」更糟——UI 上那把锁还在，用户以为纠正还在生效。`record_axis_inference` 有 `axis_is_locked` 检查，`intake` 没有，两条写路径的规则不一致。

### 1.2 `apply_intake`

```rust
pub fn apply_intake(
    prior_log: &[EvidenceRef],
    answers: &[IntakeAnswer],
    mode: WriteMode,
) -> IntakeReport
```

- 逐条按输入顺序应用；轴上有用户纠正 → 不动轴，记入 `IntakeReport::ignored`（带 `evidence_id / axis / position / IntakeSkip::AxisLockedByUser`，`as_str()` 给审计行用，不含答案正文）。
- **答案行照样写进 `log_after`。** 用户确实答了这一题，那是关于用户的事实；锁挡的是轴动，不是证据表长大。形状与 `record_axis_inference` 一致（推断存下来但不应用）。
- 一个轴锁住不冻结其他轴（`a_lock_on_one_axis_does_not_freeze_the_others`）。
- 后来的**纠正**仍能改锁定轴：锁挡机器和问卷，不挡用户本人（`a_later_correction_still_moves_a_locked_axis`）。

关键的一条回归性质：**命令式的 `apply_intake` 与重放 `a0_all_axes_with(log_after, mode)` 结果全等**，在两种 mode × 全部 14 个夹具上验证（`intake_matches_replay_on_every_fixture_and_mode`）。这正是 Goal 1 破掉的不变量——它的 `intake` 让「按日志重放」与「按命令写入」得出不同的轴。补丁的正确性判据就是这条等式重新成立。

### 1.3 遗忘

`forgotten` 行在任何聚合之前被丢掉，A0 与 A1 都是。遗忘掉一条纠正后轴自己退回问卷答案，锁一并消失（`forgotten_rows_are_dropped_before_anything_is_aggregated`）；整条日志遗忘后五个轴全回 `Unknown / Band::None`（`a_fully_forgotten_axis_returns_to_unknown`，两种 mode × 全夹具）。没有任何算法持有遗忘触及不到的累加量。

### 1.4 后写覆盖：契约 vs 修复

`WriteMode::LastWriteWins` 是 **A0 回归契约**，钉在 `last_write_wins_is_the_a0_contract`：一条 Weak 推断压在 Moderate 问卷答案上，轴降到 Weak，而且问卷那条**不再被引用**。测试里写明这是记录缺陷、不是背书，作用是让以后任何对 A0 的改动都必须是一次自觉的基线变更。

不覆盖的路径给了两条，任选其一即可满足「plus a mode or A1 path that does not clobber」：

| | 同方向新证据 | 反方向更弱证据 | 反方向同档或更强 | 锁 |
|---|---|---|---|---|
| `LastWriteWins` | 替换，只引自己 | 替换（缺陷） | 替换 | 优先 |
| `NoDowngrade` | 加入引用列表，band 取最强单条 | `RefusedWeaker`，留在 shadow | 替换 | 优先 |
| A1 | 归组聚合 | 两个方向都有 → Mixed/Weak | 同左 | 优先 |

`NoDowngrade` 只加了一个 `ApplyResult::RefusedWeaker`，没有引入任何数值权重；「更弱的反方向证据不能顶掉更有依据的一侧」是能用中文一句话说清的规则。

## 2. A1 — 去留：**KEEP**（不占名额，作为 A0 的升档规则）

### 2.1 独立性键

按 `CANDIDATE_SPEC.md` 冻结的定义实现：`IndependenceKey { kind: EvidenceKind, utc_day: i64 }`，`utc_day = div_euclid(recorded_at_unix, 86_400)`。用欧几里得除法而不是截断除法，1970 年之前的时间戳才会落在它真正所属的那一天（`the_day_key_works_either_side_of_the_epoch`）；否则「同一天」在纪元两侧含义不同，那就不叫键。

同键的任意多行 = **一组**。组内方向不一致 → 该组记 `Mixed`，两边都不支持（`a_source_that_contradicts_itself_in_a_day_supports_neither_side`）。

### 2.2 F16 / F16b

| 夹具 | Given | Then | 测试 |
|---|---|---|---|
| **F16** | 同轴同方向问卷，同一 UTC 日重填 5 次 | 1 组，band 仍 `Moderate`，五条**仍全部被引用**（它们是证据，只是不构成五个理由） | `f16_five_refills_in_one_afternoon_do_not_upgrade` |
| F16 ×10 | 同上，重填 50 次 | 仍 1 组，仍 `Moderate` | `f16_holds_however_many_times_the_questionnaire_is_refilled_in_a_day` |
| **F16b** | 问卷（d1）+ 三条不同日的行为证据（d5/d20/d40，同方向） | 4 组 → `Strong`，解释能报出组数、日数、来源类数 | `f16b_three_independent_agreeing_groups_reach_strong` |
| 边界 | 恰好 3 组 / 恰好 2 组 | 3 → `Strong`；2 → 最强单条封顶 `Moderate` | `exactly_three_groups_is_the_upgrade_boundary` |
| 同日多源 | 同一天的 message + app_usage + aggregate | 3 组 → `Strong`（键是 (kind, day)，不是 day） | `three_different_sources_on_one_day_are_three_groups` |

**没有设每组的 band 下限**：`A1_GROUP_FLOOR = Band::Weak`，即三组 Weak 也能到 Strong。这是冻结规范的读法，也是 F16b 的生存条件（行为佐证本身通常只值 Weak）。理由：**把关的是独立性，不是单条强度**——在 (kind, utc_day) 键下，三组 Weak 已经意味着三个不同日子或三个不同来源。常量是 public 具名的，Round 3 想抬高不用满文件找比较符。

### 2.3 分歧规则：比冻结规范更保守（留痕）

规范写「两个方向的 independent 组数**均 ≥ 2** → mixed」。本实现取 **≥ 1**（`A1_GROUPS_FOR_DISAGREEMENT = 1`）。

两个理由：

1. 规范在 1 对 1 时**没有定义结果**。它只说 `Strong if ≥3` / `Moderate if ≥1`，两个方向各 1 组时哪一个方向胜出，字面上没有答案。留一个洞给实现者填，填法就会变成隐式多数决。
2. 保守方向是安全方向。「两边都有依据 → 记成两端都有」永远比「按条数取多数」更容易向用户交代，而 `Position::Mixed` 本来就是词汇表里的合法值，不是降级。

不改任何夹具期望：F16c（2 对 2）在两种读法下都是 `Mixed`，我这条只是把未定义区间也判成 `Mixed`。`a_majority_never_settles_a_disagreement` 钉住四比一仍是 Mixed。

### 2.4 `EMPTY_ON_V01 = false`

**结论：在 v0.1 只有问卷+纠正的数据面上，A1 会触发。** 这不是推理，是一条跑着的测试（`a1_does_fire_on_v01_data_when_the_questionnaire_is_refilled_across_days`）。

**会触发的确切场景：**

> 同一条轴，用户在**三个不同的 UTC 日**重填问卷，三次都答同一个方向（例如 `social_energy = leans_high`），且这条轴从未被 `correct_axis` 纠正过。
> 独立组 = `(questionnaire, d0)`、`(questionnaire, d30)`、`(questionnaire, d90)` = 3 组 ≥ `A1_GROUPS_FOR_STRONG` → band 从 `Moderate` 升到 `Strong`。
> 不需要任何行为证据、任何导入、任何 v0.2 能力。三次进入问卷即可。

支撑这个判断的事实（也有测试）：v0.1 能对一条 trait 轴产生证据行的写路径只有两条，`intake`（`EvidenceKind::Questionnaire`）与 `correct_axis`（`EvidenceKind::UserCorrection`）。`record_axis_inference` 在 Goal 1 里只有测试调用它，没有生产调用方（`git grep AxisProposal` 在 `crates/` 与 `soulcore` 下只命中 `service.rs`、`lib.rs` 的 re-export、`commands/profile.rs` 的转发和三个测试文件）。`EvidenceKind::reachable_for_axis_in_v01` 把这条事实变成函数，`only_two_evidence_kinds_can_reach_an_axis_in_v01` 让第三个生产者出现时有东西会红。

**这个触发是退化的，而且我不建议留着它。** 「同一份问卷、同一个人、同一段记忆，换个日子再答一遍」在任何意义上都不是独立观测——独立的是**工具**，不是日历。(kind, utc_day) 键把「同源不同日」当独立，对行为证据是对的（周二的一次互动和周五的一次互动确实是两件事），对自陈量式的问卷是错的。而 `Strong` 在这个产品里的含义是「用户看着结论纠正过」或「几件互相独立的事互相印证」，不是「用户说了三遍」。

**给 Round 3 的建议（不是本轮擅自改的默认）：** 把默认独立性变体从 `A1Independence::KindAndDay` 改为已实现并已测的 `A1Independence::TwoKindsAcrossDays` —— 键仍是 (kind, utc_day)，只多一条「升档的那几组至少跨 2 种 kind」。效果：

- `the_strict_variant_cannot_fire_on_v01_data`：上面那个三次重填场景回落到 `Moderate`。在该变体下 **`EMPTY_ON_V01 = true`**，且是可证的——v0.1 对轴只能产出两种 kind，其中 `UserCorrection` 一出现就先锁轴、根本轮不到计数，所以升档所需的第二种 kind 在 v0.1 不可达。
- `the_strict_variant_still_passes_f16b`：F16b 仍升 `Strong`（问卷 + message，2 种 kind）。生存线不受影响。

等价的替代改法（未实现，一句话记档）：保持 `KindAndDay`，但规定 `Questionnaire` 这一 kind 无论跨多少天最多贡献一组——「仪器是单位，日子不是」。语义更精准，但要给键加一张例外表，可测性不如前者干净。

### 2.5 为什么是 KEEP 而不是 KILL

`CANDIDATE_SPEC.md` 给 A1 列了四条消杀条件，逐条对账：

| 消杀条件 | 结果 |
|---|---|
| F16 挂 → 杀 | 过（含 50 次重填的加强版） |
| F16b 过不去 → 杀 | 过（含严格变体下仍过） |
| 「独立性」仍需人肉判断 → 杀 | 不需要。`(kind, div_euclid(t, 86400))` 是纯函数，无浮点、无随机、无墙钟 |
| v0.1 数据面上永不触发 → 归档「正确但空转」 | 默认变体下**会**触发（§2.4），所以连「空转归档」都不适用；建议改默认后才变成受控空转 |

另外两条本轮才拿到的正面证据：

- A1 顺手解决了 A0 的后写覆盖缺陷。它每次从整个存活集重新推导，没有「已应用」的概念，所以一条 Weak 猜测抹不掉问卷的引用（`a1_is_the_path_that_does_not_clobber`：A0 引用 `[2]`，A1 引用 `[1, 2]` 并记 `Mixed`）。
- A1 是**唯一**能在没有用户纠正的情况下到达 `Strong` 的候选（`only_a1_reaches_strong_without_a_correction` 在全夹具上反证 A0 做不到）。若 v0.2 接入行为证据而 A1 不在，那么「多条独立证据互相印证」这件事就永远没有出口，用户唯一的升档手段只剩下手动纠正。

**A1 不占「保留 1–2 个」的名额。** 它没有独立的数据面、没有独立的输出类型、不能脱离 A0 存在——它是 A0 聚合阶段的一条规则。这与 `ACCEPTANCE.md` 对 A0 的定性一致（产品锁基底，不参与淘汰）。

## 3. A2 — 纯渲染器

### 3.1 它不再算 3/10/3

Round 1 的 A2 自带 `MODERATE_MIN_INTERACTIONS` / `STRONG_MIN_INTERACTIONS` / `STRONG_MIN_ACTIVE_DAYS`，理由是「让摘要和边对同一组计数不打架」。那是对真问题的错解法：两份规则就是两条规则，T 族消融一改阈值，摘要立刻开始和它所描述的那条边互相矛盾。

现在 `a2_render(&TieScore) -> PersonnelSummary` 是**唯一**入口，唯一输入，band 从 `TieScore::band` 抄下来，A2 不做任何比较、不封顶、不降档。每一条 bullet 携带的 band 就是边的 band，一字不改（`the_band_on_every_bullet_is_the_band_it_was_handed`，跨全部夹具 × 四个 band）。

`as_of_unix` 是 `TieScore` 的字段而不是函数参数，这样「输入是一个 TieScore-like 结构」是字面为真的，函数也拿不到钟。它的语义是 `R1-SYNTHESIS.md` 定的「数据内最大时间戳，禁读墙钟」。

**这条性质有源码级的守卫。** `a2_defines_no_band_thresholds` 用 `include_str!("../src/a2.rs")` 读源文件，命中 `"STRONG_MIN"`、`"MODERATE_MIN"`、`"MIN_INTERACTIONS"`、`"MIN_ACTIVE_DAYS"`、`">= 10"`、`">=10"` 任一即失败。守卫不是空转：`TEST_LOG.txt` 记录了一次故意注入 `const STRONG_MIN_INTERACTIONS: u32 = 10;` 的变异，测试如期变红，随后回滚并校验源文件逐字节还原。

### 3.2 输出

固定顺序：方向 → 计数 → 场合（仅群聊时才出）→ 近因 → 沉寂（条件）→ 归档。归档句放最后，读起来才像「由上面那些数得出的结论」。

- **弱档不得声称强关系。** 实现上更硬一点：`Band::Strong` 渲染成「归在「强」一档」，任何 band 下都不产生「强关系」三个字；「强关系」「弱关系」进了 `PEER_CLAIM_DENYLIST`，`no_band_ever_claims_a_relationship` 在全夹具 × 四 band 上扫。另有 `a_weak_band_never_reads_as_a_strong_relationship`：一条又厚又互惠、但被打成 Weak 的边，输出里连「强」字都不出现。
- **沉寂句是描述，不是新档。** `(as_of − last_contact) > 180 天` 时追加一句「已经 N 天没有新的往来了；下面的归档说的是过去的记录，不是现在的联系频率」。`dormancy_is_a_sentence_not_a_demotion`：`dormant_but_strong` 夹具在 400 天沉寂下所有 bullet 的 band 仍是 `Strong`。边界是**严格大于**：恰好 180 天不出这句（`the_dormancy_threshold_is_strictly_more_than_the_constant`）。这条常量是 A2 唯一拥有的数，而它只决定一句话是否出现。
- 未来时间戳（导入日志时钟跑快）钳到 0 天，读作「就在最新的记录当天」，不产生负数天。

### 3.3 空 `evidence_ids` 的处理 —— 决定与理由

**决定：不出任何 bullet。** 派发允许的另一种做法（单条「还看不出」+ `Band::None`）被否掉，理由是这个类型里每条 bullet 都承诺 `evidence_ids` 非空，一条「什么都不引用、什么都不说」的 bullet 会成为「无证据不得落库」的第一个例外。差别在写边界上是实的：空摘要不落库，占位 bullet 会落库。

那句话仍然存在，作为 `PersonnelSummary::nothing_to_say_zh()` 常量——它是调用方的文案，`PersonnelSummary::is_empty()` 是调用方判断何时显示它的依据。`empty_evidence_means_no_bullets_at_all` 钉住 `empty` 与 `counts_without_citable_evidence` 两个夹具（后者有 9 次往来但一条证据都引不出来，照样零 bullet）。

区分清楚：**`Band::None` 但有证据**的边是另一回事，照常渲染，归档句说「还看不出该归在哪一档」（`counts_with_no_band_yet_still_render`）。空的是证据，不是档。

### 3.4 A2 还算不算一个「算法」

不算。它现在没有任何自主判断：给定同一个 `TieScore`，输出是它的纯函数；band 是抄的，顺序是固定的，唯一的自有常量只控制一句话出不出。这正是 `R1-SYNTHESIS.md` 预留的那个判定——「A2 若完全无独立判断，可降为『不算独立算法』」。

我把这条明确落下来，因为它有后果：**「最终保留 1 或 2 个」实际上会落在「一个人脉算法（T3 / T3R / T4 三选一）」上**，A0 是产品锁基底不参与淘汰，A1 是 A0 里的一条规则，A2 是渲染器。这不是坏结果——它意味着 v0.1 只有一处定义 band，其余全是消费者，词汇表单源。但父代理在写最终名单时应当按这个形状写，而不是硬凑成两个。

## 4. A3 — 保持拒绝

`a3_from_message_text` / `a3_from_messages` 恒返回 `Err(A3Refused)`，理由字符串稳定且不含任何正文。`A3_ALGORITHM_ID` 注册为 rejected，`no_live_algorithm_stamps_the_rejected_identifier` 在全夹具上验证 A0/A1 产出的 state 都不带它。本轮对 A3 的唯一改动是文档里加一句「Round 1 已判、Round 2 不动」——这个模块的价值就在于它不变。

## 5. 消融表（由 `tests/a0_vs_a1.rs::the_ablation_table_is_what_the_report_says_it_is` 跑出，不是誊抄）

| 夹具 | A0 LWW | A0 NoDowngrade | A1 KindAndDay | A1 TwoKinds |
|---|---|---|---|---|
| `questionnaire_only` | Moderate | Moderate | Moderate | Moderate |
| `questionnaire_then_correction` | Strong 🔒 | Strong 🔒 | Strong 🔒 | Strong 🔒 |
| `questionnaire_refilled_after_correction` | Strong 🔒 | Strong 🔒 | Strong 🔒 | Strong 🔒 |
| `f16_five_refills_same_day` | Moderate | Moderate | **Moderate** | Moderate |
| `f16b_..._three_independent_days` | Weak | Moderate | **Strong** | **Strong** |
| `three_questionnaire_refills_on_three_days` | Moderate | Moderate | **Strong** ⚠ | Moderate |
| `weak_inference_over_moderate_questionnaire` | **Weak** ⚠ | Moderate | Weak(Mixed) | Weak(Mixed) |
| `contradiction` | Moderate | Moderate | Weak(Mixed) | Weak(Mixed) |

🔒 = 锁定，四种读法完全一致（`neither_algorithm_ever_moves_a_locked_axis` 在全夹具上验证）。
⚠ 两处：A0 LWW 那一格是 §1.4 的后写覆盖契约；A1 KindAndDay 那一格是 §2.4 的退化触发。两处各自的修法都在表里同一行。

## 6. 评分（1–5，相对 Round 1 的变动加粗）

| | C1 产品锁 | C2 可测 | C3 可解释 | C4 隐私 | C5 稳健 | C6 成本 | C7 SOTA | C8 创新必要 |
|---|---|---|---|---|---|---|---|---|
| A0 (+ patch) | 5 | 5 | 5 | 5 | **5**（intake 绕锁已堵） | 5 | 4 | — 基底 |
| A1 | 4 | **5**（F16/F16b 均有测；独立性纯函数） | **5**（能报组数/日数/来源数） | 5 | 4（默认变体有退化触发） | 5 | 4 | **3**（唯一的非纠正升档路径，但 v0.1 内净胜有限） |
| A2 | 5 | 5 | 5 | 5 | **5**（band 不再有第二来源） | 5 | — | — 渲染器，不参与 |
| A3 | 2 | 2 | 1 | 1 | 1 | 3 | 4 | 1 → 维持击毙 |

C6 说明：A0 是每轴一次线性扫；A1 多一次分组，组数上界是行数，用 `Vec` 线性查找而不是哈希表——证据行数是人类量级（问卷 5 条/次），常数因子比哈希小，且没有哈希就没有迭代顺序不确定性。全程无 O(n²) 全图操作。

## 7. 与上游文档的分歧（留痕）

1. **A1 分歧阈值 ≥1 而非规范的 ≥2**（§2.3）。不改任何夹具期望，只把规范未定义的 1 对 1 区间判成 `Mixed`。
2. **A1 默认变体建议改为 `TwoKindsAcrossDays`**（§2.4）。本轮**未**擅自改默认，代码默认仍是规范的 `KindAndDay`；建议以 REPORT 形式提交，由父代理裁决。
3. **A2 的 band 不再分条封顶**。Round 1 的近因句/群聊句封顶 `Moderate` 也是一种自主判断，一并去掉了。
4. **`量表` 移入 `DIAGNOSTIC_DENYLIST`** 以逐字匹配派发给的十词清单（它同时仍在 `NUMERIC_RATING_MARKERS` 里，两张表各自完整可读）。`强关系` / `弱关系` 新增进 `PEER_CLAIM_DENYLIST`。
5. **`EvidenceRef` 增加 `kind` 与 `recorded_at_unix` 两个字段**。A1 的键要求，A0 完全不读（`a0_ignores_the_timestamp_entirely`：整体平移十年不改任何结果）。

## 8. 给 Round 3 的四件事

1. **裁决 A1 的默认独立性变体。** `KindAndDay`（规范原样，`EMPTY_ON_V01=false`，三次跨日重填即升 Strong）还是 `TwoKindsAcrossDays`（`EMPTY_ON_V01=true`，v0.1 受控空转，v0.2 自动激活）。两种都已实现、已测、可一行切换。我的建议是后者，理由在 §2.4。
2. **裁决 A0 的默认 write mode。** `LastWriteWins` 是现状与回归契约；`NoDowngrade` 修掉后写覆盖但改变了基线语义。若最终同时保留 A1，A0 的 mode 影响面会小很多——A1 那条路径本来就不覆盖。
3. **`TieScore` 的最终字段形状。** 本包用的是 `band + 5 个计数 + any_direct + last_contact_unix + as_of_unix + evidence_ids + last_contact_evidence_id`。`any_direct` 是 opus-a 在 Round 1 提的建议，这里当成已采纳来实现；若 T 族胜者最终不带 `any_direct`，A2 的群聊句就得砍掉——**不能**让 A2 自己去查证据补出这个事实，那等于把纯渲染器的性质还回去。
4. **别把 A2 硬凑成第二个算法**（§3.4）。

## 9. 硬约束自检

| 约束 | 本包状态 |
|---|---|
| 推断必须有可解引用 `evidence_ids` | `neither_algorithm_ever_claims_without_citing`、`every_bullet_cites_something`；空证据 → 零输出 |
| 禁 score / percentile / 诊断词 | 三张表、两个入口，扫全部可产出字符串（`denylist_scan.rs` 11 测）；纯计数句不误伤 |
| 第三人数据不出本机 | A0/A1/A2 的输入里没有正文、没有姓名、没有可解析到人的标识；A3 拒绝 |
| 可向用户解释 | A1 有 `explain_zh()`（组数/日数/来源类数，全部可手数）；A2 每句都是计数的复述；无浮点、无 `exp`、无权重 |
| v0.1 无 E0、无 key 不依赖 LLM | 零依赖、零网络、零 IO |
| 不承诺临床效度，可纠正可遗忘 | 锁两条路径成立；遗忘后重放全夹具回落 |
| Linux 可测，不依赖 Windows API | 纯 std |
| 确定性 | 无墙钟、无随机、无哈希、无并行归约；迭代顺序即输入顺序 |
