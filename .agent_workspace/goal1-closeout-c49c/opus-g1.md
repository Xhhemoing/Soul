# Goal 1 关闭线 — G1 / G1+ 实现报告

分支 `cursor/goal1-closeout-c49c`。参考实现 `origin/cursor/goal1-unblock-a073` 只用 `git show` 读，没有 checkout、没有 merge、没有 commit。

一句话结论：人脉图已经不再自己判档。`soul-graph` 依赖 `soul-algo-tie`，`build.rs` 里第二套 3/10/3 阈值连同 `Tally::band` 一起删掉了，全库 `as_of` 与分列计数随边落库，owner 群消息不再对历史发言人写 Outgoing，图纠正（锁定档 / 机器档 / P5 静默）整套移植过来了。全仓 `cargo test --workspace --all-targets` 117 个测试二进制全绿，`cargo clippy -D warnings` 与 `cargo run -p xtask -- all`（e0 / denylist / schema-freeze）干净。

---

## 1. 改了哪些文件

下面只列本任务（G1 / G1+ / G3）的改动。同一工作区里另有一个兄弟代理在做 AC-31（`crates/soul-profile/**`、`crates/soulcore/src/commands/profile.rs`、`crates/soulcore/tests/profile_memory_commands.rs`、`crates/soul-profile/tests/intake_replay.rs`），那几个文件不是我改的，我也没有动。

### 1.1 `soul-graph` — 吸收 T4D

| 文件 | 状态 | 做了什么 |
| --- | --- | --- |
| `crates/soul-graph/Cargo.toml` | 改 | 加 `soul-algo-tie = { workspace = true }`（普通依赖，不是 dev） |
| `crates/soul-graph/src/t4d_adapt.rs` | 新增 | 存储观测 → `soul_algo_tie::Interaction` 的唯一边界。`InteractionInterner` 把 peer UUID 与 conversation_ref 映射成稠密 `u64`（每次 rebuild 新建，绝不落库、绝不由 UUID 截断或哈希得到）。RFC 3339 → Unix 秒的解析也在这里 |
| `crates/soul-graph/src/build.rs` | 改 | 第二套阈值删除。改为一次遍历 evidence → 每 peer 一份 `PeerEvidence` → `tie_reading(peer, rows, as_of)`。`as_of` = 全库 `max(occurred_at)`，**在丢掉解不开的联系人之前**累加（`peers_unresolved` 那个 `continue` 在 `as_of` 更新之后），所以遗忘一个联系人不会让其他人显得更近 |
| `crates/soul-graph/src/model.rs` | 改 | `TieStrength` 加 `direct_out_count` / `direct_in_count` / `group_out_count` / `group_in_count` / `direct_active_day_count` / `last_direct_contact_utc` / `silent_days` / `as_of_utc` / `algorithm_id` / `locked_by_user` / `user_band` / `machine_band`；另加手写 `Serialize` / `Deserialize`，见下面第 3 节 |
| `crates/soul-graph/src/correct.rs` | 新增 | `correct_tie` / `release_tie`：写 `UserCorrection` 证据行 → 改 `band` / `locked_by_user` / `user_band` / `machine_band` → 记 `user_verdict` → 记审计（`ProfileCorrect`，只有 id，没有正文）。`corrected_relationship` 是 rebuild 用来把纠正行也算进边的引用清单的谓词 |
| `crates/soul-graph/src/view.rs` | 改 | `read_strength` 提出来共用（`build` 与 `correct` 读同一份反序列化，读不出来就报错而不是猜） |
| `crates/soul-graph/src/error.rs` | 改 | 新增 `UnreadableInteraction { evidence_id }`：时间戳读不出来的证据让 rebuild 失败并点名那一行，不按猜测折进去 |
| `crates/soul-graph/src/lib.rs` | 改 | `pub mod correct;` / `pub mod t4d_adapt;` 与相应 re-export |

### 1.2 `soul-import` — G1+ 群消息归因

| 文件 | 状态 | 做了什么 |
| --- | --- | --- |
| `crates/soul-import/src/commit.rs` | 改 | `peers` 的分支从 `match sender.is_owner` 改成 `match (sender.is_owner, message.group)`。`(true, true)` → `Vec::new()`：owner 发到群里的一条消息不再对每个历史发言人各写一行 Outgoing。`(true, false)` 一对一照旧，`(false, _)` incoming 仍记实际发送者 |
| `crates/soul-import/src/soul_import_v1.rs` | 改 | 注释同步（群消息的下游语义变了） |

### 1.3 `soul-draft` — A2 渲染与锁定边文案

| 文件 | 状态 | 做了什么 |
| --- | --- | --- |
| `crates/soul-draft/src/a2_adapt.rs` | 新增 | `TieEdge` → 冻结 A2 渲染器的适配器。`is_locked_by_user` / `venue_split` / `as_of_unix` 现在真的从 `TieStrength` 读，不再是 TODO(G1) 桩 |
| `crates/soul-draft/src/analysis.rs` | 改 | 句子交给冻结渲染器；本 crate 只留两件渲染器不知道的事：哪些证据行可以被引用（纠正行不算「往来次数」的依据），以及 GC-9a——锁定边跳过 `FILED_BAND_KEY` 那条 P5 句 |
| `crates/soul-draft/src/lib.rs`、`Cargo.toml` | 改 | 模块声明与 `soul-algo-tie`（dev）/ `soul-algo-trait`（普通）依赖 |

### 1.4 工具与契约

| 文件 | 状态 | 做了什么 |
| --- | --- | --- |
| `crates/xtask/src/denylist.rs` | 改 | `EXEMPT_CRATES` 加 `soul-algo-tie` / `soul-algo-trait`（冻结 crate 的 API 早于本审计）；新增 `EXEMPT_FILES`，只豁免 `crates/soul-graph/src/t4d_adapt.rs` 与 `crates/soul-draft/src/a2_adapt.rs` 两个适配器文件。整 crate 豁免会把字段名、statement key 与用户读到的句子一起放过，按文件豁免不会 |
| `crates/soul-schema/tests/schema_wiring.rs` | 改 | `NOT_ENTITY_IDS` 收 `algorithm_id`：它是 `["T4D","T4"]` 闭词表，不是 UUIDv7 引用 |
| `crates/soulcore/src/commands/graph.rs` | 改 | `correct_tie` / `release_tie` / `band_named` 透传；`TieEdgeView` 多 `locked_by_user` / `user_band` / `machine_band` 三个 token。**没有新增 tauri command**，`COMMANDS` 仍是 36 |
| `crates/soul-algo-tie/Cargo.toml`、`crates/soul-algo-trait/Cargo.toml` | 改 | 只加 `license.workspace = true` |
| `crates/soul-algo-tie/Cargo.lock`、`crates/soul-algo-trait/Cargo.lock` | 删 | workspace 成员的嵌套 lock 文件是死文件。**两个 crate 的判档逻辑一个字节都没动** |

### 1.5 测试

| 文件 | 状态 | 覆盖 |
| --- | --- | --- |
| `crates/soul-graph/tests/t4d_band.rs` | 新增 | 全库 `as_of`、分列计数往返、降档闭区间、任一场地近因时钟、坏时间戳报错、legacy 边升级、锁定边、**整张冻结夹具表回放**、AC-28 / AC-29 / AC-30 具名门 |
| `crates/soul-graph/tests/t4d_product.rs` | 新增 | 产品边界的档位阶梯（次数门、自然日门、两道沉寂门），全部走 SQLCipher + `rebuild` |
| `crates/soul-graph/tests/graph_correction.rs` | 新增 | GC-1..GC-5 / GC-10：纠正落地、rebuild 不撤销、释放、纠正行是边引用的证据、legacy 边也能纠正 |
| `crates/soul-draft/tests/locked_tie_summary.rs` | 新增 | GC-9a：锁定边的 P5 句消失、其余句一字不改、释放后 P5 句回来 |
| `crates/soul-draft/tests/day_constants_agree.rs` | 新增 | D52：`DEMOTE_ONE_BAND_DAYS == DORMANT_AFTER_DAYS`，且产品树里没有第三份数字 |
| `crates/soul-draft/tests/people_summary.rs` | 改 | AC-33 两侧（带分列 / 分列缺席）+ AC-28 的同屏文案 |
| `crates/soul-import/tests/import_to_graph.rs` | 改 | AC-34：owner 群消息不产生 Outgoing 行，且 A/B 的 `last_contact` 不被这条消息刷新 |
| `crates/soulcore/tests/graph_correction_commands.rs` | 新增 | GC-8：命令面上的纠正、审计条目、三 token 的 `TieEdgeView` |
| `crates/soulcore/tests/session_import.rs` | 改 | `evidence_written` 从 `>= 16` 改成 `15`，跟上新的群消息归因 |
| `crates/xtask/tests/self_test.rs` | 改 | 豁免名单的自测：算法 crate 与两个适配器文件豁免，同 crate 内其他文件不豁免 |

`crates/soulcore/tests/session_screens.rs` 等既有诚实文案测试**一个都没删**。唯一删掉的两个文件是上面那两个嵌套 `Cargo.lock`。

---

## 2. 验收行对照

| 行 | 在哪里 | 说明 |
| --- | --- | --- |
| AC-28 | `t4d_band.rs::the_decisive_fixture_is_not_a_close_tie_at_the_store` + `people_summary.rs::the_decisive_fixture_reports_both_venues_on_one_screen` | 夹具 `group_heavy_plus_one_direct_each_way` 从 `soul_algo_tie::testing` 导入；边上落齐四个分列计数与 `direct_active_day_count`；摘要同屏报「一对一往来 2 次，群里同场 30 次」。同一份 log 交给 `TieAlgo::T4` 判 Strong 也一并断言了——口径改回任一场地计数必然让这行变红 |
| AC-29 | `t4d_band.rs::the_anchor_fixture_is_untouched_and_the_way_back_up_is_open` | `lilei_12` 仍 Strong，`group_heavy_plus_three_directs` 回 Moderate，同一测试内 |
| AC-30 | `t4d_band.rs::a_store_holding_an_active_tie_and_a_2019_one_uses_a_single_as_of`、`one_store_wide_as_of_demotes_the_dormant_not_the_active` | `dormant_2019` + `lilei_12` 同库；两条边的 `as_of_utc` 相等且随边落库；休眠边 Weak，`silent_days` 大于沉寂下限 |
| AC-32 | `graph_correction.rs`、`locked_tie_summary.rs`、`soulcore/tests/graph_correction_commands.rs` | 生效档 = 用户档；`machine_band` 另存且 rebuild 继续更新；图与 A2 只消费 `band`；审计记 `ProfileCorrect` 且无正文；COPY_ZH 未批准前锁定边**不渲染** P5 原句 |
| AC-33 | `people_summary.rs::a_rebuilt_edge_says_the_split_the_rule_banded_on` / `an_edge_that_carries_no_venue_split_claims_none` | 带分列的边渲染 P1b；分列缺席（未声明 `algorithm_id` 的遗留边）整句不出现 |
| AC-34 | `import_to_graph.rs::an_owner_group_message_does_not_write_one_outgoing_row_per_speaker`、`what_the_user_says_to_a_group_is_attributed_to_nobody` | 新造的只是那份群聊导出样本，没有顺带新造阈值 |

另外加了一条不在矩阵里但值钱的：`t4d_band.rs::every_frozen_fixture_bands_the_same_through_the_store` 把 `soul_algo_tie::testing::all()` 的**每一条**夹具写进 SQLCipher、rebuild、读回边，和 `soul_algo_tie::score` 直接算的结果逐字段比对（band / 四个分列 / 两种自然日 / 次数 / `silent_days` / `as_of`）。适配器丢了一个场地、把某个瞬间取整错了、或者让 `as_of` 变成 per-peer，整张表会一起红，而不是等谁想起来手抄哪一条。

---

## 3. 两个值得单独说的实现决定

**红线 11 的执行。** 产品 crate 里现在只剩一处 `180` 字面量，在 `day_constants_agree.rs` 第 50 行的文档注释里，写的是「一个 `const DORMANT_DAYS: i64 = 180;` 会怎样绕过这个测试」——是反例说明，不是阈值。`360` / `179` / `359` 一个都没有。移植过来的 `t4d_band.rs` 原本在 `demotion_edges_are_closed_at_the_store` 里写了 `179 / 180 / 359 / 360` 四个数，已经改成从 `DEMOTE_ONE_BAND_DAYS` / `WEAK_AFTER_SILENT_DAYS` 推日期；`the_graph_source_holds_no_second_threshold` 里那张 `">= 180"` 的针串也改成从常量 `format!` 出来，免得这个禁止第三份拷贝的测试自己变成第三份拷贝。`t4d_product.rs` 整个重写成从 `soul_algo_tie::constants` 取门槛、从 `soul_algo_tie::testing::at` 取瞬间，所以「差一天就到降档日」这句话在冻结数字改变时仍然说的是它字面的意思。

**`TieStrength` 的手写 serde（`model.rs::Wire`）。** 契约 `relationship.schema.json` 有 `dependentRequired`（`direct_out_count` 等一律要求 `algorithm_id` 在场）与 `if(algorithm_id) then required[...]`（声明了算法就必须整套可复核面在场，「不允许半迁移」）。derive 出来的序列化会给遗留边写 `direct_out_count: 0` 而没有 `algorithm_id`，也会给它写 `algorithm_id: ""` —— 两种都过不了 schema。真实触发路径是**在任何 rebuild 之前纠正一条遗留边**，我加的 `graph_correction.rs::a_pre_rule_edge_can_be_corrected_before_anything_rebuilds_it` 先把它钉红了：

```
ContractViolation("relationship.schema.json rejected the instance:
  - \"\" is not one of [\"T4D\",\"T4\"] at /tie_strength/algorithm_id
  - \"as_of_utc\" is a required property at /tie_strength")
```

修法是让整个冻结规则面按 `algorithm_id` 是否为空整体在场或整体缺席。语义上这也正是 AC-33 那句「分列缺席只指未声明判档算法的遗留边」要的：没测过的数不是测出来的 0。`model.rs` 里两个单元测试（`an_unmeasured_split_is_absent_rather_than_zero` / `a_measured_split_is_written_out_in_full_zeroes_included`）钉住两个方向。

---

## 4. 怎么跑测试

```bash
# 全仓（117 个测试二进制）
cargo test --workspace --all-targets

# 本任务改动最密的四个包
cargo test -p soul-graph  --all-targets
cargo test -p soul-draft  --all-targets
cargo test -p soul-import --all-targets
cargo test -p soulcore    --all-targets

# 单个门
cargo test -p soul-graph  --test t4d_band
cargo test -p soul-graph  --test t4d_product
cargo test -p soul-graph  --test graph_correction
cargo test -p soul-draft  --test locked_tie_summary
cargo test -p soul-draft  --test day_constants_agree
cargo test -p soul-import --test import_to_graph

# 红线与契约
cargo run -p xtask -- all          # e0-audit / denylist-audit / schema-freeze --check
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

# 仓库既有的组合口令
just lint && just schema && just e0 && just denylist && just test
```

本机跑过的结果（Linux，`cargo 1.83.0`）：

- `cargo test --workspace --all-targets` — 117 个测试二进制全部 `test result: ok`，0 failed。
- `cargo clippy --workspace --all-targets -- -D warnings` — 干净。
- `cargo fmt --all -- --check` — 干净。
- `cargo run -p xtask -- all` — `e0-audit` 16 个 crate / 210 个文件 clean；`denylist-audit` 94 词 / 117 文件 clean；`schema-freeze` 与锁一致。

---

## 5. 已知未完成 / 未验证

1. **桌面壳（`apps/desktop/src-tauri`）在本环境编不了。** `gdk-3.0` 等 GTK 系统库缺失，`cargo test` 在 `gdk-sys` 的 build script 就停了。所以 `command_surface` / `ipc_roundtrip` / `no_egress_path` / `one_store` / `shell_is_local_only` 这五个壳测试**没有在这轮跑过**。静态看是安全的：没有新增 tauri command（`COMMANDS` 仍 36），`TieEdgeView` 只是多了三个字段，而 `apps/desktop/src/core.ts` 的 `TieEdge` 接口对多出来的 JSON 键是宽容的，`ipc_roundtrip.rs` 也没有钉死 `TieEdgeView` 的键集合。但这是推理，不是绿。**需要在有 GTK 的机器上补跑一次。**
2. **前端 vitest 没跑。** 同上，`ui-lint` / `ui-test` 不在本轮验证范围内。`Graph.test.tsx` 目前不认识 `locked_by_user` / `user_band` / `machine_band` 这三个 token，界面上「这一档是你自己定的」怎么画还没有人做——这是 UI 侧的后续工作，不是本任务范围。
3. **锁定边的 P5 文案是「静默」而不是「改写」。** `docs/algorithms/COPY_ZH.md` 冻结且没有「由你本人指定」的变体键，所以按 D35 / D48 现在的做法是：锁定边整条归档句不出现。用户仍能从图视图拿到 `band` / `locked_by_user` / `machine_band` 三个 token 自己组句，摘要里什么都没有被藏起来，但**摘要确实少了一句**。COPY_ZH 加性批准之后要回来补 `a2_adapt` 的键映射与 `locked_tie_summary.rs` 的断言。
4. **T4 回退没有产品侧开关。** `soul_algo_tie::TieAlgo::T4` 只在 `t4d_band.rs` 的 AC-28 断言里被直接调用（用来证明那个缺陷仍会被 T4 触发）。产品路径走的是 `soul_algo_tie::score`，也就是 crate 内部的 `DEFAULT`。DECISION.md 第 5 节说的「档内一行回退」因此是**改冻结 crate 一行**，不是改 `soul-graph`——这是有意的，但如果验收期望在产品侧能选算法，那还没有。
5. **`silent_days` 的类型是 `i64`，schema 要求 `minimum: 0`。** 目前 `soul_algo_tie` 不会返回负值（`as_of` 是全库最大值，不可能早于任何一条 `last_contact`），所以写不出负数。但类型层面没有挡住，靠的是上游不变量而不是本地约束。
6. **`Goal 2` 没有启动，`AC-27` 文件写执行没有做。** 按要求。
7. **同工作区里有兄弟代理在改 `soul-profile` 相关文件。** 本轮的全仓绿是**含**他们当时工作树状态跑出来的。如果他们之后又改了东西，`cargo test --workspace` 要重跑。我没有 commit、没有 push、没有切分支。
