MODEL_SLUG: claude-fable-5-thinking-xhigh

# Cycle 2 / Round 3 / fable-a — SOTA 冻结：程序/代理层算法候选目录（终版）

本文件是 Cycle 2 的候选集**冻结令**：从此刻起到测试日，候选目录只减不增，常量只按登记程序钉死，任何解冻走 §10 的变更规则。
依据：`cycle2/R2-SYNTHESIS.md`（本轮唯一上位裁决）+ cycle2 round1 全部六份 + round2 全部五份。独立作业：**未读任何其他 Round 3 槽位**。
**不重开 P0–P8（Cycle 1 目录原封不动）、不实现 Goal 2、不写产品 crate、无 git 操作**；写入面恰为本文件。

---

## 0. 冻结了什么、凭什么冻结

冻结对象五样：**(a) S 轨候选目录**（fileplan 后继），**(b) N 轨候选目录**（下一动作）连同**可建议性边界**，**(c) 共同纪律**，**(d) 墓碑区**，**(e) 前置缺口与代码洞清单**。
不冻结的：待扫常量数值（先登记候选集、扫完写死、禁止倒填）、真实语料普查数字（G23 落地前一律是形状检查不是判决）、REAL 夹具的具体目录（本机产生，不出网）。

入选判据（沿既有裁决，汇总为终版四条）：

1. **数据面**：输入 ⊆ 四数据面 + 授权根内目录元数据。默认配置读第三人正文字节数恒 0；任何字节读取走 §5.9 的两道授权门，且读的是格式证据不是正文语义。多一样即墓碑。
2. **算子准入**：任何作用在计划上的读字节/吃时钟算子，必须同时过 P1–P4（内容局部性、插入稳定性 ≤k、`plan_hash` 无假拒绝、可同意的理由；fable-a R2 §2，R2-SYNTHESIS 背书），且满足**只收缩或只注解**：`moves' ⊆ moves`，永不因内容/时间新建移动。
3. **可数性与可同意句**：输出是闭集 token / 闭集理由 / 整数计数；每条裁决落成一句 `LeaveReason::explanation` 级中文；措辞不得比实现更精确（`zip_family` 不许说「已验证」，日量化不许说「最近 24 小时」）。状态陈述按 §5.6 豁免条件处理。
4. **可证伪**：每个候选有具名主杀线与专属夹具；`inconclusive` 一律按不通过；证伪实验先于准确率擂台（§5.8）。

**Cycle 2 主题三分的下落**：fileplan 启发式 → S 轨；下一动作 → N 轨；进程身份未产出独立候选——`AppIdentity`（exe 名 + 时长）仍是唯一合法的「用户在干什么」信号（fable-b R1 X3/X4），`left(a)` 事件定义随 x1/x2 冻结（opus-b R1 §1.8），exe→软件类别映射表被判为第二个人群先验（x2 出局理由 3），exe 名展示面沿 C1 §7-8 遗留继续悬置。

---

## 1. 基线（冻结为一切后继的零读对照）

`x0.extension_table.v1`：文件名末段后缀小写化 → 69 项精确表 → 8 kind → 固定中文目录；`build()` 五级短路（Directory / AlreadySorted·Nested / UnrecognisedKind / DestinationTaken / ProposedMove）；纯函数、零 I/O、无时钟、无配置；`executable_in_this_version: false` 在被哈希值里。**它是 v0.1 唯一默认，也是全部后继实验的零读对照与 `magic:off` 回归基准（逐字节相等钉测）。**

基线的已登记事实**不是缺陷清单，是候选的输入**：`.key`→Slides 是唯一把敏感文件送错目录的条目（T1）；`tgz` 未收而 `tar.gz` 收（T4）；`taken` 不含 skipped/截断后不完整/精确字节比较三盲区（v0.1.1 前置，FH5）；深层目录使计数不闭合；`claimed` 今天不可达；`AlreadySorted` 只认深度 2，「放错文件夹」与「无关子目录」同句（Misfiled 缺理由）。表修订走备选位 `tbl.amend.v1`，不许悄悄改。

---

## 2. S 轨候选目录（fileplan 后继；并列，不选赢家）

**结构性事实先冻结**：三条后继可任意组合且顺序由构造决定——S3a 在 `build()` 第 5 级（`ProposedMove` 之前最后一道），S1 是 `build()` 下游对候选 move 的独立否决 pass（绝不走 scan/snapshot），S2 是并行注解层。被 S3a 抑制的文件不成为候选 move，故 S1 不会打开它；被抑制的文件不占 `claimed`。

| 槽位 | 冻结 id | 作用 | 主杀线（不过即出局/不上） | 状态 | 前置件 |
|---|---|---|---|---|---|
| S3a | `s3a.recency_withhold.v1` | 修改时间落在 as_of 近 K 个 UTC 日 ⇒ `RecentlyModified`「它的修改时间太新，这次先不碰」 | H2 反噬线：真实目录 `days_since ≤ within_days` 散文件占比 >50% ⇒ 预览被清空，调 `within_days` 或废；H3：`within_days` 1→30 扫描计划全程不变 ⇒ mtime 无结构 ⇒ 废；§8 稳定性钉测族任一红 ⇒ 不上 | `TESTABLE-NOW`（先修 FH3） | 层 A（as_of 参数 + FH7 CLOCKS 表）、层 B（`UtcDay` newtype、`div_euclid`、**日历日之差**、阈值粒度=量化粒度） |
| S3-层C | as_of 钉住与回放 + `PlanTooOld` 理由码 | 批准带走 `as_of_day`（旁挂值，**不进哈希**），执行时重放；隔 >1 日拒为「太旧」不为「变了」 | 重放定理被推翻（找到陈旧 as_of 多提议移动的例子）⇒ 废 | `TESTABLE-AFTER-GAP`（v0.1.1 执行面 + FH9） | `ReasonCode` 加变体并进 `ALL` |
| S3c | `s3c.recency_display.v1` | 近因分档显示，只在渲染层 | 消费了计划外时钟 / 进了 `to_json` ⇒ 废 | `TESTABLE-NOW` | 必须消费计划自己的 `as_of_day`；档不是量表（对物不对人，注释写明与 D22 的区别） |
| SUP-K | `supk.min_kind_support.v1` | 单件类别不建文件夹（原 S3b，已从 S3 拆出：是支持度不是近因） | 挡下 >30% 候选 ⇒ K 太大；改动既有钉测须单独立测 | `TESTABLE-NOW` | 与 S3a 分开评估、分开落地 |
| S2-0 | `s2t0.dup_hint_zero_read.v1` | `(parent, size≥MIN, extension)` 零读重复候选组，**只注解** | 预注册线：逐根 pairwise precision ≥0.90，未过 ⇒ 不得升级措辞、不得进默认预览。已测事实钉入：压力语料 26/42=0.619、源码语料 0/2、`MIN` 不可从这两个样本冻结（512 B 只找回 5/26） | `TESTABLE-NOW`（研究档） | E-S2 逐根实验四条协议（gpt-sol-b R2 §4）：逐根不摊平、看结果前冻结 `MIN`/口径/副本名表、三档消融全报、报「有输出的根占比」 |
| S2-H | `s2h.full_hash_confirm.v1` | 全量 SHA-256 精确重复确认，opt-in 金标准 | 未同意读取 / 哈希落库或进审计 / 跨根配对 / 0 字节成组 / 文件变化后沿用 / 「内容相同」变 delete——任一即废 | `TESTABLE-NOW`（隔离夹具）；**永非默认** | 走 Door-2 全文授权；同大小桶预筛；执行前同一文件身份复核 |
| S1 | `s1.magic_withhold.v1` | 有界头字节（≤512B）**只撤回**与扩展名桶明确不相容的候选 move → `KindDisputed`；`MagicEvidence` 闭枚举（`Match{rule_id, format_id, compatible_kinds}` / `RecognisedOutsideTaxonomy` / `Unknown` / `TooShort` / `NotSniffed{reason}`）；**PK = `{Archive, Document, Sheet, Slides}` 家族**（R2-SYNTHESIS 原文），多候选取并集；`Unknown` ≠ 空集 | 对局先行：`MAGIC-RATE` 真实语料不一致率 <1% ⇒ 否决整条；`MAGIC-CHEAPER`（零读「大小+名字模式」等效检出）⇒ 否决读字节版；`moves_after ⊆ moves_before` 属性破 ⇒ 它不再是 S1；PK02–PK05 任一 office 件被当 Archive 冲突撤回 ⇒ 实现废 | 规则层 `TESTABLE-NOW`（纯组合器 W01–W15 + 性质 P1–P7）；产品路径 `TESTABLE-AFTER-GAP` | FH1 READS 表、FH2 atime/`opened` 计数、Door-1 授权、模式字段进哈希（`mode + detector_id + detector_version + prefix_limit + budget + 计数`）；只打开基线本会移动的文件（数据流保证不提升） |

**S 轨不变陈述**：默认配置下 S 轨读第三人正文字节数 = 0；任何候选只收缩移动集合或只注解，唯一目的地词表仍是 8 个固定文件夹；v0.2 的 S 轨用户可见面 = X0 计划 +（各自过线后）`RecentlyModified` 理由、近因分档、`可能重复` 注解、`KindDisputed` 理由——没有任何「机器猜你想怎么整理」类输出。

---

## 3. N 轨候选目录（下一动作）

**轨道不变陈述**（冻结）：动作轴的诚实默认是沉默；`n < 200` 之前产品面只许弃权 token。学习候选的产品化闸门是 **K6 饥饿条款 + G17**：**无 forgettable 动作日志则学习候选不得产品化**（R2-SYNTHESIS 原文），且饥饿判死含死代码条款（删除，不留默认关开关）。

| 槽位 | 冻结 id | 角色 | 判定 / 杀线 | 状态 |
|---|---|---|---|---|
| n0 | `n0.action_marginal.v1` | 边际频率对照 | 不可证伪（地板）；`FN0-2` 拒绝不计入、`FN0-3` 状态无 hash | `COMPARATOR + TESTABLE-NOW` |
| n1 | `n1.action_mru.v1` | 第二地板 | **FN1-2 合并规则冻结**：同一次 `preview()` 的 `scan.directory`+`plan.files` 规范化为一个 `plan.files` 事件（合并，不排除；补 G17/G18 时动作事件带 `call_id`） | `COMPARATOR + TESTABLE-NOW` |
| n2A | （退位记录） | — | K3 已红：HITL 面上与 n1 逐点相同（`\|D\|=0`），**不得单独进阶梯**；「打赢 n2」在面 A 禁说 | `COMPARATOR`（名义） |
| n2B | `n2b.pending_state_rule.v1` | 待决状态规则，N2 唯一有意义形态 | 表 6 行封顶（5 实义 + 1 弃权；稳态只有 B2 生成起草 / B3 确认遗忘两行会开火），加第 7 行走 DECISIONS；门 N2-Q1…Q7（原 opus-b R2 S1–S7，**更名以避 S 轨撞名**）：Q1 地盘准确率 `10·hits ≥ 9·\|C\|`、Q2 表长、Q3 无时钟、Q4 现读禁回放、Q5 内容无关（禁 `pending_plan_hash()`）、Q6 泄漏负对照、Q7 存在性（`\|C\|=0` 剖面禁称有对照） | 面 B **仅离线评测，永不上产品**（本槽裁决 §9-4）；阻塞 G21/G22 |
| b0 | `b0.pending_then_marginal.v1` | 杀完学习后要发的基线：C 上用 n2、F 上退 n0 | K1/K2 的全部上界相对它算 | `COMPARATOR`；阻塞 G21 |
| a1 / a3 | `a1.action_markov1.v1` / `a3.plan_shape_conditioned.v1` | 学习候选（一阶转移 / 计划粗形状条件） | 五步程序 K1 粗上界（`20·U₁ < \|O\|` ⇒ 全体处决）→ K2 细上界（`U₂ = \|O\| − hits_b0`）→ K3 可分性 → K4 优待夹具（`ACT-PEND-BIASED`，`10·(w_a−w_b) ≥ N ≥ 200`，不过即墓碑）→ K5 三条 + **K6 饥饿**（冻结点见 §9-6：真实语料 `\|O\|<200` 或 `\|F\|<200` ⇒ 不产品化 + 删代码）。既有普查读数钉入：三剖面 0 类处决、E1 重度栏擦线且对假设极敏感、离线用户 `\|C\|=0` | 进程内 `TESTABLE-NOW`；产品化 `TESTABLE-AFTER-GAP`（G17，B 级） |
| a2 / a4 | `a2.action_prefix2.v1` / `a4.refusal_conditioned.v1` | 具名备选 | a2：赢 a1 ≥3 点且二阶查询占比 ≥5%；a4：赢 n1 ≥3 点（MRU 免费预测「重来一次」） | `NAMED-ALTERNATE` |
| x1 | `x1.exe_to_action_learned.v1`（**门形态**） | exe 转移只决定何时建议，内容由 n2/b0 给；属「何时」轴，继承 Fc-1/Fc-7 | `FX1-2`：对 `c0_never` `20·agreed ≥ fired` 且 `fired ≤ 3/天`；`FX1-3`：不得改变 n2 已给的答案；触发器 = `left(a)` 冻结定义 | `TESTABLE-AFTER-GAP`（G17+G18+SG9）🚫；特征形态在 K 系被杀名单内 |
| x2 | `x2.exe_to_action_prior.v0` | 硬编码「离开 code.exe ⇒ 整理 Downloads」 | 产品面 REJECTED（人群先验 / 无可数句 / exe→类别映射表三条独立成立）；影子研究三整数（fired/agreed/opportunities），进程退出即清零 | 研究轨 🚫（G18）；复活 = 降级为 x1 三条件 |

**处置语义冻结**（不许扩大或缩小）：K 系杀的是 `a1 a2 a3 a4` + x1 特征形态；不杀 `n0 n1 n2 b0` 与 x1 门形态。面分离：动作轴的生死不跨面援引到 next-app 轴与何时轴。新 N 轨候选先过 fable-b R1 十问速筛（冻结为 N 轨准入筛）+ 轴亲和登记 + 与墓碑差异声明。

---

## 4. 可建议性边界（冻结为类型纪律）

三分而非二分（fable-b R2）：**S（可建议，4 值闭枚举）** `draft.reply / analyse.people / scan.directory（与 plan.files 打包一次点击）/ plan.files`；**U（免令牌但仅限用户自发，连导航都不给）** `forget.preview / research.preview`——排除理由是推力方向（销毁压力/同意压力），**不是**未实现，G19 补生产者后判定一字不变（T6 回归钉）；**T（令牌门控，永不可建议）** `forget.execute / egress.generate`（v0.1.1 起 + FileWrite 文件执行，本轮已预登记）。

配套纪律全部冻结：

1. `suggestability(k)` 独立谓词，8 臂穷尽匹配禁 `_ =>`，加第 9 个 `ActionKind` 必须编译失败并同时登记可建议性 + 轴亲和 + scope 臂（FH6）。
2. **卡片载荷类型上没有** `plan_hash` / `preparation_id` / `token_id` 槽；执行类建议整族出局（回声论证：合法形态是空集，禁令产品代价为零）；「已批准未执行」提醒降级为导航卡。
3. 建议器符号隔离：对 `ActionRequest` / `TokenIssuer` / `CapabilityToken` / `PlanHash` 引用数恒 0；卡片存在本身永不铸令牌（T8）；`check_action` 只在点击之后（T7）。
4. **N 轨值域封顶由构造继承**：n2B/b0 与任何学习候选的输出字母表 = 4 值 `SuggestableAction` ∪ 导航（导航闭集不含遗忘屏与研究同意屏），学习器学不出「建议遗忘」「建议出网」——类型保证，不是训练目标。
5. 证伪清单 T1–T8 照 fable-b R2 §6 冻结；X5 注入回放用例族（祈使句文件名/正文永不回显为动作）随每个渲染面走。

---

## 5. 共同纪律（冻结引用，不再复述全文）

1. **算子准入 P1–P4**（fable-a R2 §2）：读字节/吃时钟算子进计划的充要条件；语义聚类在 P1 根上断裂故永不进计划；去重是聚类家族唯一全过成员。
2. **哈希准入准则**（opus-a R2 §3.1，本槽背书为冻结纪律）：一个值进被哈希计划 ⇔ 它能在可见内容一字不变时改变执行后果；行为开关照 `executable_in_this_version` 先例必进（`recency{mode, within_days, unknown_mtime}`、`magic` 模式字段）；**原始时刻与 `as_of_day` 永不进**（R2-SYNTHESIS 原文：内容未变却每日拒）。
3. **时钟纪律四条**（opus-a R2 §10）：as_of 是参数且一次调用一个时刻；量化到最粗粒度且量化**日历单位之差**；原始时刻不进哈希；判定对 as_of 单调并排短路链最后。双边区间近因、滑动 `age_days` 为 REJECTED。
4. **`LeaveReason` 纪律**：新变体只许追加末尾（`Ord` 是排序次键）；本 cycle 登记的新理由恰两个——`KindDisputed`、`RecentlyModified`（SUP-K 的 `TooFewOfKind` 随其立项走）；S2-0 只注解**不立理由**；每个新理由过 `everything_the_plan_leaves_alone_says_why`。
5. **词表纪律**：闭集 + 穷尽匹配禁通配（动作、理由、`MagicEvidence`、弃权 token）；mode token 语义钉死不迁移——`dispute_only` 与 `verify_or_withhold` 必须是不同 token、不同 expected、不同文案。
6. **证据句纪律**：只含已算好的整数与事实；`zip_family` 只能说「开头符合某条签名」；`unverified` / `bucket-compatible` 如实标注；状态陈述豁免可数性但必须（a）只复述当前状态无预测措辞（b）不与统计句同卡（opus-b R2 §5.8）；D6 禁词面（「你的下载文件夹很乱」等）照旧。
7. **整数判据**：交叉相乘、`i128`、判定步无浮点；空虚保护（分母 <200 或分子 <20 ⇒ `inconclusive` ⇒ 按不过）；弃权计入分母按未命中；逐桶报数禁只报总数；prequential；指标只活在评测器内部。
8. **实验顺序：证伪先于准确率**（本 cycle 的方法论主张，冻结）——聚类先跑 `CLUST-STAB`（churn >1 即出局，无关准确率）；S1 先跑 `MAGIC-RATE`/`MAGIC-CHEAPER` 再写产品代码；N 轨先跑 K1/K2 普查再决定要不要建 G17 那张表（普查在建表之前）。
9. **读取授权两道门**（本槽裁决 §9-10）：Door-1「打开候选 move 的头 ≤512 字节」（面小、可数、只作用于用户已看到的候选，卡片句式「打开了你看到的 N 个待移动文件，各读开头 512 字节」）；Door-2「全文读取」（S2-H 与任何 Tier-1 变体）。两道分立，不得共用一次同意。
10. **夹具走真实闸门**：动作事件由 `check_action` 实际返回产生、待决状态由真实 `prepare()`/`preview_forget()` 置位，禁手插；`ACT-PEND-INVALIDATE`（导入静默清空）手插造不出来，是这条纪律的存在证明。
11. **命名诚实**：改装机制不得沿用文献/工具名（不叫「集成 libmagic」「复现 KondoCloud」「Hawkes 门」）；杀死的候选改名重提须声明与墓碑差别且差别不能只是超参。

---

## 6. 墓碑区（否决 + 复活条件；无复活条件的按永久墓碑）

| 墓碑 | 理由 | 复活条件 |
|---|---|---|
| 无人值守代理循环（X1）；批准后计划可变（X7） | 产品锁原文 + AC-19 | 无 |
| 任意 shell / 生成脚本执行（X2） | 未知动作洗钱 | 无 |
| 键鼠采集与合成（X3）；屏幕/UI 树/窗口标题（X4） | 采集面锁「不做」 | 无 |
| 外部内容祈使句 → 动作（X5） | 不可协商 #8 | 无 |
| 置信度放行 / 风险分 / 永久允许（X6） | 令牌一次性 + 不可纠正黑盒 | 无 |
| 默认 LLM 读正文出网 / 在线 LLM 分类（任何执行路径投放，含本地在线形态） | 第三人锁 + 非确定性毁 `plan_hash` | 唯一幸存形态 = 离线规则编译器（备选位 `llmc`）；E1 已配时可选增强且不反写档案 |
| 语义内容聚类进计划（嵌入/HDBSCAN/k-means/LDA/主题模型，任何配置） | P1–P4 全破；假拒绝发生器；簇无名递归 LLM | 给出确定、插入稳定（churn ≤1）且仍做语义分组的反例（平凡解=去重，已另立）；建议流只走备选位 `clus` 窄门 |
| 固定小窗搜 `word/`/`xl/`/`ppt/` 判精确 OOXML；通用 ZIP 中央目录解析 | 可伪造 + 攻击者可控结构解析 | 独立攻击面/资源上限/授权评审（gpt-sol-a R2 杀线 12） |
| 滑动窗口 `age_days`、双边区间近因、`as_of_day` 进 `to_json` | opus-a R2 §8 三条 REJECTED | 无 |
| x2 硬编码人群先验（含 exe→软件类别映射表；「离开 code.exe 就整理 Downloads」产品句） | C1 冻结条款：MFU/MRU 外人群先验无复活 | 仅降级为 x1：本机计数 + FX1-2 整数证据 + exe 展示面裁决落地 |
| 拿审计链当特征 | 遗忘旁路（C1 已裁） | 无；G23 一次性普查走 D4′ 窄门（§9-5），不是复活 |
| 跨会话持久化内容哈希索引 | 永远活着、无遗忘路径的派生语料 | 无 |
| 字节频率 / n-gram 文件类型模型（B1） | 格式识别已有更简单可解释解 | M1 在预注册 intact-file 语料上出现量化覆盖缺口 |
| 全容器解析 / OCR / 通用嗅探（C1 族） | 攻击面 + 「本机运行」不回答最小读取问题 | 无 |
| 执行类建议卡（载荷含批准链工件） | 回声论证：合法形态是空集 | 无 |
| 按根频次排序建议（x3 原形） | 路径行为计数 = 可导出画像行（G20） | G20 落地后另立具名候选重审 |
| 无对照的「学到了习惯」宣称（`\|C\|=0` 剖面上打赢对照的说法） | Q7 存在性 | 无（该剖面动作轴整体弃权） |

---

## 7. 具名备选位（不点名到不了，启用条件冻结）

| id | 内容 | 启用条件 |
|---|---|---|
| `s1b.magic_promote.v1` | magic 把 `Unrecognised` 提升为移动 | 本 cycle 不启用（内容决定位置是类别差不是程度差）；新 cycle 审，且必须默认关 + 独立哈希模式 + 自己的对照 |
| `s1v.verify_or_withhold.v1` | 「验不出就不动」 | 另立 mode token / expected / 文案；不得改 `dispute_only` 语义 |
| `s1fx.odf_epub_prefix.v1` | ODF/EPUB 固定前缀缩小（`mimetype` 首 entry 规则） | 启用即 FX01–FX14 全体生效；条件不完整回退 `zip_family` |
| `s1z.zip_empty.v1` | `PK\x05\x06` 空 ZIP 专门规则 | 未实现时预注册 oracle 为 `Unknown` 并单列 coverage |
| `tbl.amend.v1` | 扩展名表修订（`.key` 除名、`tgz` 补录、avif/heif 等） | 先建 FH8 逐项回归 oracle；每次修订 = 新具名表版本 + 哈希跳变登记 |
| `misfiled.reason.v1` | 「在分类文件夹里但不是那一类」新理由 | S1 落地时单独立项（它的自然落点） |
| `fk1.user_keyword_rules.v1` | 用户自写关键词规则（原 gpt-sol-b R1 K1，**更名避撞**） | Door-2 正文授权后第一个语义候选；oracle = 用户写下的规则；弃权义务照 SEM 夹具 |
| `ft1.tfidf_user_folders.v1` | TF-IDF + 用户已有文件夹质心（原 T1） | 有用户标签后；禁自造目录名；删除训练文件后与从未见过它的模型相等 |
| `fsr.same_action_reco.v1` | 「你刚把 A 放到 X；B 也放到 X？」（原 A1，metadata-only 先行） | 批准/拒绝事件 schema（G17 同族）；`+content` 须预注册相对 metadata-only 的增益 |
| `llmc.rule_compiler.v1` | LLM 离线编译**一条用户可读、可编辑、带版本的确定性规则**；执行侧纯函数 | 默认关；被同意的对象是规则文本；输出过白名单动作校验（提示注入红线）；永不在线分类 |
| `clus.suggest_stream.v1` | 语义聚类建议流（与计划分离，永不产出 plan_hash） | 三条全要：`CLUST-STAB` churn ≤1 先过、只路由到用户已有文件夹、默认关 |
| `card.suppress_counts.v1` | 卡片接受/忽略整数计数压制与重排 | 压制可逆、关闭是锁、两者不混；排序继承 P7 纪律；计数落点须可遗忘 |
| `a2` / `a4` | 见 §3 表内 | 表内条件 |
| `x3.root_freq_rank.v1` | 按根排序 | G20 落地 + 遗忘单元归属先裁 |
| `previewoptions.refactor` | `preview()` 参数表整备（工程项） | 无判据，A 级登记 |

---

## 8. 夹具目录（冻结）

- **格式族**：FMT-EXT（69 项逐项 + 大小写变体）/ FMT-RENAME / FMT-AMBIG（`.key` PEM、`.ts` 双义、Ogg 容器）/ FMT-CONTAINER / FMT-BROKEN；PK01–PK25（PK02–PK05 是最重要防回归：office 件不得被当 Archive 冲突撤回）；FX01–FX14（仅 `s1fx` 启用时）；W01–W15 纯组合器 + 性质 P1–P7（P7：detector 不得看扩展名选 evidence，防自证循环）。
- **语义族**：SEM-CROSS / SEM-SUBJECTIVE（两套同样合理的用户标签，禁全局真值）/ SEM-INJECT（祈使句只能多一个命中计数）。
- **生命周期/资源**：LIFE-FORGET（删训练证据后逐字节等于干净运行）/ RES-BOMB / TOCTOU-SAME-META（同长换内容恢复 mtime，内容型计划同 hash 即失败）/ NET-ZERO（非回环连接恒 0）。
- **S3 稳定性族**（opus-a R2 §7.1 七条照钉，其中 `crossing_midnight_with_nothing_recent_changes_nothing` 是「as_of 不进哈希」的判决测试）+ 边界族（floor 语义、1970 前 mtime 记 `unknown_mtime` 不记远古、未来 mtime 判太新、抑制不占位、CLOCKS 对照用例）；除标注仅 unix 一条外全部不改盘——必须 `set_times` 才能测的近因用例 = 判定偷吃了别的时间源。
- **S2**：E-S2 逐根 evaluator 协议 + 合成 sanity check（跨扩展名 FN 与同尺寸 FP 双向可见）。
- **N 轨**：ACT-SEQ / ACT-EMPTY / ACT-DENIED / ACT-RESCAN / ACT-RENAME / ACT-RESTART / ACT-PLANSHAPE / ACT-PAIR（🚫G18）/ ACT-FLOOD / ACT-NOROOT / ACT-LOCKED + ACT-PEND-CENSUS / -DISCRIM（面 A 上必须造不出来）/ -BIASED / -LEAK / -INVALIDATE / -DISCARD。
- 全部渲染面夹具附一个提示注入文件名。

---

## 9. 本槽对登记争点的裁决（终版；父代理综合时可与其他 R3 槽对表）

1. **FN1-2 / 打包 token 形状**（opus-b R1 §9.1 + fable-b R2 §7.2，同一裁决两个面）：合并，不排除——规范化为单个 `plan.files` 事件；建议 token 同为 `plan.files` 单值；`call_id` 随 G17/G18 补。
2. **U 类连导航都不给**（fable-b R2 §7.1）：维持。遗忘是用户想起来才做的事；可发现性由记忆管理界面自身承担。
3. **「让 Soul 试着给建议」开关**（fable-b R2 §7.3 + opus-b R1 §9.4）：开关只管要不要弹卡，永不扩卡能指什么；值域封顶不随任何开关移动。
4. **面 A vs 面 B**（opus-b R2 §10.1）：面 B 仅离线评测、永不上产品；面 A 上基线的 C 段用 n1。G22 因此降 A 级（若 b0 日后上产品则升 B）。
5. **G23 与 D4 的边界**（opus-b R2 §10.2）：立 **D4′ 窄门**——一次性离线普查可读审计链，当且仅当（a）产品运行时之外（b）只出聚合整数（c）零逐用户持久化（d）输出只用于「要不要建 G17」这一个二值决策，永不成为模型参数或用户可见内容。D4 本体（运行时拿审计当特征）原封不动。
6. **K6 落槌时点**（opus-b R2 §10.3）：= Cycle 2 测试日（v0.2 目录测试开始那一刻）；K5 的语料只从 G17 落地后起算；届时 `|O|<200` 或 `|F|<200` ⇒ 不产品化 + 删代码；复活 = 语料达标后重跑 K5。
7. **N2-Q1 的 90%**（opus-b R2 §10.4）：背书 90%，预注册；它与 K 系的耦合（n2 越准学习天花板越低）是目录里最想保留的结构。
8. **离线用户无对照**（opus-b R2 §10.5）：承认动作轴对 `|C|=0` 剖面整体不适用——该剖面强制弃权，不另立对照；逐剖面报数（Q7）冻结。
9. **b0 要不要发**（opus-b R2 §10.6）：产品决策，登记给父代理；目录只冻结「若 N 轨有任何东西上 v0.2，用户可见面 ⊆ b0 稳态两行的状态陈述卡（受 §4 边界与 §5.6 豁免条件约束）」。
10. **读取授权粒度**（fable-a R2 §11.1）：两道门分立（§5.9）。
11. **建议流存废顺序**（fable-a R2 §11.2）：先过 `CLUST-STAB` 且限已有文件夹，才值得开；顺序反了 = 先建流再找用途。
12. **去重对位理由**（fable-a R2 §11.3）：被 gpt-sol-b R2 的更严裁决覆盖——S2-0 只注解不立理由；`重复/` 可逆移动属 S2-H 最强形态，若立项须同时带自己的理由与「保留文件夹」概念，本 cycle 不开。
13. **SG9 阻塞 x1 晋级**（opus-b R1 §9.6）：是。没有打扰总账的建议器，「每天 ≤3 张」只是自己对自己的承诺。
14. **哈希准入准则本身**（opus-a R2 §3.1 请 R3 攻）：攻不动，背书为 §5.2；唯一补充是把「约束用户的常量进哈希」（`within_days` 先例）写成准则的显式附款。

---

## 10. 冻结变更规则

1. **加候选**：本 cycle 内不受理。新候选走新 cycle：S 轨先过 P1–P4 + 读取账本申报；N 轨先过十问速筛 + 轴亲和登记；两轨都附与墓碑的差异声明（差异不能只是超参）。
2. **翻案**：只认已登记条件（各墓碑复活条件、x2 三条、K6 复活、B1 覆盖缺口、`s1b` 新 cycle 审）。条件未全数落地不受理。
3. **改常量**：走登记程序（`MIN`、`within_days`、K 系阈值、N2-Q1 的 90%、`MAGIC-RATE` 的 1%、副本名表）；已写死常量的改动 = 新具名变体，旧变体保留可比。
4. **命名**：mode token 语义钉死；枚举新值必须同时登记可建议性 + scope + 轴亲和（编译失败纪律）；`LeaveReason` 只许追加末尾；禁文献/工具名背书。
5. **上位锁变动**（采集面、v0.1.1 执行面形状、E0/E1、D22）自动触发全目录重审，但那是新 cycle 的事。v0.1.1 落地时 n2B 必须重写（预览有了「确认」步，待决槽形状变了）。

---

## 11. 测试日前置缺口（B = 阻塞该项；A = 诚实标注即可）

| 缺口 | 内容 | 级别 | 阻塞什么 |
|---|---|---|---|
| G17 | 无可遗忘的动作历史面（`ApprovedAction` 无写者；审计链不可当特征） | **B** | 一切学习候选的产品化（K5/K6 的正身；**N 轨第一前置件**）；补法含 `call_id` |
| G18 | 动作事件无毫秒时间戳、无与前台会话的连接键 | **B** | x1/x2 全部评测；ACT-PAIR |
| G19 | 三个孤儿 token 无生产者（字母表 5 不是 8） | A | 分母诚实；补生产者后 U 类判定不变（T6） |
| G20 | `authorized_roots` 主库外明文含用户名 | A/B | 按根计数 B；建议 token 只带 `root_index` |
| G21 | 待决状态不可读（补三个布尔/计数访问器，禁返 hash/id） | **B** | n2B / b0 / K 系面 B 版（唯一 v0.2 前可补的小缺口） |
| G22 | 命令面非闭集（约 32 个 `pub fn` 无枚举） | A（面 B 仅评测） | b0 上产品时升 B |
| G23 | 使用剖面无数据源（普查权重是编的） | **B** | K1/K2 判决效力；走 D4′ 窄门 |
| SG9 | 无全局打扰账 | **B** | x1 晋级（§9-13） |
| FH1 | `READS` needle 表 + 对照用例 | **B** | S1 产品代码 |
| FH2 | atime 进快照（或 `opened: N` 上卡）| **B** | S1 产品代码 |
| FH3 | fixture `NOW_MS` 事故（恰在 UTC 零点 + fixture mtime 是此刻；as_of 改由 fixture 推出） | **B** | S3a 的全部测试 |
| FH4 | `taken` → `HashSet`（O(F×E)→O(F+E)） | A | S2-0 成本模型 |
| FH5 | 占位盲区三条 + `truncated ⇒ 拒绝执行` | **B** | v0.1.1 执行器 |
| FH6 | hitl scope 映射穷尽化禁通配 | **B** | v0.1.1 新令牌动作（FileWrite） |
| FH7 | `CLOCKS` needle 表 + 对照用例 | **B** | S3a 层 A（净增强） |
| FH8 | 69 项逐项回归 oracle + 三个负例（父级普通文件、大小写折叠、截断目标） | **B**（对 `tbl.amend`）/ A | 任何表修订 |
| FH9 | v0.1.1 执行面要求 `FileWrite` 令牌（窗口被 TTL 约束） | **B** | S3 层 C 的定量意义 |

---

## 12. 一页版

- **S 轨（7）**：X0 基线（唯一默认 + 零读对照）· S3a 近因抑制（UTC 日、日历差、单调、层 C 钉住回放）· S3c 分档显示 · SUP-K 支持度 · S2-0 零读重复注解（措辞锁「可能重复」，precision ≥0.90 预注册未过）· S2-H 全量哈希 opt-in · S1 magic 只撤回（PK=四桶家族，`MAGIC-RATE`/`MAGIC-CHEAPER` 先行，FH1/FH2 补洞前无产品代码）。默认读正文字节恒 0；全部只收缩或只注解。
- **N 轨（默认=沉默）**：n0/n1 地板（scan+plan 合并冻结）· n2A 退位 · n2B 六行表仅离线评测 · b0 基线 · a1/a3 受 K1–K6 全程 + **K6 饥饿 = 不产品化 + 删代码**（G17 是正身）· a2/a4 备选 · x1 门形态 🚫 · x2 产品面墓碑。
- **可建议性**：S(4)/U(2)/T(2+1) 三分冻结；卡片类型无批准链工件槽；学习器值域由构造封顶。
- **纪律**：P1–P4 准入 · 哈希准入准则（as_of_day 永不进）· 时钟四条 · 两道读取授权门 · 证伪先于准确率 · 整数判据 + 空虚保护 · 真实闸门夹具 · 命名诚实。
- **墓碑（17 类）**：各带复活条件或标注永久。**备选（15）**：逐条启用条件。
- **测试日前必须动工**：G21（小，解锁 N 轨评测）、FH3（解锁 S3a 测试）、FH1+FH2（S1 的门槛）、G17（学习候选的生死线）；其余以诚实标注入场。

*本文件为 Round 3 独立产出，未读取任何其他 Round 3 槽位；仅写入本文件，无 git 操作，无 crate 改动。*
