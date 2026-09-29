# STATUS

本文件是当前状态入口。此前记录完整保留在 [2026-09-28 状态快照](STATUS_HISTORY_20260928.md)；该快照继续链接 2026-09-27 的历史。历史测试和平台证据只覆盖其标注的版本。

## 当前状态（2026-09-29）

- **Goal 1、正式 Goal 2 与 Wave 1 均未关闭。** 实现、局部检查、完整门禁、人工观察和主干集成分别记账，不互相替代。
- 本次分支：`codex/soul-forget-outcomes-20260929`，从 `codex/soul-followup-20260928` @ `3cd0c73fe9d00f22f86a7b1bf54e2e0e2e0c7214` 延续。没有合并或修改 `main`；没有替代 PR #65 的验收。
- 范围继续按 [Wave 1 计划](SOUL_WAVE1_IMPLEMENTATION_PLAN_2026-09-27.md) 执行，优先处理 P0 遗忘结果语义。不改冻结算法、数据库 schema、权限或采集能力。

## 本轮阶段 1：结果类型与问题复现

**IMPLEMENTED，尚未进行 Rust 编译或平台验收。**

- 为存储接口增加 `WalCheckpoint`、`ForgetCleanup`、`ForgetOutcome` 与显式扩展接口 `ForgetOutcomeOps`。保留原 `ForgetReceipt` 和 `ForgetOps` 的形状，不为旧后端默认签发清理成功。
- 清理判断读取完整三元组；只有 `(0, 0, 0)` 表示 TRUNCATE 已确认完成。忙状态、未截断、非 WAL 和异常状态不能算通过。
- “销毁已提交”和“清理待完成”在类型上分开。清理重试接口不接受遗忘对象，不得重新销毁密钥或重复写审计。
- 新增一个 Rust 判定测试，覆盖完成状态及七种不能签发完成的状态；**已编写，未运行**。
- 新增 `scripts/probe-forget-wal.py`：只使用临时合成 SQLite 数据库，不读写用户 Soul 数据，不调用网络，不修改门禁。
- 本阶段仅提供类型和可复现探针；SQLCipher 后端、服务层和界面接入仍待后续提交，不把接口定义记为产品路径已修复。

## 本轮实际验证（非门禁）

环境：Linux，Node v22.16.0、Python 标准库 SQLite 3.46.1。没有 Rust、pnpm、PowerShell 或项目 React/Vitest 依赖；依赖源 DNS 解析失败。

- 原接口文件工作副本经 Git blob SHA 核对，与远端 `9836dbe3065c45833740be0268ecf5dc76400673` 完全一致后才修改。
- `python scripts/probe-forget-wal.py`：退出 0，**6/6**。实际观察到 `(1, 3, 3)`：所有帧已复制但有读者，WAL 未截断；以及 `(1, 4, 3)`：销毁已提交但读者仍持有旧快照。释放读者后的清理重试只执行 checkpoint，不增加 DML 次数。
- 该探针验证 SQLite 行为，不执行 Rust、rusqlite 或 SQLCipher，也不证明加密、物理擦除或 Windows 行为。
- 没有重跑前轮前端测试；没有签发 Rust、Vitest、lint/build、G-L/G-W/G-M、独立评审或发布 PASS。

## 仍待完成

P0 遗忘：SQLCipher 后端接入完整 checkpoint 状态与已提交回执；服务层保留审计未确认状态；界面区分逻辑提交、清理和审计；恢复入口只重试清理；补真实 Rust/SQLCipher 与组件测试。

P0 控制链路：Session 长任务准备／锁外执行／提交拆分，独立撤权路径，过期授权、取消及来源变化后的结果拒绝提交，仍未实现。

既有导入和图谱改动沿用前轮状态，不算本轮新增或通过。通用 `SqlCipherStore::checkpoint()` 忽略状态列的问题也仍待接入修复，不能用当前 `flush` 返回成功证明日志已清理。

G-L 继续按既有指示暂缓；NSIS 安装／卸载和 G-M 按既有指示跳过，不重试、不勾选。完整关闭仍按 [ACCEPTANCE](ACCEPTANCE.md)、[gates/README](gates/README.md) 与[作者清单](../scripts/author-manual-checklist.md)。没有对应证据就不关闭。
