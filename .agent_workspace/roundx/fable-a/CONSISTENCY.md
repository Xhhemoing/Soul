MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round X 交叉验证（fable-a）：DECISION.md ↔ crates 一致性审计

审计对象：`docs/algorithms/DECISION.md`（含同批 `REJECTED.md`、`COPY_ZH.md`）对照 `crates/soul-algo-tie`、`crates/soul-algo-trait`，基准为提交 `752cdec`（"Freeze T4D and A0; land reference crates and Round 3 docs"）。

审计方法：逐常量、逐规则读源码比对；运行全部**已提交**测试目标（工作树内另有其他 Round X 槽位的未提交探针文件，且在本审计期间被其作者并发增删，故不计入本审计的通过/失败统计；其发现在第 6 节独立复核）。

已提交测试结果：**228 通过，0 失败**（soul-algo-tie：lib 60 + ablation 20 + as_of_discipline 10 + direct_gate 8 + explain_zh 17 + goal1_fidelity 5；soul-algo-trait：a0_lock 19 + a0_vs_a1 8 + a1_independence 27 + a2_render 22 + a3_refusal 6 + denylist_scan 14 + frozen_defaults 12）。

---

## 1. 常量表逐条比对（DECISION 第 3 节 vs 代码）

| DECISION 常量名 | DECISION 值 | 代码常量名 | 代码值 | 代码位置 | 判 |
|---|---|---|---|---|---|
| `MODERATE_MIN_INTERACTIONS` | 3 | `MODERATE_MIN_INTERACTIONS` | `u64 = 3` | `soul-algo-tie/src/constants.rs:24` | **一致**（名+值） |
| `STRONG_MIN_INTERACTIONS` | 10 | `STRONG_MIN_INTERACTIONS` | `u64 = 10` | `soul-algo-tie/src/constants.rs:29` | **一致**（名+值） |
| `STRONG_MIN_ACTIVE_DAYS` | 3 | `STRONG_MIN_ACTIVE_DAYS` | `u64 = 3` | `soul-algo-tie/src/constants.rs:35` | **一致**（名+值） |
| `DEMOTE_ONE_BAND_DAYS` | 180，闭区间 `>=` | `DEMOTE_AFTER_SILENT_DAYS` | `i64 = 180`，`>=`（`recency.rs:32`） | `soul-algo-tie/src/constants.rs:51` | **值与区间语义一致；名漂移**（见 D1） |
| `FORCE_WEAK_DAYS` | 360，闭区间 | `WEAK_AFTER_SILENT_DAYS` | `i64 = 360`，`>=`（`recency.rs:30`） | `soul-algo-tie/src/constants.rs:54` | **值与区间语义一致；名漂移**（见 D1） |
| `DORMANT_NOTE_DAYS` | = `DEMOTE_ONE_BAND_DAYS`（即 180），"共用同一常量，不得分裂" | `DORMANT_AFTER_DAYS` | `i64 = 180`，**独立定义**，比较用严格 `>`（`a2.rs:185`） | `soul-algo-trait/src/a2.rs:91` | **值一致；定义已分裂 + 名漂移**（见 D2、D3） |
| `GROUP_ONLY_CEILING` | Moderate，仅 T4 使用，T4D 无此常量 | `GROUP_ONLY_CEILING` | `Band = Band::Moderate` | `soul-algo-tie/src/constants.rs:48` | **一致**：名+值匹配；仅 `t4.rs` 消费；`t4d.rs::counts_band` 无封顶路径，仅群聊者经一对一互惠门自然落 Weak |

区间语义由测试钉死：`constants.rs::the_frozen_recency_edges_are_closed_intervals_at_180_and_360`、`recency.rs::the_edges_are_closed_and_land_on_whole_days`（179→不降、180→降一档、359→降一档、360→Weak）、`t4.rs::the_demotion_boundaries_are_exact_whole_days`、`ablation.rs::the_demotion_boundaries_land_where_the_constants_say_under_both_rules`。`>180` 写法未出现于产品路径。

代码中另有 DECISION 表未列的常量（不属冻结面，如实记录）：`SECONDS_PER_DAY = 86_400`（两 crate 各定义一次）；A1 的 `A1_GROUPS_FOR_STRONG = 3`、`A1_GROUPS_FOR_DISAGREEMENT = 1`、`A1_GROUP_FLOOR = Weak`、`A1_DEFAULT_INDEPENDENCE = TwoKindsAcrossDays`、`A1_EMPTY_ON_V01 = true`（A1 未保留，空转已被 `a1_independence.rs` 27 项测试证明）；A2 的 `A2_STATEMENT_KEYS`。另：COPY_ZH §4 P2 的方向句 **2:1 阈值**是一个只存在于 COPY_ZH 的数字阈值，既不在 DECISION 常量表，也不在任何代码中（见 D6）。

## 2. T4D 冻结判档规则逐条比对（DECISION 第 3 节代码块）

| DECISION 条款 | 代码实现 | 判 |
|---|---|---|
| `direct_* = venue 为一对一的行的计数` | `Tally` 四分计数 `direct_out/direct_in/group_out/group_in`，`direct_days` 单独集合（`types.rs`） | 一致 |
| Strong iff 一对一双向 ∧ 一对一次数 ≥10 ∧ 一对一自然日 ≥3 | `T4D::observed` 喂 `is_direct_reciprocal / direct_count / direct_active_day_count` 给共享阶梯 `gate::band`（`t4d.rs:72-86`） | 一致 |
| Moderate iff 一对一双向 ∧ 一对一次数 ≥3 | 同上，`gate.rs:46-58` | 一致 |
| Weak otherwise | 同上；一对一单向恒 Weak（`gate.rs` 首门 + `one_sided_can_never_leave_weak`） | 一致 |
| 降档时钟：沉寂 = max(0, as_of − **任一场地**最后往来) / 86400，整数向下取整 | `Tally::silent_days` 读 `last_contact`（任一场地），`age_days` = `max(0, Δ)/86400`（`types.rs:262-269, 442-448`）；`dormant_direct_group_ping_yesterday` 实证群聊 ping 阻止降档（Strong，silent=1） | 一致 |
| ≥360 → Weak；≥180 → 降一档 | `recency::demote`，闭区间 | 一致 |
| 空观测 → Weak，不进时间运算 | `silent_days` 空时返 0；空日志 Weak、`silent_days == 0`（`nothing_observed_is_weak_and_not_demoted_from_anything`，两规则各一） | 一致 |
| direct-only 时钟变体作废 | 代码无此路径；两规则共用同一 `recency` 模块与同一 `last_contact`；`last_direct_contact_unix` 仅上报、从不判档 | 一致 |
| as_of 纪律：全库单值、调用方传入、禁 per-peer、不读墙钟 | 一切入口带 `as_of_unix` 形参；crate 零依赖、无 `SystemTime`；`as_of_max` 为调用方辅助而非缺省路径；per-peer 陷阱由 `a_peer_local_as_of_would_hide_every_dormant_tie` 钉死（per-peer 下 dormant_2019 错判 Strong，全库下判 Weak、沉寂 2632 天，与 DECISION 数字吻合） | 一致 |

## 3. 保留清单（DECISION 第 2 节）逐项比对

| 保留项 | 代码状态 | 判 |
|---|---|---|
| T4D 发布默认 | `TieAlgo::DEFAULT = T4D`，`#[default]`，自由函数 `score` 走 T4D（`lib.rs`），`the_default_rule_is_t4d` 钉死 | 一致 |
| T4 档内回退，行为字节兼容 | `t4.rs` 全量保留，`GROUP_ONLY_CEILING` 消费者，`TieAlgo::T4` 一行可切 | 一致 |
| A0 问卷+纠正锁、intake 不绕锁、与 replay 全等 | `apply_intake` 经同一 `place`；`RefusedLocked` 上报不静默；`a0_lock.rs::intake_matches_replay` 等 19 项通过；`A0_ALGORITHM_ID` v2 未动、默认 LastWriteWins 冻结（`frozen_defaults.rs`） | 一致 |
| A2 纯渲染器、禁第二套阈值、分列句只消费不推导 | `a2.rs` 无任何 band 阈值；源码级扫描测试（`the_a2_source_still_holds_no_threshold`，禁 `STRONG_MIN`/`>= 10` 等字样）；`venue_split` 要求双数俱在否则整句不出（`venue_split_bullet`），无从证据重算的路径 | 一致 |
| A1 默认 TwoKindsAcrossDays、v0.1 空转 | `A1_DEFAULT_INDEPENDENCE`、`A1_EMPTY_ON_V01 = true`，且由 `a1_can_fire_on_v01_data` 从数据面推导而非断言；问卷三天三填只到 Moderate（`NeedsASecondSource`） | 一致（语义小注见 D7） |
| 纯函数、零重依赖、forbid(unsafe_code)、不读墙钟、整数-only | 两 crate `[dependencies]` 为空、`unsafe_code = "forbid"`；产品路径无浮点由 `the_product_path_contains_no_floating_point` 读源钉死 | 一致 |

## 4. REJECTED.md（墓碑）比对

- T0：仅存于 `testing/oracle.rs`（测试 oracle），非 `TieAlgo` 变体，`from_id("T0") == None`；`goal1_fidelity.rs` 保留等价锚测试。一致。
- T1 / T2 / T3R：`src/tombstones.rs` 为 `#[cfg(test)]`，每座墓碑挂着击毙夹具（T1 afternoon_20 / group_only_50；T2 补偿性；T3R `revived_after_gap` 复活对价 + `t3r_would_not_have_fixed_the_venue_latch_either`）。T3R 权重为四分位 4/2/1/0，milliscale 已废。一致。
- T3：即 `counts_band` 无近因运行（`t3_cannot_see_that_seven_years_went_by`）；无独立产品路径。一致。Round 2 opus-a 的群聊 ≥5/≥2 附加门未出现在代码。一致。
- A3：`a3_from_message_text` / `a3_from_messages` 恒返 `Err(A3Refused)`，正文不读不存不哈希；`a3_refusal.rs` 6 项通过；`A3_ALGORITHM_ID` 以 `.rejected` 结尾并被 `is_rejected_algorithm` 识别。一致。

## 5. 漂移清单

**D1（P1，名漂移）**：DECISION 常量表的规范名 `DEMOTE_ONE_BAND_DAYS` / `FORCE_WEAK_DAYS` / `DORMANT_NOTE_DAYS` 在全仓库代码中**不存在**；代码（含 DECISION 自己援引的 `round3/fable-a/t4d-verify`）用 `DEMOTE_AFTER_SILENT_DAYS` / `WEAK_AFTER_SILENT_DAYS` / `DORMANT_AFTER_DAYS`。值与语义全等，行为无差；但同批冻结文档与参考实现对同一常量各持一名，合并时必须择一（流程上先改文档或按 DECISION 名重命名代码，二选一并留痕）。

**D2（P1，单点定义已分裂）**：DECISION 第 3 节要求 DORMANT 句阈值"与降档共用同一常量，不得分裂"且"全仓库禁止第二处出现数字字面量"；现状 `soul-algo-trait/src/a2.rs:91` 独立定义第二个 `180`（两 crate 尚未合并，无法互相 import）。缓解：值相等且两处各有测试钉死（`assert_eq!(DEMOTE_AFTER_SILENT_DAYS, 180)`、`assert_eq!(DORMANT_AFTER_DAYS, 180)`）；DECISION 第 0/6 节已把"统一 crates/soul-algo"declared 为冻结后合并义务。此漂移即合并义务本体，合并时以单一常量模块 import 消除。

**D3（P2，边界算子记录）**：降档为闭区间 `>= 180`；A2 休眠句为严格 `> 180`（测试 `the_dormancy_threshold_is_strictly_more_than_the_constant` 刻意钉死，且与 COPY_ZH P4"**超过** 180 天"字面一致）。恰好 180 天时：降档已发生、休眠句不出现。DECISION 声称的不变式是单向的（"句一出现，降档必已发生"），`>180 ⇒ >=180` 恒真，**不变式成立**，无矛盾；记录在案防止后人把常量表误读为句子也在 180 当天触发。

**D4（P1，话术未绑定——已声明的合并义务）**：crate 现有中文输出与 COPY_ZH 冻结模板**结构性不一致**，COPY_ZH 第 5 节绑定断言尚不存在（DECISION 第 0 节明言"模板绑定测试进合并 crate"是冻结后义务，第 6.4 节规定模板先改 COPY_ZH 再改代码）。具体差异：
  - 档位词汇：COPY_ZH §0.1 单源 `强/中等/弱`；tie crate `Band::as_zh` 输出 `强联系/中等联系/弱联系`；trait crate `Band::label_zh` 输出 `强/中/弱`（"中" ≠ "中等"）。同批三套词汇，违反 COPY_ZH §5.5。
  - 全部边解释缺 §0.7 透明句（"怎么算的：只统计次数和日期，不读聊天内容……"）与 §0.6 收尾句（"这是工作假设，你可以直接改。"）。
  - A2 渲染句差异：P1/P4/P5 措辞不同（P4 现渲染"距最新的记录 X 天"而非日期；P5 现为"这是对记录的归档，不是对这个人的评价"）；P3 的"有过一对一交流"分支刻意未发（a2.rs 注明理由）；唯 P1b 分列句与 COPY_ZH **逐字一致**。
  - 判档下的"最弱一档""往下降到"等句式与 COPY_ZH 模板句式不同构。
  以上均为"COPY_ZH 是规范、代码待对齐"性质：语义方向无一处与冻结规则相抵触（数字纪律、无拉丁字母、无小数/折算、引用一对一计数判档等实质约束已由 `explain_zh.rs` 17 项测试保证）。

**D5（P2，证据引用的夹具参数不一致）**：DECISION 引用其验证工作区（`.agent_workspace/round3/fable-a/t4d-verify/`，在库、可溯）的夹具，与 crate 同名/近名夹具构造不同：§1 `group_heavy_plus_one_direct_each_way` 记"群聊 100 条 / 50 天"，crate 同名夹具为 30 条 / 10 天（判决同为 T4 Strong✗ / T4D Weak）；§4.1 引 `group_heavy_plus_three_direct_each_way`（每方向各 3），crate 为 `group_heavy_plus_three_directs`（共 3 条 2 发 1 收）→ Moderate（DECISION 的"每方向各 3 条即回 Moderate"仍为真，crate 证明了更便宜的路径）；§4.3 引 `direct_quiet_200_group_yesterday`，crate 对应 `dormant_direct_group_ping_yesterday`（300 天）→ Strong。**全部判决方向一致，无一相抵**；仅提醒后续读者勿把 DECISION 的夹具参数当作 crate 夹具参数。

**D6（P1，常量表缺项）**：COPY_ZH §4 P2 冻结了方向句的 **2:1** 互斥分支阈值（5 分支）。该阈值不在 DECISION 常量表，且当前 `a2.rs::direction_bullet` 只有 3 分支、无任何比例判断——2:1 规则目前只存在于文档。合并落地 COPY_ZH 绑定测试时必须实现，并把 2:1 纳入单点定义纪律（进常量表或明确 COPY_ZH 为其定义点）。

**D7（P2，A1 变体语义注记）**：DECISION §2.4 括注"两种不同来源、**跨不同自然日**才升档"；代码 `TwoKindsAcrossDays` 实为"≥3 个 (kind, utc_day) 独立组 ∧ 覆盖 ≥2 种来源"——三种来源同一天也可升档，不必跨日。v0.1 数据面（仅问卷可计数）下不可达，A1 又非保留算法，无发布影响；v0.2 接入行为证据前应对齐措辞或实现。

**D8（P2，代码边角，非规范冲突）**：`Tally::last_direct_contact` 以 `0` 兼作"从未"哨兵（`types.rs:369`），一条恰在 Unix 纪元第 0 秒的一对一行会被后续更旧的行覆盖。仅影响上报字段，**永不判档**（该字段"Reported, never banded on"）；负时间戳常规路径已有测试覆盖。另：自由函数 `explain_zh` 对未知 `algorithm_id` 回落默认规则解释，对测试 oracle 的 `RawCounts` 分数可拼出自相矛盾句——产品两规则均产出已知 id + `Demoted` detail，产品路径不可达。两项均为合并时的廉价加固。

**D9（记录，非漂移）**：任一场地时钟的极端面——一对一历史任意陈旧、只要群聊持续活跃则 Strong 永不降档——是 DECISION §4.3 已定价代价的直接推论（本轮另一槽位的临时探针亦触及此面）。ablation TRUTH 表将 `dormant_direct_group_ping_yesterday` 如实记为 Unjudged。与冻结文本一致；若产品裁定不可接受，走 §4.3 载明的参数级变更通道，不动候选结构。

## 6. 结论

冻结的实质面——保留清单、常量值、闭区间语义、T4D 一对一计数口径、任一场地降档时钟、as_of 纪律、A0 锁、A2 无第二套阈值、A1 默认与空转、墓碑与拒绝项——**文档与代码全等，且全部由通过中的测试钉死**（228/228）。全部漂移为：常量命名（D1）、跨 crate 常量分裂（D2，即已声明的合并义务本体）、话术未绑定（D4/D6，同为已声明的合并义务）、证据引用参数（D5）与边角注记（D3/D7/D8/D9）。未发现任何会改变判档结果或与冻结规则相抵触的分歧。
