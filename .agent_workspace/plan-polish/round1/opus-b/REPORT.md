# Round 1 · opus-b · 报告

MODEL_SLUG: claude-opus-5-thinking-high-fast

角色：起草 FORMAL 提示词、计划索引、README、关系 schema 候选。只写 `.agent_workspace/plan-polish/round1/opus-b/`，未碰 `docs/`、未碰 `crates/`、未 commit / push / 开 PR。

## 1. 交付物

| 文件 | 目标落位 | 一句话 |
|---|---|---|
| `FORMAL_WORK_PROMPT.md` | `docs/FORMAL_WORK_PROMPT.md` | 新增「计划面与代码面」拓扑节与「冻结算法」结论表；验收矩阵补 AC-28…AC-31；红线补 11/12；WP 清单与 AC-18 / AC-27 原样保留 |
| `PLAN_INDEX.md` | `docs/PLAN_INDEX.md`（新文件） | 30 秒定位：先读四份 / 按问题查 / 仓库拓扑 / 什么不是权威 / 改文件的规矩 |
| `README.md` | `README.md` | 用「这个仓库现在有什么」两行表替换「还没有可运行的应用」 |
| `relationship.schema.json` | `docs/schemas/relationship.schema.json` | `tie_strength` 从裸 `object` 类型化为 T4D 的可复核面 |
| `SCHEMA_MERGE_OBLIGATION.md` | `docs/schemas/` 附注或 `DECISIONS.md` 一条 | 哈希会变、谁在什么时候重算、为什么不动 `required` |
| `probe/verify_relationship_schema.py` | 不合入（过程材料） | 13/13，证明「今天的 Goal 1 输出照旧通过」与「该拒的都拒了」 |

## 2. 逐条对派发

### 2.1 FORMAL 候选

- **WP 清单**：逐字保留 WP01–WP11、WP13，`WP12 已删除` 原样在。另补一句「吸收 `ALGO_FROZEN` 不新开工作包：判档替换落 WP05，特质轴锁落 WP03，摘要渲染落 WP10」，把「只减不增」与「必须吸收冻结算法」这两条同时钉住。
- **「当前几乎无代码」**：整句删除，换成表格化的「计划面与代码面」节。原句对 `main` 成立（只有算法 crate 与文档，本 PR 合入后仍然如此），对 Goal 1 分支不成立（WP01–WP11、WP13 已落地）。新节把这一点写成表，并明确「Goal 1 分支是唯一实现主干，不要另开第二条实现线」。
- **AC-18**：未删。并在 AC-27 说明旁补了一句它为什么容易被误删——它是**只读**证明，与 AC-27 的写执行不是同一件事（D31）。
- **AC-27**：文字未动，仍标 v0.1.1；红线第 7 条「v0.1 不实现文件写」未动。
- **新增四行**（全部 CI，作者手动清单不承担算法门禁，因为它们不需要真机）：

| ID | 钉住的东西 | 反向断言（改坏了必须变红） |
|---|---|---|
| AC-28 | `group_heavy_plus_one_direct_each_way` 不是 Strong；边上落四个分列计数 | 把判档口径改回「任一场地计数」 |
| AC-29 | `lilei_12` 仍是 Strong；`group_heavy_plus_three_directs` 自愈回 Moderate | 吸收 T4D 时误伤锚测试 |
| AC-30 | 一次 rebuild 全库单一 `as_of`，落库可复核；休眠边判 Weak | 改成 per-peer 各取自己的最大时间戳 |
| AC-31 | intake 不移动已锁定的轴；被拒答案如实回报而非静默丢弃；`apply_intake` 与 replay 全等 | 回到 Round 1 那个「留着锁的样子却已经被推断改过」的缺陷 |

  另把 AC-08 的 Then 补成「band 由 `soul-algo-tie` 的 T4D 给出，图侧无第二套阈值」——原行只要求「节点≥3、边有证据」，吸收 T4D 之后它不再够。
- **夹具来源写清**：四行用的夹具已经在 `crates/soul-algo-tie/src/testing` 与 `crates/soul-algo-trait/tests/a0_lock.rs`，Goal 1 侧应当**导入**并断言产品路径同判，而不是重新推导阈值（重新推导直接违反新红线 11）。

### 2.2 relationship schema 候选

关键决定：**base 取 Goal 1 的正文，不取计划分支的**（见 §3.2），因此与 Goal 1 的 diff 恰好只有 `tie_strength` 一块。

不动任何 `required`：顶层五项逐字相同，`tie_strength` 的必填集合 = Goal 1 今天 `TieStrength` 实际序列化的那八个字段。T4D 的九个字段全部可选，但用 `dependentRequired` + `if/then` 绑成一整包——**没有 T4D 就照旧，有 T4D 就必须完整**，半迁移当场红。WP05 落地后它自动变成实质必填，M4 再做形式上的提升。

没有发明分数：`additionalProperties: false` 就是 D22 的执行点，探针里 `score` 与 `tie_weight` 都被拒。`band` 沿用 WP01 已在 `profile`/`evidence` 上用过的 `allOf` 写法（引用 `_defs` 且只收紧，`none` 不是关系档）。`algorithm_id` 枚举只有 `T4D` / `T4`。

哈希与合并义务在 `SCHEMA_MERGE_OBLIGATION.md`：候选 `63c424fe…`，Goal 1 lock 现钉 `df34747e…`；`main` 上没有 `schemas.lock.json`，所以计划 PR 本身不会红，义务落在 Goal 1 下一次 rebase（M2）。

### 2.3 README 与索引

README 原句「当前处于产品锁定阶段，还没有可运行的应用」在 `main` 的语境下**是对的但会误导**——它读起来像「整个项目还没开始写」，而 Goal 1 分支上桌面壳都装得出来了。换成两行表：`main` = 计划 + 算法 crate（装不出来），`cursor/soul-goal1-7b1c` = 实现主干（那条线上可以，但没合回来、hosted CI 与作者手动清单未全绿）。另加两条**当场能跑**的命令（已实测，见 §4）。

`PLAN_INDEX.md` 用「按问题查」而不是「按文件列」组织——后来者的问题是「这件事以谁为准」，不是「有哪些文件」。特意写了「不是权威」一节点名 `.agent_workspace/**` 与 `scan-rounds/**`，这是 P1 单源在实操中最容易破的地方。

## 3. 过程中发现的两处不一致（我无权改，交给父代理裁决）

### 3.1 `ALGO_FROZEN` 正文与夹具实况对不上（低危，但会误导实现者）

`docs/algorithms/DECISION.md` 描述决胜夹具的两处与代码里的夹具不符。以 `cargo run -p soul-algo-tie --example matrix` 打印的矩阵为准：

| 位置 | DECISION.md 写的 | 夹具实际 |
|---|---|---|
| §1 表 | `group_heavy_plus_one_direct_each_way` = 群聊扇出 **100 条 / 50 天** + 每方向各 1 条一对一 | 群聊互惠 **30 次 / 10 天** + 每方向各 1 次（总 32，一对一 2） |
| §4.1 | 自愈实测夹具名 `group_heavy_plus_three_direct_each_way`，「**每方向各 3 条**」 | 夹具名是 `group_heavy_plus_three_directs`，一对一共 **3 次（2 发 1 收）** |

**结论不受影响**：两条冻结条件（`lilei_12` 保持 Strong、`group_heavy_plus_one_direct_each_way` 非 Strong）在实际夹具上都成立，T4D 判 weak / T4 判 strong 的对照也成立。受影响的只是照着 DECISION.md 复现的人会造错夹具，以及照着写测试名的人会写出一个不存在的名字。我的 AC-28 / AC-29 已按**实际夹具**措辞。建议父代理在合并计划 PR 时顺手修 `docs/algorithms/DECISION.md` 这两处描述（属于 11.3 直改白名单：措辞，不动结论）。

### 3.2 九份 schema 已在两条线上分叉（中危，会静默回退 WP01）

计划分支的 `docs/schemas/` 是 PR #1 的旧正文（各 `*_id` 是裸 `string`）；Goal 1 在 WP01 里把它们接到了 `_defs` 并用 `schemas.lock.json` 钉住。逐份比对：`_defs` 与 `soul-import-v1` 两份相同，其余**九份全部不同**，且 Goal 1 的版本严格更紧。

若计划 PR 把旧正文推回 `main`，Goal 1 下一次 rebase 会在九个文件上冲突，而每一次「取 main 的版本」都是在悄悄回退 WP01 的接线。建议：计划 PR 整体采用 Goal 1 的九份正文，再叠 `tie_strength` 这一块。本候选已按此做。

## 4. 验证（本机实跑，Linux，Rust 1.83，无外网）

```
cargo test --workspace                              # 全绿（含 soul-algo-tie / soul-algo-trait 与 doc-test）
cargo run -q -p soul-algo-tie --example matrix      # 打印判档矩阵，AC-28/29/30 的数字取自这里
python3 probe/verify_relationship_schema.py         # RESULT: PASS，13/13
```

探针的三条硬断言：Goal 1 文件的 sha256 与它自己的 lock 一致（`df34747e…`，说明比对基线没错）；候选顶层 `required` 与 Goal 1 逐字相同；同一批用例喂给 Goal 1 今天的 schema，九条该拒的**一条都没拒**。

## 5. 对评价维度的自评

| 维 | 做了什么 | 残留 |
|---|---|---|
| P1 单源 | `PLAN_INDEX.md` 显式点名「不是权威」的目录；FORMAL 只给算法结论表并指向 `DECISION.md`，**一个阈值数值都不复述**（初稿写过 3/10/3/180/360，已删——那正是红线 11 要禁的东西） | 无 |
| P2 可测 | 四条新 AC 都有 Given/When/Then、「谁跑」，且都带反向断言（改坏了必须变红） | AC-30 的「必须让此行变红」需要 Goal 1 侧真的写一个负向测试，不能只靠正向断言 |
| P3 冻结吸收 | T4D/A0 进入 FORMAL 结论表、红线 11/12、AC-08、AC-28…AC-31，以及 schema 的 `tie_strength` 类型 | `DECISIONS.md` 里还没有一条「采纳 ALGO_FROZEN」的拍板（D32？）——不在我的派发内，建议补 |
| P4 现状诚实 | FORMAL 的拓扑表、README 的两行表、索引的「仓库拓扑」三处口径一致：计划冻结 ≠ Goal 1 关闭 ≠ main 有应用 | `docs/STATUS.md` 我没动（不在派发内），它现在写「v0.1 实现｜未开始」，与上述三处**互否**，必须由负责 STATUS 的槽位改掉 |
| P5 不膨胀 | WP 清单逐字未动、WP12 保持删除、AC-27 仍是 v0.1.1、红线 7 未动；新增的只有门禁与类型，没有新功能 | 无 |
| P6 SOTA | FORMAL 可整份粘贴；索引让后来者先读四份 | 索引里指向 `GOAL1_PLAN.md` / `BLOCKERS.md` 的两行现在写的是 `origin/<branch>:<path>`，等那两份合入 `main` 后应改成普通链接 |

## 6. 我没做的事

- 没改 `docs/`、`crates/`、`README.md` 本体；没 commit / push / 开 PR。
- 没动 `docs/STATUS.md`（不在派发内，但见 §5 P4：它与本候选的口径冲突，必须有人改）。
- 没动 `types` 字段的类型（收紧它是产品变更，v0.1 没有推断关系类型的依据）。
- 没有把 T4D 字段直接写进 `required`（理由见 `SCHEMA_MERGE_OBLIGATION.md` §3）。
- 没有为 F04c 加第三道门；没有把任何存储/网络依赖引入算法 crate 的讨论。
