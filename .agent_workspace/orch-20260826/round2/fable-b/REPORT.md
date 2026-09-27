MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 2 / fable-b：Goal 1 关闭门与切片残余（只读核验）

核于 2026-08-26。承接 R1 fable-b（`../../round1/fable-b/REPORT.md`）与 R1 opus-a / opus-b 两份算法审计。

审计对象（尖端与提交时间，全部本轮 fetch 后实测）：

| 线 | 尖端 | 提交时间 (UTC) | 相对 R1 |
|---|---|---|---|
| `main` | `a0ec14b` | 08-25 03:30 | 未动 |
| 主干 `origin/cursor/soul-goal1-7b1c`（PR #2，CONFLICTING/DIRTY） | `6d1058b` | 08-25 17:59 | 未动 |
| 吸收线 `origin/cursor/goal1-unblock-a073`（PR #7 → `main`，MERGEABLE/CLEAN） | `6133307` | 08-25 05:02 | 未动 |
| **close-loop `origin/cursor/goal1-close-loop-a073`（PR #10 → 吸收线）** | `d4490b0` | 08-25 07:08 | **R1 未核验其内容，本轮补上** |

方法：对四条远程分支 `git show`/`git grep` 逐文件实读；`git merge-base --is-ancestor`、`git cherry`（patch 等价）、`git merge-tree --write-tree`（不落盘试合并）量化拓扑；`gh`（只读）核 PR 状态与 hosted run；**把吸收线与 close-loop 两棵树 `git archive` 提取到 `/tmp` 后真实跑了目标 crate 测试**（未 checkout，本树分支与工作区未动）。

---

## 一、总判

1. **四个关闭级缺口在主干全部仍开**——不是靠「HEAD 没动」推断，是本轮再次源码级实读确认（逐项证据见第二节）。主干自 R1 以来零新提交。
2. **四项修复在吸收线全部落地且我实测全绿**：在 `6133307` 提取树上跑 `soul-graph`(41) / `soul-profile`(26) / `soul-draft`(51) / `soul-import`(45) 共 **163 项测试 0 失败**；在 close-loop `d4490b0` 树上 `soul-graph` 48 项 0 失败（多出的 7 项是具名夹具产品对拍与 `UnscoredEdge` 混库测试）。
3. **未回主干**：吸收线不是主干的祖先（`merge-base --is-ancestor` 否），也不在 `main`。两线自 `80c9011` 分叉后各自前进：吸收线 +37、主干 +49（patch 等价的只有 1 个：`3161e02`≡`3015f0f`）。试合并（merge-tree）**恰好 5 个文件真冲突**：`.github/workflows/ci.yml`、`apps/desktop/src-tauri/tests/ipc_roundtrip.rs`、`crates/soul-import/tests/telegram.rs`、`crates/soulcore/tests/session_e1.rs`、`docs/STATUS.md`；其余全部自动合并。
4. **R1 未覆盖的重大新事实**：PR #10 线上已有 **D61**（「主干由 `cursor/soul-goal1-7b1c` 更替为 `cursor/goal1-unblock-a073`；旧主干降为历史祖先，不是合入路径」），PR #7 正文同口径，close-loop R3 综合写明合入路径 **PR #10 → PR #7 → `main`**。R1 的结构性风险 5（「合并路径没有 owner」）已升级为：**有 owner 了，但三个权威面当面互斥**——`main` 的 STATUS 仍写主干是 `soul-goal1-7b1c`、吸收线「待回主干（D50）」；主干 STATUS 仍写「不要合 PR #7 / #10」；D61 只存在于 PR #10 链上，`main` 的 DECISIONS 止于 D60。
5. **D54 关闭门今天过不了**，且差的东西分四层：两项非代理阻塞照旧（账单、作者清单——本轮均再实测/再核）、一项**仍存在的代码类工序**（主干分叉的 17 个代码提交没有吸收进 D61 主干——close-loop R3 自称「没有合法代码任务能推进 D54」，这个自评**漏了这一项**）、若干权威面收尾、若干已定价小项。详见第四节。

---

## 二、四个关闭级缺口逐项核验

### G1 — 图谱仍 T0（主干：仍开；吸收线：已修，实测绿）

**主干 `6d1058b`**：`crates/soul-graph/src/build.rs` 第 40–44 行仍是本地常量 `MODERATE_MIN_INTERACTIONS=3` / `STRONG_MIN_INTERACTIONS=10` / `STRONG_MIN_ACTIVE_DAYS=3`；`Tally::band()`（:118–131）= 互惠 ∧ 全场地计数 ∧ 活跃天——逐字是 T0。无一对一/群聊分场地、无 180/360 降档、无 `as_of`、不依赖 `soul_algo_tie`。违反红线 11（第二套阈值字面量）与 PRODUCT_LOCK 约束 13；AC-08/AC-28/AC-29/AC-30/AC-33 在主干必红。

**吸收线 `6133307`**：`build.rs` 全面改写——经 `t4d_adapt.rs` 调 `soul_algo_tie`；**全库单一 `as_of`** 在丢掉「联系人解不开」的行**之前**取 max（build.rs :246 附近注释与代码同证，正合 BLOCKERS G1 规格）；分列计数、`as_of_utc` 随边落库。实测：`t4d_band.rs` 11 项绿（含 `a_group_flood_with_one_direct_hello_each_way_is_weak_at_the_store`、`one_store_wide_as_of_demotes_the_dormant_not_the_active`、`demotion_edges_are_closed_at_the_store`、`split_counts_round_trip_through_the_store`），`t4d_product.rs` 12 项绿（含 2/3、9/10、日 2/3 边界与 `dormant_peer_is_scored_against_the_store_wide_as_of`）。

**close-loop 增量**：`e489769` 把 AC-28/29/30 收严成 FORMAL 字面要求的形态——`use soul_algo_tie::testing::{dormant_2019, group_heavy_plus_one_direct_each_way, group_heavy_plus_three_directs, lilei_12}` **整只导入**、写穿加密库、rebuild 后与冻结 crate 对同一批行的判读逐字段对拍（`t4d_product.rs` 增至 16 项，实测绿）。FORMAL :161 的「应当导入它们而不是重新推导」在吸收线上只是行为等价（自造史料），在 close-loop 上才是字面达成。

**G1+（owner 群消息扇出，AC-34）**：吸收线 `90da257` 已修；`soul-import/tests/import_to_graph.rs` 的 `what_the_user_says_to_a_group_is_attributed_to_nobody` 与 `an_owner_group_message_does_not_write_one_outgoing_row_per_speaker` 实测绿。

### G2 — intake 绕轴锁（主干：仍开；吸收线：已修，实测绿）

**主干**：`crates/soul-profile/src/service.rs` 的 `intake`（:179–253）对 `(Axis, Position)` 答案**无条件** `place_axis`，不查 `axis_is_locked`——函数文档甚至把这写成设计（「axis answers are not locked」）。`axis_is_locked` 只在 `record_axis_inference`（:406）被查。重答问卷会移动用户锁定轴；AC-31 必红。

**吸收线**：`62840ff`。intake 分支带守卫 `if axis_is_locked(&profile, axis.axis_id)`（service.rs :288），被拒答案进 `IntakeReport::ignored`，理由 `IntakeSkip::AxisLockedByUser => "axis_locked_by_user"`（:109），答案行照常落库。实测：`correction_lock.rs::re_answering_the_questionnaire_does_not_move_a_corrected_axis`（:213）绿；`intake_replay.rs`（`apply_intake` 与冻结 crate replay 全等，AC-31 末项）绿。

### G3 — 图边不可纠正（主干：仍开；吸收线：已修，实测绿）

**主干**：`build.rs` :322 恒写 `user_verdict: Some(UserVerdict::Unreviewed)`；无 `correct.rs`，无任何边纠正入口。AC-32 必红。

**吸收线**：新增 `crates/soul-graph/src/correct.rs`；rebuild 继承既有推断的 `user_verdict`（build.rs :336、:424 的 `held_verdict`）；机器档继续更新、生效档另存。实测 `graph_correction.rs` 9 项绿：`a_corrected_band_and_its_verdict_survive_a_rebuild`、`a_locked_edge_keeps_counting_and_keeps_its_band`、`releasing_a_tie_hands_the_band_back_to_the_counts`。命令面在 `soulcore`（`5e5b284`）。

**GC-9 / D48 话术**：吸收线**没有**给 COPY_ZH 加「由你本人指定」变体，而是走了 D48 允许的保守分支——锁定边**不渲染任何归档句**（`soul-draft/tests/locked_tie_summary.rs::a_corrected_edge_is_not_filed_under_anything`），并加了一条仓库级绊线 `no_unfrozen_filing_variant_is_anywhere_in_the_repository`（任何人把未冻结变体写进任何文件即红）。5 项实测绿。合规：D48 说的是「变体落地前锁定边不得渲染冻结 P5 原句」，不渲染满足它。

**close-loop 增量**：`0f8a073` 加混库测试——对未接线旧边（`UnscoredEdge`）的纠正拒绝不碰同库已锁边（`graph_correction.rs` 增至 12 项，实测绿）。

### M2 — 算法 crate 不在成员表（主干：仍开；吸收线：已修）

**主干 `Cargo.toml`**：16 个成员，无 `soul-algo-*`。红线 12 的依赖箭头在主干不存在。
**吸收线**：18 个成员，`soul-algo-tie` / `soul-algo-trait` 在列；xtask denylist 对冻结 crate 的豁免在 `4f0bee8`。注意实际落法与 D50 字面（「在 Goal 1 线上一次 merge 时加，**不在计划 PR 里做**」）不同——是在独立吸收分支上做的，事后由 D61 追认拓扑。

### 顺带核验：M3（fileplan 大小写夹具）

吸收线 `7aee812`/`c81c233` 的写法与 BLOCKERS 规格逐项吻合：**运行时**文件系统探针（写 `soul-case-probe.marker` 实测两种拼写是否两个目录，`tests/common/mod.rs` :105–114），守卫精确式 `25 + case_variant_entries + symlink_entries`（`unauthorized_paths.rs` :138），Bravo 无条件在语料。残余 `cfg(unix)` 仅用于 symlink 造件（合法）。主干 `fc96e46` 被 BLOCKERS 明判不采纳的写法在主干上原样未动。

---

## 三、回主干对照：拓扑现状

### 3.1 机械事实

- 吸收线**不是**主干祖先，**不在** `main`；`main` **是**吸收线祖先（所以 PR #7 对 `main` 是 MERGEABLE/CLEAN，GitHub 实查如此）。主干 PR #2 对 `main` 是 CONFLICTING/DIRTY。
- 分叉点 `80c9011`。主干侧 49 个独有提交 = **18 个碰代码**（其中 1 个已被吸收线 patch 等价吸收：`3161e02`≡`3015f0f`，证明樱桃摘工序可行）**+ 31 个纯文档**。未吸收的 17 个代码提交包括：产品 HEAD `478f19f` 收尾的六连诚实文案修复（遗忘 SSD 句、研究空状态、向导三处、端点/图谱出网措辞、WebView2 关自有流量 `928ef5a`）、第三证明轮的 IPC 级 AC 补证（AC-05/11/13/14/16/18/19/23 过 `invoke_handler`，`28f106c`…`c597358`…`8e23e7f`）、采集页修复（`8cf5da5`、`401a032`）、单实例托盘（`88cf931`）、Telegram 注入夹具。
- **试合并冲突面恰 5 个文件**（merge-tree 实测，前文列举）；其余（含 `soul-graph`/`soul-profile`/`soul-draft` 全部修复面与 desktop 大部分）自动合并。这次合并的体量是已知的、有限的。

### 3.2 权威面互斥（R1 风险 5 的升级）

| 权威面 | 说法 | 所在树 |
|---|---|---|
| `main` `docs/STATUS.md` :19/:49/:51 | 主干 = `soul-goal1-7b1c`；吸收线「待回主干（D50）」（且引用的吸收线提交 `c81c233` 已过期） | `main`（DECISIONS 止于 **D60**） |
| 主干 `docs/STATUS.md` | 「不要合 PR #7，……也不要合 PR #7 / #10」（两处） | 主干 `6d1058b` |
| **D61**（`docs/DECISIONS.md` 追加行）+ PR #7 正文 + close-loop R3 综合 | 主干**更替**为吸收线；旧主干降为历史祖先；合入路径 **PR #10 → PR #7 → `main`** | 仅 PR #10 链（`a29f792`） |

R1 说「没有任何一份文档写明 PR #7 如何回主干」——现在写明了（close-loop R3），但**写明它的那份文档不在 `main` 也不在主干**，而被更替的两个权威面还在各说各话。更实际的麻烦：主干在 D61 落笔（08-25 07:08）**之后**仍推进产品提交到 17:59（close-loop R3 fable-b 已把这记为 P1「7b1c live drift」；`d4490b0` 因此把主干从 ci.yml 自动触发里撤掉）。降级为「历史祖先」的线还在长出唯一的产品诚实性修复，这不是账面问题，是 D54 的实体问题——见下节。

---

## 四、D54 关闭门评估：矩阵 + 十三片同时过，现在差什么

门的定义（FORMAL :29 + D54 行）：**验收矩阵全部 v0.1 行通过，且 PRODUCT_LOCK 十三条切片同时成立；T0 图谱即使 AC-01–26 全绿也不能关**。按「谁能修」分层：

### A. 只有作者能动的（两项，照旧，本轮再实测）

1. **hosted CI 账单**。近 100 个 run 里成功的只有历史那次 `32754617268`（`2e72ddf`，08-24）；最新 run（08-26 00:37 UTC，Bead 线）仍是 **0 step、`runner: null`**——Billing & plans/spending limit 未解除。吸收线与 close-loop 分支**从未有过任何 hosted run**（双重原因：账单 + 两线 ci.yml 的 push 触发列表都不含自己；`d4490b0` 已把触发改为 D61 主干，等账单解除后 `workflow_dispatch`/push 才能兑现）。AC-26 的 hosted 半边、切片 13 在 D61 主干上零证据。
2. **作者 Win11 手动清单**：`scripts/author-manual-checklist.md` 实数 **76 个未勾选框**。AC-01 与十三片「在干净 Win11 x64 上证明」的总前提仍是零项。AC-26 的 NSIS 半边按 D56 也是作者签名件。

### B. 代理可做、尚无排期的（本轮最重要的判定）

3. **拓扑归一是仍然存在的代码类工序**。close-loop R3 归档写「不要再开 Round 4：没有合法代码任务能推进 D54」——这个自评对 close-loop 线内部成立，但**漏算了分叉本身**：D61 主干（吸收线∪close-loop）缺主干的 17 个代码提交，其中六连诚实文案修复直接关系切片 1/6/8/10/11 的「UI 如实写」义务（D15/PRODUCT_LOCK），第三证明轮把十几条 AC 从 crate 缝提到 IPC 缝。如果今天按 PR #10 → #7 → `main` 合入，得到的树在这些点上**比主干退步**，而其 STATUS 还会引用主干侧才有的 run 记录。反向吸收的成本已量化：1 个已摘、16 个待摘（多数是独立测试/文案提交，樱桃摘友好）、外加 5 文件冲突的一次显式合并。**这是当前唯一能推进 D54 的代码工序，没有 owner。**
4. **权威面对齐**：D61 落到 `main` 的 DECISIONS（现只在 PR #10 链）；主干 STATUS 与 `main` STATUS 的旧拓扑句按 D61 改写；**BLOCKERS（PR #6）至今 OPEN 未上 `main`**——D54 行的权威出处本身还不在关闭候选树上（close-loop 的 DECISIONS 已预记了合入时要改写 BLOCKERS 里 dev-sota 的「D32」编号冲突）。

### C. 已定价小项（不挡关闭判定，如实滚动）

5. **D52 的跨 crate 常量钉相等测试三线皆无**（本轮在 close-loop 上再核：`DORMANT_AFTER_DAYS` 仍只在 `soul-algo-trait` 三个文件出现，工作区没有把它与 `DEMOTE_ONE_BAND_DAYS` 钉相等的测试）。R1 fable-b 残余 8、opus-b 5.2 原样未动。
6. **S2**：AC-12 严格口径的嵌中文名夹具在 session 缝仍无专用测试（P1，升 P0 条件未触发）。
7. **G4** 重复导入幂等（P1，D55 不进矩阵）；`soul-egress` 无 `Authorization` 头 → 无 key 输入框（吸收线 STATUS 已如实记）。
8. COPY_ZH ↔ crate 渲染话术漂移（opus-a A2/A3、opus-b 5.1）——算法冻结侧义务，与 D54 平行，不属本节门内。

### 结论

D54 **今天在任何一棵树上都不可关**，且「差什么」不再是模糊的一堆，而是可枚举的四件：**账单（作者）→ 76 项清单（作者）→ 拓扑归一（代理，17 提交 + 5 文件冲突，无 owner）→ 权威面对齐（代理，D61/BLOCKERS 上 main）**。代码功能面（四缺口 + G1+ + M3 + GC 命令面 + 具名夹具对拍）在 D61 主干上已经齐了，我实测 211 项相关测试全绿。最短关闭路径顺序建议：先做 B3/B4（不依赖作者、消除「合入即退步」），账单解除后在归一树上 `workflow_dispatch` 五门，最后作者清单。

---

## 五、与 R1 报告的增量对照

| R1 结论 | 本轮判定 |
|---|---|
| 四缺口主干仍开、修复在 PR #7 | **维持**，并升级为实测证据（163+48 项测试真跑全绿；此前只有 `git show` 实读） |
| 风险 5「合并路径无 owner、无文档写明」 | **升级**：D61 与「PR #10→#7→main」已成文，但只在 PR #10 链上；三权威面互斥；主干在被降级后仍推进（live drift） |
| 「主干必红：AC-28…34」 | 维持；另补：AC-28/29/30 的**具名夹具导入**字面要求只在 close-loop 达成，吸收线是行为等价 |
| 残余 8（180 双字面量无钉相等测试） | 维持，close-loop 亦无 |
| AC-26 账单阻塞 | 维持，08-26 00:37 UTC 最新 run 仍空 runner；近 100 run 仅 1 次成功 |
| R1 未覆盖 | PR #10 的 36 个提交、D61、ci.yml 触发改动、`UnscoredEdge` 混库测试、GC-9 的「不渲染归档句 + 仓库绊线」具体落法、试合并冲突面=5 文件、主干独有 17 个代码提交的清单化 |

## 六、本轮未做（按指令)

未 `git checkout`/`commit`/`push`，未离开当前分支；未改权威文档与任何 crate；测试全部跑在 `/tmp` 的 `git archive` 提取树上，`/workspace` 仅新增本报告一个文件。
