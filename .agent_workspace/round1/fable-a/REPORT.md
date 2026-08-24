MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 1 / fable-a — SOTA 综述与 Soul 约束映射

角色：架构 / SOTA / 验收标准。产出配套文件：`EVAL_MATRIX.md`、`CANDIDATE_SPEC.md`、`ACCEPTANCE.md`、`NOTES.md`。

## 0. 先说一个改变格局的事实（现实快照漂移）

SHARED_BRIEF 写「WP10 人事分析未开工」。但 `origin/cursor/soul-goal1-7b1c` 分支 tip（`fbad85b`，提交信息即 "STATUS: WP10…"）已经落地了：

- `crates/soul-draft/src/analysis.rs`：`SummaryPoint`（无证据无法构造、构造期过 `assert_non_clinical`）、`PersonSummary`、`points_for()`（往来总数/方向比/一对一 vs 群聊/最近一次/归档句五类陈述）、无 key 时 `SummarySource::Counts` 降级、E1 改写只动 `narrative` 不动 points。

**这意味着 A2 不是白纸候选，而是在位实现（incumbent）**，且它是 `TieStrength` 计数的纯消费者。选型结论因此简化为：**A2 板上钉钉，真正的自由度全部在 tie-strength 一侧**——换 T 家族算法，A2 的句子自动跟着变，无需第二套阈值。这也是我推荐配对形态的核心依据（见 `ACCEPTANCE.md`）。

## 1. Ego 网络关系强度（tie strength）SOTA

### 1.1 理论谱系（按对 Soul 的可用性排序）

**Granovetter (1973), "The Strength of Weak Ties"** — 定义关系强度为时间量、情感强度、亲密度、互惠服务四者的组合。后续操作化研究 **Marsden & Campbell (1984), "Measuring Tie Strength"** 发现：亲密度（closeness）是最好的单指标，接触频率和持续时长只是有偏代理（proxy）。
→ **对 Soul 的映射**：Soul 不读正文（第三人 local_only、body 封存），情感强度与亲密度不可测；能测的只有 Granovetter 四要素里的「时间量」（count、active days、span）和「互惠」（双向都发过）。所以 Soul 的 tie strength 天然只能是**结构代理**，而且必须在 UI 措辞上承认这一点（现实现的 falsifier 字段已经这么做了）。T3（Granovetter-span）就是把这条谱系做成规则：强 = 互惠 ∧ 多日 ∧ 私聊。

**Gilbert & Karahalios (2009), "Predicting Tie Strength With Social Media" (CHI)** — 用 74 个变量预测 Facebook 关系强度，最强的预测维是 intimacy/intensity 类变量，其中大量依赖文本内容（wall words、inbox 深度）。
→ **映射**：证明「有正文能做得更准」，同时也证明了纯结构变量（交流天数、最近联系）单独就有可观信号。Soul 砍掉正文后剩下的恰好是这批结构变量——不是残废，是该论文里的可用子集。

**Onnela et al. (2007), PNAS（手机通话网络）** 与 **Eagle, Pentland & Lazer (2009), PNAS（从手机元数据推断友谊）** — 纯元数据（通话次数/时长/互惠）即可高置信重建自报友谊，Eagle et al. 报告 ~95% 的区分准确率。
→ **映射**：这是「不读正文的人脉图」的最强背书。同时 **Mayer, Mutchler & Mitchell (2016), PNAS（电话元数据敏感性研究）** 提醒：元数据本身就足够敏感——反过来支撑 Soul 的第三人 `local_only`、conversation id 只存哈希的设计。计数不出本机不是过度设计，是必要设计。

**近因/衰减谱系**：**Hawkes (1971) 自激点过程**（交流成簇、事件抬升后续强度，核函数常取 `exp(-λΔt)`）；**Burt (2000/2002) 关系衰减函数**（无接触的 tie 按可测函数衰减）；**Hidalgo & Rodríguez-Sickert (2008)**（手机网络中 tie 存续性）；**Miritello et al. (2013)** 与 **Saramäki et al. (2014)**（个人通信容量有限、社交签名随时间轮换但形状稳定）。流式计算侧有 **Cormode, Korn & Tirthapura (2008) 指数衰减聚合**，证明 `Σ exp(-λΔt)` 可 O(1) 增量维护。
→ **映射**：现实现 T0 最大的理论缺口就在这里——三年前的强关系永远显示强。T1 的 `sum(exp(-λΔt))` 是 Hawkes 的退化可解释版。但注意两个 Soul 特有约束：(a) **确定性**——衰减的"现在"不能读墙钟，必须用数据集内最大时间戳作 as_of，否则 C2 可测性死；(b) **可解释性**——连续指数权重（"有效次数 12.37"）用户无法数出来，必须量化成分桶半衰（近 90 天按 1 计、90–180 天按半计），用户才能用中文复核。`CANDIDATE_SPEC.md` 里 T1 按分桶版冻结。完整 Hawkes（MLE 估计 μ、α、β）被我直接排除：O(n²) 似然、参数不可解释、对 v0.1 是装饰（C6/C3 双杀）。

**RFM（Recency / Frequency / Monetary）**：直邮营销经典（**Hughes 1994**；**Bult & Wansbeek 1995** 最优直邮选择；**Fader, Hardie & Lee 2005** 把 RFM 接到 CLV 模型）。标准做法是对人群做分位数切分再打分。
→ **映射**：T2 的致命伤在这里暴露——分位数是**人群相对**的。第一，导入一个无关联系人会改变既有联系人的档位（违背 C2 确定性、违背「可向用户解释」——"他变弱了因为你认识了新人"荒谬）；第二，"你在前 1/3" 这种解释在语义上就是 percentile，而 PRODUCT_LOCK/D22 明令特质轴禁 percentile，人脉侧沿用同一词汇表（SupportedBand），打擦边球没有意义；第三，冷启动（<3 个联系人）分位数无定义。**固定阈值版 RFM 则退化成 T0/T1 的换皮**。所以 T2 在 Round 1 即建议击毙，理由记录在案。

**Dunbar 分层（Dunbar; Zhou, Sornette, Hill & Dunbar 2005; Sutcliffe, Dunbar, Binder & Arrow 2012）** — 人类社交圈按接触频率呈离散分层（~5/15/50/150），支持「档位」而非连续分数的产品选择。
→ **映射**：weak/moderate/strong 三档在理论上站得住：人对关系的心理表征本来就是分层的，不是连续分。这是对 D22「档不是分」在人脉侧的独立佐证。

**互惠**：**Gouldner (1960) 互惠规范**；有向网络互惠度量（Garlaschelli & Loffredo 2004）。单向大流量是广播/骚扰/营销号形态，不是关系。
→ **映射**：现实现的互惠门闩（不互惠封顶 Weak）理论正确，四个 T 候选全部保留该门闩，不作为差异点。

### 1.2 结论（tie 侧）

四个候选其实分布在两个正交维度上：**计数度量**（原始计数 / 衰减计数 / 分位数）×**门闩集**（互惠 / 跨日 / 私聊场合）。

| | 度量 | 门闩 | 理论锚点 | 主要缺口 |
|---|---|---|---|---|
| T0 | 原始 | 互惠+跨日 | Marsden-Campbell 频率代理 | 无近因；群聊场合盲 |
| T1 | 指数衰减 | 互惠 | Hawkes/Burt 衰减 | 无跨日/场合门闩；连续权重难解释 |
| T2 | 分位数 | 互惠 | RFM | 人群相对＝不确定、类 percentile |
| T3 | 原始 | 互惠+跨日+私聊 | Granovetter/Marsden-Campbell | 无近因 |

最优解显然是**T1 的度量 × T3 的门闩**的合成（本文档族记作 **T3R**，定义与消杀条件见 `CANDIDATE_SPEC.md`）。这不是发明新算法，是把两条已有谱系（Granovetter 结构门闩 + Burt/Hawkes 衰减）按 Soul 的可解释性约束量化拼装，C7 对齐、C8 需在 Round 2 用消融证明打赢 T3 单体。

## 2. 非临床特质工作模型（方向轴，不是大五分数）

**大五谱系**：Goldberg (1992) 词汇学大五标记；John & Srivastava (1999) 大五分类综述；Costa & McCrae 的 NEO 问卷传统。这些是**连续分数量表**——Soul 明确不做（D22）。
→ **映射**：Soul 的五条方向轴（`profile_axes.rs`）是「大五方向命名 + 三值方向（leans_low/mixed/leans_high/unknown）+ 证据档」。这个形态最近的亲戚是类型学（MBTI 式二分），而类型学挨过的批评（**Pittenger 1993**：把连续特质硬切两半、重测不稳定）恰好被 Soul 的三件套化解：(a) `mixed`/`unknown` 合法且是默认；(b) 位置是 working hypothesis 不是 type，带 falsifier；(c) 用户纠正锁定优先。**结论：方向轴形态在心理测量上是可辩护的，前提是永远不声称效度**——现实现的 `NotAClinicalClaim` 结构化标记和 `assert_non_clinical` 已经把这一点做成了类型系统的事，不是文案的事。这是 A0 的最大架构优点，任何替代方案必须保留。

**自我报告的证据权重**：自我报告有社会赞许性偏差（**Paulhus 1984**）；行为频次路线（**Buss & Craik 1983** act frequency approach）主张用可数行为佐证特质。
→ **映射**：现实现「问卷=Moderate、对着具体断言的纠正=Strong」的分层与文献方向一致：泛泛自述 < 对质具体断言。A1（多条独立证据可升档）的理论根据是**三角验证**（Campbell & Fiske 1959 multitrait-multimethod：多来源汇聚才算强）与 **Dawes (1979)**（简单计数规则稳健优于精巧加权）。A1 成立的前提是「独立」有可执行定义——同一份问卷一小时内填五遍不是五条独立证据。定义草案见 `CANDIDATE_SPEC.md`，能否钉死是 Round 2 的事，钉不死则 A1 归档。

**证据分档先例**：证据分级方法学（如 GRADE，Guyatt et al. 2008）的通用洞见是「结论强度 = 证据质量档位，而非点估计」。**仅作方法学类比，Soul 不做任何临床声明**。weak/moderate/strong 三档 + 每档可解引用证据，正是这个模式的非临床移植。

**从文本推断人格（A3 的理论背景，列出以便否决有据）**：LIWC（Pennebaker 词频词典；中文有 SC-LIWC/TextMind 移植）；**Yarkoni (2010)** 博客用词与大五相关；**Mairesse et al. (2007)** 文本人格识别；**Schwartz et al. (2013)** 开放词表；**Park et al. (2015)** 语言测评；**Kosinski, Stillwell & Graepel (2013), PNAS** Facebook likes 预测人格；**Youyou, Kosinski & Stillwell (2015), PNAS**。
→ **映射（否决理由，逐条对约束）**：(1) 需要正文——而 Soul 的图管道从 `InteractionRef` 开始就故意不携带正文，body 封存在 `body_ref`，采纳 A3 等于推翻整条数据面设计；(2) 相关强度在文献里是 r≈0.2–0.4 的量级，撑不起「强档」推断，只配 weak，投入产出差；(3) 中文词典效度需要独立验证，v0.1 无资源；(4) `emotional_steadiness` 轴 + 词频 = 临床化高危走廊（情绪词计数一步滑向筛查量表）；(5) 解释暴露文本挖掘（"因为你常说XX"），用户观感是监控不是复刻；(6) A0 的纠正锁已把用户话语权做成最高优先级，行为推断的边际价值被压缩。**判决：A3 在 Round 1 归档「不采用」，v0.2+ 若做需用户显式开启且走 E1。**

## 3. 无正文的统计式人事摘要

**元数据分析有效性**：Eagle-Pentland-Lazer 2009 与 Blondel, Decuyper & Krings (2015)（手机数据集分析综述）证明：次数、方向、场合、近因这四类元数据足以支撑"这段关系当前是什么形态"级别的陈述。
→ **映射**：A2 的五类句子（总量分布/方向比/场合/近因/档位归属）每一句都落在元数据可支撑范围内，没有一句需要正文，没有一句超出证据。方向比句式（"多数时候是你先开口"，阈值 2:1）是纯计数陈述，用户可自行数出。

**RFM 的正确用法**：RFM 在 A2 里以**绝对量描述**复活（"最近一次是 X 月 X 日""最近半年没有往来"），而不是以分位数打分复活。这是 T2 被否 与 A2 被留的边界线：同一理论，描述性用法合规，比较性打分违规。

**隐私侧**：Mayer et al. 2016 结论（元数据可重识别、可推断敏感事实）直接映射为 A2 的三条铁律：contact 只以 `contact_id`/`这个人` 出现、E1 改写只送 Soul 自产的计数句且过 redactor、`exportable_to_research=false`。现实现全部满足。

## 4. 约束映射总表

| Soul 硬约束 | tie 侧含义 | trait/人事侧含义 |
|---|---|---|
| 无证据不落库 | 边与推断只由 evidence 汇总产生；遗忘即重算/orphan | `SummaryPoint` 无证据无法构造；`AxisProposal` 空证据拒绝 |
| 禁分数/percentile/诊断词 | band 三词 + 原始计数；**T2 类 percentile 解释违规** | 方向 + band；`reject_numeric_rating` + `assert_non_clinical` |
| 第三人 local_only | 边 `EgressScope::LocalOnly`；conversation id 只存哈希 | 摘要不带姓名；E1 只送自产计数句 |
| 中文可解释 | 每档必须能写成"因为 N 次、D 天、有/无一对一" | 每句自带"依据 N 条记录" |
| 无 LLM | 全部候选纯规则 | A2 Counts 降级为主路径；A3 出局 |
| 确定性可测 | **禁读墙钟：as_of = 数据集最大时间戳**（T1/T3R 关键） | `now_unix_seconds` 已参数化，保持 |
| O(n) | 单遍 tally（现实现即如此）；T1/T3R 分桶 O(n)；完整 Hawkes 出局 | 摘要 O(边证据数) |
| 可 orphan | 重算即降档；无新增持久态 | 同 |

## 5. 排名（Round 1 立场，PROVISIONAL）

**Tie 族**：`T3R（T3 门闩 × T1 分桶衰减，合成）> T3 > T1 > T0（基线，预期被 T3 严格支配后归档）≫ T2（Round 1 即建议击毙）`

**Trait/人事族**：`A2（在位、AC-16/17 门禁所系，钉死保留）；A0（产品锁基底，不参与淘汰）> A1（有条件：独立性定义钉死才收，否则归档）≫ A3（Round 1 即建议击毙）`

**PROVISIONAL 保留对**：**T3R + A2**（A0 作为产品锁政策基底继续存在，不占算法名额——理由与"打架时单算法回退"见 `ACCEPTANCE.md`）。

评分矩阵与对抗夹具见 `EVAL_MATRIX.md`；公式、默认超参、中文解释话术、消杀条件见 `CANDIDATE_SPEC.md`；Round 2 待办见 `NOTES.md`。
