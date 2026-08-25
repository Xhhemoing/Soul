# 分支图（第一次正式测试候选）

核于 2026-08-25。对照 `origin` 当时的 tip。权威产品树是 **`cursor/soul-goal1-7b1c`（PR #2）**。
本页是过程图，不改 `PRODUCT_LOCK` / `DECISIONS` / 验收矩阵。

「合并分支」在这里的意思：先分类，再只合**已经接在主干上、且 merge-tree 干净**的继续。
不是把所有 remote 压成一棵树。

## 候选树

| 树 | tip（origin） | 相对主干 | 处置 |
|---|---|---|---|
| `cursor/soul-goal1-7b1c` | `5309656`（产品 `478f19f` 是祖先） | 自身 | **第一次正式测试默认打这一棵。** |
| `cursor/goal1-build-audit-c441`（PR #13） | `9b4bcd4` | 超前 41、落后 0、merge-tree 0 冲突 | 主干的继续，**可以快进合入主干**。父代理本机另超前 origin 20+ 个提交（R3 姓名占位与 R4 文档），推送仍被 HTTP 401 挡着。 |
| `cursor/goal1-closeout-c441-2d70`（PR #12） | `9eeef94` | 超前 4、落后 1、`docs/STATUS.md` 双方都改 | **不要盲合。** 空态修正已在 audit 上另有提交。 |

## 禁止合进主干

| 树 | 原因 |
|---|---|
| `origin/main` / `cursor/polish-project-plan-5280` | AC-28+ 计划冻结。与 Goal 1 树**故意**冲突（PR #2 正文已写）。用合 `main` 来「修好冲突」会换合同。 |
| `agent/dev-sota`（PR #4） | 另一条产品分叉（T4D 绑边等）。停。 |
| `cursor/goal1-unblock-a073`（PR #7） | T4D/A0 接到 `main` 拓扑。N6：只有作者能裁是否吸收。 |

`main` 与唯一主干 **没有**可用的 `merge-base`（历史根不同：主干根是 `d510f1e`，`main` 家族根是 `ea6f62f` Initial commit）。凡从 `main` 长出来的过程枝，对主干做 `merge-base` 都会失败——这不是漏抓，是两棵合同树。另有两条**独根**过程枝连 `main` 也对不上：`cursor/blockers-analysis-a073`（单提交 `e669b78`）与 `cursor/predict-algo-survey-a073`（自带 30 提交史，根 `b3cb6d4`）。

## 过程枝（归档，不合入）

这些枝相对主干「超前」只是另一份历史，不是可以快进的功能：

`cursor/algo-verify-opt-a073`、`cursor/blockers-*`、`cursor/predict-algo-survey-a073`、`cursor/roundx-opus-b-poll-c71f`、`cursor/soul-product-lock-7b1c`、`cursor/soul-status-round1-665b`、`cursor/orchestrator-prompt-c441`（PR #11，1 个编排模板提交，基线是 `main`）。

需要那份编排模板时：在 `main` 上留着 PR #11，不要 cherry-pick 进产品主干。

## 全量清单（origin，2026-08-25 核账时点）

每一行都用 `git rev-list --count` 与 `git merge-base` 对主干 `5309656` 核过；「无 merge-base」是命令失败，不是没有记录。

| 分支 | tip | 末次提交 | 对主干 | 家族 |
|---|---|---|---|---|
| `cursor/soul-goal1-7b1c` | `5309656` | 08-25 | 主干本身（30 提交） | 主干，根 `d510f1e` |
| `cursor/goal1-build-audit-c441` | `9b4bcd4` | 08-25 | +41 / −0，可快进 | 主干 |
| `cursor/goal1-closeout-c441-2d70` | `9eeef94` | 08-25 | +4 / −1，叉于 `a9a7490`，`docs/STATUS.md` 双改 | 主干 |
| `main` | `a0ec14b` | 08-25 | 无 merge-base（**禁止合入**） | `main`，根 `ea6f62f` |
| `cursor/polish-project-plan-5280` | `a0ec14b` | 08-25 | 无 merge-base | 与 `main` 同 tip |
| `cursor/orchestrator-prompt-c441` | `7ca4894` | 08-25 | 无 merge-base | `main` + 1（PR #11） |
| `cursor/goal1-unblock-a073` | `6133307` | 08-25 | 无 merge-base（**禁止合入**，N6） | `main` + 30 |
| `agent/dev-sota` | `24c539f` | 08-25 | 无 merge-base（**禁止合入**） | `main` 家族，叉于 `7b35bde`，+81 |
| `cursor/blockers-r2-fable-a-7d67` | `0d8c8a9` | 08-24 | 无 merge-base | `main` 家族，叉于 `7b35bde` |
| `cursor/blockers-r2-opus-a-1efe` | `73f8d6f` | 08-24 | 无 merge-base | 同上 |
| `cursor/blockers-r2-opus-b-52e4` | `71f33a3` | 08-24 | 无 merge-base | 同上 |
| `cursor/blockers-r3-opus-a-52bd` | `cbbbe9e` | 08-24 | 无 merge-base | 同上 |
| `cursor/blockers-r3-opus-b-e326` | `6071240` | 08-24 | 无 merge-base | 同上 |
| `cursor/roundx-opus-b-poll-c71f` | `cb89814` | 08-24 | 无 merge-base | 同上 |
| `cursor/algo-verify-opt-a073` | `d79c8ba` | 08-24 | 无 merge-base | `main` 家族，叉于根 `ea6f62f` |
| `cursor/soul-product-lock-7b1c` | `a785317` | 08-24 | 无 merge-base | 同上 |
| `cursor/soul-status-round1-665b` | `0d57141` | 08-24 | 无 merge-base | 同上 |
| `cursor/blockers-analysis-a073` | `e669b78` | 08-24 | 无 merge-base | 独根单提交 |
| `cursor/predict-algo-survey-a073` | `2b0b555` | 08-25 | 无 merge-base | 独根 `b3cb6d4`（30 提交） |

## 本轮实际要做的合并

1. **不要**合 #4 / #7 / `main`。`scripts/branch-disposition.sh` 对这三名 `--merge` 一律 exit 2。
2. **已快进并推 origin：** 从 `origin/cursor/soul-goal1-7b1c`（`5309656`）快进到 audit。`cursor/first-test-candidate-c441` 与 `cursor/goal1-build-audit-c441` 同尖端、都在 origin。`--merge` 对 `agent/dev-sota` / `main` / closeout 均 exit 2。closeout 仍是 `UNSAFE`（`docs/STATUS.md` 冲突）。
3. PR #12 不必再合。`cursor/first-test-candidate-c441` 与 `cursor/goal1-build-audit-c441` 已推 origin。作者决定是否把这一快进合进 PR #2。
