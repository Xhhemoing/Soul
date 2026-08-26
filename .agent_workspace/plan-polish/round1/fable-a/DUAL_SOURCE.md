# DUAL_SOURCE — 每一个有两个及以上家的事实

MODEL_SLUG: claude-fable-5-thinking-xhigh

核对方法：逐文件 `diff`（本工作树 vs `.agent_workspace/context/plan/` vs
`git show origin/cursor/soul-goal1-7b1c:<path>` vs `git show origin/cursor/goal1-unblock-a073:<path>`
vs `git show 7b35bde:<path>`）。「状态」列：**同** = 逐字节相同；**歧** = 文本已分叉；
**指** = 设计上的索引/指针关系（可接受）；**述** = 约束在多处复述（原文有互指）。

处置代号：`DEL` 删除；`PTR` 改为指针/加互指；`ADOPT-G1` 采纳 Goal 1 文本；
`SCOPE` 定义分工边界；`KEEP` 保留但打标签；`OK` 现状可接受。

## A. 权威级双源（必须清除或裁决）

| # | 事实 | 家 1 | 家 2 | 家 3+ | 状态 | 风险 | 处置 |
|---|---|---|---|---|---|---|---|
| A1 | 产品锁全文 | `docs/PRODUCT_LOCK.md`（本分支 = Goal 1 = unblock，逐字节同） | `.agent_workspace/context/plan/PRODUCT_LOCK.md`（= main `7b35bde` 上的唯一副本） | — | 同 | main 上权威缺席、快照升格；任何一侧单改即分叉 | `DEL` 家 2（与 docs/ 落 main 同批） |
| A2 | 自主拍板账本 | `docs/DECISIONS.md`（D1–D31） | `.agent_workspace/context/plan/DECISIONS.md`（同字节） | `origin/cursor/goal1-unblock-a073:docs/DECISIONS.md`（**D1–D40**） | **歧**（家 3 已多 9 条） | 本打磨若新增 D32 起的编号 → 与 unblock 线同号异义 | `DEL` 家 2；新拍板从 **D41** 起；M2 时 DECISIONS 取并集 |
| A3 | 开工提示词 + 验收矩阵 | `docs/FORMAL_WORK_PROMPT.md` | `.agent_workspace/context/plan/FORMAL_WORK_PROMPT.md`（同字节） | Goal 1 线同字节 | 同 | 同 A1 | `DEL` 家 2 |
| A4 | 9 份数据契约（`audit contact event evidence export-manifest inference memory profile relationship`） | 本分支 `docs/schemas/*`（PR #1 裸 string 版） | Goal 1 `docs/schemas/*`（`$ref`→`_defs` uuid7、`allOf` 收紧、附 `schemas.lock.json` 钉 11 份 sha256） | `inference/profile/relationship` 三份另在 `.agent_workspace/context/plan/`（= 本分支字节） | **歧** | 本分支版**落后**；照现状合 main 则 M2 add/add 冲突且有把 Goal 1 收紧倒退回裸 string 的风险 | `ADOPT-G1`（逐字节采纳 Goal 1 版 → 不动 `schemas.lock.json` 哈希）；`DEL` context/plan 三份 |
| A5 | 安全规范 | `docs/SECURITY.md`（PR #1 短版，37 行） | Goal 1 = unblock `docs/SECURITY.md`（多「加密落地（WP01 实测结论）」「DPAPI 落地」两节） | — | **歧** | 两节描述已落地代码，先于代码上 main 违反 P4；polish 若另行编辑则 M2 三方冲突 | 本打磨**不动** SECURITY；合并手册记「M2 冲突时取 Goal 1 侧」 |
| A6 | 项目状态 | 本分支 `docs/STATUS.md`（24 行「未开始」版，≥4 句为假，见 REPORT Q2） | Goal 1 `docs/STATUS.md`（数百行实现纪事） | unblock 线 STATUS（再改 WP03/WP06 条目）；main **无** STATUS | **歧**（三分叉） | 「单一事实来源」自我声明失效；合 main 后误导新父代理重开 Goal 1 | `SCOPE`：main 版 STATUS 重写为「计划态 + 仓库拓扑 + 指向 Goal 1 分支 STATUS」；实现纪事永远只住 Goal 1 线；合并手册记分工 |
| A7 | 关系强度算法（判档规则、常量、话术） | `docs/algorithms/DECISION.md`（权威，ALGO_FROZEN）+ `COPY_ZH.md`（冻结话术） | `crates/soul-algo-tie/src/constants.rs` 等（实现，DECISION §3 授权的唯一代码单点） | `.agent_workspace/context/impl/graph_build.rs`（**过期 T0 快照** @ Goal 1 `ac3d9b3`，含第二套 3/10/3 字面量）；Goal 1 真身 `crates/soul-graph/src/build.rs`（unblock 已换 T4D） | 指 + **KEEP 需打标** | 新代理误把快照当现状；DECISION §3「全仓库禁止第二处数字字面量」被快照字面违反（属历史证据豁免，需写明） | `KEEP` impl 快照 + 在 `context/README.md` 打「历史快照非现状」标签 |
| A8 | Goal 1 工作包定义与 DAG | `docs/FORMAL_WORK_PROMPT.md`（WP01–WP13 一行清单，权威） | Goal 1 `docs/GOAL1_PLAN.md`（展开的 DAG、批次、完成定义） | — | 指（GOAL1_PLAN 自称「权威矩阵在 FORMAL」） | 低；但 FORMAL 落 main 后两文件分居两线，互指要在 README 写清 | `PTR`（README 拓扑图注明 GOAL1_PLAN 住 Goal 1 线） |

## B. 约束复述（原文有互指，风险低，登记即可）

| # | 事实 | 家 | 状态 | 处置 |
|---|---|---|---|---|
| B1 | 「文件写执行 = v0.1.1，Goal 1 不做」 | PRODUCT_LOCK 砍/留表 + 后期路线；D10/D19/D31；FORMAL「AC-27 标 v0.1.1」；GOAL1_PLAN 钉死栏 | 述 | `OK`（口径一致；FORMAL AC-27 是门禁权威，其余是理由） |
| B2 | 「禁止第二份 PRODUCT.md」 | PRODUCT_LOCK 头部；D27；FORMAL 11.5 | 述 | `OK` |
| B3 | 「Goal 2 在 Goal 1 关闭前不启动」 | FORMAL；GOAL2_POLISH_PROMPT 前置；本分支 STATUS；Goal 1 STATUS；BLOCKERS | 述 | `OK`（STATUS 重写时保留一处即可） |
| B4 | v0.1 十三条切片 | PRODUCT_LOCK（权威） | FORMAL 只写「以 `docs/PRODUCT_LOCK.md` 的 13 条为准」 | 指 | `OK`（模范做法，其他事实应效仿） |
| B5 | 子代理模型选择（fable/opus slug 与降级链） | FORMAL 11.2/11.4（常设权威）；PLAN_VERIFY_PROMPT「当次模型（覆盖常设 11.2）」；templates/THREE_ROUND_DUAL_SCAN「覆盖 11.2」；GOAL2_POLISH_PROMPT | 述（各处均声明覆盖关系） | `OK` |
| B6 | 产品一句话 | PRODUCT_LOCK「## 一句话」（权威） | README 第 3 行（**改写**，非引用：如锁写「后期含手机」，README 略去） | 歧（轻微） | `PTR`：README 重写时改为压缩引用 + 显式链接，不再自由改写 |
| B7 | 「审计无正文」字段口径 | SECURITY「禁止 body/text/content/quote/summary/prompt」 | `docs/schemas/audit.schema.json`（白名单，SECURITY 自指 schema） | 指 | `OK` |

## C. 算法过程稿的索引对（设计使然）

| # | 事实 | 家 1（对外索引，docs/） | 家 2（全文，.agent_workspace/） | 状态 | 处置 |
|---|---|---|---|---|---|
| C1 | 算法 R1/R2/R3/RX 轮次结论 | `docs/algorithms/R{1,2,3,X}-SYNTHESIS.md`（各 3–5 行短页，首行注明「见 `.agent_workspace/…`」） | `.agent_workspace/R{1,2,3,X}-SYNTHESIS.md`（全文） | 指 | `OK`；README 拓扑说明「.agent_workspace 是过程档案，非权威」后此结构自安 |
| C2 | 冻结依据归档 | `docs/algorithms/DECISION.md` §7 | `.agent_workspace/round{2,3}/**`（t4d-verify、ablation 等） | 指 | `OK`（证据链，勿动） |

## D. 跨线事实分布（不是双源，是「住错线」或「缺指针」）

| # | 事实 | 现住址 | 问题 | 处置 |
|---|---|---|---|---|
| D1 | Goal 1 合入/关闭阻塞单（M1–M3、G1–G5、S1–S5） | `origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`（PR #6，冻结） | 本分支与 main 均无指针；计划打磨若复述条目即造第二个家 | README/STATUS 建指针，**不复述** |
| D2 | tie_strength 应携带的字段（band、分列、last_contact、沉寂天数、as_of、algorithm_id） | 仅 `docs/algorithms/DECISION.md` §6.4（TieScore 描述） | schema（两线皆 `"tie_strength": {"type":"object"}`，relationship.schema.json:14）与计划文档零承接 | GAPS P0-4：义务成文进计划面；schema 正文改动走 Goal 1 线并重钉 lock |
| D3 | DECISION §6.1 的文件/crate 命名（`graph_build.rs`、`crates/soul-algo`） | DECISION.md（冻结，名已过期）；BLOCKERS §0 已裁「实现跟真名」 | 新代理按字面找不到文件；冻结文正文不宜改 | 在 `docs/algorithms/README.md` 加两行勘误注（加性） |
| D4 | 「实现已开工」这一事实本身 | Goal 1 STATUS（真）；本分支 STATUS（假：「未开始」）；main（无声明） | 同一问题三个答案 | 随 A6 的 SCOPE 一并解决 |
