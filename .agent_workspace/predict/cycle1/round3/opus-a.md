MODEL_SLUG: claude-opus-5-thinking-high-fast

# Cycle 1 / Round 3 / opus-a — 冻结目录（v0.2 浅层行为预测）

**冻结依据**：`R1-SYNTHESIS.md`（数据面事实 + 九候选并列）＋ `R2-SYNTHESIS.md`（P4 降级、APPM 并入、不变式 F 收紧、夹具与评测协议）＋ `round1/opus-b.md`（九候选逐条规范）。
**冻结的是目录，不是选型**：本文件登记「哪些候选经确认可测、拿什么当对照、哪些出局、测试日之前必须先补什么」。**不选赢家**——赢家由夹具在实现轮打出来。
**边界**：不实现 Goal 2，不改 T4D/A0，不往产品 crate 写任何模型，不 git。本文件是 Cycle 1 交给父代理的成品。

---

## 0. 冻结口径（状态词表，全文只用这五个）

| 状态 | 含义 | 谁能改 |
|---|---|---|
| `TESTABLE-NOW` | 只吃冻结数据面；有整数可数的输出 token；有**预登记**的杀线；杀线的对照组已存在；今天就能在合成夹具上跑离线评测 | 实现轮的实测 |
| `TESTABLE-AFTER-GAP` | 同上，但**某个 schema 缺口不补就跑不了**（§5 指名） | 补缺口 |
| `COMPARATOR` | 本身不是产品候选，**存在的唯一理由是让别人的杀线有意义**。对照组缺席 = 那条杀线是空话 | 不可删；删了要同时删掉依赖它的杀线 |
| `NAMED-ALTERNATE` | 已具名、已登记，**默认不启用**；只有在指定测试上打赢主选才升格 | 打赢即升格；打不赢永久留位 |
| `REJECTED` | v0.2 出局，附出局理由与（若有）v0.4 复活条件 | 复活条件全部满足才重开 |

一条候选够得上 `TESTABLE-*` 的**五个必要条件**（缺一即降为 REJECTED 或退回规范）：
1. 只吃 §1 的冻结数据面；
2. 输出是闭集 token + 整数支持计数（无浮点、无概率、无百分位）；
3. 有弃权 token，且「猜」不许替代弃权；
4. 有**先写死后实现**的杀线，且杀线的对照组在 §3 里存在；
5. 过 §6 的通用门（含不变式 F 与 P6 锁门）。

---

## 1. 数据面（冻结事实，附源码坐标）

这一节是全表的地基，**与 R1 的 opus-b 规范有实质出入，以本节为准**。

| 事实 | 源码坐标 | 对候选的后果 |
|---|---|---|
| 前台会话的 `app` 与 `duration_ms` **只在密封体里**，各占一个字段，`deny_unknown_fields` | `soul-collect/src/session.rs::SealedSessionBody::FIELDS = ["app","duration_ms"]` | 任何消费 app/时长的预测器必须是**本机、持内容密钥、在线**的组件，不能是跑在导出数据上的离线脚本 |
| `events` 表明文列只有 `event_id/ts/source/kind/actor_subject/privacy_subject/body_blob_id/doc`，**无 app 列、无 duration 列** | `soul-store/src/sql.rs::DDL` | SQL 侧看不见 exe |
| 研究预览滚桶按 `(kind, time_bucket, privacy_subject)` 计数，**打不开 body**，所有 exe 合成一个 `app.foreground` | `soul-store/src/research_preview.rs::EVENT_ROLLUP_SQL` | 预测面**不可**建立在研究导出上；研究导出也拿不到 next-app 的任何信号 |
| 时间桶是 **UTC 小时**；带偏移的时间戳降级为日级桶，注释明写「不把不是 UTC 的东西相对论式地relabel 成 UTC」 | 同上，`EVENT_ROLLUP_SQL` 的 `CASE` 分支 | 见下一行 |
| **无本机时区偏移落库**：会话只有 `started_at_unix_millis`/`ended_at_unix_millis` | `session.rs::ForegroundSession` | **UTC 小时可算，但不可把 UTC 小时叫「你的上午」**。P3 只能诚实标注；本地小时版本是受阻变体（`p3l`，§2.3） |
| `EvidenceKind::AppUsage` 在枚举里有、有 `as_str`、**全仓无构造点** | `soul-algo-trait/src/types.rs`（镜像 `soul-schema/src/evidence.rs`） | 预测结论想落 `SoulInference` 就没有合法 `evidence_id` 可引（`evidence_ids` 是非空契约）→ §5 SG1 |
| Markov 夹具形状 | `FakeForegroundSource` 序列 → 解封 body → `Counter(zip(seq, seq[1:]))` | 同 app 连续轮询会被合并，**A→A 不会自然产生**；任何断言 A→A 的测试是在测夹具不是测算法 |

**与 R1 `opus-b` §0.1 的两处出入（以本节为准，且这是本轮的实质修订）**：

1. opus-b 的 S1 写了「本机时区偏移」是可用字段——**不可用**。凡依赖本地墙钟小时的分桶一律受阻（P3 的主线改 UTC 桶 + 诚实标注，本地桶降为 `p3l`）。
2. opus-b 把 `duration_s` 当作平列可读字段——**它在密封体里**。P8 与 P4 因此绑死在持钥进程内，且它们的评测不可能在研究面复现（这不是缺陷，是产品锁的正确后果，但必须写进已知代价）。

**没有的（要它 = 出局，不是调参）**：窗口标题、键鼠、剪贴板、文件名、聊天正文、跨用户数据、`sessions_discarded`（§5 SG5）。

---

## 2. 经确认可测目录（并列，不选赢家）

### 2.1 主表

| ID | `algorithm_id`（冻结） | 状态 | 预测什么 | 上下文 | 主输出 token | 杀线（不过即出局） | 对照 | 亲和轴 |
|---|---|---|---|---|---|---|---|---|
| P0 | `p0.marginal_top_k.v1` | COMPARATOR + TESTABLE-NOW | 下一个 app | 无 | `next_app` ×3 + `count` | 不可证伪（它是对照） | — | 无 |
| P1 | `p1.markov1_next_exe.v1` | TESTABLE-NOW | 下一个 app | 当前 app | `next_app` ×3 + 行计数/行总数 | top-1 比 P0 高 **≥10 点**（≥2000 次转移、≥3 份日志同时成立） | P0、C1 | 无 |
| P2 | `p2.ngram_backoff.v2` | TESTABLE-NOW | 下一个 app | 前 1–3 个 app（可变阶） | `next_app` ×3 + `order_used ∈ {3,2,1,0}` | 比 P1 高 **≥3 点** **且** 状态条目 ≤ P1 的 8× | P1 | 无 |
| P3 | `p3.utc_hourbin_top.v2` | TESTABLE-NOW | 下一个 app | (工作日/周末, **UTC** 小时) 48 桶 | `hour_bin_top` ×3 + 桶计数 | **无上下文格**（开机首段、boundary 后首段）比 P0 高 **≥8 点** | P0 | orderliness |
| P4 | `p4.hawkes_burst_int.v1` | COMPARATOR（R2 降级）+ TESTABLE-NOW | 会话起始的突发态（**何时**，不是哪个） | 最近 24h 起始计数环 | `burst.hot/warm/cold` | hot 的 15 分钟复发率 ≥ cold 的 **2×**，自助区间不重叠 | **C3**（不是 P1） | emotional_steadiness |
| P5 | `p5.contact_due_reminder.v1` | TESTABLE-NOW | 该一对一联系谁了 | T4D 冻结列 + 边自身间隔中位数 `g̃` | `reminder.contact_due(contact_id, silent_direct_days, g̃)` | 上线后本机忽略/关闭率 **< 70%** | 无（打扰类指标不与命中率比） | social_energy, accommodation |
| P6 | `p6.lock_gate.v1` | TESTABLE-NOW（强制包装，非中间件） | 不预测；执法 | 五轴 + 边锁（只读） | 过滤后 token + `abstain.axis_locked(axis)` | 全锁夹具下轴向输出为空 **且行为 token 照出**（双向） | — | 全部 |
| P7 | `p7.deterministic_arbiter.v1` | TESTABLE-NOW | 下一个 app（融合） | 各源支持计数 | `next_app` ×3 + `source(pN)` | ≥ 全部单源最大值，**且每个夹具上不比最好单源差 >2 点** | 各单源 | 继承并集 |
| P8 | `p8.dwell_hazard_bucket.v1` | TESTABLE-NOW | 当前段还会不会继续（打扰门） | 当前 app + 已停留秒数 | `dwell.likely_ends_within(b)` / `dwell.likely_continues` | 打赢 **C2**（该 app 历史时长中位数常数基线） | C2 | orderliness |

**九条全部 `TESTABLE-NOW`**——指的是「在合成夹具上离线评测今天就能跑」。**没有一条 `PRODUCTIZABLE-NOW`**：把任何一条的结论落成 `SoulInference` 都卡在 §5 SG1（`EvidenceKind::AppUsage` 无写者）。评测可测 ≠ 可落库，这两件事本轮明确分开。

### 2.2 R2 的三处演进（冻结形式）

- **P4 从「默认」降为「对照」。** 理由不是它弱，是它**问的问题不同**（何时 vs 哪个 app），而且事件率与采集同意开关纠缠（丢弃中的会话不落库，见 SG5），R1 那句「P4 必须打赢 P1 才配做默认」提法不成立、已作废。它的对手是 **C3 条件间隙基线**。P4 若打不赢 C3，出局的是 P4 的整个 tick 环，不是调半衰期。
- **APPM 不是第十个模型，拆成四份处置**（这是 R1「APPM-lite」并列条目的最终去向，登记齐全以防它日后作为「新想法」重新混进来）：

| APPM 的组成 | 冻结去向 |
|---|---|
| 可变阶前缀 | 并入 **P2**，`MAX_ORDER = 3`，stupid backoff 阶梯 3→2→1→0 |
| 命中加权 | 降为 **`p7c.hit_weighted_wins.v1`**（整数 pairwise wins，**禁除法**），NAMED-ALTERNATE |
| TTU（time-to-use） | 移到 P4 同轴，登记为 **`p4c.time_to_use_bucket.v1`**，与 P4 一起对 C3 测 |
| 预取（prefetch） | **REJECTED**——产品锁禁止代用户启动进程。不是性能问题，是权限问题 |
| 位置特征 | **REJECTED**——数据面没有位置；文献里增益本来就小 |

- **`p2.ngram2_backoff.v1` 这个 id 退休**，改 `p2.ngram_backoff.v2`。理由：可变阶改变了它的规范，两份规范共用一个 id 会让日后的测试红了也查不出改的是哪一版。**退休的 id 不得复用。**

### 2.3 受阻变体与具名备选（登记即冻结，不点名到不了）

| `algorithm_id` | 状态 | 升格/解阻条件 |
|---|---|---|
| `p3b.hourbin168.v2` | NAMED-ALTERNATE | 只在 **≥6 个月**数据的夹具上比 48 桶赢 **≥3 点**才启用（F3-4） |
| `p3l.localhour_top.v1` | **TESTABLE-AFTER-GAP** | 阻塞于 SG2/SG3（无本地偏移、无 DOW 原语）。补齐前**一行都不写**，且**禁止**用 UTC 小时冒充本地小时来「先跑起来」 |
| `p7b.borda_vote.v1` | NAMED-ALTERNATE | 打赢确定性阶梯 **≥2 点**（F7-2） |
| `p7c.hit_weighted_wins.v1` | NAMED-ALTERNATE | 同上，且必须整数 pairwise 比较、零除法 |
| `p4c.time_to_use_bucket.v1` | TESTABLE-NOW（对照轴） | 与 P4 同批对 C3 测；单独不构成候选 |

### 2.4 冻结常量表（单源；「出处」列是本表存在的理由）

| 常量 | 值 | 用于 | 出处 |
|---|---|---|---|
| `MERGE_GAP_S` | 5 | 去抖合并同 app | 约定 |
| `MIN_SESSION_S` | 3 | 丢弃过短段 | 约定 |
| `SESSION_GAP_S` | 1800 | ≥30min → `boundary`，转移链断开 | 约定 |
| `TOPK_APPS` / `TOPK_ROW` | 256 / 32 | Misra-Gries 重击者上限 | 约定（与 SG6 耦合） |
| `MIN_ROW_SUPPORT` | 5 | P1 行总下限 | **猜** |
| `MIN_ROW_SUPPORT_2` | 8 | P2 二阶行下限 | **猜** |
| `MIN_ROW_SUPPORT_3` | 12 | P2 三阶行下限（R2 新增阶带来的新常量） | **猜**，本轮新引入，必须与上两个一起扫 |
| `MIN_BIN_SUPPORT` | 10 | P3 桶下限 | **猜** |
| `TICK` / `HALFLIFE_TICKS` / `W` | 300s / 12 / 288 | P4 环形桶 | 半衰期**无先验依据**，须在 {30min,1h,4h} 三档各跑并如实登记 |
| `HOT` / `WARM` | 8 / 3 | P4 阈值 | **猜** |
| `COOLDOWN_DAYS` / `MAX_DAILY_REMINDERS` | 14 / 3 | P5 频控 | 约定 |
| 停留桶边界 | `[0,30) [30,120) [120,300) [300,900) [900,2700) [2700,7200) [7200,∞)` | P8 | 约定 |
| `MIN_DWELL_HISTORY` | 20 | P8 弃权门槛 | **猜** |

标 **猜** 的七个不许在实测后被悄悄改成赢的那一档再宣称「设计如此」。改了要留改动记录。P7 的三个阈值（8/5/10）**互相耦合**，单独调任何一个都会改变归因：要么三个一起网格扫一次，要么在此栏写死「约定」并不再动。

---

## 3. 对照组（COMPARATOR — 这一节缺了，上面的杀线全是空话）

| ID | `algorithm_id` | 是谁的对照 | 形状 | 缺席后果 |
|---|---|---|---|---|
| C0 | `p0.marginal_top_k.v1`（即 P0） | P1/P2/P3/P7 | 无上下文 top-k 频次 | 所有命中率杀线失效 |
| C1 | `c1.shuffle_leakage_probe.v1` | **评测器自身** | 把序列随机置换（边缘分布不变）后重跑：P1 必须退化到 P0 ±2 点 | 没这条，harness 漏未来时你会把泄漏当成算法的功劳 |
| C2 | `c2.dwell_median_constant.v1` | P8 | 「该 app 历史时长中位数」常数判定 | P8 的整张风险表无从证明其存在必要 |
| C3 | `c3.conditional_gap_baseline.v1` | **P4 / p4c** | 挂在转移表上的条件间隙基线：给定 `prev_app`，用该行历史**间隔中位数**（整数天/秒，取序中位，无除法）判「多久后会有下一次起始」 | R2 的核心修订。没有 C3，P4 就只能跟 P1 比，而它们预测的不是同一件事 |
| C4 | `c4.bayes_upper_bound` | P1（仅 SYN-MARKOV） | 已知转移矩阵的贝叶斯上界；P1 差距须 ≤5 点 | 分不清「算法弱」和「实现错」 |

**对照与备选的区别**（本轮反复被混淆，故写死）：对照组**永远保留**，它的价值在于别人赢它；具名备选**默认不启用**，它的价值在于它赢主选时能顶上。P0 同时是候选也是对照，这是它唯一的特权。

---

## 4. 否决（REJECTED）

### 4.1 v0.2 硬否决 + v0.4 复活条件

| 被否 | 出局理由 | 复活条件（全满足才重开） |
|---|---|---|
| 深度序列模型（LSTM / Transformer next-app） | 需批训练与浮点权重，违反「只有在线计数」；输出是分数不是可数 token；**遗忘破产**——权重里带着已删事件的痕迹 | (a) 本机可训不出网；(b) 删一条事件后能**可证明地**回到「从未见过它」的状态；(c) 有可数的解释面。三条缺一不谈 |
| 联邦学习 / 跨用户协同过滤 / 联邦 SeqMF | 需非 E1 云通道；v0.1/v0.2 **E0 无代码路径**；天然涉及跨用户数据 | v0.4 起：单独同意 + 只传 derived 聚合 + 可随时关 + 关掉后本机功能不降级 |
| 完整 ATPP | 要位置/POI/多用户预训练——三样这里全没有 | 数据面变了再谈；v0.2 无路径 |
| LLM 推「下一步意图」 | 需 key；无 key 时全部功能须仍可用，所以它当不了默认；黑盒不可数、不可纠正 | 可作**已配 E1 时的可选增强**，永不作默认，且不得反写档案 |
| 读窗口标题 / 读正文做上下文 | 窗口标题是「不做」，不是「推迟」（`window_titles_are_not_collected.rs` 就在那儿）；正文与「第三人正文默认不出本机」冲突 | **无复活条件** |
| 深网 / GNN（人脉图上） | 同「深度模型」三条，外加图上第三人数据 | 同深度模型，且须先解决第三人 `local_only` |
| **对锁定轴做人格预测** | 锁 = 用户已经说了算 | **无复活条件**——这不是能力问题 |
| 时长加权 / 指数衰减权重（作为通用机制） | 引入浮点权重则可数性与不变式 F 同时破 | 已在 P4 内以整数量化 + 环形桶收编；**不再开第二个衰减机制** |
| **预取 / 代用户启动进程**（R2 新增） | 产品锁禁止代启动。哪怕预测 100% 准也不做 | 无 |
| 预测输出直写 `trait_axis.*` | 面分离（P6 R1） | 无。唯一路径是用户点头 → `UserCorrection` |

### 4.2 R1 曾并列、R2 后不再单列（去向已在 §2.2 登记）

`APPM-lite` 的五个部件：可变阶 → P2；命中加权 → `p7c`；TTU → `p4c`；预取 → 否决；位置特征 → 否决。**它不再是一个候选**。

---

## 5. schema 缺口（测试日之前的前置件；本轮只记录，不实现）

| ID | 缺口 | 证据 | 阻塞谁 | 补法（形状，不是实现） |
|---|---|---|---|---|
| SG1 | `EvidenceKind::AppUsage` **有词无写者**；`SoulInference.evidence_ids` 是非空契约 | `soul-algo-trait/src/types.rs` 有枚举与 `as_str`，无构造点（承 R1 opus-a **G14 / G6**） | **全部九条的落库路径**（不阻塞离线评测） | 先定义 AppUsage 证据链的形状：聚合行落哪张表、怎么挂 content key（G12）、`Derivation::Aggregate` 与 `ExportField::DurationBucket` 三个空位是否一起填 |
| SG2 | 采集侧**不存本机时区偏移** | `ForegroundSession` 只有 unix millis | `p3l`；P3 的「用户作息」措辞 | 事件同时存 UTC 与当时 offset；不补则 P3 永远只能说 UTC 小时 |
| SG3 | 无 day-of-week / 民用日历原语导出（三处各写一份，无一导出 DOW） | 承 R1 opus-a **G1+G2** | P3 的 `weekday_class` 分桶 | 单独一轮补原语；这是所有节律类算法的共同前置件 |
| SG4 | 研究面「时长」的唯一位置 `ExportField::DurationBucket` / `ExportRow.duration_bucket` **有字段、有 `fields_present` 分支、有 CLI 透传、无生产者** | 承 R1 opus-a **G6** | P8 结论进研究导出 | 与 SG1 同批定义 |
| SG5 | **`sessions_discarded` 不落库** → 「缺席」是「闲置」还是「不可观测」无法区分 | 承 R1 opus-a **G3** | P4 的事件率（同意开关一关，率就失真）；所有候选的缺席假设 | 在补齐之前，每个候选必须**显式标注自己的缺席假设**；不标注的规范不算完整 |
| SG6 | 重击者驱逐**没有计数器**，不变式 F 无法断言 | R2 结论：F 在去抖合并 + heavy-hitter 驱逐下不「天然成立」 | P0–P3、P8 的不变式 F 测试 | 状态表暴露 `evictions` 计数；测试断言 `evictions == 0`（越界即测试无效，而不是算法失败） |
| SG7 | 删除事件缺**局部重算**接口 | 去抖合并使「删一条 = 减一个桶」不成立：删掉中间样本会改变分段 | 全部的遗忘语义 | 定义**最小重算窗口**＝以被删事件为中心向两侧扩到最近的 `boundary`；窗口内重放，窗口外不动 |
| SG8 | 密封体只能持钥在线读 | `SealedSessionBody` + `events` 表无 app/duration 列 | `REAL-*` 本机评测的形态 | 评测器必须是持钥进程内的组件；**不得**为了跑评测而把 app/时长旁路出密封体 |
| SG9 | 没有**全局打扰账** | P4/P5/P8 各有各的门，加起来仍可能吵 | P5 上线后的忽略率杀线 | 一个整数账：「今天已经打扰过你几次」，三个门共用 |

**SG1 是第一前置件，不是可选项**：在它之前，「基于 app 用量的预测」在这个仓里**没有合法的 evidence_id 可引**，而「无证据不得落库」是产品锁。

---

## 6. 通用门与不变式（九条都要过；过不了不用比命中率）

1. **确定性**：同输入同输出 1000 次，含全部并列格（并列一律 `app_id` 字典序升序打破）。
2. **`as_of` 平移不变**：禁读墙钟，`as_of` 由调用方传入。
3. 空日志 → 弃权；未同意 → 事件 0 → `abstain.consent_off`，**不许拿旧计数续命**。
4. **不变式 F**：`replay(events \ {e}) == decrement(replay(events), e)` 逐桶相等。**R2 修订：这条不「天然成立」**——测试必须同时满足 SG6（`evictions == 0`）与 SG7（删除走局部重算）。P4 改环形桶正是为了这条（指数衰减累加器的右移不可逆）。
5. **P6 锁门双向**：轴向输出被挡 **且** 行为 token 照出。门把有用的一起挡掉同样是失败。
6. **源码级面分离**：predict 侧不得依赖 profile 写入 API；全量 grep `trait_axis.` 出现次数为 0（测试的反向断言处需排除）。
7. **遗忘即消失**：被引用事件删除后，该条预测输出当场降为 orphaned。

**评测协议（冻结）**：预顺序 prequential（test-then-train，单遍，禁打乱、禁用未来）；夹具 `SYN-MARKOV` / `SYN-RHYTHM` / `SYN-NOISE` / `SYN-TIE` / `ADV-DST` / `ADV-TZ_CHANGE` / `ADV-FLOOD` / `ADV-NEW_APP` / `ADV-SINGLE_APP` / `ADV-EMPTY` / `ADV-CONSENT_JUST_ON`；泄漏探针 C1 恒开。

**一处看起来矛盾、其实不矛盾的地方**（写明以免日后被当成漏洞）：产品锁禁分数/百分位，而本文杀线满口「百分点」。区别在受众——**命中率只活在评测器内部，永不上屏**；用户面只有整数计数与桶名。这两条并存是有意的。

---

## 7. 本轮未裁、明确留给后面的

1. P4 半衰期三档实测后登记，**禁止事后倒填**。
2. P3 的 48 vs 168 由 F3-4 的数据裁。
3. P7 三阈值耦合：一起扫，或承认是约定。
4. **预测输出进不进研究导出**：`app_id` 是本机身份不是第三人正文，但「某人开了什么软件」的敏感度不为零（医疗类 app）。是否桶化/哈希，本轮不裁。
5. P5 与 A2 的话术边界：两者会说到同一条边的天数，措辞必须单源。
6. 全局打扰账（SG9）的归属层。

---

## 8. 反例检查表（怎么证明本文件错了）

- 「app 与 duration 都在密封体」→ 读 `soul-collect/src/session.rs::SealedSessionBody::FIELDS` 与 `soul-store/src/sql.rs::DDL` 的 `events` 建表语句。**`events` 一旦出现 app 或 duration 列，§1 作废、§2 的状态列要重判。**
- 「`EvidenceKind::AppUsage` 无写者」→ `rg 'EvidenceKind::AppUsage' crates/*/src/` 今天只命中枚举成员与 `as_str`。**任一处出现构造赋值，SG1 解除。**
- 「研究预览打不开 body」→ 读 `research_preview.rs::EVENT_ROLLUP_SQL`。**出现 join 到 `sealed_blobs` 的分支，§1 第 3 行作废。**
- 「无本机时区」→ `ForegroundSession` 出现 offset 字段，SG2 解除、`p3l` 从受阻升为可测。
- 「A→A 不自然产生」→ 若夹具里出现连续同 app 转移，先查是不是去抖合并被绕过了，再查算法。

---

## 9. 交给 Cycle 2 的一句话

Cycle 1 的结论不是「哪个算法好」，是**这九条的可测性已经与产品锁对齐了**：数据面只有密封的 exe 名 + 时长 + UTC 时刻，所以能做的就是整数计数上的 Markov/n-gram/时段桶/停留桶/沉寂天数，而它们的对照组、杀线、锁冲突、以及九个 schema 缺口都已具名。Cycle 2（fileplan 启发式、下一动作、进程身份）**继承本文件的 §0 状态词表、§3 对照组纪律、§5 缺口编号与 §6 通用门**，不要另起一套。
