# Soul

灵魂级个人软件。根据你授权的社交档案、电脑操作和日常记录，在你自己的设备上复刻电子版的你——人格、记忆、心理倾向、人脉图——并辅助处理电脑事务、起草回复、分析人与事。本机采集与浅层处理；云端深度分析默认关闭。行为数据可在授权下用于行为预测研究。

v0.1 在 Windows 11 上开发与验收；核心后续部署于 Linux / macOS / Windows，接入端含 macOS / Linux / Windows / Android（D65）。产品边界见 [PRODUCT_LOCK](docs/PRODUCT_LOCK.md)。

## 从这里开始

- **继续开发：**读 [AGENTS.md](AGENTS.md)，再看 [当前状态](docs/STATUS.md) 和 [按任务查阅的计划索引](docs/PLAN_INDEX.md)。
- **确认进度：**只以 [STATUS](docs/STATUS.md) 的当前摘要及对应版本证据为准；详细旧回执在 [历史目录](docs/history/)。
- **确认范围：**[验收矩阵](docs/ACCEPTANCE.md)、[安全规范](docs/SECURITY.md)、[已定决定](docs/DECISIONS.md)。
- **规划后续：**[长期路线](docs/ROADMAP.md)、[Goal 1 计划](docs/GOAL1_PLAN.md)、[Goal 2 计划](docs/GOAL2_PLAN.md)。

Goal 1、U7、正式 Goal 2 与 Wave 1 尚未关闭。G-L 暂缓，完整 G-M、安装卸载和真实任务效用仍未齐；开发验证、平台门禁、合入与发布分别记录，旧版本的通过结论不覆盖新源码。

## 项目目录

```text
Soul/
├── AGENTS.md         开发规则与项目记忆维护
├── .codex/           本项目实现 / 只读评审角色
├── crates/           根 Rust workspace（19 个 crate）
├── apps/desktop/     React UI；src-tauri 为独立 Cargo workspace
├── fixtures/         合成测试材料
├── scripts/          Windows 检查与人工验收入口
└── docs/
    ├── PLAN_INDEX.md 按问题定位规范与计划
    ├── STATUS.md     当前状态摘要
    ├── design/       方案、实施计划与验证回执
    ├── history/      历史状态与阶段记录
    ├── reviews/      审查及修复记录
    ├── gates/        按提交和平台记录的门禁证据
    ├── algorithms/   算法与话术权威
    ├── schemas/      数据契约与既有 lock
    └── architecture/ 架构图源与导出图
```

Rust 工具链以 [rust-toolchain.toml](rust-toolchain.toml) 为准，前端使用 pnpm 锁文件。检查命令和构建调度见 [AGENTS.md](AGENTS.md)；根测试不覆盖独立桌面 workspace。构建与依赖缓存留在既有目录，临时日志和实验放仓库外。

## 架构与离线展示

| 入口 | 使用范围 |
| --- | --- |
| [系统架构](docs/ARCHITECTURE.md) | 当前实现分层、操作流程与未来设计，附图源和 SVG |
| [离线架构地图](docs/architecture.html) | 固定于 `b47ddfa` 的单文件说明快照；四图切换、节点说明、缩放和筛选，不自动联网 |
| [使用闭环与算法 HTML](docs/framework-design.html) | 各区按页内版本标记阅读；[研究依据](docs/design/2026-10-15-usage-framework.md)与[参考模型验证](docs/design/2026-10-15-framework-validation.md)不替代生产验证 |
| [历史总览](docs/project-overview.html) | 固定于 `6a8ad64` 的历史展示，不是 Soul 客户端，也不实时更新 |

[架构审查报告](docs/reviews/2026-10-15-architecture-review.md)说明设计取舍；[门禁说明](docs/gates/README.md)说明如何读取版本证据。历史 `ebff0c9` 的缺失外部证据继续标为 NOT VERIFIED，不作为当前安装或发布证明。
