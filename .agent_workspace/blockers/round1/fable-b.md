MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 1 / fable-b — PRODUCT_LOCK vs 代码现状：阻塞 honest v0.1 的缺口

范围：灵魂层复刻（人格、记忆、人脉图「用户可查看和纠正」）。
核对对象：`origin/cursor/soul-goal1-7b1c`（下称 **goal1**，tip `3e88b48`）、`origin/agent/dev-sota`（下称 **dev-sota**，tip `be1abee`）、`origin/main`（tip `7b35bde`，含冻结的 `crates/soul-algo-tie` / `soul-algo-trait` 与 `docs/algorithms/DECISION.md`）。
三条线共同祖先是 `ea6f62f`（Initial commit）；main 只有算法冻结，两条 dev 线只有应用，谁也没并谁。

PRODUCT_LOCK 三处逐字一致（dev-sota 仅多 D32 导入注记），引用行号按 goal1 的 `docs/PRODUCT_LOCK.md`。

分类约定：
- **[LOCK]** 产品锁违规——Goal 1 关闭前必须修；
- **[T4D-COST]** 冻结 T4D 的已知代价——如实入档，禁止静默修补（DECISION.md §4，不得新增第三道降档门）；
- **[v0.1.1]** 明确推迟项——缺席是正确的，不是阻塞。

---

## B1 [LOCK] 人脉图不可纠正；rebuild 把 user_verdict 复位、整行覆写 tie_strength

**锁文本**：PRODUCT_LOCK L37 灵魂层表格「……人脉图、偏好。**用户可查看和纠正**」；L52「用户纠正锁定且优先」；L94 切片 4「用户纠正锁定，后续推断不覆盖」；L136 约束 5。

**现状（两条 dev 线逐字节相同，`crates/soul-graph/src/build.rs` diff 为空）**：
- `build.rs:322`：每次 `rebuild` 对每条 tie inference 写 `user_verdict: Some(UserVerdict::Unreviewed)`——任何既有裁决（Corrected/Accepted/Rejected）被无条件复位。
- `build.rs:246-251`：`tie_strength` 由本次 `tally.strength()` 整行覆写，没有锁字段可保留。
- 全仓库 grep：`UserVerdict::Corrected` 在两条 dev 线上**零构造点**；`soul-graph` 无 `correct.rs`，`soulcore` 无对应命令；goal1 的 `/graph` 视图（`apps/desktop/src/routes/Graph.tsx`）纯只读，没有任何纠正控件。

用户对一条关系档位的纠正在 v0.1 里**无处发生**；即便发生了，下一次导入 + rebuild 也会抹掉。这正是 R1-SYNTHESIS P0-2 与 main 上既有规格 `.agent_workspace/round2/fable-b/GRAPH_CORRECTION.md` 点名的缺口。

**修法**（规格已在 main，照抄即可，勿重新设计）：
1. `soul-graph/src/correct.rs`：`correct_tie` / `release_tie`，四步与 `correct_axis` 对称（UserCorrection 证据 → 边更新 → 锁标记 → 审计 `ProfileCorrect`）。
2. `TieStrength` 增三个 `Option` 字段：`locked_by_user` / `user_band` / `machine_band`（未锁全缺省，旧行无损读回；`relationship.schema.json` 的 `tie_strength` 是自由 object，零契约变更）。
3. `band` 恒为**生效档**（锁定=用户档），机器档另存 `machine_band`——现有消费者（图 view、人事摘要、起草语气）零改动即尊重纠正。
4. rebuild 防 clobber：锁定边保留 `user_band`、计数照更新、`machine_band` 重算；inference 既有 `user_verdict` 一律保留，仅新 inference 写 `Unreviewed`；遗忘压过锁（GC-6）；支撑清零时纠正证据行本身满足 `minItems: 1`（GC-7）。
5. 验收：GRAPH_CORRECTION.md §6 的 GC-1…GC-10。
6. `soulcore` 两个透传命令 + 图 UI 一个档位纠正控件（UI 可与 B4 的视图批次同落）。

---

## B2 [LOCK] 问卷 intake 绕过轴锁：重答问卷覆写已锁轴，挂锁图标仍显示

**锁文本**：L52「用户纠正锁定且优先」；L94「用户纠正锁定，后续推断不覆盖」；约束 5。冻结面：DECISION.md §2.2「A0——intake 不绕锁（Round 2 补丁，`apply_intake` 与 replay 全等）」。

**现状（两条 dev 线同病）**：
- goal1 `crates/soul-profile/src/service.rs:216-223`（dev-sota 同文件 :184-191）：`intake` 对每条轴回答**无条件**调 `place_axis(..., locked_by_user: None)`，没有 `axis_is_locked` 检查。
- `place_axis`（goal1 :500-532）无条件替换 `position` / `evidence_band` / `evidence_ids`；`locked_by_user: None` 意味着**锁旗原样保留**（:528-530 只在 `Some` 时写）。
- 结果：用户 `correct_axis` 之后重跑一遍问卷 → 位置被问卷答案覆写，`locked_by_user` 仍是 `Some(true)`——**UI 挂锁还亮着，锁住的内容已经没了**。比没有锁更糟：这是一次虚假认证。
- 对照组：推断路径 `propose_axis`（goal1 :406）**有**锁检查并返回 `RefusedAxisLocked`；`tests/correction_lock.rs` 只测了推断路径，两条线都没有「intake 撞锁」测试。
- 冻结参照物在 main：`crates/soul-algo-trait/src/a0.rs::apply_intake`（:295-344）经由 `place`（:350-361）对锁轴返回 `RefusedLocked`，被拒答案进 `IntakeReport::ignored`（证据照记、轴不动、用户看得见「我们保留了你的纠正」）。

**修法**：把 `intake` 的轴分支对齐 `propose_axis` / 冻结 `apply_intake`：`axis_is_locked` 为真 → 不调 `place_axis`，答案照常落证据行与事件，回执里列 ignored（含 `AxisLockedByUser` 理由），审计照写。补测试：correct → 重跑 intake → 位置不动、锁旗仍在、被忽略答案可见。约 10 行代码 + 1 条测试；两条 dev 线都要打。

---

## B3 [LOCK] 判档仍是 T0（venue-blind 3/10/3、无降档时钟）；冻结的 T4D 未合并；群扇出可制造 Strong

**冻结面**：main `docs/algorithms/DECISION.md` §6.5——「替换合并完成前，Goal 1 现行 `Tally::band` 只是**遗留行为，不是规范**」。把 T0 的档位当成产品结论展示给用户并宣布 Goal 1 关闭，即违规。

**现状（两条 dev 线 `build.rs` 逐字节相同）**：
- `Tally::band`（:118-131）：`interaction_count = outgoing + incoming`（**全场地**）、互惠判定全场地、本地常量 3/10/3（:40-44，与 DECISION §3「全仓库禁止第二处出现数字字面量」冲突）、**无 180/360 降档时钟、无 as_of**。这是 T0。
- 导入扇出（`soul-import/src/commit.rs:198-207`，两线相同）：owner 在群里发 1 条 → 对每个「说过话的人」各写 1 条 Outgoing 互动证据。扇出本身是合法观察（保留），但配上 venue-blind 判档，完整复现冻结决胜夹具 `group_heavy_plus_one_direct_each_way`：T0 判 **Strong**，冻结答案是 **Weak**。200 人活跃群把全员刷成 Strong/Moderate 在当前代码是现实路径。
- `crates/soul-algo-tie` / `soul-algo-trait` 只存在于 main；两条 dev 线的 `crates/` 里都没有。合并义务（DECISION §6）没人执行。

**修法**（照 DECISION §6，不是新设计）：
1. `Tally::band()` 整体替换为冻结判档（入口 `soul_algo_tie::score`，默认 T4D）；`build.rs` 本地常量删除，从算法 crate 的 `constants.rs` 导入（3/10/3、`DEMOTE_ONE_BAND_DAYS = 180`、`FORCE_WEAK_DAYS = 360`，闭区间，均已在 main 钉住并有测试）。
2. `Tally` 补一对一分列计数（direct out/in/自然日）与任一场地 last_contact——`InteractionRef.venue` 字段已在，只是判档没读它；口径细分不动存储 schema。
3. `rebuild` 签名收一个全库 as_of（缺省 = store 的 `max(occurred_at)`），禁止 per-peer as_of，judgement 不读墙钟。
4. 依赖方向：Goal 1 → 算法 crate；**禁止把 SQLCipher / soul-store / Tauri / HTTP 拉进算法 crate**（main 上两个 crate 的 Cargo.toml 均为零运行时依赖并写明此禁令——保持）。`Interaction.peer_id` 是 `u64`，适配层在 soul-graph 侧做 uuid→稠密 id 映射，或随「统一 `crates/soul-algo`」的合并义务一起解决。
5. 渲染面照 §6.4：A2/图 UI 只消费 `TieScore`；中文话术以 `docs/algorithms/COPY_ZH.md` 为准。

**[T4D-COST] 随替换如实入档、禁止静默修补的三条**（DECISION §4，改判只走 §5 回退链 + 父代理 DECISIONS）：
- **F04c 复燃一响**：七年休眠 + 昨天一来一回 → Strong。禁止为此加「近 90 天 ≥3 次」之类第三道门——那是新候选，须重开消融。
- **任一场地降档时钟的不对称面**：一对一停 200 天、群聊昨天活跃 → 不降档（`direct_quiet_200_group_yesterday` → Strong）。刻意为之：保证 A2 的 last_contact 句与档位永不同屏矛盾。
- **仅群聊者落 Weak**：`group_only_50` 从 T4 的 Moderate 变 Weak；群聊事实由 A2 场合句展示，自愈路径便宜（每方向 3 条一对一回 Moderate）。

---

## B4 [LOCK] 问卷没有 UI，AC-03 只在 headless fixture 上绿；/profile 视图两条线都是 Pending

**锁文本**：L91-94 v0.1 切片 1-3「安装、托盘、**灵魂向导**……问卷 + 导入……**可编辑档案**（特质轴/偏好/边界）」——要求在干净 Windows 11 上对**用户**成立，不是对 CI。L50「无文件则问卷回退」。

**现状**：
- `Wizard.tsx`（两线 127 行，goal1 与 dev-sota 内容均只有「默认全关」确认表 + 一个勾选框）：**零问卷题**。router 无问卷路由。
- goal1 STATUS 自己承认（下一步 §1）：「向导还没有画那十一道题」；headless 主流程用 `include_str!` 编进二进制的 `fixtures/questionnaire/answers_basic.json` 走 AC-03。安装后的产品里，没有导出文件的用户**造不出档案**——灵魂层入口对无文件用户关死。
- `/profile`（灵魂档案：特质轴、语气、纠正锁定）在 goal1 router 是 `ownedBy: "WP09 功能视图"` Pending，dev-sota 同样 Pending。核心的 `correct_axis` 无 UI 入口 → 「用户可查看和纠正」的**轴侧**在产品面同样不成立（与 B1 的图侧并列）。
- 命令面已备好：goal1 已把两套问卷并成十一题（`profile::questions()` / `profile::intake`、`fixtures/questionnaire/v0_1.json` 钉题号）；dev-sota 还是两套题号未合并（其 STATUS 亦承认）。

**修法**：goal1 线上接 UI 即可——向导补一屏十一题（题面/选项/形状全部来自 `questions()`，答案走 `intake`，留白合法）；`/profile` 视图接 `profile_view` + `correct_axis`（含锁态显示——B2 修完锁态才是真话）。dev-sota 线若被选为 canonical，先做它 STATUS 里欠的问卷合并再接 UI。

---

## B5 [LOCK/P1] KnownIdentifiers 恒为空（E1 姓名占位形同虚设的一角）；图 UI 密封标签不显示任何名字

**锁文本**：L76「进入 E1 的请求体必须经 redactor。第三人正文默认换成占位符；**姓名/账号同样占位**」。

**现状（goal1）**：
- `soulcore/src/commands/session.rs:326` → `draft::closed_session()` → `DraftSession::new(UNNAMED_MODEL, KnownIdentifiers::new())`（`commands/draft.rs:318`）。注释说填名单「需要打开的 store，那是 WP13 的事」——**WP13 第二段已落地 store，名单仍然没人填**。第三人 turn 整条占位兜住了大头，但 owner 自己写的 turn 里提到联系人姓名（中文名无形状特征，shape-scrub 抓不到）会原样进 E1 请求体。dev-sota 的 `ST01_IMPLEMENTER.md` 已写明正确做法：`PersonNode.label_ref` 经 `BlobStore::open` 解出的明文只进 `add_name/add_account` 词典、绝不进 body 字符串、`is_empty()` 为真值得测试报警——没人实现。
- 图 UI（goal1 `Graph.tsx`）刻意不开封：用户看到的是标识符摘要前几位（`identifier_hint`），页面自己写着「这一版不显示姓名」。配合 B1（不可纠正），v0.1 的「人脉图」对用户是一列哈希——「可查看」名存实亡。本机向 owner 展示第三人显示名**不是出网**（约束 6 管的是不出本机），开封路径存在（`BlobStore::open`），只是 UI 没接。

**修法**：session 打开后遍历联系人、开封 `display_label_ref`，喂进 `KnownIdentifiers`（唯一去向是占位词典），Drafter/PolicySession 共用同一份名单（现有 `closed_session()` 的成对构造保留）；补「名单非空时姓名零裸露」的 wire 测试。图 UI 在本机渲染解封标签（owner 视图），E1/研究面照旧只见占位。E1 姓名占位是锁文本，判 LOCK；图上显示名字判 P1（锁没逐字要求 UI 显示姓名，但关闭 Goal 1 时不得宣称「人脉图可查看」已达成）。

---

## B6 [P1·工程完整性] 两份 STATUS 互相矛盾，两条分支都自称 Goal 1 的「单一事实来源」

- **goal1 `docs/STATUS.md`**：WP13 两段**完成**；「`/files`、`/graph` 与起草的端点确认屏都不再是空路由」；问卷已并成一套十一题；WP10/WP11 均「完成并接到界面上」。
- **dev-sota `docs/STATUS.md`**：「WP09 功能视图 / **WP13 | 未开始**」；`/draft` `/files` 仍 Pending；两套问卷题号未合并；且它自己第 7 行还写着「分支 `cursor/soul-goal1-7b1c`」。
- 两份文件开头都是「单一事实来源。每个子代理完工必须更新本文件」。两条线自 Initial commit 分叉后各自重写了 soul-profile / soul-draft / soul-fileplan / UI 路由（文件清单与实现都不同），互不包含。
- 后果：任何「Goal 1 完成度」陈述在被引用前先要回答「按哪条线」。B1/B2/B3 在两条线上证据相同（build.rs、commit.rs 逐字节一致，intake 同病），不受裁决影响；B4/B5 的修复落点取决于 canonical 线。

**修法**：父代理裁决 canonical 分支（证据面 goal1 明显更完整：视图已接、问卷已合并、WP13 两段有测试背书），另一条降级为参考并在其 STATUS 头部写明；被选线的 STATUS 增补本报告 B1-B5 为未清项——当前两份 STATUS 都没有列出 B1/B2/B3 中的任何一条，这本身就是「STATUS 声称 vs 现实」的缺口。

---

## 不阻塞项（核对过，缺席是正确的）

- **[v0.1.1] 文件写执行**：`soul-fileplan` 无 execute 成功分支、`/files` 无执行按钮、写令牌不签发不消费（两线一致）——与 L109/L124（砍/留表、后期路线）一致。**不要补**。
- **[v0.1.1] 目录文件元数据采集**、**[v0.2] 研究导出落盘 / 用户全量导出 / OAuth**：同上，按锁推迟，未见越界代码。
- 群扇出的**数据收集侧**（每 speaker 一条互动证据）：保留——T4D 在判档侧消解其放大面（B3），扇出行本身是「此人没有从生活中消失」的合法观察与近因证据来源。

## 排序依据

B1/B2/B3 属「已发布的声称为假」（虚假挂锁、被抹掉的裁决、错误档位）——先于 B4/B5 的「能力缺席」（缺席至少是诚实的，且两份 STATUS 部分承认）；B6 是让前五条能被验收的前提性工序。全部六条中没有一条需要新算法决定：B1 有现成规格、B2/B3 有冻结参照物、B4/B5 是接线、B6 是裁决。
