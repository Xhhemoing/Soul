MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 1 / fable-a — 全局规划与权威面审计报告

审计对象：`main` @ `a0ec14b`（本审计分支 `cursor/project-status-audit-c49c` 由其长出）。只读审计，未改任何权威文件。核于 2026-08-26 03:00 UTC 前后（git/gh 实查）。

---

## 一、变更摘要（自权威面上次核定 2026-08-25 以来，仓库真实发生了什么）

1. **计划面已如期落地 `main`。** PR #8 于 2026-08-25T03:29 合入（`095c1f8`），随后 `a0ec14b` 直接在 `main` 上刷新了 STATUS 的下一步。`main` 现状与 README 的自我描述一致：`docs/` 权威面 + 两个冻结算法 crate（`crates/soul-algo-tie`、`crates/soul-algo-trait`），无可安装应用。`PLAN_DOCS_FROZEN_FOR_MAIN` 成立。

2. **Goal 1 主干继续前进但未合入。** `cursor/soul-goal1-7b1c` 从 STATUS 核于的 `df5d2dd` 前进到 `6d1058b`（`df5d2dd` 仍是其祖先）。该分支自己的 STATUS（`git show origin/cursor/soul-goal1-7b1c:docs/STATUS.md`）记载：WP09 界面接线大量收尾（导入页、采集页、E1 端点、AC-13 二次确认、语气接起草、拒绝与注入进审计链等），本机 `just ci-full` 全绿；**hosted CI 因私有仓库 Billing & plans/spending limit 全部空 runner**（run 32818145279 等，五门 0 step）；作者 Win11 手动清单 76 项未勾；**PR #2 相对 `main` @ `a0ec14b` 是有意的 CONFLICTING**（冲突面是文档与 schema 锁，不是产品代码），分支侧明言不要用合 `main` 修冲突、不要合 PR #7/#10。主干尖端 `Cargo.toml` 成员表仍无两个算法 crate（实查 `git show origin/cursor/soul-goal1-7b1c:Cargo.toml`），STATUS 的 M2 阻塞在新尖端依然成立。

3. **`main` 权威面完全不知情的三层新分支栈已长出**（全部发生在 2026-08-25 17:00 UTC 前后及以后）：
   - `cursor/first-test-candidate-c441`（PR #14 → goal1）：自 goal1 的 `5309656` 分叉，自称「唯一主干 fast-forward + BUILD audit」，携带 `docs/FIRST_FORMAL_TEST.md` 测试方案；其上还有 PR #12/#13（closeout / BUILD audit）。
   - `cursor/soul-integration-4a8e`（PR #15 → first-test-candidate）：自称「exclusive parent branch」，已跑到 Round 17，PR body 引用 **D62/D63**，并声明「Exclusive → `main` BLOCKED。Do not merge PR #7 or #4」。实查 `git show origin/cursor/soul-integration-4a8e:docs/DECISIONS.md` 含 **D61–D63**——超出 `main` 上 DECISIONS.md 的 D60 截止线。
   - `cursor/beadflow-integration-c441`（PR #16 → first-test-candidate）：**同仓「兄弟产品」BeadFlow（拼豆规划）**，PR #16–#56 共 41 个 PR（21 个已合入该集成分支、20 个仍开放），自带三轮 R1–R3、工作包 WP-B01…B10、测试规模 `@bead/app` 608 / `bead-core` 155。PR body 自述「不改写 Soul、不合 `cursor/soul-goal1-7b1c`」。
   - 另有 PR #9（v0.2 行为预测调研，→ unblock 线）、PR #10（Goal 1 close-loop，→ unblock 线）、PR #11（`docs/templates/PARENT_ORCHESTRATOR.md` 编排模板，→ `main`，未合入——`main` 的 `docs/templates/` 目前仍只有 THREE_ROUND_DUAL_SCAN.md，与链接核验一致）。
   - 汇总：PR #8 之后新开 PR #9–#56 共 48 个；`main` 的 STATUS/PLAN_INDEX/README 对以上全部零登记。

4. **本审计分支今日已有同轮子代理落盘**：`13b322e`（远程缺口盘点）、`f75d7d6`（远程分支快照）、`8943889`（opus-a 对 soul-algo-tie 的 ALGO_FROZEN 复核）。属本轮编排产物，不算仓库漂移。

---

## 二、权威面一致性核验（按工单五项逐条）

### 1. STATUS.md 的冻结标记 / 分支 tip / PR 编号 vs git/gh

| 声称 | 实查 | 判定 |
|---|---|---|
| `PLAN_FROZEN` + `ALGO_FROZEN` + `PLAN_DOCS_FROZEN_FOR_MAIN`（已合入）；`BLOCKERS_FROZEN` 在 PR #6 | PR #8 MERGED（2026-08-25T03:29:45Z）；`docs/algorithms/DECISION.md` 在树；PR #6 OPEN | **一致** |
| `main` 尖端 = `7b35bde`（STATUS 第 11、17 行两处） | `origin/main` = `a0ec14b`（且 STATUS 本身就在这棵树上） | **自我过期**，见 P1-1 |
| 本分支「`cursor/polish-project-plan-5280`，从 `main` @ `7b35bde` 长出」 | 该文件已在 `main` 上；自我指涉停留在合入前视角 | 同 P1-1 |
| goal1 @ `df5d2dd`（核于 2026-08-25） | 现 `6d1058b`，`df5d2dd` 是祖先 | 核于日期合规（D57），但已落后 |
| unblock @ `c81c233`（核于 2026-08-25） | 现 `6133307`，`c81c233` 是祖先 | 同上 |
| M3 引用主干 `fc96e46` | 提交存在 | 一致 |
| PR 编号：#2 goal1、#4 dev-sota、#6 blockers、#7 unblock、#8 已合 | gh 实查逐一吻合（#2/#4/#6/#7 均 OPEN，#8 MERGED） | **一致** |
| 「未见 PR #4 关闭」（M1） | PR #4 仍 OPEN | 记录准确；但 D49 的执行悬置，见 P1-2 |

### 2. PLAN_INDEX 链接完整性

- 机械校验 `docs/**/*.md` + `README.md` 全部相对链接：**全部存在，无死链**（含 PRODUCT_LOCK、DECISIONS、algorithms/DECISION、FORMAL、STATUS、SECURITY、schemas/、GOAL2_POLISH_PROMPT、PLAN_VERIFY_PROMPT、templates/THREE_ROUND_DUAL_SCAN、COPY_ZH、REJECTED、scan-rounds/R3-SYNTHESIS）。
- 跨分支指针：`origin/cursor/soul-goal1-7b1c:docs/GOAL1_PLAN.md` 存在；`origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md` 存在。
- schema 计数声明（「十份正文 = 九份存储契约 + 一份导入契约，另有 `_defs` 与 `schemas.lock.json`」）与 `docs/schemas/` 实际 12 个文件吻合。

### 3. 权威文件间双源检查

- **判档阈值：无违规双源。** 数值只出现在 `docs/algorithms/DECISION.md` §3 常量表；`COPY_ZH.md` 第 73 行的 `=180` 引用有 D60/PLAN_INDEX 明文授权；FORMAL 矩阵里的数字全是夹具身份并有自查段。PRODUCT_LOCK/README/PLAN_INDEX 只写常量名。合规。
- **「WP01–WP11 与 WP13 已落地」三处复述**：README 第 10 行、FORMAL 第 21 行、STATUS 第 19 行。STATUS 有核于日期，README/FORMAL 没有。这是跨分支事实的三份拷贝，goal1 前进后将同步漂移（见 P2-3）。
- **STATUS 文件内部**：`main` 尖端在同一文件写了两处 `7b35bde`（第 11、17 行），与「已合 main @ `095c1f8`（PR #8）」（第 18 行）互相打架——同一文件里对同一事实两个口径（见 P1-1）。
- **决策登记簿分叉**：`main` DECISIONS.md 止于 D60 并声明「新增拍板只在本文件追加」；`origin/cursor/soul-integration-4a8e:docs/DECISIONS.md` 已有 D61–D63。叠加已知的 BLOCKERS「D32 撞号」（DECISIONS.md 脚注），合流时是两处撞号义务（见 P1-3）。

### 4. `.agent_workspace` 旧轮结论是否已被 docs 吸收

- **算法三轮（顶层 `round1–3`、`roundx`、`R*/RX-SYNTHESIS.md`）**：已吸收——`docs/algorithms/` 有编辑后的 R1/R2/R3/RX-SYNTHESIS 与 DECISION.md（`ALGO_FROZEN`）。顶层副本与 docs 版有差异（docs 为准），属过程噪音，且已被 PLAN_INDEX 第四节明文圈护为非权威。**注意**：`docs/algorithms/DECISION.md` 第 35 行把 `.agent_workspace/round3/fable-a/REPORT.md` 引为证据，该文件实存——将来清理 `.agent_workspace` 时必须保留或搬迁此文件，否则权威文件出死链。
- **plan-polish 三轮（`.agent_workspace/plan-polish/**`）**：结论已吸收为 D41–D60、schema 收紧（D58/D59）与 STATUS 更新，随 PR #8 进 `main`。synthesis 只留过程稿，可接受。
- **`context/plan/**`**：6 份权威文件旧快照（DECISIONS 无 D4x/D6x、schema 旧版）与 docs 现行版分叉 19–146 diff 行；`context/impl/**` 是 goal1 源码快照。均为**过期噪音**，已被 PLAN_INDEX 四圈护，但仍在 `main` 树上（见 P2-2）。
- **顶层 `PROGRESS.md`**：仍自称 plan-polish 为「本会话」活跃任务——任务已完结，指涉过期（见 P2-2）。

### 5. BLOCKERS.md 是否在本树

**确认不在本树**（`docs/BLOCKERS.md` 不存在；`docs/PRODUCT.md` 亦不存在，D27 合规）。远程 `origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md` 存在，PR #6 仍 OPEN。STATUS/PLAN_INDEX/FORMAL 对此的描述（「尚未在本树」「本树也没有这份文件」）全部与实况一致。

---

## 三、不完善项清单（P0–P2）

### P0

- **P0-1 `main` 权威面对仓库真实活动大面积失明。** STATUS 自称「计划线的单一事实来源」，但对 PR #9–#56（48 个）、三层新分支栈（`cursor/first-test-candidate-c441` / `cursor/soul-integration-4a8e` / `cursor/beadflow-integration-c441`）、goal1 的 hosted CI billing 阻塞与 PR #2 有意冲突策略零登记。只读 `main` 的编排者会按 STATUS「下一步」直接合 PR #6、推 PR #7 回主干，而集成线正在喊「Do not merge PR #7 or #4」。两套「下一步」在打架，`main` 侧不知道有另一套。
  证据：`docs/STATUS.md`（核于 2026-08-25）vs `gh pr list --state all`；PR #15 body（Round 17、BLOCKED 声明）；`git show origin/cursor/soul-goal1-7b1c:docs/STATUS.md`（PR #2 CONFLICTING 段）。
- **P0-2 兄弟产品 BeadFlow 无任何 `main` 权威承认与共存规矩。** PRODUCT_LOCK 自称「唯一产品权威」、PLAN_INDEX 自称 30 秒定位仓库权威，但两者对同仓已存在 41 个 PR、两位数工作包、763 项测试的第二个产品只字未提；仓库拓扑三行（PLAN_INDEX 第三节）也没有它。现状下「唯一产品权威」这句话在仓库层面已经名不副实——要么权威面登记兄弟产品与边界（不进 `main`？何时进？CI 如何隔离？），要么明文把它逐出本仓库。这是 D27/D41 防双源纪律在「仓库级」的空洞。
  证据：PR #16 body（「在 Soul 旁边开 BeadFlow 专属线」「不合 cursor/soul-goal1-7b1c」）；PR #17–#56；`docs/PRODUCT_LOCK.md` 第 4 行；`docs/PLAN_INDEX.md` 第三节。

### P1

- **P1-1 STATUS 在 `main` 上的自我指涉过期。** 第 11 行仍自称「本分支 `cursor/polish-project-plan-5280`」；第 11、17 行写 `main` 尖端 = `7b35bde`，而该文件所在提交 `a0ec14b` 本身就是 `main` 尖端。同一文件第 18 行又说「已合 `main` @ `095c1f8`」。新读者无法从 STATUS 判断自己站在哪棵树上。
  证据：`docs/STATUS.md` 第 11/17/18 行；`git log --oneline -3 origin/main`。
- **P1-2 D49 拍板悬置未执行。** D49 明令「停 `agent/dev-sota`、关 PR #4」，STATUS M1 也把它列为合入阻碍；PR #4 至今 OPEN，分支健在。拍板与现实的缺口没有 owner。
  证据：`docs/DECISIONS.md` D49；`gh pr view 4`（OPEN）；`origin/agent/dev-sota` @ `24c539f`。
- **P1-3 决策登记簿已在分支侧分叉（D61–D63），撞号义务堆积。** `main` 止于 D60，soul-integration 线已用 D61–D63；另有 PR #6 合入时 BLOCKERS 正文「D32」需改写的既有义务（DECISIONS.md 脚注）。收编顺序不定、越晚合并撞号面越大。
  证据：`git show origin/cursor/soul-integration-4a8e:docs/DECISIONS.md | rg 'D6[0-9]'` → D60/D61/D62/D63；`docs/DECISIONS.md` 第 73 行脚注。
- **P1-4 Goal 1 关闭门的真实卡点（外部阻塞）在 `main` 侧不可见。** hosted CI 空 runner 的根因是私库 Billing & plans/spending limit（作者线下动作），作者 Win11 清单 76 项未勾——这两件事只活在 goal1 分支 STATUS；`main` 的 STATUS「阻塞」表与未合入的 BLOCKERS 都没有登记。按 D54，这是关闭门的硬前置。
  证据：`git show origin/cursor/soul-goal1-7b1c:docs/STATUS.md`（run 32818145279 五门 0 step、Billing 注解、76 项）；`docs/STATUS.md` 阻塞表。
- **P1-5 PR #2 与 `main` 的有意冲突及其解法只存在于分支侧。** goal1 STATUS 明言冲突面是「文档与 schema 锁」且禁止用合 `main` 修冲突；`main` 侧 M2 仍写着旧的吸收路径（unblock @ `c81c233` 待回主干），对「合同树冲突」这一新事实无字。两侧对同一合并动作的操作指引不同源。
  证据：goal1 STATUS「PR #2 相对 origin/main（a0ec14b）是 CONFLICTING / DIRTY」段；`docs/STATUS.md` M2 行；PR #14 body（Not merged: main — intentional contract-tree conflict）。

### P2

- **P2-1 过期开放 PR 未清场。** PR #1（计划种子，已被 PR #8 实质取代）、PR #3（Round 1 旧现状盘点）仍 OPEN 对 `main`；PR #11（编排模板）待裁决。挂着的旧 PR 会误导「哪份文档在途」。
  证据：`gh pr view 1 / 3 / 11`（均 OPEN）。
- **P2-2 `.agent_workspace` 过程噪音仍在 `main` 树上（510 个 tracked 文件）。** `context/plan/**` 6 份旧权威快照与 docs 分叉 19–146 diff 行、`context/impl/**` 源码快照、顶层 `PROGRESS.md` 自称已完结任务为「本会话」。均已被 PLAN_INDEX 四圈护为非权威，属可清理项；清理时必须保留 `docs/algorithms/DECISION.md` 第 35 行引用的 `.agent_workspace/round3/fable-a/REPORT.md`。
  证据：`git ls-files .agent_workspace | wc -l` = 510；diff 实测；`.agent_workspace/PROGRESS.md`。
- **P2-3 跨分支事实的无日期复述。** README 第 10 行与 FORMAL 第 21/25 行写「WP01–WP11 与 WP13 已落地」不带核于日期与提交号（STATUS 侧带）；goal1 已再前进（WP09 又长了六七段），三处将不同步。D57 纪律目前只约束 STATUS。
  证据：`README.md` 第 10 行、`docs/FORMAL_WORK_PROMPT.md` 第 21 行 vs goal1 分支 STATUS。
- **P2-4 核于提交号落后一格（合规但待刷新）。** STATUS/SECURITY 里 goal1 = `df5d2dd`、unblock = `c81c233`，现分别为 `6d1058b`、`6133307`（均为快进）。下次动 STATUS 时一并刷新。
  证据：`git rev-parse origin/...` 实查 + `merge-base --is-ancestor` 验证。

---

## 四、给 Round 2 的问题清单

1. **BeadFlow 的仓库级地位由谁拍板、落哪份文件？** 选项：PLAN_INDEX 拓扑加第四行 + 追加 D 号明确「同仓兄弟产品，权威面在其集成分支，不进 Soul 的 `main` 权威文件」；或迁出仓库。现状是治理真空（P0-2）。
2. **「唯一实现主干」（D49）现在指谁？** goal1 @ `6d1058b` 之上叠了 first-test-candidate（自称唯一主干 FF）与 soul-integration（自称 exclusive parent、Round 17、BLOCKED→main）。三者的合流顺序与谁有权宣布「后继主干」（FORMAL 开工第一动作第 4 步的措辞）需要裁决，否则 D49 正在被温水违反。
3. **PR #6（BLOCKERS）现在合不合？** STATUS 下一步第 1 条自 2026-08-25 起悬置；合入时须执行 D32 撞号改写。若 Round 2 允许写权威文件，这是最高杠杆的一步（把合入/关闭阻碍权威接进本树）。
4. **D61–D63 收编规则**：沿用 DECISIONS.md 脚注的「合入时改写为下一个空闲 ID / 加括注」方案，还是要求 soul-integration 线先自行重编号？需要一条明文，防第三处再开新号。
5. **STATUS 刷新是否入下轮工单？** 内容至少含：改成 `main` 视角的自我指涉、更新 main/goal1/unblock 尖端、登记三条新集成线与 BeadFlow 的存在及其「不进本树」边界、登记 hosted CI billing 外部阻塞。本轮为只读，未动。
6. **过期 PR 清场（#1、#3，以及按 D49 关 #4）** 由父代理直接执行还是留作者动作？gh 在子代理侧只读。
7. **hosted CI billing 是作者唯一能解的外部动作**——是否需要父代理在给作者的汇总里单列（连同 Win11 手动清单 76 项），避免继续被当成「CI 还没跑」的普通待办？
