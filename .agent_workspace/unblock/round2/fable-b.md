MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 2 复审（fable-b）— G2 代码对照 `apply_intake`；G3 规格完备性

只读复审，零产品 crate 改动。复审切点：**HEAD `814064e`**（`feat(graph): band ties with
the frozen T4D rule under one store-wide as_of`，复审进行中落地并已推送）。切点时工作区
另有**未提交的并发在途改动**：`soulcore/commands/graph.rs`（§1.4 视图三字段 + §3 两命令 +
`band_named`）、`soul-draft/analysis.rs`（GC-9a 抑制）、三个未跟踪测试文件
（`soul-graph/tests/graph_correction.rs`、`soulcore/tests/graph_correction_commands.rs`、
`soul-draft/tests/locked_tie_summary.rs`）与 `apps/desktop/src/core.ts`。本文对在途件的
描述以切点快照为准，读者核对时先 `git log`。

---

## 1. G2（`62840ff`）对照冻结 `apply_intake`（`soul-algo-trait/src/a0.rs`）

**结论：语义一致，测试全绿（`a0_lock` 19 通过、`correction_lock` 5 通过）。共享定义域上
逐点吻合；产品路径是 A0 定义域的真子集，收窄处全部发生在 A0 之前的校验层，不构成分歧。
两个可见性缺口（1.3），均已在 STATUS.md 诚实记录，建议 Round 3 收口。**

### 1.1 吻合矩阵

| A0 规范（`apply_intake` / `place`） | 产品（`soul-profile/src/service.rs::intake`） | 判定 |
|---|---|---|
| `place`：`state.locked_by_user && !row.locked_by_user` → `RefusedLocked` | 每条轴回答先查 `axis_is_locked(&profile, …)`（match guard，service.rs:287-298） | 等价。intake 只产非纠正行，且 `place_axis(…, locked_by_user: None)` 不会改锁位、问卷也锁不了轴，所以 run 内锁态恒定，逐条查与 A0 的逐行 fold 结果相同 |
| 被拒行仍进 `log_after`（「answer row is still written」） | recorder 在 apply 循环**之前**已写事件 + `Questionnaire` 证据行；被拒行的 evidence/event 都解得开（测试断言） | 一致 |
| `IntakeReport::ignored` 带 `IntakeSkip::AxisLockedByUser`，永不折叠进 applied | `IntakeOutcome::ignored` 带同名枚举；reason token `"axis_locked_by_user"` 两 crate 拼写一致，且**各自都有测试钉住**（a0_lock.rs:65、correction_lock.rs:249） | 一致，且防了两边字符串各自漂移 |
| 冻结默认 `WriteMode::LastWriteWins`：`place_axis` 整体替换 position/band/citations | G2 未动写入模式（commit message 明说）；没有顺手引入 `NoDowngrade` / A1 | 一致，遵守冻结 |
| 拒后 state 原样：仍 cite 纠正行、band 仍 Strong | 测试逐项断言（position、锁位、`evidence_band: Strong`、`evidence_ids` == 纠正行） | 一致 |

### 1.2 定义域边界（收窄，非分歧）

- **同轴两答一 run**：A0 明文定义（in-order fold，后者胜）；产品在
  `questionnaire::check` 就以 `DuplicateAnswer` 拒收，路径不可达。子集，成立。
- **`Position::Unknown` 的问卷行**：A0 `effective_band(Unknown) = Band::None`；产品的
  `LEANING` 选项表没有 `"unknown"`，recorder 以 `UnknownOption` 拒收，行落不了库。子集。
- **空问卷**：A0 返回空报告；产品返回 `ProfileError::EmptyQuestionnaire`。产品级约束，成立。

### 1.3 缺口（按重要度）

1. **可见性止步于 API 层（已知，WP03 遗留 8 后半）。** `IntakeSkip::as_str` 在 A0 文档里
   写明是「for the audit row」，产品侧它只被测试消费。审计链上 intake 只有 recorder 那条
   `ImportCommit`，`counts.items` = 全部回答数（含被拒的）；`soulcore::IntakeReceipt` 不读
   `ignored`（`answered = evidence_ids.len()` 同样把被拒的算进 answered）。**只看链或回执，
   分不清整卷生效与部分生效。** STATUS.md WP03 第 8 条已如实记账（「要让『我们保留了你的
   纠正』出现在屏幕或 /audit 上，得有人接 ignored」）。建议形状：`IntakeReceipt` 加法字段
   `ignored`（question_id + reason token，serde 加法），不加审计枚举——与被拒推断同款
   「存而不播」姿态，链上不用新动作。
2. **跨 crate 对齐无测试。** `soul-profile` 不依赖 `soul-algo-trait`（已核对 Cargo.toml），
   冻结侧的 `intake_matches_replay_on_every_fixture_and_mode` 只钉自己；产品 `intake` 与
   `apply_intake` 的一致性目前靠人肉对读（即本文）。tie 侧刚立了范本
   （`fa25c2e` → `soul-graph/tests/t4d_product.rs` 的先红后绿门），轴侧照抄：dev-dependency
   引入 `soul-algo-trait`，把 store 证据映射成 `EvidenceRef` 后断言五轴一致。**顺带能抓住
   1.4 那条真实分歧。**
3. **（小）`IntakeOutcome` 没有 `applied` 名单。** A0 报告有；产品的 `evidence_ids` 混着
   voice/prose 行，调用方无法只凭 outcome 算出「哪些回答动了轴」。可与第 1 条一并收口。

### 1.4 复审中发现的相邻分歧（越出 G2 diff，记录待裁）

- **`correct_axis(…, AxisPosition::Unknown)`**：产品无条件盖 `band_of(CORRECTION_STRENGTH)`
  = Strong；A0 有明文测试 `a_user_may_correct_an_axis_to_unknown`：纠正到 Unknown 锁轴但
  band = **None**（「unknown claims nothing」）。soulcore 的 `position_named` 已滤掉
  Unknown（commands/profile.rs:581-583），命令层不可达，纯 API 面分歧——但 2 号平价测试
  一旦落地它就会红，先裁后写。
- **intake 非原子（先于 G2 存在，非其引入）**：`recorder::record` 逐条写事件+证据，中途
  拒收（如上述 UnknownOption，经 `Answer::for_question(…, "unknown")` 可达）会让前几条已
  落库而 intake 返回 Err。A0 是纯函数不存在此态。记录在此，不算 G2 账。
- **提交卫生**：`62840ff` 捎带了并发的 soul-import G1+ 改动；`90da257` 的 message 已如实
  说明并补齐残件。无需处理。

---

## 2. G3 规格（`e58661c`，`unblock/round1/fable-b-G3.md`）完备性

**结论：事实性引用全部核对为真（2.1）。规格在本轮已被大量兑现——R1、R2、correct/release
四步、审计复用、GC-9a 抑制、视图三字段都按规格落地（2.2）；未兑现的 R3/R4/R5 中，R3 已经
构成落地代码里的一个真缺陷（纠正行会被下一次 rebuild 从 `edge.evidence_ids` 挤掉，且现有
测试恰好没盖到），R4/R5 仍开放（2.3）。规格自身有四处需要修订或补裁（2.4）。**

### 2.1 事实核对（全部属实，按规格出现顺序）

| 规格断言 | 核对 |
|---|---|
| `build.rs` 尾部无条件写 `Some(UserVerdict::Unreviewed)` | ✅ 规格时点（`faf60f0`）tie_inference:322 |
| `Tally` 本地 3/10/3 仍在、G1 在途 | ✅ 规格时点属实；`814064e` 已换 `soul_algo_tie::score` + 全库单一 `as_of` |
| `GraphError::UnreadableEdge` 为现有变体；`Store(StoreError)` 在、无 `NotFound` 变体 | ✅ error.rs 全文核对 |
| `TieStrength` 基线 8 字段 | ✅ 规格时点 model.rs:58-69 |
| `TieEdgeView.first/last_contact_utc: String` | ✅ soulcore graph.rs:228-229 |
| `recency_point` 读 `last_contact_utc`、找不到匹配行走 None | ✅ analysis.rs:344-359 |
| 「无第三个直接读者，view.rs 只透传」 | 基本属实：view.rs:60-62 确有一处把边的 `last_contact_utc` 摊到 `PersonNode`（节点字段本就是 Option），Option 化时是第三个触点，但改动确实平凡 |
| COPY_ZH 冻结 P5 原文、无 S9 变体 | ✅ COPY_ZH.md:71；「由你本人指定」不在任何已提交代码/fixture |
| WP10 在位措辞是自造的「往来不多/中等/密集」三档词 | ✅ analysis.rs::band_word:386-392（归档句本身也与冻结 P5 不同文：「这段**往来**归在「…」」vs「这段**关系**归在『…』」——A2 债认定准确，抑制与换模板两步分开是对的） |
| `relationship.schema.json`：`tie_strength` 自由 object、`evidence_ids` minItems 1 | ✅ docs/schemas/relationship.schema.json:20-25 |
| `user_verdict` 枚举含 `corrected` | ✅ docs/schemas/inference.schema.json:33 |
| `A2_STATEMENT_KEYS` 9 元含 `personnel.tie.filed_band` | ✅ a2.rs:71,80 |
| `correct_axis` 四步可对照 | ✅ service.rs:340-389 |

### 2.2 已按规格兑现（`814064e` + 切点在途件）

R1（锁定边 band=user_band、计数照更、machine_band=机器档——`tie_strength_of`）；
R2（`held_verdict` 非 Unreviewed 即保留——`tie_inference`）；解析失败拒绝而非猜
（`read_strength` → `UnreadableEdge`）；`correct.rs` 四步 + get 先行 + store 自己的
NotFound（GC-4）+ verdict 置 `Corrected`；release 当场生效、三锁字段清空、verdict 回
`Unreviewed`；审计复用 `ProfileCorrect`、`about=[relationship_id, evidence_id]`；证据行
三键（band/origin/relationship_id）无散文；`corrected_relationship` 谓词公开；命令层
thin 透传 + `band_named` 闭集；GC-9a 抑制分支连理由注释都与规格 §4 同构；零 schema 改动、
零新审计枚举、COPY_ZH 未动。实现与规格的对得上程度非常高。

### 2.3 未兑现（Round 3 攻坚位）

1. **R3 证据并集——已是落地缺陷，不只是缺功能。** rebuild 里
   `edge.evidence_ids = acc.evidence_ids`（仅互动行，build.rs:284/299）；
   `corrected_relationship` 在 build.rs 中**零调用**。时序：`correct_tie` 把纠正行 append
   进边（`rewritten`）→ 下一次 `rebuild` 整行重写，纠正行**从 cite 名单里消失**（证据表里
   还在，但 GC-5 的「在列」在第一次 rebuild 后就破了）。切点的
   `graph_correction.rs::the_correction_row_is_evidence_the_edge_cites_and_resolves`
   恰好**只在 correct 之后、rebuild 之前**断言——绿不代表对。Round 3 第一刀：先加
   「correct → rebuild → 仍 cite」的红测试，再在 rebuild 的 `list_evidence()` 单遍里按
   谓词归边（规格 R3 原文即可执行）。
2. **R4（遗忘压过锁、含 P2-6 陈旧未锁边）**：rebuild 仍只遍历有 tally 的 peer，任何
   existing edge 都不会被删。规格裁决仍开放。
3. **R5 + §1.1 Option 放宽**：落地 `TieStrength` 保持 `first/last_contact_utc: Timestamp`
   非 Option，也没有「锁定且无 tally 的存活边」补遍。GC-7 目前不可实现——要么按规格补
   Option 化，要么修订规格改走「墓碑时间戳」路线；这是裁决，不能默认。
4. GC 编号对照切点测试文件：GC-1/2/3/4/8/10 有对应测试（含 replay 同参重放）、GC-5 半盖
   （见上）、GC-6/7 无、GC-9a 在 `locked_tie_summary.rs`、GC-9b 按规格等 COPY_ZH。

### 2.4 规格自身需修订/补裁的四处

1. **§1.1 不变式与落地实现冲突（最要紧）。** 规格：「未锁时三字段全 None」。落地：
   `tie_strength_of` 对**每条**重建边无条件 `machine_band: Some(machine_band)`
   （build.rs:162），未锁边也带；视图上未锁边因此也有 `machine_band` token。另外 release
   把 `machine_band` 清成 None、下次 rebuild 又填回，与「rebuild 后未锁边恒 Some」不对称。
   实现的选择站得住（`machine_reading` 有回退，GC-10 逐字节幂等不受影响），但**规格不变式
   照原文写进测试会红**。裁一个，钉一个：建议改规格为「`locked ⟺ user_band.is_some()`；
   `machine_band` 是重建边的常在字段」，并在 graph_correction.rs 补断言。
2. **§1.1 struct 草图未与 fable-a T4D_WIRING 合成。** 落地 TieStrength 还有 8 个 T4D 字段
   （direct/group 四计数、direct_active_day_count、last_direct_contact_utc、silent_days、
   as_of_utc、algorithm_id）；「序列化输出与今天逐字节相同」的说法随之过期。两规格幸而
   无冲突，但 R5 的「计数 = 0」应明说覆盖全部 13 个计数字段 + `last_direct_contact_utc = None`。
3. **§5 负向断言按原文不可执行。** 「全仓库（含测试 fixture）grep 不得出现『由你本人
   指定』」——切点在途测试正是用该字面量做反向断言（`locked_tie_summary.rs:42` 的
   `UNFROZEN_VARIANT` 常量、`graph_correction_commands.rs:134`），repo 级 grep 门会被
   自己的执法者触发。修订为「产品 crate 的**非测试**源码与 fixtures 不得出现」，或要求
   门规则排除断言处。
4. **未裁的两个边角**：release 未锁边（落地选择：允许、记 release 行、band 原地）；
   correct 成与机器同档（落地：照锁，`overrides_machine() == false`）。两者行为都合理，
   但都该有一行裁决与一条测试，免得下轮被当 bug「修」掉。

---

## 3. Round 3 建议（按序）

1. R3 红测试 + rebuild 归边（2.3-1，规格 R3 原文可直接执行）。
2. §1.1 不变式改判 + 钉测（2.4-1）。
3. R4/R5 与 P2-6 一并裁决落地（2.3-2/3；Option 化 vs 非 Option 先裁）。
4. `IntakeReceipt` 接 `ignored`（1.3-1，加法字段，不动审计枚举）。
5. 轴侧平价测试：`soul-profile` dev-dep `soul-algo-trait`，仿 `t4d_product.rs`
   （1.3-2）；随手裁 `correct_axis(Unknown)` 的 band（1.4）。
6. GC-6/7 测试随 R4/R5 落；负向断言按 2.4-3 的修订形式落。
