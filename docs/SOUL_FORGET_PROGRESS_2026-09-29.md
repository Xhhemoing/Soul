# Soul 遗忘语义阶段记录 — 2026-09-29

## 结论与范围

这是 Wave 1 P0 遗忘结果语义的阶段实现，不是项目完成证明，也不是可发布版本。工作分支为 `codex/soul-forget-outcomes-20260929`，基于 `codex/soul-followup-20260928` 的 `3cd0c73fe9d00f22f86a7b1bf54e2e0e2e0c7214`。未改动或合并 main，未关闭 Goal 1、正式 Goal 2、Wave 1 或 PR #65 的验收。

冻结 schema、加密算法、权限、网络出口、采集能力和锁文件不在本轮修改范围。现有暂缓／跳过项保持原状：G-L 暂缓；NSIS 安装／卸载、G-M 跳过。没有恢复托管 CI，没有把实现者自查记作独立评审。

## 已保存的实现

### 阶段 1 — `fcbb501`

`crates/soul-store-api/src/forget.rs` 新增显式 `ForgetOutcomeOps` 扩展、`ForgetOutcome`、`ForgetCleanup`、`WalCheckpoint`。保留原始回执及旧接口形状，不为未实现扩展的后端默认签发清理成功。只有 checkpoint 三列均为零，才确认 TRUNCATE 完成。

### 阶段 2 — `0e6e852`

`crates/soul-store/src/store.rs` 读取全部三列，不把 SQL 查询成功等同于清理成功；通用 checkpoint / flush 同样拒绝未确认的截断。

`crates/soul-store/src/forget.rs` 的新接口在事务提交之后保留原始回执，把清理忙、查询失败分成独立待完成状态。只清理重试不接受遗忘对象、不重新计算影响面、不删除密钥、不写审计。旧接口仍返回明确说明“已经提交、应只重试清理”的错误，而不是谎报全部成功。

### 阶段 3 — `40ad88d`

`crates/soul-memory/src/outcome.rs` 新增 `forget_with_outcome`。销毁提交后只尝试一次审计；审计写入错误可能发生在实际落盘之前或之后，所以报告 `unconfirmed`，不自动补写，不吞掉已经提交的回执。清理成功不会把未确认的审计变成已确认。

旧 `soul_memory::forget` 仍保留。新增服务不是旧桌面调用路径的自动替换。

### 阶段 4 — 呈现逻辑与可复现探针

新增 `apps/desktop/src/forgetOutcome.ts`，分别识别提交、清理、审计及预览匹配状态。缺失字段、旧回执、未知状态不能默认获得成功标签；前端不重新解释 checkpoint 帧数；单独的清理结果不能证明销毁或审计发生。

新增 14 个纯状态判定测试，以及 `scripts/probe-forget-presentation.cjs`。探针只在临时目录转译测试副本，将唯一的测试器导入从 Vitest 换成 Node 自带测试器；断言和测试正文不改动。它不安装依赖、不联网、不读写 Soul 用户数据库。

**此模块尚未被 Memory 路由或 core.ts 导入。阶段 4 是经过局部验证的呈现逻辑，不是界面接线完成。**

## 实际执行的检查

所有诊断均在仓库外的部分工作副本 `/mnt/data/soul-work` 运行，不是完整 checkout，不是仓库 PowerShell 门禁。当前环境为 Linux，Node v22.16.0、TypeScript 5.8.3、Python SQLite 3.46.1。未找到 cargo、rustc、pnpm、pwsh；访问原始源码下载域名的实际检查遇到 DNS 解析失败。GitHub 连接器可读写仓库，因此阶段提交已通过连接器保存。

| 检查 | 实际结果 | 不代表什么 |
| --- | --- | --- |
| `python /mnt/data/soul-work/scripts/probe-forget-wal.py` | 退出 0，6/6 | 不是 Rust / rusqlite / SQLCipher / Windows / 加密门禁 |
| 单文件 `tsc --strict --noEmit --target ES2022 --module commonjs .../forgetOutcome.ts` | 退出 0 | 不是整个应用类型检查或构建 |
| `NODE_PATH=$(npm root -g) node /mnt/data/soul-work/scripts/probe-forget-presentation.cjs` | 退出 0；单文件严格检查通过；Node 测试器 14/14 | 不是 Vitest、React 组件测试、IPC 联调或原生测试 |

SQLite 探针实际观察到 `(1, 3, 3)`：所有帧已复制，但读者阻止截断；以及 `(1, 4, 3)`：删除已提交，旧读者仍持有之前的快照。释放读者后，重复清理只执行 checkpoint，不增加 DML 次数。这个探针只使用临时合成数据。

复制和执行的新文件已与 GitHub blob SHA 核对：

- SQLite 探针：`a0577ce1098b54005f512d70da17526cd0bdb9d1`
- 状态判定模块：`d4c011e176f44a679f64997726e214537fb61241`
- 状态判定测试：`8ba4132f5c4776ca9bf744da14b8a8b081f79c79`
- Node 复现探针：`6252366a0ab448d7b967f24d1ba4d72fa9a8d523`
- 新服务模块：`ec8e45c3a8c86920e7921790512939a39c62f4d7`
- 新服务测试：`ad0c9775a88b5a0706a268c9dc8e1f7efd249a84`

其中服务模块／测试的 SHA 核对只证明提交内容一致，不证明 Rust 代码能编译或测试通过。

**记录校正：**阶段 1 的提交说明提前把 SQLite 探针记成已执行。可核验的实际执行发生在该提交之后；阶段 2 已明确纠正。此前没有执行记录支持的原接口工作副本 SHA 核对断言也已移除。这里以真实终端输出及上述局部检查范围为准。

## 编写但没有运行的 Rust 测试

共 **14 个测试函数**：存储接口判定 1 个、SQLCipher 存储回归 7 个、服务边界回归 6 个。覆盖忙状态回执、已复制但未截断、重启和重复清理无 DML、提交前回滚、checkpoint 查询错误、旧接口保守失败、flush 忙状态，以及审计落盘前／后错误、待清理时审计、缺失记忆和提交前拒绝。

服务测试中强制的 pending 值只验证状态传递，不冒充实际读者竞争；真正的读者竞争案例写在 SQLCipher 测试中，尚未运行。Rust 格式、编译和原生测试均为 **NOT RUN**。没有签发 REVIEWED、GATED、INTEGRATED 或发布 PASS。

## 当前产品路径仍有的缺口

1. **桌面仍调用旧服务。** `crates/soulcore/src/commands/memory.rs`、Session 回执 DTO、`apps/desktop/src/core.ts`、原生 IPC 注册和 `Memory.tsx` 的迁移尚未提交。当前分支不是端到端 P0 修复完成版，不能直接当成发布候选。
2. **恢复按钮未实现。** 只清理接口在存储／服务层存在，但没有桌面入口。需要同一提交接齐核心方法、原生命令、注册清单、TS 包装和路由，保留原回执且不把清理结果当作审计确认。
3. **真实链路验证缺失。** 需要补／更新回执与文案契约、fakeCore、组件和 IPC 用例，运行所有新增 Rust 测试并重跑既有遗忘、重启、审计和命令表测试。未知 IPC 错误不得被界面解释为已回滚；明确的预览拒绝与提交后待处理状态须分开。
4. **预览后的数据变化仍需检查。** 现有 Session 代码匹配预览标识，回执再比较影响面。本轮没有证明所有并发路径都能在销毁前拒绝已变化的影响面，不签发这一项通过。
5. **另一条 P0 控制链路尚未改造。** Session 长任务准备／锁外执行／提交、独立撤权、过期授权、取消及来源变化后的结果拒绝提交，仍按 Wave 1 计划待实现。

本地尝试的核心／前端接线草稿没有作为已验证成果提交；不使用它们补足上述清单。未完成项继续以 [Wave 1 计划](SOUL_WAVE1_IMPLEMENTATION_PLAN_2026-09-27.md)、[ACCEPTANCE](ACCEPTANCE.md)、[门禁说明](gates/README.md) 和作者人工清单为准。
