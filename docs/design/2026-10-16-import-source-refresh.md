# 导入提交后的来源选项刷新

基线 `760c269f4e0adcab2886bf4ffe30b5f4564166e4`，PR #66。属于 D68 现有导入使用链路的最小缺陷修复；不启动 VAL-01 或改变平台验收前置。

## 问题与源码依据

`Import.tsx` 只在挂载及显式回执查询末尾读取来源列表。新来源在 `soul-import::indexed::commit_indexed` 的事务中才持久化，准备时不存在。首次 JSONL 提交成功后，当前页仍显示提交前的空列表，用户无法直接明确选回同一来源。切页后、提交前完成初读的新页面也会保持旧列表。核心已有去重能力，但界面没有及时提供它需要的来源选择。

## 具体方案（PLAN REVIEWED）

- 输入仍为既有 `listImportNamespaces()` 与共享 `DurableImport` 回执；不增加 IPC、核心用途、迁移、算法或依赖。
- 将来源列表读取移入独立 effect：初次挂载、共享回执变化、完成显式回执核对后重读。挂载身份由原生命周期 effect 管理，不因回执刷新而重置。
- effect 的本次 active 标记、挂载身份及共享回执身份共同约束交付；新读取或卸载使旧成功/失败失效，避免提交前读取覆盖提交后的选项。
- 显式核对统一触发同一读取路径，不保留可绕开交付约束的第二处 `setNamespaces`。刷新失败只提示重读来源列表，不修改确认结果、操作编号、unknown 或原提交回执。
- 只刷新选项，不自动选择来源、不重读文件、不调用 prepare/confirm。用户选择已创建来源后，下一次 prepare 才携带该 UUID。

## 验收与实际检查

先新增真实 `core.ts` 包装器与 React 联合回归，native 远端使用合成 fake：首导完成出现来源选项、默认仍为空、显式选择后下一次 prepare 精确携带来源；切页在途提交完成也刷新；提交前列表晚到不覆盖；卸载后的成功/错误不更新页面；刷新拒绝保留成功回执且不锁新意图，显式核对后可恢复。不得将该范围称为原生 IPC/Windows 验收。

主流程先运行新增反例确认 RED，再最小修复、运行导入关联测试、全前端类型/lint/测试及生产 build、`git diff --check`。无 Rust 产品变更，不重复编译核心，也不沿用旧 Rust 数量为本轮结果。独立只读 reviewer 在编码前核方案、完成后核实际 dirty diff 与结果。具体命令、退出码及未运行项在本节后续更新。

## 实际结果

问题与具体方案先通过独立只读规划复核（PLAN REVIEWED），再新增反例、最小实现和实际 dirty diff 复审。实现与新增卸载迟到失败断言均无剩余源码阻塞（REVIEWED）；包含本文的提交标识本轮版本。未新增决策、schema 或核心命令。

环境：Linux，pwsh 7.4.6，Node 24.19.0；工作区维护后按既有冻结锁恢复前端依赖，实际恢复器为 pnpm 11.25.0，使用 `--frozen-lockfile --ignore-scripts`，未改 package/依赖锁。测试直接运行锁定依赖的 CLI，前端 2 workers，未运行 Rust 构建。

| 实际命令（desktop 目录） | 退出与结果 |
| --- | --- |
| `node node_modules/vitest/vitest.mjs run src/routes/Import.sources.test.tsx --maxWorkers=2 -t "提交后立即"`，修复前 | 1；1 项失败、5 项未选；失败为已提交后找不到新来源 option，不是编译错误 |
| 同文件不带 `-t`，首轮修复后 | 0；当时 6/6，通过后按 reviewer 建议补第 7 项卸载迟到拒绝 |
| `node node_modules/typescript/bin/tsc --noEmit` | 0 |
| `node node_modules/eslint/bin/eslint.js .` | 0 |
| `node node_modules/vitest/vitest.mjs run --maxWorkers=2`，最终重跑 | 0；44 文件 / 524 测试，其中新 7 项包含在内，不与定向相加 |
| 临时 `NODE_ENV=production` 后 `node node_modules/vite/bin/vite.js build`，finally 恢复 | 0；51 modules，生成 WebView 静态包，不是原生安装包 |
| 根目录 `git diff --check` | 0 |

新增合成回归通过真实 `core.ts` 包装器验证：提交后出现新来源、默认仍为空、明确选择后下一次 prepare 精确携带 UUID；切页在途提交完成后的刷新；提交前列表迟到成功/失败不覆盖新列表；列表读取失败保留 committed 回执及可用状态，显式核对后恢复；卸载后的成功/失败均不影响新页面。不读取真实文件或数据库，不发外部服务请求，native 端是 fake，不称原生 IPC 已运行。

首次全量命令进入 Vitest 后被自动审批因 PowerShell 遥测外联中断，没有完整汇总或成功退出，不计通过。依微软 about_Telemetry 的要求，改在启动进程前设置 `POWERSHELL_TELEMETRY_OPTOUT=1`，并关闭启动更新检查，再以同一前端命令序列重跑退出 0。参考：[官方遥测说明](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_telemetry)。先前只在脚本内设置变量不足以关闭启动遥测；未申请放行该地址。

核心/原生 desktop/IPC、完整 G-L、G-W/G-M 未在本轮执行；沿用已有平台限制和暂缓决定，不将 524 项前端通过扩成平台验收，不关闭 Goal 1、正式 Goal 2 或 Wave 1。57 个 IPC、持久 schema、算法、依赖锁保持原样，PR 不合并 main。
