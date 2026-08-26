# Round 3 opus-b — import / as_of / clock 口径对齐

模型：`claude-opus-5-thinking-high-fast`（按派单 slug，无降级）。
BINDING：`.agent_workspace/plan-polish/R2-SYNTHESIS.md`。核于 2026-08-25，分支 `cursor/polish-project-plan-5280` @ `c3d5960`。
配套：同目录 `CLOCKS.md`（时间量清册、分层图、不变式、边界表、探针清单）。

---

## 结论（先看这段）

**四份指定文件在 import / as_of / clock 上已经一致，本轮按 report-only 收工，未改任何 `docs/` 文件。** `PRODUCT_LOCK.md` 也确实没有复抄任何判档常量数值，本轮没有往里加数字。

同时查出五项，都不阻塞合入，但需要落到权威面：

| ID | 事 | 严重度 | 我的处置 |
|---|---|---|---|
| F1 | `COPY_ZH.md` P4 触发条件仍写「超过 180 天」，是 `DECISION.md` 明文作废的 `>` 写法，也与已落地的代码 `>=` 不一致 | P1（真矛盾，但方向安全） | **给出补丁，未擅自改**：COPY_ZH 是 `ALGO_FROZEN`，改它须同批在 `DECISIONS.md` 留痕，而 D 号只能由父代理分配 |
| F2 | 「全库一个 as_of」在存储层没有可观测断言：没有任何一行要求同一次 rebuild 落库的各边 `as_of_utc` 相等 | P2 | 建议补进 AC-30 的 Then，给出措辞 |
| F3 | 「降档时钟必须读任一场地」只有墓碑，没有负向门禁行 | P3 | 建议，不改矩阵 |
| F4 | 导入一条远期（未来）时间戳会污染全库 as_of，把整张图推向 weak；D37/D38 与 schema 都拦不住 | P2（未命名风险） | 只报，处置属导入层新拍板，不在本轮擅断 |
| F5 | `DORMANT_AFTER_DAYS` 是第二处 `180` 字面量定义（跨 crate 无法互引所致） | P3 | 确认它已被 D52 收编为合并义务，无需新决议 |

D36（会话分类）与 T4D 的 `direct_*`（计数）是**两层**，本报告第五节与 `CLOCKS.md` §4.1 把「不得互相顶替」写死了，并给出一条可选的 D36 增补措辞。

---

## 一、核对方法

只读交叉核验，不重推算法。四个面逐句比对，另取三处旁证钉住「文档说的等于代码做的」：

- 文档面：`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/DECISIONS.md`、`docs/algorithms/DECISION.md`；牵连面 `docs/algorithms/COPY_ZH.md`、`docs/schemas/relationship.schema.json`、`docs/schemas/_defs.schema.json`、`docs/PLAN_INDEX.md`。
- 代码旁证：`crates/soul-algo-tie/{constants,recency,types}.rs`、`crates/soul-algo-trait/src/a2.rs`。
- 实跑：`cargo test -p soul-algo-trait --test a2_render` → 22 passed（用来确认 A2 沉寂句的边界现在是闭区间）。

本报告与 `CLOCKS.md` 遵守 FORMAL 红线 11：阈值一律写常量名，出现字面数字的只有带文件行号的逐字取证行。

---

## 二、as_of 四面对照（一致）

| 面 | 原话要点 | 单一 as_of | 来源=数据 | 禁 per-peer | 禁墙钟 | 落库可复核 |
|---|---|---|---|---|---|---|
| `PRODUCT_LOCK.md:79` | 「时间口径来自数据，不来自墙钟。一次重建全库只用一个 `as_of`；不按每个人各自的最后时间戳算」 | ✓ | ✓ | ✓ | ✓ | —（产品面不承诺存储细节，正确） |
| `DECISIONS.md:56`（D45） | 「一次重建全库只有一个 `as_of`（缺省 = 全库 `max(occurred_at)`）。禁 per-peer，禁墙钟」 | ✓ | ✓ | ✓ | ✓ | — |
| `FORMAL_WORK_PROMPT.md:153`（AC-30） | 「全库只用**一个** as_of（调用方传入，缺省 = 全库 `max(occurred_at)`）；该值与沉寂天数随每条边落库、可复核；休眠边判 weak」+ per-peer 负向探针 | ✓ | ✓ | ✓（负向探针） | ✓（经 `:47` 冻结表行） | ✓ |
| `algorithms/DECISION.md:57`（§3 as_of 纪律） | 「一次 rebuild 全库**一个** as_of，由调用方传入（缺省 = 整个 store 的 `max(occurred_at)`）；禁止 per-peer……judgement 一律不读墙钟」 | ✓ | ✓ | ✓（附实测夹具） | ✓ | —（§6.4 由 `TieScore` 承接） |

四面同义，用词也没有分叉：三处写「全库」，`DECISION.md` 写「整个 store」，指同一个集合。粒度分工也正确——产品面不写存储字段，验收面写落库与可复核，算法面写规则与反例，拍板面写一句话结论。这是 P1 单源该有的样子，**不需要动**。

补两条本轮确认、之前没人明写的读法：

1. **「缺省」是调用方的缺省，不是 crate 的缺省。** `DECISION.md` §6.3「soul-algo 不提供缺省墙钟路径」与 crate 现状对得上：`as_of_max()` 存在，但注释写明「The rules never call this themselves」（`types.rs:271-280`），它是给调用方取数用的助手，不是隐式回退。四份文件里的「缺省 = 全库 `max(occurred_at)`」都应按这个意思读。
2. **空库没有 as_of。** `as_of_max` 对空库返回 `None`。四份文件都没写这一格，但也不构成缺口：空库没有边，`as_of_utc` 只在 `algorithm_id` 在场时必填（`relationship.schema.json:117-136`），落不到任何行上。记录备查，不建议加字。

## 三、clock 四面对照（一致）

| 命题 | PRODUCT_LOCK | DECISIONS | FORMAL | DECISION.md |
|---|---|---|---|---|
| 只有一个近因时钟，读任一场地 | `:86`「一对一停了很久、群聊仍然活跃 → 不降档（§4.3 已定价）」 | D42「群聊行只供展示与近因，不判档」 | `:42` 冻结表「群聊行保留展示与近因，不参与判档」 | §3 时钟公式 + `:72` 段；`REJECTED.md:111` 收 direct-only 变体 |
| 降档句与降档同阈值 | `:80` 三条之三 | D44（不加第三道门）间接支撑 | 不复述（正确，红线 11） | §3 常量表 `DORMANT_NOTE_DAYS`「不得分裂」 |
| 判档面不读墙钟 | `:88` crate「纯函数、不读墙钟、不落库」 | D45 / D51 | `:47` 冻结表 + 红线 12 | §3 末句 / §6.2 |
| 不为 F04c 加第三道门 | `:85` + 不可协商约束 13 | D44 | `:49` | §4.2 / §5 |

四面一致。`PRODUCT_LOCK.md:80` 用「『半年』『一年』各自对应 `DECISION.md` 常量表里的一个天数常量，本文件不复抄数值」把承诺和数值分开，正是本轮要求的写法，保持原样。

**唯一的分歧不在这四份文件里，在第五份**——见 F1。

## 四、import 面：三条耦合，一条缺口

导入层与 as_of/clock 的耦合已经分散在 D37、D38、D47、D33、AC-34 里，彼此不矛盾，但从来没有在一处写成「导入决定判档的输入」。`CLOCKS.md` §5 补了这张图。三条耦合：

1. as_of 的缺省值全部由导入层写入的 `occurred_at` 决定，所以导入的时间正确性是每一条边档位的上游。
2. 导入非事务（D37），半途失败也会推进全库 `max(occurred_at)`；跟着必须跑**全库** rebuild，不能只补算受影响的 peer，否则库里会同时存在两次 rebuild 的 `as_of_utc`。
3. 归因错误直接改写时钟读数：D47/AC-34 的 owner 群消息若刷新了 A、B 的 `last_contact`，降档时钟就被一次并不存在的往来推后。

缺口是 F4：现有防线只挡「不成立的时间」，不挡「成立但离谱的时间」。

## 五、D36 与 T4D `direct_*`：两层，禁止互相顶替

完整对照在 `CLOCKS.md` §4.1。要点四条：

1. **D36 是导入/归一化层给会话贴的分类**（按发言人数，单活跃发言人的群可判 Direct），**T4D 的 `direct_*` 是算法层对已贴好分类的行做的加法**。`types.rs:363-375` 里 `venue_direct` 是入参字段，算法从不自己判定它。
2. 分类判错，只能改 D36 的判据（D36 原话：「改判据要动冻结契约」）。**不许**在 T4D 里加一道门去补救——那是 FORMAL 红线 11 与 D42 禁止的第二套阈值，也是 `REJECTED.md:113` 已经击毙过的那类附加门。
3. 反向同样禁止：`direct_*` 不定义「什么叫一对一」。写「T4D 认为单发言人群是一对一」就是把两层揉在一起——那是 D36 认的，T4D 只是照单计数。
4. 现实后果要说破：因为 D36 这条判据，一个来自群导出的会话**可以**贡献 `direct_*` 并参与判档。这是 D36 的定价，不是 T4D 的漏洞。审「这条边为什么是强」，第一步查 D36 分类，第二步才查 T4D 计数。同一条纪律适用于 D33、D47：它们改变的是喂给 T4D 的行，不是 T4D 的规则。

D36 现行行文没有说错，只是没有挡住误读。**可选**增补（父代理决定要不要用，只动理由列，不动决定列）：

```text
| D36 | v1 按发言人数判 Direct/Group | 维持。单活跃发言人的群可被判 Direct | 改判据要动冻结契约。
Direct/Group 是**导入/归一化层**给会话贴的分类，T4D 的 `direct_*` 只是消费该分类后的计数：
两层不得互相顶替——分类不合意只改本行判据，禁止在 T4D 里加门补救（红线 11 / D42） |
```

---

## 六、发现清单

### F1（P1）COPY_ZH 的 P4 触发条件是已作废的 `>` 写法

**事实。** `DECISION.md` §3 常量表写「沉寂 ≥ `DEMOTE_ONE_BAND_DAYS` 天降一档，**闭区间**（`>=`；任何 `>180` 写法作废）」，且 `DORMANT_NOTE_DAYS` 与之「共用同一常量，不得分裂」。而 `COPY_ZH.md:70` 写的是「最后往来距 as_of **超过 180 天** → 追加『你们最近半年没有往来。』」——严格大于，正是被点名作废的那种写法。`COPY_ZH.md:73` 又同时把两者声明为同一个常量，所以这份文件自己前后也不一致。

**代码已经不是这样了。** `crates/soul-algo-trait/src/a2.rs:185` 是 `days >= DORMANT_AFTER_DAYS`，`tests/a2_render.rs:305-307` 的 `quiet_for_exactly_the_threshold` 钉住恰好等于阈值即判沉寂。本轮实跑 22 passed。降档侧 `recency.rs:56-63` 同样是闭区间。**三方一致取闭区间，只剩 COPY_ZH 一处文字。**

**为什么不阻塞。** 分歧只在恰好等于阈值的那一天，且方向安全：`>` 写法只会少说一句，不会在未降档时说出沉寂句。PRODUCT_LOCK 三条之三要的不变式（「用户读到那句话时，档位必然已经降下」）在两种写法下都成立。这与 Round X 的判定一致（`.agent_workspace/roundx/fable-b/LOCK_FIT.md` §9、`roundx/fable-a/CONSISTENCY.md` D2/D3 已提出同一条，处置意见也是「以 DECISION 为规范统一为 `>=`，走 DECISIONS 留痕」）。

**为什么我没有直接改。** 三个理由，任一成立都足够：其一，`COPY_ZH.md` 是 `ALGO_FROZEN`，文件第 5 行自定的改法是「改模板先改本文件并留痕」，留痕落在 `DECISIONS.md`；其二，新 D 号只能由父代理分配，Round 3 有六路并行，我自取号必撞；其三，Round X 的处置意见现在只活在 `.agent_workspace/roundx/`，按 D41 那不是权威面——真正要做的动作是**把它搬进 `docs/`**，而搬运需要文稿加决议同批落地，不是一行字的事。

**补丁（父代理可原样应用）。** 只动未加书名号的触发条件与规范注解，**用户可见句子一字不改**，因此不触发 D33/D35 关于模板措辞的约束：

```diff
-- **P4 近因**：「最近一次是 {最后日期}。」；最后往来距 as_of 超过 180 天 → 追加「你们最近半年没有往来。」（只引最近一条证据。）
+- **P4 近因**：「最近一次是 {最后日期}。」；{沉寂天数} 不少于 `DORMANT_NOTE_DAYS`（闭区间 `>=`，取值单点见 DECISION.md 第 3 节常量表）→ 追加「你们最近半年没有往来。」（只引最近一条证据。）
```

```diff
-约束重申：P5 的档位词来自 T4D 产出的 band，A2 不得自带第二套阈值；P4 的 180 天与降档常量是**同一个常量**（`DORMANT_NOTE_DAYS = DEMOTE_ONE_BAND_DAYS = 180`，单点定义见 DECISION.md 常量表）。
+约束重申：P5 的档位词来自 T4D 产出的 band，A2 不得自带第二套阈值；P4 的沉寂阈值与降档常量是**同一个常量**（`DORMANT_NOTE_DAYS = DEMOTE_ONE_BAND_DAYS`，取值单点定义见 DECISION.md 常量表），且同为闭区间 `>=`。
```

同批要追的 `DECISIONS.md` 行（号由父代理分配）：

```text
| D6x | A2「最近半年没有往来」句的触发边界 | 闭区间 `>=`，与降档共用 `DORMANT_NOTE_DAYS = DEMOTE_ONE_BAND_DAYS`。
COPY_ZH P4 原「超过 180 天」是 DECISION.md 宣布作废的 `>` 写法，改写为「不少于常量」；用户可见句子一字未动，
故不属 D33/D35 意义上的模板措辞变更 | `DECISION.md` §3「任何 `>180` 写法作废」；
`crates/soul-algo-trait/src/a2.rs:185` 已是 `>=`，`tests/a2_render.rs` 的 `quiet_for_exactly_the_threshold` 钉住边界 |
```

顺带的收益：改完之后 `COPY_ZH.md` 的规范注解层不再出现阈值字面量，与 `PLAN_INDEX.md`「其余文档一律只写常量名」对齐。用户可见模板里的数字（如 T4D 强档句里的次数与自然日门、封弱句里的一年）**保持不动**——那是用户必须能自己数的数，属于 PRODUCT_LOCK 三条之一要求的可数性，不是复述阈值。

### F2（P2）「全库一个 as_of」在存储层没有可观测断言

AC-30 要求 as_of「随每条边落库、可复核」，并用 per-peer 负向探针挡住最危险的那种错法。但没有任何一行直接断言**同一次 rebuild 落库的各边 `as_of_utc` 相等**。差别是实打实的：per-peer 探针要靠休眠夹具的档位翻转来间接发现问题，而部分 rebuild、并发 rebuild、导入后只补算受影响 peer 这三种走法，都可能让库里同时存在两个 `as_of_utc`，却不必然翻掉那条休眠边的档位。

建议（**未改矩阵**，措辞供父代理取用）——在 AC-30 的 Then 末尾追加一句：

```text
；同一次 rebuild 落库的每条边 `as_of_utc` 必须相等（分批或按 peer 增量重算导致取值分裂即为缺陷）
```

这是把 D45 已有的结论变得可观测，不是新增门禁，符合 `PLAN_INDEX.md` 第五节「新增行写清 Given/When/Then」的约束（本条不新增行，只补 Then）。若父代理判定这属于扩门，丢弃即可——AC-30 的现有负向探针仍能覆盖最主要的错法。

### F3（P3）「降档时钟读任一场地」缺负向门禁行

direct-only 时钟变体已经作废并入墓碑（`REJECTED.md:111`），`DECISION.md` §3 也解释了为什么刻意读任一场地。但矩阵里没有对应的负向行：把时钟改成 direct-only，AC-28…AC-34 里没有哪一行必然变红。crate 侧有夹具（`dormant_direct_group_ping_yesterday` 与 `dormant_direct_no_ping` 恰好只差一条群消息，`testing/mod.rs:511`），产品边界上没有。

不建议本轮补行：D30 只减不增，且这条在算法 crate 已被钉住。记录在案，等 Goal 1 吸收线（PR #7）落地后由那条线决定是否上升为产品边界门禁。

### F4（P2）远期时间戳会污染全库 as_of

**机理。** 缺省 as_of = 全库 `max(occurred_at)`。导入一条时间戳落在远期（例如四位年的 2099）的消息，就把全库 as_of 拉到那个未来点，于是**其余每一条边**的 `silent_days` 都被同量放大，足以越过 `FORCE_WEAK_DAYS`，把整张图推成 weak。

**现有防线都拦不住。** D37 只要求「对应 RFC3339 年必须在 1970–9999（四位年）」，2099 合法；D38 只校验民事日成立，2099-06-01 成立；`_defs.schema.json:9` 的 `timestamp` 只有 `format: date-time`，无上界；算法层 `age_days` 只对**负**差值钳零（`types.rs:262-269`），对被拉大的正差值无能为力，而且算法层本来也不该管——它按定义不知道「现在」。

**与 AC-30 的关系。** AC-30 挡的是 as_of 取得**太局部、太近**（per-peer）。F4 是反方向：as_of 取得**太远**。两个失效方向共用同一个字段，目前只有一边有门禁。

**为什么我不在本轮拍板。** 干净的处置只有一条：在**导入层**把远期时间戳判为缺陷（与 D37/D38 同族的加法，不动冻结算法）。另一条看似省事的做法——rebuild 时把 as_of 夹在某个上界内——会把墙钟重新引进判档路径，直接违反 D45 与 `DECISION.md` §3 末句，**必须否掉**。前者是新拍板，属父代理职权，且需要定「多远算远」这个新常量（它是导入层常量，不进 T4D 常量表）。本轮只命名风险，附建议探针：

```text
Given 一份导入夹具里混入一条远期时间戳的消息
When  导入并跑一次全库 rebuild
Then  该消息判缺陷不入库；全库 as_of 与只导入干净数据时相同；其余边的 band 与 silent_days 不变
```

### F5（P3）`DORMANT_AFTER_DAYS` 是第二处 `180` 定义，但已被 D52 收编

`crates/soul-algo-trait/src/a2.rs:91` 独立定义了第二个 `180`，原因是两个 crate 尚未合并、无法互相 import。这看着像违反 `DECISION.md` §3「全仓库禁止第二处出现数字字面量」，实则已经被 D52 明文收编为合并后义务（「两侧天数常量用工作区测试钉相等；产品 crate 禁止再写第三处 `DEMOTE_ONE_BAND_DAYS` 字面量」），两侧也各有测试钉住取值。**不需要新决议，也不需要改文档。**

一处名字漂移供合并时一并处理，本轮不动：规范名 `DEMOTE_ONE_BAND_DAYS` / `FORCE_WEAK_DAYS` / `DORMANT_NOTE_DAYS` 在代码里分别叫 `DEMOTE_AFTER_SILENT_DAYS` / `WEAK_AFTER_SILENT_DAYS` / `DORMANT_AFTER_DAYS`（`constants.rs:56-58` 已给前两个建了同名别名，第三个跨 crate 建不了）。值与语义全等，行为无差。

---

## 七、本轮没有做的事

- **没有改任何 `docs/` 文件。** 四份指定文件本就一致，F1 虽是真矛盾但落在 `ALGO_FROZEN` 的 COPY_ZH 上，须与 `DECISIONS.md` 新行同批落地，而 D 号只能由父代理分配。补丁与决议措辞都写在 F1 里，可原样应用。
- **没有往 `PRODUCT_LOCK.md` 里加任何数字**，也没有把 `DECISION.md` 的常量表往任何别的文件搬。
- 没有改验收矩阵（F2/F3 只给措辞）、没有改产品方向、没有加 F04c 第三道门、没有碰 Goal 1 与 PR #7 的代码。
- 没有 `git commit`：核对时工作区里已有他人未提交的 `docs/PLAN_INDEX.md`、`docs/STATUS.md` 改动（Round 3 并行槽位在写），本目录之外的树状态不归我处置。

## 八、给父代理的收口顺序

1. 先定 F1：分配 D 号 → 应用 COPY_ZH 两处补丁 + 追 `DECISIONS.md` 一行。两者必须同批，否则会留下一处无留痕的冻结文稿改动。
2. F2 取舍：接受就把那句话并进 AC-30 的 Then；不接受就明确记「以 per-peer 负向探针为准」，别让它悬着。
3. F4 单独立项：这是导入层新拍板，不要顺手塞进本 PR，也**不要**用「as_of 夹上界」的写法糊过去——那会把墙钟带回判档路径。
4. F3、F5 归档到 Goal 1 吸收线，本 PR 不动。
5. D36 增补措辞是可选的；不采纳也不影响合入，`CLOCKS.md` §4.1 已经把分层写死，后来者查得到。
