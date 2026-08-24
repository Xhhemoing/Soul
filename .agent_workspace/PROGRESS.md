# Soul 项目盘点 — 进度文档

**任务**：单轮总结本项目目前情况，并简单分析接下来可以做什么。  
**分支**：`cursor/soul-status-round1-665b`（基于 `main`）  
**循环**：本目标明确要求「先单轮」，因此只执行 Round 1（6 并发子代理），不进入 Round 2/3 实现冲刺。  
**工作区约束**：当前 checkout 是几乎空的 `main`（仅 `README.md` 一行 `# Soul`）。真实产物在远程分支，子代理必须用 `git show` / `git ls-tree` 读取，**禁止 checkout 其它分支**。

## 仓库拓扑（父调度器基线）

| 引用 | 角色 | 状态 |
|---|---|---|
| `origin/main` | 默认主干 | 仅 `ea6f62f Initial commit` + `# Soul` |
| `origin/cursor/soul-product-lock-7b1c` | 产品锁定 / 计划冻结 | Draft PR [#1](https://github.com/Xhhemoing/Soul/pull/1) |
| `origin/cursor/soul-goal1-7b1c` | Goal 1 实现（WP01–WP09 壳） | Draft PR [#2](https://github.com/Xhhemoing/Soul/pull/2)，~293 文件 / +44523 行 |

产品一句话：本机复刻电子版的你（人格、记忆、心理倾向、人脉图），Windows 本地优先，采集默认关，v0.1 零业务出网（E0）。权威文档：`docs/PRODUCT_LOCK.md`。

## Goal 1 DAG 进度（来自 goal1 分支 `docs/STATUS.md`）

已完成：WP01 骨架、WP02 加密存储、WP08 权限/出网、WP03 档案、WP04 记忆、WP05 人脉图、WP06 导入、WP07 前台采集、WP09 桌面壳第一段。  
未开始：WP10 起草、WP11 只读文件计划、WP09 功能视图、WP13 收尾。  
明确禁止：Goal 1 关闭前不要启动 Goal 2；文件写执行属 v0.1.1。

## Round 1 子代理

| ID | 模型 | 输出文件 | 主攻 | 状态 |
|---|---|---|---|---|
| r1-fable-architecture | fable / `claude-fable-5-thinking-xhigh` | `.agent_workspace/round1/r1-fable-architecture.md` | 全局架构与产品锁定对照 | 完成 |
| r1-fable-sota-next | fable / `claude-fable-5-thinking-xhigh` | `.agent_workspace/round1/r1-fable-sota-next.md` | SOTA 差距与下一步优先级 | 完成 |
| r1-opus-core-inventory | opus-fast / `claude-opus-5-thinking-high-fast` | `.agent_workspace/round1/r1-opus-core-inventory.md` | 核心 crate/命令面实证盘点 | 完成 |
| r1-opus-gap-debt | opus-fast / `claude-opus-5-thinking-high-fast` | `.agent_workspace/round1/r1-opus-gap-debt.md` | 接缝、债务、剩余 WP | 完成 |
| r1-sol-env-probe | gpt-sol / `gpt-5.6-sol-xhigh-fast` | `.agent_workspace/round1/r1-sol-env-probe.md` | 环境/分支/CI 探针 | 完成 |
| r1-sol-test-boundary | gpt-sol / `gpt-5.6-sol-xhigh-fast` | `.agent_workspace/round1/r1-sol-test-boundary.md` | 测试边界与未覆盖面 | 完成 |

## 状态

- [x] 隔离分支创建
- [x] Round 1 六子代理完成（均声明实际 slug，无静默降级）
- [x] Round 1 结论简报：`.agent_workspace/round1/R1-SYNTHESIS.md`
- [ ] PR 提交

父仲裁要点：双问卷不能按 STATUS「import 零改」合并；DPAPI 阻塞 AC-01 但不阻塞批 5 核心 crate；不合并 PR #1/#2。
