# SOTA_BAR — 「计划完成」对新父代理意味着什么

MODEL_SLUG: claude-fable-5-thinking-xhigh

定义：一个从未见过本仓库的父代理，拿到 main（合入本打磨之后），**不翻任何分支历史、
不读 .agent_workspace、不问人**，30 秒内知道读什么，30 分钟内能做出与三轮扫描 + 算法
三轮消融 + 阻碍分析相同的行动决策——不重开产品方向、不重选算法、不重开 Goal 1、
不把过程稿当权威。达不到任何一条，计划不算完成。

## 冻结门（全部可由第三方跑命令核验）

### 门 1 — 30 秒定位（P6）
- [ ] README 一屏内含：产品权威、拍板账本、算法权威（ALGO_FROZEN，T4D + A0）、
      开工/验收权威、状态页、阻塞单、schema 目录、分支拓扑三行、
      「.agent_workspace 永非权威」一句。
- 核验：读 README，对每个权威问「链接在吗、一句话说明在吗」。八项缺一即红。

### 门 2 — 单源（P1）
- [ ] `.agent_workspace/context/plan/` 不存在（或仅存指针 README）。
- [ ] 除 git 历史外，PRODUCT_LOCK / DECISIONS / FORMAL 全文在工作树恰好一份。
- [ ] main 的 docs/schemas 与 Goal 1 线逐字节相同（或差异已随 lock 重钉声明）。
- 核验：`find . -name "PRODUCT_LOCK*" -not -path "./.git/*"` 恰一条；
  逐份 `diff docs/schemas/<f> <(git show origin/cursor/soul-goal1-7b1c:docs/schemas/<f>)` 为空。

### 门 3 — 冻结吸收（P3）
- [ ] PRODUCT_LOCK 拍板表有算法冻结行（指向 docs/algorithms/DECISION.md）。
- [ ] DECISIONS 有一条 D41+ 记录计划吸收算法冻结。
- [ ] FORMAL「先读」含 algorithms/DECISION.md；验收矩阵含算法门禁行
      （锚夹具、180/360 闭区间、as_of 全库单值、A0 不绕锁、A2 无第二套阈值）。
- [ ] tie_strength 字段义务成文（计划义务 + Goal 1 线的 schema/lock 执行路径写明）。
- 核验：`grep -l "algorithms/DECISION" docs/PRODUCT_LOCK.md docs/FORMAL_WORK_PROMPT.md README.md`
  三命中；矩阵内 `grep -c "lilei_12\|180\|360\|as_of"` 非零。

### 门 4 — 现状诚实（P4）
- [ ] STATUS 三态分明：PLAN_FROZEN ≠ Goal 1 关闭 ≠ main 已有应用；
      明写「实现已在 `cursor/soul-goal1-7b1c` 基本落地、未合入、未关闭」，并指向该线 STATUS
      与 BLOCKERS 工序。
- [ ] 全 main 树 `grep -rn "未开始\|尚未写应用代码" docs/ README.md` 零命中
      （或命中处均为历史存档且有存档声明）。
- [ ] FORMAL 的「开工第一动作」不再指示 CreateGoal：Goal 1（改为存在性检查 + 收尾工序）。
- 核验：上述 grep + 人读 STATUS 首屏。

### 门 5 — 不膨胀（P5）
- [ ] 13 条切片逐字不变（除人脉图行按 P0-4 加的半句结构指针）；砍/留表不变；
      WP 仍是 12 个（WP12 保持删除）；AC-27 仍标 v0.1.1；无 OAuth/云端/发送/写执行回潜。
- 核验：对 PRODUCT_LOCK 与 FORMAL 跑 `git diff 7b1c 种子..HEAD`，改动应仅为指针/验收行/
  现状前言三类；出现新能力词（发送、执行写、OAuth、云端 HTTP）即红。

### 门 6 — 可测（P2）
- [ ] 新增的每一条算法 AC 行有 Given/When/Then 与「谁跑」，且引用的是冻结夹具名
      （DECISION §1/§3 与 BLOCKERS G1 清单里已存在的），不发明新阈值、不加 F04c 第三道门。
- 核验：逐行比对 DECISION.md 夹具清单；出现 DECISION/COPY_ZH 之外的新数字即红。

### 门 7 — 合并可执行（本仓库特有）
- [ ] M2 合并手册存在（逐文件取法：DECISIONS 并集、SECURITY 取 Goal 1、STATUS 按分工、
      schemas 字节同自动清算），且 DECISIONS 新编号从 D41 起避让 D32–D40。
- 核验：读手册 + `git merge-tree $(git merge-base origin/main origin/cursor/soul-goal1-7b1c) origin/main origin/cursor/soul-goal1-7b1c` 输出的每个冲突文件在手册里有一行。

## 反目标（做了任何一条即倒退）

1. 为「文档完整」新建 PLAN.md / PRODUCT.md / OVERVIEW.md —— D26/D27 违规。
2. 把 BLOCKERS 的 G/M/S 或 Goal 1 STATUS 的 WP 纪事抄进 main 文档 —— 制造新双源。
3. 改冻结文件正文（DECISION/COPY_ZH/REJECTED/BLOCKERS）以求措辞统一。
4. 在 main 上宣称加密落地/DPAPI/CI 绿 —— 那些证据住在还没合入的实现线上。
5. 为算法 AC 行发明 DECISION 常量表之外的数字，或给 F04c 加第三道降档门。
6. 删除 `.agent_workspace/round*` / `context/impl` 证据链（冻结依据会断链）。

## 完成宣言的形状

计划完成时，STATUS 里应当能诚实写下这一句且七门全绿：
「计划权威面已单源化并吸收 ALGO_FROZEN；v0.1 的产品、算法、验收、阻塞各有唯一权威文件；
实现在 `cursor/soul-goal1-7b1c` 收尾，按 `docs/BLOCKERS.md` 工序合入与关闭。」
