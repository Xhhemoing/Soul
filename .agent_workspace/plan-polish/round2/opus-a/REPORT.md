# Round 2 · opus-a · L1（`tie_strength` 类型化）与 L3（SECURITY 指针）

模型：`claude-opus-5-thinking-high-fast`。无静默降级。
工作树：`cursor/polish-project-plan-5280`。未 commit / push / 开 PR。
核于 2026-08-25。

## 0. 一句话

两处都判定 drop-in 安全，**已直接写进 `docs/`**，本目录留同字节副本。`docs/schemas/relationship.schema.json` 的 `tie_strength` 从裸 `object` 变成按 T4D 可复核面类型化的对象，`schemas.lock.json` 十一份哈希全部重算（实际只有一行变）；`docs/SECURITY.md` 加一节指向 Goal 1 的加密落地 / DPAPI 落地两节，**没有把那两节的实证抄过来**。探针 57/57，基线 31/57。

## 1. 落地清单

| 文件 | 动作 | 本目录副本 |
|---|---|---|
| `docs/schemas/relationship.schema.json` | `tie_strength` 类型化。全文件只删掉一行（`"tie_strength": { "type": "object" },`），其余全是新增 | `relationship.schema.json` |
| `docs/schemas/schemas.lock.json` | 十一份重算，实际变动一行 | `schemas.lock.json` |
| `docs/SECURITY.md` | 新增「实现实证在哪（指针，不是本树结论）」一节，插在「密钥与遗忘」与「审计」之间 | `SECURITY.md` |

未碰 `crates/`。未碰 `docs/` 里其它任何文件（`FORMAL_WORK_PROMPT.md` / `PLAN_INDEX.md` / `PLAN_VERIFY_PROMPT.md` / `STATUS.md` 上的改动是本轮另一位子代理的，我没有覆盖它们）。

哈希变化：

```
relationship.schema.json  df34747e28ef2e01ff02f094b26a2b7722c096256563474835f2c61f1ec122ff   (旧，= Goal 1)
                       →  350a7ffc61ab78652518b07f5bb02fea8ee567d66f3bd5d5f9411364f9752aca   (新)
```

其余十份 `_defs` / `audit` / `contact` / `event` / `evidence` / `export-manifest` / `inference` / `memory` / `profile` / `soul-import-v1` 哈希逐字未变，与 `origin/cursor/soul-goal1-7b1c` @ `6c91d39` 仍然字节相同（逐份 `cmp` 断言过）。

## 2. L1：`tie_strength` 怎么收紧的

基线是 `origin/cursor/soul-goal1-7b1c:docs/schemas/relationship.schema.json`（`6c91d39` 上与本树旧版**字节相同**），所以与 Goal 1 的 diff 恰好只有 `tie_strength` 一块。

**没动：** 顶层 `required` 五项、`tie_strength` 仍不必填、`schema_version` / `$id` / `title` / `egress_scope`、`types` 仍是自由数组、三个 `*_id` 与 `evidence_ids` 的 `_defs` 接线。

**动了：**

1. `tie_strength` 得到 `properties` 与 `additionalProperties: false`。这一条就是 D22 的执行点：`score` / `percentile` / `tie_weight` / `weight` / `rank` 全部当场红。
2. `band` = `allOf: [$ref evidenceBand, {enum: [weak, moderate, strong]}]`。真引用 `_defs`（沿用 WP01 在 `profile` / `evidence` 上的写法），同时把 `none` 排除掉——`none` 不是关系档。
3. `algorithm_id` 枚举只有 `T4D` 与 `T4`。`T4` 是 `DECISION.md` 第 5 节档内回退的待命形态。第三套判档规则没法靠一个新字符串偷渡进来。
4. T4D 可复核字段全部**可选**：`direct_out_count` / `direct_in_count` / `group_out_count` / `group_in_count` / `direct_active_day_count` / `last_direct_contact_utc` / `silent_days` / `as_of_utc`。
5. `dependentRequired`：上面任何一个字段一出现，就必须同时有 `algorithm_id`。
6. `if: {required: [algorithm_id]}` → `then.required` = 基础八项（`band` / `interaction_count` / `outgoing_count` / `incoming_count` / `conversation_count` / `active_day_count` / `first_contact_utc` / `last_contact_utc`）**加**分列四项 + `direct_active_day_count` + `silent_days` + `as_of_utc`。半迁移当场红。
7. 常量与阈值一个都没进文件，只 `$comment` 指向 `docs/algorithms/DECISION.md` 第 3 / 4.3 / 5 节。

### 2.1 与 Round 1 opus-b 候选的唯一差别（有意为之）

opus-b 把 Goal 1 今天实际序列化的那八个字段设成了 `tie_strength.required`。**本轮没有**，八项被移进 `if/then` 的 `then.required`。

为什么改：本轮派发把「**空 `tie_strength` 对象仍然通过**」列为硬要求，而八项无条件必填会把 `{}` 判红。移进 `then` 之后两条同时成立——`{}` 通过（探针 `accept/empty_tie_strength`），Goal 1 今天的八字段输出也通过（`accept/goal1_today_eight_fields`）。

代价与补偿：无条件必填没了，理论上一条 `tie_strength: {}` 的边能过校验。补偿是**收紧点从「字段在不在」挪到了「声明了算法就必须完整」**——一旦 `algorithm_id: "T4D"` 落库，基础八项与 T4D 七项一个都不能少。WP05 吸收 T4D 之后每条边都会带 `algorithm_id`，那时 `then` 分支就是实质必填，强度与 opus-b 候选相同。在那之前，「八个字段在不在」本来也由 Rust 结构体与 serde 保证，不靠 schema 兜底；而「有没有人往里塞分数」只有 schema 能拦，那一条一点没松。

`last_direct_contact_utc` 沿用 opus-b 的处理：进 `dependentRequired`，**不进** `then.required`。它可为 `null`，把它设成必填会跟 `skip_serializing_if = "Option::is_none"` 这类实现打架。

### 2.2 探针

`probe/verify_tie_strength.py`，两段：

- **A 段（锁一致性，只用 `hashlib`，无第三方依赖）**：lock 覆盖盘上每一份、没有幽灵条目、冻结集恰好十一份、键序与 `xtask` 的 `BTreeMap` 一致、每份文件哈希与锁一致。摘要口径与 `xtask::digest_file` 对齐：**先把 CRLF 折成 LF 再哈希**，所以 Windows 检出不会在没有真实改动时把这一段打红。
- **B 段（校验行为，需要 `jsonschema` + `referencing`；缺失则跳过并说明，A 段照跑）**：本机 `jsonschema` 4.26，Draft 2020-12，无外网。

派发点名的四条硬要求：

| 用例 | 期望 | 结果 |
|---|---|---|
| `accept/empty_tie_strength`（`tie_strength: {}`） | 通过 | 通过 |
| `accept/t4d_complete` | 通过 | 通过 |
| `reject/score_field` | 拒绝 | 拒绝 |
| `reject/partial_t4d_extras`（只补两个分列计数） | 拒绝 | 拒绝 |

其余覆盖：

| 应通过 | 应拒绝 |
|---|---|
| 完全没有 `tie_strength` | `algorithm_id` 单独出现 |
| Goal 1 今天的八字段 | T4D 包缺 `as_of_utc` / 缺 `direct_active_day_count` / 缺 `silent_days` / 缺 `band` |
| 仅群聊者（分列计数为 0，`last_direct_contact_utc` 为 `null`） | `silent_days` 或 `as_of_utc` 没带 `algorithm_id` |
| T4D 包省略可空的 `last_direct_contact_utc` | `tie_weight`、`percentile`、`band: none`、负数计数、`algorithm_id: T5`、`as_of` 写成 Unix 整数、`tie_strength` 写成字符串 |

外加结构断言（不依赖 `jsonschema`）：顶层 `required` 逐字未变、`tie_strength` 不在顶层 `required`、本层无无条件 `required`、`band` 枚举排除 `none` 且真引用 `_defs`、`then.required` 含分列计数与 `as_of`、`then.required` 不含可空的 `last_direct_contact_utc`。

结果（`probe/RESULTS.txt` 存了两次完整输出）：

```
基线 Goal 1 @ 6c91d39 的 docs/schemas/ : 31/57 passed
落地后 本分支 docs/schemas/            : 57/57 passed
```

基线那 26 条失败里有 15 条是「本应被拒却通过了」——裸 `object` 一条都没拦住。这就是这次类型化的全部收益。

另跑 `Draft202012Validator.check_schema`：新正文本身是合法的 Draft 2020-12 文档。

`probe/regen_lock.py` 复刻 `xtask -- schema-freeze --write` 的字节形态（CRLF 折 LF 后取摘要、`note`/`algorithm`/`schemas` 键序、`schemas` 内字典序、两空格缩进、末尾补换行、数量必须十一）。本分支没有 `crates/xtask`（只有两个算法 crate），所以 `cargo run -p xtask` 在这里跑不了；对同一份输入连跑两次输出 `unchanged`，说明字节稳定。

## 3. 合并义务（相对 Round 1 的 `SCHEMA_MERGE_OBLIGATION.md` 有更新）

opus-b 那份的 M1 写的是「`main` 上没有 `schemas.lock.json`，所以这一步不会让任何 CI 变红」。**这句现在过时了**：Round 1 已经把 `schemas.lock.json` 种进本分支 `docs/schemas/`，本 PR 合入后 `main` 上就有锁了。修正后的义务表：

| # | 时机 | 动作 | 谁 |
|---|---|---|---|
| M1 | 本计划 PR 合入 `main` | 带上新正文与新锁。`main` 上仍然没有 `crates/xtask`，所以没有 `schema-freeze --check` 会跑，不会变红；但锁与正文本身是自洽的（探针 A 段断言） | 计划线父代理 |
| M2 | Goal 1 下一次 rebase 到 `main` | 冲突只会出现在两个文件：`relationship.schema.json` 与 `schemas.lock.json`（一行）。**两个都取 `main` 侧**。取完 `cargo run -p xtask -- schema-freeze --check` **直接是绿的，不需要 `--write`** ——本 PR 已经把正确哈希算进锁里了。其余九份两条线字节相同，不会冲突 | Goal 1 线 |
| M3 | WP05 吸收 T4D | `TieStrength` 补齐 `algorithm_id` 与那七个可复核字段并序列化；rebuild 把调用方算出的全库 `as_of` 一路带到边上落库。此后 `if/then` 分支自动生效，半迁移会被 schema 挡住；AC-28…AC-31 转绿 | Goal 1 线 WP05 |
| M4 | M3 合入之后（可选清理） | 把 T4D 那套从 `dependentRequired`/`if-then` 提升为 `tie_strength.required`，删掉过渡期分支；再跑一次 `--write`；在 `DECISIONS.md` 追一条拍板 | 任一线 |

**M2 不能被误判。** 它的表现是 `schema-freeze --check` 报 `relationship.schema.json` 内容变更，而不是任何业务测试失败，很容易被当成「rebase 弄脏了文件」然后 `git checkout` 掉，把这次收紧一起回退。

## 4. L3：SECURITY 指针写了什么、没写什么

新增一节「实现实证在哪（指针，不是本树结论）」。内容：

- 先声明本文件是**规范**，不记录任何已跑过的实测结论；上面那条 `DPAPI → KEK → DB DEK → 每单元 CK` 在本树是设计约束，不是已验收事实。
- 一张两行表，指出 `cursor/soul-goal1-7b1c`（PR #2，核于 2026-08-25，尖端 `6c91d39`）的同名文件多出「加密落地」「DPAPI 落地」两节，各记什么，以及**本树都不能引用为已完成**。
- 四条规矩：跨分支事实带分支名 + 提交号 + 核于日期（D57）；不要提前抄那两节；PR #2 合入时两侧合成一份且规范面保留；那两节不改变开头「不承诺」的三条。

**没写**：SQLCipher 版本号、`PRAGMA` 写法、`keys.dpapi` 的字节布局、`CryptProtectData` 标志位、具体测试文件名与断言清单。这些都是 Goal 1 那条线的实证，抄过来就等于宣称 `main` 上已有这些实现。表格里只留了「记的是什么」这一层的概括，不含可被当成本树结论的细节。

约束自查：全中文散文；无 `180` 字面量（全文 `rg 180` 无命中）；无 F04c、更没有第三道降档门（全文 `rg F04c` 无命中）；没有新增 `PRODUCT.md`；没有改产品方向。

## 5. 需要父代理跟进的三处（都不在我可写范围内）

1. **`docs/STATUS.md:54`** 现在是假话：`| schema 倒退 | 计划 | Round 1 已改用 Goal 1 接线版；`tie_strength` 仍为裸 object（D58） |`。建议换成：
   `| schema 倒退 | 计划 | 已关闭：Round 2 把 `tie_strength` 按 T4D 可复核面类型化（`dependentRequired` + `if/then`），`schemas.lock.json` 同批重算。Goal 1 rebase 时两个文件取 `main` 侧，`schema-freeze --check` 即绿 |`
2. **`docs/STATUS.md:37`** 那条「Round 2 · 其余 | 进行中」把 `tie_strength` 收紧与 SECURITY 指针都列为未做，两件现已落地，需改成完成态。
3. **`docs/DECISIONS.md:69`（D58）** 的前半段仍然成立（字段清单是计划义务、先采纳 Goal 1 正文），末句「进一步收紧 `tie_strength` 须与 Goal 1 `schemas.lock.json` 同批重算」也仍然成立且**已被本轮执行**。建议在依据列补一句本轮结果，或另起一条拍板记录这次收紧——我没动 `DECISIONS.md`，编号留给父代理分配以免与其它子代理撞号。

另有一处与我无关但顺手核到的漂移：`docs/STATUS.md:19` 记 `cursor/soul-goal1-7b1c` 尖端为 `3161e02`，我今天 fetch 到的是 `6c91d39`（forced update）。我的 SECURITY 指针按实际核到的 `6c91d39` 写。两处日期都是 2026-08-25，父代理需要挑一个口径统一。

## 6. 本目录内容

```
REPORT.md                      本文件
relationship.schema.json       与 docs/ 同字节
schemas.lock.json              与 docs/ 同字节
SECURITY.md                    与 docs/ 同字节
probe/verify_tie_strength.py   探针（A 锁一致性 + B 校验行为）
probe/regen_lock.py            复刻 xtask schema-freeze --write 的字节形态
probe/RESULTS.txt              基线 31/57 与落地后 57/57 的完整输出
```

复现：

```
python3 probe/verify_tie_strength.py                          # 默认打 docs/schemas/，应 57/57
python3 probe/verify_tie_strength.py --schemas-dir <任意目录>  # 也可打候选目录
python3 probe/regen_lock.py                                    # 重算 docs/schemas/schemas.lock.json
```
