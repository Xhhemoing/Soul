# STATUS

本文件是当前状态入口。此前记录保留在 [2026-09-28 状态快照](STATUS_HISTORY_20260928.md)；该快照继续链接 2026-09-27 的历史。历史证据只覆盖其标注的版本。

## 当前状态（2026-09-29）

- **Goal 1、正式 Goal 2 与 Wave 1 均未关闭。** 实现、检查、独立评审、完整门禁与主干集成分别记账。
- 工作分支：`codex/soul-forget-outcomes-20260929`；起点是 `codex/soul-followup-20260928` @ `3cd0c73fe9d00f22f86a7b1bf54e2e0e2e0c7214`。没有修改或合并 `main`，没有替代 PR #65 的验收。
- 继续按 [Wave 1 计划](SOUL_WAVE1_IMPLEMENTATION_PLAN_2026-09-27.md) 处理 P0 遗忘结果语义。不改冻结算法、数据库 schema、权限或采集能力。

## 本轮阶段成果

**IMPLEMENTED；未 REVIEWED / GATED / INTEGRATED。**

阶段 1（`fcbb501`）：增加 `WalCheckpoint`、`ForgetCleanup`、`ForgetOutcome` 和显式扩展接口 `ForgetOutcomeOps`，保留原 `ForgetReceipt` 与 `ForgetOps` 的形状。旧后端不自动取得清理成功的默认实现。新增完整 checkpoint 三元组判定测试，以及临时合成 SQLite 探针。

阶段 2：SQLCipher 后端接入结构化结果。事务提交后，即使 WAL 清理忙或查询出错，也保留原始回执，返回清理待完成；不能把它当成未发生的遗忘重新执行。新增的 `retry_forget_cleanup` 只运行 checkpoint，不重新解析遗忘对象、不销毁密钥、不写审计。通用 `checkpoint()` / `flush()` 也不再忽略 busy 和帧计数。旧接口不能返回待清理状态，因此仍返回明确说明已提交、只能重试清理的错误；桌面路径尚待迁移。

新增 7 个 SQLCipher 单元测试：忙状态保留回执、已复制帧但未截断、清理重试与重启不重复 DML、提交前失败回滚、checkpoint 查询失败、旧接口不能误报成功，以及 checkpoint/flush 的忙状态。连同阶段 1 的 1 个判定测试，**8 个 Rust 测试已编写，未运行**。

## 本轮实际检查与边界

- 环境检查：Linux；Node v22.16.0；没有 `cargo`、`rustc`、`pnpm` 或 `pwsh`，标准工具链路径也未找到可执行文件。连接 `raw.githubusercontent.com` 的实际检查报 DNS 解析失败。没有安装依赖或修改锁文件。
- 将远端探针逐字复制到仓库外工作目录后，核对其 Git blob SHA 为 `a0577ce1098b54005f512d70da17526cd0bdb9d1`，与已提交脚本一致。
- 实际命令：`python /mnt/data/soul-work/scripts/probe-forget-wal.py`；退出 **0**，**6/6**；Python SQLite **3.46.1**。实际日志保存在工作目录 `logs/probe-forget-wal.log`。
- 观察到 `(1, 3, 3)`：帧已复制但有读者，WAL 未截断；以及 `(1, 4, 3)`：删除已提交但读者仍持有旧快照。释放读者后的清理重试只执行 checkpoint，没有增加 DML 次数。
- **记录校正**：阶段 1 的提交说明和状态文件提前将探针写成已执行；可核验的实际执行发生在该提交之后。这里以实际终端输出为准，并删除了此前没有执行记录支持的原接口工作副本 SHA 核对断言。
- 通过 GitHub 生成的父提交差异核对存储层候选改动，确认 `store.rs` 只改变 checkpoint 实现及相关导入；没有改变加密、密钥封装或其他 CRUD 代码。这是实现者自查，不是独立评审。
- SQLite 探针不是 Rust、rusqlite、SQLCipher、Windows 或加密验证。Rust 格式、编译、原生测试、Vitest、lint/build、完整门禁、独立评审均未运行或未完成，不签发 PASS。没有执行仓库要求的 PowerShell 门禁；仓库外 Python 探针不替代它。

## 仍待完成

P0 遗忘：服务层保留审计未确认状态，核心／IPC／界面接入新结果，提供只重试清理的恢复入口，补跨层和组件回归测试，并实际执行 Rust/SQLCipher 验证。

P0 控制链路：Session 长任务准备／锁外执行／提交拆分、独立撤权路径，以及过期授权、取消、来源变化后拒绝提交，仍未实现。

既有导入和图谱改动沿用前轮状态，不计为本轮新增或通过。G-L 继续暂缓；NSIS 安装／卸载和 G-M 继续跳过，不重试、不勾选。正式关闭仍按 [ACCEPTANCE](ACCEPTANCE.md)、[gates/README](gates/README.md) 与[作者清单](../scripts/author-manual-checklist.md)。
