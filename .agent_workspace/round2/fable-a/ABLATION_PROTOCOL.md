# Round 2 / fable-a — C8 消融协议：T3 vs T3R vs T4（tie 族终选裁判规程）

编制：Round 2 fable-a（claude-fable-5-thinking-xhigh）。地位：**裁判文书**。实现者（opus / gpt-sol Round 2 槽位）按本协议跑数，裁决按第 4 节程序机械执行，不留现场发挥空间。本协议服从 R1-SYNTHESIS（BINDING）与 `round1/fable-a/CANDIDATE_SPEC.md` 的冻结定义；与二者冲突处以二者为准并先留痕再改。

本轮不实现 Rust；参考实现与跑数是 opus/gpt-sol 槽位的义务（见第 5 节）。

---

## 1. 冻结的三个候选定义

通用记号沿用 CANDIDATE_SPEC：`out/in/count/days/reciprocal/any_direct`；`Δd(i) = floor((as_of − occurred_at(i)) / 86400)` 天（负值钳到 0）；`last_contact = max(occurred_at)`（该 peer 范围内）；`age_days = max(0, floor((as_of − last_contact) / 86400))`。

全族共同前置（非差异点）：单向（¬reciprocal）封顶 Weak；空观测返回 Weak + 空计数，不触发任何时间运算；band 只写 `SupportedBand` 三词；judgement 不读墙钟。

### 1.1 T3 — Granovetter-span（无衰减，回退单体）

```text
band = Strong    if reciprocal ∧ any_direct ∧ count ≥ 10 ∧ days ≥ 3
     = Moderate  if reciprocal ∧ count ≥ 3          # group-only 封顶于此
     = Weak      otherwise
```

超参：3 / 10 / 3 + `STRONG_REQUIRES_DIRECT=true`。**不使用 as_of**。

### 1.2 T3R — T3 门闩 × 分桶半衰计数（与 CANDIDATE_SPEC 逐字一致）

```text
w(i) = 1     if Δd(i) ∈ [0, 90)
     = 1/2   if Δd(i) ∈ [90, 180)
     = 1/4   if Δd(i) ∈ [180, 360)
     = 0     if Δd(i) ∈ [360, ∞)      # 历史总数仍展示，只是不计入判档

eff_count = Σ w(i)                      # 以 1/4 为最小单位的整数运算，禁浮点判档
eff_days  = Σ_{d ∈ 活跃日} max{w(i): i 属于该日}

band = Strong    if reciprocal ∧ any_direct ∧ eff_count ≥ 10 ∧ eff_days ≥ 3
     = Moderate  if reciprocal ∧ eff_count ≥ 3
     = Weak      otherwise
```

实现纪律：`eff_count`、`eff_days` 用「四分之一单位」整数表示（阈值 10 ⟺ 40 个单位，3 ⟺ 12 个单位），比较全整数。派生不变式：group-only ≤ Moderate；one-sided → Weak；全部证据 Δd ≥ 360 → 必然 Weak。

### 1.3 T4 — T3 门闩 + 整数近因降档（本文件首次成文冻结）

```text
band0 = T3(observations)                     # 1.1 的原始判定，用原始 count/days
if count == 0:            band = Weak        # 空观测不进入降档运算
elif age_days ≥ 360:      band = Weak
elif age_days ≥ 180:      band = demote_one(band0)
else:                     band = band0

demote_one: Strong → Moderate；Moderate → Weak；Weak → Weak
```

超参：`DEMOTE_ONE_BAND_DAYS = 180`，`FORCE_WEAK_DAYS = 360`。

**边界钉死**：

- `age_days` 用整数天向下取整（`div_euclid`，与 epoch-day 纪律一致）：179 天 23 小时 → 179 → 不降；恰好 180×86400 秒 → 180 → 降。
- 未来时间戳钳 0（沿用 opus-a 的钳制，防止跑快的导入时钟制造负年龄）。
- 常量出处：360/180 正是 T3R 桶边界（4H / 2H，H=90），两候选共用一张常量表——不是巧合，是刻意对齐，禁止各自漂移。注意这**取代** opus-a Round 1 报告里的建议值 180/540：R1-SYNTHESIS 已把 540 收敛为 360，以 R1-SYNTHESIS 为准。
- 群聊封顶后再降档是合法路径（group-only 重度 + 200 天沉寂 → Moderate → Weak）。

结构不变式（T4 独有，T3R 不具备，见 4.4）：`band = Strong ⟹ age_days < 180`；`band ≥ Moderate ⟹ age_days < 360`。

### 1.4 as_of 纪律（含一个会毁掉整场消融的陷阱）

`as_of = 调用方显式传入的值；未传则 = 整个 store 的 max(occurred_at)`。禁读墙钟（F15）。

**陷阱警告**：`dormant_since_2019` 这类夹具的 store 里只有 2019 年的消息，`max(occurred_at)` 本身就在 2019。如果消融 harness 无脑套用「as_of = 数据内最大时间戳」而忽略夹具显式携带的 `now = NOW_2026_08_24`（opus-a `testing.rs` 每个夹具都返回自己的 now；EVAL_MATRIX F04 的 Given 明写「as_of=今」），那么 F04 上衰减永不触发，T3R/T4 与 T3 全等，第 4.1 节的简单性门会**假性触发**、错杀衰减族。裁判在验收跑数时第一件事就是核对 F04 的 as_of 取值。真实产品里对应的语义是：as_of 取全 store 最大时间戳（任何 peer 的新导入都会推动它），单 peer 夹具必须用显式 as_of 模拟这一点。

### 1.5 Δ-T3 定义漂移（裁判裁定）

opus-a 的 `t3.rs` 按其 Round 1 任务书实现了额外的群聊 Moderate 门（group-only 需 count≥5 ∧ days≥2 才 Moderate），CANDIDATE_SPEC 的 T3 没有这道门（group-only 互惠 count≥3 即 Moderate）。二者在共享矩阵上无分歧（group_only_50 都给 Moderate），但在「群聊互惠 3–4 条或仅 1 天」的角落分叉。

**裁定：绑定定义 = CANDIDATE_SPEC 的 T3（1.1 节原文）**，依据是 R1-SYNTHESIS 指定 CANDIDATE_SPEC 为冻结源。消融中的 T3、以及 T4 的 `band0`，一律按 1.1 节实现。opus-a 的 `GROUP_ONLY_CEILING` 常量保留（= Moderate，维持 opus-a 3.1 节的裁决），额外的 ≥5/≥2 门**不带入** Round 2。此漂移需进父代理 DECISIONS 留痕。

---

## 2. 夹具集

### 2.1 绑定集 B（Round 1 共享夹具，净胜 / 零回归的唯一计分依据）

来源：opus-a `soul-algo-tie/src/testing.rs`（公开模块，逐字复用）+ EVAL_MATRIX 的 F05。期望列先于实现存在，是判卷答案。

| # | 夹具 | 构造要点 | as_of | 钉死期望 |
|---|---|---|---|---|
| B1 | `empty` | 空观测 | 显式 NOW_2026_08_24 | Weak（空计数，不崩） |
| B2 | `single_inbound` | 1 条 in，D | 同上 | Weak |
| B3 | `lilei_12_over_6_days` | 12 条互惠 D，6 天，3 天前收尾 | 同上 | **Strong** |
| B4 | `twenty_in_one_afternoon` | 20 条互惠 D，1 个自然日 | 同上 | **Moderate 封顶** |
| B5 | `group_only_50` | 50 条互惠 G，50 天，无 D | 同上 | **≤ Moderate** |
| B6 | `dormant_since_2019`（=F04） | 20 条互惠 D，2019 年 10 天内 | **显式 NOW_2026_08_24（见 1.4 陷阱）** | **非 Strong** |
| B7 | `one_sided_100_outbound` | 100 条 out，0 in | 同上 | Weak |
| B8 | F05 细水长流 | 近 8 周每周 2 条互惠 D，8 个自然日，16 条 | 同上 | **Strong** |

约束附加项（不进净胜计分，挂一个即候选出局或退回修复）：

| # | 项 | 判定 |
|---|---|---|
| K1 | F15 墙钟无关 | 同一 store，mock 两个不同墙钟跑两次，band 与解释逐字节一致 |
| K2 | F09 遗忘降档幂等 | 删证据重算只会降档或持平；更新原行不新增 |
| K3 | F13 跨午夜 | 行为确定 + 文档化即可（v0.1 不加 span≥72h 门，NOTES Q3 结论沿用） |
| K4 | F14 体量 | 10 万交互 O(n) 单遍；debug < 10s |

### 2.2 分歧探针集 D（Round 2 新增，仅用于 T3R vs T4 内部对决）

T3R 与 T4 在绑定集 B 上**预测完全同构**（见第 3 节），必须有探针暴露真实分歧面。分歧面恰好有两个方向，各造一个探针。期望在跑数之前钉死于此；任何人想改期望，走父代理 DECISIONS，不许在测试文件里就地改。

**F04b 半沉寂重历史**（T3R 的失效面）：

```text
peer=6。i ∈ 0..22：第 (as_of − (211−i)·86400) 天上，09:00 一条 out(D)、21:00 一条 in(D)。
共 44 条，22 个自然日，全部年龄 ∈ [190, 211] 天 ⊂ [180, 360)。as_of 显式 = NOW_2026_08_24。
```

- 推演：T3 → Strong（44≥10，22≥3）。T3R → eff_count = 44×¼ = 11 ≥ 10，eff_days = 22×¼ = 5.5 ≥ 3 → **Strong**。T4 → band0=Strong，age_days=190 ∈ [180,360) → **Moderate**。
- **钉死期望：非 Strong。** 理由一（常识）：190 天没说过一句话的人不是「当前强关系」——F04 之所以成立的同一条理由，只是年龄从 7 年缩到半年多。理由二（自洽性，取自 T3R 自己的冻结规范）：T3R 规定 last_contact > 180 天时展示「你们最近半年没有往来」句；若 band 同时显示「强」，同一屏幕自相矛盾。这条理由来自 T3R 规范内部，不是为 T4 量身定做的。

**F04c 复燃一响**（T4 的失效面）：

```text
peer=7。dormant_since_2019 的原 20 条（构造照抄 testing.rs）
+ (as_of − 86400) 天上 09:00 一条 out(D)、21:00 一条 in(D)。
共 22 条，11 个自然日。as_of 显式 = NOW_2026_08_24。
```

- 推演：T3 → Strong（22≥10，11≥3，互惠、有 D）。T4 → age_days=1 < 180，不降档 → **Strong**（一次寒暄把 7 年前的全部历史按原值复活）。T3R → eff_count = 2×1 + 20×0 = 2 < 3 → **Weak**。
- **钉死期望：非 Strong。** 理由：一来一回两条消息连 F02「一次对话不算习惯」的门槛都够不着，凭它复活 2019 年的计数直接违背 F04 的立法本意。（T3R 的 Weak 也偏严——常识答案约为 Moderate——但「非 Strong」期望下判 Weak 合格；错弱是保守错，几条新往来即自愈；错强才是本轮清算的 P0 错误类。）

---

## 3. 预注册期望矩阵（跑数前的预测，防对着实现出题）

| 夹具 | 钉死期望 | T3 | T3R | T4 |
|---|---|---|---|---|
| B1 empty | Weak | Weak ✓ | Weak ✓ | Weak ✓ |
| B2 single_inbound | Weak | Weak ✓ | Weak ✓ | Weak ✓ |
| B3 lilei_12 | Strong | Strong ✓ | Strong ✓ | Strong ✓ |
| B4 one_afternoon | Moderate | Moderate ✓ | Moderate ✓ | Moderate ✓ |
| B5 group_only_50 | ≤Moderate | Moderate ✓ | Moderate ✓ | Moderate ✓ |
| B6 dormant_2019 | 非 Strong | **Strong ✗** | Weak ✓ | Weak ✓ |
| B7 one_sided_100 | Weak | Weak ✓ | Weak ✓ | Weak ✓ |
| B8 细水长流 | Strong | Strong ✓ | Strong ✓ | Strong ✓ |
| F04b 半沉寂重历史 | 非 Strong | Strong ✗ | **Strong ✗** | Moderate ✓ |
| F04c 复燃一响 | 非 Strong | Strong ✗ | Weak ✓ | **Strong ✗** |

预测汇总：绑定集 B 上 T3R ≡ T4，双双仅在 B6 净胜 T3，零回归。探针集 D 上 1:1 平（F04b 归 T4，F04c 归 T3R）。**任何实测偏离本表的格子，实现者必须给出该格的一行算式并上报，不许就地改期望。**

---

## 4. 判定程序（按序执行，前一阶段出局即停）

### 4.1 阶段 S0 — 简单性门

在**绑定集 B**上（探针集 D 不参与——D 是为了区分两个衰减候选而造的，用它论证「衰减有必要」是循环论证），若 T3R 与 T4 相对 T3 **均未改变任何一个夹具的 band** → 衰减无净胜，**终选 = T3**，消融结束。

预测：不触发（B6 翻档）。若实测触发，第一嫌疑是 1.4 节的 as_of 陷阱，先查 harness 再下结论。

### 4.2 阶段 S1 — 零回归筛（红线）

回归的定义：存在夹具 f ∈ B，T3 的 band 满足 f 的钉死期望，而候选的 band 不满足。任一候选出现回归 → 该候选出局。两候选都出局 → 终选 = T3。

补充：候选与 T3 不同但**双双满足**期望（如 ≤Moderate 下一个给 Moderate 一个给 Weak）不算回归，记「无端分歧」，进 S4 作负项。预测：本轮无此情形。

### 4.3 阶段 S2 — 净胜计数

先数 B（对 T3 的净胜数），再数 D（T3R 与 T4 互比）。预测：B 上双方各 1（B6）；D 上 1:1。若某候选在 D 上 2:0 → 该候选直接胜出，不进 S4。

### 4.4 阶段 S3 — 不变式与约束附加项

对存活候选逐条验证（性质测试，NOTES Q9 清单扩充）：

| # | 不变式 | 适用 |
|---|---|---|
| I-1 | one-sided → Weak；group-only ≤ Moderate | 全体 |
| I-2 | 全部证据年龄 ≥ 360 天 → Weak | T3R、T4 |
| I-3 | 新增一条证据永不降档；遗忘一条证据永不升档 | 全体（注意 T4：遗忘最近一条会推大 age_days，只会降档，方向正确） |
| I-4 | Strong ⟹ age_days < 180（「强 ⟹ 最近半年有过往来」） | **仅 T4 成立**；T3R 在 F04b 上被证伪。此不对称性进裁决记录 |
| I-5 | 置换不变、整体时间平移不变（时间戳与 as_of 同步平移，band 不变）、遗忘子集一致性 | 全体 |
| K1–K4 | 2.1 节约束附加项 | 全体 |

I-1/I-2/I-3/I-5/K 系挂任何一条 → 退回修复重跑；修不掉 → 出局。I-4 不是生死线，是 S4 的证据。

### 4.5 阶段 S4 — 解释可数性判（平局裁决）

S2 平局时执行。判据是**模板级**的（对模板判，不对个例判，防止挑选恰好整数的个例）：

> 候选的中文解释模板（COPY_ZH.md 冻结版）中，每一个数字占位符必须是**整数**，且等于用户可通过数消息、数日期直接得到的原始计数、自然日数、天数差或日期。模板若要求用户做分数折算（「按半次计」「四分之一」「折算 N.5 次」）即判不可数。

预判：T4 通过（判据只有原始次数、自然日数、最后一次日期、整数天数差）；T3R 不通过（折算是它的本体，eff_days 还会出现 5.5 这类半整数——F04b 即实例）。**通过者胜。** 双双通过或双双不通过（预计不会发生）→ 上报父代理仲裁，附 I-4 记录。

### 4.6 裁决树汇总

```text
S0 门触发？ ──是──► 终选 T3
   │否
S1 有回归？ ──T3R回归──► T4（若 T4 也回归 → T3）
   │          └T4回归──► T3R
   │均无
S2 D 上 2:0？ ──是──► 该候选胜
   │否（1:1 或 0:0）
S4 可数性：T4 过、T3R 不过 ──► 终选 T4（预测落点）
             T3R 过、T4 不过 ──► 终选 T3R
             其他 ──► 父代理仲裁（附 I-4）
```

---

## 5. 实现者义务（opus / gpt-sol Round 2 槽位）

1. **前置**：`TieScore` 增加 `any_direct: bool`（R1-SYNTHESIS 攻坚项 2）；T4 另需 `last_contact` 与 `as_of` 可达（前者已有）。
2. 三个候选按第 1 节冻结定义实现为纯函数；T3R 判档全整数（四分之一单位），T4 全整数；`#![forbid(unsafe_code)]`、零重依赖、无网络、不读墙钟。
3. 跑第 3 节全表 + 第 4.4 节全部不变式，输出机器可复现的矩阵（照 opus-a `examples/matrix` 的做法：报告表格由代码打印，不许手抄）。
4. **每一个 band 改变的格子，附一行用户可复核的算式**（例：B6/T4：「最后一次 2019-06-10，as_of 2026-08-24，相差 2632 天 ≥ 360 → 弱」）。
5. 实测与第 3 节预注册矩阵不一致的格子：先查实现，再查夹具构造，仍不一致则上报父代理并引用本协议格子编号；**禁止**修改钉死期望。
6. 产出写各自 round2 槽位目录；本协议与裁决记录归 fable-a 目录。

## 6. 判卷纪律（沿 EVAL_MATRIX 第五节，全文有效）

期望先于实现存在；红线夹具（S1、K1、F12/F15 类）挂一个即出局无分数可谈；任何期望变更走父代理 DECISIONS 留痕。本协议第 3 节矩阵即「先写成失败的测试」的判卷答案。
