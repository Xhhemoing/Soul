# GRAPH_CORRECTION — 人脉图纠正/锁定最小 API（Round 2 / fable-b）

地位：落实 R1-SYNTHESIS P0-2「人脉图不可纠正」的接口草案（Round 3 出验收话术）。
修复对象：PRODUCT_LOCK 灵魂层表格「人脉图…用户**可查看和纠正**」+ 不可协商约束 5，
而 Goal 1 现状是 `build.rs` L322 每次 rebuild 把 tie inference 的 `user_verdict` 复位为
`Some(UserVerdict::Unreviewed)`、L246-251 整行覆写 `tie_strength`，且不存在任何纠正入口。

设计原则：**与轴侧纠正对称**（`correct_axis` 的四步：UserCorrection 证据 → 状态更新 → 锁标记 → 审计），
**零冻结契约变更**（`relationship.schema.json` 的 `tie_strength` 是自由 object；`UserVerdict::Corrected`
枚举已存在于 `inference.schema.json`），**现有消费者零改动**（见 §2 的 band 字段语义选择）。

---

## 1. API 面（v0.1 只有两个函数 + 命令层透传）

```rust
// crates/soul-graph/src/correct.rs（新文件，~150 LOC）

/// 用户把一条边的档位定死。机器的看法照旧计算、存而不用。
pub fn correct_tie<S>(
    store: &mut S,
    relationship_id: Uuid,
    band: SupportedBand,
    at_unix_seconds: i64,          // 调用方时钟，与 correct_axis 同款，replay 可复现
) -> GraphResult<TieCorrection>
where
    S: GraphStore + ProfileStore + AuditLog;

/// 解除锁定，恢复「让计数说话」。
pub fn release_tie<S>(
    store: &mut S,
    relationship_id: Uuid,
    at_unix_seconds: i64,
) -> GraphResult<TieCorrection>
where
    S: GraphStore + ProfileStore + AuditLog;

pub struct TieCorrection {
    pub relationship_id: Uuid,
    pub evidence_id: Uuid,         // 本次 UserCorrection 证据行
    pub band: SupportedBand,       // 纠正后的生效档（release 时=机器档）
    pub machine_band: SupportedBand, // 机器按计数的档，供 UI 并排展示
}
```

`soulcore` 命令层照 `soulcore_profile.rs::correct_axis` 的样子透传两个函数（thin wiring，无逻辑）。

**`correct_tie` 的四步**（与 `profile_service.rs::correct_axis` 逐步对应）：

1. `get_relationship(relationship_id)`——不存在即 `GraphError::NotFound`。v0.1 **不支持**凭空造边
   （没有观察的边违反「无证据不落库」；用户想记录图外关系走 v0.2 的 user-stated 节点，不在本 API）。
2. 写一条 `UserCorrection` 证据：`kind: UserCorrection`，`subject: Owner`（这是 owner 对**自己关系**的
   裁决，行内只有 id 与 band 词，无第三人姓名/正文），`strength: Strong`（复用 `CORRECTION_STRENGTH`），
   `source_refs: [{ "origin": "graph_correction", "relationship_id": …, "band": "strong|moderate|weak" }]`，
   `exportable_to_research: false`，privacy `local_only`。
3. 更新边：`tie_strength` 增加 `locked_by_user: true`、`user_band: band`、`machine_band: <原 band>`，
   并把 `band` 字段置为用户档（§2）；把本次 evidence_id 追加进边的 `evidence_ids`。
4. 更新该边的 tie inference（按 `is_tie_statement_about` 定位）：`user_verdict = Some(Corrected)`。
   `statement_key` 与 `evidence_ids` 不动——那是机器的断言及其支撑，「存而不用」，与轴侧
   `RefusedAxisLocked` 同一句式。
5. 审计：复用 `AuditAction::ProfileCorrect / Allowed`，`about: [relationship_id, evidence_id]`。
   语义站得住（「用户纠正了灵魂层的一个工作假设」）；若父代理认为图侧应有独立动作
   `GraphCorrect`，那是 audit schema 枚举的一行增补，本规格不依赖它。

**`release_tie`**：写 `{ "origin": "graph_correction_release" }` 的 UserCorrection 证据；
`locked_by_user = false`，移除 `user_band`，`band = machine_band`（立即生效，不等下次 rebuild）；
inference 的 `user_verdict` 置回 `Unreviewed`；审计同上。锁必须有回头路，否则支持成本全落在「重装重导」。

---

## 2. band 字段语义（本规格最重要的一个决定）

`TieStrength.band` 恒为**生效档**（锁定时=用户档，未锁时=机器档）；机器档另存 `machine_band`。

选这个方向而不是「band 恒机器、user_band 另读」的理由：**全部现有消费者零改动即自动尊重纠正**——
WP10 `analysis.rs` L244 读 `edge.tie_strength.band`、图 view、起草语气选择，全都不用碰。
反向设计则要求每个消费者记得「先查锁再选字段」，漏一处就是锁形同虚设，而这正是 R1 认定
「装饰性 user_verdict 比缺失更糟」的翻版。

Rust 侧 `TieStrength` 增加：

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub locked_by_user: Option<bool>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub user_band: Option<SupportedBand>,
#[serde(default, skip_serializing_if = "Option::is_none")]
pub machine_band: Option<SupportedBand>,
```

未锁的边三个字段全缺省，序列化输出与今天完全一致（旧行可无损读回）。schema 无需改动
（`tie_strength: { "type": "object" }`）。

---

## 3. rebuild 语义（防 clobber 的核心）

`build.rs` 的每边写入改为（伪码）：

```text
existing_edge  = 按 pair 匹配（现状逻辑）
computed       = tally.strength()                    # 机器档与全部计数
locked         = existing_edge.tie_strength.locked_by_user == true

edge.tie_strength =
  if locked:  computed 的全部计数/时间戳 + band = existing.user_band
              + user_band/locked_by_user 原样保留 + machine_band = computed.band
  else:       computed（三个可选字段缺省）

edge.evidence_ids = 存活的互动证据 ∪ {全部 graph_correction 证据}   # 纠正行本身是证据，须可解引用

inference.user_verdict =
  if 既有 inference 存在: 保留其原值        # 不只 Corrected：Accepted/Rejected 同样不得复位
  else: Some(Unreviewed)
inference.statement_key = 机器档照写        # 机器的看法持续更新，存而不用
```

边界规则：

- **R1 锁定边计数照更新**：锁定的是 band，不是计数。导入新消息后 count/days/last_contact 正常增长，
  机器档正常重算进 `machine_band`。用户看到的解释是：「档位是你定的；计数是真实往来，照常累计。」
- **R2 遗忘压过锁**（对齐「审计不得阻止遗忘」的姿态）：peer 联系人被遗忘 → 边按遗忘流程
  删除/墓碑，锁不豁免。需要补的正是 DEFECTS P2-6 的「遗忘→rebuild→图中无此人」测试。
- **R3 支撑清零但联系人仍在**：互动证据全部被遗忘、锁定边失去 tally → 边保留，计数清零，
  `evidence_ids = [graph_correction 证据]`（非空，满足 schema `minItems: 1`，且可解引用——
  纠正本身就是这条边此刻唯一的证据），`machine_band = Weak`，`band = user_band`。
  未锁定的无 tally 边照 P2-6 的裁决处理（删除或墓碑，归 Round 3）。
- **R4 幂等**：连续两次 rebuild（无新导入）输出逐字节相同，锁定与否皆然。

---

## 4. 中文话术（草案，冻结权归 fable-a；全部须过 `assert_non_clinical`）

| 场景 | 句子 |
|---|---|
| 纠正成功回执 | 「已把这条关系定为「{band}」。之后的导入和重建不会改动它；机器按计数得到的档位会放在旁边，仅供参考。」 |
| 锁定边展示 | 「这一档是你亲手定的；按你们的往来计数，机器现在会给「{machine_band}」。」 |
| WP10 摘要 P5 变体 | 「这段往来归在「{band}」一档——这一档由你本人指定。」（cite 纠正证据 id） |
| 解除锁定回执 | 「已恢复为按计数判定，现在是「{machine_band}」。」 |

词汇约束沿用 R1-SYNTHESIS 攻坚 #4：只谈次数/天数/是否私聊/是否互惠 + 三档词，禁 score/百分位/诊断词。

---

## 5. 明确不做（v0.1-small 的边界）

- 凭空创建边、手改计数/时间戳、纠正 `types`（自由数组已给 v0.2 的 user-stated 关系类型留位，
  见 `graph_model.rs` L22-26 注释，但不在本 API）；
- 合并联系人（跨导出同人合并是另一张图纠正票，v0.2）；
- 逐条证据的 accept/reject（`UserVerdict::Accepted/Rejected` 在 v0.1 无入口，保留枚举不实现）；
- UI 形态（本规格只到 soulcore 命令层）。

---

## 6. 验收测试（Given/When/Then）

| ID | Given | When | Then |
|---|---|---|---|
| GC-1 | 机器判 Strong 的边 | `correct_tie(Moderate)` → rebuild | `band=Moderate`、`machine_band=Strong`、`user_verdict=Corrected` 全部保留 |
| GC-2 | GC-1 之后 | 再导入 20 条新消息 → rebuild | 计数增长、`machine_band` 重算；`band` 仍= Moderate |
| GC-3 | 锁定边 | `release_tie` → rebuild | `band` = 机器档；user 字段清空；verdict 回 Unreviewed |
| GC-4 | 不存在的 relationship_id | `correct_tie` | `NotFound` 错误，store 无任何写入 |
| GC-5 | 纠正后的边 | `edge_evidence()` | 纠正证据行在列且解引用成功 |
| GC-6 | 锁定边的 peer | 遗忘该联系人 → rebuild | 图中无此边（遗忘压过锁） |
| GC-7 | 锁定边的全部互动证据被遗忘 | rebuild | 边存活：计数 0、`evidence_ids=[纠正行]`、`band=user_band` |
| GC-8 | 任意纠正/解锁 | 读审计 | `ProfileCorrect` 条目 about 含 relationship_id 与 evidence_id；同参数 replay 产出相同条目 |
| GC-9 | 锁定边 | WP10 人事摘要 | P5 句用用户档 + 「由你本人指定」变体；无第二套阈值参与 |
| GC-10 | 无新导入 | rebuild ×2 | 两次输出逐字节相同（R4） |

---

## 7. 体量核算（证明 v0.1-small）

零 schema 破坏性变更；新增：`soul-graph` 一个 ~150 LOC 模块 + `TieStrength` 三个可选字段 +
`build.rs` 每边写入处 ~30 LOC 补丁 + soulcore 两个透传命令 + 上表 10 条测试。
无新依赖、无新存储表、无 UI 承诺。与轴侧 `correct_axis` 的对称性意味着评审可以逐行对照既有实现。
