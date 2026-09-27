MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# Round 2 / gpt-sol-b：集成栈与 BeadFlow 边界探针

核验时间：2026-08-26T03:14:13Z。仓库：`Xhhemoing/Soul`。事实源为带 `--repo Xhhemoing/Soul` 的 `gh pr list/view/diff`、GitHub PR files/tree API 的完整分页结果，以及 `git ls-remote`。未执行 `git checkout`、`git commit` 或 `git push`。

## 结论

1. R1 所述 PR #9–#56 共 48 个，数量成立；截至核验时其中 27 OPEN、21 MERGED。
2. BeadFlow 栈为 #16–#56，共 41 个 PR；其中 20 OPEN、21 MERGED。R1 的 `41 = 20 open + 21 merged` 成立。
3. 三层命名拓扑成立：
   - #14：`cursor/first-test-candidate-c441` → `cursor/soul-goal1-7b1c`
   - #15：`cursor/soul-integration-4a8e` → `cursor/first-test-candidate-c441`
   - #16：`cursor/beadflow-integration-c441` → `cursor/first-test-candidate-c441`
4. **#9–#56 中只有 #15 修改 Soul 产品锁 `docs/PRODUCT_LOCK.md`。** 修改为 Telegram 全量导出入口/形状与 Desktop 版本假设，共 `+2/-2`。#16 及其 BeadFlow 子 PR 均未改该锁。
5. **没有第二份 `PRODUCT.md`。** 三个命名分支的完整 tree 均只有 `docs/PRODUCT_LOCK.md`，不存在根 `PRODUCT.md`、`docs/PRODUCT.md` 或任意 `*/PRODUCT.md`。BeadFlow 的产品计划实际放在 `docs/bead/PLAN.md`，另有 `docs/bead/README.md`、`WORK_PACKAGES.md`；“第二个产品存在”不等于“已有第二份 PRODUCT.md”。
6. 全仓当前 OPEN PR = **34**。按可复现的二分口径（base/head/title 命中 bead/BeadFlow 即 BeadFlow）为 **BeadFlow 20 / 非 BeadFlow（Soul 侧）14**。后者含编排/审计 PR #11、#57，因此“14 个 Soul”是线路简称，不表示 14 个都是纯产品实现。
7. PR #2 当前为 **OPEN、非 draft、`mergeable=CONFLICTING`、`mergeStateStatus=DIRTY`**，没有上报 checks。

## 命名集成栈：PR 端点与远端尖端

| PR | GitHub head → base | PR 报告的 head/base OID | 状态 |
|---|---|---|---|
| #14 | `cursor/first-test-candidate-c441` → `cursor/soul-goal1-7b1c` | `85aae68` / `5309656` | OPEN |
| #15 | `cursor/soul-integration-4a8e` → `cursor/first-test-candidate-c441` | `a27cfd1` / `85aae68` | OPEN |
| #16 | `cursor/beadflow-integration-c441` → `cursor/first-test-candidate-c441` | `83818fc` / `85aae68` | OPEN、MERGEABLE/CLEAN |

`git ls-remote` 的实时 branch tips：

| ref | tip |
|---|---|
| `main` | `a0ec14b723c4ea7a6203b2faf6c4d40e8ad3a181` |
| `cursor/soul-goal1-7b1c` | `6d1058b1f20be76097254a81fe9b8b8bff8f6e1a` |
| `cursor/first-test-candidate-c441` | `85aae68c25758365e37c4ca589e0ddd9dd2dc118` |
| `cursor/soul-integration-4a8e` | `a27cfd1f9ff3e39e54181bee2a7db7f9244aae23` |
| `cursor/beadflow-integration-c441` | `83818fc9b09597dee68fbe0b70299787a94789cf` |

#14 的 PR base OID 是分叉点 `5309656`，而目标 ref `cursor/soul-goal1-7b1c` 现已前移到 `6d1058b`；这与 R1 所述“自 Goal 1 的 `5309656` 分叉”一致，不能把 PR base OID 误写成当前目标分支 tip。

## PR #9–#16 逐项 base/head

| PR | 状态 | base | head | 改 `docs/PRODUCT_LOCK.md` | 含第二份 `PRODUCT.md` |
|---:|---|---|---|---|---|
| 9 | OPEN | `cursor/goal1-unblock-a073` | `cursor/predict-algo-survey-a073` | 否 | 否 |
| 10 | OPEN | `cursor/goal1-unblock-a073` | `cursor/goal1-close-loop-a073` | 否 | 否 |
| 11 | OPEN | `main` | `cursor/orchestrator-prompt-c441` | 否 | 否 |
| 12 | OPEN | `cursor/soul-goal1-7b1c` | `cursor/goal1-closeout-c441-2d70` | 否 | 否 |
| 13 | OPEN | `cursor/soul-goal1-7b1c` | `cursor/goal1-build-audit-c441` | 否 | 否 |
| 14 | OPEN | `cursor/soul-goal1-7b1c` | `cursor/first-test-candidate-c441` | 否 | 否 |
| 15 | OPEN | `cursor/first-test-candidate-c441` | `cursor/soul-integration-4a8e` | **是** | 否 |
| 16 | OPEN | `cursor/first-test-candidate-c441` | `cursor/beadflow-integration-c441` | 否 | 否 |

## PR #17–#56 逐项 base/head

以下 40 个 PR 的 base **全部**是 `cursor/beadflow-integration-c441`；完整分页文件核验结果也全部是“未改 `docs/PRODUCT_LOCK.md`、未新增第二份 `PRODUCT.md`”。

| PR | 状态 | head |
|---:|---|---|
| 17 | MERGED | `cursor/bead-r1-ui-c441` |
| 18 | MERGED | `cursor/bead-r1-algo-c441` |
| 19 | OPEN | `cursor/bead-r1-map-c441` |
| 20 | MERGED | `cursor/bead-r1-ci-c441` |
| 21 | MERGED | `cursor/bead-r1-shell-c441` |
| 22 | MERGED | `cursor/bead-r1-shell-review-c441` |
| 23 | MERGED | `cursor/bead-r1-core-c441` |
| 24 | MERGED | `cursor/bead-r1-algo-ts-c441` |
| 25 | MERGED | `cursor/bead-r1-shell-sh-c441` |
| 26 | MERGED | `cursor/bead-r1-algo-ts-review-c441` |
| 27 | MERGED | `cursor/bead-r1-core-review-c441` |
| 28 | MERGED | `cursor/bead-r1-algo-align-c441` |
| 29 | MERGED | `cursor/bead-r2-data-c441` |
| 30 | MERGED | `cursor/bead-r1-align-review-c441` |
| 31 | MERGED | `cursor/bead-r2-data1-c441` |
| 32 | MERGED | `cursor/bead-r2-al-review-c441` |
| 33 | MERGED | `cursor/bead-r2-b04-ia-c441` |
| 34 | MERGED | `cursor/bead-r2-b05-ia-c441` |
| 35 | OPEN | `cursor/bead-r2-build-c441` |
| 36 | OPEN | `cursor/bead-r2-cov-c441` |
| 37 | OPEN | `cursor/bead-r2-b04-impl-c441` |
| 38 | MERGED | `cursor/bead-r2-b04-review-c441` |
| 39 | OPEN | `cursor/bead-r2-b05-impl-c441` |
| 40 | MERGED | `cursor/bead-r2-b05-review-c441` |
| 41 | OPEN | `cursor/bead-r2-b10-c441` |
| 42 | MERGED | `cursor/bead-r2-b10-review-c441` |
| 43 | MERGED | `cursor/bead-r3-b03-ia-c441` |
| 44 | OPEN | `cursor/bead-r3-b06-ia-c441` |
| 45 | OPEN | `cursor/bead-r3-b03-impl-c441` |
| 46 | OPEN | `cursor/bead-r3-b07-ia-c441` |
| 47 | OPEN | `cursor/bead-r3-b03-review-c441` |
| 48 | OPEN | `cursor/bead-r3-b06-impl-c441` |
| 49 | OPEN | `cursor/bead-r3-b06-review-c441` |
| 50 | OPEN | `cursor/bead-r3-b06-fix-c441` |
| 51 | OPEN | `cursor/bead-r3-b07-impl-c441` |
| 52 | OPEN | `cursor/bead-r3-b07-review-c441` |
| 53 | OPEN | `cursor/bead-r3-b08-ia-c441` |
| 54 | OPEN | `cursor/bead-r3-b08-impl-c441` |
| 55 | OPEN | `cursor/bead-r3-b08-review-c441` |
| 56 | OPEN | `cursor/bead-r3-al4-c441` |

## 产品锁与第二份 PRODUCT.md

不能只用 `gh pr view --json files` 的首批结果判断：#15 有 176 个 changed files，#16 有 195 个，前 100 个文件会漏掉按路径排在后面的 `docs/PRODUCT_LOCK.md`。对 #9–#56 的 REST files endpoint 做完整分页后，产品文件命中只有：

```text
#15  docs/PRODUCT_LOCK.md  modified
```

#15 的实际改动：

- v0.1 Telegram 导入口径从泛称 `Export chat history → result.json` 改为 `Settings → Advanced → Export Telegram data` 的全量导出，并明确单聊导出是另一形状、拒收。
- assumption 增加官方 schema、Desktop ≥4.1、`date_unixtime` 与 `contacts.list.user_id` 的约束说明。
- 变更量：2 additions / 2 deletions；不涉及 BeadFlow。

三个命名分支的完整 Git tree 均为 `truncated=false`：

| branch | 精确匹配 `PRODUCT(_LOCK)?.md` | `docs/PRODUCT_LOCK.md` blob |
|---|---|---|
| `cursor/first-test-candidate-c441` | 仅 `docs/PRODUCT_LOCK.md` | `5f85c42f47378aca873b4202f964a576dff5debb` |
| `cursor/soul-integration-4a8e` | 仅 `docs/PRODUCT_LOCK.md` | `06b3d3f8795bc7c5da0c2db0e24ca3897209c8ed` |
| `cursor/beadflow-integration-c441` | 仅 `docs/PRODUCT_LOCK.md` | `5f85c42f47378aca873b4202f964a576dff5debb` |

因此：

- Soul integration 的锁 blob 与其 base 不同，正是 #15 的修改。
- BeadFlow integration 的锁 blob 与 first-test-candidate 完全相同，证明 #16 未改 Soul 产品锁。
- BeadFlow 虽有 `docs/bead/PLAN.md`，但没有自己的 `PRODUCT.md`。R1 关于“同仓第二产品”的事实成立；若进一步推断“已有第二份 PRODUCT.md”，则不成立。

## OPEN PR 统计

仓库级 OPEN 总数：**34**。

- BeadFlow：**20** — #16、#19、#35、#36、#37、#39、#41、#44–#56。
- 非 BeadFlow / Soul 侧：**14** — #1、#2、#3、#4、#6、#7、#9–#15、#57。

目标范围 #9–#56 内：

- OPEN 27：Soul 侧 #9–#15 共 7；BeadFlow #16 加其开放子 PR 共 20。
- MERGED 21：全部是 BeadFlow 子 PR。
- #16–#56：41 = 20 OPEN + 21 MERGED。

## PR #2

`gh pr view 2 --repo Xhhemoing/Soul` 的最终快照：

```text
state=OPEN
isDraft=false
base=main@a0ec14b723c4ea7a6203b2faf6c4d40e8ad3a181
head=cursor/soul-goal1-7b1c@6d1058b1f20be76097254a81fe9b8b8bff8f6e1a
mergeable=CONFLICTING
mergeStateStatus=DIRTY
statusCheckRollup=[]
```

R1 的“PR #2 已从未合入升级为实际冲突风险”仍准确。

## 对 R1 的最终判定

| R1 要点 | Round 2 判定 |
|---|---|
| #9–#56 共 48 个 | 成立 |
| BeadFlow #16–#56 共 41 个，21 merged / 20 open | 成立 |
| first-test-candidate → Goal 1；Soul/BeadFlow integration → first-test-candidate | 成立 |
| BeadFlow 自称“不改写 Soul” | 就 #16 文件差异而言成立：未改 `docs/PRODUCT_LOCK.md` |
| 新集成栈整体没有动 Soul 产品锁 | **不成立**：#15 修改了 `docs/PRODUCT_LOCK.md` |
| `docs/PRODUCT.md` 不存在 | main 与三个命名集成分支均成立 |
| 第二个产品已有第二份 `PRODUCT.md` | **不成立**：实际权威候选是 `docs/bead/PLAN.md`，不是 `PRODUCT.md` |
| PR #2 `CONFLICTING / DIRTY` | 成立 |

