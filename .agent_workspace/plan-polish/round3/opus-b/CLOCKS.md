# 时钟与时间量清册（Round 3 opus-b）

范围：`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`（AC-30 及冻结算法表）、`docs/DECISIONS.md`（D45 及相邻行）、`docs/algorithms/DECISION.md` §3 as_of 段。旁证：`docs/schemas/relationship.schema.json`、`docs/algorithms/COPY_ZH.md`、`crates/soul-algo-tie`、`crates/soul-algo-trait`。

本文件遵守 FORMAL 红线 11 与 `PLAN_INDEX.md`「其余文档一律只写常量名」：阈值一律写常量名。出现数字的地方**只有**逐字引用现有文本或源码的取证行，并且都带文件与行号。

---

## 一、七个时间量，各属哪一层

| # | 时间量 | 属哪一层 | 谁写入 | 谁读 | 权威 |
|---|---|---|---|---|---|
| 1 | 墙钟（进程当前时刻） | 应用层，且**判档面禁用** | OS | 审计时间戳、UI 一般显示 | `DECISION.md` §3 末句、§6.2；D45；D51；FORMAL 红线 12 |
| 2 | `occurred_at` | **导入/归一化层** | 导入适配器（`soul-import-v1` / Telegram） | rebuild 聚合、as_of 缺省的取材 | `soul-import-v1.schema.json`；D37；D38 |
| 3 | `as_of` | **rebuild 调用方（应用层）** | 调用方一次算一个，全库同一个 | 算法 crate 收作入参；随边落库为 `as_of_utc` | `DECISION.md` §3 / §6.3；D45；AC-30 |
| 4 | `last_contact`（任一场地） | 算法层聚合，源数据来自导入层 | `Tally::absorb` | **降档时钟**与 A2 的 P4 近因句，二者共读 | `DECISION.md` §3 时钟公式、§4.3；`relationship.schema.json:62-65` |
| 5 | `last_direct_contact` | 算法层聚合 | `Tally::absorb`（只在 `venue_direct` 行上推进） | **只展示，不判档** | `relationship.schema.json:66-72`；`REJECTED.md:111`（direct-only 时钟已作废） |
| 6 | `silent_days` | 算法层派生 | `Tally::silent_days(as_of)` | 降档、P4 句、用户复核 | `relationship.schema.json:73-77`；`DECISION.md` §3 |
| 7 | 自然日（UTC 桶） | 算法层派生 | `epoch_day`（`div_euclid`，UTC 向下取整） | Strong 的自然日门读 `direct_active_day_count` | `crates/soul-algo-tie/src/types.rs:254`；`relationship.schema.json:51-60` |

### 层与层之间只有一条箭头

```text
导入/归一化层            算法层（soul-algo-*，纯函数）        渲染层
─────────────────       ────────────────────────────       ──────────
occurred_at        ──►  Tally（分场地计数、自然日桶）   ──►  A2 / 图 UI
venue（Direct/Group）──►  direct_* / group_* 计数
                        ▲
调用方算出 as_of  ──────┘（一次 rebuild 一个值）

墙钟 ✗ 不进入这张图的任何一格（审计与一般 UI 显示除外）
```

依赖箭头恒为「应用 → 算法 crate」（D51、`DECISION.md` §6.2、PRODUCT_LOCK 末段）。算法层**不回头**决定导入层的任何分类，导入层**不回头**决定档位。

---

## 二、七条不变式（现行文档与代码共同成立）

| ID | 不变式 | 出处 | 现状 |
|---|---|---|---|
| I1 | 一次 rebuild 全库只有一个 `as_of` | D45；`DECISION.md` §3；AC-30；PRODUCT_LOCK 三条之二 | 四面同义，一致 |
| I2 | `as_of` 来自数据（调用方传入，缺省取全库 `max(occurred_at)`），禁 per-peer | 同上 | 一致；per-peer 有专门的负向探针 |
| I3 | judgement 不读墙钟；算法 crate 不提供缺省墙钟路径 | `DECISION.md` §3 末句 / §6.3；D45；D51；FORMAL 冻结算法表 as_of 行 | 一致；crate 侧 `as_of_max` 只是给调用方的取数助手，规则自己不调用（`types.rs:271-280`） |
| I4 | 只有**一个**近因时钟，读任一场地的 `last_contact` | `DECISION.md` §3 / §4.3；schema `last_contact_utc` 注释；`REJECTED.md:111` | 一致；direct-only 变体已入墓碑 |
| I5 | 降档阈值与「你们最近半年没有往来」句的阈值是**同一个常量、同一个闭区间** | `DECISION.md` §3 常量表 `DORMANT_NOTE_DAYS`；PRODUCT_LOCK 三条之三 | **有一处 `>` 写法未清**，见第四节 F1 |
| I6 | 沉寂只降档、不升档 | `DECISION.md` §3 公式；`recency.rs` `silence_never_promotes` | 一致 |
| I7 | 空观测不进时间运算（`silent_days` 为 0，直接判 Weak） | `DECISION.md` §3；schema `silent_days` 注释 | 一致 |

---

## 三、边界表（闭区间）

`DECISION.md` §3 常量表原文：「沉寂 ≥ `DEMOTE_ONE_BAND_DAYS` 天降一档，**闭区间**（`>=`；任何 `>180` 写法作废）」。据此，以 `D` 记 `DEMOTE_ONE_BAND_DAYS`、`W` 记 `FORCE_WEAK_DAYS`：

| `silent_days` | 档位动作 | P4「最近半年没有往来」句（规范意图） | P4 句（COPY_ZH 现行字面） |
|---|---|---|---|
| `< D-1` | 不动 | 不出现 | 不出现 |
| `= D-1` | 不动 | 不出现 | 不出现 |
| `= D` | 降一档 | **出现** | **不出现** ← 唯一分歧点 |
| `D < x < W` | 降一档 | 出现 | 出现 |
| `= W` | 封 Weak | 出现 | 出现 |
| `> W` | 封 Weak | 出现 | 出现 |

取证（逐字引用，故含字面数字）：

- `docs/algorithms/COPY_ZH.md:70`：「最后往来距 as_of **超过 180 天** → 追加「你们最近半年没有往来。」」——这正是 `DECISION.md` 宣布作废的 `>180` 写法。
- `crates/soul-algo-trait/src/a2.rs:185`：`matches!(self.days_since_last_contact(), Some(days) if days >= DORMANT_AFTER_DAYS)` ——**代码已经是闭区间**。
- `crates/soul-algo-trait/tests/a2_render.rs:305-307`：夹具 `quiet_for_exactly_the_threshold` 断言恰好等于阈值时 `is_dormant()` 为真。本轮实跑 `cargo test -p soul-algo-trait --test a2_render`：22 passed。
- `crates/soul-algo-tie/src/recency.rs:56-63`：`the_edges_are_closed_and_land_on_whole_days` 钉住降档侧同样是闭区间。

所以现状是：**规范、代码、测试三方一致取闭区间，只有 COPY_ZH 的触发条件行还写着严格大于**。方向上不会制造用户可见的自相矛盾（PRODUCT_LOCK 要的是「读到那句话时档位必然已降下」，`>` 写法只会少说一句，不会多说），因此不阻塞；但它违反 `DORMANT_NOTE_DAYS`「不得分裂」，且是一处一天宽的实现陷阱。处置见 `REPORT.md` F1。

---

## 四、两组「看起来同名、其实不同层」的量

### 4.1 D36 的 Direct/Group ≠ T4D 的 `direct_*`

| | D36 的 Direct/Group | T4D 的 `direct_out/in/day` |
|---|---|---|
| 层 | 导入/归一化层：给**会话**贴分类 | 算法层：对已贴好分类的**行**做计数 |
| 判据 | 按发言人数；单活跃发言人的群可被判 Direct（D36 维持） | 无判据，只做加法：`venue_direct` 为真则计入 `direct_*`（`types.rs:363-375`） |
| 输入 | 导出文件里的会话结构 | `Interaction { venue_direct: bool, .. }` |
| 改它要动什么 | 冻结契约（D36 原文：「改判据要动冻结契约」） | 什么都不用动——它没有判据可改 |

**不得互相顶替**：

1. 分类错了（某个群被判成 Direct，或某个双人会话被判成 Group）是 **D36 的事**，只能改 D36 的判据并动冻结契约。**不许**在 T4D 里加一道门去补救——那就是 FORMAL 红线 11 与 D42 禁止的「第二套阈值」，且会让 `REJECTED.md:113`（Round 2 opus-a 的群聊 ≥5/≥2 附加门）复活。
2. 反过来，`direct_*` 计数也**不定义**什么叫一对一。它只是 D36 分类结果的下游算术。任何文档写「T4D 认为单发言人群是一对一」都是把两层揉在一起：那是 D36 认的，T4D 只是照单计数。
3. 实际后果要说清：因为 D36 把「单活跃发言人的群」归为 Direct，所以一个来自群导出的会话**可以**贡献 `direct_*`，从而参与判档。这不是 T4D 的漏洞，是 D36 的定价。审计「这条边为什么是强」时，第一步查 D36 分类，第二步才查 T4D 计数。
4. 同一条纪律对 `group_*` 成立：D33（`{群聊次数}` 渲染持久化的 `group_out_count + group_in_count`，不从证据重算）与 D47（owner 群消息不写 Outgoing）都是**导入/归因层**的决定，它们改变的是喂给 T4D 的行，不是 T4D 的规则。AC-34 就测在这一层。

### 4.2 D38 的「民事日」≠ 算法层的「自然日」

| | D38 民事日校验 | 算法层自然日 |
|---|---|---|
| 层 | 导入层 | 算法层 |
| 干什么 | `is_civil_datetime` 按月天数（含闰年）校验，`2026-02-31` 判缺陷 | `epoch_day = occurred_at.div_euclid(86400)`，UTC 去重桶 |
| 目的 | 挡掉不存在的日期，防 fail-hard rebuild 永久失败（D37 同族） | 给 `active_day_count` / `direct_active_day_count` 分桶 |
| 时区 | 输入串自带 | 恒 UTC，不读机器时区（`types.rs:242-256`） |

两者都叫「日」，但一个是**输入合法性**，一个是**聚合粒度**。已知代价已在代码里如实入档：UTC+8 用户在凌晨 01:00 的一条往来会落进前一个自然日（`types.rs:250-253`）。这是算法层的定价，不是导入层的缺陷，不要拿 D38 去「修」它。

---

## 五、导入层与 as_of 的耦合（三条，供实现者读）

1. **as_of 的缺省值是导入产物。** 缺省 = 全库 `max(occurred_at)`，而 `occurred_at` 全部由导入层写入。因此导入层的时间正确性是**每一条边档位**的上游，不只是那一条消息的事。
2. **导入非事务（D37），所以 as_of 会随导入前进。** 一次半途失败的导入照样可能改变全库 `max(occurred_at)`。跟着必须跑一次**全库** rebuild，而不是补算受影响的 peer——否则库里会同时存在两个 rebuild 的 `as_of_utc`，I1 在存储层被破。
3. **归因决定时钟读数。** D47/AC-34：owner 的一条群消息不得给历史发言人 A、B 刷新 `last_contact`。刷新了，降档时钟就被一条并不存在的往来推后——这是导入层直接改写算法层输入的最短路径。

---

## 六、负向探针清单（现有的 + 建议补的）

| 探针 | 期望 | 现状 |
|---|---|---|
| as_of 改成 per-peer 各取自己最大时间戳 | AC-30 变红 | **已有**，AC-30 Then 明写；crate 侧 `tests/as_of_discipline.rs` 与 `peer_local_as_of_wrongly_revives_the_dormant_tie` |
| 判档口径改回「任一场地计数」 | AC-28 变红 | 已有 |
| 降档时钟改成 direct-only | 应变红 | 规范已作废该变体（`REJECTED.md:111`），**矩阵无对应负向行**；见 `REPORT.md` F3 |
| 同一次 rebuild 后各边 `as_of_utc` 不等 | 应变红 | **无**，见 `REPORT.md` F2 |
| 一条远期（未来）时间戳污染全库 as_of | 应变红 | **无**，见 `REPORT.md` F4 |
| A2 的 P4 句在恰好等于阈值时缺席 | 应变红 | 代码侧已有（`quiet_for_exactly_the_threshold`）；文档侧 COPY_ZH 仍写 `>`，见 F1 |
