MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 3 fable-a — G3 规格修订落笔 + D32–D40 归档 + A2 产品接线 SOTA 验收单

角色：架构 / SOTA 规格修订，**只动文档**。本轮写入面恰为两个文件：本文件与
`.agent_workspace/unblock/round1/fable-b-G3.md` 的冲突段（逐处以「R3 修订」标注）。
零产品 crate 改动、零 `docs/algorithms/*` 改动、零 DECISIONS/STATUS 改动——拍板已在
`docs/DECISIONS.md` D32–D40 在案，本文只负责把拍板落进规格与验收面，不重开方向战。

撰写时点：HEAD `016951d`。工作区有并发在途改动（opus-a 的 `crates/soul-draft/Cargo.toml`
已声明 `soul-algo-trait` 依赖但 `analysis.rs` 尚未换血；opus-b 的 soul-import、gpt-sol-a 的
`t4d_product.rs`、gpt-sol-b 的 `intake_replay.rs` 与 `ci.yml` 均未提交）——本文引用代码
一律以已提交时点为准，验收单（§6）以 opus-a 落地后的 HEAD 为对象。

---

## 1. G3 规格 §1.1 已按 D32 修订（任务 1）

**冲突原文**（`round1/fable-b-G3.md` §1.1 不变式，Round 1 版）：
「`locked_by_user == Some(true)` ⟺ `user_band.is_some()` ⟺ `machine_band.is_some()`；
……未锁时三字段全 `None`」。落地代码（`crates/soul-graph/src/build.rs::tie_strength_of`）
对**每条**重建边无条件写 `machine_band: Some(…)`，未锁边也带；照原文写测试会红。

**修订后不变式**（已就地改字，全文见规格 §1.1）：

- 锁定 ⟺ `user_band.is_some()`（与 `locked_by_user == Some(true)` 同真同假；
  `TieStrength::is_locked_by_user` 两者并查是防御性写法，不是第三种状态）；
- `machine_band` 是**重建边的常在字段**：任何经 rebuild 写出的边恒 `Some`，锁定与否皆然。
  `None` 只出现在换血前的遗留行与 `release_tie` 清空后、未经下次 rebuild 的行上——
  **缺席 ≠ 未锁**，「缺席无法区分未锁与换血前的行」正是 D32 选常在的理由；
- 锁定时 `band == user_band.unwrap()`；未锁时 `band == machine_band.unwrap()`；
- 钉测在案：`soul-graph/tests/graph_correction.rs::an_unlocked_edge_still_records_what_the_counts_say`。

同批一并修订：§1.1 struct 草图加注「已过期一半」（落地 `TieStrength` 另有 8 个 T4D 字段，
「序列化输出与今天逐字节相同」随 `814064e` 过期；锁字段定义不受影响）——对应
round2/fable-b §2.4-2 的指认。release→rebuild 的 `machine_band` None→Some 往返不是缺陷，
是上述「缺席只标识两种行」语义的直接推论，无需第三条规则。

## 2. D34 归档：GC-6/GC-7 / R4–R5 本 Goal 记 STATUS 遗留（任务 2）

**拍板**（D32 之外本轮对 G3 影响最大的一条）：遗忘 vs 锁（GC-6「peer 被遗忘则边删除，
锁不豁免」、GC-7「互动证据清零的锁定边存活为计数 0 / 时间戳 None」）**Goal 1 不实现**。
rebuild 仍只遍历有观测的 peer（`build.rs` 主循环 `for (peer_id, acc) in &peers`），
`first/last_contact_utc` 保持非 Option，**产品 crate 不设计墓碑 schema**——墓碑/Option 化
是迁移，不是默认。

已落进规格的三处标注：§1.1 Option 放宽段改为「R5 未来实施时的设计记录」；§2 伪码后加
D34 顺延注（同时记 R3 证据并集已由 `9b12268` 兑现）；§5 表 GC-6/GC-7 两格改为
「可实现，但随 D34 顺延（本 Goal 不做，记 STATUS 遗留）」。**给后续轮次的一条硬边界**：
任何人在 Goal 1 期间往 `crates/` 里加 tombstone 字段、Option 化这两个时间戳、或给 rebuild
补「锁定且无 tally 的 existing_edges」遍历，都是违反 D34，不是抢进度。P2-6 陈旧未锁边的
裁决与 R4 同一补丁位，一并顺延。

## 3. D33/D40 归档：A2 产品接线的口径（任务 3）

**D40**：Goal 1 人事摘要走冻结渲染器——`soul-draft` 调用 `soul_algo_trait::a2_render`，
不改 `crates/soul-algo-trait/src/a2.rs` 的任何模板字面量；依赖方向 soul-draft → soul-algo-trait
（渲染器进产品 crate，合法；反向永远禁止）。锁定边在 `a2_render` 之后**丢掉**
`personnel.tie.filed_band` bullet——GC-9a 在换渲染器之后继续成立。

**D33 + 遗留零判据**（round2/fable-a #8/#13 的收口）：

- **P1b 分列句的在场判据是 `as_of_utc.is_some()`**（等价 `algorithm_id == "T4D"`，
  以 as_of 为钉），**不得以计数 == 0 当缺席**。`TieStrength` 分列字段是裸 `u64 +
  serde(default)`：换血前的遗留行读回是 0/0/0/0，与实测零（仅群聊边的一对一侧、
  G1+ 后的 `group_out_count`）从计数本身无法区分。判据错一边就是「对遗留边渲染捏造的
  0 次分列句」或「吞掉真实 0」。
- **`{群聊次数}` := 持久化 `group_out_count + group_in_count`**，可以是真实 0。G1+ 后
  owner 群消息不再归因，`group_out_count` 结构性为 0——照 D33 渲染持久化的数，
  **不从证据重算**（重算即第二意见，违反 A2 纯渲染器地位），**不改 COPY_ZH 措辞**。
  由此留下的「用户自数群消息会比渲染值大」解释缺口归 §5 的文案漂移遗留，定价不修。
- `direct_count` / `group_count`：仅当 `as_of_utc.is_some()` 时各为 `Some(out + in)`
  （含真实 0）；遗留边两者都 `None`，冻结的 `venue_split()` 自然抑制整句。
- `DORMANT_AFTER_DAYS` 只存在于冻结 A2；soul-draft 源码不得再写字面量 180
  （现状 `rg "180" crates/soul-draft/src/` 为空，保持）。
- **不发明 COPY_ZH key**：A2 statement key 集合保持 9 元、`A2_ALGORITHM_ID` 保持 v3。

## 4. D35 归档：GC-9b 仍被挡；grep 门的合法形状（任务 4）

GC-9b（锁定边渲染「由你本人指定」归档变体）**仍不可实现**：COPY_ZH 未冻结该 key 之前，
产品**非测试**源码与 fixture 不得出现该句。解锁通道不变，即 G3 规格 §4 解锁条件——
经父代理 DECISIONS 留痕向 COPY_ZH 加法新增 key，A2 侧以加法 statement key（v3→v4）落地；
两步都不在 Goal 1。

**grep 门的形状**（规格 §5 原文「全仓库（含测试 fixture）」按原文不可执行，已修订）：

- 禁区 = `crates/**` 的非测试源码与 fixture；
- 豁免 = 执法测试内的反向断言字面量（现状唯一在案：
  `crates/soul-draft/tests/locked_tie_summary.rs::UNFROZEN_VARIANT`，其树级扫描排除
  执法文件自身）；文档与 `.agent_workspace/**` 不在禁区（拍板记录必须能引用被禁句）。
- 复核命令（供验收）：`rg -l "由你本人指定" crates/` 的输出应恰为执法测试文件本身。

## 5. 文档遗留：COPY_ZH §4 与冻结 `a2.rs` 的文案漂移（任务 5）

`docs/algorithms/COPY_ZH.md` §4 的 A2 模板与 `crates/soul-algo-trait/src/a2.rs` 的实际
字面量存在漂移，且不只是措辞——**这是 docs 遗留，不是 Goal 1 对冻结 crate 的补丁**（D40）。
两边都冻结，收口须走父代理 DECISIONS 留痕，方向是「改 COPY_ZH 以如实描述冻结 crate 的输出」
或「COPY_ZH 修订后 a2 出 v4」；任一都不在本 Goal。逐句对照（a2.rs 侧为实测字面量）：

| 句 | COPY_ZH §4 | a2.rs 落地 | 漂移性质 |
|---|---|---|---|
| P1 总量 | 「一共 {次数} 次往来，分布在 {天数} 个自然日、{会话数} 个会话里。」 | 「有记录的往来 {n} 次，出现在 {days} 个不同的日子、{conversations} 个会话里。」（另有 0 日/0 会话的短形态） | 措辞 |
| P1b 分列 | 「其中一对一往来 {一对一次数} 次，群里同场 {群聊次数} 次。」 | 逐字相同 | **无漂移** |
| P2 方向 | 五分支，含 2:1 阈值句「多数时候是你/对方先开口」「两边说得差不多」 | 三分支（只发出/只收到/双向），双向句「往来是双向的：你发出过 {out} 次，对方发来过 {incoming} 次。」，**无任何比较** | **结构**：COPY_ZH 的 2:1 是比较，与 a2「不比较任何计数」的冻结原则相抵；crate 侧是更严格的一方 |
| P3 场合 | 有一对一 →「有过一对一交流。」；仅群聊 →「只在群聊里见过。」 | 仅群聊才出句「到目前为止只在群聊里见过往来，没有一对一的记录。」；有一对一时**刻意无句**（正向句是「描述关系」的一步） | **结构** |
| P4 近因 | 「最近一次是 {最后日期}。」；超 180 天追加「你们最近半年没有往来。」 | 「最近一次往来距最新的记录 {days} 天。」/「就在最新的记录当天。」；dormant 句「已经 {days} 天没有新的往来了；下面的归档说的是过去的记录，不是现在的联系频率。」 | **结构 + 口径**（日期 vs 整数天差；无「最近半年」字样） |
| P5 归档 | 「按上面的计数，这段关系归在『{强/中等/弱}』一档；这是工作假设，不是对这个人的判断。」 | 「按上面的计数，这条往来归在「{强/中等/弱}」一档。这是对记录的归档，不是对这个人的评价。」另有 `Band::None` 变体「还看不出该归在哪一档」 | 措辞（关系 vs 往来；工作假设 vs 归档；引号制式）+ crate 多一个 None 形态 |

附属同类项：COPY_ZH §5-3「{群聊次数}=群消息条数（用户可自行数出）」在 D33 口径下对 owner
自己那侧不再成立（结构性 0），复核承诺的兑现措辞也归这条遗留。**本 Goal 的直接后果只有
一条**：A2 接线的验收以 a2.rs 字面量为准逐字断言，**不得**以 COPY_ZH §4 散文为断言目标
（P1b 除外，两边逐字相同）。

## 6. A2 产品接线 SOTA 验收单（任务 6；opus-a 落地后父代理逐条打勾）

冻结面零改动（先查，红了后面不用看）：

- [ ] 1. `crates/soul-algo-trait/**`、`crates/soul-algo-tie/**`、`docs/algorithms/**`
      在本轮 diff 中零改动；`A2_ALGORITHM_ID == "a2.tie_summary_renderer.v3"`、
      `A2_STATEMENT_KEYS` 仍 9 元。
- [ ] 2. 依赖方向：`soul-draft/Cargo.toml` 增 `soul-algo-trait` 为普通依赖；无反向边；
      `soul-algo-trait` 依赖清单不变（纯函数，无 store/Tauri/HTTP）。

换血本体（`crates/soul-draft/src/analysis.rs`）：

- [ ] 3. `points_for`（或其后继）经 `soul_algo_trait::a2_render` 产句；自制话术源删净：
      `band_word`（「往来不多/中等/密集」）、自制方向句、自制归档句不复存在——
      `rg "往来不多|往来中等|往来密集" crates/soul-draft/` 为空。
- [ ] 4. `rg "180" crates/soul-draft/src/` 为空（`DORMANT_AFTER_DAYS` 只在冻结 A2）；
      soul-draft 无任何计数比较（无 2:1、无阈值），dormancy 判断整个来自渲染器。
- [ ] 5. 适配器（TieStrength → TieScore）逐字段：band 三档一一对应（`Band::None` 对
      SupportedBand 边不可达，走 unreachable 而非静默映射）；u64→u32 显式转换不 panic
      （saturating 或 try + 测试钉边界）；`as_of_unix`/`last_contact_unix` 来自持久化
      `as_of_utc`/`last_contact_utc`，不读墙钟；`any_direct` 取自持久化观察形状
      （建议 `edge.types` 含 `direct`），**不得**从遗留行 serde 默认 0 的分列计数推得。
- [ ] 6. 证据 id：UUID 稠密 intern 成 u64、渲染后映射回去，每个 bullet 的 evidence_ids
      非空且都能 resolve；计数句的支撑沿 `counted_rows` 语义扣除纠正行（含全遗忘 fallback）；
      recency bullet 经 `last_contact_evidence_id` 只 cite 最近那一行。
- [ ] 7. **P1b 三态测试**（D33 核心，缺一不可）：(a) T4D 边有群往来 → 分列句在场，数字 =
      持久化 `direct_out+direct_in` / `group_out+group_in`；(b) T4D 边分列侧真实 0
      （仅群聊边或 G1+ 后 group_out=0）→ 分列句在场且如实渲染 0；(c) 遗留边
      （`as_of_utc == None`，计数 serde 默认 0）→ 分列句缺席，无捏造的 0。
- [ ] 8. GC-9a 存续：锁定边渲染后 `personnel.tie.filed_band` bullet 被丢弃，其余句照常
      且至少一句在（activity bullet 恒在）；对照组未锁边归档句在场且**逐字等于**冻结
      crate 输出（断言目标是 a2.rs 字面量，不是 COPY_ZH §4 散文——§5 漂移已归档）。
- [ ] 9. D35 门：`rg -l "由你本人指定" crates/` 输出恰为执法测试文件；
      `locked_tie_summary.rs` 全绿（含树级扫描）。
- [ ] 10. 结构不变量存续：每个 `SummaryPoint` 仍过 `assert_non_clinical` 且证据非空
      （构造期，非渲染期）；`PersonSummary.notice` 仍为工作假设声明（盖计数句，per G3 §4）；
      `phrase_with`/`summary_body` 的 E1 路径与 redactor 不变。
- [ ] 11. 回归：`cargo test -p soul-draft -p soulcore -p soul-graph` 全绿，特别是
      soulcore 的 AC-16/17 会话测试（`person_summary` source=counts、user_endpoint 改写路径）
      与 `graph_correction` 全套；`xtask` 的 denylist/e0 门照旧红绿。
- [ ] 12. 负面边界：本单不含也不许夹带——COPY_ZH 改字、a2.rs 改字、新 statement key、
      GC-6/GC-7 实现、tombstone 字段、F04c 第三道门。出现任何一项即整单打回。

——以上 12 条全绿，PROGRESS「A2 债」行可关；§5 的文案漂移与 D34 的 GC-6/7 作为两条
显式遗留移交 STATUS/Goal 2，不随本单关闭。
