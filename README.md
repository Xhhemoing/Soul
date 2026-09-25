# Soul

灵魂级个人软件。根据你授权的社交档案、电脑操作和日常记录，在你自己的设备上复刻电子版的你——人格、记忆、心理倾向、人脉图——并辅助处理电脑事务、起草回复、分析人与事。本机采集与浅层处理；云端深度分析默认关闭。行为数据可在授权下用于行为预测研究。

平台：v0.1 在 Windows 11 上开发与验收；核心后续部署于 Linux / macOS / Windows；接入端（数据采集与结果呈现）含 macOS / Linux / Windows / Android（DECISIONS D65）。

状态：唯一发布候选工作线为 `codex/release-candidate-20260925`，产品源码基线是独立集成源码 `ebff0c9`；当前根工作树的文档收尾分支不是发布候选。`ebff0c9` 的 Windows 自动门禁与补充检查已通过，但同源码 NSIS 的外部证据目录当前缺失，G-M 0 作者手动打包、真实安装/卸载、其余人工验收和同源码 Linux 门禁均未齐，Goal 1 未关闭。后续产品源码修改不沿用旧门禁结论。见 docs/STATUS.md。

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
