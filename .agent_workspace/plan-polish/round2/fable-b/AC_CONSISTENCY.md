# AC-28–AC-33 × D32–D58 × BLOCKERS G 项交叉核验

槽位：Round 2 fable-b（slug `claude-fable-5-thinking-xhigh`）。只读核验，不改 `docs/`。
核验对象：**工作树现状**（`b380f67` + 父代理 Round 2 未提交改动，含 FORMAL 历史段注记、红线 11 自查段、AC-29 措辞修订、`relationship.tie_strength` 收紧）。凡「行号」指工作树版 `docs/FORMAL_WORK_PROMPT.md`。
BLOCKERS 取 `origin/cursor/blockers-analysis-a073:docs/BLOCKERS.md`（`BLOCKERS_FROZEN`）。

## 〇、先把编号讲清楚：Round 1 过程稿与落地矩阵的映射

父代理落地时对 Round 1 fable-b 提案（`round1/fable-b/AC_MATRIX_GAP.md` §二、`SOTA_ACCEPT.md`）做了**内容换位**。过程稿按 D41 不是权威，但后续读者会翻它们，必须留这张映射表：

| 落地编号（权威，FORMAL） | 内容 | Round 1 fable-b 过程稿编号 |
|---|---|---|
| AC-28 决胜夹具 `group_heavy_plus_one_direct_each_way` → 非 strong | 群扇出不产档 | 旧 AC-29（部分） |
| AC-29 锚 `lilei_12` → strong + 自愈 `three_directs` → moderate | T4D 锚保全 | 旧 AC-28（部分） |
| AC-30 as_of 单钟 | 同 | 旧 AC-30 ✔ 同号 |
| AC-31 intake 不绕**轴**锁（A0 / G2） | 特质轴 | 旧 AC-32 |
| AC-32 人脉**边**用户纠正锁定（G3） | 人脉边 | 旧 AC-31 |
| AC-33 A2 纯渲染器 | 同 | 旧 AC-33 ✔ 同号 |

`docs/` 内部对 AC-28–33 的引用只有 FORMAL 自身与 `STATUS.md` 下一步第 1 条（「FORMAL AC-32/33」）；后者按落地编号读（边锁话术门 + A2 渲染器，正是两条与 COPY_ZH 耦合、Round 2/3 还要打磨的行），**语义自洽，无需改**。`docs/` 其余文件（PRODUCT_LOCK、DECISIONS、schemas、GOAL2、PLAN_VERIFY、templates）零处按编号引用这六行——编号换位没有在权威面制造悬空引用。

## 一、逐行核验（AC ↔ D-id ↔ G 项 ↔ 算法权威）

### AC-28（决胜夹具 → 非 strong；分列计数落边）

| 对拍面 | 结论 |
|---|---|
| D42（T4D 唯一权威，群聊只展示不判档） | ✔ 一致 |
| D33（渲染持久化分列，可含 0，不从证据重算） | ✔ 「边上落四个分列计数」即 D33 的持久化前提 |
| BLOCKERS G1（「群洪 + 每向 1 条一对一 → Weak 且分列精确」；夹具测产品边界） | ✔ 方向一致；**「分列精确」半句落地时被删**，见 ACCEPTANCE_GAP G-2 |
| `ALGO_FROZEN` §1（T4 Strong✗ / T4D Weak 实证）、§3 | ✔ 「同口径下 T4 判 strong」措辞与 §1 实测表逐字对得上 |
| 夹具字节（`crates/soul-algo-tie/src/testing/mod.rs:329`） | ✔ 群 30 次 / 10 天 + 每方向各 1 次。Round 1 过程稿的 100/50 是 DECISION §1 叙述旧值，落地版已按 R1-SYNTHESIS 第 8 条校准 |
| 「解释文案同屏报出一对一与群里两个数」 | ✔ 可满足：冻结 crate `T4D::explain_zh` 恒以 `zh_counts_split` 开头（两个数都在），弱档另附 `zh_group_note`「你们在群里还有 N 次往来…不算进这一档」（`t4d.rs:147-155`）。COPY_ZH §1「弱（一对一往来太少）」句本身只含 {次数}/{一对一次数}，「同屏」由分列字段 + 解释合成满足，不逼改冻结模板 |

**命名注记（P3）**：AC-28 写 `direct_out/direct_in/group_out/group_in`（`Tally` 字段名），本轮收紧的 `relationship.schema.json` 与 `TieScore` 用 `direct_out_count/...`。同物两拼法，探针写字符串匹配时要认两套。建议 Round 3 在 FORMAL 该行加「_count 后缀以 schema 为准」一句或统一拼法；非矛盾。

### AC-29（锚保全 + 自愈路径）

| 对拍面 | 结论 |
|---|---|
| D42；BLOCKERS G1 锚夹具 `lilei_12` | ✔ 一致 |
| `ALGO_FROZEN` §1（锚测试保全 ✓）、§4.1（自愈便宜，实测 `group_heavy_plus_three_directs` → Moderate） | ✔ 一致 |
| 夹具字节（`testing/mod.rs:167`、`:346`） | ✔ `lilei_12` = 12 次 / 6 天 / 3 天前收尾；`three_directs` = **共 3 条（2 发 1 收）**。Round 1 过程稿写「每方向各 3 条」是错的（那是 t4d-verify 工作区的近名夹具），落地版正确 |
| 红线 11 自查段（本轮新增，FORMAL:164） | ✔ 本轮把「（一对一 3 次）」改写为「夹具名中的 `three_directs` 即其一对一条数」，消除了与 `MODERATE_MIN_INTERACTIONS`（值恰为 3）的读法碰撞——数字字面量不再出现在「回到 moderate」旁边。这是红线 11「改夹具规模也不要顺手写成因为门槛是 N」的正确执行 |

### AC-30（as_of 单钟）

| 对拍面 | 结论 |
|---|---|
| D45（全库一个 as_of，缺省 = `max(occurred_at)`，禁 per-peer、禁墙钟） | ✔ 逐句一致 |
| `ALGO_FROZEN` §3 as_of 纪律（per-peer 会让 2019 休眠边假 Strong，实测沉寂 2632 天） | ✔ 变异条款「改成 per-peer 必须红」与 crate 测试 `peer_local_as_of_wrongly_revives_the_dormant_tie` 同一语义 |
| BLOCKERS G1 接线段（「在丢掉联系人解不开的行**之前**算」） | ⚠ 该精度**没进 AC**，Round 1 过程稿有、落地被删。见 ACCEPTANCE_GAP G-4 |
| 夹具（`dormant_2019`，`testing/mod.rs:198`） | ✔ 存在，构造与 Given 相符 |
| 本轮 schema（`as_of_utc`、`silent_days` 声明算法后必填） | ✔ 「该值与沉寂天数随每条边落库、可复核」已被 schema 字节承接 |

### AC-31（intake 不绕轴锁 ↔ G2）

| 对拍面 | 结论 |
|---|---|
| D46（锁轴不 `place_axis`；答案落证据；`ignored` 理由 `axis_locked_by_user`） | ✔ 语义一致。AC 只写「已跳过 + 原因」，没点名理由 token——token 由 D46 持有，AC 经「与 `apply_intake` replay 全等」传递钉住，可接受（见 ACCEPTANCE_GAP G-5） |
| D39（`IntakeReceipt.ignored` 加法字段；`answered` 不计被锁轴拒答） | ✔ 「答案行照常落库」「不静默丢弃」对应；`answered` 计数细则未复述，同上由 replay 全等传递 |
| D43（A0；A1 空转是预期） | ✔ 不冲突。A1 空转钉死断言未进本行，见 ACCEPTANCE_GAP G-6 |
| BLOCKERS G2（含「保持 LastWriteWins；不开 A1 / NoDowngrade」） | ✔ 禁改半句未进 AC，但 replay 全等使任何冲突策略改动必红；无矛盾 |
| `a0_lock.rs`（FORMAL:160 声称已存在） | ✔ 文件在 `crates/soul-algo-trait/tests/a0_lock.rs` |

### AC-32（人脉边用户纠正 ↔ G3）

| 对拍面 | 结论 |
|---|---|
| D48（生效档 = 锁定时用户档；机器档另存；A2/图只消费生效档） | ✔ 逐句一致 |
| D32（未锁边也写 `machine_band`；`locked ⟺ user_band.is_some()`） | ✔ 「机器档另存**且继续更新**」与 D32 的常 `Some` 语义相容 |
| D35（「由你本人指定」变体禁令，测试反向断言字面量除外） | ✔ AC 明引 D35/D48；且 D35 的测试豁免恰好让本行可写成 CI 断言而不自违禁令 |
| BLOCKERS G3（GC-9 话术不可照抄；工序：先机器档、再生效档、再 A2 读生效档） | ✔ 一致；工序属实现侧，不必进 AC |
| COPY_ZH §4 P5 原句存在性 | ✔ 「按上面的计数，这段关系归在『…』一档」在 COPY_ZH:71——对锁定档确为撒谎，禁渲染成立 |

### AC-33（A2 纯渲染器 ↔ D33/D43）

| 对拍面 | 结论 |
|---|---|
| D43（A2 纯渲染器，源码级禁止第二套阈值） | ✔ 一致 |
| D33（渲染持久化 `group_out+group_in`，可含 0；不从证据重算） | ✔ 关键区分成立：**分列 = Some(0,0) 照渲染（D33），分列缺席整句不出（AC-33）**。两者不打架——前者是 G1+ 之后的结构性 0，后者是未声明算法的遗留边 |
| BLOCKERS G1 A2 段（「适配器必须把分列填成 Some，否则 A2 静默丢掉冻结句」） | ✔ 与本轮 schema 咬合：声明 `algorithm_id` 后分列必填 → 产品新边不可能「缺席」，AC-33 的缺席臂只测渲染器契约与遗留边。无矛盾，建议 Round 3 在行末加半句防误读（见 ACCEPTANCE_GAP G-7） |
| D52（产品 crate 禁第三个 `180`） | ✔ 「无第二套阈值字面量」即其 AC 化 |
| COPY_ZH §4 P1b、§5.7 | ✔ P1b 句原文与「任一数缺席则整句不出现」逐字对得上 |

## 二、受命附核的三行旧 AC

| 行 | 对拍 | 结论 |
|---|---|---|
| AC-08 | D42 + G1「图侧无第二套阈值」 | ✔ 已按 Round 1 增补 T4D 半句。Round 1 过程稿建议的「字段在场」清单没进本行——已由 AC-28（分列 + `direct_active_day_count`）、AC-30（`as_of_utc`、`silent_days`）与本轮 schema `if(algorithm_id)→required` 分摊覆盖，不缺 |
| AC-12 | BLOCKERS S2（crate 测试自带 `KnownIdentifiers`，嵌中文名夹具在现测试红不了；升 P0 条件 = session 缝断言） | ✔ AC 已把 S2 的升级条件直接写成门禁（「必须在 session 缝变红，crate 内单独绿不算过」）。矩阵比 BLOCKERS 的 P1 定级更严，方向正确，无矛盾 |
| AC-26 | D56 + BLOCKERS S5 | ✔ 「package job 绿 + 作者签名安装包；CI 不下完整 `tauri build`」与 S5 建议措辞逐字一致 |

## 三、矛盾清单

**硬矛盾（会导致两个权威给出相反判定）：0 个。**

准矛盾 / 漂移（都已有单点权威裁决，列出防误读）：

1. **`ALGO_FROZEN` §6.1 用旧名**（`crates/soul-algo`、`graph_build.rs`）↔ 真名 `soul-algo-tie`/`soul-algo-trait`、`crates/soul-graph/src/build.rs`。已被 D53 + BLOCKERS §0 裁决「实现跟真名」；FORMAL 第 4 条指向 §6 时读者会撞见旧名。**不动冻结文稿**，Round 3 可在 FORMAL 第 4 条括注一句 D53。
2. **`ALGO_FROZEN` §4.3 夹具名与参数**：写 `direct_quiet_200_group_yesterday`（200 天），crate 真名是 `dormant_direct_group_ping_yesterday`（300 天，`testing/mod.rs:384`）；200 天版只存在于 `.agent_workspace/round3/fable-a/t4d-verify/`。判决方向一致（roundx fable-a CONSISTENCY D5 已记录）。**唯一会咬人的场景**：将来给「任一场地时钟」补 AC 或探针时照抄 §4.3 的名字会指向不存在的 crate 夹具——届时必须写 crate 真名。
3. **字段拼法两套**（AC-28 的 `direct_out` ↔ schema/`TieScore` 的 `direct_out_count`），见上文 P3。
4. Round 1 过程稿（SOTA_ACCEPT / AC_MATRIX_GAP / WP_DAG）的编号与夹具参数**均已过时**（编号换位、100/50、「每方向各 3 条」）。按 D41 过程稿无权威性，本文件第〇节的映射表即解毒剂。

## 四、顺手完成的 R1 指定探针

R1-SYNTHESIS「性能瓶颈」节要求 Round 2 确认 schema lock 哈希与文件一致：**已跑，工作树 11 个 schema（含 `_defs`）sha256 与 `schemas.lock.json` 全部一致（含本轮改过的 `relationship.schema.json`，锁已同批重算），JSON 全部可解析。** 若父代理落提交前再改任一 schema 字节，须重跑。
