MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 3 / fable-a — 工作报告（终局 SOTA 验收）

核于 2026-08-26 03:30–04:00 UTC。本轮职责：综合 round1 + round2 全部 12 份报告，产出终局验收判定书 `SOTA_ACCEPT.md`（同目录）。本文件记录方法、本轮新锚定的事实、对前轮分歧的裁断依据与未做声明。

---

## 1. 输入与方法

**输入**：`.agent_workspace/orch-20260826/round{1,2}/{fable-a,fable-b,gpt-sol-a,gpt-sol-b,opus-a,opus-b}/REPORT.md` 全文 12 份，全部逐字读毕。round2/opus-b 附带探针 `probe/verify_r2_opus_b.{py,out}` 已确认在树。

**方法**：验收判定不转抄——凡是「四门判定」直接依赖的树内事实，本轮在验收树上重新实锚（见下节）；跨报告结论按「round2 修正优先于 round1、实测优先于 git show 实读、实读优先于自述」的次序采信；四门的 PASS/FAIL 与管辖范围、终表的 P0/P1/P2 分级与修复归属为本轮独立判断。

## 2. 本轮新锚定的事实（区别于转抄）

1. 验收树 = `cursor/project-status-audit-c49c` @ `78febb0`；实测 `git diff origin/main HEAD -- docs crates` 为空，`origin/main` 仍 `a0ec14b`（fetch 后确认无前移）。四门判定的「main」即此树。
2. `cargo test --workspace` 本轮复跑：**249 passed / 0 failed / 0 ignored**。这是继 R1 gpt-sol-a、R2 gpt-sol-a 之后第三次独立全绿，算法冻结门的测试基线三轮闭合。
3. `gh pr view {2,4,6,7,15}` 本轮复核：#2 OPEN CONFLICTING/DIRTY、#4 OPEN CONFLICTING/DIRTY、#6 OPEN MERGEABLE/CLEAN、#7 OPEN MERGEABLE/CLEAN、#15 OPEN MERGEABLE/CLEAN。与 round2 各报告记录一致，round2 以来无状态漂移——终表引用的 PR 事实不需要修订。
4. 工作区状态：`git status --porcelain` 仅本轮 round3 未跟踪目录；未离开分支、未提交、未推送。

## 3. 对前轮分歧的裁断记录（终表采用口径的出处）

| 分歧 | 终局口径 | 理由 |
|---|---|---|
| R1 fable-a「PLAN_INDEX 无死链」vs R1 gpt-sol-b「6 个死链」 | 两者并立：显式权威链接全部可解析；scan-rounds 三份 SYNTHESIS 的裸 `bc-…` run-id 是 6 个真实死链 | 校验口径不同（显式相对链接 vs 一切可被解析为相对路径的目标）；死链入 P2-2 |
| R1 opus-a「加一行 `SystemTime::now()` 不会有测试变红」vs R2 opus-a 反证 | 采 R2：粗暴替换会红 28 项（副作用非护栏）；但 DECISION §6.3 点名禁止的缺省墙钟入口形态可全绿 + clippy 干净加入 | R2 有两组实验实证；缺口的准确形状是「新增读钟路径无护栏」，入 P1-3 |
| R1 opus-b「schemas.lock 无校验器，中」vs R2 opus-b「校验器在 Goal 1 线且挂 CI 与写库路径，降低」 | 采 R2：lock 本身降为低（不单列终表，并入背景）；但换来的升级是 D32 schema 缺口从「中」升「高」（schema 是落库路径上今天就在跑的门） | R2 有链路逐段实查（justfile→xtask→put_relationship）与 3×3 全真值表 |
| R1 opus-a「强联系靠字形差异躲过筛子」vs R2 opus-a 实测 | 采 R2：42 条解释全部通过最严筛，今天无安全问题；结构性问题（tie 从未调用筛子）是真的 | 实测 42/42；定性入 P1-2② 的裁决面 |
| R1 fable-b「合并路径没有 owner、无文档写明」vs R2 fable-b | 采 R2 升级版：D61 与「PR #10→#7→main」已成文但只在 PR #10 链上，三权威面互斥 + 主干 live drift | 入 P0-2；「无 owner」保留给拓扑归一工序本身（P0-1） |
| R1 fable-a「PRODUCT_LOCK『唯一产品权威』名不副实」vs R2 fable-a 修正 | 采 R2：不是 PRODUCT_LOCK 被违反（它防的是 Soul 自建双源，BeadFlow 未建第二份 PRODUCT.md——R2 gpt-sol-b 树级核实），是仓库级产品登记无人在管 | 入 P0-6 的定性 |
| R1 对三层栈的一揽子「温水违反」vs R2 fable-a 分层 | 采 R2 分层：first-test-candidate 基本不构成违反；soul-integration 是主体（四件叠加，其中无 D 号改锁最重）；beadflow 不构成但有搭车耦合 | 入 P0-5、P0-6、P1-5 |
| R2 opus-b 新发现（R1 全部未见） | D02 反向误拒、E01–E03、`is_locked_by_user()` 静默吞纠正、fixture 零覆盖、两个同名 `TieScore`、证据 id 上游缺席 | 全部纳入 P1-4 / P1-8 |

## 4. 四门判定的形成逻辑（摘要，全文见 SOTA_ACCEPT.md）

- **计划冻结 PASS**：冻结对象（计划判断、工序框架、schema 锁、双源纪律）在 main 树内全部核验成立且三轮无反例；STATUS 登记失明与支线漂移是治理/时点问题，不在冻结标记的语义范围内——但必须以「管辖范围」明写边界并转为 P0 义务，否则 PASS 会被误读为对登记现状的背书。
- **算法冻结 PASS**：判档层四路独立核验（opus-a 逐条对表、opus-b 义务矩阵、fable-b 代价覆盖、gpt-sol-a 基线扫描）零判档偏差；R2 opus-a 对「不阻塞冻结」的三层论证（不动 band / DECISION §0 预归类 / D52 已定价）成立。话术单源、常量物理单源、墙钟护栏三缺口如实排除在 PASS 射程外。
- **应用可装 FAIL**：三段缝（本机→hosted→真机）无一在当前尖端闭合，且唯一历史绿 run 的工件不可用于安装验证；外加「装哪棵树」的前置未裁。管辖不在 main。
- **Goal1 可关闭 FAIL**：采 R2 fable-b 的四件可枚举差额（账单/清单/归一/权威对齐），并正面认定代码功能面已不是阻塞（211 项实测绿）。

## 5. 与本轮工单要求的对应

- 四件事 PASS/FAIL 及管辖范围：`SOTA_ACCEPT.md` §1（总表 + 四节展开，每节含「不及于」清单）。
- 不完善项终表：`SOTA_ACCEPT.md` §2（P0×8、P1×11、P2×8，共 27 项，已跨 12 份报告去重合并，每项带证据出处与修复归属）。
- main 可修 / Goal1 侧 / 作者侧划分：`SOTA_ACCEPT.md` §3（四值归属汇总，另含「必须裁决」第四类——有五项在拍板前没有合法修复动作，硬归给任何一侧都会误导执行）。

## 6. 未做声明

- 未执行 `git checkout` / `git commit` / `git push`；未离开 `cursor/project-status-audit-c49c`。
- 未改动 `docs/**` 与 `crates/**` 任何字节（`git status` 仅本目录未跟踪文件）。
- 本轮写入仅两个文件：`.agent_workspace/orch-20260826/round3/fable-a/SOTA_ACCEPT.md` 与本 `REPORT.md`。
- 本轮执行的全部命令为只读（`git diff/log/status/fetch`、`gh pr view`、`cargo test`）。
