# NOTES — Round 2 必须先修什么（Round 1 / fable-b）

按依赖顺序排，不是按 severity 排（severity 见 DEFECTS.md）。

## 先修（阻塞其他一切验证）

1. **`intake` 锁绕过（DEFECTS P1-1）**。最小、最确定、纯函数级：`service.rs` intake 循环里加 `axis_is_locked` 跳过 + 扩展 `correction_lock.rs` 一个「纠正后重答问卷」用例。任何后续 A0 评测都以此为前提，先修免得所有人对着一个有洞的锁打分。
2. **导入去重（P1-4）+ 时间戳归一化（P1-6）**。这两个决定「计数是否可信」；T0 的任何参数讨论（阈值、封顶、会话段）在计数会翻倍、排序会错位的管道上都是空谈。去重键 `sha256(source|external_id)`，归一化在 importer 边界做。

## 算法本体（Round 2 的主菜）

3. **venue 分列 + 群聊封顶（P0-2 / INNOVATION #1）**。注意：`import_to_graph.rs::a_message_sent_to_a_group_is_evidence_of_contact_with_everyone_in_it` 把扇出钉成了规范，改判定时必须连测试语义一起改（扇出可以保留为「联络存在性」，但不得进 Strong 计数）。参考实现建议直接写进 `crates/soul-algo` 作纯函数：输入分venue 计数元组，输出 band + 中文解释句，与现库解耦。
4. **图侧纠正/锁定 + rebuild 尊重 user_verdict（P0-1）**。schema 无需破坏性变更：`tie_strength` 是自由 object，可加 `locked_by_user`；rebuild 覆写前读旧 inference 的 `user_verdict` 并保留。
5. **休眠降档（P1-2）+ 互惠每侧下限（P1-3）**。休眠的 as-of 必须取「导入数据的最大时间戳」而非墙钟，否则测试引入 flaky 时间，违反 C2。

## 补测试（便宜、防回归，可与 3-5 并行）

6. 纯 `band()` 参数化表：Moderate 档、count 9/10、days 2/3、单日 20 条互惠 → Moderate（把 build.rs L42-43 注释里的立法场景变成断言）、单向 1000 条 → Weak、`+08:00` 时间戳、跨 UTC 午夜两会话。
7. 「遗忘 contact → rebuild → 图中无此人、无陈旧边」用例（P2-6 需要先核实 store 的 forget 是否清 relationship 行——`soul-store/src/forget.rs` 经 `relationship_evidence` 把 inference 置 orphaned，但边行本身去留未验证）。

## 可推迟但要立案

- 单向 DM 对端登记（P1-5）：Telegram `personal_chat` 的 chat id 即对端，改动在 importer，不碰算法。
- 规模项（P1-8）：代表性证据引用、预索引匹配、WP10 BTreeSet——在 fixture 规模下不疼，真数据前必须做。
- 两套问卷合并（P2-5）：STATUS.md 已自认，接口（`UserStatedSink`）已留好，纯执行。
- band 语义钉子（P1-7）：一段文档 + WP03/WP10 各一处措辞，防止「强」在同屏两个意思。

## 给评委轮的提醒

- 不要被 T0 测试的绿色迷惑：三档测了两档，所有边界零覆盖，扇出行为被测试**钉成规范**而非被发现。
- 不要把 A0 的锁测试当成生产保障：它守的写入者（`AxisProposal` 生产者）在 v0.1 生产代码里不存在；真实存在的第二写入者是 intake，而它绕锁。
- 「20 条一下午不能 Strong」这条设计是对的，别在优化热情里把 span 门槛拆了；要改的是 span 的计量单位（会话段/本地日），不是门槛本身。
