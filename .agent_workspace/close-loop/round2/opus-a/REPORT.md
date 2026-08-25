claude-opus-5-thinking-high-fast (requested; the run exposes no slug — `cursor-cloud run-info` returns `originalModelName: null`, and the serving model self-identifies as Claude Opus 5. No downgrade observed, but the exact variant string is unverifiable from inside the run.)

# Round 2 · opus-a · CODE-2

Branch `cursor/goal1-r2-code2-df52`, two commits on top of `032374f`:

- `2d96fa9` test(soul-graph): hold AC-28/29/30 to the named soul-algo-tie fixtures at the product boundary — `crates/soul-graph/tests/t4d_product.rs`
- `057e318` test(soul-graph): name the direct active day count on the decisive Weak edge — `crates/soul-graph/tests/t4d_band.rs`

## 做了什么

产品边界（`put_evidence` → `soul_graph::rebuild` → `soul_graph::load` 读边）上新增四个测试，全部**导入** `soul_algo_tie::testing` 的具名夹具，一条阈值算术都没有重推。

新增的公共设施（都在 `tests/t4d_product.rs` 内，没有动 `src/`）：

- `load_fixture(store, peer_uuid, &Fixture)` —— 把夹具的 `log` 逐条写成 evidence，方向/场地/时刻原样搬运；会话名带夹具名前缀，两个夹具同库不会撞会话 id。
- `rfc3339_utc(unix)` —— 用冻结 crate 自己的 `civil_from_epoch_day` / `epoch_day` / `SECONDS_PER_DAY` 把秒写成库里那种字符串。日历取自算法 crate，正是 `t4d_adapt.rs` 读回来的那一套。
- `assert_edge_matches_frozen(strength, fixture, as_of)` —— 对拍：落库边的 band、`algorithm_id`、四个分列计数、三个总计、`active_day_count` / `direct_active_day_count`、`silent_days`、first/last/last-direct/as_of 四个时刻，逐项等于 `soul_algo_tie::score(fixture.peer_id, &fixture.log, as_of)`。期望值向冻结 crate 要，不在 Goal 1 侧写死。

| 测试 | 行 | 断言 |
|---|---|---|
| `the_fixture_loader_preserves_the_instant_and_the_venue` | 装载器自证 | `rfc3339_utc(AS_OF_2026_08_24) == "2026-08-24T14:00:00Z"`；`lilei_12` 单夹具入库后与冻结 crate 全等 |
| `the_decisive_group_flood_edge_is_weak_and_carries_its_direct_active_day_count` | AC-28 | `group_heavy_plus_one_direct_each_way` → Weak；`direct_active_day_count == 1` 且 `< active_day_count == 10`；分列 (1,1)/(15,15)；`as_of_utc` 全库单一；`TieAlgo::T4` 对同一份证据判 Strong（口径回退即红的实证） |
| `the_anchor_stays_strong_while_three_directs_reach_moderate_in_one_rebuild` | AC-29 | 同一测试、同一个库、同一次 rebuild：`lilei_12` → Strong，`group_heavy_plus_three_directs` → Moderate；两个 group-heavy 夹具的群聊分列全等，差别只在一对一（3 条 / 2 天） |
| `both_edges_carry_the_same_store_wide_as_of_and_only_the_dormant_one_is_weak` | AC-30 | `lilei_12` + `dormant_2019` 两条边一个库；两边 `as_of_utc` 取自 `BTreeSet` 且只有一个元素，等于 `as_of_max(两份 log 拼接)`；休眠边 Weak、`silent_days > 2500`，活跃边 Strong、`silent_days == 0`；per-peer `as_of` 下同一份 2019 证据被冻结 crate 判 Strong（改 per-peer 即红的实证） |

夹具的 `as_of` 是 `AS_OF_2026_08_24`，而 rebuild 默认取全库 `max(occurred_at)`。AC-28 / AC-29 因此加了一个只有一行、时刻正好落在 `2026-08-24T14:00:00Z` 的锚点 peer，让全库 `as_of` 就是夹具自己的 `as_of`；那行字符串由上面第一个测试对着 `AS_OF_2026_08_24` 钉住，不是手抄的。AC-30 反过来走成文的缺省口径（全库 `max`），期望值同样向冻结 crate 要，所以两条路都覆盖到了。

`t4d_band.rs` 里那条手工搭的群聊洪水用例原本断言了 AC-28 列的其余每个数，独独漏了判档真正踩的那个；补上 `direct_active_day_count == 1` 与 `< active_day_count`。

## 反证（临时改 src，跑完即 `git checkout --` 还原，工作树已确认干净）

两个探针都真的把新行打红了，不是空过：

1. `build.rs` 的 `as_of` 改成 per-peer `as_of_max(&acc.rows)`：`t4d_product` 16 中 7 红，含 AC-30（`one rebuild, one as_of, written onto every edge it touched`）、AC-28、AC-29（`lilei_12: silence`）。
2. `t4d_adapt.rs::tie_reading` 改调 `TieAlgo::T4`（判档口径回到「任一场地计数」）：AC-28 决胜边 `left: Strong / right: Weak` 变红，AC-29 自愈边同样从 Moderate 变 Strong 变红；`t4d_band.rs` 那条也红。

## 测试

`cargo test -p soul-graph --offline`（在 `/workspace`）：

```
unittests src/lib.rs        2 passed
tests/ego_graph.rs          7 passed
tests/graph_correction.rs  11 passed
tests/t4d_band.rs          11 passed
tests/t4d_product.rs       16 passed   (原 12 + 新 4)
doc-tests                   0 passed
```

0 failed。`cargo fmt -p soul-graph -- --check` 干净，`cargo clippy -p soul-graph --tests --offline -- -D warnings` 干净。

## 禁区

没有碰 `soul-algo-*` 的常量或任何 `src/` 判档代码（两个探针已还原，`git status` 已确认）；没有第三道降档门；没有 Goal 2；没有 `ci.yml`；没有 `import_to_graph.rs`；没有 `DECISIONS.md`；工作区 `Cargo.lock` 未动。改动只有 `crates/soul-graph/tests/` 两个文件，加本报告。

## 给父代理的两条环境事实

1. **shell 会话是和 opus-b 共用的。** 本轮中途 `pwd` 从 `/workspace` 变成了 `/tmp/wt-opus-b`（opus-b 的 worktree），一次 `cargo test` 因此跑在了对方树上、读出的是对方的文件。此后每条命令都显式带 `working_directory: /workspace`。后续派单最好给每个槽位单独的 shell，或在 prompt 里要求一律用绝对路径。
2. **分支基点。** 我从 `/workspace` 当时的 HEAD `032374f` 切出，而 `origin/cursor/goal1-close-loop-a073` 的 tip 是 `d3b83f3`，`032374f` **不是**它的祖先——切branch时本地树上还压着未推的兄弟槽位提交（opus-b 的 AC-34 `import_to_graph.rs`、gpt-sol-a 的报告文件）。所以本分支除了我那两个提交，还驮着 `032374f`。按「不许 force push」我没有 rebase；父代理若要干净的 delta，把 `2d96fa9` `057e318` 两个提交 rebase 到 `d3b83f3` 即可，二者与 `032374f` 无内容依赖。
