# STATUS

本文件是当前状态入口。历史见 [2026-09-28 快照](STATUS_HISTORY_20260928.md) 与[遗忘阶段 1–4 记录](SOUL_FORGET_PROGRESS_2026-09-29.md)。历史记录中的“尚未接入”描述其当时版本，不覆盖本页的新进展。

## 剩余框架续写（2026-10-16，D71–D75）

起点 `b66f093a1407887eb873fe33defe984906252d29`。作者要求完成现有计划的剩余本地用例，逐模块规划、复核、实现和验证。新增语义决定已记录，扫描切片已 REVIEWED，其余模块仍在实现，未将工作中源码或局部检查标为完整交付。

- 扫描采用 Session 内登记、锁外原只读扫描、Session 内归还唯一 file-plan ledger、审计及来源校验；与 E1 共用物理慢工作槽。来源修改不提前释放在途槽，生产 Tauri worker 在 IPC 等待方消失后仍完成接收处理。Files 已补初始读取/输入/请求/挂载归属及来源变化提示。
- 本切片实际验证：核心 lib **21/21**（含扫描 panic、外来 completion、来源变化后审计与拒收），前端定向 **4 文件 / 49 测试**、TypeScript、ESLint 通过。后续集成回归继续执行，不能沿用这里的结果为新源码背书。
- 记忆选择库的明确候选、排除/澄清、精确字节预算及 snapshot 复验已实现并独立复审，**10/10** 定向测试通过；Session/UI 和 E1 材料接线仍进行中。持久导入/去重/迁移/本地洞察尚未交付。
- 完整 G-L 继续按既有指示暂缓，原生桌面构建/IPC、G-W/G-M 未补齐；不标 GATED/INTEGRATED，不关闭 Goal 1、正式 Goal 2 或 Wave 1。

见 [剩余模块计划与实际结果](design/2026-10-16-remaining-implementation.md)。下列 D70 内容是此前切片的验证记录。

## 导入交付与人物摘要执行（2026-10-16，D70）

起点为 PR #66 的 `c1d4f8fc82f9535e12c9e9a07823df1599581432`。按作者最新框架要求，两模块均先具体规划、独立复核方案，再编码与只读 diff 复审；状态为 REVIEWED，未标 GATED/INTEGRATED。Goal 1、正式 Goal 2 与 Wave 1 未关闭。

- 导入页在同一 WebView 的路由挂载之间共享真实提交槽。切页不释放正在执行的提交，成功回执可在返回后查看；只保留计数和格式，不缓存原文/文件名。结构化核心拒绝可显式重试；非结构化失败或畸形回执进入 unknown，暂停继续导入，不伪称回滚或自动重发。重启不证明之前未提交。
- 回执提供图谱/证据与审计入口；关系数准确描述为本轮按有效证据重算写回量，包含已有关系的重算，不伪称新增或图中现存总量。D55 重复导入语义不变。
- 人物摘要沿原点击触发与统计回退路径，改为 Session 内准备、锁外 HTTP/解析、Session 内审计和来源校验。与起草共享一个真实 E1 worker 槽；失效不提前释放槽，counts 无端点时不占网络槽。资料/端点变化后拒收旧结果，WebView 补充交付微任务校验并清除已显示摘要。
- 完整前端 **30 文件 / 428 测试**、TypeScript、ESLint、production build 通过；Rust 1.83 / SQLCipher **15 套件 / 162 测试**全部通过；5 文件定向 Rust 格式及 6 项原壳命令静态断言通过；E0 审计扫描 16 个 shipped crate / 243 个源码文件，clean。16 类来源变化是一个矩阵测试的分支，不重复累计。
- 未修改持久化 schema、依赖锁、算法、39 个 IPC 命令的名称/参数/返回形状；person_summary 壳调度改为 async。原生桌面构建/IPC 未运行，缺 pkg-config 与 GTK/WebKit 开发依赖；静态断言不替代它们。完整 G-L 继续暂缓，G-W/G-M 未补跑。
- 仍未实现消息去重、持久回执、崩溃任务恢复、自动洞察与反馈学习、记忆用途/自动检索；扫描和导入长锁仍待迁移。生产 Tauri 闭包负责完整请求审计，不扩称任意底层调用方或 transport 内部 panic 均有相同保证。

详见 [模块计划、反例和实际验证](design/2026-10-16-import-summary-continuation.md)。同步沿用当前草稿分支，未合并 main。下方起草专用记录是上一切片范围，不覆盖本节新增摘要执行能力。

## 生产框架接入（2026-10-16，起草切片）

按使用框架完成前两步生产调整，基于 PR #66 工作分支的 `258c0162ca28c44c42dcb3f1c448e01a5bc5c7b6`。已独立逻辑复审；不是 GATED / INTEGRATED，Goal 1、正式 Goal 2 与 Wave 1 均未关闭。

- 起草页已接同步准入、输入/生命周期归属、预览和原文豁免绑定、跨挂载任务槽与 discard drain。资料变更期间拒绝新起草，成功 IPC 交付前校验代次，清除已显示旧结果；未知端点结果不自动重试。
- 生产起草采用 Session 内准备与发许可、锁外 HTTP/解析、Session 内审计与接收校验。Tauri worker 持 Arc，共享同一 Session/加密库，IPC 等待方消失仍完成审计。来源变化和撤回使旧结果失效，不提前释放在途槽。
- 撤回边界与两分钟单调期限见 [D69](DECISIONS.md)。已发许可可能继续出网；只迁移了起草 HTTP，摘要/扫描/导入仍可能占锁。记忆用途与自动检索、D55、迁移/遗忘重导未接入。
- 最终前端：28 文件 / 405 测试、TypeScript/ESLint、生产 build 均通过。含 6 项真实 core.ts + React 联合时序回归。HTML 13 项离线交互/下载/手机宽度检查通过。
- Rust 1.83.0：soulcore 定向 10 个 suite / 119 项全部通过（含新工作生命周期与期限的 14 项测试，两个来源矩阵各遍历 14 类修改）；7 个改动 Rust 文件格式检查通过；E0 审计扫描 16 个 shipped crate / 240 个源码文件，clean。
- schema、依赖锁、T4D/A0–A3 未改；原 39 个 IPC 命令名/参数/返回 schema 不变，起草的 Rust 壳函数改为 async。桌面原生构建与 IPC 未运行：当前 Linux 无 pkg-config / GTK / WebKit 开发依赖。完整 G-L 继续暂缓，NSIS / G-M 跳过不等于豁免；本环境 Bash 诊断不能替代仓库的 pwsh 平台门禁。

最新 [HTML 框架页](framework-design.html)、[生产起草流程图](design/draft-runtime-flow.svg)、[实现与验证](design/2026-10-16-draft-implementation.md)。代码/设计同步当前工作分支，PR 保持草稿，未合并 main。以下记录为历史，不能用其中“尚未接入起草”覆盖本节。

## 后续设计更新（2026-10-15，使用闭环与参考算法）

基于 PR #66 的 `6e8f3c06b7fd0cbc7920766337572301319b021a`，完成使用场景/源码差距、8 项一手研究依据、4 种路线比较、准备/执行/接收框架，以及页面归属、任务控制、证据选择、纠正优先与未知结果恢复设计。独立评审的槽位回收、字段类型、冲突范围及 origin 反例已修正。

新增纯 JS 非生产参考算法，37/37 合成测试（含一个枚举 120 种事件顺序的用例）、6/6 定向变异检出、13/13 离线 HTML 检查。没有改生产 Rust/React/IPC/schema/锁文件，未把模型结果视作原生/平台验证。研究、图示与实验见 [交互 HTML](framework-design.html)、[设计](design/2026-10-15-usage-framework.md)和[验证记录](design/2026-10-15-framework-validation.md)。用户既有 GitHub 同步授权继续用于当前草稿分支，未合并 main。

**下方生产修复与其测试仍是前一阶段证据。** 独立撤权与记忆参与起草尚未接入；取消、用途许可、迁移/遗忘重导仍需明确决定。Goal 1、正式 Goal 2 与 Wave 1 均未关闭。

## 当前状态（2026-10-15，架构改进与真实定向验证）

**Goal 1、正式 Goal 2 与 Wave 1 均未关闭。** 本轮状态为已实现并独立复审，附有局部测试证据；不标记 GATED 或 INTEGRATED。

源码基于 PR #66 / `codex/soul-forget-outcomes-20260929` 的 `9d3cd300c19c996f9edabf337c9d359fcb594bc3`。GitHub main 核对为 `8aae8f3877bc93529d0a0104c277a85ded9c43aa`。用户于 2026-10-15 明确要求将本轮代码及架构设计图同步 GitHub；交付落在该工作分支，PR 保持草稿，未合入 main。提交身份以包含本页的 Git 历史为准。

- 遗忘预览增加不可由 IPC 构造的精确范围记录，确认时在同一 store guard 下比较主键集合与影响统计，再执行原有销毁和 cleanup/audit 流程。影响扩大或等数量对象替换在销毁前拒绝；拒绝审计前释放 guard，防止递归锁死。此保证限定于现有单 writer 模型，不是跨连接授权协议。
- Memory 写入获得成功确认且编辑代次仍相同时才清空输入；拒绝、未知结果、新编辑和卸载不被旧回调清空。请求协调对象初始化通过真实 lint。
- Import 清空文件选择使旧 read/preview 结果失效，保留 commit 期间的保护；修复了完整前端测试中复现的 4 个既有失败。
- Linux 实际通过：前端 23 文件 / 338 测试、完整 TypeScript/ESLint、生产 build；Rust 1.83.0 存储 37 项 + soulcore 28 项，共 65 项；7 个改动 Rust 文件定向格式检查。独立只读复审未发现阻塞问题。环境使用 Bash，不能当作仓库要求的 PowerShell 平台门禁。
- schema、算法、依赖锁、IPC 命令签名与注册未改。Session 长任务占锁与独立撤权仍是 P0，已给出准备／锁外执行／结果校验的设计，尚未实施。D55 幂等与迁移/重导语义保持原决议。

图示入口见 [系统架构与设计图](ARCHITECTURE.md)，详细设计见 [架构报告](reviews/2026-10-15-architecture-review.md)，命令、退出码、失败修复与边界见 [验证记录](reviews/2026-10-15-validation.md)。后续按报告迁移一个起草用例并验证独立撤权，再扩展到其他长任务。

未运行整个 Rust workspace、独立桌面 workspace、原生 IPC、G-W 或 G-M；完整 G-L 仍按原指示暂缓，NSIS 安装/卸载与 G-M 跳过不等于豁免。以下 2026-09-29 记录保留为历史，不覆盖上述新证据。

## 历史状态（2026-09-29，阶段 5：桌面接线）

**Goal 1、正式 Goal 2 与 Wave 1 均未关闭。** 本轮把遗忘结果接到桌面源码并完成局部诊断，不是原生端到端通过、独立评审通过或发布声明。

工作分支：`codex/soul-forget-outcomes-20260929`，PR #66 保持草稿。直接父提交 `631e9ed16fcb7426cbd1f7e8df2132a6efefc211`。本轮核对 main 仍为 `8aae8f3877bc93529d0a0104c277a85ded9c43aa`，未修改或合并 main。前轮分支起点仍是 `codex/soul-followup-20260928` @ `3cd0c73`。

## 本轮完成的源码对齐

- 核心 `commands::memory::forget` 改用 `forget_with_outcome`，保留原 unit/impact 访问，增加清理与审计确认；已遗忘记忆不再次执行销毁。既有 Session 调用通过这个桥接进入新服务，没有修改 Session 的整体锁模型。
- `ForgetReceiptView` 增加 `logical_committed`、`cleanup`、`audit`。新 `retry_forget_cleanup` 命令复用 Session 的同一个加密库连接，不接收记忆 id 或销毁确认，不执行销毁或补写审计。
- TypeScript 包装、原生命令声明、原生命令名表、Tauri 注册表静态核对为同一组 **39 个命令**。原有命令未删除，新增一个清理命令。
- `Memory.tsx` 接入三种独立结果和“仅重试日志清理”按钮；重试保留原回执数字及审计不确定性。重启后已有墓碑也可发起清理，但不重建历史回执，不把墓碑当清理／审计确认。
- 页面请求在 IPC 前同步准入，卸载前后与旧预览回调不得重入；在实际启动 IPC 的微任务内再次检查归属。只有明确的预览编号不匹配保留确认面板；未知 IPC 结果不冒称回滚，不自动重发销毁。
- 旧版回执缺少新字段时不默认成功。既有 `fakeCore` 保留为旧形状测试材料；新增组件测试分别覆盖当前与旧形状。没有修改旧回归来掩盖不兼容。
- 原 `FORGET_NOTICE` 常量及其旧测试双胞胎未改；页面另行明确其永久不可读描述以日志清理确认为前提。旧底层遗忘 API 保留供旧调用者使用，本轮并非迁移所有旧 API 调用方。

## 本轮实际检查（不是完整门禁）

环境为 Linux 外部部分工作副本，Node v22.16.0、TypeScript 5.8.3。没有 cargo、rustc、pnpm、pwsh 或完整项目 React/Vitest 依赖；原始下载域名访问失败。诊断未使用仓库要求的 PowerShell，不能当作 G-L/G-W。

| 检查 | 实际结果 | 证据边界 |
| --- | --- | --- |
| `node /mnt/data/soul-checks/run-pure.cjs` | 退出 0，17/17 | 新 `memoryRequests.test.ts` 只替换 runner 导入为 node:test；断言正文不改；不是 Vitest |
| `node /mnt/data/soul-checks/probe-memory-ui.cjs` | 退出 0，12/12 | 实际 Memory 源码与 core.ts 包装，hook/JSX 和 invoke 替身；不是 React、DOM 或 Tauri 原生执行 |
| `node /mnt/data/soul-checks/check_sources.cjs` | 退出 0 | 7 个 TS/TSX 单文件转译无语法诊断；四处命令表一致、39 项；原生命令仍为单行转发 |
| `tsc --noEmit --strict --noUnusedLocals --noUnusedParameters --exactOptionalPropertyTypes --noUncheckedIndexedAccess --target ES2022` | 退出 0 | 仅 memoryRequests.ts 和 forgetOutcome.ts 两个独立模块 |
| Git blob 字节核对 | 通过 | 五个修改前完整文件匹配来源 blob；提交源码匹配工作副本；不是编译正确性证明 |

新增 **10 个 React/Vitest 用例、4 个 Rust 用例已编写但未运行**。前轮 14 个 Rust 用例也仍未运行。前轮 SQLite 6/6 和呈现断言 14/14 属于前轮局部证据，本轮没有重新计为 PASS。

没有运行完整应用类型检查、lint/build、旧回归、真实组件、Rust 格式／编译／测试、桌面 IPC、独立只读评审或平台门禁。受控地重复旧回调不等于真实浏览器正常双击复现；页面结果忽略不等于取消或回滚核心操作。

## 未闭环与后续顺序

1. **遗忘预览的影响面变化仍是 P0。** 当前 Session 校验预览编号／对象后执行，matched_preview 在返回回执时比较；本轮没有把影响面比较移动到销毁前，不能宣称阻止了预览后内容变化导致的超范围销毁。需在同一存储锁保护下重新比较再执行，并增加真实事务回归。
2. **独立撤权仍是 P0。** Session 长任务的准备／锁外执行／提交、撤权优先路径、取消与来源变化后的提交拒绝尚未实现。新清理命令仍受现有锁模型约束。
3. 补真实 Rust／SQLCipher、现有回归、React／Vitest、完整构建与原生 IPC 测试，再做独立评审。源码已接线不等于产品已验收。
4. 继续按原许可处理 G-L/G-W/G-M 与主干集成。G-L 暂缓，NSIS 安装／卸载和 G-M 跳过；没有据此勾选或免除任何关闭条件。

## 整体范围

已有 v0.1 路径包括问卷与档案纠正、导入、图谱、记忆 CRUD、Windows 前台时长采集、起草、授权目录只读计划、审计与内存研究预览；各自当前完整验证状态不得从旧版本绿迁移。

获准记忆自动检索参与起草、消息级幂等、行为预测、恢复包与受控文件执行、多设备／Android、可选云深度分析仍按 [ROADMAP](ROADMAP.md) 和 [Goal 2 计划](GOAL2_PLAN.md) 推进，不算本轮完成。

冻结 schema、算法、依赖锁文件、权限和采集能力没有扩展；未恢复托管 CI。正式关闭仍以 [Wave 1 计划](SOUL_WAVE1_IMPLEMENTATION_PLAN_2026-09-27.md)、[ACCEPTANCE](ACCEPTANCE.md)、[gates/README](gates/README.md) 与[作者清单](../scripts/author-manual-checklist.md) 为准。
