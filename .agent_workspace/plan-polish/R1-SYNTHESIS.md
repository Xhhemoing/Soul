# Round 1 结论简报 — 打磨和完善项目计划

父代理：cursor-grok-4.6-high。六路模型均按指定 slug 出活，无静默降级。

核于 2026-08-25。工作树 `cursor/polish-project-plan-5280`。

## 已实现（本轮父代理已合入 `docs/`）

1. **计划权威面落到能合 `main` 的树上**：PR #1 的锁 / 拍板 / FORMAL / SECURITY / schema 不再只困在 Draft。
2. **吸收 `ALGO_FROZEN`**：PRODUCT_LOCK 新增「灵魂层算法（v0.1）」；人脉=T4D、特质=A0、A2 纯渲染器、A1 空转是预期；常量不复抄。
3. **拍板编号防撞**：保留 Goal 1 吸收线 D32–D40；本轮新增 D41–D58（docs 单源、T4D、F04c 禁第三道门、双门关闭、schema 义务等）。
4. **STATUS 三态诚实**：`PLAN_FROZEN` ≠ Goal 1 关闭 ≠ `main` 已有应用。杀掉「尚未写应用代码」全局假话。
5. **验收矩阵**：AC-08 补 T4D；AC-12 产品缝；AC-26 按 D56 读「package job」；新增 AC-28–AC-33；关闭语义改为矩阵 ∩ 切片（D54）。
6. **schema 倒退关闭**：本分支 `docs/schemas/` 改为 Goal 1 已 `$ref` `_defs` 的正文 + `schemas.lock.json`。`tie_strength` 仍为裸 object（D58，Round 2 攻坚）。
7. **单源指针**：`docs/PLAN_INDEX.md`；`.agent_workspace/context/plan/README.md` 声明非权威。
8. **夹具叙述校准**：`DECISION.md` §1 / §4.1 以代码夹具为准（30/10 与 `group_heavy_plus_three_directs`），**不改冻结结论**。

过程稿：`.agent_workspace/plan-polish/round1/{fable-a,fable-b,opus-a,opus-b,gpt-sol-a,gpt-sol-b}/`。

## 遗留缺陷

| ID | 缺陷 | 下轮处理 |
|---|---|---|
| L1 | `tie_strength` 仍是裸 `object`；T4D 字段清单未进 schema 字节 | Round 2：按 opus-b 候选 + D58 收紧，并重算 lock |
| L2 | FORMAL「接下来使用 Goal」历史段仍诱导重派 planner | Round 2：加历史注记，不删作者原话 |
| L3 | SECURITY 仍是 PR #1 短版；Goal 1 有加密落地/DPAPI 两节 | Round 2：指针，不把实现实证提前写成 main 已落地 |
| L4 | 过程目录六份计划副本仍与 docs 字节相同 | Round 2：保留快照，加 CI/探针断言「不得当权威」 |
| L5 | BLOCKERS.md 尚未同树 | 不在本 PR 整份拷贝；索引已指向 PR #6 |
| L6 | AC-28–33 尚未有本分支可跑的产品路径（无 Goal 1 crate） | 计划义务；实现在 Goal 1 线。本分支可用算法 crate 做夹具名存在性探针 |

## 性能瓶颈

本任务是文档权威面，无运行时瓶颈。探针显示：本分支 workspace 仅两个算法 crate；`cargo test --workspace` 应保持绿。schema 编译在 Goal 1 用 xtask freeze；本分支拷入 lock 后 Round 2 须确认哈希与文件一致。

## 下轮攻坚重点

1. 把 `relationship.tie_strength` 类型化且不破坏 Goal 1 现有序列化（dependentRequired 方案）。
2. 交叉核验 D32–D58 与 FORMAL AC 无编号/语义冲突；PRODUCT_LOCK 不出现常量数字字面量。
3. README / PLAN_INDEX / STATUS / FORMAL 四处拓扑表对拍。
4. 探针：schema lock 哈希、AC 覆盖矩阵、禁止项（OAuth/E0/文件写）负向扫描、双源哈希。
5. SOTA：新父代理只读 `docs/PLAN_INDEX.md` 即可开工，不必翻 `.agent_workspace`。
