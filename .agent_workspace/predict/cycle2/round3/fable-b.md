MODEL_SLUG: claude-fable-5-thinking-xhigh

# Cycle 2 / Round 3 / fable-b — 测试日验收清单（acceptance checklist）

定位：给日后**实现并运行** Cycle 2 候选目录的人的逐项验收单，与 Cycle 1 R3 fable-b 同一职能。本文件不新增裁决、不改常量、不实现 crate；每项挂已登记出处（括号锚点），与出处冲突时以 `R2-SYNTHESIS` 为准；发现清单与已登记裁决矛盾属于**清单 bug**——修清单，不修裁决。不重开 P0–P8，不实现 Goal 2。

**判定值纪律**：每个勾选项只有四种结果——`PASS` / `FAIL` / `BLOCKED(缺口ID)` / `N-A`（已墓碑 / 属产品决策 / 属上线后指标）。`inconclusive` 一律记 `FAIL`（继承 C1；R2/opus-b §4.6 把「没数据 ⇒ 没失败 ⇒ 可以上」列为必须提前堵死的捷径）。另有一个标记 `⚖`：该项依赖一条 R3 综合尚未裁的争点，清单按**从严侧**给默认口径（见签核规则 3），裁决落地后按变更控制换向。

**双台架纪律**：Cycle 2 有两个性质不同的测试台——**fileplan 台**（确定性纯函数 + 真实临时目录 + 字节夹具）与**动作轴台**（prequential 事件流 + 机会集普查）。一个台上产出的数字不得出现在另一个台的表里。分层同 C1：L0（harness/源码级门）→ L1（夹具）→ L2（不变式）→ L3（候选杀线）→ L4（报告呈现）→ L5（缺口映射）。**低层未全绿，高层数字可以跑、不可采信。**

---

## 目录快照（本清单覆盖的受验对象）

### fileplan 轨（S 系）

| 对象 | 状态（已登记） | 主杀线所在 |
|---|---|---|
| X0 扩展名基线（69 项 → 8 类 → 中文文件夹，五级短路） | 冻结为 v0.1 默认与一切实验的零正文对照（R1/gpt-sol-b §7-1） | D-X0 |
| S3a 近因抑制 + S3c 分档显示（层 A+B） | `TESTABLE-NOW`（R2/opus-a §8） | D-S3 |
| 层 C as_of 钉住重放 + `PlanTooOld` 理由码 | `TESTABLE-AFTER-GAP`（v0.1.1 执行面） | D-S3 |
| SUP-K（原 S3b，单件类别不建文件夹） | 移出 S3 单独立项，`TESTABLE-NOW`（R2/opus-a §8） | D-S3 末条 |
| S2-0 零读重复分组 | **弱提示研究候选**；升级门未过（R2/gpt-sol-b 结论） | D-S2 |
| S2-H 全量哈希 | opt-in 金标准，不进默认路径 | D-S2 |
| S1 dispute-only 幻数（V，只撤回） | 前置件未补前**零产品代码**（R2/fable-a §5） | D-S1 |
| 精确哈希去重 | 聚类家族唯一幸存，与 V 并列，同走读取授权门（R2/fable-a §7） | D-S1 末条 |
| 语义内容聚类 C | **任何配置不进计划**；分离建议流三条件（R2/fable-a §0.4） | D-C |
| K1 用户关键词规则 / T1 TF-IDF / A1 相似推荐 | 授权/schema 门控（R1/gpt-sol-b §3） | D-K |
| U1 / B1 / C1 全容器解析 / OCR / LLM 在线分类 | 不进确认清单（R1/gpt-sol-b §4） | 出局表 |

### 动作轴（N 系）与建议器层

| 对象 | 状态（已登记） | 主杀线所在 |
|---|---|---|
| n0 边际 / n1 MRU | 对照地板，`TESTABLE-NOW` | D-N |
| n2A（HITL 动作面） | **不可分于 n1（K3 已红），不得单独进阶梯**（R2/opus-b §2.1） | D-N |
| n2B（命令面，6 行表） | `n2` 唯一有意义形态，全体 `BLOCKED(G21)` | D-N |
| b0 `pending_then_marginal.v1` | 基线（「杀死之后发什么」的名字，R2/opus-b §3.6） | D-N |
| a1 / a2 / a3 / a4 + x1 特征形态 | 被杀名单（精确列举，不扩不缩），按 K1–K6 判 | D-N |
| x2 硬编码 exe→动作 | 产品面 `REJECTED`；影子研究 `BLOCKED(G18)` | D-X |
| x1 门形态 | `TESTABLE-AFTER-GAP`（G17+G18+SG9） | D-X |
| 可建议性三分法（S4 / U2 / T2+1）+ 卡片载荷纪律 | 结构主张 → 类型与测试（R2/fable-b §2、§5） | D-SUG |

**出局清单（验收时任何报告/实现出现即整份退回）**：无人值守多步执行、自由字符串动作、键鼠/屏幕/标题/正文采集、外部祈使句回显为动作、float 阈值或永久允许参与放行、旧批准执行新计划（R1/fable-b X1–X7）；`PK\x03\x04 → Archive` 单值映射（R2/gpt-sol-a §9-1）；`as_of_day` 进 `to_json`、滑动 `age_days`、双边区间近因（R2/opus-a §8 三条 REJECTED）；预测状态含 plan_hash/文件名/路径/字节数（门 D3）；读审计链当特征（门 D4，无复活条件）；零读分组说「内容相同」、`zip_family` 说成「已验证」；卡片载荷含 `plan_hash`/`preparation_id`/`token_id`（R2/fable-b §3）；`suggestability` 或 scope 映射出现 `_ =>` 通配；面 A/面 B 数字同表；`n2` 规则表出现第 7 行。

---

## L0 Harness 与源码级门（A 系列）——任何数字被受理的前提

- [ ] **A1 整数判定**：过/不过一律交叉相乘精确整数（四元乘积 i128），判定步无浮点；先报分子分母再报派生小数（继承 C1；R2/opus-b §3.5）。
- [ ] **A2 常量预注册**：S2 的 precision 下界 0.90 与 `MIN` 档、S1 的 `prefix_limit`/`total_budget`/`rule_version`、S3 的 `within_days`、动作轴 `{5 点, 10 点, 200, 20}`、`n2` 地盘线 90%（⚖）、剖面权重、一次性行清单、种子表——**在看对应结果之前**登记并带登记时间；事后倒填 = 该项 FAIL 且相关结果作废（R2/opus-b §4.0）。
- [ ] **A3 确定性**：同输入 1000 次逐字节同输出；并列一律按稳定 token 的字典序（`as_str()` / canonical path）；S1 预算覆盖不得依赖目录遍历顺序，预算选择先按 baseline move 的 canonical path 排序（R2/gpt-sol-a §7.3）。
- [ ] **A4 时钟纪律四条**：as_of 是参数不是读数，一次调用一个时刻；量化是**日历日之差**（`UtcDay` newtype + `div_euclid`），不是经过秒除以 86400；原始时刻不进被哈希值；判定对 as_of 单调且排短路链最后（R2/opus-a §10）。配套 `CLOCKS` needle 表对全 crate 生效，**并有「这张表认得时钟」的对照用例**——没有对照的 needle 表等于没有表（R2/opus-a §2.1）。
- [ ] **A5 `READS` needle 表**：在 S1 的任何产品代码之前落地，默认全 crate 生效、只对嗅探模块开例外，且照 `the_search_recognises_a_write_when_it_sees_one` 补「认得读」对照用例（R1/opus-a §6.1-1；R2/fable-a §5.1）。缺表时 S1 相关项全部记 `BLOCKED(READS)`。
- [ ] **A6 真实闸门构造**：`ACT-*` 动作事件必须由 `check_action` 实际返回 `Ok(ApprovedAction)` 产生（R1/opus-b §6.1）；`ACT-PEND-*` 待决状态必须由真实 `prepare()` / `preview_forget()` 置位，禁手插（`ACT-PEND-INVALIDATE` 的静默清空只有真实 `commit_telegram` 才造得出，R2/opus-b §7.1）；fileplan 字节夹具全部合成、固定字节、带 manifest（长度、SHA-256、生成器版本），运行时不调用 Office/LibreOffice/`file`/网络（R2/gpt-sol-a §5）。
- [ ] **A7 面标记**：`n2` 与动作轴的任何数字必须带 `S ∈ {A, B}`；两面数字同表 = 报告退回（R2/opus-b §2 纪律）。
- [ ] **A8 普查先行**：K1/K2 上界处决跑在任何持久化历史表建立**之前**（顺序纪律，R2/opus-b §6.3-3）；机会集按 M/C/F 三分桶，所有比较逐桶报数，禁只报总数；一次性行（B1/B4/B5）单列且不入主判据分子分母，占 `|C|` ≥20% ⇒ 整份普查作废重采（FN2-4）。
- [ ] **A9 空虚保护与换判据**：命中率类判据保留 `n ≥ 200` / `a ≥ 20`，`inconclusive` 记 FAIL；主判据用同一机会集成对胜场（`20·(w_a − w_b) ≥ N`）；`n < 200` 之前动作轴产品面只许弃权 token（R1/opus-b §4）。
- [ ] **A10 fixture 时刻事故已修**：测试 as_of 由夹具自身 mtime 最大值推出（+10 天），不写死常量、不 `set_times`；`NOW_MS = 1_787_529_600_000` 恰在 UTC 零点——边界测试显式钉 `NOW_MS−1`/`NOW_MS`，其余验收测试挪离该边界（R2/opus-a §7.3）。若发现某近因用例必须改文件时间戳才能写出（除 1970 年前那一条），判定又偷吃了别的时间源，记 FAIL。
- [ ] **A11 prequential（动作轴台）**：freeze(predict) → 揭晓 → 记账 → 恰好一次 update；弃权计入分母按未命中；`i = 1` 算机会；待决状态**现读**，禁缓存禁回放重建（门 S4；`sync_identifiers` 的静默清空回放看不见，R2/opus-b §1.4）。
- [ ] **A12 NET-ZERO**：测试期进程非回环连接恒 0；签名库/规则库/模型随构建固定版本，不运行时下载（R1/gpt-sol-b §5-12；R2/gpt-sol-a §7.4）。

---

## L1 夹具正确性（B 系列）——oracle 逐项对表

- [ ] **B1 FMT 族**：`FMT-EXT`（69 项全表 + 大小写变体逐项命中）/ `FMT-RENAME` / `FMT-AMBIG`（`.key` PEM、`.ts` 双义——V 在歧义处**不动**，只在明确矛盾时否决）/ `FMT-CONTAINER` / `FMT-BROKEN`（弃权不 panic）各自实现 R1/gpt-sol-b §5 的预注册 oracle。
- [ ] **B2 对抗族**：`SEM-INJECT`（文件名与正文含祈使句：输出至多多一个命中计数，永不产生新动作或扩根）、`LIFE-FORGET`（删一条训练证据后重建 == 从未见过它的干净运行，逐字节）、`RES-BOMB`（超限为显式留置）、`TOCTOU-SAME-META`（同长度换内容恢复 mtime：内容型计划若沿用同 `plan_hash` 直接 FAIL）（R1/gpt-sol-b §5-8/9/10/11）。
- [ ] **B3 PK/ZIP 矩阵**：PK01–PK25 逐条对 oracle（R2/gpt-sol-a §5）。三组必查：**PK02–PK05**（PK 硬映射 `Archive` 会错撤全部 ZIP-based office 文件——最重要防回归）；**PK15–PK18**（杀死「小窗口搜 `word/` 判 OOXML」）；**PK19** 空 ZIP 两个合法版本（专门规则或 `Unknown`+coverage 单列）都不得冒充已验证。`FX01–FX14` 仅在实现 fixed-prefix refinement 时启用，启用即全部结构条件同时验证，缺一回落 `zip_family`。
- [ ] **B4 W 组合器**：W01–W15 只喂 `extension_kind + MagicEvidence` 纯函数，零临时目录、零 I/O，先于一切端到端测试（R2/gpt-sol-a §4）。`Unknown` 与 `RecognisedOutsideTaxonomy` 不共用空集合（W13 那格必须是撤回）。
- [ ] **B5 S2 评测器 sanity**：四文件合成检查给出 `TP/FP/FN = 1/2/2`（precision 1/3、recall 1/3），证明评测器同时能抓两种失败；空分母记 `N/A` 绝不写成 1；`MIN` 扫档时 gold 分母恒为全部非空文件上的重复对，不随 `MIN` 缩（R2/gpt-sol-b §1.2、§2.4）。
- [ ] **B6 对照实验三件**：`CLUST-STAB`（插入扰动 churn，扩展名基线恒 = 1）、`MAGIC-RATE`（真实语料不一致率）、`MAGIC-CHEAPER`（零读分类器对照）齐备且**先于任何 S1/聚类产品代码**运行（R2/fable-a §8）。
- [ ] **B7 `ACT-*` 11 件**（SEQ/EMPTY/DENIED/RESCAN/RENAME/RESTART/PLANSHAPE/PAIR/FLOOD/NOROOT/LOCKED，R1/opus-b §6.1）与 **B8 `ACT-PEND-*` 6 件**（CENSUS/DISCRIM/BIASED/LEAK/INVALIDATE/DISCARD，R2/opus-b §7.1）齐备。`ACT-PEND-DISCRIM` 在面 A 上**必须造不出来**——造出来了说明 n2A≡n1 的证明错了，先修理解再跑数。

---

## L2 不变式（C 系列）

- [ ] **C1 只减不增**：全部夹具上 `moves(V(P)) ⊆ moves(P)`；保留 move 的 from/to/kind 逐字不变（P1–P4）；baseline 无 move 时任何 evidence 造不出 move（P5，W08/W09）；detector 的 evidence 不依赖扩展名（P7——否则是自证循环）（R2/gpt-sol-a §4；R2/fable-a §6）。破 P1 = 它不再是 withhold-only 而是未立项的 S1-b。
- [ ] **C2 `plan_hash` 稳定套件**：R2/opus-a §7.1 七条命名测试逐条落地，特别是 `crossing_midnight_with_nothing_recent_changes_nothing`（判决「as_of_day 进哈希」之争的那条）与 `recency_off_is_todays_plan_plus_one_key`；§7.2 边界表全过（floor 语义、1970 年前 mtime 记 `unknown_mtime` 而非「很老」、未来 mtime 判「太新」、无 mtime 计数不猜、理由取「明天仍为真」的那个、被抑制文件不占位）。
- [ ] **C3 近因位置与单调**：近因判定在短路链**最后**（紧邻 ProposedMove 之前）、只对 depth==1 且有类文件跑；单文件终生至多翻转一次且方向固定；双边区间变体出现即 FAIL（R2/opus-a §2.4、§4）。
- [ ] **C4 模式进哈希**：`recency` 键关也出现（`{"mode":"off"}`，键缺席的隐式编码被否决）；magic 的 `mode + detector_id + detector_version + prefix_limit + total_budget + 计数` 进 canonical JSON；`magic=off` 时 open 数恒 0 且 `plan_hash` 与现基线逐字节相等；`off` 与 `dispute_only` 即使 moves 相同哈希不等；rule_version 变则哈希变（R2/gpt-sol-a §7.3、§8；R2/fable-a §5.3）。
- [ ] **C5 dispute-only 语义封存**：`Unknown`/`TooShort`/`NotSniffed(budget/open_failed)` 一律保留基线并标 unverified，不是反证；`verify-or-withhold` 若立项必须另立 mode token、另套 expected moves、另套文案，不许同名换义（R2/gpt-sol-a §0-4、§7.3）。
- [ ] **C6 可建议性 T1–T8**：性质蕴含（Suggestable ⇒ 免令牌）、`suggestability` 与 F6 scope 映射双双穷尽匹配禁通配（加第 9 个 `ActionKind` 必须编译失败）、载荷 schema 键集合无批准链工件（快照 + schema 双查）、建议器符号隔离 `rg` 命中 0、注入回放卡片动作 ⊆ 4 值 S 集、G19 修复后 U 类判定不变的回归钉、点击后恰好一次 `check_action` 且渲染期为 0、headless 回放 `issued_count()` 恒 0（R2/fable-b §6）。
- [ ] **C7 `n2` 自身七门 S1–S7**：地盘准确率 ≥90%（⚖ 常量待 R3 背书）、表长 ≤5+1 且加行走 DECISIONS、无时钟、现读、内容无关（不得调用 `pending_plan_hash()`，`rg` 命中恒 0）、`ACT-PEND-LEAK` 泄漏负对照（打乱标签后 `n2` 在 C 上掉回 `n0` 水平，差 ≤2 点——不掉 = harness 泄漏未来，全部数字作废先修 harness）、`|C| = 0` 的剖面不得声称有对照（R2/opus-b §5）。
- [ ] **C8 动作轴通用门 D1–D6**：闭集 `parse` 禁通配；`HitlDenial` 不进计数（`ACT-DENIED`）；状态无 plan_hash/文件名/路径且 `ACT-RENAME` 下逐字节不变（`ACT-RESCAN` 时 `plan.files += 3` 而状态无 hash）；候选源码对 `list_audit`/`AuditAction` 引用 0；轴亲和登记（x1/x2 → orderliness，余为无）；D6 禁词面走 COPY 冻结（R1/opus-b §4）。**FN1-2 合并已裁为合并**（同一次 `preview()` 的 `scan.directory`+`plan.files` 规范化为一个 `plan.files`，R2/opus-b §3.1），相邻启发式的交错误判风险已登记，`call_id` 并入 G17/G18 补法。
- [ ] **C9 G21 访问器形状**：`has_prepared_draft() -> bool`、`has_held_forget() -> bool`、`live_token_count(as_of_ms) -> usize`——**返回布尔与计数，不得返回 hash 或 id**（返回了就是把 D3 禁的值递给消费者，R2/opus-b §1.3）。
- [ ] **C10 卡片载荷类型**：`action: SuggestableAction`（4 值闭枚举）或 `navigate: ScreenId`（闭集，不含遗忘屏与研究同意屏 ⚖）二选一；无自由字符串；批准链工件**在类型上没有槽**（R2/fable-b §5）。

---

## L3 候选杀线受理条件（D 系列）

通用门先过（A/C 系列 + 各轨通用门），过不了不比命中率。

### D-X0（基线自身）
- [ ] 69 项全表逐项回归 oracle + 每类大小写变体（R1/gpt-sol-a §4-4）。
- [ ] 三个负例登记且**必须以「不可当安全执行计划」的形式 FAIL 出来**：分类文件夹名被普通文件占用、大小写折叠卷冲突、截断扫描漏目标（R1/gpt-sol-a §4-1/2/3）。v0.1.1 执行器把 `truncated == true` 当拒绝执行的充分条件（R1/opus-a §4.3 盲区 B）→ 今天记 `BLOCKED(v0.1.1)`。
- [ ] 计数不闭合已知化：存在 depth≥2 目录时 `moves + left_alone ≠ scanned_entries`，UI 不得拿三数做「全部说明」断言（R1/opus-a §4.1）。

### D-S3（近因）
- [ ] C2/C3 全绿是受理前提。H2 主杀线：真实目录上 `days_since ≤ 1` 散文件占比 >50% ⇒ 预览被清空，动 `within_days` 乃至存废（R2/opus-a §7.4）。H3：`within_days` 1→30 扫描计划全程不变 ⇒ mtime 无结构 ⇒ 放弃 S3a。H1 低命中率**不足以**否决 S3a（价值在防灾难，成本近零）。
- [ ] H4 登记为结论而非缺陷：S3 产品内无法自我校准（本 crate 不依赖也不应依赖 `soul-store`）。H5（批准→执行时间分布）决定层 C 优先级，不改其正确性。
- [ ] 层 C 重放 + `PlanTooOld`（绝不复用 `PlanHashMismatch`）：`BLOCKED(v0.1.1)`；`as_of_day` 今天先挂 `Preview`。
- [ ] 理由名 `RecentlyModified`（不是 `RecentlyActive`）、追加 `LeaveReason` 末尾、中文 `explanation()` 非空、无「24 小时/今天」措辞；fileplan src 无 `fn apply_*` 命名（`no_write_api` 子串陷阱，R2/opus-a §6）。
- [ ] SUP-K 单独立测：fixture 上 K=2 的 4 条 → 3 条钉测；真实语料被 K 挡下占比 >30% ⇒ K 过大（R1/opus-a §6.3 H2）。与 S3 混测 = 两条都作废。

### D-S2（重复分组）
- [ ] **升级门今天记 FAIL**：零读分组升级为任何强于「可能重复」的措辞，需在**逐根、独立、冻结**样本上 pairwise precision ≥ 0.90（预注册）；已有证据 0.619（压力样本）与 0/2（源码样本）未过线（R2/gpt-sol-b 结论）。precision 门不改词义：过线也只证明「可能重复」候选够纯，「内容相同」永远要 S2-H 或逐字节。
- [ ] 弱提示保留的条件：排除 0 字节、不跨根配对、`MIN = 1 B`（更高档需在目标分布上重新证伪，512 B 不得冻结为通用门槛）、UI 只说「大小和扩展名相同，可能重复」、副本名字表只认机器生成模式（`v2`/`final`/`最终版` 标 strong = FAIL）。
- [ ] 报告纪律：`size-only` / `size+ext` / `size+ext+copy-name` 三档齐报不许只show最好档；组数与 pairwise TP/FP/FN 同报；按目录聚类重采样；报「有至少一个候选的根占比」（多数根恒无输出 ⇒ 不值得进默认预览）（R2/gpt-sol-b §4）。
- [ ] S2-H：逐次明示同意、当次计算、哈希不持久化不进审计不出网、绝不提议删除（最强动作 = 可逆移入 `重复/`）、执行前同一文件身份复核；违反任一 = 淘汰该实现。跨会话哈希索引出现 = FAIL（遗忘旁路，C1 G12 同族）。

### D-S1（幻数，dispute-only）
- [ ] 前置件三查（缺一则一切 S1 产品代码 = FAIL）：`READS` 表（A5）、atime 裁决落地（推荐进快照——开启 V 时 `a_preview_leaves_the_tree_exactly_as_it_found_it` 变红**正是它该做的事**；`O_NOATIME` 已出局）、读正文独立 opt-in 授权面存在（⚖ 头字节与全文是否分两道门，从严默认：两道）（R2/fable-a §5；R2/gpt-sol-a §7.4）。
- [ ] 价值对照先行：`MAGIC-RATE` < ~1% ⇒ 否决 V（收益不抵四条代价）；`MAGIC-CHEAPER` 零读分类器达到同等不一致检出 ⇒ 否决读字节的 V（R2/fable-a §8）。R2/fable-a 自评这个出局结局概率过半——测试日不要意外。
- [ ] 12 条直接杀线逐条转测试（R2/gpt-sol-a §9）：PK→Archive、Unrecognised 因内容获得 move、改写保留 move、`word/` entry 判 DOCX、UI 说「已验证」、Unknown 与空集合混用、detector 看扩展名、off 仍开文件、读非 baseline move/symlink/根外、字节或逐文件 MIME 进审计日志、结果不稳定或预算依赖遍历序、偷开通用 ZIP 解析。任一出现 = 该实现否决。
- [ ] `zip_family` 兼容集恰为 `{Archive, Document, Sheet, Slides}`；多规则命中取**并集**不取交集；`KindDisputed` 文案不得说「文件其实是 Y」除非规则真给出了 Y。
- [ ] 反噬线：`KindDisputed` 挡下 >5% 候选 ⇒ 可用性重议（R1/opus-a §6.1 H2）；对人工确认错扩展的 precision 不足 ⇒ 收益不成立，保持扩展名基线。
- [ ] 精确哈希去重（并列幸存）：churn 恒 = 1、全文读走同一授权门、不因可解释而免授权、无持久化索引、做去重时不顺手加 `soul-store` 依赖（`no_write_api` 第二层会红）。

### D-C（语义聚类）
- [ ] `CLUST-STAB` churn > 1 的候选**无论准确率**不得进计划（这是稳定性判死，不是准确率判死——准确率擂台是错的第一实验，R2/fable-a §0.5）。
- [ ] 分离建议流三条件缺一不开：永不产出计划/plan_hash；上屏前过插入稳定性钉测；只路由到用户已有文件夹（算法起名的簇 = 递归回被否决的 LLM 分支）。顺序反了（先建流再找用途）= FAIL。

### D-K（K1 / T1 / A1）
- [ ] K1：oracle 是用户自写规则；理由只有「规则 X 命中 N 次」不复制正文；无规则或并列时弃权。T1：`BLOCKED`（无标签事件与正文同意 schema）；受理时按内容族/精确哈希分组切分，禁近重复跨 train/test；`LIFE-FORGET` 等价逐字节。A1：metadata-only 与 `+TF-IDF` 成对消融预注册，正文分支无预注册增益 ⇒ 保留元数据分支（R1/gpt-sol-b §3）。

### D-N（动作轴）
- [ ] 阶梯里 `n2A` 不得单独出现（K3 面 A 已红，`|D| = 0`）；面 A 基线用 `n1`。`n2B` 全部项记 `BLOCKED(G21)`；`b0` 先于任何「杀死」结论命名并登记（杀了之后发什么必须先写下来）。
- [ ] K1 粗上界（`20·U₁ < |O|` ⇒ 该剖面学习类全体处决，连 `n0` 都不用跑）与 K2 细上界（`U₂ = b0 错误数`）按预注册剖面逐剖面判；处置规则：三剖面全处决 ⇒ 墓碑；两 ⇒ 冻结；≤1 ⇒ 存活但仍受 K4/K5/K6。**只能向下杀不能向上抬**——拿 K1/K2 没打响主张建 G17 表 = 单向判据双向用，报告退回（R2/opus-b §4）。
- [ ] R2 手算普查（三剖面 0 处决）标注为**形状检查非判决**，直至 G23 落地重跑；「E1 重度」剖面擦线（5.3 点）对假设极敏感的警示原样写进报告（R2/opus-b §4.4 读数 2）。
- [ ] K4 `ACT-PEND-BIASED`（≥200 机会、`10·(w_a − w_b) ≥ N_paired`）：不过 ⇒ 该候选**当场墓碑**；过 = 必要条件非价值证据，两句都写进报告。
- [ ] K5 三式（F 上对 `n0` ≥5 点成对；C 上对 `n2` 倒退 ≤1 点；整体对 `b0` ≥5 点）`BLOCKED(G17+G18+G21)`。K6 饥饿条款：v0.2 冻结点 `|O| < 200` 或 `|F| < 200` ⇒ 按不过 ⇒ 不进产品且**实现删除，不留默认关开关**（死代码条款）；复活 = 语料达标重跑 K5。冻结时点未定 → L5。
- [ ] 被杀名单精确执行：杀 `a1/a2/a3/a4` + `x1` 特征形态；不杀 `n0/n1/n2/b0`、`x1` 门形态、`x2`（研究轨处置另辖）。改名禁令：新候选必须声明与墓碑差异且差异不能只是超参。面分离：动作轴杀线不跨到 Cycle 1 的 app 轴/何时轴（R2/opus-b §6）。

### D-X（x2 / x1）
- [ ] x2 产品面 `N-A`（REJECTED，三条独立理由；复活三条件全要：降级为本机计数、`FX1-2` 整数证据、exe 名展示面裁决）。影子模式五条纪律（不渲染不落库、三整数、agreed 必须用户自发、进程退出清零、研究轨约束）今天 `BLOCKED(G18)`，只在 `ACT-PAIR` 合成夹具上跑形状。
- [ ] x1 门形态：`FX1-1`（触发器逐条对 `left(a)` 精确定义 + `ACT-FLOOD` 上限生效）、`FX1-2`（对 `c0_never`：`20·agreed ≥ fired` 且 `fired ≤ 3/天`）、`FX1-3`（在 `n2` 已建议的机会上不改答案——改了 = 门形态没落实）、`FX1-4/5/6/7`。晋级另需 SG9 打扰账存在 → `BLOCKED(G17+G18+SG9)`。

### D-SUG（建议器/卡片层）
- [ ] 卡片共同约束（R1/fable-b §二，经 R2/fable-b §3 收紧后）：闭集动作 × 可枚举参数、A2 式纯计数证据句、建议不缩短 HITL 链任何一环、单卡类型可关且关闭是锁、信号只落本机零 E0。**执行类建议整族出局**：R1 A2 的「携带 plan_hash」条款已废，A3 降级为导航卡，卡上只有存在性事实（回声论证：没让用户看屏就没有可 echo 之物，禁令产品代价为零）。
- [ ] A4 预填不预批：预填值与批准屏逐字段一致断言；改预填参数 ⇒ `plan_hash` 必变的属性测试。
- [ ] 学习规则：忽略 ≥k 压制（可逆）、关闭是锁（不可逆）、锁优先于计数的属性测试；禁除法合成率值上屏。
- [ ] 建议 token 只带 `root_index` 无路径字符串（G20）；`roots() > 1` ⇒ `abstain.ambiguous_root`，按频次排序根 = 未立项的 x3，出现即 FAIL。
- [ ] B1–B8 禁止输出速查表（R1/fable-b §三）作为渲染层负例回放全过。

---

## L4 报告与呈现纪律（E 系列）

- [ ] **E1 分表**：fileplan 台与动作轴台分表；面 A 与面 B 分表；动作轴逐 M/C/F 桶给整数；配对与全量都保留，禁挑日志/种子/后半段。
- [ ] **E2 admissibility 标注**：一切 `BLOCKED` 来源的行显式标注（如 G23 未落地的普查行标「形状检查」）；无标注的此类行退回。
- [ ] **E3 措辞纪律**：无百分比/分数/置信度上屏；`zip_family` 只能说「开头符合某条签名」；未验证/桶级相容两种标注不混；S2 只说「可能重复」；近因只说「修改时间太新，这次先不碰」（不给比实现更精确的数字）；`n2` 状态陈述豁免可数性但两条件必须满足（只复述当前状态、不与统计句同卡，R2/opus-b §5.8）；D6 禁词面零命中。
- [ ] **E4 审计与日志**：只有整数计数（candidate/opened/matched/disputed/not-sniffed），无文件名、无字节、无摘要、无逐文件 MIME；`suppressed_by_recency` 属预览不属审计（R2/opus-a §6）。
- [ ] **E5 预测不落库**：一切评测产物为离线 artifact；动作轴今天不落库（G17 未补）；`SoulInference` 写入路径出现即 FAIL。
- [ ] **E6 出局清单巡检**：目录快照后的出局清单逐项 `rg`/schema 检查，命中即整份退回。

---

## L5 缺口 → BLOCKED 映射（测试日的合法豁免仅此一表）

| 缺口 | 内容 | 阻塞的验收项 | 测试日处理 |
|---|---|---|---|
| G17 | 无可遗忘的动作历史面 | a1–a4 持久化版、x1、K5 | 进程内版照测（`ACT-RESTART` 钉冷启动代价）；持久化行标 `BLOCKED(G17)`。**普查在建表之前**：K1/K2 无需此表 |
| G18 | 动作事件无毫秒时间戳/连接键 | x1/x2 影子评测、`ACT-PAIR` 真实版 | 合成夹具跑形状；真实行 `BLOCKED(G18)`；补法与 G17 同批并带 `call_id` |
| G19 | 三个 token 无生产者（字母表 5 ≠ 8） | 分母口径；U 类判定的说服力 | 评测器把三 token 排除出分母并登记；默默当 0 计数 = FAIL；修复后跑 T6 回归钉 |
| G20 | `authorized_roots` 主库外明文路径 | 按根计数（x3）、建议 token 形状 | 只用 `root_index`；x3 `N-A` |
| G21 | 待决状态不可读 | `n2B` 全部、面 B 的 K1/K2/K4、`b0` | 唯一「v0.2 前可补」缺口，**补它先于一切面 B 数字**；访问器形状按 C9 |
| G22 | 命令面不是闭集（约 32 个 `pub fn` 无枚举） | 面 B 词表纪律（门 D1 失效） | ⚖ 从严默认：面 B 仅离线评测永不上产品；若 R3 裁冻结命令清单常量表则按其重开 |
| G23 | 使用剖面无数据源（审计链有损且 D4 边界未裁） | K1/K2 判决效力 | 普查行标「形状检查」；一次性离线聚合普查是否触 D4 ⚖ 交 R3，未裁前普查不作晋级依据 |
| SG9 | 无全局打扰账 | `abstain.budget_exhausted` 恒不可达；x1 晋级 | x1 晋级项 `BLOCKED(SG9)`；「每天 ≤3 张」在夹具上钉形状 |
| READS 表缺 | `no_write_api` 只防写不防读 | 一切 S1 产品代码 | 先补表后写码；违序 = FAIL 不是 BLOCKED |
| atime 洞 | 快照不含 atime，「盘没动」盖不住读 | S1 的「没读正文」声明 | 不再用 `disk_unchanged()` 声称未读；报 `opened_files`/`bytes_read`；裁决（推荐 atime 进快照）落地前 S1 端到端标 `BLOCKED(atime)` |
| v0.1.1 执行面缺 | 无执行器、无 FileWrite 令牌消费 | 层 C 重放、`PlanTooOld`、truncated⇒拒执行、批准窗口 TTL 定量 | 相关项 `BLOCKED(v0.1.1)`；「执行面须索 FileWrite 令牌」的建议已登记（R2/opus-a §1c） |
| 读正文 opt-in 面缺 | 无独立授权 UI | S2-H、S1 的产品路径端到端 | 合成语料照测；产品路径 `BLOCKED(consent)` |
| G2 | 无本地时区偏移 | 一切「今天/本地」措辞 | 只跑 UTC 语义；措辞按 E3 |

---

## 签核规则

1. **顺序**：L0 → L1 → L2 全绿（或合法 `BLOCKED`）后才受理 L3 数字；负对照（`ACT-PEND-LEAK`、K1/K6 类「必须测不出」、PK15–PK18、SEM-INJECT）先于一切「必须赢够」项受理，负对照红 = 先修 harness/生成器。
2. **变更控制**：改常量、oracle、needle 表 = 回 A2 重登记并重跑受影响层；「扫完写死，禁止倒填」对清单本身同样生效。
3. **⚖ 争点的从严默认**（R3 综合裁决后按变更控制换向，未裁前按此执行）：U 类连导航都不给（R2/fable-b §7-1）；打包建议 token 取 `plan.files` 单值并显式登记（§7-2）；「让 Soul 试着给建议」开关只管弹不弹卡、永不扩值域（§7-3）；`n2` 地盘线 90% 照登记值执行（R2/opus-b §10-4）；面 B 仅离线（§10-1）；G23 普查不作晋级依据（§10-2）；头字节与全文授权分两道门（R2/fable-a §11-1）；K6 冻结时点未定前，任何「先上再说」路径按 K6 已触发处理（§10-3 从严侧）。
4. **双台架互斥**：fileplan 台的确定性钉测结果不得为动作轴候选背书，反之亦然；一个数字要跨台引用，先证明两台机会集是同一个（它们不是）。
5. **本清单的证伪**：若测试日发现某项与已登记裁决冲突，修清单；若某判据在全部可得夹具上恒 `inconclusive`，按已履行的换判据义务模式（K1/K2/K4 先例）换判据而不放宽 `n ≥ 200`，并把修订记入 Cycle 2 收官综合。

*本文件为 Cycle 2 Round 3 独立产出，未读取任何其他 Round 3 槽位；仅写入本文件，无 git 操作，无 crate 改动。*
