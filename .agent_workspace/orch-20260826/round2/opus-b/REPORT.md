MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 2 / opus-b：schema 保真与 D32 棘轮（核实 R1 opus-b 三条）

核验人：Parent Orchestrator Round 2 opus-b 子代理。核于 2026-08-26。

树 = `cursor/project-status-audit-c49c` @ `2ebd087`，其 merge-base 与 `main` @ `a0ec14b` 相同，且 `git diff main...HEAD -- docs crates` 为空——**本树的 `docs/` 与 `crates/` 与 `main` 逐字节相同**，HEAD 上唯一那个提交只收集了 R1 的六份报告。

**纪律遵守声明**：未执行任何 `git checkout` / `commit` / `push`。未改动 `docs/**`（含 `docs/schemas/**`）与 `crates/**` 任何一个字节；`git status --porcelain` 的唯一条目是我自己的 `.agent_workspace/orch-20260826/round2/`。本轮只读 + 只写报告与探针。

---

## 1. 结论摘要

三条都核实了。**两条成立、一条成立但定位错了**，而且三条的严重度评估都需要修正——两条被 R1 高估，一条被显著低估。

| # | R1 opus-b 的说法 | 本轮判定 | 严重度变化 |
|---|---|---|---|
| 1 | `schemas.lock` 无校验器 | **成立但措辞误导**。校验器存在，只是不在这棵树上 | R1 记「中」→ 应记**低**（就 lock 本身而言） |
| 2 | D32 `locked ⟺ user_band` 未双向强制 | **成立，且比 R1 描述的更宽**。R1 测了 3 条反例，完整真值表是 9 格，另有 3 个 R1 未覆盖的矛盾方向 | R1 记「中」→ 应记**高** |
| 3 | `TieScore.last_contact_evidence_id` 缺席 | **成立，但缺席的位置比 R1 说的更靠上游** | R1 记「中」→ 维持**中**，但修法不同 |

驱动这三处修正的是同一个被 R1 漏掉的事实：**`docs/schemas/relationship.schema.json` 在本树与 Goal 1 线（`origin/cursor/goal1-unblock-a073`）字节完全相同（两侧 sha256 均为 `ebf049a9…`），而 Goal 1 的 `soul-store::put_relationship` 在写库之前会拿这份 schema 校验每一条关系。** 所以这份 schema 不是一份「等 Goal 1 合并后才生效的规范」，它是 Goal 1 落库路径上**今天就在跑**的那道门。R1 把 schema 当成「导出/导入面唯一的把关点」，实际它同时是存储面的把关点——这就是为什么第 2 条要升级、第 1 条要降级。

另有一条与三条主张无关、但影响本轮可核性的发现：**R1 opus-b 报告 §7 声称留档的五份证据文件，在被收集进本树时全部丢失**（详见第 6 节）。

---

## 2. 方法

`probe/verify_r2_opus_b.py`（本目录），与 R1 的 `schema_probe.py` **独立重写**，三处方法学差异是刻意的：

1. **校验整份文档而非抽出的子模式。** R1 用 `relationship.schema.json["properties"]["tie_strength"]` 单独建校验器。我校验完整的 relationship 实例，让顶层 `additionalProperties:false` 与 `required` 一起参与——这才能回答「某字段在 `tie_strength` 里没落点，那顶层有没有」。探针内有 `assert` 强制两种建法对同一个 `tie_strength` 给出同一判定，全程未触发，所以 R1 的子模式建法本身没有引入偏差。
2. **枚举完整真值表而非挑反例。** D32 那部分枚举 (`locked_by_user` ∈ {缺席, false, true}) × (`user_band` ∈ {缺席, null, "strong"}) 全部 9 格。
3. **`format` 单独成组。** R1 没挂 `FormatChecker`。Draft 2020-12 里 `format` 默认只是注解不是断言，所以「时间戳」字段实际不被校验；我把挂与不挂各跑一遍，差值本身作为一条记录。

运行环境：Python 3 / `jsonschema` 4.26.0 / `referencing`，`_defs` 经 registry 解析，无外网。

```bash
python3 .agent_workspace/orch-20260826/round2/opus-b/probe/verify_r2_opus_b.py
```

全量输出留档在 `probe/verify_r2_opus_b.out`。**退出码 1 是预期的**：7 条「不符」全部是本报告要报告的缺口本身（D02/D06/D07/D08/E01/E02/E03），不是探针故障。除这 7 条外，49/56 条判定与登记的预期一致。

一处工具链注意：`jsonschema` 的 `FormatChecker` 默认**不认识** `date-time`，除非另装 `rfc3339-validator`。未装时它静默跳过，看起来像「校验通过」。我装上后重跑，非法时间戳才被拒（G00/G02）。任何后续做 schema 校验的人都会踩这个坑。

---

## 3. 主张一：`schemas.lock.json` 无校验器

**判定：就本树而言成立，但「无校验器」这个说法会把读者带偏，应改写。**

### 3.1 R1 说对的部分

本树确实没有任何一道门（我把 R1 的检查面扩大了一圈）：

| 检查 | 结果 |
|---|---|
| `crates/xtask` | 不存在（workspace members 只有 `soul-algo-tie`、`soul-algo-trait`） |
| `.github/` | 不存在 |
| `Makefile` / `justfile` / `Taskfile.yml` / `noxfile.py` | 均不存在 |
| 非 sample 的 git hook | 无 |
| 任何 crate 读 `docs/schemas` | 无（`include_str!` 在两个 crate 里只用来读自己的 `.rs` 源文件） |
| 11 份 schema 的 sha256 vs lock | **11/11 吻合**（独立重算，探针 A 组） |
| 双向覆盖：磁盘每份都在 lock 里 / lock 每条都在磁盘上 | 两个方向的差集都为空 |

R1 手算 11 份吻合这点我复现了，并补了 R1 没做的双向覆盖检查——lock 没有漏钉任何一份，也没有钉住任何一份不存在的文件。

### 3.2 R1 漏掉的部分：校验器存在，在 Goal 1 线上，而且是两个

`crates/xtask/src/schema_freeze.rs` 存在于 88 个远端分支上，含 Goal 1 吸收线 `origin/cursor/goal1-unblock-a073`。链路完整、逐段可查：

```
.github/workflows/ci.yml  →  just ci
justfile: ci: lint schema e0 denylist fixtures-verify test smoke-lint sbom ui-lint ui-test
justfile: schema:  cargo run -q -p xtask -- schema-freeze --check
xtask/src/main.rs: Some("schema-freeze") => run_schema_freeze(&root, ...)
```

`xtask` 的 `all` 子命令也把 `schema-freeze --check` 包在里面。所以 lock 门**是真的**，只是不在这棵树上。

更值得记的是 R1 完全没提到的**第二种**校验器：`crates/soul-schema/src/validate.rs`。它是一个 Draft 2020-12 实例校验器，用宏把 11 份 schema 逐份 `include_str!` 编译进二进制（`concat!("../../../docs/schemas/", $file)`），并用一个 `LocalRetriever` 从内嵌文本回答所有 `$ref`，「never touches the filesystem and never opens a socket」，`jsonschema` 的 `resolve-http` feature 关闭。

而且它挂在写库路径上：

```rust
fn put_relationship(&mut self, relationship: SoulRelationship) -> StoreResult<Uuid> {
    self.check(SchemaId::Relationship, &relationship)?;
```

`include_str!` 这个写法还有一个副作用值得父代理知道：schema 字节被编译进产物，所以在 Goal 1 上**改错一个字节，红的是编译产物而不是文档检查**。

### 3.3 修正后的说法

> `schemas.lock.json` 的校验器（`xtask schema-freeze --check`）与实例校验器（`soul-schema`）都存在且都接进了 Goal 1 的 CI，但**都不在本树**。本树的 11/11 吻合靠的是没人动过文件。

严重度我建议从 R1 的「中」降到「低」：本树 `docs/` 与 `main` 逐字节相同、无人在此改 schema，而门在 schema 真正被消费的那条线上是通的。R1 说的「有锁无匙」在字面上对，但「匙」只是不在手边，不是不存在。

**但这条降级换来的是第 4 节的升级**：既然 `relationship.schema.json` 两侧字节相同，且 Goal 1 每次 `put_relationship` 都拿它校验，那么下一节所有 schema 层的放行就不是纸面问题，而是**落库路径上真实的放行**。

---

## 4. 主张二：D32 `locked ⟺ user_band` 未双向强制

**判定：成立。而且 R1 只看到了一半——完整真值表里 schema 与 D32 有 4 格分歧，不是 3 格，且分歧是双向的。**

D32 原文（`docs/DECISIONS.md:43`）：「写。`locked ⟺ user_band.is_some()`；重建边 `machine_band` 常为 `Some`」。schema 侧的全部执行力只有一行：

```106:116:docs/schemas/relationship.schema.json
      "dependentRequired": {
        "direct_out_count": ["algorithm_id"],
        ...
        "user_band": ["locked_by_user"]
      },
```

`dependentRequired` 只要求「出现 `user_band` 这个**键**就必须同时出现 `locked_by_user` 这个**键**」。它不看任何一方的**值**，也不管反方向。

### 4.1 完整 3×3 真值表（探针 D 组）

「D32 判定」列按 D32 语义推出：键缺席一律读作「未锁 / 无锁定档」，`null` 读作 `None`。

| # | `locked_by_user` | `user_band` | D32 | schema 实测 | |
|---|---|---|---|---|---|
| D01 | 缺席 | 缺席 | 允许 | 合法 | 一致 |
| D02 | 缺席 | `null` | 允许 | **拒绝** | ← schema 比 D32 **更严** |
| D03 | 缺席 | `"strong"` | 禁止 | 拒绝 | 一致（`dependentRequired` 唯一拦住的一格） |
| D04 | `false` | 缺席 | 允许 | 合法 | 一致 |
| D05 | `false` | `null` | 允许 | 合法 | 一致 |
| D06 | `false` | `"strong"` | 禁止 | **合法** | ← 放行 |
| D07 | `true` | 缺席 | 禁止 | **合法** | ← 放行 |
| D08 | `true` | `null` | 禁止 | **合法** | ← 放行 |
| D09 | `true` | `"strong"` | 允许 | 合法 | 一致 |

R1 的 P13 / P23 / P24 分别对应 D07 / D06 / D08，三条我全部复现，判定一致。**R1 漏掉的是 D02**：schema 在这一格反过来比 D32 更严，而且它与 schema **自己的 `$comment`** 直接冲突——

```89:90:docs/schemas/relationship.schema.json
        "user_band": {
          "$comment": "用户锁定档。D32：locked ⟺ user_band 有值。未锁时省略或 null。",
```

注释说「未锁时省略**或 null**」，但只写 `"user_band": null` 而不带 `locked_by_user` 会被拒：

```
{'user_band': None} -> ["'locked_by_user' is a dependency of 'user_band'"]
```

R1 没测到是因为它的 P15 顺手带上了 `locked_by_user: false`，正好绕开了触发条件。这一格今天伤不到 Goal 1（见 4.3，它的 serde 从不吐 `null`），但它是留给任何**其他**生产者的一个陷阱：一份手写导出、或一个没配 `skip_serializing_if` 的序列化器，会写出一份符合注释、却被 schema 拒收的文档。

净效果：D32 禁止的 4 格里 schema 只拦住 1 格，**放行 3 格**；D32 允许的 5 格里 schema **误拒 1 格**。

### 4.2 R1 未覆盖的三个方向（探针 E 组）

D32 与 D48 不只约束「锁没锁」，还约束「生效档是谁」。D48（`docs/DECISIONS.md:59`）：「生效档 = 用户锁定时的档，机器档另存」。这三条 R1 一条都没测，全部放行：

| # | 实例 | 应当 | 实测 |
|---|---|---|---|
| E01 | `band:"weak"` + `user_band:"strong"` + `locked_by_user:true` | 拒绝（D48：生效档=锁定档） | **合法** |
| E02 | `band:"weak"` + `machine_band:"strong"` + `locked_by_user:false` | 拒绝（未锁边生效档=机器档） | **合法** |
| E03 | 完整 T4D 边，`locked_by_user:false`，**完全不写** `machine_band` | 拒绝（D32：「未锁边也写」） | **合法** |

E01 是里面最难看的一条：一条边可以同时宣称「用户把我锁成了强」和「我的生效档是弱」，而这正是 D48 那句话要防的事。E03 说明 D32 的**前半句**（「未锁边是否写 `machine_band`」的拍板结论就是「写」）在 schema 层同样零执行力——`machine_band` 不在 `if/then` 的 `required` 里，声明了 `algorithm_id` 的重建边照样可以一个字不写。

两条 `none` 相关的收紧倒是成立：`machine_band:"none"`（E04）与 `user_band:"none"`（E05）都被 `allOf` 里的 `enum` 拦下，与 `band` 一致。

### 4.3 Goal 1 侧：不变式靠两条写路径维持，不靠类型也不靠门

D32 的依据列写着「Goal 1 吸收线已钉测试」。我去查了，这句话**部分为真，但钉住的不是不变式**。

`crates/soul-graph/src/model.rs`：

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub locked_by_user: Option<bool>,
/// The band the user chose. `Some` exactly when the edge is locked.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub user_band: Option<SupportedBand>,

impl TieStrength {
    pub fn is_locked_by_user(&self) -> bool {
        self.locked_by_user == Some(true) && self.user_band.is_some()
    }
}
```

`crates/soul-graph/src/correct.rs` 的两条写路径确实成对读写，`graph_correction.rs` 也成对断言（`correct_tie` 写 `Some(true)` + `Some(band)`；`release_tie` 写 `None` + `None`；测试在 171/180、229、273–275、427–428 行钉住）。所以**沿这两条路径，不变式成立**。

但：

1. **类型让非法态可表示。** `Option<bool> × Option<SupportedBand>` 是 9 个状态，其中 3 个违反 D32。没有任何构造器、`TryFrom` 或断言把它们挡在外面。
2. **没有任何测试把不变式当不变式来钉。** 现有断言钉的是「这两条 API 调用之后字段长这样」，不是「任何一条 `TieStrength` 都满足 `locked_by_user == Some(true) ⟺ user_band.is_some()`」。
3. **`is_locked_by_user()` 的合取式会静默吞掉用户的纠正。** 一条 `locked_by_user: Some(true), user_band: None`（即 D07/D08 的形状）会被读成**未锁**，于是机器档接管、A2 照常渲染冻结的 P5 归档句。用户明明纠正过，系统安静地当作没发生。这不是「schema 少一条约束」，这是**数据面已经能表达的一个静默降级**，而 `put_relationship` 那道门放它过。
4. **`put_relationship` 是公开的存储 API。** 导入、迁移、以及任何绕开 `correct_tie` 的写入都只受 schema 约束，而 schema 放行 D06/D07/D08。

顺带澄清 D02 对 Goal 1 的影响：`skip_serializing_if = "Option::is_none"` 意味着 `user_band: None` 序列化为**键缺席**，永远不会写成 `null`。所以 Goal 1 自己不会撞上 D02 那格，它是留给别的生产者的坑。

### 4.4 schema 测试面：D59 棘轮与 D32 三字段在 Goal 1 语料里零覆盖

Goal 1 的 `crates/soul-schema/tests/schema_wiring.rs` 有三个通用测试（每份 schema 至少接受一份 valid fixture、每份 invalid fixture 都被拒、四条点名的负例）。relationship 的 fixture 一共三份：

| fixture | 内容 |
|---|---|
| `valid/relationship/colleague_edge.json` | `tie_strength` 是 `{ "band": "moderate", "last_contact_utc": ... }` ——**T4D 之前的宽松形态** |
| `invalid/relationship/contact_id_not_uuid7.json` | 与 `tie_strength` 无关 |
| `invalid/relationship/no_evidence.json` | 与 `tie_strength` 无关 |

`named_negative_cases_are_present_and_rejected` 点名的四条是 event / export-manifest / inference / audit，没有 relationship。

也就是说：**D59 的整个棘轮、以及 `machine_band` / `user_band` / `locked_by_user` 三个字段，在 Goal 1 的 schema fixture 语料里一份样例都没有**。唯一那份 valid fixture 连 `algorithm_id` 都不写，所以 `if/then` 那 15 字段整包必填的分支从未被任何 fixture 走过。R1 的 24 条探针（和我的 56 条）是这套约束**目前唯一**的行为证据，而探针不在 CI 里。

### 4.5 严重度：建议从「中」升到「高」

R1 把 S5 记为「中」，理由是「D32 的备注写「Goal 1 吸收线已钉测试」，所以这条在 Goal 1 侧可能有守卫」。这个理由现在站不住：守卫只覆盖两条写路径，而 schema 是**存储边界上真正会跑的那道门**，它放行了 D32 禁止的 3 格中的全部 3 格，其中 D07/D08 会让 `is_locked_by_user()` 静默把用户锁定读成未锁。这条同时命中「与已拍板决议直接冲突」和「失效模式是静默的」两个特征。

---

## 5. 主张三：`TieScore.last_contact_evidence_id` 缺席

**判定：成立。但 R1 把缺席定位在 schema 侧，实际上判档侧压根不产生这个值——缺席比 R1 说的更靠上游，修法也因此不同。**

### 5.1 先澄清一个歧义：有两个 `TieScore`

这一点 R1 全程没提，而它是理解本条的前提。仓库里有两个同名、不同定义、互不引用的结构体：

| | `soul-algo-trait::a2::TieScore` | `soul-algo-tie::types::TieScore` |
|---|---|---|
| 角色 | A2 渲染器的**输入**（消费方） | T4D/T4 判档的**输出**（生产方） |
| 整数宽度 | `u32` | `u64` |
| 分列 | `direct_count: Option<u32>` 字段 | `direct_count()` 方法（`direct_out + direct_in`） |
| 方向计数 | `outgoing` / `incoming` | `outgoing_count` / `incoming_count` |
| 证据字段 | `evidence_ids: Vec<u64>` + `last_contact_evidence_id: Option<u64>` | **一个都没有** |

两个 crate 互不依赖（`soul-algo-trait` 的 `[dependencies]` 为空），仓库里**没有任何适配器**在两者之间转换——`impl` 只有各自的 `Default` 与固有方法，没有 `From`。

R1 的表 B 用的是 a2 侧那个，本身没错，但没说清有两个，导致「缺席」读起来像是「这个字段不存在」。

### 5.2 逐项核实

**(a) 字段在 a2 侧存在。** `crates/soul-algo-trait/src/a2.rs:147-149`：

```147:149:crates/soul-algo-trait/src/a2.rs
    /// The row the most recent interaction came from, when it is known
    /// separately. Falls back to [`TieScore::evidence_ids`].
    pub last_contact_evidence_id: Option<u64>,
```

它被 `recency_evidence()` 消费，这就是「近因句只引最近一条证据」那条承诺的载体：

```443:447:crates/soul-algo-trait/src/a2.rs
fn recency_evidence(score: &TieScore) -> Vec<u64> {
    match score.last_contact_evidence_id {
        Some(id) => vec![id],
        None => score.evidence_ids.clone(),
    }
```

**(b) schema 侧确实无落点，顶层也没有。** 探针 F13 / F90：写进 `tie_strength` 被 `additionalProperties:false` 拒；写进 relationship 顶层同样被拒（顶层也是 `additionalProperties:false`，属性表固定）。F92 记录：`tie_strength.properties` 里含 `evidence` 字样的键，**零个**。顶层只有一个关系级 `evidence_ids`（`minItems:1`，必填，F91）——它是「这条边引了哪些证据」，回答不了「最近那次是哪一条」。R1 的 P11 结论成立。

**(c) R1 漏掉的上游缺席。** 我扫了整个 `crates/soul-algo-tie/src`，`evidence` 只出现在**注释和一个测试函数名**里，**没有任何一个字段**。也就是说 `last_contact_evidence_id` 不是「判档算出来了但存不下」，而是**判档从来没算过它**。今天它有值的地方只有 `soul-algo-trait` 的 `fixtures.rs` 与测试——全部是手填的常量。

所以这条真实的形状是：**A2 有一个承诺（近因句只引最近一条证据），它依赖一个上游从不生产、存储也存不下的字段。** 今天 CI 全绿，是因为 fixture 替上游把这个值填上了。R1 说「重建后该承诺只能降级为『引全部 `evidence_ids`』」——这个结论对，但已经不是「重建后」的事，而是**今天在 fixture 之外就已经是这样**。

### 5.3 a2::TieScore 的完整落库保真度（探针 F 组）

既然要谈保真，把 13 个字段全过一遍，而不只看这一个：

| a2 字段 | 按原名写进 `tie_strength` | 说明 |
|---|---|---|
| `band` | 合法 | |
| `interaction_count` | 合法 | |
| `active_day_count` | 合法 | |
| `conversation_count` | 合法 | |
| `outgoing` | 拒绝 | schema 叫 `outgoing_count` |
| `incoming` | 拒绝 | schema 叫 `incoming_count` |
| `direct_count` | 拒绝 | schema 只有 `direct_out_count`/`direct_in_count`，需求和 |
| `group_count` | 拒绝 | 同上，D33 只写了群聊侧口径 |
| `any_direct` | 拒绝 | 只能由 `direct_out + direct_in > 0` 导出 |
| `last_contact_unix` | 拒绝 | schema 是 `last_contact_utc`（RFC3339） |
| `as_of_unix` | 拒绝 | schema 是 `as_of_utc`（RFC3339） |
| `evidence_ids` | 拒绝 | 只在关系顶层，且是 uuid7 不是 u64 |
| `last_contact_evidence_id` | 拒绝 | **本条主张** |

**13 个字段里只有 4 个能原样落库，9 个不能**，而 `additionalProperties:false` 意味着每一个「不能」都是硬拒绝而不是被忽略。这 9 条里 8 条是命名/表示/求和的适配问题（有解，只是无处成文），只有 `last_contact_evidence_id` 是**语义上无处安放**。

严重度维持 R1 的「中」，但修法不同：R1 建议「要不要进 schema」是个二选一；实际要先决定的是**判档侧要不要开始产生证据 id**。schema 加字段而上游不产生它，加了也是空的。

---

## 6. 本轮附带发现（与三条主张无关，但影响可核性）

### 6.1 R1 opus-b 声称留档的五份证据文件全部丢失

R1 报告 §7 结尾写：

> 本目录留档：`schema_probe.py` / `schema_probe.out` / `copy_drift_probe.rs` / `copy_drift_probe.out` / `cargo_test.out`。

`.agent_workspace/orch-20260826/round1/opus-b/` 在本树只有 `REPORT.md` 一个文件。查 git：五份文件确实存在过，在 `e640b62`（分支 `origin/cursor/round1-opus-b-trait-schema-audit-ab42`），但把 R1 报告收集到本树的那个提交 `2ebd087`「docs(audit): collect Round 1 subagent reports onto orchestrator branch」**只带了 `REPORT.md`**。

我从 `e640b62` 取回了 `schema_probe.py` 与 `schema_probe.out` 做交叉核对（R1 的 24 条我全部复现，判定无一分歧），所以 R1 的结论没有问题。但**收集动作本身丢证据**这件事值得父代理知道：报告里所有「实测」都变成了不可复核的断言，下一轮要么重做（我这轮就是重做），要么只能采信。六个 R1 子目录**全部**只有 `REPORT.md`，所以这不是 opus-b 一家的问题，是收集脚本的问题。

**建议**：收集时用目录级复制而不是只取 `REPORT.md`；或者在收集提交里注明证据留在哪个分支的哪个 commit。

### 6.2 `format: date-time` 在两侧都不是断言（探针 G 组）

`_defs.schema.json` 的 `timestamp` 定义是 `{"type": "string", "format": "date-time"}`。JSON Schema 2020-12 规定 `format` **默认只是注解**。实测（G01）：`as_of_utc: "香蕉"`、`first_contact_utc: "2025-13-45"`、`last_contact_utc: ""` 三个一起塞进一条完整 T4D 边，**校验通过**。

对照组：`uuid7` 用的是 `pattern`，`pattern` 一直是断言，非法 uuid 照拒（G03）。所以这不是校验器没配好，是 `_defs` 里两个定义的执行力本来就不一样——`sha256` 与 `uuid7` 走 `pattern`（真拦），`timestamp` 走 `format`（不拦）。

Goal 1 的 `soul-schema` 用的是 Rust 的 `jsonschema` crate，其 `format` 断言行为需要单独确认（我没在本树核，因为跑不了 Goal 1 的 crate）。**这条建议记为待核项而不是缺陷**，但如果 Goal 1 侧也是注解语义，那么 `as_of` 纪律（「一次 rebuild 全库一个值」）在存储层就完全没有格式兜底。

### 6.3 顶层保真两条（探针 H 组，复核 R1 §4.4 的记录）

- `egress_scope` 是 `const: "local_only"` 但**不在** `required` 里，省略即通过（H01/H02）。R1 已记，我复现。第三方关系数据的 local_only 承诺在这一层是可选的。
- `types` 仍是裸 `array`，`[{"任意":"对象"}, 42, null]` 照过（H05）。R1 已记为 D58 残留面，我复现。
- `evidence_ids: []` 被 `minItems:1` 拒（H04）——这条是好的，「无证据不落库」在 schema 层真的可执行。

---

## 7. 探针结果索引

`probe/verify_r2_opus_b.out`，56 条判定 + 13 条无预期的事实记录。

| 组 | 内容 | 结果 |
|---|---|---|
| A | lock 完整性 + 双向覆盖 | 14/14 一致（11 份 sha256 吻合，两个方向差集为空） |
| C | D59 棘轮复核（独立重写 R1 的结论） | 16/16 一致，与 R1 无分歧 |
| D | D32 完整 3×3 真值表 | 5/9 一致，**4 格分歧**（D02 更严，D06/D07/D08 放行） |
| E | D48 生效档 / `machine_band` 义务 | 2/5 一致，**3 条放行**（E01/E02/E03） |
| F | a2::TieScore 13 字段落库保真 | 4 个可落库，9 个被拒；顶层同样无 `last_contact_evidence_id` 落点 |
| G | `format` 断言语义 | 4/4 一致（`date-time` 默认不拦，`pattern` 拦） |
| H | 顶层保真 | 5/5 一致 |

---

## 8. 给父代理的建议（本轮不执行，仅提请裁决）

按「能不能不改冻结字节就修」排序。**注意 D58 尾句的约束**：「进一步收紧 `tie_strength` 须与 Goal 1 `schemas.lock.json` 同批重算」，所以下面凡是动 schema 字节的都必须与 Goal 1 同批，不能在计划线单独做。

1. **【不动字节，可立即做】给 `TieStrength` 补一条不变式测试。** 在 Goal 1 侧断言「任何从 store 读回的 `TieStrength` 都满足 `locked_by_user == Some(true) ⟺ user_band.is_some()`」，并对 `correct_tie` / `release_tie` 之外的写入路径做一遍。这不需要动 schema，也不需要新决议——D32 已经拍过了，缺的只是把它钉住。

2. **【不动字节，可立即做】裁决 `is_locked_by_user()` 的合取式。** 现在 `Some(true) + None` 被静默读成未锁。要么改成遇到半锁状态时报错/告警，要么在 `TieStrength` 上加一个构造器让半锁态无法被建出来。静默是这里最坏的选项，因为丢的是用户的纠正。

3. **【动字节，须与 Goal 1 同批】补 D32/D48 的 schema 层双条件。** 用 `if/then` 表达三件事：`locked_by_user: true ⇒ required[user_band]` 且 `user_band` 非 null；`user_band` 非 null ⇒ `locked_by_user: const true`；锁定时 `band` 必须等于 `user_band`（E01）。同时考虑放宽 D02 那格（把 `dependentRequired` 换成看值而不是看键），让 schema 与它自己的 `$comment`「未锁时省略或 null」对上。

4. **【先裁决，再谈 schema】`last_contact_evidence_id` 的归属。** 顺序不能反：先定 `soul-algo-tie` 的判档侧要不要开始携带证据 id。不携带的话，A2 的「近因句只引最近一条证据」这条承诺应当**降级并写进 COPY_ZH**，而不是靠 fixture 维持一个上游不存在的能力；携带的话，才轮到给 `tie_strength` 加字段（并同批重算 lock）。

5. **【低成本，高回报】给 relationship 补 fixture。** Goal 1 的 fixture 语料里 D59 棘轮与 D32 三字段零覆盖，唯一那份 valid fixture 还是 T4D 之前的形态。把本报告 C/D/E 三组里已经写好的实例转成 `fixtures/schemas/valid|invalid/relationship/` 下的文件，`schema_wiring.rs` 的两个通用测试就会自动覆盖它们——这是让探针结论进 CI 的最短路径，且不动任何冻结字节。

6. **【流程】修收集脚本。** 6.1：六个 R1 子目录全部只剩 `REPORT.md`，证据在收集时被丢掉了。

7. **【待核】确认 Goal 1 的 `jsonschema` crate 对 `format` 的断言语义**（6.2）。若同为注解语义，`as_of` 纪律在存储层无格式兜底。

---

## 9. 复现

```bash
# 三条主张的全部实测
python3 .agent_workspace/orch-20260826/round2/opus-b/probe/verify_r2_opus_b.py
# 退出码 1 是预期的：7 条「不符」即第 4 节报告的缺口本身
# 注意：date-time 的 format 断言需 pip install rfc3339-validator，否则静默跳过

# lock 完整性（独立于探针）
cd docs/schemas && for f in *.schema.json; do sha256sum "$f"; done

# 本树无门
ls /workspace/crates                      # 只有两个算法 crate
ls -a /workspace | grep -iE 'github|ci'   # 空
ls /workspace/.git/hooks | grep -v sample # 空

# 校验器在 Goal 1 线上
G=origin/cursor/goal1-unblock-a073
git show $G:justfile | grep -A2 '^schema:'
git show $G:crates/soul-store/src/store.rs | grep -A2 'fn put_relationship'
git show $G:docs/schemas/relationship.schema.json | sha256sum   # 与本树相同

# 取回 R1 丢失的证据
git show e640b62:.agent_workspace/orch-20260826/round1/opus-b/schema_probe.py
```
