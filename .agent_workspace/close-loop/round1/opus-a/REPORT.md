claude-opus-5-thinking-high-fast

（自证依据：系统标识为 Claude Opus 5，thinking 开启，requested slug 即上行。`cursor-cloud-run-info`
的 `originalModelName` 为 `null`，云端未回吐 slug，故此行取自本 agent 自身系统标识，无降级。）

# Round 1 opus-a — R-1 修复报告

分支 `cursor/goal1-close-loop-a073`，提交 `9fd6870`。只改 soul-graph，schema 未动。

## 修了什么

`.agent_workspace/plan-polish/round3/fable-b/REGRESSIONS.md` R-1：Round 2 把
`tie_strength` 收紧成带 `algorithm_id` 枚举（`T4D`/`T4`）+ `dependentRequired` 之后，
`correct_tie` / `release_tie` 对一条**从未 rebuild 过的八字段 legacy 边**读全量 `TieStrength`
→ 改锁定字段 → 整包写回，产出 `algorithm_id: ""`，被 `put_relationship` 的 schema 校验拒掉。

实测复现（把修复临时短路后跑新测试拿到的原始报错）：

```
Store(ContractViolation("relationship.schema.json rejected the instance:
  - \"\" is not one of [\"T4D\",\"T4\"] at /tie_strength/algorithm_id
  - \"as_of_utc\" is a required property at /tie_strength"))
```

即 R-1 的两条都真实命中：枚举拒空串，且即便跳过空串序列化，`dependentRequired` /
`if algorithm_id then required` 仍会因缺 `as_of_utc` 拒收。

## 怎么修的（rebuild-first，带具名拒绝兜底）

`crates/soul-graph/src/correct.rs` 新增内部函数 `scored()`，`correct_tie` 与 `release_tie`
都从它取边：

1. 读边、读 `TieStrength`。`algorithm_id` 非空 → 原样返回（这是任何跑过一次 rebuild 的库里的每条边，
   常规路径零行为变化、零额外读写）。
2. `algorithm_id` 为空（legacy 八字段边）→ 先跑 `crate::build::rebuild`，并把它欠审计链的
   `AuditContent` 用 `append_or_store_error` 落链（`correct_tie` 本来就有 `AuditLog` 约束）。
   锁定/释放随后落在 rebuild 留下的 T4D 整行上。
3. rebuild 之后仍为空（该 peer 的观测已不在库里，通常连 contact 行一起被遗忘）→
   返回新错误 `GraphError::UnscoredEdge { relationship_id }`，错误文案点名 rebuild：
   「…a rebuild did not score it; the observations behind this tie have to be in the store and
   **rebuilt** before the band can be corrected」。

顺序上 rebuild 发生在本模块写任何自己的东西（证据行、verdict、审计条）之前，所以第 3 步的拒绝
路径**不留纠正痕迹**——GC-4「a refused correction leaves no trace」的同类性质在 legacy 路径上
也成立（新测试逐项断言：evidence / inference / audit 计数与边的 JSON 全部原样）。

明确**没有**做的事：没有放松 schema（`relationship.schema.json` 一字未动）；没有给 T4D 行的零值加
`skip_serializing_if`；没碰 `soul-algo-*`；没加第三道降档门；没开 Goal 2；没动 ci.yml /
PLAN_INDEX / DECISIONS / FORMAL / 其他 slot。

## 改了哪些文件

| 文件 | 改动 |
|---|---|
| `crates/soul-graph/src/correct.rs` | 新增 `scored()`；`correct_tie` / `release_tie` 改从它取边；文档注释说明为什么不能直接写锁 |
| `crates/soul-graph/src/error.rs` | 新增 `GraphError::UnscoredEdge { relationship_id }`，文案点名 rebuild |
| `crates/soul-graph/tests/graph_correction.rs` | 新增 legacy 八字段夹具 `pre_wiring_edge()` 与两条回归测试 |

`GraphError` 无 `#[non_exhaustive]`，但全树没有对它做穷尽 `match` 的地方
（`soulcore::commands::session` 走 `error.to_string()`，`draft` 走 `#[from]`），加变体不破坏下游。

## 新增的回归测试

`crates/soul-graph/tests/graph_correction.rs`：

- `correcting_a_pre_wiring_edge_scores_it_before_locking_it`（好路径）——库里放 R-1 原样的八字段
  JSON（`band: moderate` + 七个计数，无分列计数、无 `as_of_utc`、无 `algorithm_id`），先断言读回来
  `algorithm_id == ""`，再直接 `correct_tie(.., Strong, NOW)`。断言：调用成功；写回的
  `SoulRelationship` 经 `SchemaSet::validate_model(SchemaId::Relationship, ..)` 通过冻结文档校验；
  `algorithm_id == "T4D"`；锁定字段齐全（`band=Strong` / `user_band=Some(Strong)` /
  `locked_by_user=Some(true)` / `machine_band=Some(Weak)`）；计数是 rebuild 的（1 / direct_out 1），
  不是 legacy 行声称的 4；`as_of_utc` 有值；inference 的 verdict 为 `Corrected`；审计链两条
  （rebuild 的 + 纠正的），即被迫跑的 rebuild 不是静默的。
- `an_edge_no_rebuild_can_score_is_refused_and_names_the_rebuild`（坏路径）——同样的 legacy 边，
  但 peer 的 contact 行不存在（已遗忘），rebuild 无从评分。断言 `Err(GraphError::UnscoredEdge)`
  且点名了这条边、文案含 `rebuilt`；evidence / inference / audit 计数与边 JSON 全部未变；
  band 仍是 legacy 的 `moderate` 且未锁；`release_tie` 同样拒绝。

**反向验证**：把 `scored()` 的 rebuild 分支临时短路（`if true { return ... }`）重跑，两条新测试
双双 FAILED 并给出上文那段 `ContractViolation`，其余 9 条 graph_correction 用例仍全绿——证明新测试
真的在守 R-1，而不是恰好通过；也证明既有用例（都经 `seeded()` 先 rebuild）确实照不到这条路径，
与 fable-b「CI 不会红」的判断一致。恢复修复后 11/11 全绿。

## 跑的命令

```
cargo test -p soul-graph --offline
```

结果：`graph_correction` 11 passed（原 9 + 新 2）、`t4d_band` 11 passed、`t4d_product` 12 passed、
`ego_graph` 7 passed、单元 2 passed，0 failed。

额外自查（非要求，防下游回归）：

```
cargo clippy -p soul-graph --offline --all-targets -- -D warnings   # clean
cargo fmt -p soul-graph
cargo test -p soulcore -p soul-draft -p soul-import -p soul-store --offline   # 全绿，0 failed
```

## 留给合并侧的一句话

R-1 可以从 Goal 1 合并检查单里划掉：不再是「合并时记得看一眼」，而是产品路径上已被两条测试钉住。
D59 那条「合并后跑 `xtask schema-freeze` 复核 lock」不受影响——本轮没有改任何 schema 文件，
lock 应当零 diff。
