MODEL_SLUG: claude-fable-5-thinking-xhigh

# Cycle 1 / Round 3 / fable-b — 测试日验收清单（acceptance checklist）

定位：给日后**实现并运行**这些测试的人的逐项验收单。本文件不新增裁决、不改常量、不实现 crate；每项都挂已登记出处（括号内锚点），与出处冲突时以 `R2-SYNTHESIS` 为准，发现清单与已登记裁决矛盾属于**清单 bug**——修清单，不修裁决。

**判定值纪律**：每个勾选项只有四种结果——`PASS` / `FAIL` / `BLOCKED(Gxx)`（被 §L5 的缺口阻塞，允许跑但结果不受理）/ `N-A`（该候选已墓碑或该项属上线后指标）。`inconclusive` 一律记 `FAIL`（R2/opus-a §4.3：没数据 ⇒ 没失败 ⇒ 可以上，是必须提前堵死的捷径）。

**分层纪律**：L0（harness）→ L1（夹具）→ L2（不变式）→ L3（候选杀线）→ L4（报告与呈现）。**低层未全绿，高层的一切命中率数字可以跑、不可采信**；L5 是缺口映射，决定哪些项标 `BLOCKED` 而非 `FAIL`。

---

## 目录快照（本清单覆盖的受验对象）

| 对象 | 轨道 | 主杀线所在 |
|---|---|---|
| P0 `p0.marginal_top_k.v1` | 对照地板（next-app） | D-P0 |
| P1 `p1.markov1_next_exe.v1` | 默认轨候选 | D-P1 |
| P2 家族（共享后缀 trie，阶 0–3；`p2@2`=R1 的 v1 配置，`p2@3`/自适应选阶为对照配置，merge-by-outcome 收编） | 默认轨恒只剩一个 n-gram 幸存者 | D-P2 |
| P3 `p3.hourbin_top.v1`（UTC 小时，诚实标注） | 默认轨候选（boundary 格） | D-P3 |
| P4-gate `p4a`/`p4b`（整数突发窗口） | **对照**（R2-SYNTHESIS：非 v0.2 默认） | D-P4 |
| P4-fit（参数化 Hawkes）、经验 TTU CDF | 离线「何时」对照工具 | D-P4 |
| C0/C1/C2/C3 | 「何时」任务对照阶梯 | D-C |
| P5 `p5.contact_due_reminder.v1` | 规则（非学习器） | D-P5 |
| P6 `p6.lock_gate.v1` | 强制门 | D-P6 |
| P7 `p7.deterministic_arbiter.v1`（备选 `p7b` Borda、`p7c` 整数成对胜场；两者互斥启用） | 融合层 | D-P7 |
| P8 `p8.dwell_hazard_bucket.v1` | 打扰门 | D-P8 |

出局项（验收时任何报告出现即整份退回）：加权插值混合（复活须满足 R2/opus-b §5 三条）、预取/预启动（产品锁）、`appm` 字样（R2/opus-b §6.1）、任何浮点权重与除法判定（R1/opus-b §0.3–0.4）。

---

## L0 Harness 有效性（A 系列）——任何候选数字被受理的前提

- [ ] **A1 prequential 顺序**：每个评测机会严格 freeze(predict) → 揭晓 → 记账 → 恰好一次 update；绝不从全量日志预建计数表或候选词表（R2/gpt-sol-b §2）。
- [ ] **A2 段切分**：输入按同意状态、锁屏/来源错误等未知区间切段；跨段不构成转移；每段首事件只入状态不计机会；资格集合在运行任何模型前一次性冻结，所有模型同一批机会（R2/gpt-sol-b §2）。
- [ ] **A3 空状态起步**：每次重放从空状态开始；shuffle 的每个种子间状态彻底清零，无复用（R2/gpt-sol-b §4）。
- [ ] **A4 `as_of` 纪律**：判定为纯函数 `(counts, context, as_of) -> tokens`；全链路禁读墙钟；`as_of` 整体平移后输出不变（R1/opus-b §0.4.3、F3-2/F4-4）。
- [ ] **A5 确定性**：同输入重复 1000 次逐字节同输出，含全部并列格与 fallback 格；并列一律 `app_id` 字典序升序（F0-3/F7-4/F7-10）。
- [ ] **A6 整数判定**：所有过/不过比较用交叉相乘的精确整数（四元乘积用 i128）；浮点只允许出现在自助置信区间内部，最终判定必须精确有理（R2/opus-a §4.3）。
- [ ] **A7 空虚保护**：任一判据分母 `n < 200` 或分子 `a < 20` → `inconclusive` → 记 FAIL（R2/opus-a §4.3）。
- [ ] **A8 常量预注册**：`MIN_ROW_SUPPORT_3 ∈ {8,10,12}` 扫档、P4 半衰期 `{30min,1h,4h}` 三档、shuffle 种子表与容差 ε、全部支持阈值与并列规则——**在查看对应日志结果之前**登记并带登记时间；事后倒填=该项 FAIL 且相关结果作废（R2/opus-b §2.1、R1/opus-b §2.5、R2/gpt-sol-b §4）。
- [ ] **A9 指标恒等式**：只有 top-1 / top-3 / 弃权率三项；弃权按未命中计且留在分母；`0 ≤ top1_hits ≤ top3_hits ≤ N − abstentions` 每份报告断言成立；先给整数分子/分母再给派生小数（R2/gpt-sol-b §3）。
- [ ] **A10 记录形状**：每条离线记录含 `protocol_version / trace_fixture_id / model_id / eligible_count / top1_hits / top3_hits / abstention_count / 三个 rate / shuffle_seed_or_none`，仅存于测试目录或 CI artifact（R2/gpt-sol-b §3）。
- [ ] **A11a 泄漏探针（shuffle）**：先封存原顺序结果；逐段打乱 app 身份、保段边界/事件数/时间戳；≥32 个固定种子，`ε = max(0.01, 1/N)`；P1−P0 的三个配对差的 95% 种子区间均落在 `[−ε,+ε]`。失败按 R2/gpt-sol-b §4 的六条排查序处理（先修 harness，不归罪算法）。
- [ ] **A11b 泄漏探针（前缀因果，零容差）**：任意两条日志截至预测点前缀相同 ⇒ 该点候选、弃权、支持计数逐字节相同，与后缀无关（R2/gpt-sol-b §4 末）。
- [ ] **A12 对照可比性**：P0/P1（以及 P2 家族各配置）使用同一候选集、并列规则、backoff 规则与支持门；否则 A11a 混入支持门差异，判无效（R2/gpt-sol-b §1）。
- [ ] **A13 「何时」任务时点栅格**：固定 tick 栅格（与结果无关）、暖机跳过 W 个 tick、右删失时点弃掉不入分母、标签 `Y(t)=1 iff [t, t+900s) 内出现规范化段起始`（R2/opus-a §4.0）。禁止「在每次会话起始时评测」。

---

## L1 夹具正确性（B 系列）——oracle 逐项对表

通用：每个夹具 fresh store + 已授收集同意 + 独立模型状态；trace/夹具边界不跨；持久会话一律经 `body_ref` 解封核对，**不得**从明文列推断 exe 身份（R2/gpt-sol-a 序言）。

- [ ] **B1 SYN-MARKOV**：解封序列与 13 步脚本逐条相符；有向计数恰为 `alpha→beta=4, alpha→gamma=2, beta→alpha=4, gamma→alpha=2`，无其他边、无 `alpha→alpha`；末 `alpha` 后 P1 答 `beta`（6 中 4）而 MFU 答 `alpha`——这格是 P1 与 P0 的判别格；五条 Failure conditions 各有一条**负例测试**证明会被抓（R2/gpt-sol-a §SYN-MARKOV）。
- [ ] **B2 SYN-RHYTHM**：直方图恰为 `(alpha,08)=4`、`(beta,20)=4`，交叉格全零；边际相等（频率选不出）；转移 oracle 为空（八条独立 trace）；未见小时按预先声明的弃权/回退规则，不造支持；桶名诚实标 UTC（R2/gpt-sol-a §SYN-RHYTHM）。
- [ ] **B3 SYN-NOISE**：去抖/大小写合并后 N1 只有一个 `alpha`；转移 oracle 仅 `alpha→beta=1`、`zeta→eta=1`（不跨 clear、不跨来源错误、不跨 trace）；N4 按插入/会话结束序保持 `zeta→eta`，仅按 `ts` 排序会反转——必须按前者；合法零时长不拒绝；N3 的「store-only 重建无法诚实区分」限制原样承认（R2/gpt-sol-a §SYN-NOISE）。
- [ ] **B4 ADV-DST**：生产等价输入下只报 `(alpha, UTC 13)=3`、`(alpha, UTC 14)=3` 且显式标 UTC；**两个 must-fail 模式真的 FAIL**——固定 UTC−04 换算（D6–D8 变 10:00）与 UTC 冒充本地小时；隐藏 New York 列永不进训练输入（R2/gpt-sol-a §ADV-DST）。
- [ ] **B5 「何时」夹具族**：`SYN-M1P / SYN-M1H / SYN-I0H / SYN-M1D / SYN-APPTIME / ADV-uptime / ADV-flood / ADV-adjacency` 各自实现 R2/opus-a §4.2 表中「谁该赢 / 谁该测不出」两列，作为夹具自身的验收 oracle。特别地：`SYN-M1P` 上 P4/C1/C3 全部测不出、P1 逼近贝叶斯上界；报告中出现「P4 在 SYN-M1P 上赢了 P1」= harness 红灯，不是战绩（R2/opus-a §3）。
- [ ] **B6 保节律零假设**：K5 用非齐次 Poisson 自助（按经验 `(工作日/周末, UTC 小时)` 强度重采样，种子固定，200 次），**不是**日内均匀重排（R2/opus-a §4.4 注）。
- [ ] **B7 F5 专属夹具**：`group_only_50`、`dormant_2019`、`X2 direct_strong_group_lifeline`、`lilei_12` 四件齐备，且 `lilei_12` 的期望值**先手算写死后实现**（R1/opus-b §2.6 F5-4）。

---

## L2 不变式（C 系列）

- [ ] **C1 F（强式）前置断言**：所有 F 类测试的夹具按容量上限设计并显式断言 `evictions == 0`（逐深度 `evicted_prefixes[depth] == 0`）；缺该断言的 F 测试判**假绿灯**，整项 FAIL（R2-SYNTHESIS；R2/opus-b §6.2）。此条同样覆盖 P1 的 `TOPK_ROW` other 桶。
- [ ] **C2 删除 = 有界局部重算**：`decrement(state, e, as_of)` 在 `e` 所在 run ∪ 前后紧邻 run（≤3 个）上重跑规范化取增量，**不是单桶减一**；`ADV-adjacency` 覆盖拆段、并段、跨 `SESSION_GAP_S` 边界生灭三形状；外加固定种子 10⁴ 组 `(夹具, 事件)` 属性测试 `local_decrement(replay(E), e) == replay(E\{e})` 逐桶相等（R2/opus-a §6 F-1、引理三）。**F 测试集不含 `ADV-adjacency` 即整层无效**。
- [ ] **C3 F 对 `as_of` 参数化**：`∀ as_of` 成立；窗口外事件的删除是**有类型的 no-op**（不悄悄取模）；窗口边界事件（`tick(e) == tick(as_of) − W`）两侧与全量重放一致；先删后加 == 先加后删（F4-3b/F4-3c）。
- [ ] **C4 F₂（弱式，驱逐后）**：被遗忘事件的增量已从所有存活桶减去、被驱逐行无残留；`evictions > 0` 后恢复精确性的唯一路径是事件表全量重建，重建等价性有测试（R2/opus-b §6.2）。
- [ ] **C5 F′（遗忘覆盖）**：候选消费的每个输入字段遗忘后被删除或不可读。`F′-1`（销毁 key 后 P4 输出与「从未采集」重放不可区分）对 `p4a` **预期红**（G16），红灯保留——为让它绿而弱化断言 = 本项 FAIL（R2/opus-a §6 F-3）。
- [ ] **C6 p7c 遗忘清零**：任一事件被遗忘 ⇒ 整个胜场窗口清零、退回固定阶梯；不做 checkpoint 局部重放（R2/opus-b §3.3）。
- [ ] **C7 有损只在读路径**：写路径状态精确可逆，读路径才许 `CAP=63` 截断；发现写时饱和加法/老桶归并即 FAIL（推广到所有候选，R2/opus-a §5 D5）。等价性单测：`W=72` 与 `W=288` 在全部夹具上 `excite` 逐位相等。
- [ ] **C8 P4 无持久状态**：环不落库，每次求值从 `[as_of − 6h − 1800s, as_of)` 现算（LOOKBACK 前置读窗防切错合并 run）；F 对 P4 构造性成立（R2/opus-a §6 F-3 决策 (a)）。

---

## L3 候选杀线受理条件（D 系列）

通用门（每个候选先过，过不了不比命中率）：确定性、`as_of` 平移不变、空日志→弃权、未同意→事件 0→`abstain.consent_off`（不许拿旧计数续命）、不变式 F（按 L2 口径）、P6 门通过（R1/opus-b §0.6）。

### D-P0
- [ ] F0-2：`ADV-single_app` top-1 = 100%。 F0-3：tie 稳定 1000 次。
- [ ] F0-1 的适用范围**修正后执行**：只约束 next-app 候选（P0–P3、P7）；P4/P8 不适用（样本空间不同，R2/opus-a §3）。

### D-P1
- [ ] F1-1 主杀线：prequential top-1 ≥ P0 + 10 点，在 ≥2000 次转移、≥3 个独立日志上同时成立。
- [ ] F1-3：SYN-MARKOV/`SYN-markov1` 上距贝叶斯上界 ≤ 5 点。 F1-5：`ADV-flood` 不打爆状态、`MIN_SESSION_S` 挡掉的比例如实登记。
- [ ] F1-2 由 A11a 统一执行（P1 退化到 P0 ± 2 点）。

### D-P2（家族，共享后缀 trie）
- [ ] F2-5 合并回归钉：`p2@2` 在 R1 全部夹具上与 `p2.ngram2_backoff.v1` **逐字节相同**——合并不得改变现状配置任何 token。
- [ ] F2-1 逐配置：`p2@k` top-1 ≥ P1 + 3 点 **且** 条目总数 ≤ P1 × 8；独立晋级，不许打包。
- [ ] Δ1（升阶）/ Δ2（整数自适应选阶）各自的「赢」= 对 P2 的 prequential hit@3 ≥ +2 点，且 REAL 夹具上不输（仅合成歧义夹具赢不算数）；`order_used=3` 占比 < 5% ⇒ Δ1 判死重不收编（R2/fable-a §2.3）。
- [ ] F2-6a：`SYN-order3` 上 `p2@3` ≥ `p2@2` + 2 点，其余夹具不低于 `p2@2` 超过 1 点。 F2-6b：阈值三档扫全登记后写死。
- [ ] F2-2 留出末 20% 不退化；F2-3 一阶源上 `p2@2 ≈ p2@3 ≈ P1`（≤1 点）；F2-4 `ADV-new_app` 下 `order_used` 8 次内爬到 2、24 次内爬到 3；F2-7 `app>60` 档 depth-3 驱逐率 > 20% ⇒ 该深度结论作废；F2-8 shuffle 上退化到 P0 ± 2 点。
- [ ] merge-by-outcome 收编按 R2/fable-a §2.2 表执行；Cycle 末默认轨恒只剩**一个** n-gram 幸存者。

### D-P3
- [ ] F3-1 主杀线：无上下文格（开机后/boundary 后第一段）top-1 ≥ P0 + 8 点。
- [ ] F3-2 整体平移 +6h 命中率 ±1 点；F3-3 `ADV-DST`/`ADV-tz_change` 不 panic、无负桶号、跌幅如实登记；F3-4 168 桶仅当 ≥6 个月夹具上赢 ≥3 点才启用。
- [ ] 桶一律 UTC 小时并诚实标注；任何「本地上午/morning」措辞出现即 FAIL（G2；B4 的 must-fail 模式）。
- [ ] 耦合登记：P2 升三阶抬高 boundary 弃权率 ⇒ F3-1 的权重上升，报告必须写出这条耦合（R2/opus-b §2.6 末、§7.5）。

### D-P4（对照位；「何时」任务）
前置：负对照先过，正面成绩才受理（R2/opus-a §3）。
- [ ] K1：`SYN-M1P` 上 hot/cold 区分度不成立（1.2 倍整数判据）且自助 CI 覆盖零。
- [ ] K2 分层：接上 P4 前后全部 `next_app` 输出逐字节相同；`p7.*` 源码零引用 `p4.*`；`D1-guard`：默认打扰路径对 `p4.*` 引用数为 0。
- [ ] K3：区分度 ≥ 1.25 × C1（盒核）。 K4：≥ 1.25 × C3（app 条件基线；「vs P1」的正解）。 K5：`SYN-M1D` 上不成立 + REAL 保节律零假设下仍成立且 ≥1.25。 K6：`ADV-uptime` 表观区分度低于 1.2 倍阈值。 K7：`excite ≤ 1440`（u16）、`HOT` 不恒真。 K8 = C2/C3。
- [ ] D4 修正落实：`excite(as_of)` 只求和已闭合 tick；单测——`as_of` 后紧插一次起始，`excite` 逐字节不变。
- [ ] `p4a`/`p4b` 分别登记禁串号；hot 时点集合对称差 > 10% 而报告只给一个数 ⇒ 报告退回；`p4a` 一律命名「事件密度」。
- [ ] `REAL-*` 数字在 G15 补上前一律标 `inadmissible`（可跑可看，不作晋级依据）→ 记 `BLOCKED(G15)`。
- [ ] P4-fit / 经验 TTU CDF / P3-Poisson 只出现在离线「何时」对照表：P4-fit 的 held-out 时间 NLL/MAE 输给经验 TTU CDF 或 P3-Poisson ⇒ 整体墓碑；时间重标 KS 残差登记（R2/fable-a §1.2）。
- [ ] 晋级（脱离对照位）四条全查：G15 落地、K6 过、K3+K4 过、全局打扰预算存在——缺一即维持对照位（R2/opus-a §2 翻案条件）。

### D-C（对照阶梯自身）
- [ ] C0/C1/C2/C3 按 R2/opus-a §4.1 实现；C3 只在 P1 已有行上挂两个整数、不加新状态面；C2 诚实标 UTC。「P4 输给 C3」按其含义登记（环形缓冲冗余于 P1 一张表的两个字段）。

### D-P5
- [ ] F5-1 `group_only_50` → `reminder.none`；F5-2 `dormant_2019` → none（Weak 不刨坟）；F5-3 X2 → 必须发且带 `only_group_recently`（钉 direct 时钟）；F5-4 `lilei_12` 期望先写死；F5-5 边锁 Weak → `suppressed_edge_locked`，不因 `machine_band=Strong` 漏网；F5-7 200 人全到期日输出 ≤ 3 条（交叉相乘排序，无除法）。
- [ ] F5-6（忽略/关闭率 ≥70% 出局）为上线后指标：测试日记 `N-A` 并登记为发布门，不许拿夹具数字冒充。

### D-P6
- [ ] F6-1 五轴全锁 → 轴向输出为空；**F6-2 同一夹具行为 token 照常输出**（双向，同等重要）；F6-3 predict crate 对 `trait_axis.` 全量 grep 为 0（排除测试自身反向断言处）；F6-4 proposal 未确认前档案逐字节不变；F6-5 锁一轴不冻结他轴。
- [ ] 轴亲和登记表核对：P3/P8→orderliness，P4→emotional_steadiness（连 proposal 都不许 + 禁词表），P5→social_energy/accommodation；新候选无亲和登记不受理。

### D-P7
- [ ] 输入集恒 {P0, P1, P2, P3}（无 P4）。F7-1：top-1 ≥ 全部单源最大值，且每个夹具不差超过 2 点。F7-3：`source` 分布随夹具变化；从不被选中的源删除。F7-4 确定性。
- [ ] F7-9 决策规则单源：P2 独立 top-1 与 P7 限制在 `p2@*` 上的 top-1 全夹具逐字节相同。
- [ ] `p7c`：F7-5 ≥ 固定阶梯 + 2 点；F7-6 `SYN-random` 上选层接近均匀/频繁 `cycle_fallback` 且 top-1 ≈ P0；F7-7 「每 100 事件遗忘 1 条」夹具上 `order_rule(fixed)` 占比 > 80% ⇒ 出局；F7-8 单事件扰动后 1000 决策点 top-1 翻转比例 ≤ 固定阶梯 × 3；`cycle_fallback` 出现率必报。
- [ ] `p7b` 与 `p7c` 不许同时启用（R2/opus-b §7.3）。

### D-P8
- [ ] F8-1 主杀线：打赢「该 app 历史时长中位数」常数基线。F8-2 `ADV-flood` 下不得全体 `likely_ends`。F8-3 接 P5 后：`likely_continues` 期间的提醒 ↓≥50% 且提醒总量 ↓≤20%。F8-4 F（`survives_past` 多桶累加可精确减回）。

---

## L4 报告与呈现纪律（E 系列）——贯穿全程

- [ ] **E1 两张表**：`next_app`（P0/P1/P2 家族/P3/P7）与「何时」（C0–C3/P4/TTU/P8）分表出；混表退回（R2/opus-a §3）。
- [ ] **E2 逐配置行**：P2 家族每配置一行，`order_used` 分布、条目总数、驱逐次数三列齐备；缺列该行作废（R2/opus-b §1.3）。
- [ ] **E3 配对与全量**：P0/P1 及各配置对比逐同一机会配对；保留逐日志整数与合并整数；禁挑最好的日志/种子/后半段（R2/gpt-sol-b §3）。
- [ ] **E4 命名纪律**：全部文档/算法 id/指标表零 `appm` 字样；`p4a` 标「事件密度」；小时桶标 UTC；P4 输出不比对「你平时这个点」（D6 禁语句检查）。
- [ ] **E5 admissibility 标注**：受 G15 影响的 `REAL-*` 行（P4、C1、C2）显式标 `inadmissible`；无标注的此类行退回。
- [ ] **E6 上屏隔离**：命中率、差值、CI、算法名只存在于本机离线评测产物；不进产品 schema、审计正文、UI；禁「准确率 78%」「置信度 82」「表现强/中/弱」、星级、综合指数（R2/gpt-sol-b §5）。产品若展示建议，只许「可纠正暂定建议 + 整数证据句」，低支持用无数字刻度的中性说明或静默。
- [ ] **E7 P4 无用户可见 token**：默认轨不渲染任何 burst 字样；审计 shadow 记录只含事实与 axis_id，无正文。
- [ ] **E8 预测不落库**：测试期一切预测均为离线 artifact；在 `EvidenceKind::AppUsage` 缺口补上之前，任何写入 `SoulInference` 的路径出现即 FAIL（R1-SYNTHESIS 数据面事实）。

---

## L5 测试日前置缺口 → BLOCKED 映射（G 系列）

| 缺口 | 内容 | 阻塞的验收项 | 测试日处理 |
|---|---|---|---|
| G15 | 无可被遗忘的采集在线信号（`sessions_discarded` 不落库） | D-P4 的 `REAL-*` 受理、P4 晋级路径第 1 条；波及 C1/C2 的 REAL 行 | 相关行标 `BLOCKED(G15)` + `inadmissible`；SYN/ADV 行照常受理 |
| G16 | 遗忘不移除明文 `ts` | C5 的 `F′-1`（对 `p4a`） | 红灯保留并登记；禁弱化断言 |
| G2 | 无本机时区偏移 | D-P3 的任何本地小时措辞；ADV-DST 假设模式 | 只跑 UTC 模式；本地模式记 `N-A` |
| 全局打扰预算缺失 | 无「今天已打扰几次」总账 | P4 晋级路径第 4 条；三门组合评测 | 只受理 P8+P5 两门组合（F8-3）；三门组合不开测 |
| 重建时机未裁 | `evictions>0` 后全量重建的触发时机 | C4 的自动化部分 | 重建作为手动步骤验收等价性；时机留后续轮 |
| `EvidenceKind::AppUsage` 缺构造点 | 预测无法合法落 `SoulInference` | E8 | 一切保持离线 artifact |
| F5-6 属上线后 | 忽略/关闭率只有真实使用才有 | D-P5 最后一项 | 记 `N-A`，登记为发布门 |

---

## 签核规则

1. **顺序**：L0 → L1 → L2 全绿（或明确 `BLOCKED`）后才受理 L3 的任何数字；L4 逐份报告执行；L5 决定 `BLOCKED` 与 `N-A` 的合法使用。
2. **变更控制**：改任何常量、断言或夹具 oracle = 回到 A8 重新登记并重跑受影响层；「扫完写死，禁止倒填」对清单本身同样生效。
3. **负对照优先**：K1/K6/F2-8/A11 这类「必须测不出」的项先于一切「必须赢够」的项受理；负对照红 = 先修 harness/生成器，正面成绩暂停受理。
4. **本清单的证伪**：若测试日发现某项与已登记裁决冲突、或某判据在全部可得夹具上恒 `inconclusive`，按 R2/opus-a §9 的原则换判据而非放宽阈值，并把修订记入下一轮综合。

*本文件为 Round 3 独立产出，未读取任何其他 Round 3 槽位；仅写入本文件，无 git 操作，无 crate 改动。*
