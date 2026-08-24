# PIPELINE_DEBT — 导入扇出与重复导入的 v0.1 处置规格（Round 2 / fable-b）

地位：本文件落实 R1-SYNTHESIS「下轮攻坚重点 #5：文档化导入扇出与去重，标为管道债，不是算法胜者的责任」。
它规定两件事：(1) 管道债本体与 v0.1 缓解方案（不改 PRODUCT_LOCK）；(2) `soul-algo` 与导入管道之间的接口契约，
使 C8 消融和 ALGO_FROZEN 不被管道缺陷污染。

---

## 0. 接口契约（先立规矩，再谈债）

**`soul-algo` 对它拿到的互动日志负责打分；日志本身的真实性归导入管道。Garbage in 是 importer 的事故，不是算法的。**

具体化为前置/后置条件：

### 算法侧输入前置条件（由管道保证，算法可信赖、不得自行补救）

| # | 前置条件 | 违反时的责任方 |
|---|---|---|
| PRE-1 | 全部 `occurred_at` 为 RFC 3339 UTC、`Z` 结尾（字典序=时间序；前 10 字符=UTC 日） | importer（DEFECTS P1-6） |
| PRE-2 | 同一条源消息对同一 peer 至多产生一条观察（去重已做） | importer（本文件 §2） |
| PRE-3 | `Venue` 标注可信：`Direct`=一对一会话，`Group`=多人会话；Group 观察可能是扇出副本（§1） | importer |
| PRE-4 | `self_contact_id` 恒为 owner，无自环（`build.rs` L193-203 已在管道侧强制） | graph build |
| PRE-5 | `as_of` 由调用方传入，= **store 内全部观察的最大 `occurred_at`**（不是单个导入文件的最大值，不是墙钟） | 调用方 |

PRE-5 特别说明：R1 fable-b DEFECTS P1-2 曾写「本次导出文件的最大时间戳」，fable-a CANDIDATE_SPEC 与
R1-SYNTHESIS 写「数据内 / 整个 store 的最大时间戳」。两者在「后导入旧文件」场景下分歧（文件极大值会倒退）。
**以 store 级最大值为准**（单调、确定、与导入顺序无关），文件级定义作废。此裁决须进冻结常量表。

### 算法侧输出后置条件（由 `soul-algo` 保证，管道与 UI 可信赖）

- 纯函数、确定性、不读墙钟、O(n)；band 只有三词；
- 每个档位可用一行整数算术复现（中文话术冻结归 fable-a）；
- one-sided 永不 Strong；group-only 永不 Strong（场合闩，Round 2 主线 T3/T3R/T4 共有）；
- 输出附带复核所需计数（见 §1.4 的 TieScore 分列要求）。

### 责任分派表（验收吵架时查这张表）

| 症状 | 责任方 | 修在哪 |
|---|---|---|
| 群聊 group-only 陌生人显示 Strong | 算法（场合闩缺失） | T 族胜者 |
| 群聊重度 + 单条一对一问候 → Strong | **算法与管道的组合缝**（见 §1.3，必须消融裁决） | T 族胜者 + 本文件 |
| 「一共 137 次往来」用户数不出来 | 管道（扇出计数进了展示语） | TieScore 分列 + 话术（§1.4） |
| 同一导出导两次，计数翻倍、档位升档 | 管道（无去重） | §2 |
| 跨时区消息切错 UTC 日 | 管道（归一化缺口） | DEFECTS P1-6，importer 边界 |
| rebuild 重置用户纠正 | 图服务（不是 band 公式） | GRAPH_CORRECTION.md |

---

## 1. 债 A：群消息扇出（一条主人发言 → N 条 outgoing InteractionRef）

### 1.1 现状机理（钉住行号，防止讨论漂移）

- `crates/soul-import/src/commit.rs` L198-207：owner 在群里的每条消息，对该会话**所有曾发言者**各生成一条
  `Direction::Outgoing` 观察；peer 的每条消息生成一条对 owner 的 `Incoming` 观察。
- `speakers_by_conversation`（同文件 L397-416）按**整个文件**汇总发言者、无时间约束：owner 发言之前退群、
  之后才入群的人同样计入。
- 该行为被测试 `crates/soul-import/tests/import_to_graph.rs::a_message_sent_to_a_group_is_evidence_of_contact_with_everyone_in_it`
  钉成了规范。
- 算术后果（R1 已证）：200 人活跃群、owner 发 10 条跨 3 个 UTC 日 → 每个发过言的成员
  outgoing=10、incoming≥1 → 在 T0 下全员 Strong（gpt-sol-b 故意失败探针 P_ALL_GROUP_STRONG_ATTEMPT；
  opus-a `group_only_50` 矩阵行）。

### 1.2 为什么 v0.1 保留扇出（而不是删掉它）

评估过的三个方向：

| 方案 | 判决 | 理由 |
|---|---|---|
| A. 删除扇出（owner 群消息不产生 per-peer 观察） | **否** | 群内互惠检测整体失效；group-only 关系退化为纯 incoming = one-sided，图上「同群的熟人」消失，制造 P1-5 同款盲区 |
| B. 扇出限窗（只对消息前后 N 天内发过言的人扇出） | **否（v0.1）** | 引入新超参 N，用户无法复核；不改变「计数不可数」的本质；实现复杂度全在 importer 热路径 |
| C. **保留扇出，改判定与展示语义**（本规格） | **是** | 数据零新增（`Venue` 已逐条在库）；算法层场合闩 + 展示分列即可关闭 Strong 爆炸；扇出保留为「联络存在性」信号 |

### 1.3 残余风险（算法层必须知道的两条，进消融夹具）

场合闩挡住了 group-only Strong，但**没有**挡住以下两条，必须在 Round 2 消融里显式裁决：

1. **单条私聊解锁 Strong**：若胜者的强档条件只是 `any_direct` 布尔闩（fable-a T3/T3R 现规范形），
   则「群聊重度扇出 + 双方各一条一对一问候」= `reciprocal ∧ any_direct ∧ count≥10 ∧ days≥3` → Strong。
   扇出灌的是 group 计数，闩只验证「存在过一次私聊」。gpt-sol-b ATTACK_SURFACE 的缓解方案
   （「band 只由去重后的 **direct** 事件计算」）是更强的形态。
   **要求 opus-a / gpt-sol-a / gpt-sol-b 增加夹具 `group_heavy_plus_one_direct_each_way`**：
   50 天群聊扇出（count≥100）+ 每方向各 1 条 Direct。期望：≤ Moderate。
   `any_direct` 布尔闩形态在此夹具上会给 Strong——若消融证实，胜者规范必须改为
   「Strong 的 count/days 门槛只消费 Direct 计数」。这是本文件对 T 族规范唯一的硬要求。
2. **全群 Moderate**：场合闩下 group-only 封顶 Moderate，但 200 人活跃群里人人 `reciprocal ∧ count≥3`
   → 全员 Moderate。v0.1 接受此残余（Moderate 语义=「有来往」，不算错档），但话术必须能把它说圆：
   「你们在群里有过 {group_count} 次同场往来」，不得说成一对一。

### 1.4 v0.1 缓解规格（不改 PRODUCT_LOCK）

PRODUCT_LOCK 对边只要求「互动强度/关系类型/最近接触/证据」，未规定计数语义，以下全部合法：

- **M-A1（TieScore 分列，算法接口）**：`TieStrength` 增加分列计数
  `direct_outgoing / direct_incoming / group_outgoing / group_incoming`（`tie_strength` 在
  `relationship.schema.json` 是自由 object，无契约变更；现有字段保留，`interaction_count` 仍=总和）。
  R1-SYNTHESIS 攻坚 #2 的「TieScore 增加 any_direct」由分列计数推导（`any_direct = direct_out+direct_in > 0`），
  比布尔更强且向后兼容。WP10 的 venue 句从「解析证据行找 Venue」改为纯读 TieScore，顺手消掉
  `analysis.rs::venue_point` 的按边扫证据。
- **M-A2（展示语义分列，话术冻结项，归 fable-a）**：任何用户可见计数语句必须分开说
  「一对一 X 次、群里同场 Y 次」；强/中/弱判定引用哪一列，话术必须与胜者规范一致。
  禁止再输出混合的 `interaction_count` 单数字。
- **M-A3（测试改名换义）**：`a_message_sent_to_a_group_is_evidence_of_contact_with_everyone_in_it`
  保留断言（扇出=联络存在性），但补充断言「这些观察 venue=Group」并在测试文档注明
  「group 观察不得单独支撑 Strong」——把规范从「钉死扇出喂档」改为「钉死扇出只作存在性」。
- **M-A4（性能债立案，不阻塞冻结）**：扇出行数 = Σ_群(owner 消息数 × 该群发言者数)。
  真实 Telegram（几个大群 × 数千条）会到 10⁶ 级 evidence 行，`rebuild` 全表载入（`build.rs` L191）
  与边上全量 `evidence_ids` 会先于算法成为瓶颈（R1-SYNTHESIS 性能节）。v0.1.x 走 DEFECTS P1-8
  的「代表性证据引用」；此处只立案，不展开。

### 1.5 验收测试（Given/When/Then）

| ID | Given | When | Then |
|---|---|---|---|
| PD-A1 | 200 发言者群，owner 发 10 条跨 3 UTC 日 | rebuild + 胜者判档 | 无任何 group-only 边 > Moderate |
| PD-A2 | 同上 + 某 peer 双向各 1 条 Direct | 判档 | 该边 ≤ Moderate（见 §1.3-1，消融裁决后钉死） |
| PD-A3 | 任意含群夹具 | 读 TieScore | direct/group 分列计数与手数消息一致；`interaction_count = 四列之和` |
| PD-A4 | 群夹具 | 渲染中文解释 | 出现「一对一 X 次」「群里同场 Y 次」两个独立数字，无混合总数单独示人 |

---

## 2. 债 B：重复导入加倍计数

### 2.1 现状机理

- `commit.rs` 模块文档 L22-24 自认：「Committing the same file twice writes the events twice. v0.1 has no
  external-id index to deduplicate against」。
- 两个解析器**都已产出**稳定源 id：Telegram `external_id = "{chat_id}:{message_id}"`（`telegram.rs` L177）、
  JSONL `external_id = message.id`（`soul_import_v1.rs` L174，schema 要求非空字符串）——但
  `StagedMessage.external_id` 在 commit 时被丢弃（`model.rs` L135-137 注释自认「Not stored」）。
- `rebuild` 幂等的是**边的条数**，不是计数：gpt-sol-b 探针证实重导 2 条互惠事件 → 4 条 → Weak 升 Moderate。
- Telegram 导出天然**累积**（新导出含全部旧消息），「定期重新导出」是主路径用户行为，不是边缘情况。

### 2.2 v0.1 缓解规格

**去重键**：`digest = sha256("soul.import.dedup.v1|" + source.as_str() + "|" + external_id)`。
沿用仓内既有的域分隔前缀先例（`soul.import.identifier.v1|`、`soul.graph.conversation.v1|`）；
哈希而非平台 id 明文入库，与 `conversation_ref` 同一隐私姿态。source 参与散列 ⇒ 两种格式的同名 id 不互撞。

**存放**：store 内部去重索引（SQLCipher 一张 `import_digests(digest PRIMARY KEY)` 表 / FakeStore 一个 `BTreeSet`），
通过 `EventStore` 新增两个 trait 方法暴露：

```rust
fn seen_import_digest(&self, digest: &Sha256Hex) -> StoreResult<bool>;
fn remember_import_digest(&mut self, digest: Sha256Hex) -> StoreResult<()>;
```

**不改**任何 `docs/schemas/` 冻结契约（digest 不进 event/evidence JSON 行）；conformance 套件补两个方法的断言。

**commit 行为**：逐消息先查 digest；命中则整条跳过（不 seal、不写 event、不写 evidence、不扇出），
`ImportReceipt` 增加 `duplicates_skipped: u64`；miss 则照常写入并 remember。
同一文件内部的重复 external_id 同样被折叠并计数（回执可见，不静默）；
对 `soul-import-v1`（我们自己的格式）建议解析器额外把文件内重复 id 报为 defect（SHOULD，非 MUST）。

**语义钉子**：

- **keep-first**：同 digest 的后到消息整条丢弃，即使正文不同（Telegram 编辑过的消息保持原 id）。
  图管道只消费计数与时间戳，正文差异不影响 band；v0.1 接受，写进文档即可。
- **与遗忘的交互**：digest 是无内容哈希（同审计「可保留孤立本地 UUID」先例），遗忘不清除它。
  副作用是期望的：遗忘某联系人后重导旧文件，**不会**复活其消息（重复被跳过，且 `resolve_contacts`
  本就保留 `forget_state`，`rebuild` 跳过 unresolved peer）。
- **算法零参与**：去重发生在 commit 之前，`soul-algo` 与 `soul-graph::rebuild` 不做任何 id 比对
  （接口契约 PRE-2）。禁止任何候选以「算法内去重」为卖点——那是把管道债搬进判定函数。

**审计**：现有 `ImportCommit` 条目的 `counts.items` 继续=实写 event 数；`duplicates_skipped > 0` 时
追加一条 `ImportCommit / Allowed` 审计，`counts.items = duplicates_skipped`，ReasonCode 复用现有词汇
（若审计 ReasonCode 枚举需要新变体 `DuplicateImport`，是一处冻结契约增补，交父代理拍板；回执字段不依赖它）。

### 2.3 验收测试（Given/When/Then）

| ID | Given | When | Then |
|---|---|---|---|
| PD-B1 | 已 commit 的 Telegram 夹具 | 同一文件再 commit + rebuild | 各边计数、band、active_days 与首次完全一致；`duplicates_skipped` = 首次 `events_written` 数 |
| PD-B2 | 旧导出已入库 | commit 含旧+新消息的累积导出 | 只有新消息计入；旧消息全数进 `duplicates_skipped` |
| PD-B3 | JSONL 与 Telegram 各含 external_id="7" 的消息 | 双双 commit | 互不去重（source 域分隔生效） |
| PD-B4 | 单文件内两行同 id | commit | 只落 1 条，`duplicates_skipped=1`，回执可见 |
| PD-B5 | 遗忘 peer 后重导原文件 | commit + rebuild | 该 peer 无新 evidence，图中不复活 |
| PD-B6 | gpt-sol-b 的 Duplicate-import 探针 | 在打补丁管道上重跑 | Weak 不再升 Moderate（探针转绿） |

---

## 3. 与 ALGO_FROZEN 的关系

- 本文件的**接口契约（§0）与 TieScore 分列（§1.4 M-A1）**必须进冻结规范：胜者的 Strong 门槛消费哪一列
  计数是算法定义的一部分，不写清则消融结果不可复现。
- **去重实现（§2）与扇出性能债（M-A4）不阻塞 ALGO_FROZEN**：消融夹具直接喂算法输入（绕过管道），
  结论与管道状态无关。它们阻塞的是 v0.1 端到端验收（AC 矩阵里的导入面）。
- 逐项去向见 ACCEPTANCE_GAP.md。
