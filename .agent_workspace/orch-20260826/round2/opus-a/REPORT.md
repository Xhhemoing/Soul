MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 2 / opus-a：COPY_ZH 与跨 crate 常量双源专项核实

核实对象：Round 1 各槽位（主要是 `round1/opus-a`，旁证 `round1/opus-b`、`round1/fable-b`、`round1/gpt-sol-a`）就三件事提出的主张。
基准工作树：`cursor/project-status-audit-c49c`，`crates/soul-algo-tie` v0.3.0 + `crates/soul-algo-trait` v0.3.0。
权威面：`docs/algorithms/DECISION.md`（ALGO_FROZEN）、`docs/algorithms/COPY_ZH.md`、`docs/PRODUCT_LOCK.md`、`docs/DECISIONS.md`。

**本报告未改动 `crates/` 下任何文件，未做 git checkout / commit / push。** 所有反证实验在 `/tmp/wc-probe`（工作树之外的一份拷贝）中进行，`git status --porcelain` 对 `crates/` 与 `docs/` 全空。

---

## 0. 核实结论速览

| R1 主张 | 核实结论 | 关键证据 |
|---|---|---|
| `DORMANT_NOTE_DAYS` / `DEMOTE_ONE_BAND_DAYS` / `DORMANT_AFTER_DAYS` 的 `180` 字面量分裂 | **成立，且比 R1 描述的更严重** | 两处字面量物理不可共享；**反证实验：单侧改 180→150 后全工作区 249 个测试全绿**，并当场复现 PRODUCT_LOCK 红线三之三禁止的同屏矛盾 |
| COPY_ZH 强制句在 crate 中缺失 | **成立**：收尾句 / 透明句 / `{截止日期}` 三串在 `crates/` 全树 **0 命中** | `rg` 见 §2.1，实渲染输出见 §2.2 |
| 档位词「中」vs「中等」 | **成立，且应升级定性**：这不只是 COPY_ZH 漂移，是 `docs/PRODUCT_LOCK.md:78` 的产品承诺违反 | `soul-algo-trait/src/types.rs:87` → `a2.rs:429-432` 渲染 |
| 墙钟扫描未锁进测试 | **成立，但 R1 的措辞（「加一行 `SystemTime::now()` 不会有测试变红」）过强，需修正** | 反证实验：**粗暴替换会红 28 个测试**；但 DECISION §6.3 真正点名禁止的那种形态（缺省墙钟入口）**可以 135/135 全绿 + clippy 干净地加进去** |

**对 ALGO_FROZEN 的裁断：三条全部不阻塞冻结。** 但第一条与第二条合起来使「话术单源」这一说法在今天**不成立**，第一条同时使 `DECISIONS.md` D52 的减灾义务处于**已拍板未执行**状态。详见 §4。

---

## 1. 常量双源：`180` 的两处字面量

### 1.1 规范面怎么说

四份权威文件对这条的要求是叠加的，且措辞一次比一次硬：

```45:55:docs/algorithms/DECISION.md
## 3. 常量表（单点定义，全仓库禁止第二处出现数字字面量）
...
| `DEMOTE_ONE_BAND_DAYS` | 180 | 沉寂 ≥ 180 天降一档，**闭区间**（`>=`；任何 `>180` 写法作废） |
| `FORCE_WEAK_DAYS` | 360 | 沉寂 ≥ 360 天封 Weak，闭区间 |
| `DORMANT_NOTE_DAYS` | = `DEMOTE_ONE_BAND_DAYS` | A2「最近半年没有往来」句阈值，与降档共用同一常量，不得分裂 |
```

- `docs/algorithms/COPY_ZH.md:73`：「P4 的 180 天与降档常量是**同一个常量**（`DORMANT_NOTE_DAYS = DEMOTE_ONE_BAND_DAYS = 180`，单点定义见 DECISION.md 常量表）」，并据此宣称一条不变式「**免费成立**」——出现半年句的边必然已降档，永远不是强。
- `docs/PRODUCT_LOCK.md:80` 红线三之三：「降档与『你们最近半年没有往来』是同一个阈值。用户读到那句话时，档位必然已经降下，两者**不得同屏矛盾**。」
- `docs/DECISIONS.md:63` D52：「不合，后置。**两侧天数常量用工作区测试钉相等**；产品 crate 禁止再写第三处 `DEMOTE_ONE_BAND_DAYS` 字面量。」

D52 这条特别重要：父代理已经**知道**并**接受**了 crate 不合并，但同时**指定了减灾手段**（工作区等值测试）。所以这条的正确定性不是「合并没做完的自然后果」，而是「已拍板的减灾义务未执行」。

### 1.2 代码面的实际状态

`DORMANT_NOTE_DAYS` 这个名字在 `crates/` 全树 **0 命中**（`rg DORMANT_NOTE_DAYS crates/`）。实际存在的是两个互不知情的 `180`：

```50:58:crates/soul-algo-tie/src/constants.rs
/// Silence from here on costs one band.
pub const DEMOTE_AFTER_SILENT_DAYS: i64 = 180;

/// Silence from here on caps the tie at Weak, whatever the history says.
pub const WEAK_AFTER_SILENT_DAYS: i64 = 360;

/// Names frozen in `docs/algorithms/DECISION.md` §3. Same values as above.
pub const DEMOTE_ONE_BAND_DAYS: i64 = DEMOTE_AFTER_SILENT_DAYS;
pub const FORCE_WEAK_DAYS: i64 = WEAK_AFTER_SILENT_DAYS;
```

```83:91:crates/soul-algo-trait/src/a2.rs
/// After how many days without contact the summary adds a dormancy sentence.
///
/// **This is not a band rule.** It gates one descriptive line and nothing else;
/// [`TieScore::band`] passes through untouched, and
/// `dormancy_is_a_sentence_not_a_demotion` pins that. A relationship that has
/// been quiet for a year is still whatever the graph says it is — A2's job is
/// to make sure the user is told about the quiet, not to overrule the graph
/// about what it means.
pub const DORMANT_AFTER_DAYS: i64 = 180;
```

两个 crate 的 `[dependencies]` 都是空的（`crates/soul-algo-tie/Cargo.toml:11-13`、`crates/soul-algo-trait/Cargo.toml:11-12`），互不依赖，**物理上无法共享常量**。

两侧各自钉的是**字面量**而不是彼此：

- `crates/soul-algo-tie/src/constants.rs:76` — `assert_eq!(DEMOTE_AFTER_SILENT_DAYS, 180);`
- `crates/soul-algo-trait/tests/a2_render.rs:303` — `assert_eq!(DORMANT_AFTER_DAYS, 180);`

D52 要求的那条「钉相等」的测试，两棵树上都不存在。

### 1.3 一条需要更正的历史结论：比较符已经是闭区间

`.agent_workspace/roundx/fable-a/CONSISTENCY.md:22` 与 `roundx/fable-b/LOCK_FIT.md:62` 记录 A2 侧「比较用严格 `>`（`a2.rs:185`）」，并据此提出一个「一天宽的边界摆动」。**这条今天已不成立**：

```183:186:crates/soul-algo-trait/src/a2.rs
    /// Whether the dormancy sentence applies.
    pub fn is_dormant(&self) -> bool {
        matches!(self.days_since_last_contact(), Some(days) if days >= DORMANT_AFTER_DAYS)
    }
```

`COPY_ZH.md:70` 也已改成「达到 `DEMOTE_ONE_BAND_DAYS`（闭区间 `>=`，与降档同一常量）」，与 `DECISIONS.md:71` D60 一致。**方向与闭区间现在三方一致，剩下的纯粹是「两处字面量」这一件事。**

### 1.4 反证实验：这个分裂今天完全没有护栏

R1 各槽位都把这条记成「风险」。我做了一次反证，把它从风险变成**已量化的事实**。

在 `/tmp/wc-probe`（工作树之外的完整拷贝）里模拟一次**内部自洽的单侧修改**——正是 crate 分裂时最可能发生的那种改动：开发者按某个理由调整 A2 的沉寂阈值，同步改掉 trait crate 自己的常量、自己的夹具、自己的断言，而完全不知道 tie crate 那半边存在。

改动共四处，全部在 `soul-algo-trait` 内：`src/a2.rs:91` 的 `180 → 150`、`src/fixtures.rs:176` 的夹具天数、`tests/a2_render.rs:303/306/311` 的三处断言。

结果：

```
cargo test --workspace --no-fail-fast
→ 19 个测试二进制，0 个 FAILED，249 项全绿
```

也就是说：**A2 的沉寂阈值现在是 150，T4D 的降档阈值现在是 180，全工作区没有一个测试察觉到。**

再用一个跨 crate 的探针程序（两个 crate 同时链入）确认这不是纸面问题，而是当场可复现的用户可见矛盾。构造一条一对一互惠 12 次 / 6 天、最后一次往来在 160 天前的边：

```
tie band = Strong, silent_days = 159
a2 days = Some(160), dormant = true
  A2: 最近一次往来距最新的记录 160 天。
  A2: 已经 160 天没有新的往来了；下面的归档说的是过去的记录，不是现在的联系频率。
  A2: 按上面的计数，这条往来归在「强」一档。这是对记录的归档，不是对这个人的评价。
```

沉寂句与「强」一档**同屏**。这正是 `PRODUCT_LOCK.md:80` 红线三之三明文禁止、`COPY_ZH.md:73` 声称「免费成立」的那条不变式的反面。

我也跑了反方向（`150 → 200`）：同样 249 项全绿，只是那个方向的漂移不破坏不变式（句子出现得更晚，降档已先发生）。**所以危险是单向的**：任何让 A2 阈值小于降档阈值的漂移都会立刻破锁，而且静默。

**结论**：R1 关于「实质双源」的主张成立。R1 opus-a 把它列为 A1（最高优先）、R1 opus-b 称之为「本轮唯一一个『已拍板要做、但确实没做』的工程义务」——两种定性都站得住，我的实验只是把「会漂移」升级为「漂移已被验证不可检测，且破产品锁只需一次单侧改动」。

### 1.5 附带发现：`86_400` 是四处，其中一处是 crate 内部重复

R1 opus-a §A1 末尾提到「两边还各自定义了一份 `SECONDS_PER_DAY`」。准确数字是四处：

| 位置 | 形态 |
|---|---|
| `crates/soul-algo-tie/src/types.rs:31` | `pub const SECONDS_PER_DAY: i64 = 86_400;` |
| `crates/soul-algo-trait/src/types.rs:186` | `pub const SECONDS_PER_DAY: i64 = 86_400;` |
| `crates/soul-algo-trait/src/a2.rs:94` | `const SECONDS_PER_DAY: i64 = 86_400;` |
| `crates/soul-algo-trait/src/fixtures.rs:23` | `pub const DAY: i64 = 86_400;` |

第三处值得单独指出：它是 **crate 内部**的重复定义——`soul-algo-trait` 自己的 `types.rs:186` 已经 `pub` 导出了同名同值常量，`a2.rs` 仍然私有地又写了一份。这一处与跨 crate 无关，是纯粹的就地可修项。不是判档阈值，D52 的「禁止第三处字面量」严格说管不到它；但 `a2.rs:19` 的模块文档写着「The one number A2 owns is `DORMANT_AFTER_DAYS`」，实际它拥有两个。

---

## 2. COPY_ZH 强制句缺失与档位词

### 2.1 三个全局强制项在 crate 中 0 命中

COPY_ZH §0 里有三条是**对所有模板生效**的硬性要求：

```13:15:docs/algorithms/COPY_ZH.md
5. **时间口径**：一切「距今」表述实为「距 {as_of 日期}」；as_of 来自数据（调用方传入或数据内最大时间戳），不读墙钟。模板里写「到 {截止日期}」，不写「到今天」。
6. **收尾句**（每条边解释与每条推断解释必带）：「这是工作假设，你可以直接改。」
7. **透明句**（每套边解释的首句）：「怎么算的：只统计次数和日期，不读聊天内容。」T4D 版在其后追加半句「判档只看你们一对一的往来。」
```

对 `crates/` 全树检索这五个特征串——`这是工作假设`、`怎么算的`、`不读聊天内容`、`截止日期`、`判档只看`——**结果为零命中**（`rg` 无匹配）。R1 opus-a 的结论逐字复核成立。

### 2.2 实渲染输出（在拷贝树中用未改动的源码打印）

四个代表性夹具的 T4D 解释全文：

```
== lilei_12 (Strong)
你们一共有 12 次往来：一对一 12 次（你发出 6 次，对方发来 6 次），群里 0 次（你发出 0 次，对方发来 0 次）；这些往来分布在 6 天、1 个会话里，最近一次是 2026 年 8 月 21 日，距今 3 天。你们一对一聊过 12 次，双方都发过，不少于 10 次，而且分布在 6 天里，也不少于 3 天，最近一次往来距今 3 天，还不到 180 天，不用往下降，所以算强联系。

== quiet_180_days (Moderate)
……本来可以算强联系；不过你们最近一次往来距今 180 天，已经不少于 180 天没有联系，所以往下降到中等联系。要是你们又聊起来，这一档会自己涨回去。
```

三条强制项的落地情况逐条对照：

| COPY_ZH 条款 | 冻结要求 | 实现 | 判定 |
|---|---|---|---|
| §0.7 透明句 | 每套边解释的**首句** | 首句是「你们一共有 N 次往来：…」 | ❌ 缺失 |
| §0.6 收尾句 | 每条边解释**必带** | 无 | ❌ 缺失 |
| §0.5 时间口径 | 写「到 {截止日期}」，不写「到今天」 | 一律「距今 N 天」，`as_of_unix` 虽在 `TieScore` 上（`soul-algo-tie/src/types.rs:177`）但**任何模板都不渲染它** | ❌ 未落地 |

关于 §0.5 需要一句精确定性：实现**没有**违反 COPY_ZH §5.4 的字面禁令（禁的是「今天」「现在」二词，「距今 N 天」不含这两个词，`rg` 确认这两词在渲染路径 0 命中）。它违反的是 §0.5 的**口径要求**——冻结文本要求把基准日显式写出来，实现把基准日隐去了。这是 as_of 纪律在话术层的缺口；算法层完全合规（§3）。

A2 侧同样缺收尾句。`crates/soul-algo-trait/src/a2.rs:427/430` 的 P5 句以「这是对记录的归档，不是对这个人的评价。」收尾，而 COPY_ZH §4 P5 的冻结收尾是「这是工作假设，不是对这个人的判断。」——两句语义相近但不是同一句，且都不是 §0.6 要求的那句。R1 opus-b §5 已把 A2 六类句的逐句漂移量化，本报告不重复。

### 2.3 档位词：三套词表，且违反的是 PRODUCT_LOCK 而不只是 COPY_ZH

| 来源 | Strong | Moderate | Weak | 其他 |
|---|---|---|---|---|
| `docs/algorithms/COPY_ZH.md:9`（冻结，词汇单源） | 强 | **中等** | 弱 | — |
| `docs/PRODUCT_LOCK.md:78`（产品承诺） | 强 | **中等** | 弱 | — |
| `crates/soul-algo-tie/src/types.rs:89-95` `as_zh()` | 强**联系** | 中等**联系** | 弱**联系** | — |
| `crates/soul-algo-trait/src/types.rs:83-90` `label_zh()` | 强 | **中** | 弱 | `Band::None` → 「还看不出」 |

trait crate 的「中」是**用户可见**的：

```424:433:crates/soul-algo-trait/src/a2.rs
fn filing_bullet(score: &TieScore) -> SummaryBullet {
    let text = match score.band {
        Band::None => {
            "按上面的计数，还看不出该归在哪一档。这是对记录的归档，不是对这个人的评价。".to_owned()
        }
        band => format!(
            "按上面的计数，这条往来归在「{}」一档。这是对记录的归档，不是对这个人的评价。",
            band.label_zh()
        ),
    };
```

R1 opus-a 把这条记在 COPY_ZH §0.1 与 §5.5 名下。**我认为定性应当升级**：`docs/PRODUCT_LOCK.md:78` 写的是「用户可见文本**只有**「强 / 中等 / 弱」三个词」，这是产品锁本体的承诺，不是话术文档的偏好。也就是说 `label_zh()` 的「中」同时越过了三道线（COPY_ZH §0.1 词汇单源、COPY_ZH §5.5 验收断言、PRODUCT_LOCK 档位承诺），而修法是一处字面替换，不动任何逻辑、不动任何 band。R1 opus-b 建议「无论父代理怎么裁都要先修这一条」——我同意，并补充理由：它是三条主张里**唯一一条无需先做产品裁决就能修**的。

tie crate 的「强联系 / 中等联系 / 弱联系」性质不同：三字词包含冻结词作为子串，`assert_publishable_about_peer` 也放行（见下），但它仍然是第四套写法，且 COPY_ZH §5.5 的字面要求是「只出现三种」。这一条**必须**先由父代理裁「改代码对齐 COPY_ZH」还是「改 COPY_ZH 追认代码」，因为 DECISION §6.4 的程序是先改 COPY_ZH 再改代码。

### 2.4 R1 opus-a §A3「靠字形差异躲过筛子」这一句需要更正

R1 opus-a 说 tie crate 输出的「强联系」与 `PEER_CLAIM_DENYLIST`（`crates/soul-algo-trait/src/denylist.rs:56-71`）里的「强关系」只差一个字，「靠字形差异而非设计意图躲过了这道筛」。

我把 tie crate 的全部 42 条解释（21 夹具 × 2 规则）实际送进 trait crate 的筛子跑了一遍：

```
checked 42, failed 0
强联系: Ok(())      中等联系: Ok(())      弱联系: Ok(())
强关系: Err(ForbiddenWord { screen: PeerClaim, word: "强关系" })
```

结论要分两半说：

1. **今天没有安全问题**——tie crate 的输出即使经过 trait crate 最严的那道筛（`assert_publishable_about_peer`）也全部通过。「躲过筛子」这个说法容易被读成「有不该发布的内容溜过去了」，实际没有。
2. **但结构性问题是真的**——tie crate 从**未**调用这道筛（零依赖，物理上调不到），42 条全过是复核出来的巧合而不是护栏挡出来的结果。另外顺带查明一处 R1 未提的接口细节：COPY_ZH §5.1 点名的 `assert_non_clinical` 在 crate 里实际叫 `assert_publishable`（`denylist.rs:112`），而它**不含**第三方 claim 筛；含 claim 筛的是 `assert_publishable_about_peer`（`denylist.rs:120`）。将来落地 COPY_ZH §5.1 的绑定测试时若照名字直接接 `assert_publishable`，会漏掉 `PEER_CLAIM_DENYLIST` 那一整张表。

### 2.5 `DECISIONS.md` D40 承诺的「另记」确实不存在

```51:51:docs/DECISIONS.md
| D40 | Goal 1 人事摘要走哪套话术 | `soul-draft` 调用冻结 `soul_algo_trait::a2_render`；不改 a2.rs 模板；锁定边仍丢掉 `filed_band` | A2 是渲染器；COPY_ZH 与 crate 文案漂移另记 |
```

对 `docs/` 全树检索「漂移」，命中只有 `REJECTED.md:84`、`REJECTED.md:113`（讲 T3R milliscale 与 T3 附加门）与 `DECISION.md:72`、`DECISION.md:96`（讲历史作废项与 Goal 1 常量导入）。**没有任何一处「另记」了 COPY_ZH 与 crate 文案的漂移。** R1 opus-b 的这条主张核实成立。这意味着现在有两份都自称冻结的话术权威（COPY_ZH.md 与 a2.rs/t4d.rs 的实际模板）互相矛盾，而 D40 承诺的登记通道是空的。

---

## 3. 墙钟：事实成立，护栏缺失，但 R1 的措辞需要修正

### 3.1 事实核实

`rg 'SystemTime|Instant|std::time|chrono|now\(\)' crates/` 在**两个 crate 全树**只有两处命中，且都是文档注释：

- `crates/soul-algo-tie/src/lib.rs:55` —「nothing here calls `SystemTime`, so the same evidence scores the same in…」
- `crates/soul-algo-tie/tests/as_of_discipline.rs:5` —「**No clock.** Nothing in the crate reads `SystemTime`…」

无一处代码。R1 opus-a §1.3 与 R1 gpt-sol-a §墙钟检查的事实陈述均成立。

### 3.2 已有的源码级扫描确实不含墙钟串

`crates/soul-algo-tie` 只有一处 `include_str!` 源码扫描：

```535:562:crates/soul-algo-tie/tests/ablation.rs
#[test]
fn the_product_path_contains_no_floating_point() {
    // C2 in the shared brief asks for no float instability. ...
    for (name, source) in [
        ("lib.rs", include_str!("../src/lib.rs")),
        // ... 9 个产品路径源文件 ...
    ] {
        for banned in ["f64", "f32"] {
            assert!(
                !source.contains(banned),
                "{name} mentions {banned}: the product path must be integer-only"
            );
        }
    }
}
```

`banned` 数组只有 `f64` / `f32`。`soul-algo-trait` 侧的两处源码扫描（`tests/frozen_defaults.rs:238-256`、`tests/a2_render.rs:26-27`）禁的是阈值标识符（`STRONG_MIN`、`>= 10` 等），也不含墙钟串。**两棵树上没有任何源码级墙钟扫描。** R1 主张成立。

### 3.3 反证实验一：R1 的措辞过强

R1 opus-a §B1 写：「任何人加一行 `SystemTime::now()` 不会有测试变红。」我实测了这句话。

在拷贝树中把 `soul-algo-tie/src/types.rs` 的 `Tally::silent_days` 改成忽略 `as_of_unix`、改读 `SystemTime::now()`：

```
Running unittests src/lib.rs         FAILED. 46 passed; 14 failed
Running tests/ablation.rs            FAILED. 15 passed;  5 failed
Running tests/as_of_discipline.rs    FAILED.  8 passed;  2 failed
Running tests/direct_gate.rs         ok.      8 passed
Running tests/explain_zh.rs          FAILED. 14 passed;  3 failed
Running tests/goal1_fidelity.rs      ok.      5 passed
Running tests/roundx_opus_a.rs       FAILED. 11 passed;  3 failed
Doc-tests                            FAILED
                                     → 共 28 项变红
```

**所以 R1 那句话字面上是错的。** 但要说清楚这 28 项为什么红：它们红是因为**取值变了**（真实墙钟 2026-08-26 与锚点 as_of `AS_OF_2026_08_24` 差两天，单元测试里的合成时间戳更是差上万天），而不是因为有任何测试在检查「有没有读钟」。这是副作用，不是护栏——它取决于「今天」恰好不等于锚点日期这一偶然条件。

### 3.4 反证实验二：真正被点名禁止的那种形态可以静默加进去

DECISION §6.3 的原文不是「不许出现 `SystemTime` 字样」，而是：

> **as_of 传递**：rebuild 调用方计算全库 `max(occurred_at)`（或显式传值）后传入；**soul-algo 不提供缺省墙钟路径**。

于是我测了这个精确形态。在拷贝树的 `soul-algo-tie/src/lib.rs` 里，紧挨着现有入口加一个便利函数（现有入口一个都不动）：

```rust
/// Convenience entry point for callers that have no `as_of` to hand.
pub fn score_now(peer_id: u64, interactions: &[Interaction]) -> TieScore {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    TieAlgo::DEFAULT.score(peer_id, interactions, now)
}
```

结果：

```
cargo test -p soul-algo-tie --no-fail-fast   → 135 passed; 0 failed（8 个二进制全绿）
cargo clippy -p soul-algo-tie --all-targets -- -D warnings   → 干净，零警告
```

**DECISION §6.3 明文禁止的东西，可以在 135/135 全绿、clippy 零警告的情况下加进 crate 的公开 API。** 这才是这条缺口的准确形状：现有测试保护的是「已有代码路径的取值不受钟影响」，完全不保护「有没有新增一条读钟的路径」。而后者恰恰是 DECISION 写下来的那一条。

顺带说明为什么源码扫描是对的补法而不是唯一补法：`score_now` 这种新增入口不会改变任何现有断言的取值，任何形式的**行为**测试都抓不到它；只有源码级扫描（把 `SystemTime` / `Instant` / `std::time` 加进 `ablation.rs:555` 那个 `banned` 数组）能抓。R1 opus-a 提的修法本身是对的，我只是把它的必要性从「便宜就顺手做」提升到「这是唯一能覆盖该形态的手段」。

### 3.5 对 R1 opus-b 与 R1 opus-a 分歧的裁断

R1 opus-b 表 A2-10 把「不读墙钟」判 ✅，依据是 `crates/soul-algo-trait/tests/a2_render.rs:338-350` 的 `shifting_as_of_and_last_contact_together_changes_nothing`。R1 opus-a 判「无测试锁定」。两者都对了一半：

- opus-b 的依据成立**且比它自己说的更强**——把 `as_of` 与 `last_contact` 同时平移一年后输出必须全等，这个断言在 A2 的近因路径上确实能抓住墙钟替换（若改读钟，平移只动一边，天数差就变了）。
- 但这只覆盖 **trait crate 的近因路径**。`soul-algo-tie` 侧没有等价的整体平移测试——`tests/as_of_discipline.rs` 的十个测试里，`the_store_maximum_is_the_other_legal_as_of_and_the_shift_it_causes_is_recorded`（:87）平移的是 `as_of` 单边（用途是记录阈值锐度），`scoring_is_a_pure_function_of_the_evidence_and_the_as_of`（:63）只跑两遍同一调用，其注释自己也承认「Running the same call twice is a weak test on its own」。
- 而且如 §3.4 所示，**两者都不覆盖新增入口这一形态**。

综合裁断：**R1 opus-a 的结论方向正确，具体措辞需按 §3.3 修正；R1 opus-b 的 ✅ 应降为「trait crate 近因路径有行为覆盖，tie crate 无，两者均无源码级锁」。**

---

## 4. 对 ALGO_FROZEN 的阻塞评估

### 4.1 不阻塞冻结

三条主张全部**不阻塞** ALGO_FROZEN，理由是叠加的三层：

**第一层，都不动 band。** ALGO_FROZEN 冻结的是「规则、常量与话术」（`DECISION.md:20`），而改判只有一条合法通道——§5 回退链，其触发器是 §4.1（群聊代价）与 §4.2（F04c 复燃一响）两个**判档层**的已知代价。三条主张里：

- 常量分裂在 A2 侧，而 A2 是纯渲染器，`band` 原样透传（`crates/soul-algo-trait/src/a2.rs:12-17` 的设计声明 + `tests/frozen_defaults.rs:259-276` 的 `a2_is_a_function_of_the_band_it_is_handed` 逐夹具逐档验证）。`DORMANT_AFTER_DAYS` 只门控一条描述句，不门控任何档位。
- COPY_ZH 缺句与档位词纯属渲染面。
- 墙钟是工程约束，且事实上今天没有一处代码读钟。

我的实验也印证了这一点：单侧把 A2 阈值改到 150 之后，跨 crate 探针里 T4D 依然判 `Strong`——**档位没有动，动的是同屏说了什么**。

**第二层，DECISION §0 已经把其中两条预先归类为「冻结后的合并义务」。**

```20:20:docs/algorithms/DECISION.md
「统一 `crates/soul-algo`」与「模板绑定测试进合并 crate」是**冻结后的合并义务**（见第 6 节），不是算法选型的阻塞项：本决议冻结的是规则、常量与话术；合并工作以本决议为规范执行。
```

常量分裂是「未统一 crate」的直接后果，COPY_ZH 绑定测试缺失就是「模板绑定测试进合并 crate」本身。两条都在这句话的射程内。

**第三层，`DECISIONS.md` D52 已经对 crate 不合并单独拍过板**（「不合，后置」），所以分裂状态是**已定价**的，不是新发现的阻塞。

### 4.2 但阻塞「话术单源」

「话术单源」这个说法今天**不成立**，三条独立证据：

1. **词汇不单源**：同一批冻结文档要求的三个词（强/中等/弱），在代码里有两套不同实现（`强联系/中等联系/弱联系` 与 `强/中/弱`），其中 trait crate 的「中」连子串关系都不成立，且违反 `PRODUCT_LOCK.md:78`。
2. **模板不单源**：COPY_ZH §0.5/§0.6/§0.7 三条全局强制项在 `crates/` 全树 0 命中；A2 六类句只有 P1b 与冻结文本逐字一致（R1 opus-b §5 量化）。存在**两份都自称冻结**的话术权威，且 D40 承诺的「另记」通道是空的（§2.5）。
3. **阈值不单源**：`COPY_ZH.md:73` 用来论证「不变式免费成立」的那个前提（P4 阈值与降档阈值是同一个常量）在代码里是假的——是两个恰好相等的字面量，且 §1.4 证明它们分开之后没有任何测试会红。

第 3 条尤其要点名，因为它不是「话术与代码不一致」，而是**冻结文档为一条产品锁红线给出的论证依据在实现里不成立**。`PRODUCT_LOCK.md:80` 的「不得同屏矛盾」目前靠的是巧合，不是机制。

### 4.3 建议的处置分层（不在本报告范围内执行）

按「是否需要产品裁决」分层，而不是按严重度：

**无需裁决，纯工程，可立刻做：**

1. 在 `crates/soul-algo-tie/tests/ablation.rs:555` 的 `banned` 数组里加 `"SystemTime"` / `"Instant"` / `"std::time"`。这是 §3.4 那种形态的**唯一**可行护栏。一行改动。
2. 给 `soul-algo-tie` 加 `[dev-dependencies] soul-algo-trait = { path = "../soul-algo-trait" }`（方向正确：tie 是判档方，trait 是渲染方；且是 dev-only，不引入运行时依赖），写一条 `assert_eq!(constants::DEMOTE_ONE_BAND_DAYS, soul_algo_trait::a2::DORMANT_AFTER_DAYS)`。这就是 D52 已经拍板要的那条测试，做完之后 §1.4 的实验会立刻变红。
3. `soul-algo-trait/src/a2.rs:94` 的私有 `SECONDS_PER_DAY` 改为 `use crate::types::SECONDS_PER_DAY`。纯 crate 内清理。

**无需裁决但改用户可见文本，需留痕：**

4. `soul-algo-trait/src/types.rs:87` 的 `"中"` → `"中等"`。这是唯一一处「代码与三份权威文档全部冲突、且没有任何一方为代码写法辩护过」的档位词，改它不需要先裁归属。

**必须先由父代理裁决（DECISION §6.4：先改 COPY_ZH 再改代码）：**

5. tie crate 的「强联系」系列该向 COPY_ZH 靠拢，还是 COPY_ZH 追认三字词。
6. §0.5 / §0.6 / §0.7 三条强制项落地，以及 A2 六类句的归属。这一条落地时注意 §2.4 的接口坑：COPY_ZH §5.1 点名的 `assert_non_clinical` 对应的是 `assert_publishable_about_peer`，不是同名的 `assert_publishable`。

**随合并义务一起做：**

7. 统一 `crates/soul-algo`，常量单点化，COPY_ZH 绑定测试落地。第 2 项做完之后这一项的紧迫性会下降，但不会消失。

---

## 5. 复现方式与未改动声明

工作树侧（只读，全部在 `/workspace` 执行）：

```bash
rg 'DORMANT_NOTE_DAYS' crates/                                   # 0 命中
rg '这是工作假设|怎么算的|不读聊天内容|截止日期|判档只看' crates/    # 0 命中
rg 'SystemTime|Instant|std::time|chrono|now\(\)' crates/          # 仅 2 处文档注释
cargo test --workspace                                            # 249 项全绿，19 个二进制
```

反证实验侧（在 `/tmp/wc-probe`，工作树之外的拷贝，四组探针）：

| 探针 | 改动 | 结果 |
|---|---|---|
| P1 墙钟粗暴替换 | `Tally::silent_days` 改读 `SystemTime::now()` | 28 项变红（R1 措辞被证伪，见 §3.3） |
| P2 缺省墙钟入口 | 新增 `pub fn score_now(..)` | **135/135 全绿 + clippy 干净**（见 §3.4） |
| P3 常量单侧漂移（安全方向） | trait 侧 180→200 + 同步自身夹具与断言 | **249/249 全绿** |
| P4 常量单侧漂移（危险方向） | trait 侧 180→150 + 同步自身夹具与断言 | **249/249 全绿**，跨 crate 探针复现沉寂句与「强」同屏（见 §1.4） |

**未改动声明**：本次核实没有修改 `crates/` 或 `docs/` 下任何文件，没有执行 `git checkout` / `git commit` / `git push`。`git status --porcelain` 的唯一输出是本轮各槽位的 `.agent_workspace/orch-20260826/round2/` 未跟踪目录。本报告是本槽位写入的唯一文件。四组探针全部在 `/tmp/wc-probe` 的独立拷贝中进行，与工作树无交集。
