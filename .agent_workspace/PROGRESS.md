# Soul 仓库进度枢纽

## 活跃任务

| 任务 | 分支 | 进度文件 |
|---|---|---|
| **打磨和完善项目计划**（本会话） | `cursor/polish-project-plan-5280` | `.agent_workspace/plan-polish/PROGRESS.md` |
| 算法验证优化（已合 main） | `cursor/algo-verify-opt-a073`（PR #5 merged） | 下方「已冻结」 |

## 已冻结

`docs/algorithms/DECISION.md` → **ALGO_FROZEN**

保留：

1. **T4D** — `crates/soul-algo-tie`
2. **A0** — `crates/soul-algo-trait`

`cargo test --workspace` 在算法 crate 上绿。交叉验证后解释层已按计数说话，Unix 0 哨兵已修。
