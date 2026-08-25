# SOTA 验收 — 计划权威面

MODEL_SLUG: claude-fable-5-thinking-xhigh

核于 2026-08-25，HEAD `27b4060` + opus-a 未提交措辞修复（字节以 `REPORT.md` §0 的 sha256 为准）。

## 总判：**PASS（6/6）**

| 维 | 判 | 证据 |
|---|---|---|
| **P1 单源** | **PASS** | ① 产品权威唯一：`docs/PRODUCT_LOCK.md`，第二份 `PRODUCT.md` 实测不存在（D27）。② 判档常量唯一两处：`algorithms/DECISION.md` §3（语义）+ `crates/soul-algo-tie` 常量模块（取值）；核心计划七文件 + schemas 扫 `180/360` **零命中**，D52 已改常量名。③ 计划权威只在 `docs/`：`.agent_workspace/context/plan/README.md` 首行自declare 非权威（D41），PLAN_INDEX 第四节把 `.agent_workspace/**`、`scan-rounds/**`、`PLAN_VERIFY_PROMPT.md`、FORMAL 历史段全部点名为非权威。④ 拍板单表：D1–D59 连续无缺号，BLOCKERS「D32」撞号有合入检查单脚注。⑤ schema 字节有锁：`schemas.lock.json` 11/11 sha256 实测一致。残留：「≥8 字」n-gram 三处复述（取值一致、非判档阈值，见 REPORT R-6），不构成 FAIL |
| **P2 可测** | **PASS** | 矩阵 33 条 v0.1 行（AC-01–26、AC-28–34）每行齐 Given/When/Then + 谁跑；AC-27 明确不占行。负向可红性写进门禁本体：AC-28（改回任一场地计数必须变红）、AC-30（per-peer as_of 必须变红）、AC-12（crate 内单独绿不算过，session 缝必须变红）。AC-26 的「打包绿」有 D56 定义。关闭语义可判定：矩阵 ∩ 十三切片（D54），且「矩阵没有的行不能否决切片；切片没有的行不能否决矩阵」写死在 FORMAL 第 5 条。本树可跑的部分实测：schema 探针 12/12、`cargo test --workspace` 249 通过 |
| **P3 冻结吸收** | **PASS** | T4D/A0/A2/A1 四位同时出现在：PRODUCT_LOCK「灵魂层算法（v0.1）」节 + 自主拍板行 + 不可协商约束 13；DECISIONS D42–D48；FORMAL 冻结表 + AC-28–34 七条产品边界门禁；schema 层 `tie_strength` 类型化（`algorithm_id` 枚举锁死 `[T4D, T4]`，第三套规则进不了字节）。「A1 空转是预期」三处同义（LOCK / D43 / FORMAL）。吸收未复抄任何常量数值，未新增工作包（判档落 WP05、轴锁落 WP03、渲染落 WP10），未膨胀切片（LOCK 明写「本节不新增 v0.1 切片项」） |
| **P4 现状诚实** | **PASS** | STATUS 三态分立：`PLAN_FROZEN` ≠ Goal 1 关闭 ≠ `main` 有应用，且「不要再把『尚未写应用代码』当成全局事实」明文在案（该短语全树仅此一处否定式出现）。跨分支事实全部带分支名+提交号+核于日期（D57）；同树对 goal1 主干的提交号 STATUS/SECURITY 一致（`df5d2dd`）；M2 行如实记录吸收线 `c81c233` 已加 crate 成员表但未回主干。「阻塞：无」限定本树，明写不等于可合 main。SECURITY 用指针节区隔规范与那条分支上的实证，拒绝提前抄两节落地实证 |
| **P5 不膨胀** | **PASS** | WP12 保持删除，工作包只减不增（D30）且吸收算法明写不开新 WP。文件写执行 v0.1.1（切片 9「不执行写」、AC-27 出矩阵、D10/D19、SECURITY「不消费写文件令牌」五处同向）；OAuth v0.2（六处命中全为砍/禁/后期）；研究导出只预览不落盘；E0 无代码路径；D55 幂等明确挡在切片与矩阵外。F04c 第三道门四处禁令（LOCK 已知代价 + 约束 13、D44、FORMAL、PLAN_INDEX 改法表）无一处松动 |
| **P6 新父代理可从 PLAN_INDEX 开工** | **PASS** | 干跑路径闭合：PLAN_INDEX「先读四份」→ FORMAL「开工第一动作」五步 → goal1 STATUS / BLOCKERS 的 `git show` 指针，且 PLAN_INDEX 对 BLOCKERS 明写「本树没有这份文件」不会让人白找。误导面全部排雷：历史段整体成引用块且首句「存档不是工作指令」、PLAN_VERIFY 标一次性、重开 Goal 1 / 重派 planner 在「按问题查」有直接否定答案（D49）。拓扑三行在 README / PLAN_INDEX / STATUS / FORMAL 四处对拍一致。判档阈值「去哪查」有唯一答案且附「写出数字即缺陷」的自我防御 |

## 验收边界声明

本 PASS 判的是**计划文档面**达到可给新父代理整份粘贴开工的程度。它不证明 Goal 1 任何验收行通过（那些跑在 Goal 1 线），不解除 `BLOCKERS.md` 的合入/关闭项，不覆盖 `docs/algorithms/` 冻结面上的既有措辞债（COPY_ZH P4，见 REPORT R-1，已有备好补丁）。
