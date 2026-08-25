# 工作包与 DAG 的计划级真伪 + PR #1 STATUS 陈旧声明清单

槽位：Round 1 fable-b。对照：`docs/FORMAL_WORK_PROMPT.md`（本分支）、`origin/cursor/soul-goal1-7b1c:docs/GOAL1_PLAN.md` 与 `:docs/STATUS.md`（只读）、`origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`。

## 一、计划级仍然为真的部分（保持，不动）

1. **WP 清单本身**：WP01–WP11 + WP13 作为范围切分仍然成立，与 Goal 1 线的实际 crate 布局一一对得上。**WP12 保持删除，禁止复活该编号，也禁止新增 WP14**——T4D/A0 吸收不是新工作包（见下节）。
2. **并行声明**：「WP01 冻结后 WP02–WP06、WP08 可并行」与 GOAL1_PLAN 的 DAG（WP01 独占先行；WP07←WP02+WP08；WP10/WP11←WP08；WP13 收尾）互相一致，无矛盾。
3. **单源分工正确**：DAG 细节只在 GOAL1_PLAN.md（PR #2 线），FORMAL 只留一句并行原则。打磨稿**不要**把 DAG 图复制进 FORMAL（P1 单源）。
4. **D31 分界**：WP11 只读预览在 Goal 1、写执行在 v0.1.1（AC-27），三处文档（FORMAL、GOAL1_PLAN、PRODUCT_LOCK 砍/留）说法一致，无漂移。

## 二、计划级已经不真的部分（打磨稿必须改）

FORMAL 的叙事时态整体停在「未开工」，而 Goal 1 线（PR #2，尖端 `2d2badd`/`a643aef` 附近）STATUS 声称 WP01–WP11、WP13、DPAPI 全部落地，剩 hosted CI 与作者手动。打磨稿不需要背书 Goal 1 线的每一条声明（那要等 hosted 绿），但必须停止暗示「一切从零开始」：

1. 「派 fable planner，覆盖下列 WP」「开工第一动作：CreateGoal：Goal 1」——planner 已跑，产出即 GOAL1_PLAN.md；再派一次就是重做。
2. 剩余的活不是 WP 形状，是 **G/M/S 形状**（BLOCKERS）：M2/M3 合入门禁、G1/G1+/G2/G3 关闭门禁、S1 发货残余、G4/S2 P1。打磨稿应把「WP 清单=规范义务、执行进度以 Goal 1 线 STATUS 为准、剩余关闭义务=G 项+hosted+作者清单」写成一句话。
3. **T4D/A0 吸收义务在 WP 层无处安放**是当前计划的结构洞。按 D30 只减不增，落法是范围内批注而非新包：

| 冻结/阻碍义务 | 落进哪个既有 WP | 内容 |
|---|---|---|
| G1 T4D 判档 + TieScore 持久化 | WP05（人脉图） | 替换 `Tally::band`（T0）为 T4D；分列计数、任一场地 last_contact、silent_days、as_of_utc、algorithm_id 持久化；常量从冻结 crate 导入 |
| G1+ 停止 owner 群消息扇出 | WP06（导入）与 WP05 同批 | owner 群消息不给历史发言人写 Outgoing；incoming 照记 |
| G2 intake 不绕轴锁 | WP03（灵魂档案） | 锁轴不 place_axis；证据仍落、`axis_locked_by_user`；replay 全等 |
| G3 人脉边纠正 | WP05 + WP09（图 UI）+ WP10（A2 读生效档） | 生效档=用户档、机器档另存；GC-9 话术先走 DECISIONS 加性增补 COPY_ZH |
| A2 分列句/沉寂句 + 模板绑定测试 | WP10（起草与摘要） | 换 `a2_render` 供 COPY_ZH 冻结句；源码守卫扫第二套阈值与第三个 `180` |
| as_of 单钟 | WP05（rebuild 调用方） | 全库 `max(occurred_at)`，在丢弃解不开行之前算；禁 per-peer、禁墙钟 |

4. 名称漂移照 BLOCKERS §0 收口：DECISION §6.1 写的 `crates/soul-algo` / `graph_build.rs` 真名是 `soul-algo-tie` / `soul-algo-trait` 与 `crates/soul-graph/src/build.rs`；实现跟真名，打磨稿引用时加一句对照即可，不改冻结文件。

## 三、PR #1（本分支已种子的 docs/）陈旧声明逐条

判断基准：SHARED_BRIEF P4——「计划冻结 ≠ Goal 1 关闭 ≠ main 已有应用」。三态必须同时可见：计划已冻结（真）；Goal 1 已在 PR #2 线大面积落地但未合入 main、hosted 未绿、作者清单未过（真）；main 上无应用、只有算法 crate（真）。

| # | 文件与原句 | 为什么陈旧 | 处置建议 |
|---|---|---|---|
| 1 | STATUS「v0.1 实现 \| 未开始」 | Goal 1 线 WP01–WP11/WP13/DPAPI 均声称完成；「未开始」对任何读者都是假的 | 改为三态：main 无应用；PR #2 线已落地待合入；关闭卡 G1–G3/hosted/作者清单 |
| 2 | STATUS「尚未写应用代码」 | 同上；应用代码存在于 PR #2，只是不在 main | 同上 |
| 3 | STATUS「阻塞：…WP01 需把 schema `$ref` 接到 `_defs` 并补泄漏 fixture」 | Goal 1 线 WP01 已完成该项且有 `schemas.lock.json`（钉 11 份 sha256）；本分支种子 schema 仍是裸 string 旧代 | 删除该阻塞句；改记「本分支 schema 为 PR #1 旧代，字节变更须与 Goal 1 线锁重生成同批」 |
| 4 | STATUS「下一步：CreateGoal：Goal 1。派 fable planner，再按 DAG 派 opus 写码」 | planner 已跑（GOAL1_PLAN.md），写码已发生 | 改为：吸收 BLOCKERS 工序（M3→M2→G1/G1+→G2→G3→S1→作者清单） |
| 5 | STATUS 进度表**无 `ALGO_FROZEN` 行** | 算法冻结（PR #5）已在本分支 git 祖先里，STATUS 却看不见它 | 加一行：算法冻结完成，权威 `docs/algorithms/DECISION.md`（T4D+A0） |
| 6 | FORMAL「当前几乎无代码」 | 对 main 半真（有两个算法 crate），对仓库为假 | 改为如实描述 main 与 PR #2 两线 |
| 7 | FORMAL「开工第一动作」四条 | 全部已执行完毕 | 改为「历史注记」或指向 BLOCKERS 工序 |
| 8 | FORMAL 11.5「写码前阻塞项只有：…`docs/schemas/` 九份…」与 D26「九 schema」 | 实际 11 份文件（九份业务 schema `$ref` `_defs` + `_defs` + `soul-import-v1`）；字面读会诱导「改成九份」 | 措辞精确化，防字面执行 |
| 9 | README「当前处于产品锁定阶段，还没有可运行的应用」 | 对 main 为真、对仓库误导；且 README 未链 `docs/algorithms/`（30 秒找权威失败面） | 三态一句话 + 补 algorithms 链接 |

**不属于陈旧**（不要顺手改坏）：`PLAN_FROZEN` 里程碑本身仍真；「Goal 2 在 Goal 1 关闭前不要启动」仍真；R1/R2/R3 扫描史仍真。

## 四、一句话结论

WP 拓扑不需要动；需要动的是 WP 之外的两件事——把 G1/G1+/G2/G3 作为**既有 WP 范围内的关闭义务**写进计划（不发明 WP12），把 PR #1 STATUS/FORMAL/README 的「未开工」时态改成三态诚实。
