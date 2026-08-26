# 缺失门禁的 Given/When/Then（可直接并入 FORMAL 验收矩阵）

槽位：Round 1 fable-b。六道缺失门，对应 AC_MATRIX_GAP.md 第二节的 AC-28–AC-33。

通用纪律：

- **测在产品边界**，不是只测算法 crate（BLOCKERS G1：「夹具（产品边界，不只算法 crate）」）。路径必须走 导入 → 加密库 → rebuild → 读边/读摘要；直接调 `soul_algo_tie::score` 的测试不算数。
- 夹具优先复用冻结 crate 自带的 `soul_algo_tie::testing`（`lilei_12`、`group_heavy_plus_one_direct_each_way`、`group_heavy_plus_three_directs`、`group_heavy_plus_directs_one_way` 均已存在于 main），产品侧只做观测行到导入行的翻译，禁止另造第二套语料常量。
- 全部谁跑 = CI（Linux headless 即可，无 Windows 依赖）。

---

## AC-28 T4D 判档只看一对一计数

| 段 | 内容 |
|---|---|
| Given | 加密库中已导入某 peer 的观测：一对一双向共 12 条、跨 6 个自然日、最近 3 天前（`lilei_12` 构型），另有若干群聊行 |
| When | 以全库 as_of 重建人脉图，读该边 |
| Then | band=强；解释句引用的是 {一对一次数}/{一对一天数} 而非总量（COPY_ZH §5.6） |

边界子断言（同一 AC，参数化夹具）：

1. 一对一 2 次 → 弱；3 次（双向）→ 中等（`MODERATE_MIN_INTERACTIONS` 闭门）。
2. 一对一 9 次 → 中等；10 次且自然日 ≥3 → 强（`STRONG_MIN_INTERACTIONS`）。
3. 一对一 10 次但只 2 个自然日 → 中等；3 个自然日 → 强（`STRONG_MIN_ACTIVE_DAYS`）。
4. 沉寂 179 天 → 不降；**180 天 → 降一档**；359 天 → 降一档；**360 天 → 封弱**（闭区间，`>=`；任何 `>180` 实现必须红）。
5. 一对一只有单向 → 弱，无论条数。
6. **F04c 钉死**：多年前满足强门槛、随后休眠 ≥360 天、昨天一来一回 → **强**。此断言钉的是 DECISION §4.2 的冻结代价，防静默加第三道门；它**不是**要求修复的门，改判唯一通道是回退链 §5。
7. 空观测 → 弱，不进时间运算。

## AC-29 群扇出不产档，导入不伪造归因

| 段 | 内容 |
|---|---|
| Given | 库中某 peer：群聊双向 100 条 / 50 天 + 一对一每方向各 1 条（`group_heavy_plus_one_direct_each_way` 构型） |
| When | 重建人脉图，读该边与分列计数 |
| Then | band=**弱**（T0/全场地实现会给强，必须红）；分列精确：direct=2、group=100；群聊行保留展示与 last_contact |

子断言：

1. 仅群聊 50 次双向、从未一对一 → 弱；A2 场合句「只在群聊里见过」如实出现（档位与展示分离，DECISION §4.1）。
2. 自愈路径：上述构型再加每方向各 3 条一对一 → 中等（`group_heavy_plus_three_directs`）。
3. **G1+ 归因（导入侧 Given 不同）**：Given 一份群聊导出，owner 发 1 条消息，会话里有历史发言人 A、B（未出现在该条消息中）；When 导入并重建；Then 不产生 owner→A、owner→B 的 Outgoing 行；A/B 的 `last_contact` 不因这条消息刷新；一个一对一停在 200 天前、且**本人**无群聊行的 peer 按 180 天纪律如期降档。
4. 反向控制（保住 DECISION §4.3 的定价）：peer **本人**昨天在群里发过言、一对一历史满足强门槛且停在 200 天前 → 不降档（`direct_quiet_200_group_yesterday` 语义）。禁止顺手改成 direct-only 时钟。

## AC-30 as_of 单钟

| 段 | 内容 |
|---|---|
| Given | 同一库内两个 peer：甲最近一条 2019 年（休眠），乙最近一条即全库 `max(occurred_at)`；另有若干联系人解不开的观测行 |
| When | 重建人脉图两次：一次缺省，一次显式传 as_of |
| Then | 全部边的 `as_of_utc` 相同且等于全库 `max(occurred_at)`（**在丢弃解不开的行之前计算**）；甲=弱（沉寂数千天）；若实现误用 per-peer as_of，甲会假强，本行必须红 |

子断言：

1. 无墙钟：冻结进程时间或间隔重跑，两次输出逐字节相等；judgement 路径不出现系统时钟调用（源码守卫可加扫）。
2. A2 的 {截止日期} 与边上 `as_of_utc` 同源同值——「到 {截止日期}」永不写成「到今天」（COPY_ZH §0.4/§0.5）。

## AC-31 人脉边用户纠正锁定

| 段 | 内容 |
|---|---|
| Given | 某边机器档=中等；用户把它纠正为强并锁定 |
| When | 新观测进入并重建；打开图与该 peer 的 A2 摘要 |
| Then | 生效档=强（用户档）；机器档另存且继续随观测更新；A2 P5 与图 UI 消费的都是生效档；审计记一条纠正（无正文）；解锁后机器档恢复生效 |

子断言（GC-9 话术门）：冻结 COPY_ZH 无「由你本人指定」变体。变体经父代理 DECISIONS **加性**增补 COPY_ZH（新 P5 key）之前，锁定边**不得**渲染现冻结 P5 原句（「按上面的计数……」对锁定档是撒谎）；渲染器要么出已批准变体，要么整句不出。

## AC-32 intake 不绕轴锁（A0）

| 段 | 内容 |
|---|---|
| Given | 用户已纠正并锁定某特质轴；随后在「再答几题」里提交一份含该轴题目的问卷 |
| When | intake 处理该答卷 |
| Then | 锁定轴位置不变（不调 `place_axis`）；答案仍落成证据，标 `ignored`、理由 `axis_locked_by_user`；审计可见「已记录、未生效」；与冻结 crate `apply_intake` 在映射 ID 后 replay **全等**（逐字段） |

子断言：

1. 冲突策略保持 `LastWriteWins`；不开 `NoDowngrade`。
2. **A1 空转钉死**：v0.1 只有问卷一种证据来源。Given 同一轴问卷答案分布在 ≥3 个不同自然日；Then 轴档不因 TwoKindsAcrossDays 升强——强档唯一来源是本人纠正。此为预期行为断言（DECISION §2.4），不是缺陷单。

## AC-33 A2 纯渲染器

| 段 | 内容 |
|---|---|
| Given | 一条边的 `TieScore` 分列字段为 `Some(direct, group)`；另一条边分列字段缺席 |
| When | 渲染两份人事摘要 |
| Then | 前者 P1b 句「其中一对一往来 {一对一次数} 次，群里同场 {群聊次数} 次。」恰渲染携带的两个数；后者 P1b **整句不出现**；A2 源码无从证据行重算分列的路径（COPY_ZH §5.7） |

子断言（源码守卫 + 模板绑定，随 CI）：

1. `soul-draft` / A2 / 图 UI 无第二套 3/10/3 阈值字面量；产品 crate 无第三个 `180`（`DORMANT_NOTE_DAYS = DEMOTE_ONE_BAND_DAYS`，单点导入）。
2. band 词只出现「强 / 中等 / 弱」，与 `SupportedBand` 一一对应；P5 档位词来自生效档，A2 不自算。
3. P4 不变式：出现「你们最近半年没有往来」句的边必然已降档，任何输入下不得与「强」同屏（时钟与 P4 读同一个任一场地 last_contact，不变式应免费成立，测试防实现漂移）。
4. 全部渲染输出匹配 COPY_ZH 模板结构，过 `assert_non_clinical`，无拉丁字母、无「%」、无「今天/现在」、无非整数数字（COPY_ZH §5.1–§5.4）。

---

## 编号与落位说明

- 六行插入 FORMAL 矩阵，编号 AC-28–AC-33（AC-27 保留 v0.1.1 标记，不占用、不重排现有行）。
- 这六行同时是 BLOCKERS G1/G1+/G2/G3 的验收化：G 项关闭时逐条打钩，矩阵与切片两套门重新对齐。
- 不新增工作包：AC-28/29/30 落 WP05+WP06 范围，AC-31 落 WP05+WP09+WP10 范围，AC-32 落 WP03 范围，AC-33 落 WP10 范围（详见 WP_DAG.md）。
