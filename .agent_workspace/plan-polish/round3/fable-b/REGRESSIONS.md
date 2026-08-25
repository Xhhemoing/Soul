# Round 3 fable-b — Round 2 引入/暴露的回归

核于 2026-08-25，基线 `c3d5960`。四条，无一条阻塞本计划 PR 合 `main`；R-1 需要在 Goal 1 合并检查单里留一句话，R-4 已被同树在飞改动修正（未提交）。同树并发处置见 REPORT.md；R-1/R-2/R-3 涉及的行在在飞 diff 中均未被触碰，判定对两种树状态同时成立。

## R-1（P2，潜在运行期红点）：新 schema 拒绝 legacy 边经 `correct_tie`/`release_tie` 的写回

**这是 Round 2 收紧带来的新行为**——收紧前 `tie_strength` 是裸 object，什么都过。

事实链（全部实测于 unblock 线 `c81c233`）：

1. `crates/soul-graph/src/model.rs:106`：`algorithm_id: String` 带 `#[serde(default)]` 但**没有** `skip_serializing_if`——每次序列化必然带上它，legacy 行为空串 `""`。分列计数、`silent_days` 同理恒序列化。
2. `crates/soul-graph/src/correct.rs`：`correct_tie`/`release_tie` 读出整个 `TieStrength`、改锁定字段、经 `rewritten()` 整包 `serde_json::to_value` 写回，`crates/soul-store/src/store.rs:785` 的 `put_relationship` 会先 `check(SchemaId::Relationship, …)` 做 schema 验证。
3. 对 pre-wiring 的 legacy 边（库里只有八字段 JSON），这条写回路径产出 `algorithm_id: ""` + `direct_out_count: 0` 等——本探针 `reject/empty_algorithm_id` 证实新 schema 拒绝该形态（枚举只有 `T4D`/`T4`；即便跳过空串序列化，`dependentRequired: direct_out_count → algorithm_id` 也会拒）。`correct.rs` 的 `machine_reading()` 注释明说这条 legacy 纠正路径是**有意支持**的。

**为什么不是 P0 复活**：
- CI 不会红：`graph_correction.rs`、`locked_tie_summary.rs` 的全部纠正用例都经 `seeded()` 先 `rebuild` 再纠正，写回的是全字段 T4D 形态（本探针 `accept/rebuilt_locked_edge_full_lock_fields` 过）；`t4d_band.rs` 的 legacy 夹具只 put 八字段原文（valid）后即 rebuild。
- 现实中无 legacy 库：应用从未发布，pre-wiring 数据只存在于测试临时目录。

**修法建议（留给 Goal 1 合并侧，一句话进检查单即可）**：合并本 schema 时，`correct_tie`/`release_tie` 对 `algorithm_id` 为空的边先触发 rebuild（或拒绝并提示 rebuild），或给 `algorithm_id` 补 `skip_serializing_if = String::is_empty` 并把分列计数改为随 algorithm_id 缺席而跳过。D59 已写「Goal 1 合并后跑 `xtask schema-freeze` 复核 lock」，建议届时在同一检查单补上本条。**本轮不改 docs**（Round 3 各路只读）。

## R-2（P2，规则与冻结文档冲突）：PLAN_INDEX 新增的「其余文档写出数字即为缺陷」把冻结算法文档判成缺陷

Round 2 在 PLAN_INDEX 26 行新增查询入口：「判档阈值……只有两处：`algorithms/DECISION.md` 第 3 节常量表（语义）与 `crates/soul-algo-tie` 常量模块（取值）。**其余文档一律只写常量名，写出数字即为缺陷**（FORMAL 红线 11）」。

按字面执行，以下**冻结在先**的文件立刻不合规（本轮实测命中）：

- `docs/algorithms/COPY_ZH.md:70`「超过 180 天」、`:73`「`DORMANT_NOTE_DAYS = DEMOTE_ONE_BAND_DAYS = 180`」——第三、四处写出取值；
- `docs/algorithms/REJECTED.md:37`、`:86`「180/360」；
- `docs/algorithms/DECISION.md:39`（§1 叙述「180/360 整数降档」，在授权的「第 3 节常量表」之外）。

两点错位：其一，新规把授权域收窄到「第 3 节常量表」，但同文件 §1 与姊妹文件 COPY_ZH/REJECTED 早已带数字；其二，新规声称语义出处是 FORMAL 红线 11，而红线 11 的原文管的是**产品 crate 源码与注释**（`graph_build`/A2/UI/SQL），不管 docs——引用扩大了原规则的辖域。这不是要去改冻结算法文档（`ALGO_FROZEN`，且那些数字是裁决记录本身），而是 Round 2 的新措辞需要在下一次触碰 PLAN_INDEX 时收窄为「`docs/algorithms/` 三份权威除外」或同义豁免，否则第一个较真的审计者会拿这行判冻结文档缺陷。

## R-3（P3，纪律自洽小裂缝）：D57 原行内增补，STATUS 的 D57 复述未同步

Round 2 把「同一棵树内对同一分支的『核于』提交号必须一致」**改写进 D57 原行**（RESIDUAL P1-6 的处方原文如此），而同一份 PR 里 P1-3 的处理原则是「拍板只追加、D58 原文不改」。两种手法并存：追加派生出 D59，增补改写了 D57。历史读者 diff DECISIONS 时会看到 D57「原话变了」。

连带：`docs/STATUS.md` 61–66 行「本文件的写法纪律（D57）」逐条复述 D57，**没有**同步这半句。按 D41/PLAN_INDEX 的复述让位规则不构成矛盾，但「同一提交改了权威行却没改自己树里的复述」正是 Round 2 在 P1-2 里刚修完的那类账。下次刷新 STATUS 时补一行即可（在飞的 STATUS 改动也没补它）。

## R-4（P2，已被同树在飞改动修正）：FORMAL 夹具溯源句把 AC-34 也说成「夹具不是新造的」

Round 2 在加 AC-34 时，把夹具溯源段的范围从「AC-28…AC-33」顺手扩成「AC-28…AC-34 测在产品边界…**夹具不是新造的**：`group_heavy_plus_one_direct_each_way`、`lilei_12`、…已在 `crates/soul-algo-tie/src/testing`」（提交版 161 行，diff 证实该范围是本轮改出来的）。实测 `crates/soul-algo-tie/src/testing/` 只有 `mod.rs` 与 `oracle.rs`，四个具名夹具全在 `mod.rs`，**没有** AC-34 需要的群聊导出样本——R2-SYNTHESIS 风险 3 自己就说「AC-34 依赖导入归因，不是算法 crate 夹具」。照字面执行的 Goal 1 实现者会去算法 crate 里找一份不存在的夹具。同理，矩阵导语「`ALGO_FROZEN` 相关行（AC-28…AC-34）」把导入归因行也归进了 `ALGO_FROZEN`。同树在飞改动已把两处改为「AC-34 是例外：样本在 Goal 1 侧新造（只新造导出样本，不得顺带新造第二套阈值）」——待提交生效；若该在飞改动最终没有落进提交，本条回归回到未修状态。

## 检查过、未见破坏的面

- 验收矩阵「只增不删」：AC-01–AC-26 逐行未动（diff 证实）；AC-29 的「（一对一 3 次）」改写为指认夹具名，门禁语义不变；「这六行」→「算法相关行」与 AC-34 的加入自洽。
- schema diff 范围：`relationship.schema.json` 只动 `tie_strength` 块（119 增 / 1 删，删的正是裸 object 行）；lock 只动 relationship 一条；其余 10 份 schema 字节与哈希未动。
- 无 mojibake（全 docs 无 U+FFFD）；FORMAL 引用的五个常量名在 `constants.rs` 全部真实存在（`DEMOTE_ONE_BAND_DAYS`/`FORCE_WEAK_DAYS` 是 57–58 行的公开别名）。
- 信封层未松动：未知顶层属性、`egress_scope ≠ local_only`、空 `evidence_ids` 全拒（探针 C 节）。
- 冻结面未触碰：`docs/algorithms/**`、`docs/PRODUCT_LOCK.md`、`crates/**` 不在 Round 2 提交的改动集里。
