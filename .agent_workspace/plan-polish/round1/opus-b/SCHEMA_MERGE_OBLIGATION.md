# `relationship.tie_strength` 的收紧与合并义务

> 目标落位：`docs/schemas/README.md` 的一节，或 `docs/DECISIONS.md` 的一条新拍板 + 本文件作为附注。
> 依据：`docs/algorithms/DECISION.md`（`ALGO_FROZEN`）第 3、6 节；探针 `probe/verify_relationship_schema.py`（13/13）。

## 0. 一句话

`tie_strength` 现在是裸 `object`，任何东西都塞得进去——包括 T4D 明令禁止的分数。候选把它按 T4D 的可复核面**类型化**，同时**不动任何现有 `required`**，所以今天 Goal 1 落库的边照样通过校验；代价是 `schemas.lock.json` 的一个哈希必须重算，这是合并义务，不能静默做。

## 1. 事实（可复现）

```
$ python3 probe/verify_relationship_schema.py
goal1 relationship.schema.json : df34747e28ef2e01ff02f094b26a2b7722c096256563474835f2c61f1ec122ff
schemas.lock.json pins         : df34747e28ef2e01ff02f094b26a2b7722c096256563474835f2c61f1ec122ff
candidate                      : 63c424fe4070e8f2c47db8a74c8803142cc31d5e5bc44c574fac4648d542611e
```

- Goal 1 分支 `docs/schemas/schemas.lock.json` 钉住 11 份文档的 sha256，CI 用 `cargo run -p xtask -- schema-freeze --check` 校验。
- 候选改了 `relationship.schema.json` 的正文，所以**必然**改哈希。哈希不重算，Goal 1 那条线的 CI 直接红。这是设计好的刹车，不是缺陷。
- `_defs.schema.json` 两条线上字节相同（`5e56…be7e7`），所以本次只有 `relationship.schema.json` 一个条目要动。

## 2. 候选改了什么、没改什么

**没改（因此不是破坏性变更）：**

| 不变量 | 状态 |
|---|---|
| 顶层 `required` 五项 | 与 Goal 1 逐字相同（探针断言） |
| `tie_strength` 是否必填 | 仍然不必填 |
| `schema_version` / `$id` / `title` / `egress_scope` | 未动 |
| `types` 仍是自由数组 | 未动。v0.1 没有推断关系类型的依据，收紧它属于产品变更，不在本次范围 |
| `evidence_ids` / 三个 `*_id` 的 `$ref` 接线 | 沿用 Goal 1 已落地的 `_defs` 写法，不回退 |

**改了：**

1. `tie_strength` 从 `{"type":"object"}` 变成有 `properties` 的对象，`additionalProperties: false`。
2. `tie_strength.required` = Goal 1 今天 `TieStrength` 结构体实际序列化的**那八个字段**：`band` / `interaction_count` / `outgoing_count` / `incoming_count` / `conversation_count` / `active_day_count` / `first_contact_utc` / `last_contact_utc`。一个不多，一个不少——所以今天的输出原样通过（探针 `goal1_today_eight_fields`）。
3. 新增 T4D 的可复核字段，**全部可选**：`algorithm_id`、`direct_out_count`、`direct_in_count`、`group_out_count`、`group_in_count`、`direct_active_day_count`、`last_direct_contact_utc`、`silent_days`、`as_of_utc`。
4. `dependentRequired` + `if/then`：**一旦出现任何一个 T4D 字段，整套必须同时在场**。半迁移（只补一半计数、或声明了 `algorithm_id` 却不落 `as_of`）当场红。
5. `band` 用 `allOf: [$ref evidenceBand, {enum: [weak, moderate, strong]}]`——沿用 WP01 已在 `profile` / `evidence` 上用过的写法：既真的引用 `_defs`，又不放松（`none` 不是关系档）。
6. `algorithm_id` 枚举只有 `T4D` 与 `T4`。`T4` 是 `DECISION.md` 第 5 节档内回退的待命形态；第三套判档规则没法靠新字符串偷渡。

## 3. 为什么不直接把 T4D 字段写进 `required`

因为那会**同时**破两样东西，而且第二样是静默的：

1. Goal 1 今天的 `TieStrength` 只有八个字段，没有分列计数、没有 `as_of`。把它们设成必填，Goal 1 现有的图谱测试与已落库的边全部立刻不合法——在 WP05 完成 T4D 吸收之前，这是把一条绿线改红。
2. 更糟的是**已落库数据**：`relationship` 记录是持久化的，收紧 `required` 等于给旧行追加一个它们不可能满足的条件，而 schema 层面没有迁移概念。

`dependentRequired` + `if/then` 拿到了同样的强度而不付这两笔账：**没有 T4D 就照旧，有 T4D 就必须完整**。今天它禁止半迁移；WP05 落地后它自动变成实质必填。

## 4. 合并义务（谁在什么时候做什么）

| # | 时机 | 动作 | 谁 |
|---|---|---|---|
| M1 | 计划 PR 合入 `main` 时 | 带上候选正文。**`main` 上没有 `schemas.lock.json`**（该文件只在 Goal 1 分支），所以这一步不会让任何 CI 变红 | 计划线父代理 |
| M2 | Goal 1 分支下一次 rebase 到 `main` 之后 | 立刻跑 `cargo run -p xtask -- schema-freeze --write`，把 `relationship.schema.json` 的条目更新为 `63c424fe…`，与 rebase 同一个 PR 提交，PR 描述里引用本文件 | Goal 1 线 |
| M3 | WP05 吸收 T4D（`Tally::band()` 换成 `soul-algo-tie` 的 T4D）时 | `TieStrength` 结构体补齐第 2 节第 3 条那九个字段并序列化；图谱 rebuild 把调用方算出的全库 `as_of` 一路带到边上落库；AC-28…AC-31 转绿 | Goal 1 线 WP05 |
| M4 | M3 合入之后（清理，可选但建议） | 把 T4D 那套字段从 `dependentRequired`/`if-then` 提升为 `tie_strength.required`，删掉过渡期的 `if/then`；再跑一次 `schema-freeze --write`；在 `docs/DECISIONS.md` 追一条拍板记录这次收紧 | Goal 1 线或计划线 |

**M2 不能跳过。** 跳过的表现是 Goal 1 CI 上 `schema-freeze --check` 报 `relationship.schema.json` 内容变更，而不是任何业务测试失败——很容易被误判成「rebase 弄脏了文件」然后被 `git checkout` 掉，把这次收紧一起回退。

## 5. 顺带发现：九份 schema 在两条线上已经分叉

计划分支的 `docs/schemas/` 是 PR #1 的版本（各 `*_id` 是裸 `string`）；Goal 1 在 WP01 里把九份接到了 `_defs` 并钉了 lock。逐份比对：

```
same     _defs.schema.json           same     soul-import-v1.schema.json
DIFFERS  audit  contact  event  evidence  export-manifest  inference  memory  profile  relationship
```

九份里 Goal 1 的版本**严格更紧**（`$ref` 接线、`allOf` 收窄枚举），而且已经被 `schemas.lock.json` 钉住、被 `crates/soul-schema/tests/schema_wiring.rs` 断言过。

**建议**：计划 PR 不要把 PR #1 的旧正文推回 `main`，而是整体采用 Goal 1 的九份正文，再把 `tie_strength` 这一块叠上去。本候选就是这么做的——它的 base 是 `origin/cursor/soul-goal1-7b1c:docs/schemas/relationship.schema.json`，所以与 Goal 1 的 diff 恰好只有 `tie_strength` 一块，M2 只需重算一个哈希。若计划 PR 反而推回旧正文，Goal 1 的下一次 rebase 会在九个文件上产生冲突，而每一次「取 main 的版本」都是在悄悄回退 WP01。

## 6. 探针覆盖

`probe/verify_relationship_schema.py`，13/13，无外网（`jsonschema` 4.26，Draft 2020-12 + `referencing`）：

| 应通过 | 应拒绝 |
|---|---|
| Goal 1 今天的八字段输出 | 只补一半分列计数（无 `algorithm_id`） |
| T4D 完整包（`lilei_12` 形态） | 声明了 `algorithm_id` 却不落分列计数 |
| T4D 完整包（`group_heavy_plus_one_direct_each_way` 判 weak 的形态） | T4D 包缺 `as_of_utc` |
| 仅群聊者：分列计数为 0、`last_direct_contact_utc` 为 `null` | `score` 字段、`tie_weight` 字段（D22） |
| | `band: none`；负数计数；`algorithm_id: T5`；`as_of` 写成 Unix 整数 |

同一批用例喂给 Goal 1 今天的 schema：九条该拒的**一条都没拒**。这就是把裸 `object` 类型化的全部收益。
