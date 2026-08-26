# Round 1 / fable-a — 候选冻结规范（T0–T3, A0–A3, 合成 T3R）

本文件冻结候选定义。Round 2+ 实现与消融必须以此为准；改定义先改本文件并留痕。
通用记号：对一个 peer 的观测集 `I = {i}`，每条含 `direction ∈ {out,in}`、`venue ∈ {D,G}`、`occurred_at`（UTC RFC3339）。
`out = |{i: direction=out}|`，`in = |{i: direction=in}|`，`count = out+in`，
`days = |{utc_date(i)}|`，`reciprocal = (out>0 ∧ in>0)`，`any_direct = ∃i: venue=D`，
`as_of = max(occurred_at over 整个 store)`（**禁止读墙钟**），`Δd(i) = as_of − occurred_at(i)`（天，向下取整）。

所有 T 候选共同前置（不是差异点）：单向封顶 Weak；空图返回空结果；band 只写 `SupportedBand` 三词；每条边/推断携带全部 `evidence_ids`；幂等 rebuild（更新原行不新增）。

---

## T0 — Count+Reciprocity+Span（现实现，基线）

```text
band = Strong    if reciprocal ∧ count ≥ 10 ∧ days ≥ 3
     = Moderate  if reciprocal ∧ count ≥ 3
     = Weak      otherwise
```

**超参**：`MODERATE_MIN_INTERACTIONS=3`，`STRONG_MIN_INTERACTIONS=10`，`STRONG_MIN_ACTIVE_DAYS=3`（与 `graph_build.rs` 一致）。

**中文解释话术**：
- 强：「你们互相都发过消息，一共 {count} 次，分布在 {days} 个自然日里，达到强档标准（≥10 次且 ≥3 天）。」
- 中：「你们互相都发过消息，共 {count} 次（≥3 次），但还没跨过 3 个自然日或 10 次，先算中等。」
- 弱：「{只有一方发过消息 / 往来少于 3 次}，先算弱。」

**消杀条件**：T3R 在 F03（群聊重度）与 F04（陈年关系）净胜且全夹具零回归 → T0 归档「被支配」。**预期 Round 2 末执行。**

---

## T1 — Recency-weighted decay（分桶半衰版，冻结此版而非连续 exp）

理论形是 `score = Σᵢ exp(−λ·Δd(i))`（Hawkes/Burt 谱系）。冻结的是它的**可解释量化**：按半衰期 H 分桶的几何权重，权重是有理数，判档不碰浮点。

```text
w(i) = 1     if Δd(i) < 90        # 桶 B0
     = 1/2   if 90 ≤ Δd(i) < 180  # 桶 B1
     = 1/4   if 180 ≤ Δd(i) < 360 # 桶 B2
     = 0     if Δd(i) ≥ 360       # 不再计入判档（历史总数仍展示）

eff_count = Σ w(i)                       # 用分数运算，如以 1/4 为最小单位的整数
eff_days  = Σ_{d ∈ 活跃日} w(该日代表权重)  # 每个自然日取该日内最大 w

band = Strong    if reciprocal ∧ eff_count ≥ 10 ∧ eff_days ≥ 3
     = Moderate  if reciprocal ∧ eff_count ≥ 3
     = Weak      otherwise
```

**超参**：`H=90 天`（桶宽=H；SHARED_BRIEF 给的 30/90 中**否决 30**：半衰 30 天意味着一次三周长差旅就可能掉档，band 抖动不可解释）；`CUTOFF=360 天`（≥4H 一律 0 权，防止「三年前一万条」靠尾巴撑强档）；阈值沿用 3/10 保持与 T0 解释连续。

**中文解释话术**：「近 90 天的往来按 1 次计，90–180 天按半次计，180–360 天按四分之一计，再早的不计。这样算下来你们相当于 {eff_count} 次、跨 {eff_days} 天，所以是{band}。」——每个桶的原始条数都展示，用户可复核。

**消杀条件**：T1 单体在 F02（一下午热聊判 Strong）结构性挂 → **单体不保留**；其度量并入 T3R。若 Round 2 实测 F02 意外通过（说明我推演错了），重新入池与 T3R 平行消融。

---

## T2 — RFM-band（分位数版，Round 1 建议击毙，规范留档以示公平）

```text
对 ego 网内全部 peers：
R(p) = as_of − last_contact(p)   # 越小越好
F(p) = count(p)
M(p) = days(p)
每一维做三分位切分 → r,f,m ∈ {1,2,3}（3 为最好档）
score = r+f+m ∈ [3,9]
band = Strong    if reciprocal ∧ score ≥ 8
     = Moderate  if reciprocal ∧ score ≥ 6
     = Weak      otherwise
```

**超参**：三分位；8/6 切点。
**中文解释话术**（写出来即暴露问题）：「与你的其他联系人相比，这个人的近因/频次/天数综合排在前三分之一…」——**这是比较性解释，语义等同 percentile。**
**消杀条件（已触发）**：F12（导入无关联系人改变既有档位）与 F15（人群相对 → 无固定期望）结构性挂；解释话术违反 D22 精神。**判决：Round 1 归档「不采用」。** 否决理由：人群相对基线不确定、类 percentile 措辞、冷启动（<3 peers）无定义。绝对阈值化的 RFM 不是 T2，是 T0/T1 换皮，不另立候选。

---

## T3 — Granovetter-span（结构门闩完全体）

```text
band = Strong    if reciprocal ∧ any_direct ∧ count ≥ 10 ∧ days ≥ 3
     = Moderate  if reciprocal ∧ count ≥ 3          # 群聊-only 封顶于此
     = Weak      otherwise
# 派生不变式：group-only（¬any_direct）永不 Strong；one-sided 永远 Weak
```

**超参**：同 T0（3/10/3）+ 场合闩 `STRONG_REQUIRES_DIRECT=true`。
**中文解释话术**：
- 强：「你们互相都发过消息、有过一对一的交流，共 {count} 次、跨 {days} 天，达到强档标准。」
- 群聊封顶：「到目前为止只在群里见过对方发言，没有一对一的记录，所以最多算中等。」

**消杀条件**：若 T3R 在 F04 净胜且零回归 → T3 降为「回退单体」保留在规范附录（不删除：它是打架时的退路，见 ACCEPTANCE.md）。若 T3R 消融失败（衰减在真实形态夹具上从不改变任何档），**T3 即最终胜者**。

---

## T3R — 合成候选：T3 门闩 × T1 度量（PROVISIONAL 胜者）

不是新发明：Granovetter 结构门闩（T3）+ Burt/Hawkes 衰减计数（T1 分桶版）的拼装。受 C8 消融门约束。

```text
（w、eff_count、eff_days 定义同 T1）

band = Strong    if reciprocal ∧ any_direct ∧ eff_count ≥ 10 ∧ eff_days ≥ 3
     = Moderate  if reciprocal ∧ eff_count ≥ 3
     = Weak      otherwise

不变式：one-sided → Weak；group-only → ≤ Moderate；
       沉寂 ≥360 天 → 必然 Weak（所有权重归零）；
       任何 band 变化都可用「桶内条数 × 桶权重」的一行算式复现。
展示补充：last_contact > 180 天时，边与摘要附加「你们最近半年没有往来」句（带该条证据 id）。
```

**超参默认**：`H=90`，`CUTOFF=360`，阈值 `3/10/3`，`STRONG_REQUIRES_DIRECT=true`。全部进 `docs/algorithms/` 常量表，禁止散落。

**中文解释话术**（完整版，用户可见）：
「怎么算的：只统计次数和日期，不读聊天内容。近 90 天的往来按 1 次计，90–180 天按半次计，180–360 天按四分之一计，更早的不计。强档需要同时满足：双方都发过消息、有过一对一交流、按上面折算 ≥10 次、且分布在 ≥3 个自然日。你们折算后是 {eff_count} 次、{eff_days} 天，{有/没有}一对一记录，所以是{band}。这是工作假设，你可以直接改。」

**消杀条件**：
1. C8 消融：在 opus-a/gpt-sol 的全部夹具上，T3R 相对 T3 若无任何净胜夹具（即衰减从未改变任何档）→ 杀 T3R，留 T3（简单者胜）。
2. 零回归：T3R 在任何 T3 通过的夹具上翻车 → 杀。
3. 解释复核：若中文话术在 Round 3 用户视角评审中被判「数字不可自行复核」→ 退回 T3。

---

## A0 — Questionnaire + correction lock（现实现，产品锁基底）

```text
intake(问卷答案 a → 轴 x):
  position(x) = a.position;  band(x) = Moderate;  evidence = [该答案的 evidence 行]
  method = user_stated;  locked 不变（问卷不锁轴）
correct_axis(x, p):
  position(x) = p;  band(x) = Strong;  locked(x) = true
  写 UserCorrection 证据行（Strong）
record_axis_inference(proposal):
  空 evidence → 拒绝；statement/falsifier 过 assert_non_clinical
  locked(x) → 存推断行但不动轴（RefusedAxisLocked）
  否则应用并替换轴上的 evidence_ids
```

**超参**：`QUESTIONNAIRE_STRENGTH=Moderate`，`CORRECTION_STRENGTH=Strong`（与 `profile_service.rs` 一致）。
**中文解释话术**：「这一条来自你 {date} 的问卷回答（{N} 条依据）／来自你本人的纠正，已锁定，之后的推断不会覆盖它。」
**消杀条件**：无。产品锁 v0.1 切片第 3/4 条直接要求它，不参与淘汰；任何 A 族胜者必须以它为基底。

---

## A1 — Evidence-count band upgrade（有条件候选）

```text
independent(E) = 按 (evidence.kind, utc_date(created)) 去重后的组数
    # 同一天同一来源的任意多条 = 1 组；问卷重填不叠加
band(x) = Strong    if locked(x)                        # 锁恒强，永不被算法改
        = Strong    if independent(同方向证据) ≥ 3
        = Moderate  if ≥ 1 条证据
        = None/unknown otherwise
冲突规则：若两个方向的 independent 组数均 ≥ 2 → position = mixed, band ≤ Moderate
        # 禁止静默取多数：方向分歧本身就是「mixed」的证据
永不改 locked 轴；升档推断照常走 record_axis_inference（会被锁拒绝应用）。
```

**超参**：`K_STRONG=3` 组；独立性键 = `(kind, utc_date)`。
**中文解释话术**：「这个方向有 {n} 组不同日期、不同来源的依据（{列日期}），所以从中等升为强。」
**消杀条件**：F16（同日重填问卷 5 次不得升档）挂 → 杀。F16b（真独立升档）过不去 → 杀。若 Round 2 结束时「独立性」定义仍需要人肉判断（不可纯函数判定）→ 杀。v0.1 唯一现实证据源是问卷+纠正，若消融显示 A1 在 v0.1 数据面上**永远不会触发**（没有第三种证据来源），则归档「正确但空转」，留给 v0.2 行为证据接入时复活。

---

## A2 — Statistical personnel summary（在位实现，钉死保留）

```text
输入：一条边的 TieStrength（band、count、out、in、days、conversations、first/last_contact）
     + 已解析证据行（须与边引用完全对齐，缺一即 UnresolvedEvidence 错误）
输出 points（每条带 evidence_ids ⊆ 边证据，构造期过 assert_non_clinical）：
  P1 总量：「一共 {count} 次往来，分布在 {days} 个自然日、{conversations} 个会话里。」（cite 全部）
  P2 方向：out/in + 形态句（0:「都是对方在说」；>2:1:「多数时候是你先开口」；否则「两边说得差不多」）（cite 全部）
  P3 场合：有 D →「有过一对一交流」（cite D 行）；仅 G →「只在群聊见过」（cite G 行）
  P4 近因：「最近一次是 {date}」（只 cite 那一条）；last>180 天 → 追加「最近半年没有往来」
  P5 归档：「按上面的计数归在「{band}」一档；这是工作假设，不是对这个人的判断。」（cite 全部）
无 key：source=Counts，零网络。有 key：E1 只送这些自产计数句（过 redactor），只允许改 narrative。
称呼恒为「这个人」；无姓名字段可达。
```

**超参**：方向比阈值 `2:1`；沉寂阈值 `180 天`；近因句只引最近一条证据。
**消杀条件**：无（AC-16/AC-17 门禁所系）。可调不可杀。**约束**：A2 自身不得引入第二套 band 阈值——band 由 T 族胜者产出，A2 只消费（单源词汇表，见 ACCEPTANCE.md）。

---

## A3 — Lexicon/LIWC 正文推断（Round 1 击毙，留档）

假想形态（仅为否决留据，禁止实现）：对用户**自己的** outgoing 正文做中文词表匹配（LIWC/SC-LIWC 谱系），按相对词频超阈值给轴方向、band 恒 Weak。

**判决：归档「不采用」。** 否决理由（对硬约束逐条）：
1. 需要开封正文，推翻「图管道不携带正文」的数据面设计（C4 硬违规）；
2. 文献效应量 r≈0.2–0.4（Yarkoni 2010 等），只配 weak 档，边际价值被 A0 纠正锁压死；
3. 中文词表效度无验证资源；
4. `emotional_steadiness` × 情绪词频 = 临床化高危走廊（D5/D22 红线擦边）；
5. 解释必然暴露文本挖掘，违背「可向用户解释且用户能复核计数」的产品观感。
复活条件（v0.2+）：用户显式开启、仅限本人正文、走 E1、独立同意书。v0.1 内任何实现候选签名含正文参数即 review 直接否决（EVAL_MATRIX F22）。

---

## 冻结汇总

| ID | 状态 | 一句话 |
|---|---|---|
| T0 | 基线，预期 Round 2 被支配后归档 | 原始计数 + 互惠 + 跨日 |
| T1 | 单体不保留，度量并入 T3R | 分桶半衰计数（H=90，360 截断） |
| T2 | **Round 1 击毙** | 分位数 RFM；人群相对＝不可解释不可测 |
| T3 | 回退单体，保留 | T0 + 私聊闩 + 群聊封顶 |
| T3R | **PROVISIONAL 胜者**，受 C8 消融门 | T3 门闩 × T1 度量 |
| A0 | 产品锁基底，不参与淘汰 | 问卷 Moderate + 纠正锁 Strong |
| A1 | 有条件（F16/F16b 生死线） | 独立证据 ≥3 组升 Strong，锁优先 |
| A2 | **钉死保留**（在位实现） | 纯计数人事摘要，band 只消费不定义 |
| A3 | **Round 1 击毙** | 正文词表推断；违 C4 |
