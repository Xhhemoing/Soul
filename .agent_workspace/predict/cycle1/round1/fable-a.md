# Cycle 1 / Round 1 / fable-a — 本机浅层行为预测 SOTA 审计

MODEL_SLUG: claude-fable-5-thinking-xhigh
角色：候选算法清单（供 v0.2 以后测试），**不是 v0.1 交付物，不进产品 crate，不实现 Goal 2**。
依据：`.agent_workspace/predict/CONSTRAINTS.md`、`docs/PRODUCT_LOCK.md`（研究层三轨道、v0.2 浅层行为预测、采集仅 exe+时长）、`docs/DECISIONS.md` D8 / D22 / D23。
多个候选并列「经确认可测」，不强选赢家（CONSTRAINTS 明文）。

---

## 0. 范围与红线回执

- D8：v0.1 只要 schema 与离线入口。本文件全部内容属「以后测试」，任何条目不得被解读为 v0.1 承诺。
- D23：采集面仅前台**可执行文件名 + 时长**（含起止时间）。无窗口标题、无键鼠、无 URL、无屏幕内容。凡依赖后者的算法直接进拒绝区。
- D22 / 非临床：禁量表分数、禁百分位。预测输出必须可向用户解释、可纠正。本文件把「可纠正」具体化为：用户可屏蔽某应用/删除某条规则/锁定纠正，且被屏蔽项永不再现（可测）。
- 研究轨道只吃 derived/aggregate；第三人行数恒 0；无单独同意不出本机；E0 无代码路径。联邦/云端协同过滤 v0.4 前不可作为默认。
- 无证据不得落库：任何预测在支持数（support）低于阈值时**不持久化、不展示**。审计只记「预测已渲染/已纠正」这类事件种类，无正文。

## 1. 数据面回执（候选只能吃这四样）

| # | 数据 | 形态 | 备注 |
|---|---|---|---|
| DP1 | 前台会话 | `AppIdentity`（如 `notepad.exe`）+ duration + 起止时间戳 | 唯一的密集事件流 |
| DP2 | 人脉边 | T4D 档、分列计数、`last_contact` / `as_of` | **已冻结**。只有聚合计数与最近接触时间，没有逐事件时间戳 |
| DP3 | 特质轴 | A0 方向 + 弱/中/强档，用户锁 | 见 §1.1 建议 |
| DP4 | 记忆/导入事件 | 有同意的 derived 计数 | 不是聊天原文；粒度最多到「某日某类事件 N 条」 |

### 1.1 对 A0 特质轴作为特征的建议（fable-a 立场）

数据面允许吃 DP3，但**建议 v0.2 预测器默认不把特质档当输入特征**：
1. 特质档是方向性推断，不是行为计数；混入后「可数解释」被污染（无法说出“因为过去 N 次里有 M 次”这样的句子）。
2. 用户锁定纠正特质后，预测若随之跳变，因果链对用户不可见，违反可纠正精神。
3. 若日后确要用，只允许作 UI 分组/文案语气选择，不作概率输入。此为建议非锁，留 Round 2 辩论。

## 2. 预测对象（predictand）菜单

浅层预测能诚实回答的问题只有这几类，全部候选按此对齐：

- **P1 下一应用**：切换应用时刻，预测下一个前台 exe（top-k）。
- **P2 时段应用集合**：给定未来时间桶（下一小时/明早），预测会出现的 exe 集合或首个 exe。
- **P3 会话时长档**：当前应用这次会用「短/中/长」哪一档（按用户自身历史分位切档；对用户渲染为分钟区间，不渲染为百分位词）。
- **P4 短时回开**：刚离开的应用在 T 分钟内被切回的概率（burst/自激）。
- **P5 节律偏移**：本周节律与自身基线的差异（只渲染为应用使用事实，**永不**渲染为情绪/精神状态推断——非临床红线）。
- **P6 人脉间隔**：某条边按历史平均联系间隔推算「已超期 X 天」（只用 DP2 冻结字段；本机展示；不出本机）。

## 3. 幸存者清单（经确认可测，并列不排名）

统一字段：预测对象 / 输入 / 输出 / 可数解释话术 / 日后证伪 / 与锁的冲突及缓解 / 文献锚点。

### S1 most-frequent-k 与近因加权频次（MFU / 指数衰减计数）

- **家族**：top-k 频次；变体为指数时间衰减计数（半衰期 τ 可调）与 most-recent-k。
- **预测对象**：P1、P2 的地板实现。
- **输入**：DP1 会话计数、最近使用时间。
- **输出**：top-k exe 列表 + 各自计数。
- **可数解释**：「过去 14 天你打开 `chrome.exe` 96 次，是最常用应用」。衰减版：「最近 7 天权重更高」——衰减本身用“最近 7 天 / 之前 7 天各多少次”两个计数来解释，不给用户看 τ。
- **日后证伪**：它是**地板基线**，自身几乎不可证伪；作用是证伪别人——任何更复杂候选在 hit@1 / hit@3 上打不赢 MFU（时间切分留出集，bootstrap 置信区间不重叠）即淘汰。
- **与锁的冲突**：无。纯计数，天然满足可数与可纠正（屏蔽某 exe 即从候选池剔除）。
- **文献锚点**：移动应用预测文献的标准基线（Yan et al., FALCON, MobiSys 2012；Shin et al., UbiComp 2012 均以 MFU/MRU 为对照）。

### S2 一阶 Markov 转移计数

- **预测对象**：P1。
- **输入**：DP1 会话化后的 exe 序列，相邻对 (A→B) 的转移计数表。会话化参数（间隔阈值、防 alt-tab 抖动去重）见 §5 共享预处理。
- **输出**：给定当前 exe，下一 exe 的 top-k + 转移计数/占比。
- **可数解释**：「在 `notepad.exe` 之后的 23 次切换里，你 15 次去了 `chrome.exe`」。占比即分数除法，可整句还原为两个计数。
- **日后证伪**：hit@1/hit@3/MRR 对 S1 的提升；prequential（在线逐条）评估防止一次性划分侥幸；低频行（转移出现 <5 次）不落库不展示（无证据不落库）。若对某用户数据 S2 ≤ S1，则该用户降级用 S1——按用户判定，不全局拍死。
- **与锁的冲突**：无。转移计数是 derived/aggregate，研究轨道可吃。
- **文献锚点**：一阶 Markov 是 APPM（Parate et al., UbiComp 2013）等工作的对照基线；Song et al., IEEE TMC 2006（Wi-Fi 移动性上 low-order Markov 常胜过高阶）。

### S3 n-gram（固定阶 + 回退/平滑）

- **家族**：2–4 gram，Katz 回退或 Kneser–Ney 平滑；本质是固定 (n−1) 阶 Markov 的工程化。
- **预测对象**：P1。
- **输入**：DP1 exe 序列的 n-gram 计数表（n ≤ 4，序列字母表 = 用户实际使用的 exe 集合，通常 <200 个符号）。
- **输出**：给定最近 n−1 个 exe，下一 exe 分布 top-k。
- **可数解释**：「最近先后用了 `code.exe`、`chrome.exe`，这个组合过去出现 9 次，其中 6 次下一个是 `slack.exe`」。**回退要如实渲染**：当高阶无计数时话术必须降为短前缀版本，不得假装高阶证据存在。平滑常数不对用户暴露，且展示阈值以**未平滑原始计数**为准（平滑只影响内部排序，不制造无证据的展示）。
- **日后证伪**：对 S2 的 hit@k 提升是否随 n 增大而衰减/反转（单用户数据量下高阶稀疏是主风险）；测「有效上下文长度」：n=2,3,4 逐档对比，无显著提升即钉死低阶。
- **与锁的冲突**：无直接冲突。注意点：平滑给未见转移分配概率，若 UI 把平滑概率当证据展示则违反「无证据不落库」——缓解如上（展示门槛用原始计数）。
- **文献锚点**：Kneser & Ney 1995（平滑）；Begleiter, El-Yaniv & Yona, JAIR 2004（与 VOMM 的系统对比框架，可直接复用其评测法）。

### S4 变阶 Markov 家族（PPM-C / PST / LZ 系 / Active LeZi / CPT+）

- **家族**：PPM（escape 回退）、概率后缀树 PST、LZ78/LZMS 预测器（Vitter & Krishnan 1996）、Active LeZi（Gopalratnam & Cook，智能家居动作预测）、CPT/CPT+（Gueniche et al., ADMA 2013 / PAKDD 2015，无损保存训练子序列、按实例解释）。
- **预测对象**：P1；CPT+ 亦可做 P2 的序列补全。
- **输入**：DP1 exe 序列；最大阶数钉死 ≤3（可解释性上限，见冲突栏）。
- **输出**：变长上下文匹配出的下一 exe top-k + 每级上下文的匹配计数。
- **可数解释**：可做到，但**必须把 escape/回退链渲染成分层计数**：「序列 […→A→B] 出现 9 次中 6 次接 C；若按更短的 […→B] 算，31 次中 14 次接 C」。CPT+ 的按实例解释最友好：能直接指认「与你 3 月 2 日、3 月 9 日的两段序列相似」——但指认历史片段要过 UI 隐私评审（片段本身仅含 exe 名，仍在采集面内）。
- **日后证伪**：与 S3 同框架对比（Begleiter et al. 的 prequential log-loss + hit@k）；重点证伪问题是「单用户 exe 流上，变阶比固定 2-gram 的增益是否超过解释成本」。若增益 <2 个百分点 hit@3，判定不值得，钉死 S2/S3。
- **与锁的冲突**：无数据面冲突。冲突在可解释性预算：PPM 的 escape 概率、PST 的剪枝阈值对用户不可见——缓解：对用户只渲染原始分层计数，模型内部机制不出现在 UI；最大阶数锁 3。
- **文献锚点**：Begleiter, El-Yaniv & Yona, JAIR 22, 2004；Parate et al., APPM, UbiComp 2013（PPM 用于无传感器上下文的 app 预测+预取，与 Soul 的「仅 exe+时长」约束最接近的前例）；Gueniche et al., CPT+，PAKDD 2015。

### S5 非齐次 Poisson 时段节律（hour-of-day × day-of-week）

- **预测对象**：P2、P5；给 P6 提供节律先验。
- **输入**：DP1 会话起始时间与时长，按（小时×星期几）分桶的出现计数/占用时长直方图；平滑仅限相邻桶合并（不引入不可解释的核）。
- **输出**：每（exe, 时段桶）的历史出现率；「下一小时可能用到的应用」列表；周节律画像。
- **可数解释**：「过去 4 周的工作日 21–22 点，你 20 天里有 18 天打开了 `game.exe`」。出现率即二项计数，是全清单里最干净的可数形态。
- **日后证伪**：留出后续 2 周，比较预测桶率 vs 实际（Brier 分数、log-loss）；平稳性检验（前后两个两周窗的桶分布 χ² 距离）——若节律本身不平稳，则该 predictand 对该用户整体不成立，降级为「仅描述、不预测」。
- **与锁的冲突**：一处高危：P5 节律偏移极易被文案写成情绪/作息健康推断，撞非临床红线。缓解：文案白名单只允许「应用×时段×计数」句式；禁「熬夜/焦虑/成瘾」等词进入渲染层（进 COPY 冻结流程）。另外时长档展示不得用「百分位」字样（D22 波及），渲染为分钟区间。
- **文献锚点**：Aledavood et al., PLoS ONE 2015（个体日节律的稳定性——支撑「按人建基线」而非全局基线）；Shin et al., UbiComp 2012（时间特征是 app 预测中最强的廉价特征之一）。

### S6 参数化 Hawkes / 经典 MTPP（自激点过程，非神经）

- **家族**：单指数核 Hawkes（每 exe 一条强度）、以 S5 的时段率为基线强度的非齐次 Hawkes；拟合用 MLE + Ogata 细化，全参数量 ≤3/应用。
- **预测对象**：P4（短时回开/burst）；P1 的时间敏感修正。
- **输入**：DP1 逐会话时间戳（这是唯一有逐事件时间戳的轨道，Hawkes **只许**在这条轨道上跑）。
- **输出**：「T 分钟内回开」概率；burst 时段标记。
- **可数解释**：**有真实缺口**。拟合出的强度函数本身不可数。缓解方案（本审计的硬性建议）：Hawkes 只作内部排序器，凡进 UI 的句子一律改写为等价的经验计数——「你离开 `chrome.exe` 后 10 分钟内切回的历史比例是 42%（120 次里 50 次）」。即：**展示层永远是经验频率，Hawkes 只决定何时值得展示**。若 Round 2 认定这构成「展示与依据脱节」，则 S6 降级为纯离线研究工具，UI 只用经验计数版（该版本独立成立，等价于 S5 的分钟级细化）。
- **日后证伪**：时间重标定理残差检验（Brown et al. 2002，KS 检验重标间隔是否指数分布）；held-out 周的下一事件时间 log-likelihood 对 S5（非齐次 Poisson）——**打不赢 S5 即整体淘汰**，这是明确的可证伪判据。
- **与锁的冲突**：(a) 可解释缺口如上，已给降级路径；(b) **人脉轨道不适用**：DP2 只有分列计数 + `last_contact`，无逐事件时间戳，无法拟合边级 Hawkes。P6 因此只许用间隔启发式：「你与 X 过去平均每 6 天联系一次（`as_of` 窗口内 N 次），现已 19 天」——纯 DP2 冻结字段除法，本机展示，第三人数据不出本机。任何「为拟合 Hawkes 而新增人脉事件时间戳采集」的提案都等于扩采集面，**本审计预先否决**。
- **文献锚点**：Hawkes, Biometrika 1971；Brown et al., Neural Computation 2002（time-rescaling）；通信/交互事件的自激建模（Masuda et al. 综述脉络）。神经版见拒绝区 R3。

### S7 序列规则挖掘（RuleGrowth / ERMiner / TRuleGrowth 窗口版）

- **预测对象**：P1、P2；以及跨源弱规则（DP1×DP4，见冲突栏）。
- **输入**：DP1 会话化 exe 序列切成天/半天窗；minsup、minconf 阈值；TRuleGrowth 的窗口约束（如「同一 30 分钟内」）。可选：DP4 的按日 derived 计数作为规则左件（仅在该来源同意开启时）。
- **输出**：规则集 `{A,B} ⇒ C`（窗口内），每条带 support（出现次数）与 confidence（占比）。
- **可数解释**：规则天生就是计数对：「30 分钟内先后用过 `photoshop.exe` 和 `explorer.exe` 的 17 次里，13 次接着打开了 `wechat.exe`」。可纠正性最好：用户可逐条删除/禁用规则，禁用规则进屏蔽表，重挖掘时被过滤（可测：禁用后永不再现）。
- **日后证伪**：规则按挖掘期/验证期两段时间评估，验证期 confidence 相对挖掘期的衰减 >30% 判过拟合；规则跨周稳定性（连续两个两周窗都被挖出才可展示）；整体作为预测器与 S2 比 hit@k 与覆盖率的权衡曲线。
- **与锁的冲突**：(a) 规则文本若含人脉边引用（如「与某人联系后」），因 DP2 无逐事件时间戳，实际上挖不出来——天然规避；(b) DP4 参与的规则（如「导入事件多的当天你常开 X」）必须逐来源检查同意位，且左件只写事件类别与计数、不写内容；(c) 规则爆炸风险：minsup 阈值即「无证据不落库」的实现位，Round 2 需钉具体数值（建议起点 support ≥5 且 confidence ≥0.6）。
- **文献锚点**：Fournier-Viger et al., RuleGrowth（SAC 2011）、ERMiner（IDA 2014）、TRuleGrowth（窗口约束版）；SPMF 库为参考实现（Java，仅作离线评测对照，不进产品依赖）。

## 4. 拒绝 / 推迟清单

| # | 候选 | 判定 | 依据（锁条文级） |
|---|---|---|---|
| R1 | 联邦 SeqMF / 联邦序列感知矩阵分解（Gboard 式联邦训练脉络，Hard et al. 2018） | **推迟 ≥v0.4，且届时也非默认** | 锁：联邦学习 v0.4 前不可作为默认，需授权非 E1 云；v0.1/v0.2 E0 无代码路径，联邦聚合服务器无处安放。另：MF 隐因子不可数，即便本地版也过不了可解释门 |
| R2 | 云端协同过滤（跨用户 CF，Natarajan et al., RecSys 2013 交互上下文 CF） | **拒绝**（非推迟） | 需要跨用户数据池，违反「第三人/本人数据不出本机」与本地优先；且「像你的用户还用了 X」不是自身行为计数，对单人复刻产品语义错位 |
| R3 | 深度 ATPP / 神经点过程（Neural Hawkes, Mei & Eisner 2017；Transformer Hawkes, Zuo et al. 2020；SAHP；AppUsage2Vec, ICDE 2019；DeepApp 类） | **推迟，无明确回收版本** | 不可数、不可纠正（黑盒违反「可向用户解释」硬线）；单用户 exe 流数据量撑不起；重依赖。回收前提：出现逐预测忠实解释机制 + S4/S6 被证明显著不足，两者都未发生前不进测试队列 |
| R4 | GNN / 时序图网络在人脉图上做交互预测（TGN, Rossi et al. 2020；TGAT） | **拒绝** | DP2 已冻结且无逐事件时间戳，输入面根本不存在；黑盒不可纠正；人脉边推断必须带证据档（锁），GNN 嵌入给不出 |
| R5 | 一切需要窗口标题、URL、文档名、键鼠、剪贴板、截屏的预测（任务级上下文预测、浏览器标签预测、smart-reply 行为模型、按键动力学） | **拒绝**（不做，非推迟） | PRODUCT_LOCK 砍/留表：窗口标题「不做」；无键盘记录、无密码框；D23 采集面仅前台应用时长。扩面提案不属于本调研职权 |
| R6 | LLM 读正文做行为分类/预测（含读文件正文、聊天原文） | **拒绝** | CONSTRAINTS 明文：读正文/LLM 分类与「不读第三人正文默认出网」冲突，默认禁；研究轨道只吃 derived/aggregate |

## 5. 共享证伪协议（v0.2 测试期统一执行，防各候选自说自话）

1. **共享预处理钉死**：会话化间隔阈值、alt-tab 抖动合并窗（建议 <5 秒的回切合并）、exe 名规范化（小写、去路径、版本号折叠）——Round 2 出具体数值，全候选共用，否则对比无效。
2. **时间切分**：只许前训后测（prequential 或按周滚动），禁止随机打乱切分（序列数据随机切分=泄漏）。
3. **地板与天花板**：S1 是地板；「重复上一个应用」是第二地板；打不赢地板的候选按用户降级。天花板用同用户历史的经验上限（bayes error 的粗估）标注，防止对不可预测用户空耗。
4. **指标**：P1 用 hit@1/hit@3/MRR；P2/P4 用 Brier + log-loss；S6 加时间重标 KS。显著性用分层 bootstrap（按周分块）。
5. **落库门**：任何展示句必须能还原为「N 次里 M 次」且 N ≥ 阈值（建议 5 起步）；低于门槛的预测不持久化、不展示（无证据不落库的预测版）。
6. **纠正回路可测**：屏蔽 exe / 禁用规则 / 用户锁定后，被屏蔽项在后续任何预测输出中出现次数恒 0（与「未同意事件数为 0」同款验收句式）。
7. **审计**：只记事件种类（预测渲染、纠正、屏蔽），无预测正文，无 exe 名进审计正文——与「审计无正文」对齐（exe 名是否算正文留 Round 2 裁决，本审计倾向从严：不进）。
8. **隐私不变量**：全部特征可由 DP1–DP4 重放推导；研究导出（v0.2 起落盘）中第三人行数恒 0 的断言必须覆盖预测中间表。

## 6. 与锁的冲突总表（速查）

| 候选 | 冲突点 | 状态 |
|---|---|---|
| S1 | 无 | 干净 |
| S2 | 无 | 干净 |
| S3 | 平滑概率若当证据展示则违「无证据不落库」 | 已给缓解（展示门槛用原始计数） |
| S4 | escape/剪枝机制不可见 | 已给缓解（UI 只渲染分层计数；阶数锁 3） |
| S5 | 节律文案易滑向临床暗示；「百分位」字样 | 已给缓解（文案白名单 + 分钟区间渲染） |
| S6 | 强度函数不可数；人脉轨道无输入 | 已给降级路径（展示层只用经验频率；人脉侧只用间隔启发式）|
| S7 | DP4 规则的同意位；规则爆炸 vs 落库门 | 已给缓解（逐来源同意检查；minsup 即落库门） |

## 7. 留给 Round 2 / Round 3 的开放问题

1. 会话化参数与 exe 规范化规则的具体数值（§5.1），需要用真实 fixture 分布定。
2. 「exe 名是否算审计正文」的裁决（§5.7）。
3. A0 特质档作为特征的取舍（§1.1 本审计建议默认排除，需对手方意见）。
4. S6 的「展示与依据脱节」质疑是否成立——若成立，Hawkes 整体降级为离线研究工具。
5. P3（会话时长档）没有专属候选：S5 的桶直方图即可覆盖，还是值得引入简单生存分析（Kaplan–Meier 是可数的：'过去 40 次里 25 次超过 10 分钟'）？留 Round 2。
6. 冷启动：导入问卷/Telegram derived 计数能否给 S1 提供先验，还是前两周干脆不预测（本审计倾向后者：宁可沉默，不可编造）。

## 参考文献锚点（仅作检索索引，非引用清单）

- Begleiter, El-Yaniv, Yona. On Prediction Using Variable Order Markov Models. JAIR 22, 2004.
- Parate, Böhmer, Chu, Ganesan, Marlin. APPM: Practical Prediction and Prefetch for Faster Access to Applications on Mobile Phones. UbiComp 2013.
- Yan, Chu, Ganesan, Kansal, Liu. FALCON: Fast App Launching for Mobile Devices Using Predictive User Context. MobiSys 2012.
- Shin, Hong, Dey. Understanding and Prediction of Mobile Application Usage for Smartphones. UbiComp 2012.
- Gueniche, Fournier-Viger, Tseng. CPT / CPT+. ADMA 2013 / PAKDD 2015.
- Fournier-Viger et al. RuleGrowth (SAC 2011), ERMiner (IDA 2014), TRuleGrowth; SPMF 库.
- Vitter, Krishnan. Optimal Prefetching via Data Compression. JACM 1996.
- Gopalratnam, Cook. Active LeZi. (MavHome 智能家居动作预测脉络.)
- Song, Kotz, Jain, He. Evaluating Next-Cell Predictors with Extensive Wi-Fi Mobility Data. IEEE TMC 2006.
- Hawkes. Spectra of Some Self-Exciting and Mutually Exciting Point Processes. Biometrika 1971.
- Brown et al. The Time-Rescaling Theorem. Neural Computation 2002.
- Aledavood et al. Daily Rhythms in Mobile Telephone Communication. PLoS ONE 2015.
- Kneser, Ney. Improved Backing-off for M-gram Language Modeling. ICASSP 1995.
- 拒绝区索引：Mei & Eisner (NeurIPS 2017), Zuo et al. (ICML 2020), Natarajan et al. (RecSys 2013), Rossi et al. TGN (2020), Hard et al. 联邦 Gboard (2018), AppUsage2Vec (ICDE 2019).
