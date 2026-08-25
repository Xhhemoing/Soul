# Completion audit — 了解分支 / 安全合并 / 第一次正式测试方案

核于 `a1f0e51` on `cursor/first-test-candidate-c441`。目标句没有要求 hosted 绿、Win11 勾完、或 origin 推送。

| 要求 | 证据 | 判定 |
|---|---|---|
| 了解每个分支 | `docs/BRANCH_MAP.md`；`scripts/branch-disposition.sh` 列出全部 `origin/*` | 成立 |
| 合并可安全合入的 | `origin/cursor/soul-goal1-7b1c` 是 HEAD 祖先；候选栈 = 主干快进 + audit | 成立（本机） |
| 不合入不安全的 | `--merge main` / `agent/dev-sota` exit 2；closeout / sota 不是 HEAD 祖先 | 成立 |
| 准备第一次正式测试 | `scripts/first-formal-test-linux.sh`（本机已绿）、`scripts/first-formal-test-win11.ps1`、`scripts/author-manual-checklist.md`、bundle verify | 成立 |
| 给出方案 | `docs/FIRST_FORMAL_TEST.md` | 成立 |

未做（非本目标句）：origin 推送（令牌 401）、hosted AC-26、作者 Win11 勾选。
