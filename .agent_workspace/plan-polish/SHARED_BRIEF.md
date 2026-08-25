# 计划打磨 — 共享任务书（三轮全读）

父代理：cursor-grok-4.6-high。仓库 `github.com/Xhhemoing/Soul`。
工作分支：`cursor/polish-project-plan-5280`（基于 `origin/main` @ `7b35bde`）。
本任务：**打磨和完善项目的计划**。不是写 Goal 1 应用，不是 Goal 2，不是重选算法。

回复**第一行**必须是：`MODEL_SLUG: <实际使用的 slug>`。禁止静默降级。指定模型不可用则首行写失败原因并停止。

## 仓库事实（2026-08-25）

| 线 | 尖端 | 内容 |
|---|---|---|
| `main`（本分支祖先） | `7b35bde` | 算法 crate + `docs/algorithms/`（`ALGO_FROZEN`：T4D + A0）。无 PRODUCT_LOCK |
| PR #1 `cursor/soul-product-lock-7b1c` | 已种子到本分支 `docs/` | `PLAN_FROZEN` 产品锁、DECISIONS D1–D31、FORMAL AC-01–AC-26、九 schema、STATUS 仍写「实现未开始」 |
| PR #2 `cursor/soul-goal1-7b1c` | 远程，**只读对照** | Goal 1 实现主干；STATUS 很长；schema 已 `$ref` `_defs` 并有 `schemas.lock.json`；`tie_strength` 仍是裸 `object` |
| PR #5 | 已合进 main | 算法冻结 |
| PR #6 `cursor/blockers-analysis-a073` | 远程，只读 | `docs/BLOCKERS.md`（`BLOCKERS_FROZEN`） |
| PR #7 `cursor/goal1-unblock-a073` | 远程进行中 | Goal 1 吸收 T4D；**不要改那条线的代码** |

本分支已从 PR #1 checkout 的文档：`docs/PRODUCT_LOCK.md`、`docs/DECISIONS.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、`docs/SECURITY.md`、`docs/schemas/*`（尚无 `schemas.lock.json`）、`docs/scan-rounds/`、`README.md`。`docs/algorithms/` 来自 main，保持。

只读对照（`git show origin/<branch>:<path>`，禁止把 Goal 1 整树拷进来）：

- `origin/cursor/soul-goal1-7b1c:docs/STATUS.md`
- `origin/cursor/soul-goal1-7b1c:docs/GOAL1_PLAN.md`
- `origin/cursor/soul-goal1-7b1c:docs/schemas/`
- `origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`
- `docs/algorithms/DECISION.md`（工作区已有）

## 目标（不可稀释）

产出一套能合进 `main` 的**计划权威面**：

1. 产品锁仍是作者原意；v0.1 切片不膨胀。
2. 计划吸收 `ALGO_FROZEN`（人脉=T4D，特质=A0；A2 是渲染器；A1 在 v0.1 空转是预期）。
3. 消除双源：`.agent_workspace/context/plan/` 不得再当权威；权威只在 `docs/`。
4. STATUS 诚实：计划冻结 ≠ Goal 1 关闭 ≠ main 已有应用。
5. FORMAL 验收矩阵补上算法相关、可测的 Given/When/Then，且不与 PRODUCT_LOCK 13 条切片互否。
6. schema 与冻结算法对齐的**计划义务**写清（relationship.tie_strength 字段、direct vs group 分列、as_of 纪律）；若改 schema 正文，必须说明对 Goal 1 `schemas.lock.json` 的影响。
7. 工作包只减不增；WP12 保持删除；文件写执行仍是 v0.1.1。
8. README / PLAN 索引让后来者 30 秒内找到权威文件。

## 硬约束

- 禁止第二份 `PRODUCT.md`。
- 禁止改产品方向（不做通用助手、不做恋人、不代发、无 E0、不抓取）。
- 禁止为 F04c 加第三道降档门。
- 禁止把 SQLCipher / Tauri / HTTP 拉进算法 crate。
- 禁止 `git commit` / `git push` / 开 PR（父代理负责）。
- 只允许写入你的专属目录（见当轮派发）。父代理之后才合并进 `docs/`。
- 代码若需要，仅限你目录内的探针/校验脚本；Rust 1.83；无 `unsafe`；无外网。
- Linux 可测。不要假设本环境有完整 Goal 1 工作区。

## 评价（计划文档，不是算法重选）

| 维 | 含义 |
|---|---|
| P1 单源 | 同一事实只在一个权威文件 |
| P2 可测 | 每条门禁有 Given/When/Then 与谁跑 |
| P3 冻结吸收 | T4D/A0 出现在锁/拍板/验收，而不是只在 algorithms/ |
| P4 现状诚实 | 不把 Goal 1 未合入写成「未开始」或「已关闭」 |
| P5 不膨胀 | v0.1 不把 v0.1.1/v0.2/Goal 2 偷加回来 |
| P6 SOTA | 文档结构达到可给新父代理整份粘贴开工的程度 |
