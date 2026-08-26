MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 2 · opus-b — FORMAL / PLAN_INDEX / STATUS 遗留项 L2

绑定读物：`.agent_workspace/plan-polish/R1-SYNTHESIS.md`（L2：FORMAL「接下来使用 Goal」历史段仍诱导重派 planner）。
工作树：`cursor/polish-project-plan-5280`，Round 1 提交 `b380f67` 之上。**未 commit、未 push、未碰 `crates/`。**

## 一、改了什么（四份文件，全部在派发白名单内）

| 文件 | 改动 | 目的 |
|---|---|---|
| `docs/FORMAL_WORK_PROMPT.md` | 5 处 | L2 主体 + 红线 11 自查 |
| `docs/PLAN_INDEX.md` | 6 处 | 三项事实核对（D1–D58 / 锁文件 / BLOCKERS 仍在 PR #6）+ 补两条查询入口 |
| `docs/STATUS.md` | 3 处 | 本分支尖端注记按本轮编辑推进，守 D57 |
| `docs/PLAN_VERIFY_PROMPT.md` | 1 行横幅 + 1 行尾注 | 不被当成新工单 |

本槽位足迹（`git diff --numstat`）：`FORMAL_WORK_PROMPT.md` 21/6、`PLAN_INDEX.md` 8/4、`PLAN_VERIFY_PROMPT.md` 4/0、`STATUS.md` 4/2，合计 37 insertions / 12 deletions。

### 1. FORMAL — 历史段标注（L2 闭合）

- 原来的 `### 接下来使用 Goal：先调用子代理 claude-fable-5-thinking-xhigh……` 是一级小节标题，位置在「云端子代理模型约定」之后、工作包清单之前，读起来就是当前工单。现在改为：新增小节 `### 历史段（作者开工原话，已执行过一次，不要照着再跑一遍）`，其下一段引用块写明「存档不是指令」，**作者原话与 1–6 步原样留在下面**，只把那句原话从标题行降为正文行。
- **原话未被删改**：脚本核对 `.agent_workspace/context/plan/FORMAL_WORK_PROMPT.md`（PR #1 前的同源稿）与本树，那句 `接下来使用 Goal……` 逐字节相同（sha256 前缀 `98f20d90`），第 1–5 步逐字节相同。第 6 步的差异来自 Round 1 的 D54 双门改写，不是本轮所为。
- 引用块顺手交代了原话里**还有效**的两点去向，免得读者以为整段作废后丢了约束：第 1 条 `PLAN_FROZEN` 前置 → 「开工第一动作」第 1 步；第 6 条双门关闭 → D54 与「计划面与代码面」第 5 条。第 2–5 条作废（重开 Goal 1 / 重派 planner 属于 D49 违规）。
- 历史段结尾加一行「（历史段到此为止。以下各节仍然有效。）」，并给紧随其后的工作包清单补了 `### 工作包（仍然有效，只减不增）` 标题——否则工作包会落进历史段的辖域里，被一起当成作废内容。**工作包条目一条没动，WP12 保持删除，无新增 WP。**
- 「开工第一动作」升为唯一操作路径：标题改为 `### 开工第一动作（**本文件唯一可执行的开工路径**）`，下加一句「本节优先于本文件任何其他段落，尤其优先于上面的『历史段』」。
- 整份粘贴的读者在第 14 行那节就要看到结论，所以「计划面与代码面」列表补第 6 条，把「只有开工第一动作可执行、历史段是存档」提到前面；正文开头的必读清单把 `docs/PLAN_INDEX.md` 排到第一位（对齐 R1 的 SOTA 目标 5）。
- 「开工第一动作」第 2 步的 BLOCKERS 取法补上「仍在 PR #6」。

### 2. FORMAL — 红线 11 数字自查（3/10/180/360）

扫描结论：**FORMAL 目前没有第二份常量表**。`180`、`360` 全文不出现；不存在「次数 ≥ N」「自然日 ≥ N」这类阈值复述（`rg '180|360|次数 ?≥|自然日 ?≥'` 在四份文件上均无命中）。文中的数字全是夹具身份与用例规模：

| 出处 | 数字 | 性质 |
|---|---|---|
| AC-08 | ≥3 个对话对象、节点≥3 | 用例规模 |
| AC-09 | 切应用 10 次 | 采集用例规模，与判档无关 |
| AC-14 | 3 条记忆 | 用例规模 |
| AC-28 / 夹具说明段 | 群聊互惠 30 次 / 10 天 + 每方向各 1 次 | `group_heavy_plus_one_direct_each_way` 的身份 |
| AC-29 | 一对一互惠 12 次 / 6 天 / 3 天前收尾 | `lilei_12` 的身份 |
| 11.3 | 不超过 10 行 | 父代理直改白名单，非算法域 |

两处动作：

1. 唯一能在不丢测试身份的前提下去掉的裸数字是 AC-29 的「（一对一 3 次）」——夹具名 `group_heavy_plus_three_directs` 里已经含着这个数，改写为「（夹具名中的 `three_directs` 即其一对一条数）」不丢任何识别信息，同时不再长得像 `MODERATE_MIN_INTERACTIONS` 的复述。其余数字按派发指示保留（去掉就认不出是哪份夹具）。
2. 新增一段「**关于红线 11 的自查**」，声明矩阵里的数字一律是夹具身份、本文件不写任何阈值取值，并给出该引用的常量名清单（`MODERATE_MIN_INTERACTIONS` / `STRONG_MIN_INTERACTIONS` / `STRONG_MIN_ACTIVE_DAYS` / `DEMOTE_ONE_BAND_DAYS` / `FORCE_WEAK_DAYS`，只写名不写值）。这样下一个审计者不必每次重跑这张判定表，改夹具规模的人也不会顺手写成「因为门槛是 N」。

### 3. PLAN_INDEX — 三项事实核对与两条入口

核对结果：**三项都已成立**，只补精度。

- D1–D58：`docs/DECISIONS.md` 实际收到 D58（D41–D58 为 Round 1 新增），索引第一节表述正确，未动。
- 锁文件：`docs/schemas/schemas.lock.json` 在本树（1319 字节），索引已提及；补「锁文件已在本树」，与 BLOCKERS 那种「不在本树」的行区分开。
- BLOCKERS：仍在 PR #6。原来只写了分支路径，现补「**仍在 PR #6，尚未合入 `main`，本树也没有这份文件**；本计划 PR 不整份拷贝它」，并说明合入顺序（本 PR 先、PR #6 后），与 `STATUS.md`「下一步」第 2 条一致。这条同时把 R1 的 L5 说清楚。

新增两条查询入口，都是本轮 L2 的正面拦截：

- 「接手时该不该重开 Goal 1 / 重派 planner」→ 不该，指向「开工第一动作」与 D49。
- 「判档阈值到底是多少」→ 只有常量表与 `soul-algo-tie` 常量模块两处，其余文档写数字即缺陷。

另外：第四节「不是权威」补两条（`PLAN_VERIFY_PROMPT.md` 已执行完毕、FORMAL 的历史段是存档）；第五节「验收矩阵」改法一栏补上「只准出现夹具身份的数字，不准复述判档阈值」。第一节第 4 行给 FORMAL 加了「开工路径只认末尾的开工第一动作」。

### 4. STATUS — 尖端注记（守 D57）

- 各条线表的「本分支（计划面）」行：尖端写成「本 PR（#8），Round 1 落在 `b380f67`，其上叠 Round 2 打磨提交」，内容列补 `PLAN_INDEX.md` 与 `schemas.lock.json`。
- 进度表拆两行：「Round 2 · 引导面」列出本轮已落地的四件事；「Round 2 · 其余」写明 `tie_strength`（D58）与 SECURITY 指针**由本轮另一路处理、状态不由本行代答**——本槽位不替别的槽位报成绩，那一路无论落没落，这行都不会说假话。
- 「下一步」第 1 条注明引导面已处理，剩 schema 与 SECURITY 两处。

D57 自查：新增文字全部是本树可验证的事实（本树文件的内容、`b380f67` 可由 `git log` 核）；没有新增跨分支断言，没有动「阻塞：无只覆盖本树」的措辞，没有把计划冻结写成 Goal 1 关闭或 `main` 已有应用。

### 5. PLAN_VERIFY_PROMPT — 历史横幅

标题下一行横幅：已执行完毕、结论 `PLAN_FROZEN`、记录在 `docs/scan-rounds/`（三份文件确认在树上），并把读者引到 FORMAL / STATUS / PLAN_INDEX。文件末尾那个同名的「开工第一动作」（内容是「立刻并行派第 1 轮两个 Task」）另加一行括注，说明它指的是当年那三轮扫描、与 FORMAL 的同名小节不是一回事——两份文件小节重名是最容易踩的坑，只加横幅挡不住从中间跳读的人。

## 二、遵守的约束

- 中文；无新 WP；AC-18 原样保留（含 D31 那句「不要因为看着像就并掉」）；AC-27 仍是 v0.1.1，仍不占矩阵表行。
- 未 `git commit` / `git push`；未改 `crates/`；未改 `docs/schemas/`（L1 是别的槽位）；未改 `SECURITY.md`（L3 同理）；未改 `README.md`（不在白名单，虽然它仍链着 `PLAN_VERIFY_PROMPT.md`——见遗留项）。
- 未改任何冻结结论：T4D / A0 / A2 / A1、常量、as_of 纪律、回退链、F04c 禁第三道门，一字未动。

## 三、验证

```
$ git diff --numstat docs/          # 本槽位落笔时
21      6       docs/FORMAL_WORK_PROMPT.md
8       4       docs/PLAN_INDEX.md
4       0       docs/PLAN_VERIFY_PROMPT.md
4       2       docs/STATUS.md

$ rg -n '180|360|次数 ?≥|自然日 ?≥' docs/FORMAL_WORK_PROMPT.md docs/PLAN_INDEX.md docs/STATUS.md docs/PLAN_VERIFY_PROMPT.md
（无命中，exit=1）

作者原话逐字节核对：True（sha256 前缀 98f20d90）；历史段第 1–5 步逐字节相同。
```

**同树并发**：收尾时工作树里另外还有 `docs/SECURITY.md`（18/0）与 `docs/schemas/relationship.schema.json`、`schemas.lock.json` 的未提交改动，那是本轮另一个槽位（L1 / L3）在同一棵树上落笔，**不是本槽位所为**。父代理合并时按上表四份文件认本槽位的边界。

未跑 `cargo test`：本轮零代码改动，`crates/` 未触碰。

## 四、留给 Round 3 的

1. `README.md` 第 26 行仍把 `docs/PLAN_VERIFY_PROMPT.md` 与其他权威文件平铺在同一个列表里。本轮无权改 README，建议 Round 3 在那一行后面补「（历史存档）」，否则索引与 README 对同一份文件的定性不一致。
2. 若要把「不得复述阈值」变成可跑门禁，可加一个负向探针：扫 `docs/**` 与 `crates/**`（`soul-algo-tie` 常量模块除外）里出现 `180` / `360` 的行。夹具数字（30/10/12/6/3）会误报，需按夹具名白名单排除——本轮只写了文字纪律，没写探针。
3. FORMAL 的历史段与 `.agent_workspace/context/plan/FORMAL_WORK_PROMPT.md` 现在已经分叉（那份是 PR #1 前的旧稿，无历史注记）。R1 的 L4 说过程目录副本要加「不得当权威」断言，本轮没动那边；两处措辞谁对谁错，按 D41 一律以 `docs/` 为准。
