# GAPS — 计划缺口（对照作者原意 / ALGO_FROZEN / Goal 1 现实）

MODEL_SLUG: claude-fable-5-thinking-xhigh

判级口径：P0 = 不修则合入 main 的计划集会说假话、诱导错误行动或在 M2 制造倒退；
P1 = 不修则违反单源/可发现性，但不立即致害；P2 = 打磨项。
每条注明「谁受害」与「修在哪」。全部为文档级改动；无一条要求写应用代码。

---

## P0

### P0-1 STATUS 说假话（P4 现状诚实）
- 证据：`docs/STATUS.md`「尚未写应用代码」「| v0.1 实现 | 未开始 |」「WP01 需把 schema `$ref`
  接到 `_defs`…」「下一步 CreateGoal：Goal 1」。四句对仓库现实全部为假/过期（Goal 1 分支
  WP01–WP13 + DPAPI 已落地，schemas.lock.json 已钉，Goal 1 早已创建）。
- 谁受害：新父代理照 FORMAL「开工第一动作：确认 STATUS 为 PLAN_FROZEN → CreateGoal：Goal 1」
  执行，会**重开一个 Goal 1**，与 149 提交的现有主干平行——这正是 BLOCKERS M1 刚清理过的
  双实现线事故的复刻路径。
- 修在哪：重写 `docs/STATUS.md` 为三态 + 指针（ARCHITECTURE §3）。

### P0-2 计划权威面对 ALGO_FROZEN 零吸收（P3）
- 证据：`grep -rn "T4D\|A0\|algorithms"` 在 PRODUCT_LOCK / DECISIONS / FORMAL / SECURITY /
  STATUS / README 上 **0 命中**。README 索引没有 `docs/algorithms/` 行。
- 谁受害：只读计划面的代理不知道判档已冻结；「其余未指定问题由项目自主决定」（作者原意 10）
  会被再次触发去重选算法——三轮消融白做。
- 修在哪（最小加性，不重开拍板）：
  1. PRODUCT_LOCK「自主拍板」表加一行：`| 人脉强度/特质轴算法 | 已冻结：T4D + A0，
     权威 docs/algorithms/DECISION.md（ALGO_FROZEN）；用户可见话术权威 COPY_ZH.md |`。
  2. DECISIONS 新增 D41（编号避让见 P0-5）：记录「计划吸收算法冻结」这一拍板本身。
  3. README 权威链加 algorithms/DECISION.md。
  4. FORMAL「先读」清单（现为 PRODUCT_LOCK、DECISIONS、SECURITY、STATUS、schemas）加
     `docs/algorithms/DECISION.md`。

### P0-3 FORMAL 验收矩阵没有一行验算法（P2 可测 × P3 吸收）
- 证据：AC-01–AC-26 中与图谱相关的只有 AC-08「节点≥3，边有证据」；无 band 正确性、
  无 180/360 闭区间、无 as_of 纪律、无 A0 intake 不绕锁、无 A2 禁第二套阈值。
  这些在算法 crate 内有测试，但**产品边界**（BLOCKERS G1 夹具清单所在层）无门禁行。
- 谁受害：Goal 1 可以在保持 T0 判档的情况下把 AC-08 打绿关闭——DECISION §6.5 的
  「遗留行为不是规范」没有牙齿。
- 修在哪：FORMAL 矩阵追加算法门禁行（建议 AC-28 起，Given/When/Then + 谁跑 = CI），
  内容直接引用冻结夹具，不发明新阈值：
  - `lilei_12` → Strong（锚保全）；`group_heavy_plus_one_direct_each_way` → 非 Strong；
    仅群聊 → Weak 且 A2 场合句仍展示群计数；
  - 179/180/359/360 闭区间（`>=`）各一行；
  - as_of：全库单值、per-peer 对照必须不同（DECISION §3 的 `peer_local_as_of…` 夹具）；
  - A0：锁定轴后整卷重答，轴不动、回答仍录制并进 ignored；
  - A2：源码级无第二套 3/10/3/180、无诊断词、分列句只消费 TieScore 携带值。
  合法性：这是把已冻结决定翻译成门禁，属共享简报目标 5 的明示授权；不加工作包，
  不与 13 条切片互否（切片 3「人脉图 v0；推断带证据与证据档」被细化，未被扩张）。
  **不加** F04c 第三道降档门（红线）；F04c 作为已知限制在矩阵注记引用 DECISION §4.2 即可。

### P0-4 schema 双家分叉 + tie_strength 义务缺位（P1 单源 × 契约）
- 证据：本分支 9 份 schema 是 PR #1 裸 string 版；Goal 1 版已 `$ref` `_defs` 并被
  `schemas.lock.json`（11 份 sha256）钉死。两线的 `relationship.schema.json:14` 都是
  `"tie_strength": {"type":"object"}`，而 DECISION §6.4 要求携带 band、一对一/群聊分列、
  last_contact、沉寂天数、as_of；COPY_ZH P1b「任一数缺席则整句不出现」= 分列字段缺席时
  冻结话术被静默丢句。
- 谁受害：M2 时 9 个 add/add 冲突，且若误取 main 侧即把 uuid7 收紧倒退回裸 string；
  tie_strength 无契约则 G1 接线的持久化形状无门禁。
- 修在哪（两步，影响声明齐全）：
  1. 本打磨把 9 份 schema **逐字节替换为 Goal 1 版**（`git show origin/cursor/soul-goal1-7b1c:docs/schemas/<f>`）。
     对 `schemas.lock.json` 影响：**零**（字节同 → 哈希同；lock 本身留在 Goal 1 线）。
  2. tie_strength 的字段义务先以**计划义务**成文（FORMAL 新 AC 行 + PRODUCT_LOCK 人脉图行
     加半句「边的强度结构以 TieScore 为准（DECISION §6.4）」）；`relationship.schema.json`
     正文的实际收紧**放到 Goal 1 线执行**（G1 接线同批），因为改正文必然使
     `schemas.lock.json` 失效，重钉哈希的义务应落在持有 lock 与 CI 的那条线。
     若父代理决定在本打磨直接改正文：必须在交付说明中写明「Goal 1 合并 main 后须重钉
     schemas.lock.json 并跑 schema-freeze 门」，否则 Goal 1 CI 必红。

### P0-5 DECISIONS 编号即将相撞
- 证据：unblock 线已用 D32–D40（`git diff origin/cursor/soul-goal1-7b1c
  origin/cursor/goal1-unblock-a073 -- docs/DECISIONS.md`）。本分支账本停在 D31。
- 谁受害：本打磨若从 D32 顺排新拍板，PR #7 汇入 main 时同号异义，账本从此不可引用。
- 修在哪：本打磨新拍板一律 **D41 起**；ARCHITECTURE §4 的合并手册写明 DECISIONS 取并集。

### P0-6 M2 合并手册缺位（F10 隐雷）
- 证据：BLOCKERS M2 的「四个 add/add」是在 main 无计划 docs 时测的；本分支合入后
  SECURITY/STATUS/DECISIONS/schemas（若不做 P0-4）全部新增冲突，而现有任何文档都没有
  逐文件取法。
- 修在哪：随本打磨交付 ARCHITECTURE §4 的表（或并入 BLOCKERS 的后继/README 注记），
  让执行 M2 的代理照表清算，不现场发明。

---

## P1

### P1-1 README 过期且不可导航（P6）
- 「当前处于产品锁定阶段，还没有可运行的应用」双重过期；无 algorithms、无 BLOCKERS、
  无分支拓扑；一句话是自由改写非引用（DUAL_SOURCE B6）。修法：ARCHITECTURE §2。

### P1-2 `.agent_workspace/context/plan/` 六份权威副本（P1 单源）
- main 上产品锁唯一副本住过程目录（权威倒挂）。修法：与 docs/ 落 main 同批删除六份，
  `context/README.md` 留两行说明。REPORT Q4/Q5 已论证 D27 精神与处置。

### P1-3 FORMAL 的「现状假设」全文过期
- 「当前几乎无代码」（main 有两个算法 crate + 完整测试）；「写码前阻塞项只有：PRODUCT_LOCK、
  DECISIONS、`docs/schemas/` 九份、SECURITY、STATUS」（schema 实为 11 份：`_defs` +
  `soul-import-v1` 在列外）；WP01 描述「冻结 schema、SECURITY、CI」已在 Goal 1 完成。
  修法：FORMAL 首部加一段「现状（2026-08-25）」：算法已冻结、Goal 1 已在
  `cursor/soul-goal1-7b1c` 基本落地、本提示词自此的作用是**验收与收尾**而非从零开工；
  「开工第一动作」改为「若 Goal 1 分支已存在，禁止重开；按 BLOCKERS 工序收尾」。
  「九份」改「十一份」。

### P1-4 SECURITY 分叉的处置未记档
- 本打磨不动 SECURITY（正确），但这一「不动」本身要写进合并手册（ARCHITECTURE §4），
  否则下个代理会好心把 Goal 1 的加密落地节抄上 main（在代码进 main 之前即 P4 违规）。

### P1-5 DECISION §6.1 命名勘误无家
- `graph_build.rs` / `crates/soul-algo` 已不是真名；BLOCKERS 已裁「实现跟真名」但 BLOCKERS
  还没进 main。修法：`docs/algorithms/README.md` 加性两行勘误注（真名
  `crates/soul-graph/src/build.rs`、`soul-algo-tie` / `soul-algo-trait`），不动冻结正文。

### P1-6 「T4D 吸收在途」在计划面不可见
- PR #7 已落 T4D 判档、G1+ 扇出修复、G2 intake 锁、G3 纠正与 D32–D40，但 main/本分支
  没有任何文件提及该线存在。修法：STATUS 拓扑段 + README 分支三行（已含在 P0-1/P1-1 修法内）。

---

## P2

### P2-1 impl 快照无标签
- `context/impl/graph_build.rs` = Goal 1 `ac3d9b3` 快照，判档是被 §6.5 判为遗留的 T0，
  且含 3/10/3 第二套字面量（历史证据豁免未写明）。修法：`context/README.md` 标签行
  （并入 P1-2 的同一文件）。

### P2-2 PLAN_VERIFY_PROMPT 未标「已执行完毕」
- 三轮已跑完（scan-rounds/ 有 R1–R3），文件仍以现在时祈使句存在，可能被误粘贴重跑。
  修法：首部一行存档声明。

### P2-3 relationship.schema 其余松散处
- `"types": {"type":"array"}` 无 items；`egress_scope` 是 const 但不在 required。
  与 tie_strength 同批在 Goal 1 线收紧（同样触发 lock 重钉），不单独立项。

### P2-4 `.agent_workspace/PROGRESS.md` 与 docs/STATUS 职责重叠
- main 上它是事实状态页。docs/STATUS.md 重写后将其降为任务索引（一行注明「状态权威在
  docs/STATUS.md」）。

### P2-5 A1 空转与 F04c 已知限制在计划面无注记
- 二者都是冻结决定的「预期行为/已知代价」（DECISION §2.4、§4.2），计划面不提则后来者会
  当缺陷重开。修法：随 P0-3 的矩阵注记各一句引用，不新增门。

---

## 明确不是缺口（防重开）

- 仅群聊者落 Weak、F04c 复燃一响、任一场地时钟不对称面 —— DECISION §4 已定价入档。
- WP12 缺席、文件写执行不在 v0.1 —— D30/D31/AC-27 已裁。
- Goal 1 的 hosted CI 空 runner、作者手动清单 —— 实现线事务，BLOCKERS/Goal 1 STATUS 在管。
- `docs/algorithms/R*-SYNTHESIS.md` 与 `.agent_workspace/R*-SYNTHESIS.md` 的索引对 —— 设计使然。
