# Soul

灵魂级个人软件。根据你授权的社交档案、电脑操作和日常记录，在你自己的设备上复刻电子版的你——人格、记忆、心理倾向、人脉图——并辅助处理电脑事务、起草回复、分析人与事。本机采集与浅层处理；云端深度分析默认关闭。行为数据可在授权下用于行为预测研究。

平台：v0.1 在 Windows 11 上开发与验收；核心后续部署于 Linux / macOS / Windows；接入端（数据采集与结果呈现）含 macOS / Linux / Windows / Android（DECISIONS D65）。

状态：唯一发布候选工作线为 `codex/release-candidate-20260925`，产品源码基线是独立集成源码 `ebff0c9`；当前根工作树是独立规划线，不是发布候选。`ebff0c9` 的 Windows 自动门禁数字、时间、哈希与退出码仅保留为历史文字记录：原集成 worktree、其中的 `target/q2` 及六个关键原始证据文件当前均不存在，因此完整 G-W 为 `NOT VERIFIED`，必须在候选上重跑。历史 `ebff0c9` 同源码代理 NSIS 构建和配对 `-SkipInstall` 的声称同样因原始证据缺失为 `NOT VERIFIED`；当前唯一候选的 NSIS 为 `NOT RUN`，作者 G-M 0 与真实安装/卸载为 `NOT RUN`，同源码 G-L 为 `NOT RUN` / 暂缓，其余 G-M 人工验收未完成，Goal 1 未关闭。候选上的定向测试不能冒充完整 G-W。见 docs/STATUS.md。

- 产品锁定：[`docs/PRODUCT_LOCK.md`](docs/PRODUCT_LOCK.md)
- 长期路线与并行交付：[docs/ROADMAP.md](docs/ROADMAP.md)
- 决策记录：[`docs/DECISIONS.md`](docs/DECISIONS.md)
- 进度：[`docs/STATUS.md`](docs/STATUS.md)
- Goal 1 验收矩阵与红线：[`docs/ACCEPTANCE.md`](docs/ACCEPTANCE.md)
- Goal 1 实现规划：[`docs/GOAL1_PLAN.md`](docs/GOAL1_PLAN.md)
- Goal 2 规划（Goal 1 关闭后）：[`docs/GOAL2_PLAN.md`](docs/GOAL2_PLAN.md)
- 安全规范：[`docs/SECURITY.md`](docs/SECURITY.md)
- Schema：[`docs/schemas/`](docs/schemas/)
- 本地门禁记录：[`docs/gates/`](docs/gates/)
