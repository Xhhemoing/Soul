# Round 3 opus-a — 计划文档遗留项原子修复

槽位：Round 3 opus-a。声明 slug `claude-opus-5-thinking-high-fast`，实际按此执行，无静默降级。
绑定：`.agent_workspace/plan-polish/R2-SYNTHESIS.md`。基线：工作树 `c3d5960`（Round 2 已落盘）。
纪律：只动 `docs/` 措辞；不改产品方向；不加 F04c 第三道门；不碰 `crates/`；不 `git commit`。

## 结论

**有材料可修，已修。** 八处原子改动，全部落在 `docs/STATUS.md`、`docs/PLAN_INDEX.md`、`docs/FORMAL_WORK_PROMPT.md`，共 `+17 / −17` 行。
没有新增拍板号、没有新增工作包、没有新增验收行、没有删除任何已通过门禁。产品锁与 SECURITY 一字未动。

## 一、改了什么（逐条附理由与出处）

| # | 文件 | 改法 | 出处 |
|---|---|---|---|
| 1 | `STATUS.md` 各条线表 | 本分支状态 `Round 2 落地中` → `Round 2 已落盘，Round 3 交叉核验中` | 本轮任务：进度表 vs D59 |
| 2 | `STATUS.md` 进度表 | `Round 2 落地：…` → `Round 2 已落盘：…。Round 3 交叉核验中，尚未合入 main` | 同上 |
| 3 | `STATUS.md` M2 行 | 补吸收线已完成 M2 的事实，带分支名与提交号 | R2 fable-a `RESIDUAL.md` P2-6 |
| 4 | `PLAN_INDEX.md`「一件事算不算做完」 | 补半句：Goal 1 整体关闭需矩阵 + 十三片切片同时过（D54） | R2 fable-b `ACCEPTANCE_GAP.md` §一.2 的 P3 |
| 5 | `PLAN_INDEX.md`「数据长什么样」 | `十份正文：九存储 + 一导入 + _defs + lock` → `十份正文 = 九份存储契约 + 一份导入契约，另有 _defs 与 schemas.lock.json` | R2 fable-a `RESIDUAL.md` P2-3 |
| 6 | `FORMAL` 计划面第 4 条 | 指向 `DECISION.md` §6 处括注「该节沿用旧 crate 名，实现跟真名，见 D53」 | R2 fable-b `AC_CONSISTENCY.md` §三.1 |
| 7 | `FORMAL` 矩阵前言 + 矩阵后夹具段 | AC-28…AC-34 的两处口径拆准（见下节） | 本轮任务：FORMAL AC-28…AC-34 一致性 |
| 8 | `FORMAL` AC-33 行末 | 括注「分列缺席」只指遗留边，声明 `algorithm_id` 的边分列必填 | R2 fable-b `ACCEPTANCE_GAP.md` G-7 |
| 9 | `FORMAL` 历史段 | 作者原话与 1–6 步清单整体缩进为引用块（逐字保留，仅改格式） | R2 fable-a `RESIDUAL.md` P2-4 |

### 第 7 条展开：AC-28…AC-34 的两处不一致

改前矩阵后那段写：「AC-28…AC-34 …**夹具不是新造的**：`group_heavy_plus_one_direct_each_way`、`lilei_12`、`group_heavy_plus_three_directs`、`dormant_2019` 已在 `crates/soul-algo-tie/src/testing`」。

这句对 AC-34 是**假的**，且是 R2-SYNTHESIS「潜在边界风险」第 3 条点名的那件事：AC-34 测的是导入归因（owner 群消息不得对历史发言人扇出 Outgoing），需要的是一份**群聊导出样本**，算法 crate 是纯函数库，`crates/soul-algo-tie/src/testing`（本树实测：只有 `mod.rs` / `oracle.rs`）不可能有这种夹具。照原句读，Goal 1 实现者会去 crate 里找一份不存在的夹具，找不到之后最可能的错误反应是「那这行大概不用测」。

改法（两处，都是收窄不是改判）：

- 矩阵前言：`ALGO_FROZEN` 相关行（**AC-28…AC-33**）与**喂给它们的导入归因行**（AC-34）都排在矩阵表内。
- 夹具段：「**AC-28…AC-33** 的夹具不是新造的」；另起半句写明 **AC-34 是例外**，群聊导出样本要在 Goal 1 侧新造，且「新造的只是导出样本，不得顺带新造第二套阈值」——把红线 11 的护栏一并带过去。

AC-34 的门禁文本本身一字未动，`AC-27 不占用表行` 的注记仍然成立。

## 二、验了什么（探针全文见 `PROBES.txt`）

| 探针 | 结果 |
|---|---|
| P1 `180 / 360 / 179 / 359` 字面量，扫 PRODUCT_LOCK / DECISIONS / FORMAL / STATUS / README / PLAN_INDEX / SECURITY | **PASS，零命中**。D52 在 Round 2 已改成 `DEMOTE_ONE_BAND_DAYS`，本轮无残留可清。`algorithms/` 按约定不扫 |
| P2 stale D58-only 指针 | **PASS**。全树只剩三处 D58：DECISIONS 的 D58 行本体（拍板只追加，不改），以及 STATUS 与 PLAN_INDEX 两处已写成 `D58/D59` |
| P3 AC-28…AC-34 引用面 | 改后一致（见上节）。`docs/` 内按编号引用这七行的只有 FORMAL 自身与 STATUS 两处散文，语义自洽 |
| P4 schema 份数措辞 | PLAN_INDEX 已算得清；`DECISIONS.md` D26 的「九 schema」是历史拍板原文，**故意不动**（拍板只追加，且 R2 已判定不必动） |
| P5 负向探针：OAuth / E0 / F04c 第三道门 / v0.1 文件写 | **PASS**。四类命中全是否定式或砍项声明，无一句把禁令写反 |
| P6 `schemas.lock.json` 11 条 sha256 vs 磁盘字节 | **PASS，0 不符**（本轮未动任何 schema 字节，此探针用来证明没动） |
| P7 12 份 JSON 可解析 | **PASS** |
| P8 三份被改文件的表格列数 | **PASS**，无锯齿行 |
| P9 PLAN_INDEX / README 相对链接落地 | **PASS**，全部存在 |

跨分支事实按 D57 现场核过（`git fetch` + `git show`，核于 2026-08-25）：

- `cursor/soul-goal1-7b1c` 尖端 = `df5d2dd`，与 STATUS 第 19 行、SECURITY 第 27 行同树一致 —— R2 fable-a P1-6 的「同树三个尖端值」已闭合，本轮无需再改。
- `cursor/goal1-unblock-a073` 尖端 = `c81c233`，其 `Cargo.toml` 成员表含 `crates/soul-algo-tie` 与 `crates/soul-algo-trait` —— M2 在吸收线上确实已做，第 3 条改动有字节支撑。

## 三、看过但**故意没改**的

| 项 | 为什么不动 |
|---|---|
| `DECISIONS.md` D26「九 schema」 | 历史拍板原文。拍板只追加不改写；份数口径已在 PLAN_INDEX 单点定清 |
| `DECISIONS.md` D58 停在收紧前时态 | 同上。D59 就是它的续写，两处指针都已写成 `D58/D59` |
| `PRODUCT_LOCK.md`、`SECURITY.md`、`README.md` | 扫完无遗留。改它们只会制造无谓的合并冲突面 |
| `.agent_workspace/context/plan/` 七份旧副本（R2 P2-5） | R2-SYNTHESIS L4 已拍「保留快照 + README 非权威」。这是拍过的板，不是遗留 |
| `ACCEPTANCE_GAP` G-2 / G-3 / G-4（AC-28 分列等值、F04c 反向钉死、AC-30 过滤前取 max） | **都是加断言，不是改措辞**。超出本槽位「原子措辞修复」的授权面，且 G-3 触及 F04c 红线邻域。留给父代理裁 |
| `ALGO_FROZEN` §4.3 夹具名/参数漂移 | `docs/algorithms/` 是冻结面，不动。D53 + `AC_CONSISTENCY` §三.2 已裁决「实现跟 crate 真名」，本轮已在 FORMAL 第 4 条把这条裁决摆到读者眼前（第 6 条改动） |
| BLOCKERS 的 D32 撞号 | DECISIONS 脚注已写死 PR #6 合入检查单。本 PR 不整份拷 BLOCKERS |

## 四、给父代理的合并注记

1. 本轮零 schema 字节改动，`schemas.lock.json` 无需重算（P6 已证）。
2. 第 3 条改动给 STATUS 引入了 PR #7 的提交号 `c81c233`。若父代理在合 PR 前又刷新一次 STATUS，须按 D57 让同树对同一分支的「核于」提交号保持一致。
3. 第 9 条只改 markdown 引用层级，作者原话逐字未动；若担心 diff 噪音可单独成一个提交。
