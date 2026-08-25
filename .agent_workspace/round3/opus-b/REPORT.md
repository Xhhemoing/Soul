MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 3 / opus-b — 冻结 A0 + A2，A1 降为「已写下但不采纳」

产出：`.agent_workspace/round3/opus-b/soul-algo-trait/`（v0.3.0，edition 2021，rust-version 1.83，**零依赖**，`unsafe_code = "forbid"`，`#![deny(missing_docs)]`）。
108 测全过（Round 2 是 81），`cargo fmt --check` 与 `cargo clippy --all-targets -- -D warnings` 干净，`cargo doc --no-deps` 在 `deny(missing_docs)` 下可建。逐条见 `TEST_LOG.txt`。
未做 `git commit`（按 `SHARED_BRIEF.md` 派发要求，由父代理合并）。

## 0. 结论

1. **A0 是保留名额 #2，本轮冻结，行为一字未改。** `A0_ALGORITHM_ID` 停在 `a0.questionnaire_correction_lock.v2`，默认 write mode 停在 `LastWriteWins`，Round 2 的 intake 补锁在冻结范围之内。三条都由 `tests/frozen_defaults.rs` 钉住，改动必须是一次自觉的解冻。
2. **A1 不是保留算法。** 它没有独立数据面、没有独立输出类型、不能脱离 A0 存在，且在 v0.1 可产生的证据上与 A0 逐格相同（`a1_is_indistinguishable_from_a0_on_v01_data`）。它以「已写下、已测、未接线」的形态留在包里：`a1_is_not_on_the_retained_path` 读 `src/a0.rs` 源码，A0 一旦 import a1 就红。
3. **A1 默认独立性改为 `TwoKindsAcrossDays`，`EMPTY_ON_V01 = true`。** Round 2 报告里那个「换个日子把同一份问卷再答一遍就升 Strong」的退化触发，本轮关掉。规范原样的 `KindAndDay` 作为**具名备选**保留在枚举上，不点名就到不了。
4. **A2 冻结为渲染器，只做加性扩展：** `TieScore` 多两个 `Option<u32>` 字段 `direct_count` / `group_count`。给了就说，没给就不说，`a2.rs` 里依旧没有任何 `STRONG_MIN`。
5. **A3 维持击毙。** 6 测未动。
6. **两条本轮才浮出来的事实**，都是我先把自认为成立的性质写成断言、断言当场红了才发现的，见 §6。两条都按实际情况留档，没有改成能过的说法。

「保留 1 或 2 个」的实际形状：**一个人脉算法（T4/T4D，另一个包）+ A0**。A2 是 A0 这一侧的渲染器，不占名额；把渲染器凑成第二个算法是算术不是选型。

## 1. A0 — 冻结成什么样

### 1.1 冻结清单

| 冻结项 | 值 | 钉住它的测试 |
|---|---|---|
| 算法标识 | `a0.questionnaire_correction_lock.v2`（未动） | `the_a0_identifier_did_not_move` |
| 默认 write mode | `LastWriteWins` | `the_frozen_write_mode_is_last_write_wins` |
| intake 尊重锁 | 是，且被挡答案进 `ignored`、证据行照写 | `the_intake_patch_is_part_of_the_freeze` |
| `Strong` 的含义 | 用户纠正过（在 v0.1 数据面上） | `strong_on_the_retained_path_means_the_user_said_so` |
| 不接 A1 | 源码级 | `a1_is_not_on_the_retained_path` |

标识**不**升版，是有意的：本轮没有改 A0 的任何一个判断，Round 2 包盖的章和这个包盖的章必须表示同一件事。升版等于宣称它们不同。

### 1.2 `LastWriteWins` 为什么是冻结的默认（Round 3 裁决）

Round 2 把这条留给 Round 3 裁。裁决是**保持 `LastWriteWins`**，理由不是「它更好」，而是：

> 它允许的那次覆盖，在 v0.1 的数据面上**到不了**。

覆盖需要一条「非纠正、且与当前立场相反」的行。v0.1 对一条轴只有两个写入者：问卷（`Questionnaire`）与纠正（`UserCorrection`）。纠正会先锁轴；第二次问卷答案替换第一次，那是问卷本来就该有的行为，不是降档。所以在全部夹具限制到 v0.1 可达行之后，两种 mode 的 **position / band / locked_by_user 完全一致**（`the_frozen_write_mode_decides_nothing_a_v01_user_is_told`）。

既然用户看得到的三样东西都不变，换默认就只剩下一个效果：**改掉 Goal 1 存量被对照的那条基线**。而基线不动正是冻结要保的东西。

`NoDowngrade` 因此保留为具名、已测的 opt-in，留给真正开始往轴上写行为证据的那个版本——那个版本才需要它。

**但有一处不一致，见 §6.1：两种 mode 引用的证据行不同。** 这不构成解冻理由，但它是一条独立的产品问题，我把它单独列成测试而不是塞在通过的断言里。

### 1.3 遗忘 / 锁 / 重放，未变

- `forgotten` 行在任何聚合之前丢弃；遗忘掉纠正后轴自己退回问卷答案，锁一并消失。
- `apply_intake` 与 `a0_all_axes_with(log_after, mode)` 在两种 mode × 全部 15 个夹具上全等（`intake_matches_replay_on_every_fixture_and_mode`）。这条等式是 Goal 1 的 `intake` 破掉的那条，补丁的正确性判据就是它重新成立。
- 一个轴锁住不冻结其他轴；后来的纠正仍能改锁定轴。

## 2. A1 — 不是保留算法，但默认改了

### 2.1 为什么不保留

`R2-SYNTHESIS.md` 的保留名额是「一个人脉算法 + A0」。A1 不在其中，本轮把这个判断从「Round 2 的自述」升级成**跑出来的证据**：

`a1_is_indistinguishable_from_a0_on_v01_data`：把全部 15 个夹具按 `reachable_for_axis_in_v01()` 过滤成 v0.1 真能产出的行，A1 默认变体与冻结的 A0 默认在**五条轴 × 全部夹具**上 position / band / locked 三项逐格相同。

一个在现有数据面上与基线无法区分的候选，还没有开始承担自己的重量。C8（创新必要）要求新机制必须打败基线——A1 目前打成平手，且平手是**设计使然**（见 §2.2），不是实现不到位。

它仍然留在包里，因为删掉它的代价更大：v0.2 一旦接入行为证据，「多条互相独立的证据互相印证」这件事需要一个出口，而这条规则的难点全在「什么叫独立」，那是最容易被下一个人凭记忆重新发明成「数条数」的地方（Round 1 的 A1 就是那样，被 F16 打挂）。写下来、测好、不接线，比写在文档里可靠。

**接线防护是源码级的**：`a1_is_not_on_the_retained_path` 逐行读 `src/a0.rs`，跳过注释，命中 `a1::` 或 `crate::a1` 即失败。负控做过：给 a0.rs 加一个调 `a1_axis_state` 的函数，测试如期红（`TEST_LOG.txt` 负控 b）。

### 2.2 默认改为 `TwoKindsAcrossDays`

**改的是什么：** 键仍是 `(kind, utc_day)`，只多一条——**升档所依据的那几组至少跨 2 种 kind**。

**为什么：** Round 2 报的 `EMPTY_ON_V01 = false` 场景是

> 同一条轴，用户在三个不同 UTC 日重填问卷，三次同方向，未被纠正 → 3 组 ≥ 3 → `Moderate` 升 `Strong`。

独立的是**仪器**，不是日历。同一个人、同一份问卷、同一段记忆，礼拜二答一遍三月再答一遍，是一件事被记了三次。而 `Strong` 在这个产品里的含义是「用户看着结论纠正过」或「几件互相独立的事互相印证」，不是「用户说了三遍」。

**代价：零。** `the_default_still_passes_f16b`：问卷 + 三个不同日的行为证据仍升 `Strong`（问卷 + message = 2 种 kind）。生存线没动。

**规则读作「两种仪器」，不是「不许是问卷」。** `the_default_does_not_upgrade_three_same_kind_days_of_behaviour_either`：三条不同日的 message 推断（无问卷）同样不升。这条测试的作用是防止规则被读成一条针对 intake 的特例。

**`EMPTY_ON_V01 = true` 是推导出来的，不是断言的。** `a1_can_fire_on_v01_data(independence)` 从 `EvidenceKind::reachable_for_axis_in_v01()` 出发，扣掉 `UserCorrection`（它一出现就先锁轴、根本轮不到计数），剩下的 kind 数与变体要求比较。多出第三个 v0.1 写入者时，这个函数会翻转，而不是让文档里某句话悄悄失效。`empty_on_v01_is_true_under_the_default_and_false_under_the_alternative` 同时校验常量与推导一致。

行为侧还有一条：`no_v01_reachable_log_reaches_strong_without_a_correction` —— 全夹具限制到 v0.1 可达行后，A1 剩下的 `Strong` 全部是锁定的。

### 2.3 `KindAndDay` 作为具名备选保留

它是 `CANDIDATE_SPEC.md` 的字面读法，也是 Round 2 的默认。保留在枚举上、有 `as_str()`、在消融表里有自己一列、`the_named_alternative_still_upgrades_three_same_kind_days` 钉住它仍会升。**不点名就到不了**：`a1_axis_state` 走 `A1_DEFAULT_INDEPENDENCE`。

这样与规范的分歧是一个读者能指着看的值，不是一个被删掉的分支。

### 2.4 新增一条理由：`NeedsASecondSource`

改默认之后，「3 组但都同源」原本会落进 `NotEnoughIndependentGroups { groups: 3 }`，解释句读作「目前只有 3 组独立依据，还不够升为强」。那是**错的建议**——再答第四遍也没用。

所以拆出 `A1Reason::NeedsASecondSource`，中文说：

> 「这个方向有 3 组依据，但都来自同一种来源；换个日子把同一份问卷再答一遍不算新的旁证，所以先留在中等。」

`the_refusal_explanation_does_not_ask_for_more_of_the_same` 断言这句出现、且「还不够升为强」那句**不**出现。

### 2.5 标识升到 v3

`a1.independent_group_upgrade.v3`。默认变体变了，同一份证据日志产出的结果就变了；盖章的 state 得说清是哪条规则盖的。

## 3. A2 — 冻结为渲染器 + 两个可选字段

### 3.1 依然没有第二套阈值

`a2.rs` 里没有 `MODERATE_MIN_INTERACTIONS` / `STRONG_MIN_INTERACTIONS` / `STRONG_MIN_ACTIVE_DAYS`，band 从 `TieScore::band` 抄下来，每条 bullet 携带的 band 就是边的 band。守卫有两处（`a2_defines_no_band_thresholds`、`the_a2_source_still_holds_no_threshold`），都用 `include_str!` 读源文件。负控做过，两处同时红（`TEST_LOG.txt` 负控 a）。

放两处是因为它现在承担两件事：A2 自己的性质，以及「保留的这一对里只有一个地方决定 band」这条冻结条件。

### 3.2 `direct_count` / `group_count`

`R2-SYNTHESIS.md` 边界风险 2 要求把 Strong/Moderate 的次数与天数门改到 direct 计数上（T4D）。若 T4D 落地，**门槛作用在哪个数上，用户就得看到哪个数**，否则「你们一共 137 次往来」在一条 135 次群消息的边上根本没法复核。

所以 `TieScore` 加两个 `Option<u32>`，A2 多一条模板：

> 「其中一对一往来 2 次，群里同场 135 次。」

四条硬规矩，各有测试：

| 规矩 | 测试 |
|---|---|
| 两个字段**都**在才出这句 | `no_split_no_sentence`（`half_a_split_is_not_a_split` 夹具：只给一半 → 不出句）|
| 不碰 band，四个 band 全验 | `the_split_never_touches_the_band` |
| 不自己推导缺的那一半 | 同上；`venue_split()` 是 `Some((d, g))` 或 `None`，没有减法 |
| 与群聊句是两句话，各靠各的字段 | `the_split_and_the_group_only_flag_are_separate_sentences` |

「半个 split 不是 split」这条值得说明：给了 direct 没给 group，读者会自己去减 `interaction_count`，而那个减法的结果是 **A2 编出来的数**。缺一半就整句不出。

`direct_count: Some(0)` 与 `any_direct: true` 同时出现是输入内部打架。A2 **照实渲染两个字段，不裁决**——在自己的输入的两个字段之间选边，正是这个模块被拿掉的那种「第二意见」。

### 3.3 新增 `A2_STATEMENT_KEYS`

9 个 key，A2 的全部输出面，做成数据。`every_statement_key_is_reachable_and_listed` 双向校验：不能发出未登记的 key，也不能登记一个没有夹具能产出的 key。后半条是给 §5 的中文扫描用的。

### 3.4 标识升到 v3

`a2.tie_summary_renderer.v3`。渲染规则没变，但存量摘要可能包含的句子集合变了，读旧行的消费者有权分辨两批。

## 4. A3 — 维持击毙

`a3_from_message_text` / `a3_from_messages` 恒返回 `Err(A3Refused)`，理由字符串稳定且不含任何正文。`A3_ALGORITHM_ID` 注册为 rejected。本轮**零改动**——这个模块的价值就在于它不动。

## 5. 中文模板扫描：补了一个洞

Round 2 的扫描走**夹具**：渲染全部夹具，扫出来的每句话过三张表。这挡得住「有人加了一句话」，挡不住「有人加了一个模板但没给夹具」——那句中文就一路裸奔上线。

本轮补两条结构性测试：

- **`every_chinese_template_this_crate_ships_is_screened`** 走**输出面**而不是夹具：`A2_STATEMENT_KEYS` 里每个 key 都必须有夹具能渲染出来，`A1Reason` 的六个变体也都必须有夹具能触发。登记了却没人渲染 = 失败。
- **`the_chinese_denylist_catches_a_diagnostic_word_inside_a_real_template`**：把临床词塞进本 crate **真会发出的句型**里（「归在「强」一档，看得出焦虑倾向」「可以诊断为外向」「属于回避型人格障碍」「量表得分偏高」），确认四句全被拦。词表只在裸词上生效是没用的。
- **`the_venue_split_sentence_is_publishable_about_a_third_party`**：Round 3 唯一新增的模板，三张表全过（含第三人表——它描述的是一个从未同意被刻画的人）。

十词清单一字未改，`the_brief_s_denylist_is_the_denylist` 仍逐字比对。

## 6. 两条本轮浮出来的事实

两条都是：我把自认为成立的性质写成断言 → 第一次跑就红 → 按实际情况留档。没有一条被改成「能过的说法」。

### 6.1 两种 write mode 在 v0.1 数据上**不是**完全等价的

我断言的是「全等」，实际是 **position / band / locked 全等，`evidence_ids` 不等**。

同一天重填 5 次问卷：

| mode | `evidence_ids` |
|---|---|
| `LastWriteWins`（冻结默认） | `[5]` |
| `NoDowngrade` | `[1, 2, 3, 4, 5]` |

轴读起来一样，所以不构成解冻理由。但 SHARED_BRIEF 的 C3 是**「用户能复核计数」**，而用户问「这条是根据什么」时，冻结的这一档给出的答案更短——它只引用最后一条，前四条用户自己的答案不在引用列表里。

处理：拆成两条测试（`the_frozen_write_mode_decides_nothing_a_v01_user_is_told` 说清楚等的是哪三项，`the_two_write_modes_do_differ_in_what_they_cite` 单独把差异摆出来），`src/a0.rs` 的模块文档同步改正。**交 Round X 裁**：这是引用完整性问题，不是档位问题，我倾向于它值得单独处理而不是靠换 mode 顺手解决。

### 6.2 「只有纠正能到 Strong」是**数据面**的性质，不是 A0 代码的性质

A0 读 `EvidenceRef::band` 且从不封顶。一个写入者若产出一条自称 `Strong` 的行，A0 就给 `Strong`，不需要任何纠正；`NoDowngrade` 下后来一条同方向的问卷答案还拉不回来（同方向立场的 band 取最强单条）。

在 v0.1 到不了——`record_axis_inference` 在 Goal 1 里没有生产调用方，这正是 `only_two_evidence_kinds_can_reach_an_axis_in_v01` 钉的那条事实——所以保证今天成立。但它成立的原因不在 a0.rs 里。

处理：`strong_on_the_retained_path_means_the_user_said_so` 收窄到 v0.1 数据面（它在那里为真，且全夹具验证）；`a0_trusts_the_band_on_a_row_it_is_handed` 把缺口本身钉住，写法与 `last_write_wins_is_the_a0_contract` 同一形状——**记录缺陷，不是背书**。

**没有修**，因为 A0 已冻结而修法是行为变更。**交给开始往轴上写行为证据的那个版本**，届时二选一：把非纠正行封顶 `Moderate`，或者采纳 A1。这条已经在 `src/a0.rs` 模块文档里写明。

## 7. 消融表（由 `tests/a0_vs_a1.rs::the_ablation_table_is_what_the_report_says_it_is` 跑出，不是誊抄）

列 1 与列 4 是**本轮出货的默认**；列 2、列 3 是具名备选。`the_default_columns_are_the_shipped_defaults` 保证这个对应关系不会在默认漂移之后还留着。

| 夹具 | **A0 LWW（默认）** | A0 NoDowngrade | A1 KindAndDay | **A1 TwoKinds（默认）** |
|---|---|---|---|---|
| `questionnaire_only` | Moderate | Moderate | Moderate | Moderate |
| `questionnaire_then_correction` | Strong 🔒 | Strong 🔒 | Strong 🔒 | Strong 🔒 |
| `questionnaire_refilled_after_correction` | Strong 🔒 | Strong 🔒 | Strong 🔒 | Strong 🔒 |
| `f16_five_refills_same_day` | Moderate | Moderate | Moderate | Moderate |
| `f16b_..._three_independent_days` | Weak | Moderate | Strong | **Strong** ← 生存线未动 |
| `three_questionnaire_refills_on_three_days` | Moderate | Moderate | Strong ⚠ | **Moderate** ← 本轮关掉的退化 |
| `three_message_days_same_kind`（新） | Weak | Weak | Strong ⚠ | **Weak** ← 同源即不升，与问卷一视同仁 |
| `weak_inference_over_moderate_questionnaire` | Weak ⚠ | Moderate | Weak(Mixed) | Weak(Mixed) |
| `contradiction` | Moderate | Moderate | Weak(Mixed) | Weak(Mixed) |

🔒 = 锁定，四种读法完全一致（`neither_algorithm_ever_moves_a_locked_axis` 全夹具验证）。
⚠ 三处：两处是 `KindAndDay` 的退化升档（本轮已不是默认）；`A0 LWW` 那一格是 §1.2 的后写覆盖契约，在 v0.1 数据面上不可达。

## 8. 评分（1–5，相对 Round 2 的变动加粗）

| | C1 产品锁 | C2 可测 | C3 可解释 | C4 隐私 | C5 稳健 | C6 成本 | C7 SOTA | C8 创新必要 |
|---|---|---|---|---|---|---|---|---|
| **A0（保留 #2）** | 5 | 5 | **4**（§6.1 引用完整性） | 5 | 5 | 5 | 4 | — 基底 |
| **A2（渲染器）** | 5 | 5 | 5 | 5 | 5 | 5 | — | — 不参与 |
| A1（不保留） | 4 | 5 | **5**（新增 `NeedsASecondSource` 后不再给错建议） | 5 | **5**（退化触发已关） | 5 | 4 | **2**（v0.1 上与基线逐格相同）|
| A3 | 2 | 2 | 1 | 1 | 1 | 3 | 4 | 1 → 维持击毙 |

A0 的 C3 从 5 降到 4，是 §6.1 的直接后果：档位可复核，**引用列表在冻结 mode 下不完整**。这是本轮唯一一处降分，降的原因是本轮才发现，不是本轮才发生。

A1 的 C8 从 3 降到 2：Round 2 记的是「唯一的非纠正升档路径」，本轮跑出来的是「在 v0.1 数据面上与 A0 无法区分」。两句话不矛盾——它的价值全部在 v0.2，而 C8 问的是现在。这就是它不占名额的量化理由。

C6 说明未变：A0 每轴一次线性扫；A1 多一次分组，用 `Vec` 线性查找而非哈希表（证据行是人类量级，常数因子更小，且没有哈希就没有迭代顺序不确定性）。全程无 O(n²)。

## 9. 硬约束自检

| 约束 | 本包状态 |
|---|---|
| 推断必须有可解引用 `evidence_ids` | `neither_algorithm_ever_claims_without_citing`、`every_bullet_cites_something`、`a2_is_a_function_of_the_band_it_is_handed`；空证据 → 零 bullet |
| 禁 score / percentile / 诊断词 | 三张表、两个入口、14 测；新增的输出面扫描堵住「有模板无夹具」 |
| 第三人数据不出本机 | A0/A1/A2 输入里没有正文、姓名、可解析到人的标识；新增的两个字段是计数；A3 拒绝 |
| 可向用户解释 | A1 六种理由各有中文句、全部可手数；A2 每句都是计数复述；无浮点、无 `exp`、无权重。**例外已登记：§6.1** |
| v0.1 无 E0、无 key 不依赖 LLM | 零依赖、零网络、零 IO（`cargo tree` 只有自己）|
| 不承诺临床效度，可纠正可遗忘 | 锁两条路径成立且已冻结；遗忘后重放全夹具回落 |
| Linux 可测，不依赖 Windows API | 纯 std |
| 确定性 | 无墙钟、无随机、无哈希容器、无并行归约；迭代顺序即输入顺序 |
| 无 `unsafe` | `#![forbid(unsafe_code)]`；全 crate 里 `unsafe` 一词只出现在这一行 |

## 10. 交给父代理的四件事

1. **A2 的 `direct_count` / `group_count` 依赖 T4D 是否落地。** 若 T 族胜者不带 venue 拆分，这两个字段永远是 `None`、那条模板永远不出，**不需要任何改动**；`no_split_no_sentence` 就是这个状态的测试。反过来若 T4D 落地，A2 已经能消费。**不要**让 A2 自己回证据库补这个拆分。
2. **§6.1 的引用完整性**（冻结 mode 只引用最后一条）交 Round X。这是 C3 问题，不是档位问题，我不建议靠换默认 mode 顺手解决。
3. **§6.2 的 band 封顶缺口**交给第一个往轴上写行为证据的版本，二选一：非纠正行封顶 `Moderate`，或采纳 A1。已写进 `src/a0.rs` 模块文档，不会丢。
4. **`ALGO_FROZEN` 声明与墓碑**：本包这一侧的墓碑内容是 A3（已在代码里拒绝并注册）与 A1（不保留，但**保留在包内不接线**，理由 §2.1）。请注意 A1 的处置与 T0/T1/T2/T3R 不同——那几个是删除归档，A1 是「写下来但不接线」，因为它的难点（什么叫独立）最容易被下一个人重新发明错。
