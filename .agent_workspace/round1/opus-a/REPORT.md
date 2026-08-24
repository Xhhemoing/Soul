MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 1 / opus-a — Tie-strength 族参考实现（T0/T1/T2/T3）

产出：`.agent_workspace/round1/opus-a/soul-algo-tie/`，独立 Cargo 包，edition 2021 / rust-version 1.83，
**零依赖**（`cargo tree` 只有自己一行），`#![forbid(unsafe_code)]`，无 SQLCipher / Tauri / HTTP / 网络 / 时钟读取。

- 测试：56 个（28 单测 + 14 对照 + 9 解释语言 + 4 Goal 1 保真 + 1 doc-test），全绿。
- `cargo fmt --check` 通过，`cargo clippy --all-targets -- -D warnings` 通过。
- 完整命令与输出：`TEST_LOG.txt`。
- 对照矩阵可复现：`cargo run --example matrix`（下表就是它的输出，报告不会和代码走偏）。

```
$ cd .agent_workspace/round1/opus-a/soul-algo-tie && cargo test
test result: ok. 28 passed; 0 failed
test result: ok. 14 passed; 0 failed
test result: ok. 9 passed; 0 failed
test result: ok. 4 passed; 0 failed
test result: ok. 1 passed; 0 failed   (doc-test)
```

## 1. 对照矩阵（同一份证据，四条规则）

| fixture | 次数 | 天数 | 互惠 | 私聊 | T0 | T1 | T2 | T3 |
|---|---:|---:|---|---|---|---|---|---|
| `empty` | 0 | 0 | 否 | 否 | weak | weak | weak | weak |
| `single_inbound` | 1 | 1 | 否 | 是 | weak | weak | weak | weak |
| `lilei_12_over_6_days` | 12 | 6 | 是 | 是 | **strong** | **strong** | **strong** | **strong** |
| `twenty_in_one_afternoon` | 20 | 1 | 是 | 是 | moderate | moderate | moderate | moderate |
| `group_only_50` | 50 | 50 | 是 | 否 | strong | strong | strong | **moderate** |
| `dormant_since_2019` | 20 | 10 | 是 | 是 | strong | **weak** | moderate | strong |
| `one_sided_100_outbound` | 100 | 100 | 否 | 是 | weak | weak | weak | weak |

四条规则只在两处分歧，而且分歧正是各自被提名的理由：

1. **群聊盲区**（`group_only_50`）。同一个项目群里五十天、五十条、双向，T0/T1/T2 都判 strong，只有 T3 拒绝。
   T1 的群权重 0.4 只是把上升变慢，没有拦住它：`T1::mass` 在这条 fixture 上仍 > 8（`comparison.rs:
   fifty_group_messages_separate_the_venue_aware_rule_from_the_rest`）。**降权解决不了定义问题。**
2. **近因盲区**（`dormant_since_2019`）。2019 年停掉的密友，T0/T3 到 2026 年仍判 strong；T1 衰减到 mass < 0.001 判 weak；
   T2 只掉一档到 moderate。

其余五条 fixture 四条规则完全一致，说明**互惠门闩 + 跨天门闩这两条是全族共识**，不是某一条规则的特色。

## 2. 对 Goal 1 T0 的保真度

`src/t0.rs` 是 `soul-graph::build::Tally::band` 的直译，三个常量原样搬来并有测试钉住
（`thresholds_match_goal_one_constants`）：

| Goal 1 `graph_build.rs` | 本包 |
|---|---|
| `MODERATE_MIN_INTERACTIONS = 3` | 同名同值 |
| `STRONG_MIN_INTERACTIONS = 10` | 同名同值 |
| `STRONG_MIN_ACTIVE_DAYS = 3` | 同名同值 |
| `is_reciprocal() = outgoing > 0 && incoming > 0` | 同 |
| `band()` 三分支顺序 | 同 |
| `active_days: BTreeSet<String>`，键 = RFC 3339 前 10 字符 | `BTreeSet<i64>`，键 = `unix.div_euclid(86400)` |
| `conversations: BTreeSet<String>` | `BTreeSet<u64>` |
| `first/last_contact` 用字符串字典序取 min/max | 整数 min/max |
| `now` 不参与 | 不参与，参数命名 `_now_unix` |

**保真度不是靠眼睛看的。** `tests/goal1_fidelity.rs` 里按快照重抄了一份 Goal 1 的字符串版 `Tally`
（含 `utc_date()` 的「取前 10 个字符」原写法和 RFC 3339 格式化），然后用定长 LCG 生成
2000 组随机互动 × 3 个 peer（共 6000 次比较，时间窗口 5 天、秒级抖动，专门制造跨日边界）逐条比对
band，全等；另有 500 组跨 1970 年前后的负时间戳同样全等。日期两种写法的等价性单独有测试
（`the_two_spellings_of_a_utc_date_agree`，覆盖午夜前后 ±1 秒和纪元前后）。

`TieScore` 的七个计数字段与 `graph_model.rs::TieStrength` 一一对应（band / interaction_count /
outgoing / incoming / conversation_count / active_day_count / first / last），只把 `Timestamp`（RFC 3339 字符串）
换成 `i64`，因为本包不能引 `soul_schema`。`the_counts_t0_reports_are_the_counts_goal_one_reports` 钉住了这组数。

**唯一的行为差异是 0 条证据时的时间戳**：Goal 1 的 `Tally::new` 必须拿一条互动才能构造，所以「空 tally」在那边
不存在（没有互动就没有边）；这里 `score()` 对任意 peer 都要返回一个 `TieScore`，`interaction_count == 0` 时
`first/last_contact_unix` 填 0。**0 是「没有往来」的哨兵，不是 1970 年**，`TieScore::is_empty()` 是判定它的正路，
`explain_zh` 走的也是这条分支（不会输出「1970 年」）。若最终规范要合并进 `soul-graph`，建议把这两个字段改成
`Option<Timestamp>`，或者维持「无证据不建边」由调用方保证。

## 3. 我解决的歧义（逐条，含理由）

### 3.1 T3 的「群聊封顶」与「群聊可 Moderate」自相矛盾（最重要）

任务书同时写了「Group-only ties: cap at Weak（even if reciprocal and frequent）」和
「Moderate iff reciprocal AND (direct OR (group AND count>=5 AND active_days>=2))」。严格读第一条，第二条括号里的
group 分支就是死代码：凡它能提拔的都是 group-only，立刻被封回 weak。

**解决**：把「cap at Weak」理解为**针对 Strong 档**的断言（这也正是 Granovetter 论证真正主张的：只在群里遇到的人
是典型弱关系，永远不该进最强档），保留显式的 Moderate 子句，于是群聊天花板 = Moderate，并且要先跨过
「≥5 次 / ≥2 天」这道额外的门。

**严格读法只差一个常量**：`t3::GROUP_ONLY_CEILING`，从 `Band::Moderate` 改成 `Band::Weak`，全部群聊关系立刻回到 weak。
两种读法都有测试（`group_only_never_reaches_strong_however_much_traffic`、
`the_strict_reading_of_the_brief_is_one_constant_away`），Round 3 选哪个都是一行改动，后果已经写在测试里。

我倾向保留 Moderate 天花板，理由是产品锁要求「人脉图」反映真实关系：把一个天天在项目群里互相 at 的同事显示成
和陌生人同档（weak），用户会认为图是错的，而 moderate + 一句「你们只在群里聊过」是既真实又可纠正的说法。

### 3.2 活跃天用 epoch day 而不是本地日历

`epoch_day(t) = t.div_euclid(86_400)`。用 `div_euclid` 不用 `/`：1969-12-31 的时间戳是负数，截断除法会把它并进
1970-01-01，`comparison.rs: evidence_from_before_1970_does_not_collapse_into_one_day` 钉住这条（导入损坏时钟的
归档会踩到）。全程不读机器时区，所以同一份证据在上海的笔记本和 CI 上得分相同
（`active_days_are_utc_and_do_not_follow_the_machine_clock`）。

**代价照实说**：UTC+8 的用户凌晨 1 点的消息算前一天。活跃天是「有没有摊开在时间上」的信号，边界上最多差一天，
而且对所有观察者差得一样。这与 Goal 1 现状一致（那边也按 UTC 日期字符串切），换成本地日历会让
「同一条证据在两台机器上得分不同」，那个代价更大。

### 3.3 T1 的若干未指定处

- **互惠与活跃天不参与加权**：`reciprocal` 和 `active_days >= 3` 仍按原始计数判定，只有 mass 是加权的。
  理由：门闩是定性的（有没有回过话），加权只是「多重要」，两者混起来会让门闩随时间自己失效。
- **未来时间戳**：`age_days = max(0, now - t)/86400`，即钳到 0，权重最高 1.0。导入的日志经常带跑快的时钟，
  不钳的话权重会 > 1 无限增长（`evidence_from_the_future_is_not_worth_more_than_today`）。
- **半衰期取 90 天**（任务书指定），不是 30。30 天会把「今年春天以后没联系过的好朋友」直接降级，人脉图会变成
  「最近六周通讯录」。90 天的语义可以对用户说清：三个月前的一次往来，算今天的一半。
- **原始计数不加权落库**：`TieScore` 里七个数全是未加权原始值（`raw_counts_stay_unweighted`），用户仍能手数。
- **mass 用 `f64` 且比较用 `>=`**：确定性没问题（同输入同输出，无并行归约），但阈值附近的浮点边界属于
  「不可手算」的一部分，见下面 4.2。

### 3.4 T2 的近因用整日下取整

`recency_days = floor((now - last_contact)/86400)`，于是「7 天 23 小时」仍在最高档、「8 天」掉档
（`recency_boundaries_are_whole_days`）。用浮点天数会让边界依赖秒级抖动，测试变脆。
空证据时 recency 记 1（最低档），但这条其实不影响结果：互惠门闩已经先把它判成 weak。

### 3.5 `explain_zh` 只拿得到 `TieScore`，而 T3 的判据不在 `TieScore` 里

这是任务书结构本身的一个洞，值得抬到 Round 2：**`TieScore` 没有 venue 字段，而 venue 正是 T3 的决定性事实**；
同理 T1 的 mass 和 T2 的三档也不在 `TieScore` 里。后果是 `T3::explain_zh` 只能含糊说
「要么单独聊得不够多，要么往来都发生在群里」，不能指名道姓。

处理：签名照任务书实现（不擅自改 trait），另加一个 `T3::explain_zh_with_venue(&score, any_direct)`，
拿到事实就能说准话（「你们的往来都发生在群里，没有单独聊过」）。两者都有测试
（`t3_can_name_the_venue_when_it_is_given_the_venue`）。

**建议最终规范给 `TieScore` 加一个 `any_direct: bool`**（一个布尔，不含正文、不含姓名，不破坏隐私约束），
这样解释和判据同源。若不加，UI 层就必须再查一次证据才能解释 T3，那是把可解释性外包出去。

### 3.6 其他小决定

- `score()` 内部按 `peer_id` 过滤，其它人的证据完全不看（`other_peers_in_the_log_are_ignored`）。
- 未知 peer / 空证据一律返回 weak + 全 0，不 panic、不返回 `Option`（`unknown_peers_score_as_nothing_observed`）。
- `TieAlgo` 枚举与 trait 两套入口给同样结果（`enum_dispatch_agrees_with_the_trait`）；
  另加 `score_ego_network()` 一次扫描算全图（`BTreeMap` 分组），避免「每个 peer 扫一遍全表」的隐式 O(n·p)。
- 结果与输入顺序无关（`input_order_does_not_change_any_score`），证据库不承诺顺序。
- `explain_zh` 全族**不含任何拉丁字母**，这条比「禁 score/percentile/RFM 词表」更严，
  顺带保证 `algorithm_id` 不会漏到用户界面（`no_explanation_leaks_english_jargon`）；
  另有中文侧禁词表（分数/评分/百分/得分/指数/打分/%）。

## 4. 按共享评价维度打分（1–5，附证据）

| 维 | T0 基线 | T1 衰减 | T2 RFM | T3 Granovetter |
|---|---|---|---|---|
| C1 产品锁契合 | 4 | 4 | 3 | **5** |
| C2 可测性 | **5** | 4 | **5** | **5** |
| C3 可解释性 | **5** | 2 | 3 | 4 |
| C4 隐私 | **5** | **5** | **5** | **5** |
| C5 稳健性 | 3 | 4 | 3 | 4 |
| C6 计算成本 | **5** | **5** | **5** | **5** |
| C7 SOTA 对齐 | 2 | **5** | 4 | **5** |
| C8 创新必要 | 基线 | 3 | 2 | 4 |

证据与理由：

**T0**：C3=5，用户能逐条数出全部判据，没有任何不可复核的量。C5=3 有两个明确失效面，
`group_only_50` 和 `dormant_since_2019` 都判 strong。C7=2，只用了频次+互惠，Granovetter 的 venue 维度和
recency-weighting 都没吸收。C2=5，全整数阈值，边界可枚举（`ten_over_three_days_is_the_exact_strong_boundary`）。

**T1**：C7=5，recency-weighted decay 是成熟做法。C5=4，唯一真正修好 dormant 的规则。
**C3=2 是它的死穴**：band 由一个用户永远算不出来的浮点 mass 决定，而 `TieScore` 里的计数解释不了它——
`dormant_since_2019` 那条解释只能说「最近很久没有新的往来了」，说不出「为什么是 weak 不是 moderate」。
产品锁写着「推断必须可向用户解释」，这是硬约束不是偏好。C8=3：它解决的问题是真的，手段太贵。

**T2**：C1=3，RFM 是营销词汇搬家，「monetary = 活跃天」这个映射没有人际关系理论支撑，是类比不是论证。
C5=3，补偿性求和让短板能被长板买回来：两条今天的互相消息就到 moderate
（`two_messages_today_already_reach_moderate`，T0 判 weak）；十条消息挤在最近两天就到 strong
（`axes_compensate_for_each_other`，T0/T3 都拒绝）。C8=2：既没有比 T3 更懂关系，也没有比 T1 更懂时间。

**T3**：C1=5，直接服务「复刻人脉图」——把群聊熟人和真朋友分开正是这张图的意义。C7=5，Granovetter 是这一族的原典。
C4=5，只多要一个布尔。C5=4，扣分在仍然没有近因。C3=4，扣分只因为 3.5 那个字段缺口。

## 5. 给 Round 2/3 的结论与建议

1. **T2 建议否决**。它在任何一条 fixture 上都不是唯一正确的那个，理论基础最弱（C1=3, C8=2），
   而它带来的补偿性正是产品锁最不想要的「说不清为什么」。
2. **T1 不建议作为落地形态，但它指出的问题必须解决**。`dormant_since_2019` 判 strong 是 T0 的真实缺陷：
   用户看到七年没联系的人被标成「强联系」，会认为整张图不可信。
3. **T3 是最强的单个候选**：修掉了 T0 最刺眼的群聊盲区，而且没有牺牲任何可数性（判据全是计数和布尔）。
4. **建议 Round 2 提名 T4 = T3 + 可数的近因降档**，用「档位降级」而不是浮点 mass 来吸收 T1 的洞见，
   保住 C3=5：

   ```text
   band = T3(tally)
   silent_days = floor((now - last_contact) / 86400)
   if silent_days > 180: band = min(band, Moderate)
   if silent_days > 540: band = min(band, Weak)
   ```

   解释可以完全可数：「你们上一次说话是 2019 年 6 月 10 日，到今天已经 2632 天，所以这条关系降到弱。」
   常量 180/540 需要 Round 2 用 fixture 定；`Band::capped_at` 已经在包里，接上去只是几行。
   本轮**没有**实现 T4，因为任务书把枚举钉成 `{T0,T1,T2,T3}`，不擅自扩张竞争池。
5. **若最终规范采纳 T3/T4，请一并给 `TieScore` 加 `any_direct: bool`**（见 3.5），否则解释层要绕路。
6. **`GROUP_ONLY_CEILING` 需要仲裁**（见 3.1）。这是本轮唯一一个我按理由裁决、但任务书字面支持另一种读法的地方。

## 6. 硬约束自检

| 约束 | 状态 | 证据 |
|---|---|---|
| 推断可解引用证据 | 输入即证据行；输出只含对这些行的计数 | `every_rule_agrees_about_the_counts_whatever_it_decides_about_the_band` |
| 禁 score / percentile / 诊断词 | 输出只有 `Band` 三档 + 计数；解释无拉丁字母、无中文量化词 | `no_explanation_leaks_english_jargon`、`no_explanation_makes_a_clinical_or_quantitative_claim` |
| 第三人数据不出本机 | 输入无正文无姓名，只有 id / 方向 / 时间 / 场合 | `Interaction` 定义 |
| 可用中文解释强/中/弱 | 每档每规则都有句子，且复述可核对的次数、天数、日期 | `every_explanation_repeats_the_counts_the_user_can_check` |
| 可 orphan（遗忘后可降级） | 纯函数无状态；删证据重跑只会降档 | `forgetting_the_evidence_can_only_lower_a_band` |
| Linux 可测、无真实时间依赖 | `now` 是参数，fixture 自带 now，包内不读时钟 | `TEST_LOG.txt` |
| 无 unsafe / 无网络 / 轻依赖 | `#![forbid(unsafe_code)]`，零依赖 | `cargo tree` 只有一行 |
| O(n) | 单次扫描 + `BTreeSet` 去重；全图一次分组 | `ego_network_scoring_matches_per_peer_scoring` |

## 7. 文件清单

```
soul-algo-tie/
  Cargo.toml              零依赖，edition 2021，rust-version 1.83
  src/lib.rs              TieAlgo 枚举、score_all、score_ego_network、crate 级不变量文档
  src/types.rs            Band / Interaction / TieScore / TieAlgorithm / Tally / epoch 日期工具
  src/t0.rs               基线（Goal 1 直译）
  src/t1.rs               指数衰减
  src/t2.rs               RFM 三档
  src/t3.rs               Granovetter-span（含 GROUP_ONLY_CEILING）
  src/testing.rs          共享确定性 fixture，公开给其它槽位复用
  examples/matrix.rs      打印上面那张对照表和全部中文解释
  tests/comparison.rs     四规则同证据对照 + 稳健性（时区、纪元前、刷屏、乱序、遗忘）
  tests/goal1_fidelity.rs 与 Goal 1 字符串版规则的 6500 次随机比对
  tests/explain_zh.rs     解释语言的性质测试
TEST_LOG.txt              rustc/cargo 版本、cargo tree、fmt、clippy -D warnings、cargo test、matrix 输出
REPORT.md                 本文件
```
