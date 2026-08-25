# ARCHITECTURE — 规范文档树提案

MODEL_SLUG: claude-fable-5-thinking-xhigh

目标：合入 main 后，新父代理只读 `README.md` 一屏即可定位全部权威；任何事实恰有一个权威家；
`.agent_workspace/` 永远是过程档案。分类：**[权威]** 唯一事实来源，改动须留痕；
**[过程]** 历史记录/模板/提示词，只增不当权威；**[删]** 本打磨应删除；**[注]** 保留但打标签。

## 1. 目标树（main 合入后）

```text
README.md                       [权威·索引] 30 秒导航页（重写，见 §2）
docs/
  PRODUCT_LOCK.md               [权威] 产品唯一权威（加一行算法权威指针，其余不动）
  DECISIONS.md                  [权威] 拍板账本 D1–D31 + 本打磨 D41 起（D32–D40 留给 unblock 线汇入）
  FORMAL_WORK_PROMPT.md         [权威] 开工提示词 + 验收矩阵（补算法 AC 行 + 现状前言）
  SECURITY.md                   [权威] 安全规范（本打磨不动；Goal 1 的落地两节随 PR #2 汇入）
  STATUS.md                     [权威] main 视角状态页（重写为三态 + 指针，见 §3）
  BLOCKERS.md                   [权威·冻结] 走 PR #6 自己的通道进 main；本打磨只建指针
  schemas/                      [权威] 11 份契约（逐字节采纳 Goal 1 版；schemas.lock.json 留 Goal 1 线）
  algorithms/
    DECISION.md                 [权威·冻结] ALGO_FROZEN；不改正文
    COPY_ZH.md                  [权威·冻结] 用户可见话术；改动先改此处
    REJECTED.md                 [权威·冻结] 墓碑
    README.md                   [权威·索引] 加 §6.1 命名勘误注（加性两行）
    R1/R2/R3/RX-SYNTHESIS.md    [过程·索引] 保持指向 .agent_workspace 全文
  GOAL2_POLISH_PROMPT.md        [过程] Goal 1 关闭后的触发器，保留
  PLAN_VERIFY_PROMPT.md         [过程] 首部加「已执行完毕（R1–R3 见 scan-rounds/），仅存档」
  scan-rounds/                  [过程] 计划三轮记录，保留
  templates/                    [过程] 可复用模板，保留
.agent_workspace/               [过程] 永非权威（README 拓扑图写明这句话）
  PROGRESS.md                   [过程] 任务枢纽（状态职责移交 docs/STATUS.md 后降为任务索引）
  R*/round*/roundx/**           [过程] 算法轮次证据链，勿动
  context/
    README.md                   [注·新增] 两行：plan/ 已删（权威在 /docs）；impl/ 是 Goal 1 @ ac3d9b3 历史快照
    impl/*.rs                   [注] ALGO_FROZEN 证据输入，保留 + 依赖上行标签
    plan/                       [删] 六份副本全删（PRODUCT_LOCK/DECISIONS/FORMAL + 3 schema）
  plan-polish/**                [过程] 本任务工作区
仅住 Goal 1 线（经 PR #2/#7 汇入，main 不先收）：
  docs/GOAL1_PLAN.md            实现规划
  docs/schemas/schemas.lock.json 哈希锁（谁改 schema 谁重钉）
  docs/STATUS.md 的实现纪事      Goal 1 视角
  DECISIONS D32–D40             实现期拍板
  SECURITY 加密落地/DPAPI 两节   实现实证
```

## 2. README.md 重写要点（30 秒测试）

顺序即优先级；每行一句话说明「这份管什么」。必含：
1. 一句话（**引用** PRODUCT_LOCK，不再自由改写）。
2. 权威四链接：PRODUCT_LOCK（产品）→ DECISIONS（拍板）→ algorithms/DECISION（算法，ALGO_FROZEN，
   T4D + A0）→ FORMAL_WORK_PROMPT（开工与验收）。
3. 状态一链接：STATUS（含 Goal 1 分支指针）；阻塞一链接：BLOCKERS（PR #6 合入后）。
4. 契约：schemas/（注明 lock 住 Goal 1 线）。
5. 分支拓扑三行：main = 计划 + 算法冻结；`cursor/soul-goal1-7b1c` = 实现主干（PR #2）；
   `cursor/goal1-unblock-a073` = T4D 吸收在途（PR #7）。
6. 一句纪律：「`.agent_workspace/` 是过程档案，永非权威。」
删除现 README 第 5 行「当前处于产品锁定阶段，还没有可运行的应用」（双重过期）。

## 3. STATUS.md 分工（消灭三分叉）

- **main 的 STATUS（本打磨重写）**：只写四件事——① 三个冻结态（PLAN_FROZEN / ALGO_FROZEN /
  BLOCKERS_FROZEN）各一行 + 权威链接；② 仓库拓扑（哪条线是什么，PR 号）；③ 「v0.1 实现：
  已在 `cursor/soul-goal1-7b1c` 基本落地，未合入 main，未关闭；细节见该分支 `docs/STATUS.md`」；
  ④ 下一步 = BLOCKERS 推荐工序的指针。**不复述** WP 完成度，不复述 G/M/S 条目。
- **Goal 1 的 STATUS**：继续做实现纪事（该线已有的诚实纪律保持）。
- 首行「单一事实来源」改为「main 视角的单一事实来源；实现纪事见 Goal 1 分支」。

## 4. M2 合并手册（本打磨必须随交付附上，防 F10 隐雷）

本分支合 main 后，执行 BLOCKERS M2（`git merge origin/main` 进 Goal 1）时逐文件规则：

| 文件 | 冲突预期 | 取法 |
|---|---|---|
| PRODUCT_LOCK / FORMAL / GOAL2 / PLAN_VERIFY / scan-rounds / templates | 若 polish 未改则字节同 → 自动清算；polish 改过的（LOCK 指针行、FORMAL AC 行）→ add/add | **取 main（polish）侧**；Goal 1 线从未独立改过这些文件 |
| DECISIONS.md | 冲突（D32–D40 vs D41+） | **并集**：D1–D31 共同体 + Goal 1 侧 D32–D40 + main 侧 D41+，按号排序 |
| SECURITY.md | 冲突（Goal 1 多两节） | **取 Goal 1 侧**（polish 不改此文件，故理论上 main 侧无新增；若有，逐段并入） |
| STATUS.md | 冲突 | **按 §3 分工手工合**：里程碑/拓扑段取 main 侧措辞，实现纪事全取 Goal 1 侧 |
| schemas/*.json | polish 已逐字节采纳 Goal 1 版 → 自动清算 | 若 polish 另改了正文（不建议），Goal 1 侧必须同步重钉 schemas.lock.json |
| schemas.lock.json / GOAL1_PLAN.md | 仅 Goal 1 有 → 无冲突 | 保留 |
| README.md | 冲突（Goal 1 线 README 是否被改过需在 M2 时核；本审计未见其独立演化） | **取 main（polish）侧**，Goal 1 特有链接（如 GOAL1_PLAN）并入 |

## 5. 明确不做

- 不为消双源改任何冻结文件正文（DECISION.md、COPY_ZH.md、REJECTED.md、BLOCKERS.md）。
- 不把 BLOCKERS 的 G/M/S 条目抄进 STATUS 或 FORMAL——指针替代复述。
- 不建 `PLAN.md` / `PRODUCT.md` / `INDEX.md` 之类新顶层文件；README 就是索引（D26 精神：文档只减不增）。
- 不动 `.agent_workspace/round*` 与 `roundx` 证据链。
