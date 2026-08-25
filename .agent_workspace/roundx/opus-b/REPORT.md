# Round X · opus-b · `soul-algo-trait` 交叉验证

模型 slug：`claude-opus-5-thinking-high-fast`
提交：`752cdec`（`cursor/algo-verify-opt-a073`）· 工具链 `rustc 1.83.0`
命令：`cargo test -p soul-algo-trait --offline`

**结论：四条断言全部成立，冻结不动。** 新增两个测试文件，六个测试，补的是两个真实的空档（见第 3 节），不改任何 `src/`。

---

## 1. 测试运行

`cargo test -p soul-algo-trait --offline`，找到的树（未加任何 Round X 文件）：

| 测试目标 | 数量 | 结果 |
|---|---|---|
| `src/lib.rs` unit | 0 | ok |
| `tests/a0_lock.rs` | 19 | ok |
| `tests/a0_vs_a1.rs` | 8 | ok |
| `tests/a1_independence.rs` | 27 | ok |
| `tests/a2_render.rs` | 22 | ok |
| `tests/a3_refusal.rs` | 6 | ok |
| `tests/denylist_scan.rs` | 14 | ok |
| `tests/frozen_defaults.rs` | 12 | ok |
| doc-tests | 0 | ok |
| **合计** | **108** | **0 failed** |

加上本轮两个文件后 **114 passed / 0 failed**，`cargo fmt --check` 无输出，`cargo clippy --all-targets` 无 warning。全程离线，无网络、无 `unsafe`、无外部依赖（crate 的 `[dependencies]` 是空的）。

一句免责：本目录只跑 `-p soul-algo-trait`。同一 checkout 里另一个 Round X 代理正在往 `crates/soul-algo-tie/tests/` 写文件，`cargo test --workspace` 因此会红，但那是它的包、它的文件，与本报告的四条断言无关。

---

## 2. 四条断言逐条核对

核对方式不是「找到一个名字像的测试」，而是两步：读实现，然后把实现改坏，看有没有测试真的响。第二步是必要的——一个从来不会失败的断言等于没有断言。完整的变异输出在 `TEST_LOG.txt` RUN 4。

### A0 lock — 成立

规则在 `src/a0.rs::place` 一处收口，replay 与 intake 共用同一个 fold：

```350:361:crates/soul-algo-trait/src/a0.rs
fn place(state: &mut AxisState, row: &EvidenceRef, mode: WriteMode) -> ApplyResult {
    let band = row.effective_band();

    if state.locked_by_user && !row.locked_by_user {
        return ApplyResult::RefusedLocked;
    }
```

这就是 Round 1 找到的缺陷的补丁位置：Goal 1 的 `intake` 绕过 `axis_is_locked` 直接 `place_axis`，这里的 `apply_intake` 走的是同一个 `place`，所以 intake 不可能和 replay 分叉。`intake_matches_replay_on_every_fixture_and_mode` 在两种 `WriteMode` × 15 个夹具上跑的正是这条等价性。

配套的语义也都钉住了：被拒的答案仍然写进 `log_after`（`the_answer_row_is_still_written_even_when_the_axis_will_not_move`）、被拒的原因进 `ignored` 而不是被吞掉（`a_skipped_answer_is_reported_rather_than_dropped`）、锁只锁一条轴（`a_lock_on_one_axis_does_not_freeze_the_others`）、用户改主意仍然能动（`a_later_correction_still_moves_a_locked_axis`）、忘记纠正后档位自己落回来（`forgotten_rows_are_dropped_before_anything_is_aggregated`）。

变异验证：把 `place` 里的锁分支改成 `if false`，**11 个现有测试失败**，跨 `a0_lock.rs` / `a0_vs_a1.rs` / `frozen_defaults.rs` 三个文件。锁是有人守的。

### A1 `TwoKindsAcrossDays` 默认、`EMPTY_ON_V01` — 成立

`A1_DEFAULT_INDEPENDENCE == TwoKindsAcrossDays`、`A1Independence::default()` 同值、审计拼写 `"two_kinds_across_days"`、`A1_ALGORITHM_ID` 已随默认值变化 bump 到 `v3`——四项都有断言。行为侧：三次同源问卷跨三天停在 Moderate 且理由是 `NeedsASecondSource`（不是「再答几次就好了」的错误话术），F16b 仍然升到 Strong。

`A1_EMPTY_ON_V01 == true` 的支撑是 `a1_can_fire_on_v01_data`：它从 `EvidenceKind::reachable_for_axis_in_v01` 数可数来源（问卷、纠正，减去纠正因为纠正在计数之前就锁了轴），得到 1 < 2，于是不可能升档。这个推导比写死一个 `true` 好——多一个 v0.1 生产者会翻转函数，而不是悄悄让文档里的一句话作废。

变异验证：把默认改成 `KindAndDay`，**12 个测试失败**（其中 9 个是现有的）。

### A2 无 `STRONG_MIN` — 成立

`src/a2.rs` 里确实没有任何档位阈值：`a2_render` 读 `TieScore::band` 并原样抄给每一条 bullet，唯一的数字 `DORMANT_AFTER_DAYS = 180` 只决定多不多一句描述性的话，不碰档位（`dormancy_is_a_sentence_not_a_demotion` 钉住了这点，`quiet_for_exactly_the_threshold` 钉住了边界是严格大于）。守卫是两个 `include_str!` 源码扫描（`a2_defines_no_band_thresholds`、`the_a2_source_still_holds_no_threshold`），各扫六个字面量。

我对这两个扫描有保留意见，见第 3 节——它们拼的是 Goal 1 用过的名字，换个名字就绕过去了。但「今天的 A2 有没有阈值」这个问题本身，答案是没有。

### A3 拒绝 — 成立

`a3_from_message_text` 和 `a3_from_messages` 都无条件 `Err(A3Refused::new())`，参数只借不读；`A3_ALGORITHM_ID` 以 `.rejected` 结尾并被 `is_rejected_algorithm` 认出。四段文本（含空串和 `"焦虑"`）× 五条轴、1000 条 evidence_ids 的批量形式，全部 `Err`。

变异验证：让 `a3_from_message_text` 返回 `Ok(AxisState::unknown(...))`，两个现有测试失败。拒绝是有人守的。

---

## 3. 补的两个空档

规则是「只为真实缺失的钉子加测试」，所以每一个新文件都要能指出一个现有 108 个测试全部放过的具体改动。两个都能，变异输出见 `TEST_LOG.txt` RUN 4。

### `tests/roundx_a1_empty_on_v01.rs` — 3 个测试

`EMPTY_ON_V01` 现在有两个支撑，都没有把「数来源种类」和「真的跑聚合」接起来：

- `a1_can_fire_on_v01_data` 是关于 `EvidenceKind` 的陈述，不是关于 `a1_axis_state` 的。`empty_on_v01_is_true_under_the_default_and_false_under_the_alternative` 把常量钉在这个函数上，于是两者互相印证，谁都没碰聚合代码。
- `no_v01_reachable_log_reaches_strong_without_a_correction` 确实跑聚合，但只跑 `axis_evidence_matrix` 里那 15 条手写日志，没有一条是冲着升档分支写的。

缺的是接缝。而这个空间小到可以穷举：v0.1 能对一条轴写的行只有问卷和纠正两种，一行由 (kind, position, day) 定死。于是枚举 24 个符号上长度 1–4 的全部 **346 200** 条日志，逐条断言 `A1Reason::UpgradedByIndependentGroups` 从不出现、`Strong` 必然带锁；再补一轮 6 天 × 4 个方向的 4096 条问卷日志，覆盖「组数远超阈值」那一侧。跑完 0.33 秒。

（忘记的行故意不枚举：忘记发生在分组之前，所以长度 n 带一条忘记行的日志与长度 n−1 的日志聚合结果相同，而后者已经在枚举里。）

### `tests/roundx_a2_no_threshold.rs` — 3 个测试

源码扫描是对的形状——它挂在发布出去的文件上而不是碰巧正确的行为上——但它拼的是 `STRONG_MIN` / `MODERATE_MIN` / `MIN_INTERACTIONS` / `MIN_ACTIVE_DAYS` / `>= 10` / `>=10`。行为侧的补充是 `tie_score_matrix`，那个矩阵抓得住明显的复辟（240 次 / 61 天、90 次一对一，四个档位各渲染一遍），但它在边界上很薄。15 个夹具的实际取值（跑出来的，不是数出来的）：

- `interaction_count`：0, 1, 2, 3, 9, 12, 14, 18, 20, 41, 96, 137, 180, 240, 1000 — 没有 4、10、11
- `active_day_count`：0, 1, 2, 4, 5, 6, 7, 9, 22, 31, 40, 61 — **没有 3**，而 3 正是 `STRONG_MIN_ACTIVE_DAYS`
- `direct_count`：`None`, 0, 2, 12, 90 — 整个 `3..10` 窗口空着，而 T4D 形状的门闩正好住在那里

两个具体的、现有 108 个测试全部放过的改动：

1. **改名后的门闩。** 在 `a2_render` 里加 `const THIN_DIRECT_RECORD: u32 = 10;`，当 `direct_count` 落在 `3..10` 时把档位降到 Weak——这是半个 T4D。六个字面量一个都不匹配（用的是 `< THIN_DIRECT_RECORD`），矩阵的 direct 值全在窗口外。现有测试**全绿**；`no_count_on_either_side_of_a_gate_moves_the_band` 抓住。
2. **档位反过来决定说哪句话。** 让 `venue_bullet` 在 `band == Strong` 时不出「只在群聊里见过」这句。所有现有测试只检查「出来的 bullet 带的档位对不对」，没有一个检查「出来的是不是同一批 bullet」，而群聊-only 的三个夹具默认档位都不是 Strong。现有测试**全绿**；`the_band_never_decides_which_sentences_appear` 和 `the_band_changes_exactly_one_sentence_and_it_is_the_filing_line` 抓住。

第二个空档是这两个里更要紧的那个：它是「A2 不形成第二意见」的另一半方向。现有测试全部在问「档位有没有被计数改动」，没有一个在问「说哪几句话有没有被档位改动」。一个按档位增删句子的渲染器，档位断言可以一条都不破，而它已经在对档位下判断了。

第三个测试 `the_band_changes_exactly_one_sentence_and_it_is_the_filing_line` 是第二个的补集，免得「档位不影响输出」被读成「档位从不出现」：归档那一句必须变，因为用户得能对着它反驳；其余每一句在四个档位下必须一字不差。

新文件只加 `tests/`，不碰 `src/`，不引依赖，`RUN 5` 里 `git diff --stat -- crates/` 为空即为证。

---

## 4. 顺带记下的、不构成改判的观察

不走 DECISION 第 5 节回退链，不建议动冻结。列在这里是因为下一轮会问。

1. **`the_two_write_modes_do_differ_in_what_they_cite` 是 Round 3 明确甩给 Round X 的题。** 同一份问卷填五次，`LastWriteWins` 只引用第 5 行，`NoDowngrade` 引用全部五行。方向、档位、锁三项一致，所以档案显示的东西不变，但「用户能复核计数」是 C3 独立的一维，而冻结模式给出的答案更短。我的判断：**不动**。理由不是它无所谓，而是它是 UI 层的取数问题——证据表里五行都在，`a0_axis_state` 也没丢它们，档案页要展示「这条基于哪几次回答」可以直接查证据表按轴过滤，不需要改 A0 的写入语义。为了一个展示问题去动 Goal 1 的回归基线，代价和收益不成比例。
2. **`a0_trusts_the_band_on_a_row_it_is_handed` 记录的缺口仍然是真的**，且已经写清楚了：A0 读 `EvidenceRef::band` 而从不封顶，所以「只有纠正能到 Strong」是 v0.1 数据面的性质，不是这段代码的性质。开始写行为行的那个版本必须二选一（把非纠正行封在 Moderate，或者启用 A1）。这是已知的、有测试的、有署名的欠款，不是本轮发现。
3. **A2 源码扫描的六个字面量应当在下一次改动 A2 时一并检讨。** 我加的是行为侧的补充而不是往列表里塞更多名字——名字列表永远追不上重命名，而扫描的价值在于它挂在文件上。两者并存是对的。

---

## 5. 交付

```
.agent_workspace/roundx/opus-b/REPORT.md      本文
.agent_workspace/roundx/opus-b/TEST_LOG.txt   五段运行记录，含变异验证
.agent_workspace/roundx/opus-b/VERDICT.md     一页结论
crates/soul-algo-trait/tests/roundx_a1_empty_on_v01.rs   新增，3 测试
crates/soul-algo-trait/tests/roundx_a2_no_threshold.rs   新增，3 测试
```

未 `git commit`，未 `git push`，未开 PR。未改任何 `src/`，未改他人目录。
