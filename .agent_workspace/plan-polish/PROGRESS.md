# 打磨和完善项目计划 — 进度

- 分支：`cursor/polish-project-plan-5280`（云端命名约束；用户 SOP 的 `agent/<task-name>` 映射为此名）
- 基线：`origin/main` @ `7b35bde`（已含 `ALGO_FROZEN`）
- 计划种子：`origin/cursor/soul-product-lock-7b1c`（PR #1，`PLAN_FROZEN`）
- 循环：Round 1 完成，Round 2 待开始
- PR：https://github.com/Xhhemoing/Soul/pull/8

## 本轮目标

把已冻结但未合入 `main`、且未吸收算法冻结的 v0.1 计划打磨成单一权威、可合入、可验收的文档集。不实现 Goal 1 应用代码。不重开产品方向。不启动 Goal 2。

## 里程碑

| 轮 | 状态 | 产出 |
|---|---|---|
| 准备 | 完成 | 分支 + 种子 docs + 本文件 |
| Round 1 | 完成 | 见 `R1-SYNTHESIS.md`；权威文档已合入 `docs/` |
| Round 2 | 待开始 | 靶向重构与文档落地 |
| Round 3 | 待开始 | SOTA 验收与交叉核验 |

## 硬约束

- `docs/PRODUCT_LOCK.md` 是唯一产品权威；禁止第二份 `PRODUCT.md`
- 算法权威是 `docs/algorithms/DECISION.md`（T4D + A0）
- 子代理只写自己的目录，禁止 `git commit` / `git push` / 开 PR
- 禁止静默降级；回复首行 `MODEL_SLUG: <实际 slug>`
