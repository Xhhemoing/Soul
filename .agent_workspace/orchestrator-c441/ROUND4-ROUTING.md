# ROUND 4 路由 — 分支盘点 / 合并处置 / 第一次正式测试方案

日期：2026-08-25。父代理：cursor-grok-4.6-high。Goal：了解每个分支并合并可安全合入的分支，然后为第一次正式测试给出方案。

## 路由

AUTO → SCAN/PLAN（本轮）+ 合并处置（只合已证明安全的）+ 第一次正式测试方案。LOOP20 仍 QUEUED。不重开 `PRODUCT_LOCK` / D1–D60 / WP01–WP13。不启动 Goal 2。不做 AC-27。

「合并分支」**不是**把所有 remote 压进一条树。先出合并图，再只执行：已是子集、或 docs-only 且无冲突的合入。下列分叉**禁止**合进唯一主干，除非作者另批：

- `agent/dev-sota`（PR #4）
- `cursor/goal1-unblock-a073`（PR #7，T4D/A0，N6）
- `origin/main` → 唯一主干（AC-28+ 计划冻结与 Goal 1 树**故意**冲突）

第一次正式测试的候选树：`cursor/soul-goal1-7b1c`（PR #2）。本机 `cursor/goal1-build-audit-c441` @ `25b0aaa` 含未推送的 R3，origin 仍停在 `9b4bcd4`。

## Round 1 六路（云端，禁止同文件）

| 代理 | 模型 | 产物 | 云端 id |
|---|---|---|---|
| fable-a | claude-fable-5-thinking-xhigh | `docs/BRANCH_MAP.md` | `bc-963470bd-68be-58f2-ab2c-2d07bcc1c167` |
| fable-b | claude-fable-5-thinking-xhigh | `docs/FIRST_FORMAL_TEST.md` | `bc-85764013-e04d-5cb6-9558-ea28a4673b42` |
| opus-a | claude-opus-5-thinking-high-fast | `scripts/branch-disposition.sh`（及必要测试） | `bc-6b504d02-c2aa-57ac-8e1c-99d3bf37c1d2` |
| opus-b | claude-opus-5-thinking-high-fast | 只改 `scripts/author-manual-checklist.md` 第 0 节：点名候选树 | `bc-8170e09a-21a7-5d6d-884d-00087329e6a0` |
| gpt-sol-a | gpt-5.6-sol-xhigh-fast | `.agent_workspace/orchestrator-c441/round4/gpt-sol-a.md` | `bc-551bc314-dac0-5819-b32b-f0eeb7146e62` |
| gpt-sol-b | gpt-5.6-sol-xhigh-fast | `.agent_workspace/orchestrator-c441/round4/gpt-sol-b.md` | `bc-ef8b00bd-7407-5b0c-a432-eaec582ae6b4` |

## 本机已知 remote（派单时点，可能落后 1–2h）

见 `git branch -a`。主干 tip `5309656`。closeout 相对主干多 4 个提交且 `merge-tree` 有 `changed in both`，不得盲合。
