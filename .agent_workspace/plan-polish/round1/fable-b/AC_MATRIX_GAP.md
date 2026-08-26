# AC 矩阵差距审计（逐行：AC × PRODUCT_LOCK 切片 × T4D/A0 义务）

槽位：Round 1 fable-b。只读审计，不改 `docs/`。
权威依据：`docs/FORMAL_WORK_PROMPT.md`（AC-01–AC-26）、`docs/PRODUCT_LOCK.md`（13 条切片 + 自主拍板「人脉图：边=互动强度」）、`docs/algorithms/DECISION.md`（§2 保留清单、§3 常量与 as_of 纪律、§4 已知代价、§6 合并义务）、`origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`（G1/G1+/G2/G3、S2/S5）。

**先说合法性**：D30「只减不增」约束的是**工作包**，不是验收行；D29 把 FORMAL 矩阵定为验收权威；SHARED_BRIEF 第 5 条明确要求补算法相关 Given/When/Then。所以下面的 `add` 行不违反冻结纪律。新增行编号从 AC-28 起（AC-27 保留给 v0.1.1 文件执行，不占用）。

## 一、现有 26 行逐行判定

| AC | 对应切片 | T4D/A0 义务 | 判定 | 说明 |
|---|---|---|---|---|
| AC-01 托盘不提权 | 切片 1 | 无 | **keep** | 谁跑已如实写「作者手动 + 安装 smoke」 |
| AC-02 默认全关 | 切片 1 | 无 | **keep** | — |
| AC-03 问卷→user_stated | 切片 2 | A0 问卷是唯一 v0.1 证据来源 | **keep** | A0 的入档半边已覆盖；锁轴半边是缺口（见 AC-32） |
| AC-04 JSONL 导入无明文 | 切片 2 | 无 | **keep** | — |
| AC-05 Telegram 映射 | 切片 2 | 间接（导入行是 T4D 的观测源） | **keep** | 归因正确性另立 AC-29（G1+） |
| AC-06 inference 可解引用 | 切片 3 | DECISION §6.2「取证留在 Goal 1」 | **keep** | 覆盖档案与图两类推断 |
| AC-07 语气纠正不覆盖 | 切片 4 | A0 纠正锁（档案面） | **keep** | 只覆盖**档案轴/语气**。切片 4 与不可协商 5 同样约束人脉边，缺口在 AC-31 |
| AC-08 节点≥3 边有证据 | 切片 3 | TieScore 持久化义务（§6.4） | **amend** | 现行写法 T0 也能过（BLOCKERS G1 实证）。增补 Then：每条边携带 `band ∈ {强,中等,弱}`、一对一/群聊分列计数、任一场地 `last_contact`、`silent_days`、`as_of_utc`、`algorithm_id`。判档**行为**门禁在 AC-28/29/30，本行只钉字段在场 |
| AC-09 采集关=0 | 切片 7 | 无 | **keep** | CI 假源；真机是加分（BLOCKERS S5），不改 |
| AC-10 开≥1/关后 1s 无 | 切片 7 | 无 | **keep** | 同上 |
| AC-11 只 E1 精确 origin | 切片 8 | 无 | **keep** | — |
| AC-12 第三人占位 | 切片 8 + 第三人默认占位节 | 无 | **amend** | BLOCKERS S2：crate 边界测试**自带** `KnownIdentifiers`，嵌中文名夹具红不了。增补 Given/谁跑：断言必须落在 **session/产品缝**（headless 起草），`KnownIdentifiers` 从库中第三人显示名装载，不由测试自供 |
| AC-13 单次豁免 | 切片 8 | 无 | **keep** | — |
| AC-14 记忆 CRUD | 切片 5 | 无 | **keep** | — |
| AC-15 遗忘销毁 CK | 切片 5 | 无 | **keep** | — |
| AC-16 摘要有证据无诊断词 | 切片 6 | A2 是 band 纯消费者（§2.3） | **keep** | 渲染器纪律与模板绑定不塞进本行，另立 AC-33 |
| AC-17 无 key 降级 | 切片 6 | A2 统计降级路径 | **keep** | — |
| AC-18 只读扫描/未授权拒绝 | 切片 9（D31） | 无 | **keep** | 禁止为其加第三道门或删除（D31 已否决删除） |
| AC-19 未知/改 hash/重放拒绝 | 切片 9 + 不可协商 9 | 无 | **keep** | — |
| AC-20 研究预览第三人=0 | 切片 10 | 无 | **keep** | — |
| AC-21 零非回环连接 | 切片 11 | 无 | **keep** | Windows 侧由 install-smoke 进程外 TCP 表补位（Goal 1 已做），属实现细节非计划改动 |
| AC-22 云开关尚未启用 | 切片 11 | 无 | **keep** | — |
| AC-23 审计链无正文 | 切片 12 | 无 | **keep** | — |
| AC-24 崩溃丢 ≤1 | 切片 12 | 无 | **keep** | — |
| AC-25 三路注入 | 不可协商 8 / 切片 12 | 无 | **keep** | — |
| AC-26 CI 全绿 | 切片 13 | schema 冻结检查是常量单点的载体 | **amend** | BLOCKERS S5：「打包绿」改为「package job 绿 + 作者签名安装包」，或如实声明 Goal 1 不能只靠 CI 关闭。另：schema 检查项在 Goal 1 线含 `schemas.lock.json` 一致性——见文末锁文件注记 |
| AC-27 文件执行撤销 | 砍/留表 → v0.1.1 | 无 | **not-a-gate**（已正确标注） | 保留原句原位，防回弹 |

## 二、缺失行（add，全部可由 CI 跑，夹具可复用 `soul_algo_tie::testing`）

Given/When/Then 全文见同目录 `SOTA_ACCEPT.md`；此处只给映射。

| 新 ID | 门禁 | 对应切片 | 对应冻结义务 |
|---|---|---|---|
| AC-28 | T4D 直连判档（`lilei_12` 保全、2/3、9/10、日 2/3、179/180/359/360 闭区间、F04c 钉死为 Strong） | 切片 3（边=互动强度） | DECISION §3、§6.1；BLOCKERS G1 |
| AC-29 | 群扇出不产档 + 导入归因（群洪+每向 1 条→Weak；仅群聊→Weak；+3/+3 自愈→Moderate；owner 群消息不给历史发言人写 Outgoing） | 切片 2/3 | DECISION §4.1；BLOCKERS G1+ |
| AC-30 | as_of 单钟（全库 `max(occurred_at)`，在丢弃解不开的行**之前**算；禁 per-peer；禁墙钟；`as_of_utc` 全边一致；A2 {截止日期} 同源） | 切片 3/6 | DECISION §3 as_of 纪律；BLOCKERS G1 |
| AC-31 | 人脉边用户纠正（生效档=用户档、机器档另存、A2/图只消费生效档、GC-9 话术门） | 切片 3+4；不可协商 5 | BLOCKERS G3；COPY_ZH §4 P5 |
| AC-32 | intake 不绕轴锁（锁轴不动、证据仍落、`axis_locked_by_user`、replay 全等；附：单一来源永不升强 = A1 空转钉死） | 切片 4 | DECISION §2.2、§2.4；BLOCKERS G2 |
| AC-33 | A2 纯渲染器（P1b 只渲染携带数、分列缺席整句不出、无第二套阈值、产品 crate 无第三个 `180`、band 词单源、模板绑定断言、P4 不变式） | 切片 6 | DECISION §2.3；COPY_ZH §4/§5 |

## 三、not-a-gate（禁止有人把它们写成门禁）

| 项 | 为什么不是门禁 | 权威 |
|---|---|---|
| 预测准确率 | 冻结程序明文「不验收预测准确率」 | DECISION §7 |
| 导入幂等/去重 | P1 设计目标，需身份/事务设计，非矩阵行 | BLOCKERS G4 |
| F04c 第三道降档门 | 静默修补禁令；AC-28 反向钉死 F04c→Strong 是**冻结代价**不是缺陷 | DECISION §4.2/§5 |
| 真机托盘/UAC/真机采集/WebView2 流量 | 作者手动清单，CI 不能替 | PRODUCT_LOCK ASSUMPTION；BLOCKERS S5 |
| hosted CI minutes 恢复 | 运维事实，不是验收语义 | Goal 1 STATUS |
| 合并两个算法 crate / 180 别名 | 冻结后可后置的合并义务 | DECISION §0；BLOCKERS §0 |
| Goal 2 二十轮 | Goal 1 关闭前不启动 | D28 |
| WP12 复活 / 新 WP | D30 只减不增；上述六行是验收行不是工作包 | D30 |

## 四、关闭语义与 schema 锁注记（两条 amend 级计划义务）

1. **双门关闭**。FORMAL 现行「Goal 1 完成 = 本文件验收矩阵全部 v0.1 行通过」与 BLOCKERS_FROZEN 的「关闭需要矩阵 **和** PRODUCT_LOCK 切片两套门同时过」不一致。计划稿应采后者原则句：矩阵没有的行不能否决切片，切片没有的行不能否决矩阵。AC-28–33 落地后，两套门在算法面重新咬合。
2. **`relationship.tie_strength` 仍是裸 `object`**（本分支与 Goal 1 线两版皆然）。计划义务：写明字段清单（band、direct_out/direct_in/direct_active_days、group_count、last_contact_at、last_direct_contact_at、silent_days、as_of_utc、algorithm_id）。**但不要在本分支改 schema 字节**：Goal 1 线的 `schemas.lock.json` 钉了 `relationship.schema.json` 的 sha256，本分支改字节会在 M2 合并时逼出一次未经批准的锁重生成。建议：字段清单作为计划义务写进 FORMAL/DECISIONS 增补，schema 字节变更与 `cargo run -p xtask -- schema-freeze --write` 在 Goal 1 线同一个获批变更里完成。
