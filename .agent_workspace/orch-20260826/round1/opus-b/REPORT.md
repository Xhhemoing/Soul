MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 1 / opus-b：核心实现审计（soul-algo-trait + schemas）

审计人：Parent Orchestrator Round 1 opus-b 子代理。核于 2026-08-26，树 = `main` @ `a0ec14b`，工作分支 `cursor/round1-opus-b-core-impl-audit-ab42`。

**纪律遵守声明**：本轮**未改动** `docs/**` 与 `crates/**` 任何一个字节（`git diff -- crates docs` 为空）。全部新增文件都在 `.agent_workspace/orch-20260826/round1/opus-b/` 下，且都是探针与证据，不是权威面。COPY_ZH 漂移探针以 path 依赖从 `/tmp/copy_probe` 引用 crate，不在 crate 内新增测试。

---

## 1. 结论摘要

四项算法义务（A0 锁、A1 空转、A2 纯渲染、A3 拒诊）与 denylist 在 `soul-algo-trait` 里**都实现了，而且都被测试钉住了，钉法的质量高于我预期**：A1 的空转不是断言而是对 346,200 条 v0.1 可达日志的穷举，A2 的「无第二套阈值」既有源码扫描也有 7,168 组计数扫描的行为对照，A0 的 intake 补丁与 replay 全等有交叉测试。114 个测试全绿，clippy `-D warnings` 与 `rustfmt` 干净。

`docs/schemas/` 的 11 份 schema 与 `schemas.lock.json` 的 SHA-256 **逐份吻合**，`tie_strength` 的类型化（D58/D59）经 24 条探针实测，**D59 承诺的每一条都成立**：空对象合法、`algorithm_id` 触发 15 字段整包必填、`machine_band`/`user_band`/`locked_by_user` 都放行，而且分列计数带了一个我没在决议里读到、但设计得很好的棘轮（写 `direct_out_count` 会强制补 `algorithm_id`，进而强制补全整包）。

问题集中在**三个接缝**，都不在算法内部，而在算法与它的两个权威面之间：

1. **冻结话术 `COPY_ZH.md` §4 与 `a2.rs` 实际渲染是两套文案。** D40 把这条记作「COPY_ZH 与 crate 文案漂移另记」，但全仓库没有任何一处「另记」了它。本报告第 5 节把漂移逐句量化：六类句里只有 Round 3 新增的 P1b 逐字一致，其余五类全部不同，其中「档位词 moderate 渲染成『中』而非冻结的『中等』」直接违反 COPY_ZH §0.1 词汇单源与 §5 断言 5。
2. **D52 要求的「两侧天数常量用工作区测试钉相等」这个测试不存在。** 两个 crate 互不依赖，`DEMOTE_ONE_BAND_DAYS = 180`（tie 侧）与 `DORMANT_AFTER_DAYS = 180`（trait 侧）是两处彼此不知情的字面量，DECISION.md §3 明写这两者「不得分裂」。
3. **`schemas.lock.json` 在本树没有校验器。** 它自称「`--check` runs in CI」，但本树既无 `xtask` crate 也无 `.github/`。我手算了 11 份 SHA-256 全部吻合，但这个吻合今天靠的是没人动过文件，不是靠一道门。

没有发现任何一处「算法逻辑与冻结决议相悖」。也没有发现任何一处需要改 crate 逻辑才能修的缺陷——上面三条都是**补测试、补映射规范、补门**，不是改判档。

---

## 2. 对照表 A：冻结条款 × crate 实现 × 钉住它的测试

「依据」列指 `docs/algorithms/DECISION.md`（记作 §n）与 `docs/DECISIONS.md`（记作 Dn）。

### 2.1 A0 — 问卷 + 本人纠正锁定

| # | 冻结条款 | 依据 | 实现位置 | 钉住它的测试 | 判 |
|---|---|---|---|---|---|
| A0-1 | A0 是保留算法 #2，政策基底 | §2.2 | `a0.rs` 全模块 | `the_a0_identifier_did_not_move` 遍历 15 条夹具校验每个轴的 `algorithm_id` | ✅ |
| A0-2 | 算法 id 不动（Round 3 冻结未改任何决策） | §2.2 | `A0_ALGORITHM_ID = "a0.questionnaire_correction_lock.v2"` | `the_a0_identifier_did_not_move` | ✅ |
| A0-3 | 纠正即锁定，之后推断只记录不改动 | §2.2 / D32 | `a0.rs::place` 首个分支 `state.locked_by_user && !row.locked_by_user → RefusedLocked` | `an_inference_never_moves_a_locked_axis_but_is_still_returned`、`no_downgrade_never_overrides_the_lock` | ✅ |
| A0-4 | **intake 不绕锁**（Round 2 补丁） | §2.2 | `apply_intake` 与 replay 共用同一个 `place()`，不是两份逻辑 | `intake_does_not_move_an_axis_the_user_corrected`、`the_intake_patch_is_part_of_the_freeze` | ✅ |
| A0-5 | **`apply_intake` 与 replay 全等** | §2.2、AC-31 | 同上，单点 `place()` 是结构性保证而非巧合 | `intake_matches_replay_on_every_fixture_and_mode`（夹具 × 两种 WriteMode 全组合） | ✅ |
| A0-6 | 被拒答案「已跳过 + 原因」如实回报，不静默丢弃 | AC-31 / D39 | `IntakeReport::ignored`（`IgnoredAnswer { evidence_id, axis, position, reason }`） | `a_skipped_answer_is_reported_rather_than_dropped` | ✅ |
| A0-7 | 答案行照常落库（锁只挡轴移动，不挡证据表增长） | §2.2 | `apply_intake` 无条件 `log_after.push(row)` | `the_answer_row_is_still_written_even_when_the_axis_will_not_move` | ✅ |
| A0-8 | 其余轴不受影响 | AC-31 | `place` 按 axis 过滤 | `a_lock_on_one_axis_does_not_freeze_the_others` | ✅ |
| A0-9 | 遗忘可重算（drop 行 + replay 即回落） | 产品锁 | 三处算法统一在聚合前 `filter(!row.forgotten)` | `forgotten_rows_are_dropped_before_anything_is_aggregated`、`a_fully_forgotten_axis_returns_to_unknown` | ✅ |
| A0-10 | 默认写模式冻结为 `LastWriteWins` | §2.2 | `A0_DEFAULT_WRITE_MODE` | `the_frozen_write_mode_is_last_write_wins` | ✅ |
| A0-11 | v0.1 下 `Strong` ⟺ 用户说了算 | §2.2 | 无升档规则，`a0.rs` 不 import `a1` | `strong_on_the_retained_path_means_the_user_said_so`、`on_the_retained_path_only_a_correction_reaches_strong` | ✅ |
| A0-12 | A0 不读时钟 | §6.3 | `IntakeAnswer.recorded_at_unix` 只写进行，`place` 从不读 | `a0_ignores_the_timestamp_entirely` | ✅ |

**加分项，不是缺陷，但值得父代理知道**：crate 自己把两处「已知代价」写成了通过的测试而不是藏起来——`the_two_write_modes_do_differ_in_what_they_cite`（冻结模式只引最新一行，`NoDowngrade` 引全部五行，「用户能复核计数」这条标准上冻结模式给的答案更短）与 `a0_trusts_the_band_on_a_row_it_is_handed`（A0 读 `EvidenceRef::band` 且从不封顶，所以「只有纠正能到 Strong」是 **v0.1 数据面的性质，不是 A0 代码的性质**；v0.2 开始写行为行时必须二选一：给非纠正行封顶 Moderate，或者启用 A1）。这两条是真实的、有边界的技术债，已如实入档，符合 §4「如实入档，禁止静默修补」的纪律。

### 2.2 A1 — 空转是预期行为

| # | 冻结条款 | 依据 | 实现位置 | 钉住它的测试 | 判 |
|---|---|---|---|---|---|
| A1-1 | 默认 `TwoKindsAcrossDays` | §2.4 | `A1_DEFAULT_INDEPENDENCE` | `the_default_is_two_kinds_across_days` | ✅ |
| A1-2 | **v0.1 空转是预期行为，不是缺陷** | §2.4 | `A1_EMPTY_ON_V01 = true` | 见下三层 | ✅ |
| A1-3 | 空转有推导，不是断言 | §2.4 | `a1_can_fire_on_v01_data()` 从 `EvidenceKind::reachable_for_axis_in_v01` 数可用来源，不复述结论 | `the_a1_identifier_moved_because_its_default_did` 把常量与推导钉等 | ✅ |
| A1-4 | 空转有穷举，不只是推导 | Round X | `roundx_a1_empty_on_v01.rs` | `the_upgrade_branch_is_unreachable_on_every_v01_log_up_to_four_rows`：**24 + 576 + 13,824 + 331,776 = 346,200 条**长度 ≤4 的 v0.1 日志全走一遍 | ✅ |
| A1-5 | 空转不是「日子不够」造成的 | §2.4 | — | `more_questionnaire_days_never_help_however_many_there_are`：6 天 × 4 位置 = **4,096 条**日志，全部不升档 | ✅ |
| A1-6 | 空转不是空洞命题（换变体就会升档） | §2.4 | `A1Independence::KindAndDay` 保留为具名备选 | `the_constant_the_derivation_and_the_run_are_all_the_same_claim` 给出反例 | ✅ |
| A1-7 | 杜绝「问卷连填三天误升 Strong」 | §2.4 | `upgrades()` 要求 `distinct_kinds >= 2` | `the_default_does_not_upgrade_three_same_kind_days`、`the_named_alternative_still_upgrades_three_same_kind_days` | ✅ |
| A1-8 | A1 不在保留路径上，A0 不得调用它 | §2.4 | — | `a1_is_not_on_the_retained_path` 逐行扫 `a0.rs` 源码（跳过注释行）禁 `a1::` / `crate::a1` | ✅ |
| A1-9 | 规则变了 id 必须变 | §2.4 | `A1_ALGORITHM_ID = ...v3` | `the_a1_identifier_moved_because_its_default_did` | ✅ |
| A1-10 | F16（同一下午五次重填）不升档 | R1/R2 | `(kind, utc_day)` 分组 | `f16_five_refills_in_one_afternoon_do_not_upgrade` | ✅ |
| A1-11 | F16b（问卷 + 三天行为行）仍升档 | R2 | 两类来源满足 | `f16b_three_independent_agreeing_groups_reach_strong`、`the_default_still_passes_f16b` | ✅ |

**一处比冻结规范更严，crate 自己声明了**：`A1_GROUPS_FOR_DISAGREEMENT = 1`，即一边一组就判 `Mixed`，而 `CANDIDATE_SPEC.md` 冻结的是两组一边。crate 在 `a1.rs` 模块文档与常量文档里两处明写这条分歧并给了理由（「按条数把用户投出去就是把数字藏起来的分数」）。因为 A1 不在保留路径上，这条分歧对 v0.1 无外部效果；但它是 v0.2 启用 A1 时必须先裁决的一条，**不应该在合并 `crates/soul-algo` 时被顺手抹平**。

### 2.3 A2 — 纯渲染器

| # | 冻结条款 | 依据 | 实现位置 | 钉住它的测试 | 判 |
|---|---|---|---|---|---|
| A2-1 | A2 是 band 的纯消费者，**源码级禁止第二套阈值** | §2.3 | `a2.rs` 无任何判档常量 | `a2_defines_no_band_thresholds` + `the_a2_source_still_holds_no_threshold`：`include_str!` 读**编译进去的源文件**，是关于「发布出去那份文件」的事实 | ⚠️ 见 5.6 |
| A2-2 | 同上的行为面证明 | Round X | — | `no_count_on_either_side_of_a_gate_moves_the_band`：8×8×7×2×2×4 = **7,168 组**计数扫描，跨越 0/3/10 三个门的两侧，band 进什么出什么 | ✅ |
| A2-3 | band 不得决定「出现哪些句子」 | §2.3 | 每个 bullet 生成器只读计数 | `the_band_never_decides_which_sentences_appear`（四档 key 列表全等）、`the_band_changes_exactly_one_sentence_and_it_is_the_filing_line` | ✅ |
| A2-4 | 新增一对一/群聊分列句 | §2.3 | `venue_split_bullet` | `the_venue_split_is_rendered_when_the_scorer_supplies_it` | ✅ |
| A2-5 | **禁止自行推导分列** | §2.3、D33、AC-33 | `venue_split()` 用 `Some((self.direct_count?, self.group_count?))`，半个分列 = 没有分列，不做减法 | `no_split_no_sentence`（`half_a_split_is_not_a_split` 夹具） | ✅ |
| A2-6 | 分列不得碰 band | §2.3 | — | `the_split_never_touches_the_band`（四档 × 137 里只有 2 次一对一这个最诱人降档的形状） | ✅ |
| A2-7 | 分列句与「仅群聊」句是两句，读两组不同字段 | AC-33 | `venue_split_bullet` 读 `direct_count/group_count`，`venue_bullet` 读 `any_direct` | `the_split_and_the_group_only_flag_are_separate_sentences` | ✅ |
| A2-8 | 近因句阈值闭区间 `>=`，与降档同一常量 | §3、D60 | `days >= DORMANT_AFTER_DAYS` | `the_dormancy_threshold_is_closed_at_the_constant`（180 命中、179 不命中） | ⚠️ 常量对，但不是「同一个」，见 5.2 |
| A2-9 | 沉寂句是描述不是降档 | §3 | `dormancy_bullet` 用 `score.band` 原样 | `dormancy_is_a_sentence_not_a_demotion` | ✅ |
| A2-10 | 不读墙钟，`as_of` 随输入 | §6.3 | `as_of_unix` 在 `TieScore` 结构体上而非函数签名上——**调用方无法只传一个时钟进来** | `shifting_as_of_and_last_contact_together_changes_nothing`（整体平移一年，输出全等） | ✅ |
| A2-11 | 每句必带 `evidence_ids`，无证据不落库 | 产品锁 | `evidence_ids.is_empty()` → 零 bullet；兜底句是 `nothing_to_say_zh()` **常量**而不是 bullet | `every_bullet_cites_something`、`empty_evidence_means_no_bullets_at_all` | ✅ |
| A2-12 | 近因句只引最近一条证据 | COPY_ZH P4 | `recency_evidence()` 优先 `last_contact_evidence_id` | `the_recency_line_cites_the_one_row_it_rests_on` | ✅ |
| A2-13 | 输出面是数据，新模板加进来就被筛 | — | `A2_STATEMENT_KEYS`（9 个 key） | `every_statement_key_is_reachable_and_listed` 双向闭合：发出的必在表内，表内的必有夹具能发出 | ✅ |
| A2-14 | 渲染确定性 | §6.3 | 无哈希、无随机、迭代序 = 输入序 | `the_same_score_always_renders_the_same_summary` | ✅ |
| A2-15 | 未来时间戳不崩、不假装未来往来 | — | `(as_of - last).max(0)` | `a_last_contact_ahead_of_as_of_reads_as_today` | ✅ |

### 2.4 A3 拒绝 + 拒诊（denylist）

| # | 冻结条款 | 依据 | 实现位置 | 钉住它的测试 | 判 |
|---|---|---|---|---|---|
| A3-1 | A3 被否决，且**在代码里**否决 | R1 / REJECTED.md | `a3_from_message_text` 恒返回 `Err` | `text_inference_always_refuses`（4 段文本 × 5 轴） | ✅ |
| A3-2 | 批量入口不是无守卫路径 | — | `a3_from_messages` 同样恒 `Err` | `the_batch_form_refuses_too` | ✅ |
| A3-3 | 证据再多也买不到文本推断 | — | 签名带 `_evidence_ids` 但不读 | `plenty_of_evidence_does_not_buy_a_text_inference`（1000 条证据） | ✅ |
| A3-4 | 拒绝理由稳定且**不含任何正文** | 产品锁（第三方文本 local_only） | `A3_REFUSAL_REASON` 是 `&'static str`，函数只借用不复制不哈希不记日志 | `the_refusal_reason_is_stable_and_holds_no_text` | ✅ |
| A3-5 | 被否决的 id 注册在案，写边界可拒 | — | `is_rejected_algorithm()` | `the_a3_identifier_is_registered_as_rejected`、`no_live_algorithm_stamps_the_rejected_identifier` | ⚠️ schema 侧无落点，见 4.4 |
| A3-6 | A0/A1/A2 从不接收消息正文 | §2 | 三者签名里没有 `&str` 正文参数 | 类型层面成立（无测试也无法违反） | ✅ |
| DL-1 | 不给精神疾病诊断（D5） | D5 | `DIAGNOSTIC_DENYLIST`（10 词） | `the_brief_s_denylist_is_the_denylist` 逐字钉表；`the_diagnostic_screen_catches_what_it_is_for` 逐词造句 | ✅ |
| DL-2 | 禁 score / percentile（D22） | D22 | `NUMERIC_RATING_MARKERS`（10 词） | `a_rating_is_caught_even_when_it_is_dressed_as_a_count` | ⚠️ 词表不全，见 5.4 |
| DL-3 | 「往来 12 次」这类原始计数**不是**评分 | D22 | 两张表分开 | `a_plain_count_is_not_a_rating` | ✅ |
| DL-4 | 不替第三方声称关系/性格 | 产品锁 | `PEER_CLAIM_DENYLIST`（14 词，含「强关系」「弱关系」） | `no_band_ever_claims_a_relationship`（15 夹具 × 4 档全渲染） | ✅ |
| DL-5 | 「强」是档位词，「强关系」是关系声称，二者必须分开 | §2.3 | `Band::label_zh()` 只给单字 | `a_weak_band_never_reads_as_a_strong_relationship` | ✅ |
| DL-6 | 「内向」对自己是词汇、对第三方是声称 | — | `assert_publishable` vs `assert_publishable_about_peer` 两个入口 | `the_peer_screen_only_applies_to_peer_copy` | ✅ |
| DL-7 | ASCII 禁词大小写不敏感 | — | `first_hit` 先 `to_lowercase` | `ascii_denylist_words_are_case_folded` | ✅ |
| DL-8 | 新句子加进来就被筛，不靠人记得 | — | 筛子走 `A2_STATEMENT_KEYS` 与 `A1Reason` 全集 | `every_chinese_template_this_crate_ships_is_screened`：**表内有模板但无夹具能渲染它，测试就失败**，堵死「不写夹具就不被筛」 | ✅ |

### 2.5 工程纪律（§6.2）

| 条款 | 实测 | 判 |
|---|---|---|
| `#![forbid(unsafe_code)]` | `lib.rs` 属性 + `Cargo.toml` `[lints.rust] unsafe_code = "forbid"`，双重 | ✅ |
| 零重依赖，禁 SQLCipher / Tauri / HTTP | `[dependencies]` 段为空 | ✅ |
| 纯函数，不读墙钟 | 全 crate `rg "SystemTime|Instant|now\(\)|rand|std::time"` 仅命中 lib.rs 的一句**文档**，无代码 | ✅ |
| 无哈希（迭代序 = 输入序） | 无 `HashMap`/`HashSet`，分组用 `Vec` + 线性查找，first-appearance 序 | ✅ |
| 依赖箭头 Goal 1 → soul-algo | crate 不依赖任何东西，箭头无法反向 | ✅ |
| `#![deny(missing_docs)]` | `lib.rs`；clippy `-D warnings` 通过 | ✅ |

---

## 3. 对照表 B：`TieScore`（a2.rs）× `tie_strength`（relationship.schema.json）

这是 A2 与存储面的接缝。§6.4 说「`TieScore` 携带 band、原始计数、一对一/群聊分列、last_contact、沉寂天数与 as_of；A2 与图 UI 只消费该结构」，所以两侧字段必须能对上。实测**对不齐的有 6 处**。

| `TieScore` 字段 | `tie_strength` 字段 | 对齐情况 |
|---|---|---|
| `band: Band` | `band`（enum weak/moderate/strong） | ⚠️ **`Band::None` 无落点**。A2 明确支持 `Band::None` 并为它写了专门的归档句（`counts_with_no_band_yet_still_render` 钉住），但 schema 的 band 枚举排除了 `none`（探针 P08 实测拒绝）。映射只能是「`None` ⟺ 省略 `band` 键」，而这条映射无处成文 |
| `interaction_count: u32` | `interaction_count` | ✅ |
| `outgoing: u32` | `outgoing_count` | ⚠️ 名字不同，无绑定测试 |
| `incoming: u32` | `incoming_count` | ⚠️ 同上 |
| `active_day_count: u32` | `active_day_count` | ✅ |
| `conversation_count: u32` | `conversation_count` | ✅ |
| `direct_count: Option<u32>` | **无**（只有 `direct_out_count` / `direct_in_count`） | ⚠️ 需适配器求和。D33 只为群聊侧写了口径（「渲染持久化 `group_out_count + group_in_count`」），**一对一侧的同款口径没写**。探针 P10 实测：把 `direct_count` 直接写进 `tie_strength` 会被 `additionalProperties:false` 拒 |
| `group_count: Option<u32>` | **无**（`group_out_count` + `group_in_count`） | ⚠️ 口径由 D33 定义，但只在 DECISIONS 表格里，不在 schema 的 `$comment` 里 |
| `any_direct: bool` | **无** | ⚠️ 只能由 `direct_out_count + direct_in_count > 0` 导出。它驱动「只在群聊里见过」这一整句，导出规则无处成文（探针 P12） |
| `last_contact_unix: Option<i64>` | `last_contact_utc` | ✅ 语义一致（任一场地），表示不同（unix vs RFC3339） |
| `as_of_unix: i64` | `as_of_utc` | ✅ |
| `evidence_ids: Vec<u64>` | 关系级 `evidence_ids`（uuid7） | ⚠️ `tie_strength` 内部无证据字段；u64 ↔ uuid7 需适配 |
| `last_contact_evidence_id: Option<u64>` | **无** | ⚠️ A2 承诺「近因句只引最近一条证据」（COPY_ZH P4 + `the_recency_line_cites_the_one_row_it_rests_on`），但**这条 id 在 schema 里没有位置**，`additionalProperties:false` 也不让加（探针 P11）。重建后该承诺只能降级为「引全部 `evidence_ids`」 |
| *(方法 `days_since_last_contact()`)* | `silent_days` | ⚠️ **两处各算一遍**。schema 存了一个 `silent_days`，A2 从 `as_of - last_contact` 重算一个，两者无对账、无跨字段约束（探针 P21：把 `silent_days` 填成 99999 依然合法） |
| **无** | `first_contact_utc` | ℹ️ schema 有、A2 不用。声明 `algorithm_id` 时必填，但没有消费者 |
| **无** | `direct_active_day_count` | ℹ️ T4D 判档字段，A2 不渲染（正确：A2 不复述门槛） |
| **无** | `machine_band` / `user_band` / `locked_by_user` | ℹ️ D32/D48 面，A2 只消费生效档 `band`。**但 crate 没有任何 affordance 表达「这条边被锁了」**，见 5.7 |

---

## 4. schema 缺口

### 4.1 lock 完整性：通过

11 份 schema 的 SHA-256 与 `schemas.lock.json` **逐份吻合**（`sha256sum` 手算比对）。`schemas.lock.json` 自身不在被锁列表内，符合预期。

### 4.2 `tie_strength` 类型化（D58/D59）：24 条探针，24 条与预期一致

探针源码 `schema_probe.py`，输出 `schema_probe.out`，用 `jsonschema` 4.26 / Draft 2020-12，`_defs` 经 `referencing` registry 解析。**D59 声称的每一条都实测成立**：

| 探针 | 断言 | 实测 |
|---|---|---|
| P01 | 空对象仍合法 | 合法 ✅ |
| P02 | 整套 T4D 可复核面 | 合法 ✅ |
| P03/P04/P05/P22 | 声明 `algorithm_id` 后缺 `silent_days` / `as_of_utc` / `direct_active_day_count` / direct 全缺 | 全部拒绝 ✅ |
| P06/P07 | 写分列计数或 `silent_days` 却不声明 `algorithm_id` | 拒绝 ✅（`dependentRequired` 棘轮） |
| P09 | `algorithm_id: "T3R"` 偷渡第三套规则 | 拒绝 ✅ |
| P14 | `user_band` 无 `locked_by_user` | 拒绝 ✅ |
| P15/P16/P17 | `user_band: null` + 未锁 / `machine_band` 并存 / `last_direct_contact_utc: null` | 合法 ✅（D32/D48 不被 `additionalProperties:false` 打红） |
| P18/P19 | 裸 `band` / 裸 `interaction_count` | 合法 ✅（刻意保留 Goal 1 今日宽松度） |
| P20 | `silent_days: -1` | 拒绝 ✅ |

**棘轮设计值得点名表扬**：`dependentRequired` 把 8 个 T4D 新字段各自绑到 `algorithm_id` 上，而 `algorithm_id` 又经 `if/then` 触发 15 字段整包必填。效果是「写了任何一个新字段 ⇒ 必须写全套」，杜绝半迁移。这比 D59 字面承诺的「`algorithm_id` 触发整包必填」更强，AC-33「适配器不得援引本行少填」由此在 schema 层可执行。

### 4.3 缺口 S1–S7

| 编号 | 缺口 | 实测 | 严重度 |
|---|---|---|---|
| **S1** | `band` 枚举无 `none`，A2 的 `Band::None` 归档句无落点 | P08 拒绝 | 低（映射为「省略键」即可，但需成文） |
| **S2** | A2 消费的 `direct_count`/`group_count` 在 schema 里不存在，需适配器求和；一对一侧的求和口径未成文（D33 只写了群聊侧） | P10 拒绝 | **中** |
| **S3** | `last_contact_evidence_id` 无落点，「近因句只引最近一条证据」重建后无法保真 | P11 拒绝 | **中** |
| **S4** | `any_direct` 无落点，「只在群聊里见过」整句的导出规则未成文 | P12 拒绝 | 低 |
| **S5** | **D32「`locked ⟺ user_band.is_some()`」双向都未被 schema 强制** | P13/P23/P24 全部**合法** | **中** |
| **S6** | 棘轮只覆盖 T4D 新字段；`interaction_count` 等遗留字段单独出现不触发任何要求 | P19 合法 | 低（D58 明写「进一步收紧须与 Goal 1 同批」，属已定价债） |
| **S7** | `silent_days` 与 `as_of_utc − last_contact_utc` 无跨字段一致性约束 | P21 合法（填 99999 亦可） | 低（JSON Schema 本就难表达；应在 Goal 1 侧断言） |

**S5 展开**，因为它是唯一一条与已拍板决议直接冲突的 schema 行为。D32 拍的是**双条件** `locked ⟺ user_band.is_some()`。schema 只写了 `dependentRequired: { user_band: ["locked_by_user"] }`，这只要求「有 `user_band` 就得有 `locked_by_user` 这个**键**」，不要求它的**值**是 `true`，也完全不管反方向。三个直接反例都通过校验：

- P13：`locked_by_user: true`，无 `user_band` → 合法（违反 ⟸）
- P23：`user_band: "strong"`，`locked_by_user: false` → 合法（违反 ⟹ 的值约束）
- P24：`locked_by_user: true`，`user_band: null` → 合法（违反 ⟸）

D32 的备注写「Goal 1 吸收线已钉测试」，所以这条在 Goal 1 侧可能有守卫；但**在 schema 这一层，D32 目前基本没有被执行**，而 schema 正是导出/导入面唯一的把关点。

### 4.4 其他 schema 面缺口

| 缺口 | 说明 |
|---|---|
| `evidence.schema.json` 无 `forgotten` / 墓碑字段，且 `additionalProperties: false` | crate 的 `EvidenceRef.forgotten` 是「遗忘可重算」这条产品承诺的载体（A0-9）。schema 里表达不了这个状态。D34 说「Goal 1 不实现墓碑/Option 化」，所以这是**已知的、被决议推迟的**，但接缝仍在：导出一份 evidence 后重放，无法区分「被遗忘」与「不存在」 |
| `inference.schema.json` 无 `algorithm_id` 字段 | `is_rejected_algorithm()` 是给写边界用的，但推断行的 schema 根本没有地方记「哪个算法盖的章」。A3 的拒收在 schema 层无法执行；`method` 枚举里的 `statistical` / `llm` 也不带否决语义 |
| `statement_key` 只有 `minLength: 1`，无 enum 无 pattern | `A2_STATEMENT_KEYS` 与 `trait_axis.<key>.<position>` 都是 crate 侧的稳定契约，schema 不认识它们。写进一个手打的 key 不会被拦 |
| `relationship.schema.json` 的 `egress_scope: local_only` 是 `const` 但**不在 `required` 里** | 省略即通过。第三方关系数据的 local_only 承诺在这一层是可选的 |
| `types` / `voice` / `values` / `boundaries` 仍是裸 `array`/`object` | D58 提到的「9 份 schema 正文已分叉」的残留面。不属本轮范围，仅记录 |
| COPY_ZH §0.3 与 §5.1 引用的函数名 `assert_non_clinical` 在 crate 里不存在 | crate 的入口叫 `assert_publishable` / `assert_publishable_about_peer`。Goal 1 适配器照 COPY_ZH 找函数会找不到 |

---

## 5. 不完善项（按严重度排序）

### 5.1 【高】`COPY_ZH.md` §4 与 `a2.rs` 是两套文案，而 D40 记的「另记」无处可查

D40 备注列写着「A2 是渲染器；COPY_ZH 与 crate 文案漂移另记」。我在 `docs/` 全文检索 `漂移`/`另记`，除 D40 自身外只命中 REJECTED.md 里两条与本事无关的历史事故。**这条漂移从未被「另记」过。** 同时 §6.4 要求「中文模板绑定测试随 soul-algo 落地，模板改动先改 COPY_ZH.md 再改代码」——绑定测试目前也不存在。两件事叠加的后果是：漂移既没被测出来，也没被写下来。

用 `copy_drift_probe.rs`（path 依赖引用 crate，不改 crate）跑真实渲染器，237 次渲染得 46 条唯一句。逐句对照 COPY_ZH §4 的六类句：

| COPY_ZH §4 | 冻结模板 | `a2.rs` 实际渲染 | 判 |
|---|---|---|---|
| **P1b 分列** | 其中一对一往来 {一对一次数} 次，群里同场 {群聊次数} 次。 | 其中一对一往来 2 次，群里同场 135 次。 | ✅ **逐字一致** |
| **P1 总量** | 一共 {次数} 次往来，分布在 {天数} 个自然日、{会话数} 个会话里。 | 有记录的往来 137 次，出现在 22 个不同的日子、2 个会话里。 | ❌ 措辞不同（事实相同） |
| **P2 方向** | 互斥五分支，**阈值 2:1**：都是你在说 / 都是对方在说 / 多数时候是你先开口 / 多数时候是对方先开口 / 两边说得差不多 | 三分支，**无 2:1 比较**，且直接打印次数：往来是双向的：你发出过 70 次，对方发来过 67 次。 | ❌ **结构不同** |
| **P3 场合** | 有一对一 →「有过一对一交流。」；仅群聊 →「只在群聊里见过。」 | 仅群聊 →「到目前为止只在群聊里见过往来，没有一对一的记录。」；**有一对一 → 刻意不渲染任何句子** | ❌ 少一个分支 + 措辞不同 |
| **P4 近因** | 最近一次是 {最后日期}。 | 最近一次往来距最新的记录 4 天。 | ❌ **日期 vs 天数**。COPY_ZH §0.2 明写「日期写作「{某某某某} 年 {某} 月 {某} 日」」 |
| **P4 沉寂追加** | 你们最近半年没有往来。 | 已经 400 天没有新的往来了；下面的归档说的是过去的记录，不是现在的联系频率。 | ❌ 措辞不同 |
| **P5 归档** | 按上面的计数，这段关系归在『{强/中等/弱}』一档；这是工作假设，不是对这个人的判断。 | 按上面的计数，这条往来归在「强」一档。这是对记录的归档，不是对这个人的评价。 | ❌ 措辞不同 |

再对 COPY_ZH §5 的七条验收断言逐条实测：

| 断言 | 实测 | 判 |
|---|---|---|
| §5.5 band 词只出现「强 / 中等 / 弱」 | `Band::Moderate.label_zh() = 「中」`，**冻结词表里没有「中」**。13 条渲染句读作「归在「中」一档」 | ❌ **违反 §0.1 词汇单源** |
| §5.4 任何模板不出现「今天」「现在」 | 剔除「出现在」这个 substring 假阳性后，真命中 **1 个模板 / 2 条唯一句**：沉寂句的「不是**现在**的联系频率」 | ❌ 违反 |
| §5.4（断言措辞本身的问题） | 若按字面 substring 执行，「出现在 22 个不同的日子」里的「出现在」会被误伤，**52 条渲染句中招** | ⚠️ 断言的**拼法**有缺陷，落地时需按词而非按 substring |
| §5.1 不含 §0.3 禁词 | 12 个禁词里 **9 个不被 crate 的筛子拦截**：分数、得分、百分比、排名、指数、权重、系数、衰减、半衰期（拦住的只有评分、打分、百分位） | ❌ 见 5.4 |
| §5.1 不含任何拉丁字母 | A2 渲染句实际含拉丁字母 **0 条**（结果合规），但 **crate 无对应筛子**——合规靠的是没人写，不是靠一道门 | ⚠️ |
| §5.2 无「折算」「半次」「四分之一」及非整数 | 实际 0 条（T3R 落选后本就无此形状） | ✅ |
| §5.3 每个数字可由输入直接重算 | A2 只搬运 `TieScore` 里的整数，不做乘除；唯一的算术是 `(as_of - last).max(0) / 86400` | ✅ |
| §5.6 总量只许出现在 A2 的 P1 | A2 的方向句也打印了总量分方向的两个数（COPY_ZH 的 P2 一个数都不打印） | ⚠️ 轻微越界 |
| §5.7 分列字段缺席则 P1b 不渲染；源码无重算分列路径 | `no_split_no_sentence` 已钉 | ✅ |

**怎么读这条**：漂移不等于 crate 错。`a2.rs` 的几处偏离都有明写的、我认为站得住的理由——不渲染「有过一对一交流」是因为那是在描述关系；用天数而非日期是因为零依赖 crate 没有日期格式化；不做 2:1 比较是因为那是 A2 自己会拥有的第二个数字。问题在于**两份权威面互相矛盾且都自称冻结**，而 D40 承诺的「另记」没有发生。这必须由父代理裁一次：改 COPY_ZH 追认 crate 的话术，还是改 crate 追认 COPY_ZH。**「中」→「中等」这一条我建议无论怎么裁都要修**，因为它同时违反 §0.1 与 §5.5，且是纯字面修改，不动任何逻辑。

### 5.2 【高】D52 要求的「两侧天数常量用工作区测试钉相等」不存在

D52 原文：「两侧天数常量用工作区测试钉相等；产品 crate 禁止再写第三处 `DEMOTE_ONE_BAND_DAYS` 字面量」。DECISION.md §3 更严：`DORMANT_NOTE_DAYS` = `DEMOTE_ONE_BAND_DAYS`，「与降档共用同一常量，**不得分裂**」。

实测：

- `crates/soul-algo-tie/src/constants.rs:51` — `DEMOTE_AFTER_SILENT_DAYS: i64 = 180`
- `crates/soul-algo-trait/src/a2.rs:91` — `DORMANT_AFTER_DAYS: i64 = 180`
- 两个 crate **互不依赖**（`rg` 双向零命中，`soul-algo-trait` 的 `[dependencies]` 为空），所以没有任何测试能同时看见这两个常量
- trait 侧唯一相关的断言是 `a2_render.rs:303` 的 `assert_eq!(DORMANT_AFTER_DAYS, 180)`——它钉的是**字面量 180**，不是 tie 侧的常量

结果：改了 tie 侧的降档门而忘了 trait 侧，两侧测试都会继续全绿，而用户会看到「因久未联系降档」与「最近半年没有往来」两句在不同的天数上出现。这正是 §3「不得分裂」要防的事，也正是 §6.4「A2 的 P4 句与档位永不同屏矛盾」这个不变式的前提。

**这条是本轮唯一一个「已拍板要做、但确实没做」的工程义务。** 修法很轻：给 `soul-algo-tie` 加一条 `[dev-dependencies] soul-algo-trait = { path = ... }`（方向正确：tie 是判档方，trait 是渲染方）并写一个 `assert_eq!(constants::DEMOTE_ONE_BAND_DAYS, soul_algo_trait::a2::DORMANT_AFTER_DAYS)`。不改任何产品逻辑，不引入运行时依赖。或者等 §6 的 `crates/soul-algo` 合并——但合并没有排期，而漂移风险从今天起就在。

### 5.3 【中】`schemas.lock.json` 在本树没有校验器

lock 文件自称「Regenerate with `cargo run -p xtask -- schema-freeze --write` ... `--check` runs in CI」。本树：

- 无 `xtask` crate（workspace members 只有两个算法 crate）
- 无 `.github/`，无任何 CI 配置

我手算的 11 份 SHA-256 全部吻合，所以**今天的状态是好的**。但 lock 的价值在于「改了 schema 而没走审批会红」，这个能力目前不存在。STATUS.md 与 D59 都把 `xtask schema-freeze` 记为「Goal 1 合并后」跑，所以这是**已知推迟**；我记录它是因为在那之前，`docs/schemas/**` 处于「有锁无匙」状态——lock 会随手被改成任意值而没人发现。

### 5.4 【中】crate 的禁词表是 COPY_ZH §0.3 的真子集，且函数名对不上

COPY_ZH §0.3 的禁词表有 12 个非诊断词 + 「任何拉丁字母与「%」」 + 诊断词表全表。crate 的三张表（10 + 10 + 14）实测只拦住其中 3 个：

| 拦住 | 漏掉 |
|---|---|
| 评分、打分、百分位 | 分数、得分、百分比、排名、指数、权重、系数、衰减、半衰期 |

「衰减」「半衰期」「权重」「系数」是 T3R 那类折算式算法的特征词，正是墓碑要防复活的东西；「得分」「分数」「百分比」是 D22 直接禁的东西的近义写法。今天 A2 一条都没踩，但筛子的作用是拦住明天写的句子。另外 COPY_ZH 两处（§0.3、§5.1、§4 开头）都把这个筛子叫 `assert_non_clinical`，crate 里没有这个名字。

### 5.5 【中】A2 无法表达「这条边被用户锁了」，而 D35/D48/AC-32 要求锁定边不渲染冻结 P5 原句

D40：「锁定边仍丢掉 `filed_band`」。AC-32：「COPY_ZH 未加性批准「由你本人指定」变体前，锁定边不得渲染冻结 P5 原句（D35 / D48）」。

`TieScore` 里**没有** `locked_by_user` / `user_band` 字段，`a2_render` 也**无条件**产出 `personnel.tie.filed_band`。也就是说这条义务 100% 落在调用方（`soul-draft`）身上，而 crate 侧没有任何东西提醒调用方它存在——没有字段、没有文档段落、没有测试。这不是 crate 的 bug（A2 只该消费生效档），但它是一个**只靠人记得**的接缝，而 A2 其余每一条纪律都做到了「靠类型或测试记得」。

### 5.6 【低】A2 无阈值的源码守卫按名字写，不按语义写

`a2_defines_no_band_thresholds` 与 `the_a2_source_still_holds_no_threshold` 搜的是六个字面量：`STRONG_MIN`、`MODERATE_MIN`、`MIN_INTERACTIONS`、`MIN_ACTIVE_DAYS`、`>= 10`、`>=10`。换个名字（`THICK_ENOUGH`）或换个比较符（`> 9`、`>= 3`）都能走过去。

**但这条基本已经被补上了**，而且补得很老实：`roundx_a2_no_threshold.rs` 的模块文档主动承认这个弱点（「it is spelled against the names Goal 1 used, so a gate written with any other name or any other comparison walks past it」），并给出 7,168 组跨门计数扫描 + 「band 不得决定出现哪些句子」的双向行为断言。残余风险只剩「某个门恰好在扫描覆盖的计数组合外」，很小。我把它列出来只是为了让父代理知道**源码扫描不是这条纪律的主要保障，行为扫描才是**——如果将来有人为了「简化」删掉 `roundx_a2_no_threshold.rs`，保障强度会大幅下降而所有测试仍然全绿。

### 5.7 【低】`SECONDS_PER_DAY` 在同一个 crate 里定义了两遍

`types.rs:186` 是 `pub const SECONDS_PER_DAY: i64 = 86_400`，`a2.rs:94` 又私有定义了一份同名同值常量，`fixtures.rs:23` 还有第三份 `pub const DAY: i64 = 86_400`。三份都对，且这不是判档阈值，所以 D52 的「禁止第三处字面量」严格说不适用。但 A2 的模块文档说「The one number A2 owns is `DORMANT_AFTER_DAYS`」——实际上它拥有两个。

### 5.8 【低】`DECISION.md` §6.4 的交叉引用指向不存在的小节

§6.4 写「中文模板绑定测试（COPY_ZH.md **第 6 节**断言）随 soul-algo 落地」。`COPY_ZH.md` 只有 §0–§5，验收断言在**第 5 节**。差一节。照本条去找绑定测试规范的人会找不到。（本轮不改权威 docs，仅记录。）

---

## 6. 测试结果

`cargo test -p soul-algo-trait`（Rust 1.83.0，完整输出见 `cargo_test.out`）：

| 测试二进制 | 通过 | 覆盖 |
|---|---|---|
| `a0_lock` | 19 | 锁语义、intake 补丁、intake↔replay 全等、两种写模式、遗忘重算 |
| `a0_vs_a1` | 8 | A0/A1 消融对照、v0.1 上二者不可区分 |
| `a1_independence` | 27 | `(kind, utc_day)` 分组、F16/F16b、跨 epoch 日界、矛盾不按多数决 |
| `a2_render` | 22 | 纯渲染、分列句、沉寂闭区间、确定性、时间平移不变 |
| `a3_refusal` | 6 | 单条/批量拒绝、理由不含正文、被否决 id 注册 |
| `denylist_scan` | 14 | 三张筛子表、输出面全覆盖筛查、大小写折叠 |
| `frozen_defaults` | 12 | 四个算法 id、默认写模式、A0 不依赖 A1、A2 源码无阈值 |
| `roundx_a1_empty_on_v01` | 3 | 346,200 + 4,096 条日志穷举 |
| `roundx_a2_no_threshold` | 3 | 7,168 组跨门计数扫描 + band 不决定句子集合 |
| lib 单元 / doctest | 0 / 0 | 该 crate 把断言全放在集成测试里 |
| **合计** | **114 / 114 通过，0 失败，0 忽略** | 耗时 < 1s（纯函数、无 IO） |

附加检查：

| 检查 | 结果 |
|---|---|
| `cargo clippy --workspace --all-targets -- -D warnings` | 通过 |
| `cargo fmt --all -- --check` | 通过 |
| `docs/schemas/*.schema.json` × `schemas.lock.json` SHA-256 | 11/11 吻合 |
| `tie_strength` schema 探针 | 24/24 与本报告记录一致 |

---

## 7. 复现

```bash
# 测试 + 静态检查
cd /workspace && cargo test -p soul-algo-trait
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check

# schema lock 完整性
cd docs/schemas && for f in *.schema.json; do sha256sum "$f"; done

# tie_strength 契约探针（需 pip install jsonschema）
python3 .agent_workspace/orch-20260826/round1/opus-b/schema_probe.py

# COPY_ZH 话术漂移探针（在 /workspace 之外建 path 依赖，不改 crate）
mkdir -p /tmp/copy_probe/src
cp .agent_workspace/orch-20260826/round1/opus-b/copy_drift_probe.rs /tmp/copy_probe/src/main.rs
# Cargo.toml: [dependencies] soul-algo-trait = { path = "/workspace/crates/soul-algo-trait" } + 空 [workspace]
cd /tmp/copy_probe && cargo run
```

本目录留档：`schema_probe.py` / `schema_probe.out` / `copy_drift_probe.rs` / `copy_drift_probe.out` / `cargo_test.out`。

---

## 8. 给父代理的建议（本轮不执行，仅提请裁决）

按「能不能不改权威面就修」排序：

1. **裁 COPY_ZH vs a2.rs 的话术归属**（5.1）。这是唯一需要产品裁决的一条。最小动作是先只修「中」→「中等」——它同时违反 §0.1 与 §5.5，纯字面，不动逻辑。其余六类句的归属可以并进 §6 的合并义务。
2. **补 D52 的跨 crate 常量相等测试**（5.2）。tie 侧加 dev-dependency 即可，不动产品逻辑，不引运行时依赖。这是已拍板未做的工程义务，不需要新决议。
3. **补 S5 的 D32 双条件**（4.3）。schema 侧可用 `if/then` 表达 `locked_by_user: true ⇒ required[user_band]` 与 `user_band` 非 null ⇒ `locked_by_user: const true`。但按 D58「进一步收紧 `tie_strength` 须与 Goal 1 `schemas.lock.json` 同批重算」，这条**必须与 Goal 1 同批做**，不能在计划线单独收紧。
4. **把 S2/S3/S4 的字段映射写成规范**（第 3 节）。`direct_count = direct_out_count + direct_in_count`、`any_direct = (direct_out + direct_in) > 0`、`Band::None ⟺ 省略 band`、以及 `last_contact_evidence_id` 要不要进 schema。前三条是补一句 `$comment` 或一条 D 号的事；第四条要动 schema，同样受 D58 同批约束。
5. **`xtask schema-freeze`**（5.3）。已在 STATUS「下一步」第 2 条排队，无需新动作，只需别忘。
6. **补齐 denylist 到 COPY_ZH §0.3，并统一函数名**（5.4）。加词表是加法变更，风险低；但「加词表」会让现有 46 条渲染句重新过筛，需先确认无回归（实测目前 0 条会被新词命中）。
