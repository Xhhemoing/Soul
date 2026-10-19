# Soul

灵魂级个人软件。根据你授权的社交档案、电脑操作和日常记录，在你自己的设备上复刻电子版的你——人格、记忆、心理倾向、人脉图——并辅助处理电脑事务、起草回复、分析人与事。本机采集与浅层处理；云端深度分析默认关闭。行为数据可在授权下用于行为预测研究。

平台：v0.1 在 Windows 11 上开发与验收；核心后续部署于 Linux / macOS / Windows；接入端（数据采集与结果呈现）含 macOS / Linux / Windows / Android（DECISIONS D65）。

状态（2026-10-19）：开发分支 `codex/soul-rebuild-20261019` 已实现统一事项、显式目标、持久待办与恢复、提醒和通知、默认关闭的本地模板准备及明确反馈（U1–U6）。本轮重建与合成集成验证按[原计划](docs/design/2026-10-16-unified-assistance.md)推进，实际检查与同步记录见 [STATUS](docs/STATUS.md) 和[实施验证](docs/design/2026-10-19-unified-assistance-implementation.md)。Windows 真机通知、原生桌面及完整平台验收和真实任务效用仍未完成；Goal 1、正式 Goal 2 与 Wave 1 均未关闭，开发分支未合并。

架构导航：[离线 HTML 架构地图](docs/architecture.html)（下载后用浏览器打开；四图切换、节点说明、缩放、模块筛选和图源下载），或阅读 [系统架构与设计图](docs/ARCHITECTURE.md)（附 Mermaid 源文件与 SVG）。HTML 是固定在 `b47ddfa` 的单文件说明快照，不自动联网。设计依据和取舍见 [架构审查报告](docs/reviews/2026-10-15-architecture-review.md)。

后续框架设计（2026-10-15）：[使用闭环与算法交互 HTML](docs/framework-design.html) / [研究与具体设计](docs/design/2026-10-15-usage-framework.md)。从使用场景比较候选方案，提供任务控制、证据选择和纠正优先的可执行参考模型；37 项模型测试、6 个定向变异与 13 项离线页检查，见[验证与独立评审](docs/design/2026-10-15-framework-validation.md)。这是未接入生产的设计实验，不扩大上述产品测试或正式验收范围。

历史展示：[架构与项目进度展示页](docs/project-overview.html)（单文件 HTML，保存后用浏览器打开；包含架构、18 个 Rust 模块、能力筛选、验证账本与剩余任务）。页面是固定在 `6a8ad64` 的说明快照，不是 Soul 客户端，不实时更新，不读取个人数据，不自动联网。最新架构见上述架构导航，最新状态以 STATUS 和对应源码证据为准。

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
