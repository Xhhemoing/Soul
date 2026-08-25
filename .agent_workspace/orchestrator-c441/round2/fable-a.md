# fable-a — BUILD/AUDIT R2 未关闭 DAG（只读规划）

- model: `claude-fable-5-thinking-xhigh`（未降级）
- round: BUILD/AUDIT R2
- role: plan（READONLY；本文件是唯一落笔）
- 基线: 唯一主干 `cursor/soul-goal1-7b1c` @ `5309656`；工作分支 `cursor/goal1-build-audit-c441`，写作时尖端 `1f52ca5`（opus-a 的文件页空态修复已落在本分支）
- 已读: PLAN_INDEX（`7ca4894`，本树无此文件）、本树 STATUS / PRODUCT_LOCK / DECISIONS / FORMAL_WORK_PROMPT（只认「开工第一动作」）/ GOAL1_PLAN、`origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`（`BLOCKERS_FROZEN` @ `2e72ddf`）、`origin/cursor/goal1-unblock-a073:docs/STATUS.md`（只读对照，不当主干）

## 一句话

本主干上 Goal 1 剩余面已经没有必须派单的 P0 代码任务：唯一已知代码缺口（`/files` 空态谎称「读不到任何文件」）已由 opus-a 在本轮以 `1f52ca5` 重落，剩两件跟班活（STATUS 回写、本地门禁复跑）。挡住关闭的三件事全部 blocked-on-user：hosted Billing（AC-26）、作者 Win11 清单（AC-01 等）、以及 PR #7 所有的 T4D/A0 吸收线的合并决策。D54 双门（验收矩阵 + 产品锁十三片）因此仍不能关；LOOP20 保持 QUEUED，Goal 2 不启动。

注意 BLOCKERS 冻结在 `2e72ddf`，本主干此后已推进：它的 G5（导入入口 + 采集开关）与 S2（KnownIdentifiers 空）在本树**已关闭**（WP09 第四/五段、WP10 遗留 7 消除），不要按 BLOCKERS 原文重派。它的 G1 / G1+ / G2 / G3 / G4 在本主干**仍开放**，但按硬停归 PR #7，见 N6。

## DAG（只列未关闭/本轮项；WP01–WP11、WP13、DPAPI、补证 1–6 均为 done，不再列）

```
N1(done) ──▶ N2(open P1) ──▶ N4(P0, user)
      └────▶ N3(open P1) ──┘
N7(open P1, 条件) ──▶（可能的 opus 修复）──▶ N2
N5(P0, user)、N6(P0, user) 无代码前驱
关闭 Goal 1 = N4 ∧ N5 ∧（N6 的用户裁决）∧ D54 双门
```

### N1 — `/files` 空态诚实 — **done（本轮，`1f52ca5`），审计未过**

- paths: `apps/desktop/src/routes/Files.tsx`、`Files.test.tsx`（各一处；新文案点名目录扫描器 + 两个例外，测试钉「不看任何目录 / 导入页 / 自己的配置与数据库」并断言旧句已不在）
- redlines: 只改文案与测试；`READ_ONLY_NOTICE`、`COMMANDS`（36）、schema、IPC、`config.json` 不动 —— 提交信息声称已守住，**待 fable-b 独立核**
- AC: 无矩阵行；诚实类（产品锁「UI 必须如实写」一族，同 `17b56e9` / `928ef5a` / `f0a2363`）
- not-touch: 上列红线全部
- opus: 0（已完成）
- 状态: done；本分支树上的 `just ui-test` / `ui-lint` 尚无人跑过 → N3

### N2 — STATUS.md 回写本轮 — **open P1**

- paths: 仅 `docs/STATUS.md`（本分支）。STATUS 头两行写明「每个子代理完工必须更新本文件」，`1f52ca5` 没有带 STATUS 条目
- 内容: 文件页空态改句（新旧两句、testid、`COMMANDS` 仍 36）、本轮分支与提交号、hosted 仍空 runner（不许写成绿）、Win11 清单仍未勾
- redlines: 只写本树可验证事实；本地绿 ≠ hosted 绿；不 empty-commit；不动其他文档
- AC: 无（记账纪律）
- not-touch: `docs/PRODUCT_LOCK.md`、`docs/DECISIONS.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/schemas/**`、`docs/GOAL1_PLAN.md`
- opus: 与 N3 合并派 1 个 opus-fast（改动超出父代理 11.3 的 10 行直改白名单的量级；若压到 10 行内也可父代理直改并说明）
- 状态: open

### N3 — 本地门禁在 `1f52ca5` 树上复跑 — **open P1**

- paths: 无产品改动；跑 `just ui-lint`、`just ui-test`（至少 `Files.test.tsx`），有余力跑 `just ci-full`。结果只进 N2 的 STATUS 条目
- redlines: 红了就修测试所在的那一处或回报，不许放宽断言；不得据此声称 hosted 绿
- AC: AC-26 的本机那一半（证据链，不是门禁本身）
- not-touch: `.github/workflows/ci.yml`（不加 job、不改触发；hosted 是账本问题不是 workflow 问题）
- opus: 计入 N2 那 1 个
- 状态: open

### N4 — hosted 五门在 HEAD 上真正开跑（AC-26） — **open P0，blocked-on-user，非代码**

- 现状: 最新产品空 run [32818145279](https://github.com/Xhhemoing/Soul/actions/runs/32818145279)（`478f19f`）五门 0 step、空 `runner_name`，注解是 Billing & plans / spending limit；2026-08-25 07:58 UTC 复查未变
- 用户动作: 处理 GitHub 账本/额度 → `workflow_dispatch` 本主干（或本分支）
- redlines: **硬停** —— 不 empty-commit 去戳 CI；这不是代码任务；不改 workflow「修好它」
- AC: AC-26（HEAD hosted 绿是关闭清单项）
- opus: 0
- 状态: open（等用户）

### N5 — 作者 Win11 手动清单 — **open P0，blocked-on-user，非代码**

- 现状: `scripts/author-manual-checklist.md` 七节零勾。托盘图标 / UAC 观感 / `soul.exe` 进程名 / WebView2 系统层流量 / 真机采集 / 真装真卸（NSIS 从 **`478f19f` 或之后**本机 `tauri build`，不许用 `2e72ddf` 旧工件；卸载不得删 `keys.dpapi`）
- redlines: **硬停** —— 不是代码任务；CI 不能替；不许在 STATUS 假装过了。§8 界面导入、§9 本机端点、§10 语气与审计是可选项，不是门禁
- AC: AC-01 全部 + AC-21/AC-22 的肉眼那一半
- opus: 0
- 状态: open（等作者）

### N6 — T4D/A0 吸收与集成线裁决 — **open P0，blocked-on-user；归 PR #7**

- 现状: BLOCKERS 的 G1（图仍是 T0 常量 3/10/3，无 180/360 / 全库 `as_of`）、G1+（owner 群消息对历史发言人扇出）、G2（intake 绕过轴锁）、G3（图不可纠正）、G4（导入去重，P1）在**本主干**未做；`cursor/goal1-unblock-a073`（PR #7）自述已全部关闭并吸收了 `main` 的计划面（AC-28+）。PR #2 对 `main` CONFLICTING 是故意的（`5309656` 记录在案），PR #10 叠在 #7 上
- 用户动作: 裁决唯一集成线（#2 线 vs #7 线）与合并顺序
- redlines: **硬停** —— 不合 PR #4 / #7 / #10；**禁止在本主干秘密重实现 T4D/A0 接线**（那是把两条实现线的历史重演一遍）；不把「合 `main`」当修冲突的办法
- AC: 矩阵外，但挡 D54 的「诚实关闭」（产品锁切片 3/4 的算法语义由 `ALGO_FROZEN` 定）
- not-touch（本主干上）: `crates/soul-graph/src/build.rs` 的档位常量、`crates/soul-import` 的 commit 扇出、`crates/soul-profile` 的 intake 锁路径、schema 与 `schemas.lock.json`
- opus: 0
- 状态: open（等用户）

### N7 — 本轮审计/探针的发现 — **open P1（条件）**

- 输入: fable-b（独立 AUDIT）、gpt-sol-a（文案-行为）、gpt-sol-b（出网/边界）三份报告，落 `.agent_workspace/orchestrator-c441/round2/`
- 处置: 发现属实且是代码缺口 → 按既有诚实类修法派 opus-fast（每缺口一单，路径最小化）；发现属于矩阵外新要求 → 记 STATUS，不扩工作包（D30 只减不增）
- redlines: 不重派已完成 WP 为 greenfield；不做 AC-27；不动 `COMMANDS`（36）与 `StoredConfig`（两字段）除非缺口本身要求且有测试钉住
- opus: 预留 0–2（按报告定，宁缺）
- 状态: open（等本轮其余子代理）

### P2（记账停放，本轮不派）

| 项 | 归属 | 为什么停 |
|---|---|---|
| G4 导入去重（`sha256("soul.import.dedup.v1\|source\|canonical_external_id")`） | 身份/事务设计，随 N6 线走 | BLOCKERS 明说不能当小修 P0；矩阵无此行 |
| 大小写敏感 NTFS 目录上 `CaseFolded` 会收下无人授权的 `alpha` | 产品 P2（BLOCKERS M3 尾注） | 预期红，禁止用 `cfg` 藏 |
| `/audit` 无分页、`collect_status` 每刷全数一遍 | 已记录的取舍 | 行数没到，修法是存储边界改动 |
| 真机可选项（清单 §8/§9/§10） | 作者 | 标着可选，不是门禁 |

## 派单建议汇总

本轮再派 **1 个 opus-fast**（N2+N3 合一单：跑本地门禁、把结果与 `1f52ca5` 写进 STATUS），外加 **0–2 个条件预留**给 N7。不为 N4/N5/N6 派任何实现单。不凑数。

---

## 输出合同

- **round**: BUILD/AUDIT R2
- **role**: plan（fable-a，只读）
- **scope**: Goal 1 close-out 在唯一主干 `cursor/soul-goal1-7b1c` @ `5309656`（分支 `cursor/goal1-build-audit-c441`，尖端 `1f52ca5`）上的剩余/未关闭 DAG；不含 Goal 2，不含 AC-27，不重派 WP01–WP11/WP13
- **done**: N1（文件页空态诚实，`1f52ca5`，opus-a）；此前主干已完成 WP01–WP11、WP13 两段、DPAPI、WP09 十段、补证一至六轮、`session_matrix_replay`；本机 `just ci-full` 曾在 `478f19f` 树上全绿
- **open**:
  - **P0（全部 blocked-on-user，非代码）**: N4 hosted AC-26（GitHub Billing → `workflow_dispatch`）；N5 作者 Win11 清单（AC-01、AC-21/22 肉眼半）；N6 集成线裁决与 T4D/A0 吸收（归 PR #7，禁止本主干重实现）
  - **P1**: N2 STATUS 回写；N3 本地门禁在 `1f52ca5` 树复跑；N7 本轮审计/探针发现的条件修复
  - **P2**: G4 去重、`CaseFolded` 大小写敏感目录、`/audit` 分页与 `collect_status` 计数、真机可选项 §8/9/10
- **tests**: 无（只读规划，未跑任何测试；本轮对 `1f52ca5` 的验证归 N3）
- **assumptions**: ①当次 Goal 按派单假设为 Goal 1 close-out 续跑，LOOP20 QUEUED；②`1f52ca5` 尚未经本地门禁复跑，暂按提交自述记 done；③BLOCKERS 以 `2e72ddf` 为准，其 G5/S2 在本主干已关闭、G1/G1+/G2/G3/G4 归 PR #7 线；④`origin/cursor/goal1-unblock-a073` 的 STATUS 只作对照，不作本主干事实
- **do_not_touch**: PR #4/#7/#10（不合、不评、不摘）；`docs/PRODUCT_LOCK.md`、`docs/DECISIONS.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/GOAL1_PLAN.md`、`docs/schemas/**` 与 `schemas.lock.json`；`.github/workflows/ci.yml`；`COMMANDS`（36）与 `StoredConfig`（两字段）；`crates/soul-graph` 档位常量、`crates/soul-import` 扇出、`crates/soul-profile` intake 锁（N6 属地）；不 empty-commit；不做 AC-27；不启动 Goal 2；hosted 账本与 Win11 清单不当代码任务
- **next**: ①父代理收 fable-b / gpt-sol-a / gpt-sol-b 报告，按 N7 决定 0–2 个 opus-fast；②派 1 个 opus-fast 做 N2+N3；③向用户请求 N4（Billing + dispatch）、N5（清单）、N6（集成线裁决）三件事，任何一件回来前 Goal 1 保持不能关、LOOP20 保持 QUEUED
