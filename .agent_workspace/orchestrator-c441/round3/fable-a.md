[Model: claude-fable-5-thinking-xhigh]
round: BUILD R3
role: plan
scope: architecture confirmation, process files only — 确认并映射已冻结的架构/计划（`PLAN_FROZEN` + `GOAL1_PLAN` + `PRODUCT_LOCK` + `DECISIONS` + `origin/main` 的 `ALGO_FROZEN`），不重开 D1–D60，不 CreateGoal，不启动 Goal 2，不提第二份 PRODUCT.md。基线：主干 `cursor/soul-goal1-7b1c`，分支 `cursor/goal1-build-audit-c441` @ `2bd56a2`。
done:
- 读齐权威面：本树 `PRODUCT_LOCK.md` / `DECISIONS.md`（D1–D31）/ `GOAL1_PLAN.md` / `FORMAL_WORK_PROMPT.md` / `SECURITY.md` / `STATUS.md`；`origin/main:docs/PLAN_INDEX.md`（权威分布 + 仓库拓扑）；`origin/main:docs/algorithms/DECISION.md`（`ALGO_FROZEN`：T4D/A0/A2/A1、常量表、回退链、第 6 节合并义务）。
- 逐 crate 核 DAG：16 个 `crates/*/Cargo.toml` + `apps/desktop/src-tauri/Cargo.toml` + `deny.toml`，对照 GOAL1_PLAN「钉死」四条（schema 无 IO、store 唯一落盘、egress 唯一 HTTP、policy 唯一签发 permit/令牌、core 只编排、UI 无业务逻辑），另做 src 级 fs/HTTP grep 与 `commands.rs` / `one_store.rs` / `no_write_api.rs` 抽查。
- 落 `ARCHITECTURE_LOCK.md`（同目录）：冻结声明与权威分布、三层散文图、crate DAG 与四条钉死的证据、十三片切片指针（指向 PRODUCT_LOCK，不复述立法）、不归架构裁量清单、漂移结论。
open:
- 无本轮新开口。挡关闭的仍是 R2 已记录的三件 blocked-on-user（hosted Billing→AC-26、作者 Win11 清单、PR #7 集成线裁决），均非架构问题，本文件不重开。
tests: none required (read-only)
p0/p1/p2:
- P0: 无新增（三件既有 P0 全部 blocked-on-user，归 R2 N4/N5/N6，不属架构确认）。
- P1: 无。DAG 漂移为零，无需派 opus。
- P2（记账，不派）: `ALGO_FROZEN` 第 6 节合并义务（`soul-graph` 判档换 T4D、常量单点化）在 PR #7 线裁决前保持停放；本树缺 `PLAN_INDEX.md` / `algorithms/DECISION.md` 是 PLAN_INDEX 第三节写明的拓扑现状，合并顺序已有安排，不当缺陷报。
assumptions:
- ①「确定架构和计划」= 确认冻结面并画对照图（父代理 PROGRESS「路由」的 ASSUMPTION），不是重写 PRODUCT_LOCK 或重派 WP01。
- ②`ALGO_FROZEN` 权威在 `origin/main`；本 Goal 1 树 `soul-graph/src/build.rs:40-44` 的本地 3/10/3 按该决议 §6.5 记「过渡期语义」，不算 DAG 漂移。
- ③`.agent_workspace/**` 非权威（PLAN_INDEX 第四节）；`ARCHITECTURE_LOCK.md` 自述为对照图，与权威冲突时改它自己。
- ④STATUS 措辞仅提案（见下），由父代理决定是否派 opus-fast 回写。
do_not_touch: apps/**、crates/**、docs/PRODUCT_LOCK.md、docs/DECISIONS.md、docs/FORMAL_WORK_PROMPT.md、docs/GOAL1_PLAN.md、docs/schemas/** 与 schemas.lock.json、docs/STATUS.md（本轮只提案不落笔）；PR #4/#7/#10 不合不评；不 empty-commit；不做 AC-27；不启动 Goal 2；不在本主干重实现 T4D/A0。
next: concrete opus scopes — **none**（无 DAG 漂移）。给父代理的两条非派单事项：①若要把本轮结论进 STATUS，建议措辞（提案，仅一句，落「当前里程碑」段尾或进度表）：「R3 架构确认：16 crate 对照 GOAL1_PLAN 钉死逐条核过，无 DAG 漂移；对照图在 `.agent_workspace/orchestrator-c441/ARCHITECTURE_LOCK.md`（过程稿，非权威）。」②等 fable-b / gpt-sol-a / gpt-sol-b 报告后再定条件修复，与本文件无耦合。
