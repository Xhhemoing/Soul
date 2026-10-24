# 仓库协作与记录

本页承接 [AGENTS](../../AGENTS.md) 的执行细则；当前状态只在 [STATUS](../STATUS.md) 维护，产品、算法和验收仍由各自权威定义。治理变更见 [ADR-001](../decisions/ADR-001-repository-governance.md)，远端检查见 [CI 操作说明](ci.md)。本页不自动授予提交、推送、发布、真实数据或研究权限。

## 分工与交接

一文件一名写者。最多三个活跃子 agent、两名实现者、三个活跃 worktree；worktree 仅隔离工作目录，不是凭据或数据安全沙箱。主线程持有唯一完整构建/门禁/打包通道，统一集成、维护当前状态并确认必需检查。实现者不得覆盖、还原、暂存或提交他人的修改；相关授权、锁顺序、事务协议、schema 或共享接口须串行。

以下是单写者热点：`crates/soulcore/src/commands/session.rs`、Tauri `commands.rs`、IPC 类型/注册表、两个 `Cargo.lock`、根 `Cargo.toml`、schema/lock、`docs/DECISIONS.md` 和 `docs/STATUS.md` 当前段。不同路径若共同改变授权、锁顺序或事务协议，同样串行。

| 工作面 | 相关模块 | 跨层验收要求 |
| --- | --- | --- |
| 数据与证据 | `soul-store`、`soul-store-api`、`soul-import`、`soul-memory` | 事务回滚、遗忘、重启、审计、evidence 可解引用 |
| 推断与展示 | `soul-graph`、`soul-profile`、`soul-algo-*`、`soul-draft` | 导入 → 加密库 → rebuild → 图/摘要；用户生效值与机器值分离 |
| 授权与出网 | `soul-policy`、`soul-egress`、`soulcore` | origin、许可消费、脱敏、拒绝时零出站，不仅核对返回值 |
| 桌面交互 | `apps/desktop`、Tauri IPC | 实际 command surface、异步旧结果、卸载/重试与 Windows 行为 |
| 构建与分发 | `xtask`、`scripts`、两个 Cargo workspace | 失败传播、实际产物路径与安装边界，局部检查不代替完整门禁 |

任务卡至少写：任务与 owner、起点 SHA/dirty、允许写路径、只读依据、依赖接口、目标/非目标、验收断言、实际检查命令和风险。只传必要上下文，相关后续问题复用原 agent；模型、思考强度及不可用时的处理遵循 [AGENTS 的协作方式](../../AGENTS.md#协作方式)。

简单、低影响改动可由主线程直接完成并验证；复杂、跨模块或涉及授权、安全、数据正确性的改动必须独立只读评审，其他任务按需评审，不固定配置实现者与评审者。评审者先读任务、候选和实际 diff，再看实现说明，只报告可定位的缺陷、反例、影响及最低修复建议；不暗改实现，不自签独立评审。敏感路径增加针对性反例，修复后验证；同一候选和覆盖面无新问题即停止重复扫描。子 agent 不自行启动重编译、安装依赖、更新锁文件或推送。操作员/重构建由主线程统一安排，不额外创建长期聊天角色。

阶段可标为 `READY → CLAIMED → IMPLEMENTED → REVIEWED → GATED → INTEGRATED`，以及 `BLOCKED/REJECTED`。局部检查只证明其覆盖范围；完整验收前不标 GATED，实际合入前不标 INTEGRATED。交接只写任务与候选版本、实际改动、命令/退出码/日志、未执行及原因、风险/依赖/回滚边界和下一动作；区分静态风险、已复现、已运行、正式验收与范围外改进。

提交和 PR 标题遵循 Conventional Commits，例如 `docs: clarify synthetic development startup`。不为文档改动自动提交；PR 使用[四栏模板](../../.github/pull_request_template.md)，填写实际证据及未知项。合并前检查目标 SHA 对应的远端结果，不能把本地通过、其他 SHA、已取消或跳过的 job 记为通过。CI 恢复依 D110，不恢复旧工作提示词、扫描轮次目录或第二套常驻流程系统。

## 检查与失败处理

所有终端使用 `pwsh`，脚本首行 `$ErrorActionPreference = 'Stop'`；文本读写显式 `-Encoding UTF8`。原生命令后立即检查 `$LASTEXITCODE`，它不由 `$ErrorActionPreference` 代管。文本和源码改动用 `apply_patch`，不全仓重写格式。根据改动风险选择现有检查，不为每次改动新增测试或门禁；`--locked` 失败时停止定位，不静默改锁。

普通失败遵循：保存失败及环境 → 找原因 → 最小修复 → 重跑原命令并追加结果。首次失败、慢样本、重试和范围限制保留；不得用缩小分母或额外预检覆盖原命令失败。实际执行/模拟/测量优先，前置检查不挤占它们。

按 [D112 / 战略指引](../plans/2026-10-23-long-term-strategy.md#51-控制验证与单点投入)控制投入：冒烟用于必要启动/接线确认，不在候选、环境和问题均未变化时反复运行；原命令重跑应有修复、条件变化或新假设。任务写明时间盒与停止条件，问题已回答即交付；无增量证据或到期时记录未决及恢复条件，转向无依赖的已授权工作。默认不新增哈希台账、冻结基线、空间阈值 gate，也不把全量构建或远端恢复设为所有开发的统一前置；既有安全检查和正式交付条件继续遵守。

| 检查面 | 现有入口；每条命令后立即检查退出码 |
| --- | --- |
| 授权与出网 | `cargo test -p soul-policy -p soul-egress --locked` |
| Rust 格式 | `cargo fmt --all -- --check`；桌面另跑 `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check` |
| schema / E0 / 禁词 | `cargo run -q -p xtask --locked -- schema-freeze --check`；`cargo run -q -p xtask --locked -- e0-audit`；`cargo run -q -p xtask --locked -- denylist-audit` |
| 前端 | `pnpm --filter @soul/desktop lint`；`pnpm --filter @soul/desktop test` |
| 生产包 | `pnpm --filter @soul/desktop build`；不得携带 `NODE_ENV=test`，需覆盖时临时设为 `production` 并在 `finally` 恢复 |
| 完整 G-L | Linux 按现有 recipe 运行 `just ci-full`；`just ci` 不完整；用户暂缓未解除前不启动 |
| 完整 G-W | `pwsh -NoProfile -File scripts/gate-win.ps1`；不传 `-SkipWorkspaceTests`；需对应平台和干净候选，限定合成的过滤检查不冒充完整 G-W |
| G-M | [作者手动清单](../../scripts/author-manual-checklist.md)；真实观察，不用源码、脚本语法或 mock 代替，不自动勾选 |

根 Cargo workspace 测试不覆盖独立桌面 workspace。限定合成的桌面测试可按本次范围运行以下入口；这是过滤测试，不能宣称完整全量通过：

```pwsh
$ErrorActionPreference = 'Stop'
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- --skip windows::runs_on_windows --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again
if ($LASTEXITCODE -ne 0) { throw 'Synthetic desktop tests failed' }
```

根 workspace 若运行采集测试，同样排除 `windows::runs_on_windows`。桌面另两项来自 `ipc_roundtrip`，在 Windows 会调用真实采集源；不得为了消除 filtered 而触碰真实数据。不要添加真实外部请求来冒充 mock 测试，也不自动读取真实聊天、日常数据库、凭据、密钥或终端历史。

同一时刻仅跑一份完整构建/门禁/打包。Linux 小规格门禁机从 `CARGO_BUILD_JOBS=1` 开始；前端沿用现有 2 workers。不覆盖全局 `CARGO_TARGET_DIR` 提速，G-W 依赖固定 root/desktop target 路径；缓存策略需先核对实际产物路径。不移动或清空依赖缓存制造目录整洁，保持两个 workspace 和现有构建路径。

正式回执沿用 [gates/README](../gates/README.md)，不把历史绿灯迁给新源码。发现检查误报应记录并按授权修复，不仅凭绿色文字签发 PASS。配置恢复和远端 CI 通过本身不代表 Goal 关闭、平台人工验收、已合入或发布。

## 记录与周期整理

每次调研、测试、实验、测量、验证及评审都保存结果。执行前确定所属记录及输出位置；尝试结束及时落盘，长任务在可恢复阶段保存，交付/交接前补齐摘要和证据链接。成功、失败、中断、取消和无结论都须记录，不只留在聊天或终端。

优先续写原任务设计、实施或验证记录。新设计、独立评审、正式平台证据分别归 `docs/design/`、`docs/reviews/`、`docs/gates/`；ADR、实施计划与操作说明按 [PLAN_INDEX](../PLAN_INDEX.md#目录与材料归属)归属。常规短检查可在原任务文档用一段文字或表格保存，不为每条命令创建新文件、新 gate 或另一份状态账本。

| 记录项 | 最少保留内容 |
| --- | --- |
| 目的与版本 | 日期/时区、执行者、问题/假设、范围、分支、HEAD、dirty 与候选路径；必要时保存相关 diff 或脚本副本，dirty 不能只标 HEAD 冒充提交版本 |
| 方法与输入 | 实际命令/步骤、参数、环境/工具版本、合成 fixture 来源；调研记来源链接/本地路径与访问日期，区分来源事实/推断；性能比较记配置、样本数、计时边界和干扰 |
| 结果与证据 | 各执行命令退出码、通过/失败/跳过/未运行及原因、指标与分母、安全原始输出或注明来源/省略范围的摘录、实际产物路径；无日志或不可保留时说明原因，非命令活动记完成/中断，不虚构退出码 |
| 结论与后续 | 支持的结论、反例/失败、限制、未验证项及必要下一步；分别描述静态判断、行为实测、独立评审和正式验收，不沿用历史绿灯 |

不适用须注明原因，未知和缺测不填零或通过。低影响文档核对用一段文字覆盖适用项即可，记录不代替执行，不强迫未授权提交、推送或完整门禁。

原始日志、测量数据、任务卡与必要复现输入保存到仓库外明确的持久目录，例如 `E:/SoulSyntheticLab/` 下的任务目录；所属文档写明实际路径、文件名和复现入口。临时目录只中转，交接前保存到持久位置；本机证据不暗示随仓库发布。仅保留获准合成数据和安全元数据，不保存凭据、密钥、真实正文/库内容，不扩大采集、出网或研究留存许可。

保留首次失败、重试、慢样本、负例、取消、缺测和排除项。修正须追加原因、版本与复验结果，不用最后一次成功覆盖过程或改分母隐藏失败。旧证据缺失时标记缺失及影响，不补造记录，不为补齐证据自动重跑外部请求或真实数据操作。

每次收尾或交接把结论放回所属文档，检查新增/修改的引用和证据路径，按需更新 PLAN_INDEX 与 STATUS；仅已定选择进入 DECISIONS/ADR，不复制结果全文到多个入口。STATUS 是不超过 40 行的当前摘要；新归档进入 `docs/archive/`，既有 `docs/history/` 保留，并保留日期、版本、失败、命令、限制及正确相对链接。纯文档整理不冒充功能或平台验证；新证据及时修正当前摘要，旧结论保留原日期/版本。`.learnings/` 只作本地草稿；已验证稳定经验归原设计或 AGENTS，不把推测、错误日志、旧授权升级为规则，不写入用户全局配置。

活跃开发至少每 7 天整理一次。启用日为 **2026-10-22（America/Los_Angeles）**，尚无完整整理时首次到期 **2026-10-29**。开工查看 STATUS 的最近完整整理日期与范围；已到期，或日期信息不足以判断到期时，本次主线程在收尾前核对自上次完整整理（首次从启用日）以来新增/变更材料的归属、结论冲突、证据可定位性和未完成项。

暂停期间不后台执行，恢复后的首个任务补做到期整理。只整理部分须明确剩余范围，不记整轮完成、不延后到期日；启用前未核对的存量材料另列，不冒充已整理。完整整理后，仅在 STATUS 维护最近日期、覆盖范围、剩余项与下次到期日。整理默认归类、去除重复摘要、修链接；删除或迁移原始证据必须另有明确授权和可核对清单，不以过期或通过为由清理证据。
