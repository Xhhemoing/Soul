# Round 2 fable-a — Round 1 落地演进核验

MODEL_SLUG: claude-fable-5-thinking-xhigh

核于 2026-08-25 02:50 UTC。工作树 `cursor/polish-project-plan-5280`，HEAD `b380f67`。

## 0. 审核对象与并发注记（先读，否则下文时态会误导）

本轮审核期间，**工作树在被并发修改**：HEAD 仍是 Round 1 提交 `b380f67`，但 02:49 起出现七份未提交改动（`FORMAL_WORK_PROMPT.md`、`PLAN_INDEX.md`、`PLAN_VERIFY_PROMPT.md`、`SECURITY.md`、`STATUS.md`、`schemas/relationship.schema.json`、`schemas/schemas.lock.json`）——即父代理正在同轮落地 Round 2 其他槽位的产出（L1/L2/L3 攻坚）。

本报告因此分两个基线陈述：**R1 基线**（提交 `b380f67` 的字节）与**在飞基线**（02:50 工作树字节）。凡写「在飞」的结论，合并前须以父代理最终落盘为准。

另一个环境事实：本 VM 的 git 克隆原为 shallow（深度 1），跨分支拓扑结论在 shallow 状态下**全错**（曾测得 Goal 1 与 main「无共同祖先」）。本轮已 `git fetch --unshallow` 后重测，下述拓扑结论均出自完整历史。后续轮次先查 `git rev-parse --is-shallow-repository`。

## 1. R1-SYNTHESIS 八项已实现声明——逐条复核

| # | R1 声明 | 本轮核验 | 判 |
|---|---|---|---|
| 1 | 计划权威面落到能合 main 的树上 | `docs/` 齐：锁/拍板/FORMAL/SECURITY/STATUS/PLAN_INDEX/schemas+lock。`origin/main` 尖端仍是 `7b35bde`（= 本分支基底），PR #8 合入无冲突 | **真** |
| 2 | PRODUCT_LOCK 新增「灵魂层算法（v0.1）」，常量不复抄 | 该节在（§65–88）；全文 grep 无 3/10/180/360 阈值字面量，「半年/一年」处显式声明不复抄数值 | **真** |
| 3 | D32–D40 与 Goal 1 吸收线同号同义；D41–D58 新增 | 与 `origin/cursor/goal1-unblock-a073:docs/DECISIONS.md` 逐条比对：D32–D40 同号同义（措辞有微差，见 RESIDUAL P1-5）。但 DECISIONS 头部增补注仍写「**D41–D56**」，表实到 D58——防撞装置自身记账过期 | **基本真，有 P1 缺陷** |
| 4 | STATUS 三态诚实 | 三态分栏在；「尚未写应用代码」全局假话已死。但阻塞表仍写「`tie_strength` 仍为裸 object」，与在飞 schema 字节**同树自相矛盾**（见 RESIDUAL P1-2） | **R1 基线真；在飞基线出现新假话** |
| 5 | 验收矩阵 AC-08 补 T4D、AC-28–33 新增、D54 双门 | 全部在；AC-28–33 钉在产品边界并指名夹具（夹具名实测存在于 `crates/soul-algo-tie/src/testing`） | **真** |
| 6 | schema 倒退关闭：Goal 1 接线正文 + lock | R1 基线：12 份文件与 Goal 1 尖端逐字节比对全同（用旧尖端 `6c91d39` 验）；lock 哈希与文件实测全部一致。在飞基线：`relationship` 已类型化（见 §2 L1） | **真** |
| 7 | PLAN_INDEX 单源指针；context/plan 声明非权威 | PLAN_INDEX 在且在飞版又补三行；`.agent_workspace/context/plan/README.md` 首行即「历史快照，不是权威」 | **真** |
| 8 | DECISION.md §1/§4.1 夹具叙述以代码为准 | §1 表内已写「叙述曾写 100/50，以测试源为准」；FORMAL 同步 | **真** |

R1「性能瓶颈」节的两个断言也复核了：`cargo test --workspace` 在本分支 **249 通过 / 0 失败**（Rust 1.83.0）；schemas.lock 哈希与文件一致（含在飞版：`relationship` 新哈希 `350a7ffc…` 与新字节相符）。

## 2. 遗留 L1–L6 的演进

| ID | R1 时态 | 现在时态 |
|---|---|---|
| L1 `tie_strength` 裸 object | 待 Round 2 | **在飞已类型化**：band（`_defs` evidenceBand ∩ weak/moderate/strong）、`algorithm_id ∈ {T4D,T4}`、四分列计数、`direct_active_day_count`、`silent_days`、`as_of_utc`、`last_direct_contact_utc`（可 null 可缺席），`dependentRequired` + `if algorithm_id then required[15]`，未声明算法时保持今日宽松度。lock 已重算且实测相符。**兼容性正向核验**：Goal 1 现行八字段（`model.rs`）全部在枚举表内，band snake_case 相符，`algorithm_id` 实值 "T4D"/"T4" 与枚举相符。**但有一个 P0 反例**：unblock 线（PR #7）每条重建边都序列化 `machine_band`（`build.rs:161` 恒 `Some`），锁定边另有 `user_band`/`locked_by_user`——三者**不在**枚举表内，`additionalProperties:false` 会整边拒绝。这不是假想：Goal 1 的 `soul-schema` 用 `include_str!` 把 schema 字节编译进 crate 并以 jsonschema 2020-12 验证实例。**详见 RESIDUAL P0-1，此项不修完 L1 不能算关闭** |
| L2 FORMAL 历史段诱导重派 | 待 Round 2 | **在飞已修**：原话整段标「历史段（…不要照着再跑一遍）」，加引导块声明 CreateGoal/派 planner 已发生、第 2–5 条作废；「开工第一动作」升格为唯一可执行路径；正文第 6 条、PLAN_INDEX 两处、PLAN_VERIFY 存档横幅三面夹击。作者原话逐字保留 ✓ |
| L3 SECURITY 短版 | 待 Round 2 | **在飞已修**：新增「实现实证在哪」指针节，指明加密落地/DPAPI 两节在 PR #2（带分支+提交号+核于），并明令**不得提前抄进本文件**、不扩大威胁模型承诺面。未把实现实证写成 main 已落地 ✓ |
| L4 过程目录副本 | 保留快照+加探针 | **半开**：`context/plan/` 七份副本因 docs 前进而**全部漂移**（实测 7/7 differs）——从「字节相同的双源」变成「过期的旧版」。非权威横幅在；R1 说的「CI/探针断言」没有落。风险从双源变成误读旧版，见 RESIDUAL P2-5 |
| L5 BLOCKERS 不同树 | 索引指向 PR #6 | **不变且指针更准**：在飞 PLAN_INDEX 明写「仍在 PR #6，本树没有这份文件，本计划 PR 不整份拷贝」。另发现其合入时会引爆 D32 撞号（RESIDUAL P1-4） |
| L6 AC-28–33 无本分支可跑路径 | 计划义务 | **不变，且定性正确**：夹具名存在性已可在本分支验（testing 模块在 main 的 crate 里）；产品路径实现在 Goal 1 线。unblock 线实测已在做（`t4d_adapt.rs` 调 `soul_algo_tie::score`） |

## 3. 现在为真（Round 1 前为假或不存在）

1. `main` 可长出完整计划权威面：本分支与 `origin/main` 尖端零漂移，合入即净加文件。
2. 算法冻结口径在锁、拍板、验收三处同判，常量单点纪律在 PRODUCT_LOCK/FORMAL 均无阈值字面量（在飞版还加了红线 11 自查段，把夹具身份数字与阈值显式区分）。
3. 新父代理从 PLAN_INDEX 单入口可开工（详见 SOTA_REREVIEW，判定：达标带尾巴）。
4. schema 契约与 Goal 1 尖端 10/12 文件逐字节同一；分歧只剩 `relationship` + lock 两处，且是**有意的收紧**而非倒退。
5. 双门关闭语义（矩阵 ∩ 十三切片，D54）在 FORMAL 正文、开工路径、PLAN_INDEX 三处一致。

## 4. 仍为假 / 新出现的假

1. **「L1 已关闭」为假**：类型化 schema 与 D32/D48/AC-32 的锁定字段互斥（P0-1）。修掉三个字段前，这份 schema 合到 Goal 1 会让每条重建边验证失败。
2. **「STATUS 诚实」在在飞树上出现一处新假**：阻塞表「`tie_strength` 仍为裸 object」已被同树 schema 字节否定（P1-2）。
3. **「D58 记录了现状」为假**：D58 说「先采纳 Goal 1 正文、进一步收紧须与 lock 同批重算」——收紧已经发生、lock 已在本分支重算，但没有任何拍板行记录这次收紧及其对 Goal 1 的同批重冻义务（P1-3）。
4. **「核于 2026-08-25 的 Goal 1 尖端」一树两说**：STATUS 写 `3161e02`，在飞 SECURITY 写 `6c91d39`，实际远程已到 `52431d6`。各自都符合 D57（带提交号+日期），但同树互相矛盾（P1-6）。
5. **「D32–D40 防撞完成」只对本树为真**：BLOCKERS（PR #6）正文里的「D32」指 dev-sota 的 e0 发送 crate 禁令；它合入后 `docs/` 里将同时存在两个 D32 语义（P1-4）。DECISIONS 脚注预言了这件事，但没给渐进处置步骤。

## 5. 本轮跑过的探针（全部可复跑）

| 探针 | 结果 |
|---|---|
| `sha256sum docs/schemas/*` vs lock | 12/12 一致（R1 基线与在飞基线各测一次） |
| 12 份 schema vs `origin/cursor/soul-goal1-7b1c:docs/schemas/` | 尖端 `52431d6`：10 同 / 2 异（relationship、lock，即在飞收紧） |
| 阈值字面量负向扫描（3/10/180/360）于 PRODUCT_LOCK/FORMAL/STATUS/README/SECURITY/PLAN_INDEX | 无阈值泄漏；FORMAL 仅夹具身份数字（30/10、12/6/3 等），在飞自查段已定性；仅 DECISIONS D52 引 `180` 字面量一处（P2-2） |
| `cargo test --workspace` | 249 通过 / 0 失败 |
| D 编号连续性 | 表内 D1–D58 无缺号无重号；头部增补注范围写错（D41–D56） |
| 合并模拟（临时 worktree，试合后 abort） | 13 处冲突，逐文件清单与解法见 MERGE_RISK.md |
| unblock 线 `TieStrength` 序列化面 vs 在飞 schema | 15 个枚举字段全对齐 + 3 个未枚举字段（machine_band/user_band/locked_by_user）→ P0-1 |
