MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 1 · opus-b · 特质轴与人事分析算法族（A0 / A1 / A2 / A3）

产出：`.agent_workspace/round1/opus-b/soul-algo-trait/`，独立 Cargo 包，edition 2021，
rust-version 1.83，**零依赖**，`#![forbid(unsafe_code)]`，`#![deny(missing_docs)]`。
无 HTTP、无 SQLCipher、无 Tauri、无时钟读取、无随机、无正文输入。

```
cd .agent_workspace/round1/opus-b/soul-algo-trait
cargo test            # 72 passed
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

完整记录见同目录 `TEST_LOG.txt`（含 rustc/cargo 版本、fmt、clippy、cargo test 全文）。

| 文件 | 内容 |
|---|---|
| `src/types.rs` | `Band` / `Position` / `AxisId` / `EvidenceRef` / `AxisState` / `ApplyResult` / `ShadowInference` / `AxisOutcome` |
| `src/a0.rs` | A0 基线：问卷 + 纠正锁定 |
| `src/a1.rs` | A1：证据计数升档 + 三种分歧策略 |
| `src/a2.rs` | A2：统计人事摘要（WP10 无 key 降级路径） |
| `src/a3.rs` | A3：正文词典推断 **恒拒绝**（否决候选写进代码） |
| `src/denylist.rs` | 三道措辞闸：诊断词 / 评分词 / 第三人声称 |
| `src/fixtures.rs` | 确定性夹具（10 个 peer 场景 + 10 个证据日志场景），供本轮及后续轮复用 |
| `tests/` | 6 个测试文件，72 条断言用例 |

---

## 1. 实现的规则（可向用户复述的版本）

### A0 — 问卷 + 纠正锁定（对齐 Goal 1 `service.rs` 语义）

把 Goal 1 分散在 `intake` / `correct_axis` / `record_axis_inference` 里的三段就地改写，
重写为对证据日志的**纯函数重放**：同一条日志重放出来的轴，等于 store 里那条轴。
这既让基线可以脱离 store 单测，也让「遗忘后自动降级」成为重放的自然结果，而不是另写一条补偿路径。

1. 先丢弃 `forgotten` 行。
2. 其余每行按记录顺序**替换**轴（等价于 `place_axis`）：`evidence_ids` 说明的是「**现在**这个方向靠什么支撑」，不是历史全集。
3. 问卷 = `Band::Moderate`，不锁定；用户纠正 = `Band::Strong` 且锁定。
4. 锁定后的非纠正推断进入 `AxisOutcome.shadow`，标 `ApplyResult::RefusedLocked`，**不动轴**。更晚的纠正仍然生效——锁挡的是机器，不是用户。
5. 无证据 → `Unknown` + `Band::None`，`evidence_ids` 为空。
6. 全程无分数。`AxisState` 的 `Debug` 渲染也过评分词闸（`no_axis_state_carries_anything_numeric`）。

### A1 — 证据计数升档

锁的规则与 A0 完全一致（`a0_vs_a1.rs::the_lock_behaves_identically_under_both` 在全部夹具上逐条比对）。
区别只在没有锁的时候：A0 留下最后一行，A1 聚合全部存活行。

- **独立性 = 不同的 `evidence_id`**，重复 id 折叠为一条，保留首次出现。A1 不去判断两条不同的行是否同源——它做不到，装作能做就是把权重藏起来。
- **升档**：同方向、且各自至少 `Moderate` 的独立行 ≥ 3 → `Strong`；否则取最强单行并**封顶 `Moderate`**。封顶就是「单条问卷答案永远不会变成 Strong」和「机器给自己贴 Strong 也没用」这两条的实现。
- **分歧**：同时出现 `leans_low` 与 `leans_high`（或出现显式 `mixed` 行）→ `Position::Mixed` + `Band::Weak`。
- **纠正优先**：只要有存活的纠正行，轴取**最后一条**纠正的方向、`Strong`、锁定，并引用所有与之同向的纠正；其余存活行（含更早被推翻的纠正）全部进 shadow 标 `RefusedLocked`。注意与 A0 的差别：A1 从全集重推，所以**记录在纠正之前**的推断同样被拒绝。

### A2 — 统计人事摘要（无正文）

输入 `PeerStats`（互动次数、发出、收到、活跃天数、最近接触时间、是否见过一对一、是否互惠、证据 id）。
输出 `PersonnelSummary { bullets }`，每条 `SummaryBullet` 带 `statement_key` / `text_zh` / 非空 `evidence_ids` / `band`。

固定顺序，四类语句：

| statement_key | 说什么 |
|---|---|
| `personnel.tie.reciprocal` / `.two_way_unconfirmed` / `.one_way_outgoing` / `.one_way_incoming` | 往来是否互惠、往哪个方向 |
| `personnel.activity.counts` | 有记录的往来次数与出现在几个不同的日子 |
| `personnel.recency.days_since_last` | 最近一次往来距今多少天（由 `now_unix` 参数算，不读时钟） |
| `personnel.venue.group_only` | 是否只在群里出现过 |

- 单向往来**永不 Strong**；`days_since_last` **封顶 Moderate**（它只压在一个时间戳上）。
- 只有「只在群里」会出条目；见过一对一时**不出**反向条目——「你们也单独聊过」已经是在描述关系了。
- 最近接触时间晚于 `now_unix` 视为上游时钟不一致，读作「就在今天」，不产生负数。
- 措辞过三道闸：诊断词（抑郁/焦虑/障碍/诊断/人格障碍/病/score/percentile/百分位）、评分词（%/评分/打分/满分/分位/量表）、第三人声称（朋友/好友/闺蜜/知己/亲密/关系好/感情/性格/人格/内向/外向/信任）。群聊场景另外断言不出现「私聊 / 单聊 / 私下」。

### A3 — 正文词典推断：恒拒绝

`a3_from_message_text(axis, text, evidence_ids) -> Result<AxisState, A3Refused>`
永远返回 `Err(A3Refused { reason: "v0.1_forbids_text_trait_inference" })`，批量形式 `a3_from_messages` 同理。
`A3_ALGORITHM_ID = "a3.lexicon_text_inference.rejected"` 注册为「被否决」，`is_rejected_algorithm` 可在写入边界上识别它。

否决理由写在模块文档里，理由不是风格问题：需要第三人正文（C4 归零）／用户无法复核权重（C3 归零，且权重就是 D22 禁的分数）／公开词典的类目天然临床化（C5 违反 D5）／打不过已有基线（C8）。
把它做成一个有测试的拒绝函数，而不是设计文档里的一段话——只写在文档里的否决，会被下一个想到同样点子的人重新实现一遍。

---

## 2. 需要拍板记录的解释性决定

任务书里有几处措辞可以有两种读法。这里写死我选了哪一种、为什么，方便 Round 2 直接推翻或采纳。

| # | 分歧点 | 选择 | 理由 |
|---|---|---|---|
| DA-1 | A1 的「independent」怎么算 | 不同 `evidence_id` 即独立，重复 id 折叠 | 唯一可复核的定义；用户能自己数行数。同源性判断需要正文或时间聚类，两者都会把可解释性交出去 |
| DA-2 | 「majority position」vs「分歧即 Mixed」 | **默认严格 Mixed**；`DisagreementPolicy::Majority` 与 `BandFloored` 作为可选策略并存 | 用计数压过用户自己的另一条答案，就是把分数藏起来。但两种读法都值得用同一批夹具比，所以都实现、都有测试，默认取最保守的那个 |
| DA-3 | A2 空输入 | **不出任何条目**（`PersonnelSummary::is_empty()`），不出「还看不出」条目 | `SummaryBullet` 承诺 `evidence_ids` 非空；一条无证据的条目会成为「无证据不得落库」的第一个例外。「还看不出」是调用方的文案，不是算法的断言 |
| DA-4 | `Position::Unknown` 的证据行 | 非纠正的 Unknown 行在 A1 里聚合前丢弃；A0 里按替换语义应用但 `Band` 归 `None`。纠正到 Unknown 合法，会锁定且 `Band::None` | 「看不出方向」支撑不了任何方向；但用户说「别猜了」是一次合法纠正 |
| DA-5 | 无证据可引时是否仍出人事摘要 | 不出 | 同 DA-3 |
| DA-6 | A2 的 band 阈值 | 暂时复用 `graph_build.rs` 的 3 / 10 / 3 天 | 见下面的风险 R-2，这是**待消除的重复**，不是设计意图 |

---

## 3. 两个发现

### F-1（缺陷，已用测试钉住）：基线会让一条弱推断顶掉用户的问卷答案

Goal 1 的 `place_axis` 一次性覆盖 position、band 和 `evidence_ids`。于是：

- 用户问卷答 `leans_high`（Moderate，引用行 1）
- 随后一条 `Weak` 的机器推断说 `leans_low`（行 2）
- 轴变成 `leans_low` / `Weak`，**且 `evidence_ids` 变成 `[2]`——用户那条答案不再被引用**

轴没有被锁定，所以这不违反任何已写下的规则；但它把「用户说的」降级成「机器最近一次猜的」，而且在 UI 上不再可见。
测试：`tests/a0_vs_a1.rs::a_weak_inference_overwrites_a_questionnaire_answer_under_a0_but_not_under_a1`。
A1 在同一输入上给出 `Mixed` / `Weak` 且引用 `[1, 2]`，两条都还在。
`DisagreementPolicy::BandFloored` 更进一步，让 `Moderate` 的答案顶住 `Weak` 的猜测（`band_floored_policy_does_not_let_a_weak_guess_unseat_an_answer`）。

附带发现：A0 的结果**依赖记录顺序**，A1 不依赖（`a1_is_order_independent_and_a0_is_not`）。导入器不保证顺序时这会变成真实的不确定性。

### F-2（风险）：A2 与人脉图各算一遍同一组阈值

A2 现在自己按 3 / 10 / 3 天算 `sample_band`，人脉图在 `graph_build.rs` 里也算一遍。
两处一旦漂移，同一条边会在图上显示 Strong、在人事摘要里显示 Moderate，而用户看到的是同一组计数。
建议（Round 2 动作）：A2 直接消费边上已经算好的 band，`PeerStats` 增加一个 `edge_band` 字段，删掉 `sample_band`。
本轮没有直接这么做，是因为 tie-strength 的最终形态归 opus-a，A2 不应该先把接口钉死在一个还没选定的算法上。

---

## 4. 评分（C1–C8，1–5，附证据）

评分对象是「作为 v0.1 规范实现」的适配度，不是学术新颖度。

| | C1 产品锁 | C2 可测 | C3 可解释 | C4 隐私 | C5 稳健 | C6 成本 | C7 SOTA | C8 创新必要 | 均值 |
|---|---|---|---|---|---|---|---|---|---|
| **A0** | 5 | 5 | 5 | 5 | 3 | 5 | 2 | — 基线 | 4.3 |
| **A1** | 5 | 5 | 5 | 5 | 4 | 5 | 3 | 4 | 4.4 |
| **A2** | 5 | 5 | 4 | 4 | 4 | 5 | 3 | 4 | 4.3 |
| **A3** | 1 | 2 | 1 | 1 | 2 | 3 | 4 | 1 | 1.9 |

证据：

- **A0 C1=5**：逐条对应 PRODUCT_LOCK「推断带证据与证据档／用户纠正锁定，后续推断不覆盖」，切片第 3、4 条。
- **A0 C2=5**：纯函数、无时钟、无随机；13 条用例。**C3=5**：用户能数自己答了什么。
- **A0 C5=3**：扣分项就是 F-1（弱推断顶掉答案）与顺序依赖。这是本轮对基线唯一的实质性批评。
- **A0 C7=2**：没有吸收任何既有理论，就是「最后写入者获胜 + 用户锁」。这不丢人，但也别包装成方法。
- **A1 C5=4**：修掉 F-1 与顺序依赖；仍然会被三条同源但 id 不同的行升档（DA-1 的已知代价，见 §6 O-1）。
- **A1 C7=3**：证据计数升档对应 evidence aggregation / 多来源一致性这一类既有做法，不是新发明；本报告不声称它是。
- **A1 C8=4**：它打得过基线的地方有测试：`only_a1_reaches_strong_without_a_correction`、F-1 的对照测试、顺序无关性。代价是多存一份聚合逻辑。
- **A2 C3=4**：每条都能对着计数复核，扣 1 分是因为 `sample_band` 的阈值来源在文档里，不在句子里（用户看不到「为什么这条是 Strong」）。
- **A2 C4=4**：不吃正文；扣 1 分是因为输出仍然是关于第三人的陈述，`local_only` 的约束要靠调用方守。
- **A2 C5=4**：单日 1000 条、400 天冷却、时钟超前、只入不出、只出不入、只在群里，都有夹具。
- **A3 C1=1 / C4=1 / C3=1**：需要正文、无法复核、易临床化，见 §1。**C7=4**：LIWC 一类方法在文献里确实成熟——这正是它唯一的高分项，也不足以救它。**C8=1**：打不过 A0+A1。

C6 全部 5 分：A0 是 O(n) 单遍；A1 是 O(n·k)，k 为该轴存活行数，`contains` 走小 `Vec`，在轴级别的行数下比 HashMap 更省，且没有隐式全图操作；A2 的条目数是常数，成本随引用的证据行数线性。三者都没有隐式 O(n²) 全图操作。

---

## 5. 保留建议

给「最终只留 1 或 2 个」的仲裁一个明确的输入，而不是一份候选清单：

1. **档案侧留 A1，一个就够。** A1 是 A0 的严格超集：锁的语义逐条相同（有全夹具对照测试），只在没有锁的时候更保守（保留双方证据）和更有用（多条独立证据才升档）。A0 不需要单独保留为「另一个算法」，它是 A1 在单条证据下的退化形态——但 **A0 的语义必须作为回归测试保留**，它就是 A1 不许破坏的那份契约。
2. **A2 不应该占用第二个名额。** 它不产生特质轴，它把人脉图已经算出来的计数渲染成句子。真正的算法内容（band 阈值）是从 tie-strength 借来的（F-2）。建议把 A2 定位成 opus-a 选定的那个人脉算法的**输出层**，WP10 无 key 时的降级路径由它承担；这样最终形态仍然是「一个人脉算法 + 一个档案算法」，而 WP10 有实现、不占名额。
3. **A3 归档为不采用**，理由与代码里的拒绝函数一致，不需要再讨论。

如果 Round 2 认为 WP10 必须有独立的算法身份，那么第二个名额给 A2，人脉算法与档案算法二选一——但我不建议这样，因为 A2 一旦脱离人脉图的计数就没有输入。

---

## 6. 开放问题（交给 Round 2 / Round X）

- **O-1 同源证据的升档风险**：三条来自同一次导入、同一段对话的行，id 不同就会触发升档。可选缓解：要求三条至少落在不同的 `evidence_kind`，或不同的自然日。前者更可解释，后者更容易实现，都需要 `EvidenceRef` 加字段——本轮没做，因为这会改变共享类型定义。
- **O-2 分歧策略选哪个**：三种策略都在 `DisagreementPolicy` 里，测试给出了各自在同一夹具上的行为差异。建议用 gpt-sol-b 的对抗夹具再跑一轮再定。
- **O-3 F-2 的接口**：A2 应消费边上的 band 而不是重算，等 opus-a 的结论。
- **O-4 orphan 语义**：本轮把「遗忘」实现为重放时丢行，派生状态自动降级；协议里的 `orphaned` 状态位没有建模。若最终规范要求推断行显式标记 orphaned，`AxisOutcome.shadow` 是挂载点。
- **O-5 措辞闸的覆盖面**：三份词表是人工列的，只能挡已知的词。它挡不住「你最近似乎不太对劲」这类没有禁用词的句子。真正的保证来自 A2 只有四类固定句式，而不是来自词表；词表是第二道网。

---

## 7. 测试清单（72 条）

| 文件 | 条数 | 覆盖 |
|---|---|---|
| `tests/a0_lock.rs` | 13 | 空证据、问卷 Moderate、纠正 Strong+锁、**锁后推断不动轴**、shadow 保留被拒推断、更晚的纠正仍生效、纠正到 Unknown、**遗忘后回退**、遗忘纠正后解锁、全遗忘回 Unknown、跨轴隔离、五轴顺序、Debug 渲染无评分词 |
| `tests/a1_band.rs` | 22 | 空、1 条 Moderate、2 条仍 Moderate、**3 条独立同向 → Strong**、同一 id 三次不升档、三条 Weak 仍 Weak、自贴 Strong 封顶、**分歧 → Mixed/Weak**、严格策略不被计数推翻、Majority 策略、平票、显式 mixed、BandFloored 策略两例、纠正压过任意数量推断、纠正前的推断同样被拒、同向纠正全引用、遗忘后从 Strong 回落、Unknown 行丢弃、纠正到 Unknown、跨轴隔离、未触及轴保持 Unknown |
| `tests/a0_vs_a1.rs` | 5 | 全夹具上锁语义一致、无引用不落断言、**F-1 对照**、只有 A1 能不靠纠正到 Strong、A1 顺序无关而 A0 有关 |
| `tests/a2_personnel.rs` | 19 | **空 → 无条目**、无证据 → 无条目、每条都有证据、**群聊场景不出现私聊/单聊/私下**、见过一对一时不出群聊条目、互惠双计数、单向永不 Strong、单向方向正确、双向未确认、天数计算、当天文案、时钟超前、无最近接触、单日 1000 条、400 天冷却、条目顺序固定、确定性、不声称朋友/亲密/性格、推进 now 只改最近接触条 |
| `tests/a3_refusal.rs` | 6 | 五轴 × 八段正文（含注入式文本）恒拒绝、拒绝理由字符串、任意证据数量仍拒绝、批量形式拒绝、**任何输入都不产出 AxisState**、被否决 id 不会出现在任何在产状态上 |
| `tests/denylist_scan.rs` | 7 | 全部用户可见字符串（轴词汇 × 位置 + 全夹具的 A0/A1 描述 + 全夹具 ×3 个时间点的 A2 句子 + 算法 id + band/position 拼写）无诊断词、无评分词、组合闸一致、A2 无第三人声称、**闸门自检**（词表能抓到自己的词）、纯计数不被误杀、每条轴读作方向而非等级 |
