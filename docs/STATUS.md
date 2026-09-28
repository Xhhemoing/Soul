# STATUS

本文件是当前状态的唯一入口。历史记录见 [2026-09-27 状态快照](STATUS_HISTORY_20260927.md)；该文件完整保留原文，历史段落中的“本轮”“未提交”“未推送”只描述当时工作区，不是当前状态。

## 当前状态（2026-09-28）

- **Goal 1 与正式 Goal 2 均未关闭。** 代码实现、局部测试、完整平台门禁、人工观察和主干集成是不同状态；不互相替代。
- 本轮续写分支：`codex/soul-followup-20260928`，起点为 PR #65 的 `arena/01a0e375-soul` @ `efcaf8c77af0992a830ff5b9b8922c3a660ab063`。远端 `main` 核于本日仍为 `8aae8f3877bc93529d0a0104c277a85ded9c43aa`。本轮在独立分支同步，不修改或合并 `main`，不代替 PR #65 的验收。
- **继承的实现，不计为本轮新增：** 导入等待/格式切换/同文件重选保护、摘要返回的基础代次保护、导入输入预算及 IPv6 origin 规范化，均已在起点分支中。Wave1 范围继续按 [现有实施计划](SOUL_WAVE1_IMPLEMENTATION_PLAN_2026-09-27.md) 执行。
- PR #65 的改动已在 GitHub 提交，旧状态顶部“工作区未提交/未推送”不再适用于该提交。其前端 222 项通过是该 PR 作者记录的历史局部结果；本轮没有重跑，不转写为本轮 PASS。该 PR 明确记录 Rust 改动未运行，仍须补验证。

## 本轮阶段

1. **文档对齐：IMPLEMENTED，已同步 `d32d1ea`。** 修正 SECURITY 中“main 没有应用代码”的过期表述，以源码指针和带版本的平台记录分别表达实现与验证；原 STATUS 以原始 Git blob 完整保留在同目录。本阶段不改运行时代码。
2. **图谱交互补强：IMPLEMENTED。** `Graph.tsx` 与 `graphRequests.ts` 在调用 IPC 之前同步准入；同一挂起关系写入不会被连点重复启动，写入期间拒绝新摘要；同一对象的挂起摘要不重复发送，仍可选择其他对象。结果按请求代次和对象校验，关系纠正会使旧摘要失效；卸载后的写入和摘要回调不更新页面，同步抛错也进入可重试拒绝路径。只改变 UI 生命周期，不修改冻结算法、权限、schema、IPC 命令或核心取消语义。丢弃页面结果不等于取消远端请求或回滚已执行操作。
3. **回归用例已编写，平台验证待补。** 新增 `graphRequests.test.ts` 的 16 个纯逻辑场景与 `Graph.lifecycle.test.tsx` 的 11 个 React/IPC 场景；原有测试保留。纯逻辑同一组断言已通过 Node 原生测试运行器执行，处理器探针也已执行；这不是仓库 Vitest 或真实 React/WebView 测试通过。

## 本轮实际检查（非门禁）

环境：Linux；Node v22.16.0、TypeScript 5.8.3；没有项目锁定的 React/Vitest 依赖，也没有 Rust、pnpm、PowerShell。以下仅为仓库外工作副本的离线检查，不是 PowerShell 平台门禁。

- `tsc --noEmit --strict --target ES2022 apps/desktop/src/routes/graphRequests.ts`：退出 0，仅检查新增请求状态模块。
- `node --test checks/run-pure-tests.cjs`：退出 0，16/16；将纯测试的 `vitest` runner 导入替换为 `node:test`，执行同一份测试断言，不加载 Vitest。
- `SOUL_PROBE_BEFORE=1 node --test checks/graph-handler-probes.cjs`：退出 1，4 通过 / 8 失败（预期的修改前结果）；修改后同组 `node --test checks/graph-handler-probes.cjs`：退出 0，12/12。探针转译实际 Graph.tsx 并用轻量 hook/JSX 替身调用同一渲染代的处理器；它能检查准入和回调，但不验证 React 调度、DOM、Tauri 或真实出网。
- 修改前 Graph.tsx 工作副本的 Git blob 与远端 `156f7c47fb3d6f74ca776d1c938df2a5939d7b39` 一致；未用旧报告代码代替续写分支代码。
- **未运行：** 仓库前端 lint、Vitest（含新增 React/IPC 用例）、生产构建、Rust/桌面测试、完整 G-L/G-W/G-M、独立只读评审。源码转译和纯逻辑检查不能替代它们。

## 已有平台证据（按所标源码版本阅读）

- [20260925-ebff0c9-win](gates/20260925-ebff0c9-win.md)：历史四包集成的 Windows 记录，不自动覆盖当前分支或本轮新增代码。
- [20260925-95ff7d6-win](gates/20260925-95ff7d6-win.md)：保留安装失败、恢复与未验收边界。
- **G-L 仍按既有用户指示暂缓。** 本轮未启动完整 Linux 门禁；暂缓不是免除 Goal 1 关闭条件。
- **G-M 未完成。** 真实 NSIS 安装/卸载按既有指示跳过；不重试，不勾选人工清单。外部打包证据缺失及卸载残留的历史说明保留在状态快照，不把合成测试或源码检查等同于真实观察。

## 下一动作与边界

具备项目依赖的环境先执行 `pnpm --filter @soul/desktop lint`、`pnpm --filter @soul/desktop test` 和生产 `build`；检查新增 27 个 Vitest 用例及现有回归，再进行独立评审和受影响平台验证。完整关闭仍按 [ACCEPTANCE](ACCEPTANCE.md)、[gates/README](gates/README.md) 和 [作者手动清单](../scripts/author-manual-checklist.md)，没有对应证据就不关闭。

Wave1 P0 撤权执行模型与遗忘结果语义尚待实施和平台验证。消息幂等、持久化检索、行为预测以及新采集能力不在本轮修改范围；不新增冻结 contract、hash 或门禁体系。
