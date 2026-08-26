# R1-SYNTHESIS — 收口审计一轮（orchestrator-c441）

日期：2026-08-25。分支 `cursor/goal1-closeout-c441-2d70`（起点主干 `650b0f2`，已 rebase 到 `a9a7490`）。

## 为什么不是 LOOP3 / 多代理并发

1. 本 VM 没有 Task 派生工具（工具面核实过：`cursor-cloud` 只有只读诊断），opus-fast / gpt-sol 派不出去。
2. 剩余可代码化的面太小：主干已经历 WP01–WP13 全量落地 + 概览修正 + 补证六轮 + 连续多轮文案对齐，公开的遗留全部是「账本 / 真机 / 归 PR #7 线」三类。按 PARENT_ORCHESTRATOR §5.2 的防伪代理条款，**不用假代理凑 6 个 scope**，改做单线高质量 AUDIT+fix，由父代理本人（`claude-fable-5-thinking-xhigh`，声明在案）顺序执行 fable-plan → 修复 → 复审。

## fable-plan：T4D/A0 吸收归属

- 本主干 `crates/soul-graph` 仍是遗留判档；`soul-algo-tie` / `soul-algo-trait` 不在本树；FORMAL 矩阵到 AC-27 为止。
- 吸收（BLOCKERS G1/G1+/G2/G3）与矩阵扩行（AC-28…AC-34）在 **PR #7 线** `cursor/goal1-unblock-a073` 上已完成，那棵树的 STATUS 自称「Goal 1 集成分支」。
- 本树 STATUS 明写「不要合 PR #7」。两线各自宣称主干（D49 指本线；a073 线自宣）。**结论：吸收归 #7 线所有，本会话不重做、不合并；主干取舍是用户级决定，上报。**

## AUDIT 结果（竞态 / 内存 / 边界 / 文案）

审过并判无缺陷（都有代码级理由，非「看起来没事」）：

| 面 | 结论 |
|---|---|
| `soul-win-dpapi/src/sys.rs`（全仓 unsafe 密钥路径） | 输入 blob 借用活过调用；输出 blob 置零起步；成功路径 拷贝→擦除→LocalFree 恰好一次；`unprotect` 结果包 `Zeroizing`；空输出报 `Empty` 不当空钥匙 |
| `soul-collect` 线程生命周期 | condvar 停车 ≤ `STOP_BUDGET`；同意双闸（poll 前 + 写库前一行）；撤销丢弃在飞 session；时钟倒走 `saturating_sub` 且有测试 |
| 壳单实例（`instance.rs`） | `SetLastError(0)` 先行（防陈旧 183 把第一份 Soul 骗退）；`ERROR_ALREADY_EXISTS` 分支句柄经 Drop 关闭；第二启动在开库之前退出 |
| `Origin::parse` / `NetGuard` | scheme+host+port 精确比对；带凭据拒绝；不带括号的 IPv6 解析成怪 host 但 fail-closed（连不上且比不中）；E0 无变体 |
| `soul-egress` | 请求 120s / 连接 10s 超时在 |
| 审计页词表 | `Audit.tsx` 的 `ACTION` 覆盖 `audit.schema.json` 全部 14 个动作 |
| `ci.yml` | 五门步骤自洽；desktop mock-runtime step 在 ubuntu job；作者手动基线（`478f19f` 或之后）与清单一致 |
| 概览 `Home.tsx` | 挂载重读快照与账本；空「还没有的东西」不画 |

**找到并修掉的缺陷（1 条，文案-行为不一致，红线「文案与实现一致」）：**

- `Files.tsx` 空态旧句「授权之前，Soul 读不到你机器上的任何文件。」为假：导入页亲手挑的文件不走目录授权就被读，Soul 也读自己的 config/库。与 `17b56e9` 修向导那句同类同法。改成只对目录扫描许诺、点名导入例外；`Files.test.tsx` 钉新句并断言旧句已不在。提交 `c4f8ce5`（rebase 后的修复提交；原 tip 为 `791f4d3`）。

## 判定为「不做」的（含理由）

- hosted CI 空 runner：账本（Billing & plans），非代码；**不 empty-commit**。
- 作者 Win11 手动清单：Linux 上做不了。
- T4D/A0 吸收：归 PR #7 线（见上）。
- `Authorization` 头 / 导入去重（D55）/ `collect_status` 计数查询 / 语气自由 JSON：文档在案的刻意取舍，非缺陷。
- AC-27、Goal 2、LOOP20：硬停。**LOOP20 保持排队，未启动。**

## 本轮验证（本机，修复提交 `c4f8ce5`）

- `cargo fmt --check` / `clippy -D warnings`：绿。
- `xtask schema-freeze --check` / `e0-audit` / `denylist-audit` / `sbom`：绿。
- `cargo test --workspace --all-targets`：92 个测试二进制全 ok（产品源码本轮未动 Rust 侧）。
- `cargo deny check`：advisories / bans / licenses / sources 四项 ok。
- `pnpm --filter @soul/desktop lint`（tsc+eslint）：绿。
- `pnpm --filter @soul/desktop test`：14 文件 161 项绿（含新钉）。
- 桌面壳（装齐 webkit2gtk 栈后）`cargo test --all-targets`：`command_surface` 6 / `ipc_roundtrip` 50 / `no_egress_path` 3 / `one_store` 3 / `shell_is_local_only` 21，全绿。

**本机绿不是 hosted 绿。** 本分支按 `ci.yml` 设计不自动触发（push 只开 `main` 与主干）。
