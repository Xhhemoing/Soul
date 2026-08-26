# Round 1 fable-a — 全局架构 / 多源审计报告

MODEL_SLUG: claude-fable-5-thinking-xhigh
角色：计划文档集的全局架构与多源审计（不改 docs/、不改 crates/、不实现 Goal 1）。
核对基线：本分支 `cursor/polish-project-plan-5280` @ `c65a0b0`；`main` @ `7b35bde`；
`origin/cursor/soul-goal1-7b1c` @ `2e72ddf`（149 提交）；`origin/cursor/goal1-unblock-a073` @ `016951d`；
`origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`（`BLOCKERS_FROZEN`）。
全部结论来自 `git show` / `diff` 实测，diff 命令与结果见正文；配套文件：
`DUAL_SOURCE.md`、`ARCHITECTURE.md`、`GAPS.md`、`SOTA_BAR.md`（同目录）。

---

## 一句话结论

计划文档的**内容**基本站得住（产品锁自洽、切片未膨胀、验收矩阵可测），但**结构**不及格：
同一事实最多有四个家，`docs/STATUS.md` 至少四句在仓库层面为假，计划权威面对 `ALGO_FROZEN`
的吸收是**零**（`grep -rn "T4D\|A0\|algorithms" docs/PRODUCT_LOCK.md docs/DECISIONS.md
docs/FORMAL_WORK_PROMPT.md docs/SECURITY.md docs/STATUS.md README.md` → 0 命中），
且本分支若按现状把 docs/ 合进 main，将在 M2（main merge 进 Goal 1）时制造 9 份 schema、
SECURITY、STATUS、DECISIONS 的 add/add 冲突或**倒退**。修法全部是文档级，见 GAPS.md 的 P0 清单。

---

## 核心事实（实测）

### F1 本分支与 Goal 1 的 docs/ 逐文件关系

| 文件 | 本分支 vs Goal 1 @ 2e72ddf | 备注 |
|---|---|---|
| PRODUCT_LOCK.md / DECISIONS.md / FORMAL_WORK_PROMPT.md / GOAL2_POLISH_PROMPT.md / PLAN_VERIFY_PROMPT.md | **逐字节相同** | 都源自 PR #1 |
| SECURITY.md | **不同**：Goal 1 多两节「加密落地（WP01 实测结论）」「DPAPI 落地」（约 47 行实现实证） | 本分支是 PR #1 短版 |
| STATUS.md | **完全不同**：本分支 24 行「未开始」版；Goal 1 是数百行实现纪事 | 见 F3 |
| schemas/ 11 份 | `_defs`、`soul-import-v1` 相同；**其余 9 份全部不同**（Goal 1 已把 `*_id` `$ref` 到 `_defs` 的 uuid7、`evidence_band` 走 `allOf` 收紧）；Goal 1 另有 `schemas.lock.json` 钉 11 份 sha256 | 本分支是 WP01 之前的裸 string 版 |
| GOAL1_PLAN.md | 仅 Goal 1 有 | 实现规划，不属计划权威面 |

### F2 DECISIONS 编号已在别处被占用（合并炸弹）

`origin/cursor/goal1-unblock-a073:docs/DECISIONS.md` 已追加 **D32–D40**（machine_band、
{群聊次数} 口径、GC-6/7 不做、GC-9b 禁句、Direct/Group 判据、Telegram 日期校验、
IntakeReceipt.ignored、A2 话术走冻结 `a2_render` 等）。本分支的 DECISIONS 停在 D31。
**本打磨若要新增拍板，编号必须从 D41 起**（或另立前缀），否则 PR #7 汇入时同号异义。

### F3 STATUS 三个版本、四句为假（详见下方「必答」）

### F4 计划面对 ALGO_FROZEN 零吸收

`docs/algorithms/DECISION.md` 已冻结 T4D + A0、常量表（3/10/3/180/360 闭区间）、as_of 纪律、
§6 Goal 1 合并义务。但 PRODUCT_LOCK / DECISIONS(D1–D31) / FORMAL / SECURITY / STATUS / README
**没有一处**引用它。README 的索引里根本没有 `docs/algorithms/` 这一行。FORMAL 的验收矩阵
AC-01–AC-26 没有任何一行验算法（AC-08 只验「节点≥3，边有证据」）。P3 维度当前得分为零。

### F5 schema 的算法义务缺位

`docs/schemas/relationship.schema.json:14` 在**两条线**上都是
`"tie_strength": { "type": "object" }`——裸 object。DECISION.md §6.4 要求 `TieScore`
携带 band、一对一/群聊分列、last_contact、沉寂天数与 as_of，但 schema 与计划文档都没写这个义务。
COPY_ZH P1b 分列句「任一数缺席则整句不出现」意味着：schema 不钉分列字段，冻结话术会被静默丢句。

### F6 双源现状（详表见 DUAL_SOURCE.md）

最严重的三处：
1. `.agent_workspace/context/plan/{PRODUCT_LOCK,DECISIONS,FORMAL_WORK_PROMPT}.md` 与
   `{inference,profile,relationship}.schema.json` 六份，与 docs/ 版逐字节相同——而在 **main 上
   docs/ 里根本没有产品锁**（main 的 docs 只有 algorithms/），即 main 上产品锁的唯一副本
   活在过程目录里。权威倒挂。
2. 9 份 schema 在本分支与 Goal 1 是**两个不同文本**的双家。
3. STATUS 三分叉（本分支 / Goal 1 / unblock）。

### F7 `.agent_workspace/context/impl/` 快照已过期

`graph_build.rs` 逐字节命中 Goal 1 历史提交 `ac3d9b3`（WP05/WP06 期），其 `Tally` 判档
即 DECISION §6.5 判为「遗留行为，不是规范」的 T0；unblock 线已用 T4D 替换真身
（`814064e feat(graph): band ties with the frozen T4D rule under one store-wide as_of`）。
快照是 ALGO_FROZEN 证据链的输入（round1 `goal1_fidelity.rs` 引用它），**不能删**，但必须
打「历史快照 @ ac3d9b3，非现状」标签。

### F8 DECISION.md §6.1 命名漂移（已被 BLOCKERS 裁决）

§6.1 写 `graph_build.rs` 与 `crates/soul-algo`；真名是 `crates/soul-graph/src/build.rs` 与
`soul-algo-tie` / `soul-algo-trait`。BLOCKERS §0 已裁「实现跟真名」。冻结文件不必改正文，
但计划面（algorithms/README 或勘误注）应记一行，防新代理按字面找不到文件。

### F9 BLOCKERS 与本计划的关系

BLOCKERS.md（PR #6，对 main 干净）已把 Goal 1 的合入阻塞（M1–M3）与产品关闭阻塞（G1–G5、S1–S5）
冻结成单，并点名「STATUS 里『阻塞：无』只表示写代码的前置文档已齐」。计划打磨**不应复述** G/M/S 条目
（那会造第二个家），只应在 README/STATUS 建立指针。

### F10 合并拓扑的隐雷

BLOCKERS M2 记录 `git merge-tree origin/main origin/cursor/soul-goal1-7b1c` 只有四个 add/add
（.gitignore、Cargo.lock、Cargo.toml、rust-toolchain.toml）——那是**在 main 还没有 docs/ 计划面时**测的。
本分支把 docs/ 种上 main 之后，M2 将在每一份两边都存在且内容不同的文档上再爆 add/add：
SECURITY、STATUS、9 份 schema、DECISIONS（D32–D40）。逐字节相同的（PRODUCT_LOCK、FORMAL 等）
git 会自动清算。**结论：本打磨每改动一份 Goal 1 线上也存在且已分叉的文件，就为 M2 增加一处人工冲突**；
必须随交付附每文件合并规则（见 ARCHITECTURE.md「合并手册」）。

---

## 必答五问

### Q1 T4D 替换 T0 之后，PRODUCT_LOCK 是否仍内部自洽？

**自洽（无需改动正文以求自洽），但吸收为零。** 逐条核过：PRODUCT_LOCK 从未点名 T0/T4D——
T0 只是 Goal 1 `build.rs` 的临时实现，不是锁的内容。锁内与算法相关的句子全部与 T4D 相容：
「人脉图 | 节点=人，边=互动强度/关系类型/最近接触/证据」（T4D 的 TieScore 全覆盖；
「最近接触」与 T4D 刻意采用的任一场地 last_contact 时钟一致）；档位词「弱/中/强」与 COPY_ZH
词汇单源一致；「推断必须有证据」与 T4D 证据行、A0 证据档一致；切片 3「人脉图 v0；推断带证据
与证据档」不约束判档口径。T4D 的已知代价（仅群聊者落 Weak、F04c 复燃一响）没有触碰锁的
任何承诺。**真正的问题不是矛盾而是失联**：锁、拍板、FORMAL 里没有一个字把「关系强度怎么算」
指向 `docs/algorithms/DECISION.md`，新代理只读计划权威面会不知道判档已冻结——这是 GAPS P0-2。

### Q2 哪些 STATUS 句子在哪条线上为假？

**本分支 `docs/STATUS.md`（对仓库现实为假 / 过期的四句）：**
1. 「尚未写应用代码」——main 上已有两个算法 crate（非应用但确是代码）；Goal 1 分支上
   WP01–WP11、WP13、DPAPI 全部落地。仓库层面为假。
2. 「| v0.1 实现 | 未开始 |」——同上，为假。Goal 1 只剩 hosted CI 重开、T4D 吸收（PR #7 进行中）
   与作者 Win11 手动清单。
3. 「WP01 需把 schema `$ref` 接到 `_defs` 并补泄漏 fixture」——已完成：Goal 1 有
   `schema_wiring.rs`、`schemas.lock.json`（钉 11 份 sha256）、`leakage_checker.rs` 7 项全过。
4. 「下一步 CreateGoal：Goal 1。派 fable planner…」——Goal 1 已创建、已规划（GOAL1_PLAN.md）、
   已基本实现。照抄合入 main 会诱导新父代理**重开一个 Goal 1**。
   另：首行「单一事实来源」的自我声明在三版 STATUS 并存期间本身不成立，须定义分工（见 ARCHITECTURE）。

**Goal 1 线 `docs/STATUS.md`（@ 2e72ddf）：** 措辞整体诚实（hosted 绿与本机绿分开、
「Goal 1 还不能关」明写）。为假/误导的是**遗漏**：「| v0.1 其余 WP | 无 …剩下的是 HEAD hosted 绿，
以及作者 Win11 手动清单 |」——在 ALGO_FROZEN 落地后为假：DECISION §6 合并义务（T4D 替换判档）
与 BLOCKERS G1/G1+/G2/G3 都是关闭级阻塞，该行只字未提（unblock 线正在补，其 STATUS 已改写
WP06 扇出与 WP03 锁绕过两条）。「WP05 人脉图 | 完成」按原工作单定义为真，但发布口径下判档
是 §6.5 的遗留 T0，post-freeze 读来误导。

**main：** 没有 docs/STATUS.md（docs 只有 algorithms/），无句可假；但 main 的 README 只有
一行 `# Soul`，`.agent_workspace/PROGRESS.md` 承担了事实上的状态页——又一处权威倒挂。

### Q3 什么必须落 main，什么留在 Goal 1？

**落 main（本打磨交付）：** PRODUCT_LOCK（加一行算法权威指针）、DECISIONS（D1–D31 + 新拍板
从 D41 起）、FORMAL（补算法 AC 行 + 现状前言）、STATUS（重写为诚实三态：PLAN_FROZEN +
ALGO_FROZEN + Goal 1 在途于 `cursor/soul-goal1-7b1c`）、README（30 秒索引，含 algorithms 与
BLOCKERS 指针）、schemas（**建议逐字节采纳 Goal 1 版**，见 GAPS P0-4）、scan-rounds/、
templates/、GOAL2_POLISH_PROMPT、PLAN_VERIFY_PROMPT（标注已执行完毕）；删除
`.agent_workspace/context/plan/` 六份副本。

**留在 Goal 1 线（只经 PR #2/#7 汇入 main）：** GOAL1_PLAN.md、`schemas.lock.json`、
STATUS 的实现纪事、D32–D40、SECURITY 的「加密落地/DPAPI 落地」两节（它们描述**已落地代码**，
在应用代码进 main 之前放上 main 违反 P4 诚实）。BLOCKERS.md 走 PR #6 自己的通道，本打磨只建指针。

### Q4 双份 PRODUCT_LOCK（docs/ vs .agent_workspace/context/plan/）怎么处置？

两份**今日逐字节相同**（`diff` 实测），但性质完全不同：docs/ 是权威，context/plan/ 是
算法轮次开工时从 PR #1 拷来的只读上下文快照（随 PR #5 进的 main）。危险在于 main 上它是
**唯一副本**——权威缺席时快照自动升格。处置：本打磨把 docs/ 计划面落上 main 的**同一批**，
删除 context/plan/ 全部六份（三份 md + 三份 schema），在 `.agent_workspace/context/README.md`
留一行「plan/ 快照已删除，权威见 /docs；impl/ 是 Goal 1 @ ac3d9b3 的历史快照，非现状」。
不采用「改成指针 stub」——六个 stub 还是六个可漂移的家。历史考古走 git（快照在 `7b35bde` 树里永在）。

### Q5 D27（禁止第二份 PRODUCT.md）与工作区残留副本？

D27 的字面只禁「第二份 `PRODUCT.md`」，残留副本没有违反字面——没有任何文件叫 PRODUCT.md。
但 D27 的理由栏写的是「防双源」，PRODUCT_LOCK 头部也写「`docs/PRODUCT_LOCK.md` 是唯一产品权威」；
`.agent_workspace/context/plan/PRODUCT_LOCK.md` 在 main 上以唯一副本身份存在，恰是 D27 要防的
事态以更隐蔽的方式发生。**不必改 D27 措辞**（拍板不重开）；按 Q4 删副本即可让 D27 的理由重新成立。
若父代理愿意加固，可在 D41+ 新拍板一条「.agent_workspace 永非权威；权威文档不得在其下存副本」，
把这条纪律从共享简报升格为拍板。

---

## 建议优先序（详见 GAPS.md）

P0：STATUS 重写（诚实三态）；计划面吸收 ALGO_FROZEN（锁指针 + D41+ 拍板 + FORMAL 算法 AC 行）；
schema 采纳 Goal 1 逐字节版 + tie_strength 义务成文（含 schemas.lock.json 影响声明）；
DECISIONS 编号避让 D32–D40；随交付附 M2 合并手册。
P1：README 重写 30 秒索引；SECURITY 不动 + 合并规则记档；context/plan 删除；§6.1 命名勘误注。
P2：impl 快照打标签；PLAN_VERIFY_PROMPT 标历史；README 一句话改为引用而非改写。

红线自检：未动产品方向；未给 F04c 加第三道降档门；未建第二份 PRODUCT.md；WP 未增
（新增的是验收行与拍板，不是工作包；依据共享简报目标 5 的明示授权，与 D30「工作包只减不增」不冲突）。
