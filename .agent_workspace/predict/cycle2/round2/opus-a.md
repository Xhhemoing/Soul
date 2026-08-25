MODEL_SLUG: claude-opus-5-thinking-high-fast

# Cycle 2 / Round 2 / opus-a —— S3 近因的 as_of 规格：量化、钉住、单调

角色：**只读调研 + 规格**。零产品 crate 改动、零 docs 改动、**零 git 操作**，写入面恰为本文件。
读取时点：工作区除 `.agent_workspace/predict/`（未跟踪）外干净。
遵 `predict/CONSTRAINTS.md` 与 `cycle2/CONSTRAINTS.md`：不重开 P0–P8、不实现 Goal 2、不把候选算法写进产品 crate。
本轮任务：**收紧 S3 的 as_of，使 `plan_hash` 稳定**。独立完成，未读同轮他人稿。

本文写的是**规格**，不是实现：给类型签名、给 JSON 片段、给测试名与断言，但一行产品代码都不落盘。

---

## 0. 一句话结论，以及本文推翻自己的四处

**量化不能让 `plan_hash` 稳定。量化只能让它「每天最多动一次，且全目录同时动」。
真正把假拒绝降到零的是另外两件事：把量化后的 as_of 钉进批准、在执行时重放；
以及——与我上一轮的建议相反——绝不把它放进被哈希的值里。**

四条修订，改的都是 C2R1 opus-a §6.3（我自己上一轮写的）：

| # | R1 的话 | 本轮结论 | 为什么 |
|---|---|---|---|
| 1 | 「阈值加滞回」 | **撤回。不需要，也做不到** | 滞回需要上一次的判定结果；本 crate 无持久化，也不许有（`no_write_api.rs` 第二层）。而且不需要：`age` 对 as_of 单调，谓词只会单向熄灭，**结构上不可能抖**（§2.4 有证明） |
| 2 | 「把量化后的 as_of 放进 `to_json`」 | **反过来：绝不放进去** | as_of 的全部影响已经体现在 `moves`/`left_alone` 里。再把它单列进哈希，等于让「盘没变、计划内容一字不差」的两份计划因为跨了零点而哈希不等——**这正是要消灭的那种假拒绝**（§3.1） |
| 3 | 「T = 1 小时」 | **撤回。阈值必须等于量化粒度** | 日量化之下，「一小时前改过」不可表达。阈值只能以整 UTC 日计，措辞也必须随之变粗 |
| 4 | 理由名 `RecentlyActive`；短路顺序把它排在**最前** | 改名 `RecentlyModified`；排在**最后**（紧挨 `ProposedMove` 之前） | mtime 不是活跃度，R1 自己写过这句，理由名不能宣称数据支撑不了的东西。顺序改到最后，是因为**吃 as_of 的理由排得越靠后，能在午夜改变的文件就越少**（§4） |

另外把 **S3b（单件类别不建文件夹）从 S3 里拆出去**：它不吃 as_of，不是近因，是支持度。
它与本文的稳定性讨论**完全无关**，混在一起只会让两条都测不干净。本文只规格 S3a 与 S3c。

---

## 1. 先把问题的形状说准：今天什么会让 `plan_hash` 动

`to_json`（`plan.rs:164-192`）被 `PlanHash::of` 哈希。逐字段列出它的漂移源：

| 字段 | 什么情况下会变 | 与「现在几点」有关吗 |
|---|---|---|
| `action` | 永不 | 否 |
| `root` | 换挂载点 | 否 |
| `directory_snapshot` | **任何**条目的名字/类型/长度/mtime 变（mtime 精确到**纳秒**，`scan.rs:194-199`） | 否 |
| `executable_in_this_version` | 换一个会执行的构建 | 否 |
| `scanned_entries` / `skipped_entries` / `truncated` | 目录内容或限额变 | 否 |
| `moves[*].size_bytes` | 文件大小变 | 否 |
| 其余 | 计划内容变 | 否 |

**今天这张表里「与现在几点有关」一栏全是否**，这就是 `plan::build(scan) -> OrganizePlan` 是纯函数的全部含义。

两个必须先摆正的认识：

**(a) 计划哈希对「盘上的时间」早就极度敏感。** `directory_snapshot` 把每个条目的 mtime
按**纳秒**折进去了。碰一下任何一个文件的时间戳（哪怕内容不变、大小不变），哈希就变。
所以 S3 引入 mtime **不会**新增「盘上时间」这条漂移轴——那条轴已经拉满了。

**(b) S3 新增的漂移轴恰好只有一条：「现在几点」。** 这一条今天不存在，一旦引入就是全新的失败模式，
因为它**在盘完全没动的情况下**改变哈希。用户看到的那句话是
「计划在你批准之后变过了，请重新看一遍再决定」（`execute.rs:52`），而计划其实没变、目录也没变。
本文把这种情况叫**假拒绝**，定义是：

> 盘上快照相同 + 计划的 `moves`/`left_alone` 逐项相同 + `plan_hash` 不等。

**(c) 今天这个窗口是无界的。** `ActionKind::PlanFiles.needs_capability_token()` 为 `false`
（`hitl.rs:74-79`），所以 `plan.files` 的批准**不带令牌、不带 TTL**。批准到执行之间隔多久，
今天完全由界面决定，可以是一夜。这意味着「跨零点的概率」在今天不是小概率，而是
**用户把预览开着过夜就必然发生**。

由此得出一条对 v0.1.1 的前置建议（与 S3 无关，但 S3 使它变得紧要）：
**执行面必须要求 `CapabilityScope::FileWrite` 令牌**，这样批准到执行的窗口才被
`DEFAULT_TOKEN_TTL_MS = 120_000`（`hitl.rs:251`，两分钟）约束住。有了这个界，
下面 §2.3 的定量才有意义。

---

## 2. 规格：三层，缺一层都不成立

### 2.1 层 A —— as_of 只能从调用者进来（无时钟）

沿用 Cycle 1 的 G13 纪律（T4D 已冻结的同一条）：**时刻是参数，不是读数**。

两个类型，分工是「谁说粒度」与「谁说时刻」：

```rust
// crates/soul-fileplan（候选，未落盘）

/// 调用者说的：用不用近因、用多粗。**没有时刻**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecencyMode {
    Off,
    UtcDay { within_days: i64 },
}

/// `preview` 由 `RecencyMode` + 它已经收着的 `now_ms` 组装出来的，`build` 吃这个。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recency {
    Off,
    UtcDay { as_of: UtcDay, within_days: i64 },
}

pub fn build(scan: &DirectoryScan, recency: Recency) -> OrganizePlan;

pub fn preview(
    authorization: &Authorization, issuer: &mut TokenIssuer, raw: &str,
    origin: RequestOrigin, limits: ScanLimits, now_ms: u64,
    recency: RecencyMode,                                   // ← 新增，无默认值
) -> Result<Preview, Refusal>;
```

`preview()` 已经收着 `now_ms: u64`（`preview.rs:83`），顺着传下去即可。
**关键是调用者不许直接给时刻**：理由是 `preview()` 里已经有一个时刻在管 HITL 闸
（`check_action(issuer, &request, now_ms)`）。
如果近因规则收第二个时刻，一次预览里就有两个「现在」，它们可以不一致，
而不一致的那一刻正是最难复现的 bug。**一次调用，一个时刻。**

**机械保证**：照 `no_write_api.rs` 的样子，加一张 `CLOCKS` needle 表，对全 crate 生效：

```
"SystemTime::now", "Instant::now", "Utc::now", ".elapsed(", "SystemTime::UNIX_EPOCH.elapsed"
```

注意 needle 不能写成 `UNIX_EPOCH`：`scan.rs:424-428` 与 `DirectorySnapshot::of` 都合法地
用 `duration_since(UNIX_EPOCH)` 把**文件的** mtime 换算成整数，那不是读时钟。
并且照 `the_search_recognises_a_write_when_it_sees_one` 的先例补一个对照用例，
喂它几行真的读时钟的代码，证明这张表认得出来——**没有对照的 needle 表等于没有表**。

### 2.2 层 B —— 量化：floor 到 UTC 日，且是**日历日之差**

```rust
pub const SECONDS_PER_DAY: i64 = 86_400;

/// 一个计划算给哪一个 UTC 日。没有更细的读数可取——更细的读数正是哈希漂移的来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtcDay(i64);

impl UtcDay {
    pub const fn of_unix_seconds(seconds: i64) -> UtcDay {
        UtcDay(seconds.div_euclid(SECONDS_PER_DAY))   // floor，不是截断
    }
    pub const fn of_unix_millis(millis: u64) -> UtcDay {
        UtcDay::of_unix_seconds((millis / 1_000) as i64)
    }
    pub const fn number(self) -> i64 { self.0 }
    /// 从 `earlier` 到 `self` 的整 UTC 日数，钳到非负。
    pub const fn days_since(self, earlier: UtcDay) -> i64 {
        let delta = self.0.saturating_sub(earlier.0);
        if delta < 0 { 0 } else { delta }
    }
}
```

四个决定，逐条给理由：

**B1 —— newtype 而不是 `i64`。** `UtcDay` 没有任何方法能取回秒。这样「判定不小心吃到了更细的时刻」
不是靠纪律，而是**表达不出来**。这是本文唯一一处要求用类型而不是用注释的地方，
因为它保护的正是本文的全部论点。

**B2 —— `div_euclid` 而不是 `/`。** 截断除法会把 1969-12-31 折进 1970-01-01：
`(-1) / 86_400 == 0`，而 `(-1).div_euclid(86_400) == -1`。`soul-algo-tie` 已经踩过并留了钉测
（`tests/as_of_discipline.rs::evidence_from_before_1970_does_not_collapse_into_one_day`），
沿用同一个写法，不要重新发明。

但要说清一件事，免得这条被当成已经解决的问题：**在 fileplan 里，1970 年前的 mtime 今天根本到不了判定。**
`scan.rs:424-428` 写的是 `modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok())`，
而 `duration_since` 对早于纪元的时刻返回 `Err`，被 `.ok()` 吞掉 ⇒ `modified_unix_seconds == None`
⇒ 落进 §3.1 的 `unknown_mtime`。所以 `div_euclid` 在这里是**防御性**的，不是当下必需的。
它仍然该这么写（代价为零，且哪天有人把那个 `.ok()` 换成钳到 0 或换成带符号的换算，
判定就已经是对的了），但**那条 1969 年的钉测在 fileplan 里断言的应该是「它被记成 `unknown_mtime`」，
不是「它被记成一个很老的日子」**——照搬 `soul-algo-tie` 的期望值会写出一条测错东西的测试。

**B3（最关键）—— 量化的是「日历日之差」，不是「经过的秒数再除以 86400」。**
两种写法在自然语言里都叫「几天前」，但对哈希稳定性的后果完全相反：

| 写法 | 定义 | 一天里的边界时刻 |
|---|---|---|
| **滑动**（`soul-algo-tie::age_days` 用的） | `floor((as_of − mtime) / 86400)` | 每个文件一个，落在 `mtime + k·86400` |
| **日历**（本文要的） | `as_of.div_euclid(86400) − mtime.div_euclid(86400)` | **全目录共一个，UTC 零点** |

滑动版的边界随每个文件的 mtime 分布在一天中的任意时刻：一个 500 个散文件的目录，
一天里最多有 500 个不同的瞬间会让 `plan_hash` 变。**那不叫量化，那叫把漂移打散了。**
日历版把所有翻转对齐到同一个瞬间，于是「这个计划算的是哪一天」是一句能说清的话，
而且**一个整数 `UtcDay` 就足以完整描述计划的时间输入**——滑动版做不到这一点，
它需要把原始 as_of 留在身边。

代价要写明：本 crate 与 T4D 从此有**两套「几天前」**。这是有意的分叉，不是疏忽：
T4D 的 as_of 服务的是一次全库重建的分档，没有「用户按哈希批准」这一步；
fileplan 的 as_of 喂给的是**用户要按哈希批准的值**。需求不同，量化就不同。
为免混淆，**名字必须不同**：T4D 叫 `age_days`，本文叫 `days_since` / `UtcDay`，
不要在 fileplan 里出现 `age_days` 这个标识符。

**B4 —— 阈值粒度 = 量化粒度。** 日量化之下 `within_days = 1` 覆盖的真实时长是
`[24h, 48h)` 之间的某个值，取决于 as_of 落在当天的哪个时刻。于是：

- 不许说「最近 24 小时」——那是滑动窗口的措辞，而实现是台阶。
- 也不许说「今天」——G2（全仓无本地时区偏移）说了这是 UTC 的今天，对 UTC+8 用户每天错八小时。
- 可用的句子：**「它的修改时间太新，这次先不碰」**。不给数字，因为给出的任何数字都会比实现更精确。
  预览的旁注里可以说「按 UTC 日计」，那是诚实的技术说明，不是产品承诺。

### 2.3 层 C —— 钉住并重放（这一层才真正把假拒绝清零）

`Preview` 暴露它算给哪一天：

```rust
impl Preview {
    pub fn as_of_day(&self) -> Option<UtcDay>;   // Recency::Off 时为 None
}
```

批准面把这个整数**随批准一起带走**（与 `plan_hash` 并列的旁挂值，**不在被哈希的值里面**，见 §3.1）。
v0.1.1 的执行器拿到批准时，**用批准里的那个 `UtcDay` 重算计划**，再比哈希。

**定理（重放是安全的）。** 用陈旧的 as_of 重算，唯一可能错的方向是
「一个当时被判为『太新』的文件，现在其实已经不新了」——即**少提议一次移动**，不会多提议。
而反方向的担心——「一个当时不新的文件，用户在批准之后又改了它，不该再移」——
**已经被 `directory_snapshot` 覆盖**：用户改了它，mtime 变了（纳秒级），快照哈希变了，
`plan_hash` 变了，执行被正当地拒绝。

这条定理是整个设计的收口：**as_of 重放之所以安全，恰恰因为一个更新的 as_of 唯一能告诉你的事
（这个文件在你批准之后被动过），盘上快照已经告诉你了。**

配套一条**新鲜度上界**，与假拒绝分开：若 `UtcDay::of_unix_millis(now_ms).days_since(pinned) > 1`，
执行器拒绝，理由码是**新的** `PlanTooOld`（`"PLAN_TOO_OLD"`），解释「这个计划是前几天做的，
请重新看一遍」。**绝不复用 `PlanHashMismatch`**：一个是「计划变了」，一个是「计划旧了」，
用同一句话说会让用户学会忽略前者，而前者是 AC-19 的中间那一条。
代价：`ReasonCode` 加一个变体，要同步 `ReasonCode::ALL`（`audit.rs:145`）与它的字符串校验。

三层的分工，一句话各一条：

- **层 A** 让计划**可复现**（同样输入同样输出）。
- **层 B** 让漂移**可预测**（每天一次，全目录同时，能对用户解释）。
- **层 C** 让漂移**不产生假拒绝**（批准和执行看的是同一天）。

只做 A：哈希每秒都可能变，功能不可用。
做 A+B 不做 C：假拒绝概率 = 窗口跨零点的概率。窗口被两分钟令牌 TTL 约束时是
`120 / 86 400 = 1/720 ≈ 0.14%`；窗口无界（今天的样子）时，开着过夜就是 100%。
A+B+C：**0**。

### 2.4 单调性：为什么不需要滞回

**命题。** 固定一个文件的 `mtime_day`，谓词 `P(as_of) = (as_of.days_since(mtime_day) <= K)`
关于 as_of 单调不增（一旦为假，永假）。

**证明。** `days_since` 是 `max(0, as_of_day − mtime_day)`，对 `as_of_day` 单调不减；
`as_of_day = as_of.div_euclid(86400)` 对 as_of 单调不减。复合仍单调不减，
故 `days_since <= K` 单调不增。∎

**推论。**

1. 每个文件在整个生命周期里**最多翻转一次**，方向固定（被抑制 → 不再被抑制）。
2. 让它重新进入抑制集的唯一途径是 mtime 变大——那是一次真实的盘上改动，
   它**本来就**会改 `directory_snapshot`，因而本来就会改哈希。S3 在这条路径上不新增任何东西。
3. **抖不起来，所以不需要滞回。** R1 提滞回是因为默认想着「阈值附近会来回跳」，
   那是**双边**判据（区间、比率、排名）的病；单边的、以单调量为自变量的阈值没有这个病。

顺带钉死一条禁令：**任何把近因写成双边区间的变体（如「距上次修改在 1 到 7 天之间才提议」）
一律否决**，因为它立刻恢复了抖动，而抖动在无状态纯函数里无法用滞回消除。

---

## 3. 哪些东西进哈希：字段级规格

### 3.1 准则（本文提出，请 R3 攻）

> **一个值应当进入被哈希的计划，当且仅当它能在「计划的其余可见内容一字不变」的前提下
> 改变执行的后果。**

按这条准则逐个判：

| 值 | 能在内容不变时改变后果吗 | 判定 |
|---|---|---|
| `executable_in_this_version` | **能**。一个会执行的构建产出的 JSON 其余部分完全一样 | **进**（今天已经进了，且有钉测） |
| `recency.mode` | 不能（它的影响全在 `moves`/`left_alone` 上）。但它是**常量**，不随时间漂 | **进**。准则不强制，但代价恒为零，且它让「用户批准的这份计划是由一个开着近因的构建做的」写在值里 |
| `recency.within_days` | 同上，常量 | **进**。同时它把「用户被哪个数字约束」写进了值里，与 T4D 的 `crossed_threshold` 同一个惯例 |
| `recency.unknown_mtime`（拿不到 mtime 的条目数） | 不能，但它只由 scan 决定，不吃 as_of，**永不因时间漂** | **进**。这是让「元数据缺口」在用户批准的值里可见的唯一位置 |
| **`as_of_day`** | **不能。** 它的全部影响都已经体现在 `moves`/`left_alone` 上 | **不进。** 放进去等于把「跨零点但计划内容一字未变」从无事发生变成一次假拒绝 |

第五行是本文与 R1 最大的分歧，值得再说一遍：
**把 as_of 放进哈希，会让哈希每天必变一次；不放，哈希只在计划真的变了的那些天变。
两者都「诚实」，但前者制造的是用户学不会区分的噪声。**

透明度的诉求（用户应当知道这份计划算的是哪一天）由**视图**满足，不由哈希满足：
`as_of_day` 出现在 `Preview::as_of_day()` 与 `PlanPreview` 里，界面可以显示，
批准时随手带走，执行时重放。它不需要被哈希保护，因为**它伪造不了**：
执行器拿一个不同的 `as_of_day` 去重算，要么算出同样的内容（那它们本来就是同一份计划），
要么算出不同的内容（哈希不匹配，当场拒绝）。**旁挂值自证。**

### 3.2 JSON 片段

关闭时：

```json
"recency": { "mode": "off" }
```

打开时：

```json
"recency": { "mode": "utc_day", "unknown_mtime": 0, "within_days": 1 }
```

嵌套对象是安全的：`serde_json` 未启用 `preserve_order`（`rg preserve_order Cargo.lock` 今天为空），
`Value::Object` 在**每一层**都是 `BTreeMap`，键序处处稳定。

**一次性代价，明说**：加了这个键之后，`recency: off` 的哈希与今天的 v0.1 哈希**不相等**。
考虑过「关时不出这个键」以保住逐字节相等，**否决**：键的有无是一种隐式编码，
读 JSON 的人无法从缺席区分「这个构建没有近因功能」与「有但关着」。
计划不跨版本持久化（本 crate 无 `soul-store` 依赖，令牌两分钟过期），
所以一次性跳变的实际代价是**一条钉测要改数**，仅此而已。

### 3.3 `LeaveReason` 的新变体

追加在**末尾**（`LeaveReason` derive 了 `Ord`，序即声明序，被用作 `left_alone` 排序次键，
`plan.rs:270`；插在中间改的是排序语义）：

```rust
    /// 修改时间落在 as_of 所在的那几个 UTC 日里。
    RecentlyModified,
```

- `as_str()` → `"recently_modified"`（进哈希的稳定词）
- `explanation()` → `"它的修改时间太新，这次先不碰"`

**为什么不叫 `RecentlyActive`**：mtime 不是「用户在用它」的代理——`cp -p`、解压、
`git checkout`、下载器都会保留或重置 mtime，这一条 R1 §6.3 自己列过。
理由名是用户会读到的词，它不许宣称数据支撑不了的东西。

**未来时间戳落在同一个变体里**：`days_since` 钳到 0，所以 mtime 在未来的文件被判为「太新」。
这是保守方向（只收缩移动集合），而且上面那句解释对它**仍然成立**——「太新」在字面上就包含「比现在还新」。
**命名备选**（登记，不推荐）：单开一个 `ClockSkew` 变体。代价是多一个变体、多一句解释，
以及要定义「未来多久算歪」这个新阈值；收益只是措辞更精确。除非真实语料上未来时间戳的比例
高到值得单列，否则不做。

### 3.4 连带改动的爆炸半径（照实说）

| 位置 | 改什么 |
|---|---|
| `soul-fileplan/src/plan.rs` | `build` 加一个参数；新变体 + 两个字符串；`to_json` 加一个键 |
| `soul-fileplan/src/preview.rs` | `preview` 加一个 `RecencyMode` 参数；`Preview` 加 `as_of_day()` |
| `soul-fileplan/tests/*` | 14 处 `soul_fileplan::preview(...)`、2 处 `plan::build(&scan)`（`authorized_scan.rs:280,304`）要加参数 |
| `soulcore/src/commands/fileplan.rs` | `FilePlanSession::preview` 转发；`PlanPreview` 加字段（它带 `#[serde(deny_unknown_fields)]` 且有 round-trip 钉测） |
| `apps/desktop/src/core.ts` | `PlanPreview` 接口加字段；契约测试同步 |
| `soul-policy/src/audit.rs` | 若采纳 §2.3 的新鲜度上界：`ReasonCode` 加一个变体并进 `ALL` |

**命名备选（推荐 R3 讨论）**：与其让 `preview()` 的参数表继续长（今天 6 个），
不如引入 `PreviewOptions { limits, recency }`。眼下改的调用点更多，
但 S1 要一个 `magic` 开关、S2 要一个 `tier` 开关，三条后继各加一个位置参数之后，
这个签名会变成没人读得懂的东西。**这一条与 S3 的正确性无关，纯工程，登记备选不做定论。**

---

## 4. 判定放在五级短路的哪一级

新的短路链（改动只有第 5 级是新的）：

```
1. is_dir?            → depth==1 ⇒ Directory；depth>1 ⇒ 什么都不产出      continue
2. depth > 1?         → AlreadySorted / Nested                            continue
3. folder()==None?    → UnrecognisedKind                                  continue
4. 目标被占?          → DestinationTaken                                  continue
5. 太新?              → RecentlyModified          ← 新增，放在最后        continue
6. 否则               → claimed.push(dest); ProposedMove{...}
```

**为什么放在最后，而不是 R1 §7 说的最前。** R1 按「哪句话最像用户自己的事」排序，
把 `RecentlyActive` 放在了最前面。本轮推翻它，理由正是本文的主题：

> **吃 as_of 的理由排得越靠后，能在午夜改变输出的文件就越少。**

一个既「认不出类别」又「刚改过」的文件，如果先报近因，它在午夜会从
`recently_modified` 变成 `unrecognised_kind`——**哈希变了，而这个文件从头到尾都不会被移动**。
把近因排在最后，这类文件的理由是恒定的，午夜什么都不会发生。
形式化地说：能在午夜翻转的文件集合，恰好是「除了近因之外没有任何其他理由不移动它」的那些文件，
而这个集合是所有排列里**最小的**。

另外三条：

- **只对 `depth == 1` 且有类别的文件跑。** 目录不谈近因（对目录说「你刚改过它」毫无意义，
  而目录的 mtime 会因为里面任何一个文件的增删而变）；`Nested` / `AlreadySorted` 也不跑
  （它们本来就不动）。于是近因判定的次数 ≤ 候选移动数，不是 `scanned_entries`。
- **被近因抑制的文件不占位。** `claimed.push` 必须留在第 6 级里，抑制的文件不得预订目标名。
  （今天 `claimed` 不可达，见 R1 §4.4；这条是为它变得可达的那一天写的。）
- **只收缩，不新增。** 与 S1 的 `KindDisputed` 同型：S3a 不产生任何今天不存在的移动。
  这是它能与 S1、S2 任意组合的原因。

### 4.1 S3c（近因排序与分档）——仍然只在渲染层

只改显示：把移动按新旧分组、给档标签。三条约束：

1. **不进 `to_json`。** `moves` 按 `from_relative` 排序是哈希可复现的理由之一，
   把展示顺序塞进被哈希的列表等于让哈希依赖展示策略。
2. **渲染层必须用计划自己的 `as_of_day`**，不许自己读时钟。否则用户会看到
   「档说是最近改的，理由却说不是」这种自相矛盾——两个时钟一定会打架。
3. **档不是量表。** D22 禁的是给**人**打分与百分位；文件的新旧是客观有序的时间桶，
   与 T4D 的 `Band` 故意不实现 `Ord` 不是同一件事。这个区别要写在代码注释里，
   否则下一个读到「不许排序」的人会把它误用到这里。

---

## 5. 可数性

每次预览新增的整数，全部可数、可上屏、可断言：

| 量 | 从哪来 | 用途 |
|---|---|---|
| `as_of_day` | 一个 `i64` | 界面显示「这个计划算的是哪一天」；批准时带走 |
| `suppressed_by_recency` | `left_alone` 里 `RecentlyModified` 的条数 | H1/H2 的分子 |
| `unknown_mtime` | `modified_unix_seconds == None` 的条目数 | 元数据缺口可见 |
| `within_days` | 常量 | 用户被哪个数字约束 |

成本：**零新增 I/O、零 open、零字节正文**。mtime 已经在 `ScannedEntry` 里（`scan.rs:424-428`）。
判定是每个候选一次整数减法与一次比较，O(F)，相对 §R1 §4.5 那个 O(F×E) 的 `taken.contains`
可以忽略不计。

假拒绝暴露面（§2.3 已算）：不做层 C 时 `TTL / 86 400`；两分钟 TTL 下是 `1/720`；
今天没有 TTL，所以是「开着过夜必中」。做了层 C 是 0。

---

## 6. 与锁的冲突：逐条

| 锁 | S3a/S3c | 说明 |
|---|---|---|
| 不读第三人正文 | ✅ 永不 | mtime 来自目录项，不开文件 |
| 不落库、无遗忘旁路 | ✅ | 本 crate 无 `soul-store` 依赖，`no_write_api.rs` 第二层会红。**做 S3 时别顺手加依赖** |
| 无时钟 | ✅ 加强 | 层 A 顺带补上一张 `CLOCKS` needle 表，这是**净增强** |
| 窗口标题永不采集 | ✅ 无关 | |
| 输出可解释、可纠正 | ✅ | 一句中文理由，且可以关（`RecencyMode::Off`） |
| D22 禁量表 | ✅ | 见 §4.1 第 3 条 |
| 审计无正文 | ✅ 不动 | `Preview::audit()` 今天只报 `items = moves.len()`。**建议不加新计数**：审计的用途是证明发生了什么，不是携带算法诊断；`suppressed_by_recency` 属于给用户看的预览，不属于审计 |
| `no_write_api.rs` 三层 | ✅ 不受影响 | 但见下面这条命名陷阱 |

**命名陷阱（实测）**：`no_write_api.rs::nothing_in_the_public_surface_offers_to_carry_a_plan_out`
对 `src/` 全文做子串搜索，禁止出现 `fn execute` / `fn apply` / `fn perform` / `fn commit` / `fn undo`。
所以近因模块里**不许**出现 `fn apply_recency`。这不是洁癖：那个测试保护的是
「这个 crate 里没有任何东西自称会施行计划」，而它只会做子串匹配。
建议命名 `fn recency_verdict` 或直接内联进 `build`。

---

## 7. 日后怎么证伪：测试清单

### 7.1 稳定性（本文的主张，逐条可杀）

| 测试名 | 断言 | 它能杀死什么 |
|---|---|---|
| `two_previews_in_the_same_utc_day_agree` | 同一目录，as_of 取当天 `00:00:00`、`00:00:01`、`12:00:00`、`23:59:59` 四点，`plan_hash` 与 `plan()` 全等 | 任何在判定里用到了比日更细的时刻的实现 |
| `the_hash_only_moves_at_utc_midnight` | 扫一段 as_of（跨 3 天，每 997 秒取一点），对**每一对相邻样本**断言：两点落在同一 UTC 日 ⇒ 哈希相等。（只能证伪不能证明，但滑动版会在几百个采样点上违反它） | 滑动版 `age_days`（§2.2 B3）——它的边界散落在一天中的任意时刻，这条当场抓住 |
| `crossing_midnight_with_nothing_recent_changes_nothing` | 造一个所有 mtime 都很老的目录，as_of 从 `T−1` 跨到 `T`（T 为零点），`plan_hash` **相等** | **把 `as_of_day` 放进 `to_json` 的实现**（§3.1）。这一条是本文与 R1 分歧的判决测试 |
| `crossing_midnight_when_a_file_ages_out_says_so` | 造一个 mtime 恰好使它在 `T` 那天不再「太新」的文件，跨零点后哈希变、且该文件从 `left_alone` 移到 `moves` | 「稳定」被做过头变成「永不更新」 |
| `an_approval_survives_midnight_because_the_as_of_travels_with_it` | 在 `T−1s` 批准，在 `T+1s` 用批准里的 `as_of_day` 重算 → 哈希相等、不拒绝 | 层 C 缺失（只量化不钉住） |
| `a_plan_from_the_day_before_yesterday_is_refused_as_old_not_as_changed` | 新鲜度上界触发，`ReasonCode` 是 `PlanTooOld` 而**不是** `PlanHashMismatch` | 用同一句话搪塞两种情况 |
| `recency_off_is_todays_plan_plus_one_key` | `Off` 时 `moves`/`left_alone` 与今天逐项相等；`to_json` 与今天的差异**只有** `recency` 这一个键 | 「关着也偷偷改了行为」 |

### 7.2 边界与退化

| 测试名 | 断言 |
|---|---|
| `the_day_number_floors_instead_of_truncating`（单元测） | `of_unix_seconds(0) == 0`、`(−1) == −1`、`(−86_400) == −1`、`(−86_401) == −2`。截断除法会让前两条都等于 0 |
| `a_file_dated_before_1970_arrives_as_unknown_not_as_ancient`（仅 unix） | 造一个纪元前 mtime 的文件，断言它进 `unknown_mtime`，而不是被当成一个很老的文件（§2.2 B2 的实况）。这是**唯一**需要真的去设文件时间的用例 |
| `a_modification_time_in_the_future_is_treated_as_too_new` | mtime > as_of ⇒ `days_since == 0` ⇒ `RecentlyModified`，且解释非空 |
| `a_file_without_a_modification_time_is_counted_not_guessed` | `modified_unix_seconds == None` ⇒ 规则不触发，`unknown_mtime` 计数 +1 |
| `the_reason_a_file_keeps_is_the_one_that_will_still_be_true_tomorrow` | 一个既 `UnrecognisedKind` 又「太新」的文件，理由是 `UnrecognisedKind`（§4 的短路顺序） |
| `a_suppressed_file_does_not_reserve_its_destination` | 被抑制的文件不进 `claimed` |
| `the_clock_needles_recognise_a_clock_when_they_see_one` | `CLOCKS` needle 表的对照用例（§2.1） |

**上面这两张表里，除了标注「仅 unix」的那一条，没有任何一条需要改文件的时间戳。**
「所有文件都很老」用一个很大的 as_of 表达；「这个文件明天就不新了」用
`(mtime_day + within_days + 1) × 86_400` 这个 as_of 表达，mtime 从 `common::walk()`
返回的 `modified_nanos` 里读。这是「as_of 是参数」的一个回报（另一个见 §7.3）：
**时间上的每一种情形都可以通过挑 as_of 构造，不必去动盘。**
反过来说，如果有人写出一条必须 `set_times` 才能测的近因用例，那说明判定又偷偷吃了别的时间源。

### 7.3 一个必须先解决的 fixture 事故（本轮实测发现）

`tests/common/mod.rs::Tree::build()` 用 `std::fs::write` **在测试运行的那一刻**造出全部
fixture 文件，所以每个文件的 mtime 就是**真实的此刻**。而全套验收测试用的是
写死的 `const NOW_MS: u64 = 1_787_529_600_000`（`authorized_scan.rs:16`）。
两个后果，都不是小事：

1. `1_787_529_600 = 86_400 × 20_689`，**恰好是一个 UTC 零点**（2026-08-24T00:00:00Z）。
   这个常量今天无害，S3 一落地它就变成一个**边界值**。边界测试应当显式钉 `NOW_MS − 1` 与 `NOW_MS`，
   而其余验收测试应当**挪离这个边界**，免得日后有人改动量化时，一堆无关测试跟着红或跟着绿，
   谁也说不清是为什么。
2. **`NOW_MS` 与 fixture 文件的 mtime 之间没有任何约定关系。** 它对应 2026-08-24；
   测试真正跑起来的那一刻只会更晚（今天已经是 2026-08-25），于是 fixture 的 mtime 晚于 `NOW_MS`，
   `days_since` 全部钳到 0，S3a 一开就把**四条移动全部吃掉**：
   `an_authorized_directory_previews_a_plan_and_the_disk_does_not_move`、
   `everything_the_plan_leaves_alone_says_why`、
   `the_plan_hash_is_stable_until_the_directory_is_not` 同时红。
   注意这不是「常量选早了」——把常量改晚同样脆：只要它与 mtime 的差落在 `within_days` 之内，
   结论一样。**唯一稳的做法是让 as_of 由 fixture 推出来，而不是写死一个数。**

**修法（不需要新依赖）**：测试的 as_of 由 fixture 自己推出来——
`common::walk()` 已经返回每个条目的 `modified_nanos`，取最大值换成秒、加上 10 天即可。
这样测试**既不读时钟、也不改文件时间**，仍然确定。
这正是「as_of 是参数、不是读数」的直接回报：**一个吃时钟的算法会让这套 fixture 无法测试，
一个吃参数的算法不会。** 备选方案（加 `filetime` dev-dependency 或用 `File::set_times` 设定 mtime）
能用——`WRITES` 只扫 `src/`，不扫 `tests/`——但没必要，且会让 fixture 多一个平台差异面。

### 7.4 值不值得做（沿用 R1 的 H1–H4，补两条）

- **H1（命中率）**：真实目录里 `days_since <= 1` 的散文件占比。R1 说过低命中率不足以否决 S3a
  （它的价值在于避免那一次灾难，成本又近零）——本轮维持这个判断。
- **H2（反噬，新增，本文认为这才是主杀线）**：若 `days_since <= 1` 的散文件占比 **> 50%**，
  S3a 会把预览清空——用户点「整理」，Soul 说「这些我都先不碰」。
  那时该动的是 `within_days`，甚至是整条规则的存废。这个比例只能在真实目录上量，
  fixture 上量不出来（fixture 的文件全是刚造的，比例恒为 100%，见 §7.3）。
- **H3（阈值有没有结构）**：`within_days` 从 1 扫到 30，若计划内容全程不变，
  说明 mtime 在这个目录上没有可用结构 ⇒ 放弃 S3a。扫参数就能做的证伪。
- **H4（mtime 是不是真代理）**：只能离线做一次性研究（本 crate 不依赖也不应依赖 `soul-store`）。
  **S3 在产品内无法自我校准**，这一点是结论，不是日后才发现的缺陷。
- **H5（新增，测层 C 的必要性）**：埋点统计真实使用中「批准到执行」的时间分布。
  若 P99 < 两分钟，层 C 的收益从「必需」降为「便宜的保险」；若分布有长尾（用户开着预览去开会），
  层 C 是必需的。这个分布决定的是层 C 的优先级，不是它的正确性。

---

## 8. 状态词（沿用 C1R3 的词表）

| 项 | 状态 | 前置件 |
|---|---|---|
| **S3a** 近因抑制（层 A+B） | `TESTABLE-NOW` | 零新增 I/O；只需 §7.3 的 fixture 修法 |
| **S3c** 近因分档显示 | `TESTABLE-NOW` | 只在渲染层；必须消费计划自己的 `as_of_day` |
| **层 C** as_of 钉住与重放 | `TESTABLE-AFTER-GAP` | 依赖 v0.1.1 执行面存在；今天可以先把 `as_of_day` 挂到 `Preview` 上，重放无处可测 |
| `PlanTooOld` 理由码 | `TESTABLE-AFTER-GAP` | 同上；且要动 `soul-policy` 的共享枚举 |
| **S3b**（原「单件类别不建文件夹」） | `OUT-OF-SCOPE`（移出 S3，改名 **SUP-K**） | 不吃 as_of，不是近因，是支持度。单独立项单独测；它自己仍是 `TESTABLE-NOW` |
| 双边区间型近因 | `REJECTED` | §2.4：恢复抖动，且无状态下不可用滞回消除 |
| 滑动窗口型 `age_days` | `REJECTED` | §2.2 B3：把漂移打散到一天中的任意时刻 |
| 把 `as_of_day` 放进 `to_json` | `REJECTED` | §3.1：每天必产生一次假拒绝 |

---

## 9. 本文怎么被证伪

事实部分逐条给反例的找法：

- 「`plan::build` 今天不接时钟」→ 签名在 `plan.rs:203`；`rg 'SystemTime::now|Instant::now' crates/soul-fileplan/src/` 今天为空，有命中即为反例。
- 「`directory_snapshot` 含纳秒 mtime」→ `scan.rs:194-199` 的 `as_nanos()`。改成 `as_secs()` 即为反例。
- 「`ScannedEntry` 只留秒」→ `scan.rs:424-428` 的 `as_secs() as i64`。这与上一条的精度差是**有意的**，别去「修」。
- 「`plan.files` 不需要令牌」→ `hitl.rs:74-79` 的 `needs_capability_token`，`PlanFiles` 不在 `matches!` 里。
- 「TTL 是两分钟」→ `hitl.rs:251` 的 `DEFAULT_TOKEN_TTL_MS = 120_000`。
- 「`NOW_MS` 恰在 UTC 零点」→ `1_787_529_600 = 86_400 × 20_689`，余 0。算一次即可。
- 「fixture 文件的 mtime 是此刻」→ `tests/common/mod.rs` 的 `write()` 直接 `std::fs::write`，无 `set_times`。
- 「`WRITES` 表含 `set_modified`/`set_times`，但只扫 `src/`」→ `no_write_api.rs:58-59` 与 `source_files()` 的 `join("src")`。
- 「`fn apply` 是被禁的子串」→ `no_write_api.rs:206-212` 的 `acting` 数组。
- 「`ReasonCode` 里没有「计划太旧」」→ `audit.rs:62-114` 的枚举，逐个看。
- 「`serde_json` 键序稳定」→ `rg preserve_order Cargo.lock` 今天为空。
- 「`LeaveReason` 的序被用作排序次键」→ `plan.rs:270`。
- **「单调性证明」** → 找一个 `mtime` 固定、`as_of` 增大却让 `days_since` 减小的例子即为反例。
  `div_euclid` 单调不减，`max(0, ·)` 单调不减，所以找不到——但如果有人把量化改成滑动版再改回来，
  或者引入本地时区偏移（G2 一旦被补上就有人会想这么做），这个证明**必须重跑**：
  带偏移的「日」在夏令时切换那天不是 86 400 秒，单调性还在，但「每天最多一次翻转」不再成立。

---

## 10. 给 R3 的一条通用件：时钟纪律四条

S3 不会是最后一个想知道「现在几点」的候选（N2 的待办状态规则就想知道）。
把本文的结论抽出来，四条，任何吃时钟的代理层算法都该过一遍：

1. **as_of 是参数，不是读数**，且一次调用只允许有一个时刻。（层 A；G13 的同一条）
2. **量化到阈值需要的最粗粒度**，并且量化的是**日历单位之差**，不是经过时间除以单位——
   后者会把漂移打散到一天中的任意时刻。（层 B）
3. **原始时刻不进被哈希的值**；它的影响必须完全体现在人能读懂的输出上。
   进哈希的只有那些「内容不变时仍能改变后果」的东西。（§3.1）
4. **判定必须对 as_of 单调**，并**排在短路链的最后**——这两条一起把「时间能改变的输出」
   压到理论最小，而且免掉了在无状态纯函数里根本做不出来的滞回。（§2.4、§4）
