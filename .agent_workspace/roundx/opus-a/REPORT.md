MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round X / opus-a — T4D 对抗复核

**VERDICT: BLOCK**

阻断项只有一条，且**不在判档逻辑里**：`explain_zh` 会为一个它没有亲手算出工作过程的 band 编造理由，输出算术上为假的中文。另有两条非阻断项，一并入档。

`ALGO_FROZEN` 对 **T4D 判档规则本身成立**。本轮没有挑战任何常量、任何门、任何区间。3/10/3 与 180/360 全部复算通过，包括本轮之前没有人跨过的两处交叉（群聊行当时钟的 180/360 边界、跨 1970 纪元的阶梯序）。**三条缺陷全部在「怎么把结论说给用户听」这一面。**

- 测试证据：`.agent_workspace/roundx/opus-a/TEST_LOG.txt`
- 新增测试：`crates/soul-algo-tie/tests/roundx_opus_a.rs`（唯一改动的代码文件）
  - `cargo test -p soul-algo-tie --offline` → **132 passed / 0 failed / 3 ignored**（原 121 + 本轮 11 通过）
  - `cargo test -p soul-algo-tie --offline --test roundx_opus_a -- --ignored` → **3 failed**，即下述三条缺陷
  - `cargo fmt --check` 干净，`cargo clippy --all-targets -- -D warnings` 干净

命名约定：`todays_*` 钉当前行为（**通过**；缺陷被修好时它们变红，这是故意的）；`bug_*` 断言产品锁要求的行为（`#[ignore]`，**今天失败**）。这样主干保持绿色，不挡其他槽位，而 `-- --ignored` 就是未决缺陷清单。

---

## D1（阻断）解释器为 band 编造理由，不复核自己同句打印的计数

### 现象

`T4D::explain_zh` 按 `score.band` 选模板、按 `score.detail` 选降档话术，**从不拿这两者跟即将打印的计数对一遍**。只要 `detail` 不是当初算出该 band 的那个 `Demoted { band_before }`，句子就会断言证据并不满足的门。

三个实测样本（完整原文见 TEST_LOG.txt §5）：

| 输入 | 证据 | 输出片段 | 为什么是假的 |
|---|---|---|---|
| 存量 Goal 1 band（`algorithm_id="T0"`），`group_only_50` | 一对一 0 次 / 0 天 | 「你们一对一聊过 **0** 次，双方都发过，**不少于 10 次**，而且分布在 **0** 天里，**也不少于 3 天**」 | 0 ≥ 10；0 ≥ 3；双方都没私聊过 |
| T4D 自己的 score，`detail` 落库后丢失，`dormant_360_days` | 一对一 12 次 | 「你们一对一聊过 **12** 次，双方都发过，**但不到 3 次**，还只是打过招呼」 | 12 < 3 |
| T4D 自己的 score，`detail` 落库后丢失，`quiet_200_days` | 一对一 12 次 / 6 天，band=Moderate | 「只分布在 **6** 天里，**不到 3 天**」＋「**本来就已经是最弱的一档**，所以还是**中等联系**」 | 6 < 3；Moderate 不是最弱档 |

影响面：21 个夹具中 **11 个**中招，共 **26 个（夹具 × 规则）组合**——存量 T0 band 11 个，T4 自己的 score 9 个，T4D 自己的 score 6 个。两条产品规则都中招，不只是外来规则那条路径。

### 为什么这条可达，而不是理论风险

两条独立的可达路径，都是本仓库自己写下的：

1. **`src/lib.rs::explain_zh` 的文档就是这么承诺的**——「Anything this crate does not recognise — the test-only oracle, or a band from an older schema — is explained by the default rule」。而 `DECISION.md` §6.5 把这件事定成**合并过渡期的常态**：「在替换合并完成前，Goal 1 现行 `Tally::band` 只是遗留行为……新写的任何解释/摘要一律以本决议为准」。也就是说，过渡期里库里存的是 T0 的 band，渲染用的是 T4D 的话术——正好是上表第一行。

2. **更要命的是第二、三行：`algorithm_id` 就是 `"T4D"`，band 也完全正确。**唯一缺的是 `detail`。而 `DECISION.md` §6.4 规定落库 `TieScore` 携带「band、原始计数、一对一/群聊分列、last_contact、沉寂天数与 as_of」——**`detail` 不在这张清单上**。存储契约里没有它，也没有任何字段能把它重算出来，于是每一条从图里读回来的 score 都处在这个状态。`Detail::RawCounts` 是 `pub` 且从 crate 根导出，对「我只有计数，没跑近因步」的调用方来说是唯一诚实的选项。

### 根因

`zh_reason` 是「**先信 band，再补一个理由**」的结构：分支由 `score.band` 选出，选中后直接把计数插进一个断言该门已过的句子里，不做任何校验。band 与计数一旦脱钩（落库回读、跨 schema、外来规则），它就照样把话说圆。`recency` 那半边同理：`score.band == band_before` 被当成「本来就已经在最底档」的判据，但这个等式在 `Detail::RawCounts` 下恒成立。

### 违反的约束

共享任务书硬约束「**算法必须可向用户解释（能用中文说清「为什么是强/中/弱」）**」，以及 C3 可解释性「用户能复核计数」。这里的失败模式比「解释不清」更重一档：解释**在同一个句子里同时给出计数和与该计数矛盾的结论**，用户照着数反而会认定系统在骗人。

### 修法（不动任何 band，不动任何常量）

二选一，都在渲染层：

1. `explain_zh` 内部用 `TieScore` 自带的分列计数**重算一次 counts stage**，用重算结果选模板；`detail` 只在与重算一致时才用来生成降档话术。
2. band 与自身计数对不上时**拒绝解释**（返回 `Option<String>` / 退化到只念计数不下结论）。

顺带把 `Detail` 补进 `DECISION.md` §6.4 的落库字段清单，否则修完仍会因为存储契约缺字段而复发。

对应测试：`bug_an_explanation_never_contradicts_the_counts_it_prints`（`#[ignore]`，今天失败，失败列表即影响面）。基线 `every_t4d_explanation_of_a_t4d_score_is_consistent_with_its_own_counts` 今天通过——**模板本身没问题，坏的是选模板的依据**。

---

## D2（非阻断）`last_direct_contact` 拿 0 同时当时间戳和「从未」哨兵

`Tally::absorb` 里：

```rust
if self.last_direct_contact == 0 || at > self.last_direct_contact {
    self.last_direct_contact = at;
}
```

Unix 秒 0 是一个合法时刻。一条发生在 0 的一对一记录存进去之后，哨兵判据仍然为真，于是**下一条被吸收的一对一记录无论多旧都会把它覆盖掉**：

```
log = [ direct(t=0), direct(t=-86400) ]  ->  last_direct_contact = -86400   (应为 0)
log = [ direct(t=-86400), direct(t=0) ]  ->  last_direct_contact = 0        (正确)
```

同一批证据换个顺序，「最后一次单独聊天」的日期差一天。`ablation.rs::input_order_does_not_change_any_score` 之所以没抓到，是因为没有夹具把行放在纪元秒上。

**这是一处缺失的 pin，而且是被明确声称已经盖住的那一处**：`tests/as_of_discipline.rs::a_last_direct_contact_before_the_epoch_is_still_reported` 的注释写着 "a real exchange at exactly the epoch second, **or before it**, must not read as 'never'"，但函数体只测了 "before it"（`-DAY*20`），纪元秒那一半从没执行过。

不改 band（该字段只报不判），但它是用户看得见的日期。修法是 `Option<i64>`，或把「有没有一对一行」与时间戳分开存——哨兵挪到别的数值上没用，因为每个 i64 都是合法 Unix 秒。

同一处重载的另一面也钉了：群聊-only 的 tie 上 `last_direct_contact_unix == 0`，调用方直接 `age_days` 会得到 **20 689 天**、`zh_date` 会得到 **1970 年 1 月 1 日**。本 crate 自己的 `ablation.rs` 就是这么读这个字段的。类型没有强制任何 null 检查。

对应测试：`todays_tally_reads_a_private_exchange_at_the_epoch_second_as_never`（通过，钉现状）、`the_never_sentinel_is_not_safe_to_read_as_an_age`（通过）、`bug_last_direct_contact_is_the_newest_private_row`（`#[ignore]`，今天失败）。

---

## D3（非阻断）任一场地时钟的代价被标价为 200 天，实际无上界

`DECISION.md` §4.3 这样标价：

> 一对一历史停在 **200 天前**、群聊昨天还活跃 → 不降档……定价：**该关系的 Strong 资格来自真实的一对一历史**，群聊活跃证明关系未死。

标价引用了一个具体数字，但**机制里没有这个数字**：降档步骤只读 `last_contact`，从不读 `last_direct_contact`，所以不存在一个「一对一历史老到不再够格」的年龄。实测（一条**入向**群消息，即对方在双方共处的频道里发了一句，用户什么都没做）：

| 一对一历史距今 | band | silent_days |
|---|---|---|
| 200 天 | Strong | 1 |
| 300 天 | Strong | 1 |
| 1 000 天 | Strong | 1 |
| 3 000 天 | Strong | 1 |
| **7 000 天**（上次私聊在 2007 年） | **Strong** | 1 |
| 7 000 天，去掉那一条群消息 | Weak | — |

与 §4.2 的关系值得写清楚：§4.2 把「七年休眠 + 昨天**一来一回** → Strong」记为回退触发器（F04c）。本条是它的严格更弱前提版本——**不需要两人此后再私下说过一个字**，只要对方在共处群里冒过泡。§4.2 已被判定为「不可静默修补、须走回退链」级别的问题；那么它的更弱前提版本至少需要同级别的入档，而目前 §4.3 的标价读起来像是个 200 天量级的小让步。

叠加 D3 的第二半：**解释从不说出一对一历史的年龄**。`zh_counts_split` 用 `last_contact_unix`（群消息）落日期，T4D 的计数句给出一对一次数和一对一天数但**没有一对一日期**。于是 `dormant_direct_group_ping_yesterday` 读作「一对一 12 次……最近一次是 2026 年 8 月 23 日，距今 1 天」，自然读法是那 12 次私聊是近期的。它们在 300 天前，而「300」在整句里不出现。

这直接否掉 `tests/explain_zh.rs::the_tie_kept_alive_by_a_group_message_reads_as_a_live_tie` 注释里的那句断言——"A user who thinks that is the wrong band has everything needed to see why it happened"。用户手上只有「一对一 12 次」「群里 1 次」「距今 1 天」，从这三个数推不出私聊在 300 天前结束。

修法分两级，都不动 band：

1. **必做（模板级）**：两个时钟不一致时，分列句同时给出一对一的日期。`last_direct_contact_unix` 已经在 `TieScore` 里，纯渲染改动。
2. **可选（参数级，走父代理 DECISIONS）**：若产品裁定「群里冒泡不足以无限期维持 Strong」，给一对一历史加年龄上界。按 §4.3 自己的说法这属于参数级变更，不动候选结构。

对应测试：`one_incoming_group_message_holds_strong_over_a_private_history_of_any_age`（通过，钉现状 = 缺失的 pin）、`todays_explanation_never_names_when_the_private_conversation_actually_ended`（通过）、`bug_a_band_decided_on_private_rows_says_when_those_rows_happened`（`#[ignore]`，今天失败；现有 21 个夹具里有两个的两个时钟不一致——`dormant_direct_group_ping_yesterday`（差 299 天）与 `group_heavy_plus_directs_one_way`（差 2 天）——两个都不报一对一日期）。

---

## 复核通过的部分（判档算术无恙）

任务书点名的三个方向，逐个正面复算，**均未发现缺陷**：

**「off-by-one 180」** — 闭区间正确。除已有的阶梯单测外，本轮补了此前无人跨过的交叉：**证据来自远超截止线的一对一历史、时钟是一条群聊行**时的端到端边界。0/179 → Strong，180/359 → Moderate，360 → Weak，`silent_days` 与构造的天数逐个相等。`recency.rs` 的单测只测阶梯函数，`ablation.rs` 坐在边界上的夹具全是纯私聊——这个交叉之前是空的。

**「direct vs group mix」** — 门闩不漏。穷举 `d_out 0..5 × d_in 0..5 × {0,1,200} 群聊行`：T4D 的 counts band 是私聊行的函数，与群聊行数量无关。穷举 `d_out 0..12 × d_in 0..12 × spread 1..5`：纯私聊证据上 T4D 与 T4 的 counts band 处处相等。小空间穷举对照 `DECISION.md` §3 冻结定义：无分歧。

**「as_of trap」** — 规则侧干净。`as_of` 是唯一时间输入，无墙钟；`age_days` 对未来证据钳到 0；per-peer `as_of` 陷阱已被 `as_of_discipline.rs` 钉死。本轮补了 `tests/direct_gate.rs` 的一处覆盖盲区：它的生成器全部走 `testing::at`，锚在 2026 年，**从不产生负 Unix 秒**。用跨纪元（1969–1970）的 4 000 条生成证据 × 4 个 `as_of`（含负值）重跑，T4D ≤ T4 处处成立，`silent_days ≥ 0`，`direct_active_days ≤ active_days`，每个 Strong 都有 ≥10 次双向一对一、≥3 个一对一自然日撑着。

无浮点、无墙钟、解释无拉丁字母，均维持。

---

## 结论与移交

**BLOCK**，唯一阻断项是 D1。它不要求重开消融、不动 band、不动常量——是渲染层的一处校验缺失加一处存储契约漏字段。修完 D1 与 D3 的模板部分（两者都只改话术、都不改判档），我这边转 CONFIRM。

D2 与 D3 的参数部分建议入档为已知限制并各自补 pin；D3 的标价文字（`DECISION.md` §4.3「停在 200 天前」）无论是否改机制都需要更正，因为机制里没有上界。

三条缺陷各有一个 `#[ignore]` 测试作为验收条件：修好之后把 `#[ignore]` 摘掉，同时对应的 `todays_*` 现状钉会变红——那正是提示改写现状钉的信号。
