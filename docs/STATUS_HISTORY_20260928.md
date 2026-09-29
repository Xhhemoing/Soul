# STATUS

本文件是当前状态的唯一入口。历史记录见 [2026-09-27 状态快照](STATUS_HISTORY_20260927.md)；该文件完整保留原文，历史段落中的“本轮”“未提交”“未推送”只描述当时工作区，不是当前状态。

## 当前状态（2026-09-28）

- **Goal 1 与正式 Goal 2 均未关闭。** 代码实现、局部测试、完整平台门禁、人工观察和主干集成是不同状态；不互相替代。
- 续写分支：`codex/soul-followup-20260928`。该分支从 PR #65 的 `arena/01a0e375-soul` @ `efcaf8c77af0992a830ff5b9b8922c3a660ab063` 延续；本次修改的直接父提交为 `bc0d8d25f829003fabe2cb06b4ebb1124727c2fd`。独立分支同步不等于修改或合并 `main`，不代替 PR #65 的验收。
- **继承的实现，不计为本次新增：** 导入等待/格式切换/同文件重选保护、图谱摘要请求保护、导入输入预算及 IPv6 origin 规范化。Wave1 范围继续按 [现有实施计划](SOUL_WAVE1_IMPLEMENTATION_PLAN_2026-09-27.md) 执行。
- PR #65 的前端 222 项通过是该 PR 作者记录的历史局部结果；不是本分支本次 PASS。该 PR 明确记录 Rust 改动未运行，仍须补验证。

## 阶段记录

1. **文档对齐：IMPLEMENTED，已同步 `d32d1ea`。** 修正 SECURITY 的过期状态说明；原 STATUS 以原始 Git blob 保留在同目录，不改运行时代码。
2. **图谱交互补强：IMPLEMENTED，已同步 `bc0d8d2`。** 写入和摘要在 IPC 前同步准入；旧响应、错误对象与卸载回调不能更新页面。新增 16 个请求状态用例、11 个 React/IPC 用例。该阶段记录的离线 Node 16/16 与处理器探针 12/12 属于此前局部检查；本次未重跑图谱套件，未升级其验证状态。
3. **导入确认补强：IMPLEMENTED，随本提交同步。** 新增 `importRequests.ts`，只管理页面内读取、预览和确认的归属，不持有正文。`Import.tsx` 在启动文件读取或提交 IPC 之前同步准入；同一挂起确认不能重入，取消/替换/成功后的旧预览回调不能再次提交，卸载后的回调不能启动新读取或更新页面。提交期间的取消、换文件和格式变化不能把在途回执丢掉；未识别 owner 的预览在处理器处也不提交。成功后释放 staged 引用并清除字符计数和文件输入；失败保留既有明确重试路径，不自动重试。
4. **导入配套测试：已编写，真实组件执行待补。** `importRequests.test.ts` 有 24 个纯状态用例；`Import.lifecycle.test.tsx` 有 12 个展开后的 React/IPC 用例，全部使用合成输入。原有 `Import.test.tsx`、`Import.race.test.tsx` 和图谱测试没有修改。

## 本次实际检查（非门禁）

环境：Linux，Node v22.16.0、TypeScript 5.8.3；无项目锁定的 React/Vitest 依赖，无 Rust、pnpm、PowerShell。依赖源 DNS 解析失败。本次只在仓库外工作副本执行离线检查，不宣称执行了仓库 PowerShell 门禁。

- 修改前 `Import.tsx` 工作副本与远端 Git blob `b51d7575c70aa6f1f69cbf2c94ff42822a9886ac` 一致，包括结尾换行。
- `tsc --noEmit --strict --noUnusedLocals --noUnusedParameters --exactOptionalPropertyTypes --noUncheckedIndexedAccess --target ES2022 apps/desktop/src/routes/importRequests.ts`：退出 0，仅检查新状态模块。
- `node --test checks/run-import-pure.cjs`：退出 0，24/24。将已编写纯测试的 `vitest` runner 导入替换成 `node:test`，断言体不变；这不是 Vitest 执行结果。
- 修改前 `SOUL_PROBE_BEFORE=1 node --test checks/import-handler-probes.cjs`：退出 1，7 通过 / 11 失败；修改后同组 `node --test checks/import-handler-probes.cjs`：退出 0，18/18。探针转译实际组件源码，用轻量 hook/JSX 替身调用处理器，不加载 React/DOM/Tauri；覆盖重入确认、过期预览、卸载、失败重试与预算拒绝等边界。
- 4 个本次源码/测试文件的 TypeScript 单文件转译无语法诊断；UTF-8、LF、尾随空白检查通过。单文件转译不能代替项目类型检查、lint 或生产构建。
- **未运行：** 项目完整 Vitest（含新增 12 个 React/IPC 用例）、前端 lint/build、Rust/桌面测试、完整 G-L/G-W/G-M、独立只读评审。没有签发 REVIEWED、GATED、INTEGRATED 或发布 PASS。

探针的证据边界：受控地重入同一渲染回调或持有旧回调，不等于已经在真实浏览器复现普通双击。React 对独立点击分别处理状态更新，因此不能据此声称用户正常双击一定造成重复导入。本次是页面处理器加固；真实调度行为仍以待执行的组件与平台测试为准。

## 已有平台证据（按所标源码版本阅读）

- [20260925-ebff0c9-win](gates/20260925-ebff0c9-win.md)：历史四包集成的 Windows 记录，不自动覆盖当前分支或新增代码。
- [20260925-95ff7d6-win](gates/20260925-95ff7d6-win.md)：保留安装失败、恢复与未验收边界。
- **G-L 仍按既有用户指示暂缓。** 本次未启动完整 Linux 门禁；暂缓不是免除 Goal 1 关闭条件。
- **G-M 未完成。** 真实 NSIS 安装/卸载按既有指示跳过；不重试，不勾选人工清单。外部打包证据缺失与卸载残留的历史说明仍在状态快照中。

## 待补验证与下一步

具备项目依赖的环境应执行 `pnpm --filter @soul/desktop lint`、`pnpm --filter @soul/desktop test` 和生产 `build`，覆盖新增导入 36 个用例、既有图谱 27 个新增用例及全部旧回归；再进行独立评审和受影响平台验证。不将这些“已编写用例”的数量算作已通过数量。

本次不改冻结算法、权限、Rust 核心、schema、IPC 命令、存储事务或取消语义。忽略已卸载页面的结果不等于取消核心请求或回滚导入；释放 JavaScript 引用不承诺内存物理擦除。手动再次选择同一文件仍可重新预览并确认，不声称实现消息级幂等。

Wave1 P0 撤权执行模型与遗忘结果语义仍待实施及平台验证。消息幂等、持久化检索、行为预测及新采集能力不在本次修改范围；不新增冻结 contract、hash 或门禁体系。完整关闭继续按 [ACCEPTANCE](ACCEPTANCE.md)、[gates/README](gates/README.md) 和 [作者手动清单](../scripts/author-manual-checklist.md)，没有对应证据就不关闭。
