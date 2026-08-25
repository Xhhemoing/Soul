MODEL_SLUG: claude-fable-5-thinking-xhigh

# G3 — 图纠正/锁定规格（Goal 1 unblock · Round 1 · fable-b）

地位：落实 PROGRESS「G3：rebuild preserves `user_verdict`; locked effective band」。
承接 `.agent_workspace/round2/fable-b/GRAPH_CORRECTION.md`（在场，仍为基础规格），按三件冻结后事实收窄：

1. **ALGO_FROZEN**（`docs/algorithms/DECISION.md`）：T4D 是唯一权威判档；机器档从此指
   「冻结算法在全库单一 `as_of` 下的输出」，不再指 `build.rs` 本地 3/10/3（后者是过渡遗留，见 DECISION §6.5）。
2. **COPY_ZH 已冻结且不含锁定边话术**（`docs/algorithms/COPY_ZH.md`）：Round 2 §4 草拟、Round 3
   SOTA_ACCEPT 点名的 S9「由你本人指定」变体**没有进冻结稿**。冻结 P5 是
   「按上面的计数，这段关系归在『{强/中等/弱}』一档；这是工作假设，不是对这个人的判断。」——
   在锁定边上两处为假（档位不来自计数；档位不是假设，是用户裁决）。**GC-9 由此拆分**（§4、§5）。
3. **本分支现状**（`cursor/goal1-unblock-a073`）：`soul-graph/src/build.rs` 仍整行覆写
   `tie_strength` 并把 `user_verdict` 复位为 `Some(Unreviewed)`（`tie_inference()` 尾部）；
   `t4d_adapt.rs`（InteractionInterner）已落地而判档替换（G1）在途——本规格与 G1 **任意先后落地均成立**（§2 R1）。

修复对象不变：PRODUCT_LOCK 灵魂层「人脉图……用户**可查看和纠正**」+ 不可协商约束 5
（「档案可查看、纠正……」）与 10（「不做不可纠正黑盒」）。设计原则不变：与 `correct_axis` 四步对称、
零冻结契约变更、现有消费者零改动即尊重纠正（band=生效档）。

**本轮（Round 1 unblock）只交本文档；产品 crate 零逻辑改动。** 以下各节是给本分支后续轮次的实现规范。

---

## 1. 字段

### 1.1 `TieStrength`（`crates/soul-graph/src/model.rs`）——三个锁字段 + 一处 Option 放宽

```rust
pub struct TieStrength {
    pub band: SupportedBand,              // 恒为生效档：锁定=用户档，未锁=机器档
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    pub conversation_count: u64,
    pub active_day_count: u64,
    // GC-7 放宽：Option。有 tally 的边恒 Some（序列化输出与今天逐字节相同）；
    // 仅「锁定且互动证据全被遗忘」的存活边为 None（§2 R5）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_contact_utc: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_contact_utc: Option<Timestamp>,
    // 三个锁字段。未锁的边全缺省，旧行无损读回，schema 零改动
    // （relationship.schema.json 的 tie_strength 是自由 object，schemas.lock.json 不动）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locked_by_user: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_band: Option<SupportedBand>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_band: Option<SupportedBand>,
}
```

不变式（进测试，不进文档就会烂）：

- `locked_by_user == Some(true)` ⟺ `user_band.is_some()` ⟺ `machine_band.is_some()`；
- 锁定时 `band == user_band.unwrap()`；未锁时三字段全 `None` 且 `band` = 机器档；
- `first/last_contact_utc.is_none()` 仅当 `interaction_count == 0`（GC-7 形态）。

**Option 放宽的理由与代价**：GC-7 的存活边计数清零后，时间戳是「从已销毁证据派生的断言」，
与计数同罪，必须一并消失——保留旧值等于让遗忘留下可读残影。代价是两个消费者要接 `Option`：
`soulcore/commands/graph.rs::TieEdgeView`（`first/last_contact_utc: String` → `Option<String>`）
与 `soul-draft/src/analysis.rs::recency_point`（读不到 last contact 时按现有「找不到匹配行」路径返回
`None`，无新分支）。除此之外没有第三个直接读者（`view.rs` 只透传）。
**拒绝的替代**：给 GC-7 边填纠正时刻或保留陈旧时间戳——两者都是无证据支撑的时间断言。

### 1.2 `TieCorrection`（`crates/soul-graph/src/correct.rs`，新文件）

```rust
pub struct TieCorrection {
    pub relationship_id: Uuid,
    pub evidence_id: Uuid,           // 本次 UserCorrection 证据行
    pub band: SupportedBand,         // 纠正后的生效档（release 时=机器档）
    pub machine_band: SupportedBand, // 机器按冻结算法的档，供 UI 并排展示
}
```

### 1.3 推断与证据——零新增

- inference 侧只用既有物：`user_verdict = Some(UserVerdict::Corrected)`（枚举已在
  `inference.schema.json`）；`statement_key`/`evidence_band` 继续跟机器输出走（存而不用）。
- 证据行按 Round 2 §1 第 2 步原样：`kind: UserCorrection`，`subject: Owner`，
  `strength: CORRECTION_STRENGTH`（复用 `soul-profile` 的 Strong 常量语义），
  `source_refs: [{ "origin": "graph_correction" | "graph_correction_release",
  "relationship_id": …, "band": "strong|moderate|weak" }]`，`exportable_to_research: false`，
  privacy `local_only`。行内只有 id 与档位词，无第三人姓名/正文。

### 1.4 视图（`soulcore/commands/graph.rs::TieEdgeView`）——加法字段，只给词表 token

```text
locked_by_user: bool                  # 缺省 false
user_band:      Option<String>        # "strong|moderate|weak"，锁定时 Some
machine_band:   Option<String>        # 同上
```

`band` 字段语义不变（生效档），所以图 view、WP10、起草语气等现有读者零改动即尊重纠正——
这是 Round 2 §2 的核心决定，本规格照抄不改。视图只输出 token，不输出中文句（§4）。

---

## 2. rebuild 防 clobber 规则（`crates/soul-graph/src/build.rs`）

匹配逻辑不变：边按 `connects(edge, owner, peer)`，推断按 `is_tie_statement_about`
（`TIE_STATEMENT_PREFIX` + target.relationship_id）。以下按现行符号写成伪码：

```text
existing_edge   = existing_edges 中 connects 匹配者（现状逻辑）
computed        = 机器档与全部计数
                  # G1 落地前 = Tally::strength()（T0 遗留，非规范）
                  # G1 落地后 = soul_algo_tie::T4D::score(peer, log, as_of)，全库单一 as_of
locked          = existing_edge 解析出的 tie_strength.locked_by_user == Some(true)
                  # tie_strength 存在但解析失败 → GraphError::UnreadableEdge（现有变体），拒绝而非猜

R1 锁定边：edge.tie_strength =
      computed 的全部计数/时间戳（照常更新——锁的是档，不是计数）
      + band         = existing.user_band
      + locked_by_user/user_band 原样保留
      + machine_band = computed.band
   未锁：edge.tie_strength = computed，三个锁字段与 GC-7 之外的 Option 均缺省。
   锁语义只消费 computed.band，不关心谁算的 → 与 G1 判档替换互不阻塞，任意先后落地。

R2 verdict 保留：inference 已存在 → user_verdict 保留原值
      （不只 Corrected：Accepted/Rejected 同样不得复位——今天的 tie_inference() 尾部
       `Some(UserVerdict::Unreviewed)` 即 clobber 本体，改为仅新建时写 Unreviewed）。
   statement_key / evidence_band 照机器输出重写（机器的看法持续更新，存而不用）。

R3 证据并集：edge.evidence_ids = 存活互动证据 ∪ {该边全部 graph_correction(+release) 证据行}
      在 rebuild 已有的 list_evidence() 单遍扫描里顺带收集
      （kind == UserCorrection 且 source_refs[0].origin 以 "graph_correction" 起头，
       按 relationship_id 归边）。纠正行本身是证据，必须可经 resolve_evidence 解引用（GC-5）。

R4 遗忘压过锁：peer 联系人被遗忘/缺席 → 该边删除（或按 store 裁决墓碑），锁不豁免；
      「存在于库、不在 tally」的未锁陈旧边同规则（P2-6 的裁决落在这里，与锁无关但同一补丁位）。
      对齐「审计不得阻止遗忘」的姿态：任何用户造物都不豁免遗忘。

R5 支撑清零但联系人仍在（仅锁定边）：互动证据全被遗忘、tally 缺席但 peer contact 在 →
      边保留：全部计数 = 0，first/last_contact_utc = None（§1.1），
      evidence_ids = [graph_correction 证据行]（非空，满足 minItems:1 且可解引用），
      machine_band = Weak（冻结规则：空观测 → Weak，不进时间运算），band = user_band。
      注意：现行主循环只遍历 tallies，需补一遍「锁定且无 tally 的 existing_edges」。
      未锁的无 tally 边走 R4（删除），不走 R5。

R6 幂等：无新导入时 rebuild ×2，relationship 行与 inference 行的规范序列化逐字节相同，
      锁定与否皆然。审计链除外——它是只追加的，本来每次 rebuild 各记一条。
```

`release_tie` 立即生效，不等下次 rebuild：写 `graph_correction_release` 证据行；
`locked_by_user`/`user_band`/`machine_band` 三字段清空，`band` = 原 `machine_band`；
inference `user_verdict` 置回 `Unreviewed`。锁必须有回头路（对称于 DEFECTS P2-4 对轴侧的同款要求）。

`correct_tie` 四步与 Round 2 §1 一致，仅一处收紧：**第 1 步 `get_relationship` 先行**，
不存在即返回错误且 store 零写入（GC-4）。错误面用现有 `GraphError::Store(StoreError::NotFound)`
——Round 2 假设的 `GraphError::NotFound` 变体不存在，不为此加枚举。
v0.1 仍不支持凭空造边、不改计数、不纠正 `types`、不合并联系人、无逐条 accept/reject（Round 2 §5 全保留）。

---

## 3. soulcore 命令（`crates/soulcore/src/commands/graph.rs`）

照 `commands/profile.rs::correct_axis` 的样子透传，thin wiring 无逻辑；
时钟一律 `at_unix_seconds` 入参，replay 可复现：

```rust
pub fn correct_tie(
    store: &mut SqlCipherStore,
    relationship_id: Uuid,
    band: SupportedBand,
    at_unix_seconds: i64,
) -> Result<TieCorrection, GraphError>;

pub fn release_tie(
    store: &mut SqlCipherStore,
    relationship_id: Uuid,
    at_unix_seconds: i64,
) -> Result<TieCorrection, GraphError>;

/// 闭集解析，对称于 position_named：只认 "strong|moderate|weak"，其余 None。
pub fn band_named(key: &str) -> Option<SupportedBand>;
```

- 审计：复用 `AuditAction::ProfileCorrect / Allowed`，`about: [relationship_id, evidence_id]`
  （对称于轴侧 `[profile_id, axis_id, evidence_id]`；图侧无 profile_id）。语义站得住：
  「用户纠正了灵魂层的一个工作假设」。独立动作 `GraphCorrect` 仍是父代理 DECISIONS 的一行增补项，
  本规格不依赖它——依赖它就要动 `audit.schema.json` 枚举，那是冻结契约。
- 回执：`TieCorrection` 按 §1.2 序列化输出 id 与档位 token。**本分支命令层不产任何新中文句**
  ——纠正回执、锁定边展示、解除回执的四句话术全部是 COPY_ZH 治下的用户可见文案（§4），
  冻结稿里没有，就先不说话，让 UI 用 token + 既有冻结句拼画面。

---

## 4. GC-9 与 COPY_ZH 的边界（本规格相对 Round 2 的最大差分）

**事实**：COPY_ZH.md 已随 ALGO_FROZEN 冻结，头部规则「改模板先改本文件并留痕」；
其 §4 P5（`personnel.tie.filed_band` 对应句）是唯一归档句，而 Round 2 §4 草拟的
「这一档由你本人指定」变体（Round 3 SOTA_ACCEPT 编号 S9）**未进冻结稿**。

**规则（本分支约束）**：锁定边上**禁止渲染冻结 P5**——「按上面的计数」与「这是工作假设」
在用户裁决过的档位上都是假话；同时**禁止自造未冻结句**——那是绕开冻结留痕的第二话术源。
两个禁止叠加的唯一合法输出是：**锁定边的人事摘要不出归档句**，直到 COPY_ZH 以加法拿到新 key。

落点（两个渲染世代都覆盖，与 A2 合并债互不阻塞）：

- 在位 WP10（`soul-draft/src/analysis.rs::points_for`）：边锁定 → 不 push 末尾归档点。
  该函数现有的自造 P5 措辞（「往来不多/中等/密集」）本身是 A2 债（PROGRESS A2 行，R3 归属），
  抑制规则对旧措辞同样生效——先抑制，后换冻结模板，两步独立。
- A2 合并后（`soul-algo-trait::a2`）：冻结 crate **不动**；Goal 1 侧的 edge→`TieScore` 适配器
  在锁定边上过滤 `personnel.tie.filed_band` bullet。摘要其余句子（P1/P1b/P2/P3/P4）全是计数事实，
  与锁无关，照常渲染；摘要级 notice（工作假设，非临床结论）覆盖的是这些计数句，保留。
- 信息不丢：图 view 的 `band` + `locked_by_user` + `machine_band`（§1.4）仍把「档是多少、
  谁定的、机器怎么看」三件事全量交给 UI——被抑制的只是那句没有合法文案的中文。

**解锁条件（COPY_ZH DECISIONS）**：fable-a（话术冻结槽位）经父代理 DECISIONS 留痕，向 COPY_ZH
加法新增：① `personnel.tie.filed_band_user_set` 归档变体（Round 2 §4 草案句可作底稿，
须过 `assert_non_clinical`、遵守词汇单源三档词与禁词表）；② 纠正/解除/锁定展示三句回执文案。
届时 GC-9b（§5）转为可实现，A2 侧以加法 statement key（`A2_STATEMENT_KEYS` 追加一元，
算法号 v3→v4）落地，遵循 Round 3「widened by exactly two optional fields」同款加法纪律。

---

## 5. 验收测试 GC-1..GC-10：本分支可实现 vs 需 COPY_ZH DECISIONS

Given/When/Then 沿 Round 2 §6，逐条标注可实现性。落点：`crates/soul-graph/tests/graph_correction.rs`
（GC-1..7、10）、`crates/soulcore/tests/`（GC-8 命令+审计）、`crates/soul-draft/tests/`（GC-9a）。

| ID | Given / When / Then（差分后） | 本分支 | 依赖 |
|---|---|---|---|
| GC-1 | 机器判 Strong 的边；`correct_tie(Moderate)` → rebuild；`band=Moderate`、`machine_band=Strong`、`user_verdict=Corrected` 全保留 | **可实现** | §2 R1+R2 |
| GC-2 | GC-1 后再导入 20 条 → rebuild；计数增长、`machine_band` 按冻结算法重算、`band` 仍 Moderate | **可实现** | §2 R1（G1 前用遗留档、G1 后用 T4D，断言写「= 冻结评分器输出」不写数字） |
| GC-3 | 锁定边 `release_tie` → rebuild；`band`=机器档、锁字段清空、verdict 回 Unreviewed；且 release 当场生效不等 rebuild | **可实现** | §2 |
| GC-4 | 不存在的 relationship_id；`correct_tie`；`StoreError::NotFound` 上浮且 store 零写入 | **可实现** | §2 get 先行 |
| GC-5 | 纠正后的边；`edge_evidence()`；纠正证据行在列且解引用成功 | **可实现** | §2 R3 |
| GC-6 | 锁定边的 peer 被遗忘 → rebuild；图中无此边（遗忘压过锁） | **可实现** | §2 R4（前置：P2-6 陈旧边裁决与本条同一补丁位） |
| GC-7 | 锁定边全部互动证据被遗忘 → rebuild；边存活：计数 0、时间戳 None、`evidence_ids=[纠正行]`、`machine_band=Weak`、`band=user_band` | **可实现** | §1.1 Option 放宽 + §2 R5 |
| GC-8 | 任意纠正/解锁；读审计；`ProfileCorrect` 条目 `about` 含 [relationship_id, evidence_id]；同 `at_unix_seconds` replay 产出相同条目内容 | **可实现** | §3（不加审计枚举） |
| GC-9a | 锁定边；WP10 人事摘要；**归档句不渲染**（冻结 P5 原文与任何自造变体都不得出现）；其余计数句照常；全程无第二套阈值；图 view 三字段照 §1.4 可见 | **可实现** | §4 抑制规则 |
| GC-9b | 锁定边；WP10 人事摘要；渲染 COPY_ZH 新增的「由你本人指定」归档变体，cite 纠正证据 id | **不可实现，需 COPY_ZH DECISIONS**（fable-a 加法 key + 父代理留痕；A2 侧 v4 加法 statement key） | §4 解锁条件 |
| GC-10 | 无新导入 rebuild ×2；relationship 行与 inference 行规范序列化逐字节相同（锁定与否皆然）；审计链除外 | **可实现** | §2 R6 |

计 10 条中 9.5 条本分支可落（GC-9 拆半）；唯一被冻结话术挡住的是 GC-9b 的**那一句中文**，
存储、rebuild、命令、审计、抑制全部不等它。

同批必附的负向断言（防倒退，随 GC-9a 落）：全仓库（含测试 fixture）grep 不得出现
「由你本人指定」及其它未冻结归档变体句——话术进代码之前必须先进 COPY_ZH。

---

## 6. 体量核算与实现顺位（证明仍是 v0.1-small）

零 schema 改动、零新依赖、零新存储表、冻结 crate（soul-algo-tie / soul-algo-trait）零改动。
新增/补丁面：`soul-graph` 新模块 `correct.rs`（~150 LOC）、`model.rs` 五个字段位（三锁 + 两 Option 化）、
`build.rs` 每边写入处 ~40 LOC（R1..R5）、`soulcore/commands/graph.rs` 两个透传 + `band_named` +
view 三字段、`soul-draft/analysis.rs` 一处抑制分支，以及 §5 的测试。
与 `correct_axis` 的逐行对称性保留（评审可对照 `soul-profile/src/service.rs::correct_axis`）。

实现顺位建议（后续轮次）：R2（verdict 保留）与 R1（生效档）先行——它们是 PROGRESS G3 行的字面义
且不依赖任何裁决；随后 correct/release + GC-1..5、8、10；再 R4/R5 + GC-6/7（吃掉 P2-6）；
GC-9a 抑制随 WP10 触点走；GC-9b 等 fable-a 的 COPY_ZH 加法。

本轮工作区并发说明：撰写本规格期间 `soul-profile`/`soul-import` 出现同分支在途改动（G2 轴锁语义，
归属另一 Round 1 槽位），与本规格无交集；本文引用代码一律以符号名与提交 `7aee812` 时点为准。
