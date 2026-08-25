# KEEP_OR_KILL：v0.1 膨胀句与 Goal 2 重复句清点

槽位：Round 1 fable-b。逐句判定，原句引用。判定词：KEEP（原样保留）/ REWRITE（改措辞不改语义边界）/ KILL（删除或迁出权威面）。

## 一、总判定

**产品锁本体没有发现 v0.1 膨胀**：13 条切片、砍/留表、「明确不做」清单、后期路线四处互相咬合，文件写执行三处（D19/D31/AC-27）一致标 v0.1.1。**Goal 1 权威文档内也没有发现 Goal 2 内容的重复**——Goal 2 被 D28 正确隔离在 `GOAL2_POLISH_PROMPT.md`，Goal 1 侧只有守卫句。真正要杀的是**陈旧引导句**：它们不膨胀范围，但会诱导新父代理重做已完成的工作（重新派 planner、重跑三轮扫描），这是另一种形式的范围爆炸。

## 二、逐句判定

### A. 膨胀嫌疑句（逐一排除或改写）

| 原句（出处） | 判定 | 理由 |
|---|---|---|
| 「帮你处理电脑/手机事务、起草回复、执行简单工作、分析生活中的人与事」（PRODUCT_LOCK 一句话） | **KEEP** | 作者愿景层，锁内被 13 条切片显式收窄（切片 9 只读、AC-27 隔离执行）；稀释它反而违反「作者原意不得稀释」 |
| 「行为数据可在你授权下用于行为预测研究」（PRODUCT_LOCK / README） | **KEEP** | 研究层段落已钉「v0.1 只预览、不写导出文件」；无膨胀 |
| 「离线训练入口不承诺效果」（PRODUCT_LOCK 自主拍板/砍留表） | **KEEP** | 与 D8 一致，仅 schema/fixture |
| 「后期路线 v0.1.1–v1.0」（PRODUCT_LOCK） | **KEEP** | 显式标注后期，是防膨胀的锚不是膨胀源 |
| AC-26「lint/test/schema/红线/打包绿」（FORMAL） | **REWRITE** | 「打包绿」按 BLOCKERS S5 改「package job 绿 + 作者签名安装包」；否则字面读会把 NSIS 拉进 CI（出网循环，反向膨胀 CI 面） |
| 「Goal 1 完成 = 本文件验收矩阵全部 v0.1 行通过」（FORMAL） | **REWRITE** | 单门关闭低于 BLOCKERS_FROZEN 的双门语义（矩阵+切片互不否决）；不改会让 T0 图谱借 26 行全绿宣布关闭——这是**隐性砍灵魂层**，比膨胀更危险 |

### B. Goal 2 重复嫌疑句

| 原句（出处） | 判定 | 理由 |
|---|---|---|
| 「Goal 2 二十轮不在本文件；见 `docs/GOAL2_POLISH_PROMPT.md`，Goal 1 关闭前不要启动」（FORMAL） | **KEEP** | 守卫句，非重复 |
| GOAL2_POLISH_PROMPT.md 全文（含「至少二十轮，每轮 10 个子代理」「打磨到 SOTA」） | **KEEP** | D28 要求的隔离文件，前置条件句完备；只要它不被并进 FORMAL 就无重复 |
| 「Goal 2 在 Goal 1 关闭前不要启动」（STATUS 下一步 / GOAL1_PLAN） | **KEEP** | 守卫句 |
| 「每轮重跑 Goal 1 CI 门禁」（GOAL2_POLISH_PROMPT） | **KEEP** | Goal 2 文件内引用 Goal 1 门禁是单向依赖，方向正确 |

### C. 陈旧引导句（本清单的真正 KILL 区；与 WP_DAG.md 第三节互为表里）

| 原句（出处） | 判定 | 理由 |
|---|---|---|
| 「当前几乎无代码。任务是按锁定方案把 Soul 做成……」（FORMAL 提示词正文） | **REWRITE** | 新父代理整份粘贴会从零重规划，重做 PR #2 已有的树；改为两线现状一句话 |
| 「### 接下来使用 Goal：先调用子代理 claude-fable-5-thinking-xhigh，根据项目计划和目的，将项目代码的编写拆分为多个子代理……」（FORMAL） | **KEEP + 加注** | 疑似作者原话，不删；加一行历史注记「planner 已执行，产出 `docs/GOAL1_PLAN.md`（Goal 1 线）」，防止重复执行 |
| 「开工第一动作：1. 若 STATUS 不是 PLAN_FROZEN…… 2. CreateGoal：Goal 1。3. 派 fable planner。4. planner 返回前不要大面积写业务代码」（FORMAL） | **REWRITE** | 四条全部已发生；保留会诱导重跑。替换为指向 BLOCKERS 工序表 |
| 「下一步：CreateGoal：Goal 1。派 fable planner，再按 DAG 派 opus 写码」（STATUS） | **KILL（替换）** | 同上，且 STATUS 是唯一进度权威，误导代价最高 |
| 「阻塞：无计划阻塞。WP01 需把 schema `$ref` 接到 `_defs` 并补泄漏 fixture」（STATUS） | **KILL（替换）** | 该工作在 Goal 1 线已完成并有 lock 钉死；留着会催生第二次 schema 接线 |
| `docs/PLAN_VERIFY_PROMPT.md` 全文 | **KILL（迁出权威面）** | 一次性提示词已执行完毕（R1–R3 落盘、结论 PLAN_FROZEN）；留在 docs/ 根会诱导重跑三轮扫描。迁往 `docs/scan-rounds/` 或加「已执行完毕，勿重跑」头。README 对应链接同步处理 |
| 「当前处于产品锁定阶段，还没有可运行的应用」（README） | **REWRITE** | 三态诚实（见 WP_DAG.md 第三节 #9）；README 是 30 秒入口，最不能撒谎的一行 |

### D. 单源风险（不杀，但登记）

| 现象 | 判定 | 理由 |
|---|---|---|
| FORMAL「产品锁定（不可改写）」段落复述锁的两句话摘要 | **KEEP + 登记** | 整份粘贴提示词需要自足，功能性重复可接受；登记为「锁改动时必须同步之处」，防漂移 |
| README 首段复述锁的一句话 | **KEEP + 登记** | 同上 |
| 「九 schema」计数（FORMAL 11.5 / D26）vs 实际 11 份文件 | **REWRITE** | 精确化为「九份业务 schema `$ref` `_defs`，另有 `_defs` 与 `soul-import-v1`」；防字面执行者删文件凑数 |

## 三、边界重申（本清单不做的事）

- 不建议删除或降格 AC-18/D31（作者已否决过一次删除提案）。
- 不建议为 F04c 加任何新门（DECISION §4.2 禁令）。
- 不建议把 `.agent_workspace/context/plan/` 类路径写进任何保留句——双源消除方向只有 docs/（SHARED_BRIEF 目标 3）。
