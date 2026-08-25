# 调研精华：v0.2 浅层行为预测 + 程序/代理层算法

Parent: `cursor-grok-4.6-high`。两个独立 3 轮循环（每轮 2×fable + 2×opus-fast + 2×gpt-sol）。
**fable 后台独立，不等未完成的 fable。** Cycle 2 Round 3 的 fable-a 未到；本文件以已收槽位为准。

**本文件冻结的是「日后测什么」，不是「哪个算法赢」。** 多种经确认可测的算法并列；赢家由测试日夹具打出。
**不实现 Goal 2，不改 T4D/A0，不把预测模型写进产品 crate。**

完整杀线与夹具：Cycle 1 看 `cycle1/round3/{opus-a,opus-b,fable-a,fable-b}.md`；Cycle 2 看 `cycle2/round3/{opus-a,opus-b,gpt-sol-a,gpt-sol-b,fable-b}.md`。

---

## 0. 共同纪律（两条循环共用）

| 规则 | 含义 |
|---|---|
| 输出 | 闭集 token + 整数支持计数。无浮点权重、无概率上屏、无分数/百分位（D22） |
| 弃权 | 有弃权 token；「猜」不许替代弃权 |
| 判定 | 交叉相乘、`i128`；命中率只活在评测器内部，永不上屏 |
| 空虚 | 分母 `<200` 或分子 `<20` ⇒ `inconclusive` ⇒ **按不过** |
| 遗忘 | 不变式 F：只在 `evictions==0` 时断言；删除走局部重算，不是单桶减一 |
| 锁门 | P6：锁轴挡人格陈述，**不挡** next-app 等行为 token |
| 审计 | 审计链不得当特征（遗忘旁路）。一次性普查只许**杀**候选，不许标定会进二进制的常量 |
| 不选赢家 | 测试日之前所有 `TESTABLE-*` 并列 |

状态词：`TESTABLE-NOW` / `TESTABLE-AFTER-GAP` / `COMPARATOR` / `NAMED-ALTERNATE` / `REJECTED`。

---

## 1. Cycle 1 — 浅层行为预测

### 1.1 数据面（测之前必须承认）

- 前台明文只有 `ts` + 枚举；**`app` 与 `duration_ms` 在 AEAD 密封体里**。研究预览 SQL **打不开 body**，所有 exe 合成一个 `app.foreground` 计数。
- 预测器必须是**本机、持内容密钥、在线**的组件，不能是跑在研究导出上的离线脚本。
- **无本机时区偏移落库** → UTC 小时可算，**不可把 UTC 小时叫「你的上午」**。
- `EvidenceKind::AppUsage` 有枚举、**全仓无写者** → 任何结论今天都不能合法落 `SoulInference`（离线评测可以跑，落库不行）。
- 同 app 轮询会被会话合并，**A→A 转移不自然产生**；断言 A→A 的测试是在测夹具。

### 1.2 经确认可测（并列，不选赢家）

| ID | algorithm_id | 状态 | 预测什么 | 杀线一句话 | 对照 |
|---|---|---|---|---|---|
| P0 | `p0.marginal_top_k.v1` | COMPARATOR + TESTABLE-NOW | 下一个 app（无上下文） | 地板；自身不可证伪 | — |
| P1 | `p1.markov1_next_exe.v1` | TESTABLE-NOW | 下一个 app \| 当前 app | top-1 比 P0 高 **≥10 点**（≥2000 转移、≥3 份日志） | P0、C1 shuffle |
| P2 | `p2.ngram_backoff.v2`（配置 `@2`/`@3`） | TESTABLE-NOW | 下一个 app \| 前 1–3 个 | 比 P1 高 **≥3 点** 且条目 ≤8×P1；`@3` 另须过 SYN-order3 且 `order_used==3` 占比 ≥5% | P1 |
| P3 | `p3.utc_hourbin_top.v2` | TESTABLE-NOW | 下一个 app \| UTC 小时×工作日/周末 | **无上下文格**（开机/boundary 后首段）比 P0 高 **≥8 点**；若 MRU 过线还要比 MRU | P0、`c_mru` |
| P5 | `p5.contact_due_reminder.v1` | TESTABLE-NOW | 该一对一联系谁 | 只读 T4D；`last_direct_contact` 时钟；Weak/群-only 不提醒；上线后忽略率 **<70%** | — |
| P6 | `p6.lock_gate.v1` | TESTABLE-NOW（强制包装） | 不预测；执法 | 全锁：轴向空 **且** 行为 token 照出 | — |
| P7 | `p7.deterministic_arbiter.v1` | TESTABLE-NOW | 融合 next-app | ≥全部单源最大值，且每夹具不差 >2 点；源集冻结 `{P0,P1,P2,P3}`，**P4 永不入源** | 各单源 |
| P8 | `p8.dwell_hazard_bucket.v1` | TESTABLE-NOW | 这段还会不会继续（打扰门） | 打赢该 app 历史时长中位数；接 P5 后不当时机提醒降 ≥50% 且总量降 ≤20% | C2 中位数 |

共享会话规范化（约定，测试日写死）：`MERGE_GAP_S=5`，`MIN_SESSION_S=3`，`SESSION_GAP_S=1800`。

**评测**：prequential（test-then-train）；指标只有 top-1 / top-3 / 弃权率；shuffle 泄漏探针上 P1 必须退化到 P0。夹具最低集：`SYN-MARKOV` / `SYN-RHYTHM` / `SYN-NOISE` / `ADV-DST`。

### 1.3 对照轨（「何时」，v0.2 不发布用户可见突发）

| ID | algorithm_id | 角色 |
|---|---|---|
| C1 | `c1.shuffle_leakage_probe.v1` | harness 红灯：打乱后 P1≈P0 |
| C2 | `c2.dwell_median_constant.v1` | P8 的对手 |
| C3 | `c3.conditional_gap_baseline.v1` | **P4 的真对手**（挂在转移表上的条件间隙，不是 P1 命中率） |
| P4 | `p4.hawkes_burst_int.v1`（`p4a`/`p4b` 整数环） | COMPARATOR。事件率与采集开关纠缠。翻案要四条全过：G15 在线信号 + uptime 掩码过 + 打赢 C1/C3 + 全局打扰账 |
| TTU | `ttu.delay_cdf.v1` / `p4c.time_to_use_bucket.v1` | 与 P4 同轴另测；话术形态「N 次里 M 次在 T 秒内」 |
| `c_mru` | `c_mru.repeat_last.v1` | 边界格第二地板。段内 top-1 必须为 0（合并后无 A→A） |

### 1.4 具名备选（不点名到不了）

| ID | 升格条件 |
|---|---|
| `p3b.hourbin168.v2` | ≥6 个月数据上赢 48 桶 ≥3 点 |
| `p3l.localhour_top.v1` | **阻塞**：必须先有逐事件 `utc_offset_seconds_at_start`。禁止用 UTC 冒充本地 |
| `p1d.duration_cond_markov.v1` | 时长桶必须与 P8 七桶单源；在 SYN-durationsplit 上赢 P1 ≥3 点 |
| `p7b.borda_vote.v1` / `p7c.hit_weighted_wins.v1` | 打赢固定阶梯 ≥2 点；整数 pairwise 或交叉相乘，**禁除法**；两者不许同时启用 |
| `p9.window_rule_set.v1` | 可纠正性最强（用户可逐条删规则）；须赢 P2 且规则数不爆炸 |

### 1.5 否决（v0.2 默认）

联邦 SeqMF / 云协同过滤（E0，v0.4 前复活须单独同意+只传聚合）；完整 ATPP（要位置/多用户）；深网/GNN/神经 TPP（遗忘破产）；读窗口标题/正文；预取/代启动进程；对锁定轴做人格预测；用审计链当特征；APPM 原味加权插值（已拆：可变阶→P2，命中加权→p7c，TTU→对照，预取→墓碑）。**文档与指标表不得出现 `appm` 字样。**

### 1.6 测试日前置缺口（只记录）

| 缺口 | 阻塞 |
|---|---|
| SG1 / G14 | `AppUsage` 无写者 → **全部落库路径** |
| SG2 / G2 | 无本地时区 → `p3l`；P3 只能诚实标 UTC |
| SG3 / G1 | 无 DOW 原语 → P3 的工作日桶 |
| SG5 / G15 | `sessions_discarded` 不落库 → 速率类 REAL 数字 inadmissible |
| SG6 | 无 `evictions` 计数 → F 测试假绿灯 |
| SG7 | 删除缺局部重算窗口 |
| SG9 | 无全局打扰账 → P4/P5/P8/x1 加起来仍可能吵 |

P3-local 最小契约（gpt-sol C1R3 钉死）：每条会话至少 `started_at_utc` + `utc_offset_seconds_at_start`（采集时快照，禁止事后用「机器当前时区」回填）。ADV-DST 生产口径必须得到 `(alpha, UTC 13)=3` 与 `(alpha, UTC 14)=3`，不得合成「本地 09 点 = 6」。

---

## 2. Cycle 2 — 程序 / 代理层

### 2.1 现行算法（已在产品里，是对照不是后继）

`soul-fileplan`：69 扩展名 → 8 种 `FileKind` → 固定中文文件夹；五级短路；`plan::build` **不接时钟**；v0.1 `executable_in_this_version: false`。

### 2.2 文件计划后继（并列；**只减不增**）

复合不变式（一条测试覆盖整张表）：

```text
moves = C₀ \ (D_magic ∪ D_supk ∪ D_recency)
```

无论开哪几个开关，`moves(catalog) ⊆ moves(X0)`。S2 默认只注解、不改 move 集合。

短路顺序冻结（组合测才看得见的假拒绝）：

```text
目录 → 深度 → 未知种类 → 目标占用 → KindDisputed(S1) → TooFewOfKind(SUP-K) → RecentlyModified(S3) → 提议
```

吃 `as_of` 的理由必须排最后，否则一个「既太新又与签名矛盾」的文件会在午夜从 `recently_modified` 翻成 `kind_disputed`：**盘没动、文件从头到尾不移动，hash 却变了。**

| ID | algorithm_id | 状态 | 做什么 | 默认读正文 | 杀线一句话 |
|---|---|---|---|---|---|
| X0 | `x0.extension_whitelist.v1` | COMPARATOR | 今天的 69 项表 | 否 | 地板 |
| S3a | `s3a.recency_suppress.v1` | TESTABLE-NOW | mtime 落在 as_of 的 UTC 日内 ⇒ 不提议 | 否 | 同日四个取样 hash 全等；全老目录跨零点 hash **相等** |
| S3c | `s3c.recency_banding.v1` | TESTABLE-NOW | 仅渲染分档 | 否 | 不进 `to_json`；不自己读钟 |
| SUP-K | `supk.kind_support_threshold.v1` | TESTABLE-NOW | 同类散文件 < K 且文件夹不存在 ⇒ 不建文件夹 | 否 | 挡掉 >30% 散文件 ⇒ K 太大；文件夹已存在则 1 个也照移 |
| S2-0 | `s2.duplicate_candidates.v1` | TESTABLE-NOW | `(父目录, size, ext)` 成组，标「可能重复」 | 否 | precision 达不到 0.90 ⇒ 措辞封顶在「可能」；**零 move** |
| S2-H | `s2h.exact_hash_confirm.v1` | TESTABLE-AFTER-GAP | SHA-256 金标准 | 是（opt-in） | 哈希落库/出网/进审计/变 delete ⇒ 出局 |
| S1-V | `s1v.magic_withhold_only.v1` | TESTABLE-AFTER-GAP | 读有界前缀，**只撤回**不相容的 move | 默认关 | `moves ⊆ C₀`；PK 是家族不是种类；撒谎率 <1% ⇒ 继续关 |

**`as_of_day` 不进 `plan_hash`。** 它的影响已体现在 `moves`/`left_alone`。批准载荷钉住 `(plan_hash, policy_vector, as_of_day)`，执行时用批准日重放。判决测试：同一份 scan、跨 UTC 零点、无文件跨过近因阈值 ⇒ hash 不变；有文件跨过 ⇒ hash 必须变。

PK 兼容集合冻结为 `{Archive, Document, Sheet, Slides}`。硬映射 `PK\x03\x04 → Archive` **否决**（会撤回全部 ZIP-based office 文件）。

S1 默认关：收益线（扩展名撒谎频率）必须先读字节才能知道，与功能要同一份同意。可先实现形状门，**无数之前不得进默认预览**。

### 2.3 下一动作（学习候选今天不能产品化）

| ID | 状态 | 要点 |
|---|---|---|
| n0 / n1 | COMPARATOR | 边际 / MRU |
| n2A | **退位** | HITL 面上与 MRU 不可分（`prepare()` 发出的就是 `egress.generate`） |
| n2B | 仅离线对照 | 待决状态规则；产品 crate 引用数须为 0 |
| a1–a4 | TESTABLE-AFTER-GAP | 动作 Markov / 二阶 / 计划形状 / 拒绝码条件。跨启动阻塞于 **无 forgettable 动作历史（G17）** |
| x1 | TESTABLE-AFTER-GAP | `left(exe)` 只决定**何时**建议，不决定建议什么。阻塞 G17+G18+打扰账 |
| x2 | REJECTED（产品面） | 「离开 code.exe ⇒ 整理 Downloads」：人群先验 + 无数得清的句子。可作影子研究 |

**进程身份子题：诚实的空。** Cycle 1 已把 exe+时长面上的预测做完；接到动作轴对不上同一条时间轴。正确产出是「今天没有新算法」，不是硬凑一条。

动作词表核账（gpt-sol C2R3，研究记录，**本调研不修产品**）：

```text
策略词表 ActionKind = 8
真正走 check_action 的 = 5
桌面同名操作 = 8（forget.* / research.preview 绕过闸门）
可遗忘的精确动作日志 = 0
```

最坏冻结：把三个绕行 token 当成「用户从不选」让模型学恒零。正确冻结：先承认调用图分叉；`forget.execute` 绕过令牌与销毁前计划变化拒绝，是跨层不变式缺口，不是算法调参。

### 2.4 否决

默认读第三人正文 / LLM 分类出网；语义内容聚类进 `plan_hash`；幻数**提升** Unrecognised 成新 move；通用 ZIP 中央目录/OCR；双边区间近因；滑动窗口 `age_days`；`as_of_day` 进 hash；跨会话哈希索引；删除（S2 最强只到注解，或至多可逆 `重复/`——见 §2.5）；无人值守 AutoGPT / 任意 shell / 键鼠接管；建议当证据落库。

### 2.5 测试日仍并列的分歧（不要提前裁成单一赢家）

| 题 | 侧 A | 侧 B | 怎么测 |
|---|---|---|---|
| S2 能否产出 move | 永远只注解（「多余」不是单文件性质） | 最强可逆移入 `重复/` | 先跑「一路径一行」+ 局部性：裁决句里能不能只出现被移动的那一份 |
| S1 与 S3 能否同时默认开 | 次序纪律即可 | 若过半打开的文件随后被近因抑制 ⇒ 二选一 | 合成夹具数 `opened_then_recent` |
| SUP-K 支持计数取在 C₀ 还是撤回之后 | 取 C₀：开关不耦合 | 先撤回再数：更精确、开关耦合 | 「一类两文件、其中一个扩展名说谎」语料频率 |
| n2 地盘线 90% | 背书 90% | 落在 [85%,90%) 必须双报 | 边界带双结论，不许选一个 |

---

## 3. 测试日建议顺序（先结构，后命中率）

1. **Harness**：prequential、shuffle 泄漏、确定性 1000 次、整数判定、`evictions==0` 前置的 F。
2. **夹具 oracle**：SYN-MARKOV 有向计数、SYN-RHYTHM UTC 桶、ADV-DST 不得合成本地 09=6、fileplan PK 矩阵。
3. **Cycle 2 零读收益上界**（不实现候选也能算）：S3a 窗口内散文件数、S2-0 有候选的根占比。S1 算不出零读上界——跳过或走影子同意。
4. **Cycle 1 next-app 表**：P0 → P1 → P2@2/@3 → P3（对 P0 与 `c_mru`）→ P7。何时轴另表，禁混刊。
5. **打扰组合**：P8+P5；三门（再加 P4）在全局打扰账出现前不开测。
6. **动作轴**：先 K1/K2 上界处决；面 A 上 n2≡n1 已红，不要再把「打赢 n2」当战绩。

---

## 4. 明确不裁 / 留给产品

- exe 名算不算审计正文、能不能上屏（挡住 x1 的合法渲染）。
- 预测输出进不进研究导出（医疗类 app 敏感）。
- `b0`「准备好回信但还没生成」值不值得占一个产品位置。
- v0.1.1 执行面要不要给 `plan.files` 令牌（今天批准窗口无 TTL，开着过夜必跨零点）。
- 语义聚类的**分离建议流**要不要存在（永不进 `plan_hash`）。

---

## 5. 一页版

**行为预测（可测并列）**：MFU 地板 (P0) · 一阶 Markov (P1) · 可变阶 n-gram backoff (P2) · UTC 时段桶 (P3) · T4D 沉寂提醒 (P5) · A0 锁门 (P6) · 确定性仲裁 (P7) · 停留风险桶 (P8)。Hawkes/突发环只作对照。本地小时、联邦、深网、预取出局。

**程序算法（可测并列，只减不增）**：近因抑制 (S3a) · 单件类别阈值 (SUP-K) · 零读重复注解 (S2-0) · 只撤回的 magic (S1-V，默认关)。学习下一动作在可遗忘动作日志出现前不得产品化。进程身份：无新算法。

**第一前置件**：给 `EvidenceKind::AppUsage` 一个写者之前，预测结论不得落库；给「打开文件字节」同意主题之前，S1/S2-H 不得默认开。
