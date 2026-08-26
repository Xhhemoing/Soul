MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 3 / opus-a：main 上 `soul-algo-*` 不完善项终表

范围：`crates/soul-algo-tie` v0.3.0 与 `crates/soul-algo-trait` v0.3.0，**只收「在 main 上就能修完」的项**。需要先合并两个 crate 才能根治的（`DORMANT_NOTE_DAYS` 单点定义、统一 `crates/soul-algo`）不在本表，只在 §5 说明它们与本表的关系。

树的等同性先钉住：`git diff main HEAD -- crates docs` 为空，本工作树的 `crates/` 与 `docs/` 与 `main` @ `a0ec14b` 逐字节相同，所以在本树验证等于在 main 上验证。基线 `cargo test --workspace` = **249 项全绿**（实跑）。

**纪律声明**：未执行任何 git 写操作（无 `checkout` / `commit` / `push` / `add`），未改动 `crates/` 或 `docs/` 下任何字节。`git status --porcelain` 的唯一条目是本轮 `.agent_workspace/orch-20260826/round3/` 未跟踪目录。四组验证实验全部在 `/tmp/r3a`（工作树之外的 tar 拷贝）中进行。

---

## 0. 先修正提问里的一个前提：「回退链」这一格是空的

任务要求按「可立刻修 / 须走 DECISION 回退链」二分。逐条核完之后，**第二格没有任何一项落进去**，这不是遗漏，是本轮的一个结论，理由必须写清楚：

`DECISION.md` §5 的回退链是**唯一合法的改判通道**，它的两个触发器都在判档层——§4.1（仅群聊者落 Weak 的代价）与 §4.2（F04c 复燃一响）。`PRODUCT_LOCK.md` 不可协商约束 13 与 D44 用的也是这个口径：「改判只走 `DECISION.md` 回退链」。

而本表所有条目**没有一条改变任何 band**。R1 与 R2 已经把判档层逐格核过（三道门全读一对一计数、180/360 闭区间、as_of 全程由调用方传入、`GROUP_ONLY_CEILING` 仅 T4 引用），本轮没有推翻其中任何一条。所以回退链既不被触发，也不是这些项的修复通道——**把文案缺句拿去走回退链是走错门**。

真正存在的第二类不是「回退链」，而是**留痕通道**，它由三处规定，程序与回退链完全不同：

- `DECISION.md` §6.4：「模板改动**先改 COPY_ZH.md 再改代码**」；
- `PRODUCT_LOCK.md` 第 4/67 行：产品面承诺与话术权威的归属；
- `DECISIONS.md` D41 起的追号义务（D40 还专门承诺过「COPY_ZH 与 crate 文案漂移另记」——R2 已核实这个登记通道至今为空）。

所以下面按三格分，第三格如实标空：

| 格 | 判据 | 条数 |
|---|---|---|
| **甲　可立刻修** | 不动冻结字节、不需产品裁决、不改（或只按冻结文档校正）用户可见文本 | 7 |
| **乙　须先留痕再修**（DECISION §6.4 程序） | 要么改用户可见文本、要么改冻结文档，必须先定「代码向文档靠」还是「文档追认代码」 | 7 |
| **丙　须走 DECISION §5 回退链** | 改变任何 band | **0** |

---

## 1. 终表

「新」列标 ★ 的是本轮首次得出或推翻前轮结论的条目。

### 甲　可立刻修（7 条）

| # | 项 | 位置 | 依据 | 新 |
|---|---|---|---|---|
| 甲1 | 墙钟源码扫描缺口——**且 R1/R2 给的那个「一行改动」修法是错的，今天照抄会直接把测试打红** | `soul-algo-tie/tests/ablation.rs:555` | DECISION §3「judgement 一律不读墙钟」、§6.2、§6.3、D45、D51 | ★ |
| 甲2 | D52 已拍板的减灾义务未执行：两侧天数常量没有等值测试 | 跨 `soul-algo-tie` / `soul-algo-trait` | `DECISIONS.md` D52 明文「两侧天数常量用工作区测试钉相等」 | |
| 甲3 | `label_zh()` 的「中」——**5/15 个 A2 夹具上真的渲染出来了** | `soul-algo-trait/src/types.rs:87` | COPY_ZH §0.1 / §5.5 + `PRODUCT_LOCK.md:78`（产品承诺，非话术偏好） | ★（量化） |
| 甲4 | crate 内部重复常量：`a2.rs` 私有 `SECONDS_PER_DAY`，而同 crate `types.rs:186` 已 `pub` 导出同名同值 | `soul-algo-trait/src/a2.rs:94` | 纯清理；`a2.rs:19` 自称「The one number A2 owns is `DORMANT_AFTER_DAYS`」，实际拥有两个 | |
| 甲5 | REJECTED.md ↔ `tombstones.rs` 无清单绑定测试 | `soul-algo-tie/src/tombstones.rs` | 墓碑覆盖事实上完整，但无机器校验；范式照搬 `ablation.rs:203` 的 `every_fixture_appears_in_every_table` | |
| 甲6 | `tests/roundx_opus_a.rs` 模块头（1–48 行）已过期整整一轮 | `soul-algo-tie/tests/roundx_opus_a.rs:1-48` | 头部指引「`cargo test -- --ignored` 是未决缺陷清单」，实际 `#[ignore]` / `bug_*` / `todays_*` 在全仓库**零命中**（唯一命中是这段注释自己） | |
| 甲7 | `last_direct_contact_unix = 0` 哨兵重载，类型不强制下游做空值检查 | `soul-algo-tie/src/types.rs:171` | `Tally` 内部靠私有 `saw_direct` 区分，但 `TieScore`（§6.4 的对外渲染契约）把它丢了；改 `Option<i64>` 让编译器接管。本格中最侵入的一条 | |

### 乙　须先留痕再修（7 条，全部走 DECISION §6.4：先改 COPY_ZH 再改代码）

| # | 项 | 位置 | 待裁的是什么 | 新 |
|---|---|---|---|---|
| 乙1 | COPY_ZH 三条全局强制项在 `crates/` 全树 **0 命中**：§0.5「到 {截止日期}」、§0.6 收尾句、§0.7 透明句 | T4D 九条 + T4 九条 + A2 六类句 | 代码补齐这三句，还是 COPY_ZH 撤销「必带 / 首句」的措辞 | |
| 乙2 | tie crate 档位词是「强联系 / 中等联系 / 弱联系」，冻结词是「强 / 中等 / 弱」 | `soul-algo-tie/src/types.rs:89-95` | 三字词向冻结词靠，还是 COPY_ZH §0.1 追认三字词 | |
| 乙3 | **`explain_zh` 的兜底句在 21 个夹具里有 11 个真的会渲染出来，而且一条测试都没有** | `soul-algo-tie/src/t4d.rs:188-194` | 这条 COPY_ZH 未收录的句子是留、是改、还是换措辞 | ★★ |
| 乙4 | `zh_direct_clock_note` 的「一对一最近一次是 {日期}。」是 COPY_ZH 未收录的增补，且引入 REJECTED §附注点名批评过的「第二个日期概念」 | `soul-algo-tie/src/t4d.rs:157-165` | 补进 COPY_ZH 并说明与 REJECTED §附注的关系（倾向留句补文档，非删句） | |
| 乙5 | A2 沉寂句「……不是**现在**的联系频率。」逐字违反 COPY_ZH §5 断言 4，且已被测试钉死 | `soul-algo-trait/src/a2.rs:416`，测试 `tests/a2_render.rs:298` | 改句就要同时改那条断言，属于改用户可见文本 | ★ |
| 乙6 | A2 六类句逐句漂移，其中 P5 收尾是「这是对记录的归档，不是对这个人的评价。」，冻结文本是「这是工作假设，不是对这个人的判断。」 | `soul-algo-trait/src/a2.rs:427/430` | D40 承诺的「另记」通道至今为空（R2 §2.5 已核），先补登记再裁归属 | |
| 乙7 | `DECISION.md` 两处引用错误：§4.3 的夹具名与天数（写 `direct_quiet_200_group_yesterday` / 200 天，实为 `dormant_direct_group_ping_yesterday` / 300 天）、§6.4 指「COPY_ZH 第 6 节断言」而该文件只有第 5 节 | `docs/algorithms/DECISION.md` | 改的是 `ALGO_FROZEN` 文件本身，纯勘误也须在 DECISIONS 追一条勘误号 | |

### 丙　须走 DECISION §5 回退链：**0 条**

理由见 §0。若父代理希望这一格非空，唯一的合法填法是先裁定 §4.1 或 §4.2 的已知代价不可接受——那属于产品裁决，与本表列的文案/测试/扫描三类缺口没有交集。

---

## 2. 本轮四组验证实验（全部在 `/tmp/r3a`，工作树零改动）

前两轮把甲1、甲2 都标成「零风险、一行改动、顺手就做」。我把这两条**实际做了一遍**，结论是：一条的推荐修法是错的，另一条对但需要挑落点。

### 2.1 甲1：R1/R2 推荐的墙钟修法今天会直接失败 ★

R1 §建议 1 与 R2 §4.3 第 1 条都写：在 `ablation.rs:555` 的 `banned` 数组里加 `"SystemTime"` / `"Instant"` / `"std::time"`，「一行改动，零风险」。

照做，结果：

```
thread 'the_product_path_contains_no_floating_point' panicked at ablation.rs:556:
lib.rs mentions SystemTime: the product path must be integer-only
→ 20 个测试里 1 个红
```

原因是 `soul-algo-tie/src/lib.rs:55` 的模块文档里有这么一句：「nothing here calls `SystemTime`, so the same evidence scores the same in…」。**这个扫描是裸 `source.contains()`，它分不清「代码调用了 SystemTime」和「注释声明自己不调用 SystemTime」**，而 `lib.rs` 正在被扫描的 9 个文件之列。所以前两轮那句「零风险」的判断不成立——照抄会当场把 main 打红，而红的原因与墙钟无关。

可用的修法是先剥掉注释行再扫。实测这一版今天绿：

```rust
let code: String = source
    .lines()
    .filter(|l| !l.trim_start().starts_with("//"))
    .collect::<Vec<_>>()
    .join("\n");
for banned in ["f64", "f32", "SystemTime", "Instant", "std::time", "chrono"] {
    assert!(!code.contains(banned), "...");
}
```

```
cargo test -p soul-algo-tie --test ablation → 20 passed; 0 failed
```

并且它抓得住 R2 §3.4 证明过「135/135 全绿 + clippy 干净地混进去」的那个形态——`DECISION.md` §6.3 明文禁止的缺省墙钟入口。把 R2 那个 `score_now` 探针原样贴回 `lib.rs`：

```
lib.rs mentions SystemTime: the product path must be integer-only and clock-free
→ FAILED
```

**这就补上了 R2 指出的那个唯一覆盖不到的形态**：任何行为测试都抓不到「新增一条读钟的路径」，只有源码级扫描能抓，而现在这个扫描是可用的了。

### 2.2 甲2：D52 那条测试可行，落点建议放在 trait 侧

D52 要的是「两侧天数常量用工作区测试钉相等」。今天两个 crate 的 `[dependencies]` 与 `[dev-dependencies]` 全空，物理上谁也看不见谁，所以这条测试**不可能存在**。

R2 §4.3 建议给 `soul-algo-tie` 加 `[dev-dependencies] soul-algo-trait`。我建议**反过来**，理由是数据流向：A2（trait）消费 T4D（tie）产出的 band，渲染器依赖判档方，`trait → tie` 才是与 `DECISION.md` §6.2「依赖箭头」同向的那一条；反向加会在两个 crate 之间造出一条与真实消费关系相反的边。实测 `trait → tie` 的 dev-dep：

```toml
[dev-dependencies]
soul-algo-tie = { path = "../soul-algo-tie" }
```

```rust
#[test]
fn the_dormancy_sentence_and_the_demotion_share_one_threshold() {
    assert_eq!(DORMANT_AFTER_DAYS, DEMOTE_ONE_BAND_DAYS,
        "PRODUCT_LOCK 红线三之三：沉寂句与降档必须同一阈值");
    assert!(DEMOTE_ONE_BAND_DAYS < FORCE_WEAK_DAYS);
}
```

今天绿（`cargo test --workspace` → 251 passed，即 249 + 本条 + 探针）。再把 R2 §1.4 那个「249/249 全绿地把 A2 阈值单侧改成 150」的漂移重放一次：

```
assertion `left == right` failed: PRODUCT_LOCK 红线三之三：沉寂句与降档必须同一阈值
  left: 150   right: 180
```

**R2 证明过不可检测的那次单侧漂移，现在会红。** dev-only 依赖不进运行时、不违反 §6.2 的「零重依赖 / 禁 SQLCipher、存储、Tauri、HTTP」。

一处给实施者的坑：D52 后半句是「产品 crate 禁止再写第三处 `DEMOTE_ONE_BAND_DAYS` 字面量」。全树扫 `180` 时会撞到 `soul-algo-tie/src/tombstones.rs:111` 的 `d if d >= 180 => 1`。**它不违反 D52**——那是 T3R 的四分之一权重分桶边界，语义上不是降档阈值，且整个 `tombstones` 是 `#[cfg(test)]`，不在产品路径上。别把它当第三处字面量误删。

### 2.3 乙3：兜底句在 11/21 个夹具上真的会出现，且零测试覆盖 ★★

R1 §B5 把 `t4d.rs:188-194` 那条兜底句定性为「性质与 A4 相同，但**触发条件更罕见**」。我把 21 个夹具 × 5 种分数形态（T4D 新算 / T4D 补水 / T4 新算 / T4 补水 / T0 存档）共 105 条解释全渲染出来数了一遍，这个定性需要推翻：

```
=== rendered 105 explanations over 21 fixtures ===
--- fallback branch (t4d.rs:190) hits: 11 ---
```

**11 次命中，全部落在 `T0-stored` 这一列**，T4D/T4 的新算与补水分数一次都不触发。也就是说触发条件不是「罕见」，而是**精确地就是 Goal 1 现行存档 band 那条路径**——`DECISION.md` §6.5 写明「在替换合并完成前，Goal 1 现行 `Tally::band` 只是遗留行为」，即过渡期里从图里读回来的边走的正是这条。命中的 11 个夹具包括 `group_only_50`、`quiet_180_days`、`dormant_2019`、`group_heavy_plus_one_direct_each_way` 这些决胜与边界夹具。实渲染样例：

> 你们一共有 50 次往来：一对一 0 次……你们从来没有单独聊过，一对一 0 次，看不出你们私下有来往；你们在群里还有 50 次往来，那只说明你们常在同一个场合，不算进这一档，**按现在的规则应是弱联系。存档里这一档是强联系。**

三件事叠在这一句上：

1. **COPY_ZH 未收录**——T4D 的九条模板里没有这个形状，DECISION §6.4 的程序是先改 COPY_ZH 再改代码；
2. **零测试**——全仓库对「存档里」「按现在的规则」grep 只命中 `t4d.rs:190` 源码本身，没有任何测试断言过它的存在或不存在。`roundx_opus_a.rs:207` 那个跑全夹具的 `an_explanation_never_contradicts_the_counts_it_prints` 只检查措辞不与计数矛盾，兜底句满足这一点，于是安静通过；
3. **逐字违反 COPY_ZH §5 断言 4**（「任何模板不出现『今天』『现在』字样」）——「按**现在**的规则」。

还有一层来龙去脉值得写进去，否则容易被误读成纯缺陷：`roundx_opus_a.rs` 模块头 §1 记录的原缺陷是「11 个夹具 / 26 个夹具-规则组合会说出与计数矛盾的话，其中 **11 条在存档 Goal 1 band 上**」。T4 的 9 条与 T4D 的 6 条已经被「从计数重算措辞」修掉了，而**这 11 条存档 band 的出口就是这条兜底句**——数字对得上，不是巧合。所以它是一个**正确的修法**（诚实披露存档与重算不一致，好过硬说一个计数不支持的档），只是这个修法的产物从未进过 COPY_ZH，也从未被测试钉住。倾向补文档 + 补测试，而不是删句。

### 2.4 乙5 / 甲3：断言 4 与断言 5 在 A2 侧的实测，外加一个会坑到实施者的陷阱 ★

同法渲染 A2 的 15 个夹具共 59 条 bullet：

| 观测 | 结果 |
|---|---|
| 真正含「现在」的句子 | **2/15 夹具**（`dormant_but_strong`、`quiet_for_exactly_the_threshold`），均为沉寂句「……不是现在的联系频率。」= 乙5 |
| 渲染出「中」的夹具 | **5/15**（`group_only_reciprocal`、`single_day_burst`、`quiet_for_exactly_the_threshold`、`group_only_with_split`、`half_a_split_is_not_a_split`）= 甲3，`PRODUCT_LOCK.md:78` 的违反是**活的**，不是理论的 |
| 含「强」/「弱」的夹具 | 4 / 3，均合规 |

**陷阱**：我第一版探针用裸 `contains("现在")`，15/15 全中。多出来的 13 个是假阳性——A2 的 P1 总量句是「有记录的往来 N 次，**出现在** M 个不同的日子」，「出现在」里含「现在」两字。

任何人照 COPY_ZH §5 断言 4 的字面（「不出现『今天』『现在』字样」）写一条 `assert!(!text.contains("现在"))`，**会在 13 个合法夹具上误红**，然后很可能被误判成「断言 4 写错了」而放弃。正确写法要把「出现在」先排除，或者按 §0.5 的真实意图改判据（禁的是**把基准日说成阅读当日**，不是禁这两个字的任何出现）。这一条建议随乙1 的裁决一起写进 COPY_ZH，否则断言 4 落地时一定会踩。

顺带纠正 R1 §B2 的一句话：R1 写「断言 4……当前实现**恰好不含这两词**，但没有护栏」。这是错的——tie crate 在 11 个夹具上渲染「按现在的规则」，trait crate 在 2 个夹具上渲染「不是现在的联系频率」。**断言 4 不是「无测试但恰好满足」，是「无测试且今天就不满足」。**

---

## 3. 建议的执行顺序

按「改完之后会不会有新东西变红」排，而不是按严重度——目的是让每一步都能独立提交、独立回滚。

**第一批（甲，无需任何裁决，做完 249 → 约 253 项全绿）**

1. 甲1 墙钟扫描，用 §2.1 那版剥注释的写法，**不要用前两轮说的一行版**。
2. 甲2 D52 等值测试，落在 `soul-algo-trait` 的 dev-dep 上（§2.2）。这两条做完，R2 用四组探针证明过「静默不可检测」的两个漂移面就都有护栏了。
3. 甲3 `"中"` → `"中等"`。三份权威文档（COPY_ZH §0.1、COPY_ZH §5.5、PRODUCT_LOCK:78）一致要求，没有任何一方为「中」辩护过，所以它虽然改用户可见文本却**不需要裁归属**；仍建议在 DECISIONS 追一行。
4. 甲4 / 甲5 / 甲6 纯清理与纯新增测试。甲6 最好早做——它现在会让下一个读这份测试的人以为有 26 处未决缺陷。
5. 甲7 `Option<i64>`，本批最侵入，可单独一提交。

**第二批（乙，必须先有裁决输入）**

父代理需要先回答两个问题，其余六条都是它们的推论：

- **Q1**：tie crate 的三字档位词（乙2）与三条全局强制项（乙1），是代码向 COPY_ZH 靠，还是 COPY_ZH 追认代码？
- **Q2**：两条规范外增补句（乙3 兜底句、乙4 一对一日期句）留还是删？我的建议是**都留**，理由是两条都在披露真实信息（存档与重算不一致、一对一历史的年龄），删掉是让用户少知道一件事；但两条都必须补进 COPY_ZH 并补测试，乙3 尤其急——它在 11/21 个夹具上是过渡期用户实际会读到的句子，今天却一条测试都没有。

裁完之后，乙5 / 乙6 / 乙7 分别是「改沉寂句并同步它那条断言」「按 D40 补登记再统一 A2 六句」「在 DECISIONS 追一条 DECISION.md 勘误号」，都是机械执行。

落地乙1 时另有一个接口坑（R2 §2.4 已查明，此处只提醒）：COPY_ZH §5.1 点名的 `assert_non_clinical` 在 crate 里实际叫 `assert_publishable`，而**含第三方 claim 筛的是 `assert_publishable_about_peer`**。照名字直接接前者会漏掉整张 `PEER_CLAIM_DENYLIST`。

---

## 4. 与前两轮结论的差异清单

只列我改动了前轮判断的地方，其余一律沿用。

| 前轮结论 | 本轮 | 依据 |
|---|---|---|
| R1 §建议1 / R2 §4.3-1：墙钟修法是「一行数组改动，零风险」 | **推翻**。照做当场红 1 项，原因是 `lib.rs:55` 注释里有 `SystemTime` 字样；可用版需先剥注释 | §2.1 实跑 |
| R1 §B2：断言 4「当前实现恰好不含『今天』『现在』」 | **推翻**。tie 11 个夹具 + trait 2 个夹具今天就渲染「现在」 | §2.3 / §2.4 实测 |
| R1 §B5：兜底句「触发条件更罕见」 | **推翻**。11/21 夹具命中，且精确对应 Goal 1 存档 band 这条过渡期主路径；另查明它零测试覆盖 | §2.3 实测 105 条渲染 |
| R2 §4.3-2：D52 测试挂在 `soul-algo-tie` 的 dev-dep 上 | **调整落点**为挂在 `soul-algo-trait` 上，与 §6.2 的依赖箭头同向 | §2.2 |
| R1 §A3 / R2 §2.3：「中」违反 PRODUCT_LOCK | **维持，补量化**：5/15 个 A2 夹具实际渲染出来 | §2.4 |
| 任务给定的二分「可立刻修 / 须走回退链」 | **回退链一格为空**；真正的第二格是 DECISION §6.4 留痕通道 | §0 |

---

## 5. 明确不在本表的两项

这两条前两轮都记为最高优先，本表不收，因为它们在 main 上**修不完**，收进来会让「可在 main 修」这个筛选条件失效：

- **`DORMANT_NOTE_DAYS` 单点定义（R1 A1）**：DECISION §3 要求它与 `DEMOTE_ONE_BAND_DAYS` 是同一个常量，而两个 crate 零依赖，物理上无法共享。根治要合并 crate，而 D52 已经拍板「不合，后置」。**能在 main 上做的只有 D52 指定的减灾手段，即本表甲2**；做完之后分裂仍然存在，但漂移会立刻变红。这个区别要在向作者汇报时讲清楚，否则甲2 容易被读成「A1 已解决」。
- **统一 `crates/soul-algo`（R1 C5）**：DECISION §0 与 §6.1 的合并义务本体，D52 已后置。tie 与 trait 各有一个同名不同定义的 `TieScore`、两套禁词表、两套档位词都是它的下游后果；甲2、甲3 只是把其中两个后果各钉住一颗钉子。

---

## 6. 复现

只读侧（`/workspace`，不改任何字节）：

```bash
git diff main HEAD -- crates docs                                  # 空：本树 = main
cargo test --workspace                                             # 249 项全绿
rg '这是工作假设|怎么算的|不读聊天内容|截止日期|判档只看' crates/     # 0 命中（乙1）
rg 'DORMANT_NOTE_DAYS' crates/                                     # 0 命中（§5）
rg 'SystemTime|Instant|std::time|chrono|now\(\)' crates/           # 仅 2 处文档注释（甲1）
rg '存档里|按现在的规则' crates/                                    # 仅 t4d.rs:190 源码本身（乙3 零测试）
rg '#\[ignore\]|fn bug_' crates/                                   # 仅 roundx 注释自己（甲6）
```

验证侧（`/tmp/r3a`，工作树之外的 tar 拷贝，四组）：

| 探针 | 改动 | 结果 |
|---|---|---|
| Q1 兜底句可达性 | 新增只读探针，渲染 21 夹具 × 5 形态 = 105 条 | 兜底句 11 命中，全在 `T0-stored`；「今天/现在」11 命中，同一批 |
| Q2 A2 渲染面 | 新增只读探针，15 夹具 × 59 bullet | 「现在」真命中 2、假阳性 13（「出现在」）；「中」命中 5 |
| Q3 墙钟扫描 | 照 R1/R2 原方案改 `banned` 数组 | **红 1 项**（`lib.rs` 注释）；改剥注释版后绿，贴回 R2 的 `score_now` 后按预期红 |
| Q4 D52 等值测试 | `soul-algo-trait` 加 dev-dep + 一条 `assert_eq!` | 今天绿（251）；重放 R2 的 180→150 单侧漂移后按预期红 |
