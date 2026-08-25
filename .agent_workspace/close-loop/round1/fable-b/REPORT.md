claude-fable-5-thinking-xhigh

（自报依据：本会话系统身份为 Claude Fable 5，与请求 slug 同系列同代；运行时未向本代理暴露服务端变体串，无任何降级信号。若父代理侧记录的实际 slug 与此不符，以父代理记录为准并视为本行失效。）

# Round 1 fable-b — 拓扑与 SOTA 一致性审查（close-loop）

核于 2026-08-25，分支 `cursor/goal1-close-loop-a073`。**本轮工作树在审查期间持续移动**：开始时 HEAD 为 `10da234`，审查中并发落入 `f8fe32f`（opus-b：CI 触发面修复）、`52cc368` / `cdca740`（gpt-sol-b：R-1 探针），且 `crates/soul-graph/src/correct.rs` 有他人未提交的在飞改动（+60/-8，未入本报告判定）。以下所有「现状」判定核于 **`cdca740`**；写明「开始时」的判定核于 `10da234`。只读审查，未改任何权威文件；产出仅本文件与同目录 `DOC_PATCHES.md`。

## 判定总表

| # | 项 | 判定 |
|---|---|---|
| 1 | D49 旧主干名 | 属实（`docs/DECISIONS.md:60` 仍写 `cursor/soul-goal1-7b1c`）。最小合法修法：**追加 D61，不改写 D49 原行**，连带 PLAN_INDEX:12 的「D1–D60」跳号。见 DOC_PATCHES 补丁 1/2 |
| 2 | PR #4 | 属实：OPEN、`CONFLICTING`、base `main`。**不得合并**（D49 + 机械上也合不了）。关闭是 GitHub 记账，本环境 `gh` 只读，留给人工/父代理。**无需新增文档**：D49 原文与 `STATUS.md:821` 均已记「应停并关闭」 |
| 3 | ci.yml 自动 push 缺主干 | **开始时属实，本轮已被 `f8fe32f` 修复**（非本 slot 所为）。核于 `cdca740`：`on.push.branches` 与五个 job 的 `if:` 门都已含 `cursor/goal1-unblock-a073`；未加 `pull_request`；`workflow_dispatch` 未动。不再提补丁 |
| 4 | PLAN_INDEX「只写常量名」vs D60 | **主冲突已在 Round 3（`095c1f8`）修掉**：豁免句已在 PLAN_INDEX:26。残余两处小错位（DECISION.md §1 自身的数字不在豁免域；D60 被当成豁免的授权源是引申）。精确替换句见 DOC_PATCHES 补丁 3 |
| 5 | 本堆叠分支是否合法 | **合法，附四个条件**（见 §5）。它是 PR #7 主干的工作分支，不是第二条实现线；落地必须以 merge 进 `cursor/goal1-unblock-a073` 收尾，禁止从本分支另开对 `main` 的 PR |

---

## 1. D49 旧主干名（DOC-D49）

**事实。** `docs/DECISIONS.md:60`：

> | D49 | 唯一实现主干 | `cursor/soul-goal1-7b1c`。停 `agent/dev-sota`、关 PR #4；分支一律 `cursor/` 前缀 | `BLOCKERS.md` M1 |

而现行唯一主干是 PR #7 `cursor/goal1-unblock-a073`（`gh pr view`：OPEN / `MERGEABLE`，base `main`）。PLAN_INDEX §3、FORMAL「计划面与代码面」表、STATUS:9 都已按新主干写，**只剩 DECISIONS 自身没对齐**——D49 拍板时 PR #7 还不存在，这是历史事实，不是笔误。

**追加 vs 改写：追加。** 三条依据：

1. SHARED_BRIEF DOC-D49 原话：「Append, do not rewrite history silently」。
2. 本仓库有正反两个先例：D58 原文不改、追加 D59 承接收紧（正面）；D57 被原行增补，round3/fable-b REGRESSIONS R-3 记为纪律裂缝（反面）。
3. DECISIONS.md 是拍板**记录**；改写 D49 原行会让历史读者无法从本文件重建「当时为什么钉 7b1c」。

**最小补丁形状**（精确文本见 DOC_PATCHES 补丁 1/2）：

- 在 D60 行后追加一行 **D61**：问题列写明是「D49 主干名的更替」，决定列写新主干并**重申 D49 其余裁决不变**（停 `agent/dev-sota`、关 PR #4、`cursor/` 前缀），把 `cursor/soul-goal1-7b1c` 明确降为历史祖先。
- 连带一处机械跳号：PLAN_INDEX:12「已拍板的选择（D1–D60）」→「（D1–D61）」。先例：Round 3 追加 D60 时，`095c1f8` 同步把该行从 D1–D59 跳到 D1–D60。
- **刻意不动**：FORMAL:16 的「计划权威面（D1–D60…）」（描述的是当时 merge 吸收事件，追加 D61 不使其为假）；STATUS:9/:23 的「D41–D60」（同为历史吸收陈述）。STATUS:817 拓扑表的「D1–D60」会随落地时的 STATUS 刷新自然更正，不必在本补丁夹带。
- **可选**（补丁 4）：PLAN_INDEX:25「理由见 D49（唯一实现主干）」→「理由见 D49/D61」。不是必须——D61 追加后 D49 单看有旧名，但 PLAN_INDEX §3 已给现行拓扑，链路不会把读者引错。

**注意占号**：DECISIONS.md:73 已预留「PR #6 整份合入时，BLOCKERS 里的假 D32 改写为**届时下一个空闲 ID**」。D61 被本补丁占用后，那个「下一个空闲 ID」变成 D62 起；落地 PR #6 的人需按当时表尾取号，此处无需改字。

## 2. PR #4 `agent/dev-sota`（M1-PR4）

核于 2026-08-25 `gh pr list`：PR #4 OPEN，head `agent/dev-sota`，base `main`，`mergeable: CONFLICTING`。

- **不得合并**：D49 明令「停 `agent/dev-sota`、关 PR #4」；PLAN_INDEX §3 与 FORMAL 计划面第 2 条都把 PR #4 排除在合入路径外；且它对 `main` 是 CONFLICTING，机械上也合不进。三重锁死。
- **关闭动作**：本环境 `gh` 只读（实测未尝试写操作；SHARED_BRIEF M1-PR4 已声明），关闭是 GitHub 记账，归人工/父代理。**这不是本树上的代码或文档缺口**。
- **文档面已闭合，无需补丁**：D49 原文即含关闭令；`docs/STATUS.md:821` 分支拓扑表已记「`agent/dev-sota`（PR #4）… 按 D49 应停并关闭」；本报告提议的 D61 决定列重申了这一令，使其在主干更替后继续可见。
- 附带提醒（已有账，不新开）：DECISIONS.md:73 记录 dev-sota 线曾有一条被 BLOCKERS 称作「D32」的假号——关闭 PR #4 不影响该条款，樱桃摘时仍禁复用本表号。

## 3. ci.yml 自动 push 面（CI-TRUNK）

**开始时（`10da234`）属实**：`on.push.branches` 只有 `main` + `cursor/soul-goal1-7b1c`，五个 job 的 `if:` 门同样只放行这两个 ref——推主干不会起任何 hosted 检查。与 SHARED_BRIEF CI-TRUNK 一致。

**本轮已被并发修复**：`f8fe32f`（opus-b slot，`ci: trigger auto push on the Goal 1 trunk cursor/goal1-unblock-a073`）。核于 `cdca740` 复核该修复的一致性：

- `on.push.branches` 现为 `main` / `cursor/goal1-unblock-a073` / `cursor/soul-goal1-7b1c`（ci.yml:26–28）；五个 job 的 `if:` 门同步加了主干 ref（:51、:108、:168、:241、:276）。**只加触发不加门会「起 workflow、跳全部 job」**，该提交两处都改了，判定正确。
- 红线守住：**没有**加 `pull_request` 触发（分钟禁令）；`workflow_dispatch` 未动；job 步骤零改动。
- STATUS 第 2 条同步了一句，且措辞诚实：「这只是触发面，hosted 仍未跑过、更谈不上绿」。符合 D57（只写本树可验证的事实）。
- **保留 `cursor/soul-goal1-7b1c` 触发的取舍**：提交说明的理由是「祖先分支、PR #2 仍开、闲置触发不花分钟」。可辩护——没人该再往那推，触发闲置即零成本；严格派会说 D61 落地后它「不是主干」就该摘。判定：**不是缺陷**，摘除可搭任何后续触碰 ci.yml 的顺风车，不值得单开一笔。

**本 slot 不再对 ci.yml 提任何补丁。**

## 4. PLAN_INDEX「只写常量名」vs D60（DOC-INDEX）

**主冲突已修**。R-2 指控的原句「其余文档一律只写常量名，写出数字即为缺陷（FORMAL 红线 11）」在 Round 3 `095c1f8` 已被替换；核于 `cdca740`，PLAN_INDEX:26 现为：

> 只有两处：`algorithms/DECISION.md` 第 3 节常量表（语义）与 `crates/soul-algo-tie` 常量模块（取值）。产品锁 / 拍板 / FORMAL / STATUS / README / PLAN_INDEX / SECURITY 只写常量名。`algorithms/COPY_ZH.md` 与 `REJECTED.md` 允许引用常量表已钉的数字（D60）

对红线 11 的错误引申也已随替换消失（红线 11 管产品 crate 源码，FORMAL:166 的自查段现与之自洽）。COPY_ZH（:27、:42、:73）与 REJECTED（:37、:86）的实测数字全部落进豁免域。

**残余两处小错位**（就是本任务要的「exact replacement sentence」的对象）：

1. **`DECISION.md` §1 自身不在豁免域**。`docs/algorithms/DECISION.md:39`「T3 门闩 + 180/360 整数降档」在授权的「第 3 节常量表」之外，又不在「COPY_ZH 与 REJECTED」豁免名单里。较真的审计者仍可拿 PLAN_INDEX:26 判这份 `ALGO_FROZEN` 文件缺陷——R-2 预言的坑只填了三分之二。
2. **（D60）作为豁免授权源是引申**。D60 的决定列管的是 COPY_ZH P4 的**比较符**（`>= DEMOTE_ONE_BAND_DAYS`），不是「可以引用数字」这条一般规则；豁免的真实依据是三份算法文档本身就是 `ALGO_FROZEN` 裁决记录。保留 D60 引用（它确实钉了 COPY_ZH 的写法），但把豁免的理由落在「裁决记录」上。

**精确替换句**（只换 PLAN_INDEX:26 的末句，前两句不动）：

- 旧：「`algorithms/COPY_ZH.md` 与 `REJECTED.md` 允许引用常量表已钉的数字（D60）」
- 新：「`docs/algorithms/` 三份冻结文档（`DECISION.md`、`COPY_ZH.md`、`REJECTED.md`）是裁决记录本身，允许出现常量表已钉的数字；COPY_ZH 的比较符写法以 D60 为准」

完整行文见 DOC_PATCHES 补丁 3。此改动只放宽豁免名单、不动「定义只有两处」的主句，不触碰任何 `ALGO_FROZEN` 文件本身。

## 5. 本堆叠分支 `cursor/goal1-close-loop-a073` 是否合法

**判定：合法，条件成立时。** 它是主干的工作分支，不是第二条实现线。

**字面张力**：FORMAL「开工第一动作」第 4 步写「未合入 `main` 的实现工作只在 `cursor/goal1-unblock-a073`（PR #7）上进行」。本轮 `f8fe32f`（CI 改动）、R-1 探针等提交发生在本堆叠分支上，字面上不在 PR #7「上」。

**为什么仍合法**：

1. **规则的靶子是第二条合入路径，不是工作分支。** D49 的语境（BLOCKERS M1）与 FORMAL 计划面第 2 条的展开都是「不要另开第二条实现线、不要把 PR #1/#2/#4 当合入路径」。一条从主干 HEAD 切出、唯一去向是 merge 回主干的分支，不产生第二条对 `main` 的路径。
2. **机械事实**：本分支切自 `6133307` = `origin/cursor/goal1-unblock-a073` 当前 HEAD（实测 `merge-base --is-ancestor` 成立）。只要 PR #7 不动，落地就是 fast-forward；唯一合入 `main` 的路径仍是 PR #7。
3. **先例**：PR #9（`cursor/predict-algo-survey-a073`）就是 base 指向 `cursor/goal1-unblock-a073` 的堆叠 PR，OPEN 且无人判其违规。
4. **命名合规**：`cursor/` 前缀（D49）+ `-a073` 环境规约（SHARED_BRIEF 硬禁令）。`agent/` 分支零使用。

**四个条件（违反任意一条即转为违规）**：

1. 本分支**永不**对 `main` 开 PR；唯一去向是 merge 进 `cursor/goal1-unblock-a073`。
2. 必须在 PR #7 合入 `main` **之前**落地回主干，否则本轮全部代码修复（CI 触发面、R-1）会从唯一合入路径上消失。
3. 权威结论必须随落地进 `docs/`；`.agent_workspace/**` 本身不是权威（PLAN_INDEX §4），本报告与 DOC_PATCHES 只是过程材料。
4. 不得演化成长期分叉线（例如在 PR #7 移动后不回收、继续独立堆提交）。

**回答任务原问**：不是「work must land as commits on PR #7 only」的字面苛刻读法——堆叠分支作为工位允许存在；但**落地语义上等价于「commits on PR #7 only」**：一切最终必须以主干上的提交形态存在，本分支只是到达那里的运输方式。

## 附：并发纪律观察（本轮事实，供父代理汇总）

- 多 slot 共用同一工作树与分支：本 slot 两次读 ci.yml 之间文件内容变了（`f8fe32f` 落入）；`correct.rs` 有他人未提交改动悬在树上。各 slot 判定**必须带提交号**（D57 精神），否则回合汇总时会出现「同一文件两种描述」的假矛盾。本报告已全部带锚。
- 已读 D50 确认无冲突：算法 crate 进应用工作区是「Goal 1 线上一次 merge」时已完成的义务，与本 close-loop 拓扑无涉，不需要动作。

## 交付物

- 本文件。
- `DOC_PATCHES.md`（同目录）：补丁 1（DECISIONS 追加 D61）、补丁 2（PLAN_INDEX:12 跳号）、补丁 3（PLAN_INDEX:26 末句替换）、补丁 4（可选，PLAN_INDEX:25 指针）。**均未施加**，按任务要求只给精确文本。
