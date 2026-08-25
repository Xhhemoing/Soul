# R3-SYNTHESIS — 架构确认 + BUILD 续跑探针

日期：2026-08-25。分支 `cursor/goal1-build-audit-c441`。父代理：cursor-grok-4.6-high。
探针：fable-a `claude-fable-5-thinking-xhigh`；fable-b 声明同系列 `claude-fable-5-thinking`（非静默）；gpt-sol-a/b `gpt-5.6-sol-xhigh-fast`。

## 当次 Goal 的结论

「确定架构和计划」：**已确定，且本轮核过无 DAG 漂移。** 权威仍是 `PRODUCT_LOCK.md` / `DECISIONS.md` / `GOAL1_PLAN.md` / FORMAL 矩阵；算法冻结在 `origin/main`。对照图：`.agent_workspace/orchestrator-c441/ARCHITECTURE_LOCK.md`（过程稿，非新立法）。不重开 D1–D60，不第二份 PRODUCT.md。

「深度优化」：R2 存量代码 P0/P1 已闭合。R3 探针带 repro 的**新**缺口如下，不与 fable-b「存量 DAG 零代码 P0」矛盾（fable-b 把新发现权交给 gpt-sol）。

LOOP20 仍 QUEUED。N4/N5/N6 仍 blocked-on-user。

## 共识

- WP01–WP13 不重派。T4D/A0 不在本主干重做（N6 / PR #7）。
- 向导占位句与 FakeStore 无名联系人遗忘视差已在 `882b814` / `99bdd30` 闭合。
- crate DAG 四钉（schema 无 IO、store 唯一业务落盘、egress 唯一 HTTP、policy 唯一签发）成立。

## 分歧与采纳

| 议题 | 观点 | 父代理 |
|---|---|---|
| 未导入姓名 + 原文豁免是否 P0 | gpt-sol-a：P0（界面承诺姓名恒占位；`session_e1` 现把泄漏当「不是 bug」）。fable-b：未预支 | **采纳 P0。** PRODUCT_LOCK 姓名/账号占位优先于测试注释。派 opus-a |
| 摘要「端点根据本机统计改写」 | gpt-sol-a P1 | **采纳 P1。** 派 opus-b |
| IPC `return;` 假绿 / 遗忘合同停在 Session / 研究预览只比路径 | gpt-sol-b P1-1/2/3 | **采纳。** 派 opus-c（独占 `ipc_roundtrip.rs` + Memory UI + session_screens 研究哈希） |
| IPv6 Display 丢括号 | gpt-sol-a P2；R2 已停车 fail-closed | **维持停车**（与 fable-b 停车群一致） |
| Draft/Research UI P2 测试钉 | gpt-sol-b P2 | **本批不派**（先 P0/P1） |

## 落笔分工（禁止同文件）

- **opus-a** P0：`soul-policy` redactor + tests；`soulcore/tests/session_e1.rs`；`soulcore/src/commands/draft.rs` 的 E1 确认通知；`Wizard.tsx` / `Wizard.test.tsx`。禁触 Graph、Memory、ipc_roundtrip、soul-draft analysis。
- **opus-b** P1 摘要：`soul-policy/src/e1.rs`；`soul-draft` analysis/reply/tests；`Graph.tsx` / `Graph.test.tsx`。禁触 redactor、session_e1、Wizard、ipc_roundtrip、Memory。
- **opus-c** P1 测试/遗忘 UI：`ipc_roundtrip.rs`；`Memory.tsx` / `Memory.test.tsx` / `fakeCore.ts` 遗忘状态；`session_screens.rs` 研究预览前后字节快照。禁触 redactor、e1.rs、Graph、Wizard。

## 驳回

- 重开计划扫描 / 重派 WP / 启动 LOOP20 / 合 PR #7 / AC-27 / empty-commit。
- 把 PRODUCT_LOCK 改成「未导入姓名可以原文出网」。

## 下一轮

opus 三路返回后：fable 只读复核；残留 P0 不得称收口。N4/N5/N6 仍要用户侧。

## 落笔结果（本机，核于 2026-08-25）

三路 opus 已提交在 `cursor/goal1-build-audit-c441` 尖端 `c0a6295`。`soul-policy` redactor_exemption 9 / redactor_leakage 9 本机绿。远端 PR #13 未更新：GitHub 令牌无效，用户跳过刷新，**不重要同一项凭证**。fable R3 复核进行中。
