# Soul

灵魂级个人软件。根据你授权的社交档案、电脑操作和日常记录，在你自己的设备上复刻电子版的你——人格、记忆、心理倾向、人脉图——并辅助处理电脑事务、起草回复、分析人与事。本机采集与浅层处理；云端深度分析默认关闭。行为数据可在授权下用于行为预测研究。

平台：v0.1 在 Windows 11 上开发与验收；核心后续部署于 Linux / macOS / Windows；接入端（数据采集与结果呈现）含 macOS / Linux / Windows / Android（DECISIONS D65）。

状态（2026-09-29）：Goal 1、正式 Goal 2 与 Wave 1 均未关闭。当前根工作树 `codex/branch-convergence-20260927` 已合并 GitHub 最新遗忘结果分支（远程提交 `9d3cd30`）与本地集成线（`f911980`）；新遗忘服务、核心回执、原生 IPC 与桌面页面接线，并新增仅重试日志清理入口。只有离线局部诊断通过，真实组件、原生测试、完整门禁、独立评审和主干集成仍未完成。销毁前影响面重新校验与独立撤权执行模型仍待补。见 [docs/STATUS.md](docs/STATUS.md)。

快速理解：[架构与项目进度展示页](docs/project-overview.html)（单文件 HTML，保存后用浏览器打开；包含架构、18 个 Rust 模块、能力筛选、验证账本与剩余任务）。页面是固定在 `6a8ad64` 的说明快照，不是 Soul 客户端，不实时更新，不读取个人数据，不自动联网。最新状态以 STATUS 和对应源码证据为准；本次合并不改变该快照的历史基线。

历史：独立集成源码 `ebff0c9` 有 Windows 自动门禁记录；同源码 NSIS 的外部证据目录缺失，人工验收与同源码 Linux 门禁未齐。旧门禁结论不覆盖后续源码；此前暂缓 G-L、跳过 NSIS 安装／卸载与 G-M 的指示保持不变。

- 产品锁定：[`docs/PRODUCT_LOCK.md`](docs/PRODUCT_LOCK.md)
- AI 协作与工作区入口：[`AGENTS.md`](AGENTS.md)（项目规则，不替代产品与验收权威）
- 长期路线与并行交付：[docs/ROADMAP.md](docs/ROADMAP.md)
- 决策记录：[`docs/DECISIONS.md`](docs/DECISIONS.md)
- 进度：[`docs/STATUS.md`](docs/STATUS.md)
- Goal 1 验收矩阵与红线：[`docs/ACCEPTANCE.md`](docs/ACCEPTANCE.md)
- Goal 1 实现规划：[`docs/GOAL1_PLAN.md`](docs/GOAL1_PLAN.md)
- Goal 2 规划（Goal 1 关闭后）：[`docs/GOAL2_PLAN.md`](docs/GOAL2_PLAN.md)
- 安全规范：[`docs/SECURITY.md`](docs/SECURITY.md)
- Schema：[`docs/schemas/`](docs/schemas/)
- 本地门禁记录：[`docs/gates/`](docs/gates/)
