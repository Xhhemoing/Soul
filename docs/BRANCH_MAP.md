# 分支图（第一次正式测试候选）

核于 2026-08-25。对照 `origin` 当时的 tip。权威产品树是 **`cursor/soul-goal1-7b1c`（PR #2）**。
本页是过程图，不改 `PRODUCT_LOCK` / `DECISIONS` / 验收矩阵。

「合并分支」在这里的意思：先分类，再只合**已经接在主干上、且 merge-tree 干净**的继续。
不是把所有 remote 压成一棵树。

## 候选树

| 树 | tip（origin） | 相对主干 | 处置 |
|---|---|---|---|
| `cursor/soul-goal1-7b1c` | `5309656`（产品 `478f19f` 是祖先） | 自身 | **第一次正式测试默认打这一棵。** |
| `cursor/goal1-build-audit-c441`（PR #13） | `9b4bcd4` | 超前 41、落后 0、merge-tree 0 冲突 | 主干的继续，**可以快进合入主干**。父代理本机还超前 origin 约 20 个提交（R3 姓名占位等），origin 未跟上。 |
| `cursor/goal1-closeout-c441-2d70`（PR #12） | `9eeef94` | 超前 4、落后 1、`docs/STATUS.md` 双方都改 | **不要盲合。** 空态修正已在 audit 上另有提交。 |

## 禁止合进主干

| 树 | 原因 |
|---|---|
| `origin/main` / `cursor/polish-project-plan-5280` | AC-28+ 计划冻结。与 Goal 1 树**故意**冲突（PR #2 正文已写）。用合 `main` 来「修好冲突」会换合同。 |
| `agent/dev-sota`（PR #4） | 另一条产品分叉（T4D 绑边等）。停。 |
| `cursor/goal1-unblock-a073`（PR #7） | T4D/A0 接到 `main` 拓扑。N6：只有作者能裁是否吸收。 |

`main` 与唯一主干 **没有**可用的 `merge-base`（历史根不同）。凡从 `main` 长出来的过程枝，对主干做 `merge-base` 都会失败——这不是漏抓，是两棵合同树。

## 过程枝（归档，不合入）

这些枝相对主干「超前」只是另一份历史，不是可以快进的功能：

`cursor/algo-verify-opt-a073`、`cursor/blockers-*`、`cursor/predict-algo-survey-a073`、`cursor/roundx-opus-b-poll-c71f`、`cursor/soul-product-lock-7b1c`、`cursor/soul-status-round1-665b`、`cursor/orchestrator-prompt-c441`（PR #11，1 个编排模板提交，基线是 `main`）。

需要那份编排模板时：在 `main` 上留着 PR #11，不要 cherry-pick 进产品主干。

## 本轮实际要做的合并

1. **不要**现在合 #4 / #7 / `main`。
2. 凭据恢复后：把本机 `cursor/goal1-build-audit-c441` 推上 origin，再让作者决定是否把 PR #13 合进 PR #2。合进去之前，第一次正式测试仍按清单从 **`478f19f` 或之后的唯一主干**打包装；若作者要测 R3 诚实文案，改为打 **audit 尖端**（须先能推送）。
3. PR #12 保持开着或关闭，不必再合。
