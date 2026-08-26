MODEL_SLUG: claude-opus-5-thinking-high-fast

# Round 1 / opus-a：`crates/soul-algo-tie` 核心实现审计

审计对象：`crates/soul-algo-tie`（v0.3.0）
对照基准：`docs/algorithms/DECISION.md`（ALGO_FROZEN）、`docs/algorithms/COPY_ZH.md`（冻结话术）、旁证 `docs/algorithms/REJECTED.md`
分支：`cursor/audit-soul-algo-tie-r1-opus-a-1faf`（**未改动任何生产逻辑，仅新增本报告**）
基线提交：`main` 线上 `cursor/project-status-audit-c49c` 的工作树

## 0. 结论摘要

**判档层（band 算术）与 DECISION 完全一致，没有找到任何一处判档偏差。** T4D 的三道门确实全部改在一对一计数上，降档 180/360 是闭区间，as_of 全程由调用方传入，crate 内不存在墙钟调用，`forbid(unsafe_code)` 双重设防，零依赖，墓碑齐备。135 个测试全绿，`cargo fmt --check` 与 `cargo clippy -D warnings` 干净，`--example matrix` 打印的消融矩阵与 DECISION §1 的裁决表逐格吻合。

**问题全部集中在「渲染面」与「跨 crate 单源性」两处，且都不改变任何 band：**

1. **DECISION §3 要求的常量单点定义在跨 crate 层面已经破了**——`DORMANT_NOTE_DAYS` 在 soul-algo-tie 里根本不存在，A2 那半边在 `soul-algo-trait/src/a2.rs:91` 独立写了第二个 `180` 字面量，两个 crate 之间零依赖，物理上无法共享常量。
2. **COPY_ZH 冻结的中文模板在代码里基本没有落地**——透明句、收尾句、`{截止日期}` 三个 COPY_ZH 全局强制项在整个 `crates/` 树里一次都没出现；档位词有三套写法（COPY_ZH 的「强/中等/弱」、tie crate 的「强联系」、trait crate 的「中」）。DECISION §6.4 点名的「中文模板绑定测试」这项合并义务尚未开工。
3. **实现里存在一条 COPY_ZH 未收录的新增句**（一对一日期句），它引入了 REJECTED.md §附注在作废 direct-only 时钟时点名批评的「第二个日期概念」。判档时钟没变（仍读任一场地，合规），但这条句子是绕过 DECISION §6.4「模板改动先改 COPY_ZH.md 再改代码」加进去的。

以下逐项展开。

---

## 1. 实现对照表

### 1.1 T4D 直连门（direct only）

DECISION §3 冻结定义：`Strong iff direct 双向都发过 ∧ direct 次数 ≥ 10 ∧ direct 自然日 ≥ 3`；`Moderate iff direct 双向都发过 ∧ direct 次数 ≥ 3`；否则 Weak。

| DECISION 条款 | 代码位置 | 判定 |
|---|---|---|
| 三道门全部读一对一计数 | `src/t4d.rs:72-78` `T4D::observed` 只取 `is_direct_reciprocal()` / `direct_count()` / `direct_active_day_count()` | ✅ 一致 |
| 阶梯本体与 T4 共用，不得分裂 | `src/gate.rs:46-58` `gate::band` 为唯一阶梯，T4 与 T4D 只在「喂什么数」上分歧 | ✅ 一致，且比要求更强 |
| T4D 无 `GROUP_ONLY_CEILING` | `src/t4d.rs:84-86` `counts_band` 不做任何 capping；`GROUP_ONLY_CEILING` 只被 `src/t4.rs:69,93` 引用 | ✅ 一致 |
| 群聊行「保留展示与近因，不参与判档」 | 展示：`src/types.rs:524-539` `zh_counts_split` 恒列群聊数；近因：`src/types.rs:445-451` `silent_days` 读 `last_contact`（任一场地） | ✅ 一致 |
| 决胜夹具 `group_heavy_plus_one_direct_each_way` 非 Strong | `src/testing/mod.rs:329-339`（群聊互惠 30 次/10 天 + 每方向各 1 条一对一）→ T4D **Weak**，T4 **Strong** | ✅ 与 DECISION §1 表格逐格吻合 |
| 锚测试 `lilei_12` 保全 Strong | `src/testing/mod.rs:167-174` → T4/T4D 均 Strong | ✅ 一致 |
| §4.1 仅群聊者落 Weak（已知代价） | `tests/ablation.rs:118` `group_only_50` 观测行 `(T0=S, T4=M, T4D=W)` | ✅ 代价如实入档，未静默修补 |
| §4.1 自愈路径 `group_heavy_plus_three_directs` → Moderate | `src/testing/mod.rs:346-357`；`tests/ablation.rs:131` | ✅ 一致 |

补强证据：`tests/direct_gate.rs` 用固定种子 LCG 生成 4000 例，证明的是**性质而非样例**——群聊行删光后 T4D 的 counts band 不变（`group_rows_cannot_move_the_counts_stage_of_the_default_rule`）、T4D 的档位恒不高于 T4（`t4d_never_says_more_than_t4`）、任何 Strong 背后必有 ≥10 条双向一对一横跨 ≥3 天（`every_strong_band_from_the_default_rule_is_backed_by_private_traffic`）。另有一条刻意构造的对抗样本：一年内每天 3 条群消息共 1095 条 + 9 条一对一 → Moderate，补第 10 条一对一才 Strong（`no_amount_of_group_traffic_reaches_strong_without_ten_private_exchanges`）。

### 1.2 降档闭区间 ≥180 / ≥360

| DECISION 条款 | 代码位置 | 判定 |
|---|---|---|
| `DEMOTE_ONE_BAND_DAYS = 180`，闭区间 `>=`，任何 `>180` 写法作废 | `src/recency.rs:32` `silent_days >= DEMOTE_AFTER_SILENT_DAYS` | ✅ 闭区间 |
| `FORCE_WEAK_DAYS = 360`，闭区间 | `src/recency.rs:30` `silent_days >= WEAK_AFTER_SILENT_DAYS` | ✅ 闭区间 |
| 沉寂天数 = `max(0, as_of − 任一场地最后往来)/86400`，整数向下取整 | `src/types.rs:262-269` `age_days`（`saturating_sub` + 负值钳零 + 整除） | ✅ 一致 |
| 空观测 → Weak，不进时间运算 | `src/types.rs:445-451`：`is_empty()` 时 `silent_days = 0`，不做减法 | ✅ 一致 |
| 时钟刻意读任一场地（§4.3） | `src/types.rs:449` 读 `self.last_contact` 而非 `last_direct_contact` | ✅ 一致，且 `src/recency.rs:19-23` 与 `src/t4d.rs:35-39` 把这个不对称面写进了模块文档 |

边界测试三处独立覆盖，且互不重复：
- `src/recency.rs:56-63` 纯函数级 0/179/180/359/360/10000；
- `tests/ablation.rs:376-388` 夹具级 `quiet_179_days`/`quiet_180_days`/`dormant_359_days`/`dormant_360_days`；
- `tests/roundx_opus_a.rs:430-454` **交叉面**：counts 来自远超截止线的一对一历史、时钟由单条群聊行提供，逐日验证 0/179/180/359/360——这一格是 `recency.rs` 与 `ablation.rs` 都没跨到的（前者只测阶梯，后者的边界夹具全是纯一对一的）。

### 1.3 as_of 纪律与无墙钟

| DECISION 条款 | 代码位置 | 判定 |
|---|---|---|
| as_of 由调用方传入，全库一个值 | 所有入口签名带 `as_of_unix: i64`（`src/lib.rs:138,155,175,180,198`；`src/types.rs:232`） | ✅ 类型层面强制 |
| 禁止 per-peer 取该 peer 自己的最大时间戳 | `src/types.rs:271-280` `as_of_max` 是给调用方的工具，规则自身从不调用；陷阱由 `tests/as_of_discipline.rs:124-145` 反证（`dormant_2019` 在 per-peer as_of 下两条规则都判 Strong，在全库 as_of 下都判 Weak） | ✅ 一致，与 DECISION §3 描述的 t4d-verify 结论同构 |
| 沉寂 2632 天 | `tests/ablation.rs:362` `assert_eq!(score.silent_days, 2_632)` | ✅ 与 DECISION §3 的数字一致 |
| judgement 一律不读墙钟 | 全 crate grep `SystemTime|Instant::|std::time|chrono|now()`：**仅命中两处文档注释**（`src/lib.rs:55`、`tests/as_of_discipline.rs:5`），无一处代码 | ✅ 事实成立（但无测试锁定，见 §3-B1） |
| soul-algo 不提供缺省墙钟路径 | `Cargo.toml` `[dependencies]` 为空；无 `default` as_of 重载 | ✅ 一致 |

附带：`tests/as_of_discipline.rs:87-121` 把「换用 `max(occurred_at)` 会让哪些夹具翻档」显式钉成数组（只有 `quiet_180_days` 与 `dormant_360_days` 两个恰好坐在阈值上的会动一档）。这是把阈值锐度当作已知性质记录下来，而不是当作缺陷掩盖，做法正确。

### 1.4 forbid unsafe / 依赖方向

| DECISION §6.2 条款 | 代码位置 | 判定 |
|---|---|---|
| `#![forbid(unsafe_code)]` | `src/lib.rs:72`，另加 `Cargo.toml:15-16` `[lints.rust] unsafe_code = "forbid"` | ✅ 双重设防 |
| 纯函数、零重依赖 | `Cargo.toml:11-13` `[dependencies]` 与 `[dev-dependencies]` 均为空 | ✅ 零依赖 |
| 禁止 SQLCipher / 存储 / Tauri / HTTP 进入该 crate | 同上，零依赖即结构性满足 | ✅ 一致 |
| 依赖箭头 Goal 1 → soul-algo | 本分支无 Goal 1 代码（README 说明其在 `cursor/soul-goal1-7b1c`），**不可在此分支验证** | ⏸ 待合并期核对 |
| 整数纪律（无浮点） | `tests/ablation.rs:536-562` 用 `include_str!` 扫描 9 个产品路径源文件禁 `f64`/`f32`；`tombstones.rs` 因是 `#[cfg(test)]` 且浮点正是其展品而豁免 | ✅ 一致，且是源码级而非声明级 |

### 1.5 墓碑

| 要求 | 代码位置 | 判定 |
|---|---|---|
| 墓碑不可达产品路径 | `src/lib.rs:80-81` `#[cfg(test)] mod tombstones;`，不 re-export | ✅ 一致 |
| T1 处决 | `src/tombstones.rs:162-194`：`afternoon_20` 一个下午 20 条 → T1 Strong；另证「权重代替不了门」——群聊行即使打 0.4 折仍越过 mass 8 | ✅ 带处决夹具 |
| T2 处决 | `src/tombstones.rs:197-223`：补偿性（一轴不足另一轴买回）+ 无新证据也移档 | ✅ |
| T3 处决 | `src/tombstones.rs:226-242`：T3 就是两条存活规则的 counts stage，`dormant_2019` 下仍读 Strong | ✅ |
| T3R 墓碑 + 复活条件定价 | `src/tombstones.rs:245-267`：`revived_after_gap` 下 T3R 折算出 38 个四分之一单位（9.5 次对阵 10 次门）→ Moderate，而四天前刚聊过的人被判中等；`269-284` 另证 T3R 也修不了场地闩 | ✅ 定价与 DECISION §5.2 回退条件对应 |
| T0 退为测试 oracle，不可选 | `src/testing/oracle.rs:135-140` `TieAlgo::from_id("T0") == None`；`src/lib.rs:255-256` 同断言 | ✅ |
| REJECTED.md 清单覆盖 | REJECTED.md 列 T0/T1/T2/T3/T3R（+A3 属 trait crate）；tombstones.rs 覆盖 T1/T2/T3/T3R，T0 在 oracle | ✅ 覆盖完整（但无绑定测试，见 §3-B3） |

### 1.6 与 DECISION §3 常量表的双源检查

**crate 内部：单点定义成立。** `t4.rs` / `t4d.rs` / `recency.rs` / `testing/oracle.rs` 全部 `use crate::constants::…`，没有一处在产品路径重写数字。`t4d.rs:112-123` 的中文模板也用 `{STRONG_MIN_INTERACTIONS}` 插值而非硬写「10」，`t4.rs:150` 的降档句用 `crate::constants::DEMOTE_AFTER_SILENT_DAYS`。

**跨 crate：破了。** 逐条比对 DECISION §3 常量表：

| DECISION 常量名 | soul-algo-tie 实际 | 判定 |
|---|---|---|
| `MODERATE_MIN_INTERACTIONS` = 3 | `constants.rs:24` 同名同值 | ✅ |
| `STRONG_MIN_INTERACTIONS` = 10 | `constants.rs:29` 同名同值 | ✅ |
| `STRONG_MIN_ACTIVE_DAYS` = 3 | `constants.rs:35` 同名同值 | ✅ |
| `DEMOTE_ONE_BAND_DAYS` = 180 | `constants.rs:57` 存在，但是 `constants.rs:51 DEMOTE_AFTER_SILENT_DAYS` 的**别名**；产品代码一律用后者 | ⚠️ 一值两名（见 §3-C1） |
| `FORCE_WEAK_DAYS` = 360 | `constants.rs:58` 同上，别名 | ⚠️ 一值两名 |
| `DORMANT_NOTE_DAYS` = `DEMOTE_ONE_BAND_DAYS`，「不得分裂」 | **soul-algo-tie 中不存在**；A2 侧在 `soul-algo-trait/src/a2.rs:91` 独立定义 `DORMANT_AFTER_DAYS: i64 = 180` | ❌ **实质双源**（见 §3-A1） |
| `GROUP_ONLY_CEILING` = Moderate，仅 T4 使用 | `constants.rs:48` 定义，仅 `t4.rs` 引用，`constants.rs:42-47` 的文档明写「T4D has no such constant」 | ✅ |

### 1.7 与 COPY_ZH 冻结话术的对照

| COPY_ZH 条款 | 实现现状 | 判定 |
|---|---|---|
| §0.1 词汇单源「强 / 中等 / 弱」 | `src/types.rs:89-95` `as_zh()` 输出「强联系 / 中等联系 / 弱联系」；`soul-algo-trait/src/types.rs:83-90` `label_zh()` 输出「强 / **中** / 弱」 | ❌ 三处三套（见 §3-A3） |
| §0.2 数字纪律（只出现可数原始量） | 满足。`tests/explain_zh.rs:88-103` 禁小数点、禁 `/`、禁「折算/半次/四分之一」 | ✅ |
| §0.3 禁词（含任何拉丁字母与 %） | `tests/explain_zh.rs:48-59` 禁全部 ASCII 字母（比逐词黑名单更强）；`62-85` 另有 14 词表 | ✅ 强度达标（但与 trait crate 的词表是两套，见 §3-A3） |
| §0.4 称呼恒为「这个人/对方」 | 满足；`Interaction` 结构里根本没有姓名字段（`src/types.rs:103-114`） | ✅ 结构性满足 |
| §0.5 时间口径：写「到 {截止日期}」，不写「到今天」 | 实现写「距今 {沉寂天数} 天」，**从不渲染 as_of 日期**（`zh_counts_total`/`zh_counts_split` 只渲染 `last_contact` 的日期） | ❌ 未落地（见 §3-A2） |
| §0.6 收尾句「这是工作假设，你可以直接改。」每条边解释必带 | 全 `crates/` 树 grep：**0 命中** | ❌ 缺失（见 §3-A2） |
| §0.7 透明句「怎么算的：只统计次数和日期，不读聊天内容。」+ T4D 追加半句 | 全 `crates/` 树 grep：**0 命中**；T4D 首句是「你们一共有 N 次往来：…」 | ❌ 缺失（见 §3-A2） |
| §1 T4D 九条分支模板 | 分支覆盖齐全（`t4d.rs:101-129` 七种 counts 措辞 + `types.rs:546-561` 空/单向 + `t4d.rs:196-213` 三种降档措辞），但**逐字措辞与 COPY_ZH 均不同** | ⚠️ 语义覆盖 ✅ / 逐字绑定 ❌ |
| §5.6 门槛引用一致性（判档读一对一，模板须引一对一数） | `tests/roundx_opus_a.rs:72-112` 的 `contradictions()` 把 9 条措辞片段各自绑定到它断言的算术上，并在 5 个测试里跑全夹具 × 补水场景 | ✅ 覆盖到位，是全套里最扎实的一条 |
| §5 验收断言 1–7「随 crates/soul-algo 写成测试」 | 1/2/3/6 有等价测试；4（禁「今天」「现在」）、5（band 词恰为三种）**无测试**；7 属 trait crate | ⚠️ 部分（见 §3-B2） |

---

## 2. 测试结果

命令：`cargo test -p soul-algo-tie`（Rust toolchain 见 `rust-toolchain.toml`，rust-version 1.83，无外网依赖）

| 测试二进制 | 通过 | 失败 | 忽略 | 耗时 |
|---|---:|---:|---:|---|
| 单元测试 `src/lib.rs`（含 `tombstones` 7 个） | 60 | 0 | 0 | 0.00s |
| `tests/ablation.rs` | 20 | 0 | 0 | 0.08s |
| `tests/as_of_discipline.rs` | 10 | 0 | 0 | 0.00s |
| `tests/direct_gate.rs` | 8 | 0 | 0 | 0.26s |
| `tests/explain_zh.rs` | 17 | 0 | 0 | 0.00s |
| `tests/goal1_fidelity.rs` | 5 | 0 | 0 | 0.04s |
| `tests/roundx_opus_a.rs` | 14 | 0 | 0 | 0.16s |
| Doc-tests | 1 | 0 | 0 | 0.14s |
| **合计** | **135** | **0** | **0** | 约 2.3s（含编译） |

附加检查：

- `cargo test -p soul-algo-tie -- --ignored` → **0 个忽略测试**（8 个二进制全部 `0 passed; 0 ignored`）。
- `cargo fmt --all -- --check` → 干净。
- `cargo clippy -p soul-algo-tie --all-targets -- -D warnings` → 干净，零警告。
- `cargo run -p soul-algo-tie --example matrix` → 正常输出 21 行消融矩阵 + 4 行分歧表。逐格核对：`lilei_12` = strong/strong/strong，`group_heavy_plus_one_direct_each_way` = strong/strong/**weak**，与 DECISION §1 的裁决表一致；`group_only_50` = strong/moderate/**weak** 与 §4.1 记录的代价一致；`dormant_2019` 距今 2632 天与 §3 一致。

### 关于四类点名测试的覆盖评价

- **ablation**：三表结构（TRUTH 判定 / OBSERVED 观测 / VIOLATIONS 结果）职责分离得很干净，`Truth::Unjudged` 这个第三态尤其值得肯定——`quiet_179_days` 和 `dormant_direct_group_ping_yesterday` 这两个本轮没有共识的格子被显式标为「记录但不计分」，而不是被反推成「规则输出即真值」。C8 结论（T4D 违反 0 项、T4 违反 3 项、T0 违反 11 项）由 `the_c8_violation_tables_are_exactly_these` 逐条重算，文档不会悄悄过期。
- **direct_gate**：8 个测试里 7 个是 2000–4000 例的生成式性质测试，覆盖的是「所有输入」而非「想到的输入」，这正是场地闩这类主张需要的证明形态；第 8 个是上文那条刻意构造的 1104 行对抗样本。
- **as_of_discipline**：10 个测试覆盖锚点自洽、夹具无未来事件、纯函数性、全库 vs per-peer 陷阱、1970 前负秒、UTC+8 时区不变性。
- **explain_zh**：17 个测试，多数是跑全夹具 × 全规则的性质断言而非单串点检。

---

## 3. 不完善项

按严重度分三级。**A = 与冻结文档冲突或单源性已破；B = 测试缺口；C = 文档/命名漂移。全部只影响渲染面与工程约束，没有一条影响 band。**

### A1（最高优先）`DORMANT_NOTE_DAYS` 单点定义已破，两 crate 各持一个 `180`

DECISION §3 明写：`DORMANT_NOTE_DAYS = DEMOTE_ONE_BAND_DAYS`，「与降档共用同一常量，**不得分裂**」，并在表头声明「全仓库禁止第二处出现数字字面量」。COPY_ZH §4 结尾再次重申「P4 的 180 天与降档常量是**同一个常量**」。

实际状态：

- `crates/soul-algo-tie/src/constants.rs` 里**没有** `DORMANT_NOTE_DAYS`；
- `crates/soul-algo-trait/src/a2.rs:91` 写着 `pub const DORMANT_AFTER_DAYS: i64 = 180;`（名字也不是 DECISION 指定的那个）；
- 两个 crate 的 `Cargo.toml` `[dependencies]` 均为空，**彼此零依赖**，物理上不可能共享这个常量；
- 两边还各自定义了一份 `SECONDS_PER_DAY = 86_400`（`soul-algo-tie/src/types.rs:31` 与 `soul-algo-trait/src/a2.rs:94`）。

后果与 DECISION 预警的完全一致：改一处降档阈值不会带动另一处，A2 的「最近半年没有往来」句与判档降档会静默漂移；而 COPY_ZH §4 所依赖的那条「免费成立」的不变式（出现半年句的边必然已降档，永远不是强）正是建立在两者同源之上的。这项在 DECISION §6 的合并义务（统一为单个 `crates/soul-algo`）完成前无法根治，但**在报告里必须记为一条现存的规范违反，而不是待办事项**。

### A2 COPY_ZH 三个全局强制项在代码里完全没有落地

全 `crates/` 树对「这是工作假设」「怎么算的」「截止日期」「不读聊天内容」四个串 grep，**0 命中**。具体缺失：

- **§0.6 收尾句**「这是工作假设，你可以直接改。」——COPY_ZH 规定「每条边解释与每条推断解释必带」。T4D 与 T4 的九条模板全部没有。这不是措辞偏好问题：这句话是产品把档位声明为「可改的工作假设」而非「判断」的唯一载体。
- **§0.7 透明句**「怎么算的：只统计次数和日期，不读聊天内容。」+ T4D 追加半句「判档只看你们一对一的往来。」——COPY_ZH 规定它是**每套边解释的首句**。实现的首句是计数句。
- **§0.5 时间口径**——COPY_ZH 要求写「到 {截止日期} 已经 {沉寂天数} 天没有联系」，明确「不写『到今天』」。实现统一写「距今 {沉寂天数} 天」，且 `as_of_unix` 虽然在 `TieScore` 上（`src/types.rs:177`），**任何模板都不渲染它**。严格说「距今」不触发 COPY_ZH §5.4 的字面禁词（禁的是「今天」「现在」，「距今 N 天」不含这两个词），但它恰好制造了 §0.5 想避免的那个误读——用户会把基准读成阅读当天，而 as_of 可能是几周前的导入时刻。这是 as_of 纪律在**话术层**的唯一缺口（算法层完全合规）。

这三项合起来指向同一件事：DECISION §6.4 列为合并义务的「中文模板绑定测试（COPY_ZH.md 断言）随 soul-algo 落地」**尚未开工**。现有的 `tests/explain_zh.rs` 测的是自己写下的措辞，不是 COPY_ZH 冻结的措辞——它能防止措辞被无声改动，但不能防止措辞与规范不一致。

### A3 档位词三处三套，`assert_non_clinical` 词表两套

档位词：

| 来源 | Strong | Moderate | Weak |
|---|---|---|---|
| COPY_ZH §0.1（冻结，「全仓库用户可见文本只许这三个词」） | 强 | 中等 | 弱 |
| `soul-algo-tie/src/types.rs:89-95` | 强**联系** | 中等**联系** | 弱**联系** |
| `soul-algo-trait/src/types.rs:83-90` | 强 | **中** | 弱 |

COPY_ZH §5.5 的验收断言要求「band 词只出现『强 / 中等 / 弱』三种」。tie crate 的三字词是超集（含子串），trait crate 的「中」则连子串关系都不成立。更值得注意的是 trait crate 的 `PEER_CLAIM_DENYLIST`（`denylist.rs:56-71`）把「强关系」「弱关系」列为禁止的第三方claim——tie crate 输出的「强联系」只差一个字，靠字形差异而非设计意图躲过了这道筛。

禁词表同样双源：tie crate 在 `tests/explain_zh.rs:66-81` 内联了 14 个词，trait crate 在 `denylist.rs` 维护 `DIAGNOSTIC_DENYLIST`(10) + `NUMERIC_RATING_MARKERS`(10) + `PEER_CLAIM_DENYLIST`(14) 三张表并提供 `assert_publishable`。COPY_ZH §5.1 要求「每个模板渲染结果通过 `assert_non_clinical`」——tie crate 的输出**从未经过** trait crate 的那套筛子。

### A4 实现里有一条 COPY_ZH 未收录的「第二个日期」句

`src/t4d.rs:157-165` 的 `zh_direct_clock_note`：当 `last_direct_contact_unix != last_contact_unix` 时，在解释末尾追加「一对一最近一次是 {日期}。」

来龙去脉需要说清楚，因为它容易被误判：

- 这条句子由 Round X opus-a 引入，用于修补一个真实缺陷——任一场地时钟下，一段 300 天前就停掉的一对一历史靠昨天一条**入站**群消息就能保住 Strong，而用户从解释里看不到那段历史有多老（`tests/roundx_opus_a.rs:344-380` 证明这个效应对任意年龄的历史都成立，7000 天也一样）。修补动机正当，且**不触碰任何 band**。
- 但 REJECTED.md §附第一条在作废「T4D direct-only 降档时钟」时，给出的理由正是「模板需第二个日期概念，违反可数性纪律」。实现现在把**时钟**留在任一场地（合规），却把那个被点名批评的**第二个日期概念**加进了渲染面。
- COPY_ZH §1 的 T4D 九条模板里没有这句话，DECISION §6.4 要求「模板改动先改 COPY_ZH.md 再改代码」。

我的判断：这条句子在信息披露上是净收益（DECISION §4.3 承认这个不对称面存在，用户理应看得见），但它现在的状态是**未经留痕的规范外增补**。正确处置是补进 COPY_ZH 并说明它与 REJECTED §附注的关系，而不是删掉它。**本报告不做任何改动。**

### B1 无墙钟这条纪律没有测试锁定

`tests/ablation.rs:536-562` 已经用 `include_str!` 做了源码级扫描来禁 `f64`/`f32`，做法很好。但同一个扫描**没有把 `SystemTime` / `Instant` / `std::time` 加进禁列**，而「不读墙钟」是 DECISION §3 与 §6.2 反复强调的纪律，级别不低于整数纪律。当前「无墙钟」只是 grep 出来的事实和两处文档注释里的声明（`src/lib.rs:55`、`tests/as_of_discipline.rs:5`），任何人加一行 `SystemTime::now()` 不会有测试变红。`tests/as_of_discipline.rs:63-84` 的 `scoring_is_a_pure_function_of_the_evidence_and_the_as_of` 只跑两遍同一调用，其注释也坦承「Running the same call twice is a weak test on its own」。

**这是全套测试里最便宜、也最该补的一个缺口**：在现有 `include_str!` 扫描的 `banned` 数组里加三个串即可，零风险。

### B2 COPY_ZH §5 验收断言 4 与 5 无对应测试

- **断言 4**（任何模板不出现「今天」「现在」字样）：无测试。当前实现恰好不含这两词，但没有护栏。
- **断言 5**（band 词只出现「强/中等/弱」三种，且与 `SupportedBand` 一一对应）：无测试，且如 A3 所述当前实现就不满足字面要求。

### B3 REJECTED.md 与 tombstones.rs 之间无绑定测试

墓碑覆盖事实上是完整的（§1.5 已核），但「REJECTED.md 列出的每个落选者都有一个可执行墓碑」这件事没有机器校验。将来 REJECTED.md 新增一条墓碑而 tombstones.rs 忘了跟进，不会有任何测试变红。对照之下，`tests/ablation.rs:203-210` 的 `every_fixture_appears_in_every_table` 就是这类清单一致性的正确范式，可以照搬。

### B4 `last_direct_contact_unix = 0` 哨兵重载，且类型不强制空值检查

`src/types.rs:171` 用 `0` 同时表示「1970-01-01 那一秒的真实一对一往来」与「从未一对一」。`Tally` 内部靠私有的 `saw_direct: bool`（`src/types.rs:333`）区分这两者，所以 `Tally` 自身是对的（`tests/roundx_opus_a.rs:260-304` 穷举了跨纪元的两行日志组合来证明这点）。

但 `TieScore` 是 DECISION §6.4 定义的**对外渲染契约**——「A2 与图 UI 只消费该结构」——而它把 `saw_direct` 丢掉了，只留下一个 `i64`。`tests/roundx_opus_a.rs:307-321` 把后果钉成了现状测试：`group_only_50` 这类边上，`age_days(last_direct_contact_unix, as_of)` 算出 20689 天，`zh_date()` 渲染成「1970 年 1 月 1 日」。crate 自己的 `t4d.rs:158` 用 `!score.any_direct()` 做了守卫，但类型没有强制任何下游调用方也这么做。改成 `Option<i64>` 可以让编译器接管这件事；这是 API 形状问题，不改任何 band。

### B5 `Detail` 不在 §6.4 的存储契约里，`explain_zh` 对回读分数的处理是「静默改口」

DECISION §6.4 列出 `TieScore` 携带的字段：band、原始计数、一对一/群聊分列、last_contact、沉寂天数、as_of——**没有 `detail`**。所以任何从图里读回来的分数，`detail` 都只能是重新猜的。

现在 `explain_zh` 的处理已经比 Round X 报告描述的状态好很多：`t4d.rs:182` 用 `counts_band_from_score` 从计数**重算** counts stage，不再信任传入的 `detail`，`tests/roundx_opus_a.rs:132-235` 五个测试证明补水后的分数不会说出与计数矛盾的话。但 `t4d.rs:188-194` 那条兜底分支——当 `score.band` 与重算结果不一致时，输出「按现在的规则应是 X。存档里这一档是 Y。」——是**用户可见**的措辞，而 COPY_ZH 里没有这个模板。同样属于规范外增补（性质与 A4 相同，但触发条件更罕见）。

### C1 DECISION 常量名在代码里是别名而非主名

`constants.rs:51,54` 的主名是 `DEMOTE_AFTER_SILENT_DAYS` / `WEAK_AFTER_SILENT_DAYS`，DECISION §3 钦定的 `DEMOTE_ONE_BAND_DAYS` / `FORCE_WEAK_DAYS` 在 `constants.rs:57-58` 作为别名存在，且产品代码（`recency.rs:25`、`t4.rs:151`、`t4d.rs:199`）一律引用主名。值是单源的，所以这不是漂移风险，但一值两名会让「按 DECISION 常量名 grep 全仓库」这个最自然的核查动作漏掉真正的引用点。

### C2 DECISION §4.3 引用的夹具名与天数在代码里不存在

DECISION §4.3 写：「实测 `direct_quiet_200_group_yesterday` → Strong」，描述为「一对一历史停在 **200** 天前」。代码里对应的夹具是 `dormant_direct_group_ping_yesterday`（`src/testing/mod.rs:384-393`），一对一历史停在 **300** 天前。语义等价、结论相同，但名字和数字都对不上，按夹具名检索会落空。（DECISION §4.1 引用的 `group_heavy_plus_three_directs` 与 §1 引用的 `group_heavy_plus_one_direct_each_way`、`lilei_12` 则完全对得上。）

### C3 DECISION §6.4 指向 COPY_ZH「第 6 节断言」，实际在第 5 节

COPY_ZH 的验收断言标题是「## 5. 模板验收断言」，文件里没有第 6 节。DECISION §6.4 写的是「COPY_ZH.md 第 6 节断言」。落地那批绑定测试时会按图索骥落空。

### C4 `tests/roundx_opus_a.rs` 模块头文档已过期

该文件 1–48 行的文档把三条缺陷描述为**现存**（「Eleven of the twenty-one fixtures are affected, over 26 fixture/rule combinations」），并说「`#[ignore]`d `bug_*` tests assert the behaviour the product lock requires and **fail today**」，指引读者跑 `cargo test -p soul-algo-tie -- --ignored` 查看未决缺陷清单。

实际状态：§1 的缺陷已修（`explain_zh` 现在从计数选措辞）、§2 的哨兵问题在 `Tally` 层已修（`saw_direct`）、§3 的日期披露已加（`zh_direct_clock_note`）；文件里**没有任何 `#[ignore]` 或 `bug_*` 测试**，`--ignored` 返回 0 个。14 个测试全部是修后行为的正向断言。文档比代码落后一轮，会让读者以为有 26 处未决缺陷。

### C5 crate 尚未统一为 `crates/soul-algo`

DECISION §6.1 与 §0 把「统一 `crates/soul-algo`」列为冻结后的合并义务。当前是 `soul-algo-tie` + `soul-algo-trait` 两个零依赖的独立 crate，两者各有一个 `TieScore` 类型（`soul-algo-tie/src/types.rs:140` 与 `soul-algo-trait/src/a2.rs:107`）且互不相通。DECISION §6.4 说「A2 与图 UI 只消费该结构」——这条在类型层面目前**没有任何东西在保证**，A2 消费的是它自己那个同名结构。README 已如实承认这是未合并状态，此处只作记录。A1 的常量双源与 A3 的词表双源都是这一条的直接后果。

---

## 4. 建议的处置顺序（不在本报告范围内执行）

按「零风险 → 需留痕」排序，供父代理排期：

1. **B1**（把 `SystemTime`/`Instant`/`std::time` 加进已有的源码扫描禁列）——一行数组改动，零风险，收益是锁死一条被反复强调的纪律。
2. **B3 / B2**（清单一致性测试、COPY_ZH 断言 4/5 的测试）——照搬 `every_fixture_appears_in_every_table` 的范式即可。
3. **C1–C4**（文档与命名漂移）——纯文档，其中 C4 最紧急，因为它会误导下一个读这份测试的人。
4. **A2 / A3**（COPY_ZH 模板落地 + 词表统一）——须先决定「改代码对齐 COPY_ZH」还是「改 COPY_ZH 对齐代码」；DECISION §6.4 的程序是**先改 COPY_ZH 再改代码**，所以这件事必须走留痕通道，不能由实现方单方面决定。
5. **A4 / B5**（两条规范外增补句）——补进 COPY_ZH 并说明与 REJECTED §附注的关系。倾向保留句子、补齐文档，而非删句。
6. **A1 / C5 / B4**（crate 合并、常量单源、`Option<i64>`）——同一次结构性变更里一起做，是 DECISION §6 的合并义务本体。

## 5. 未改动声明

本次审计**没有修改任何生产逻辑、测试或文档**。工作树上唯一的新增文件是本报告。所有结论均可用以下命令原地复现：

```bash
cargo test -p soul-algo-tie
cargo test -p soul-algo-tie -- --ignored
cargo fmt --all -- --check
cargo clippy -p soul-algo-tie --all-targets -- -D warnings
cargo run -p soul-algo-tie --example matrix
```
