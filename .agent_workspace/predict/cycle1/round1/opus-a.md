MODEL_SLUG: claude-opus-5-thinking-high-fast

# Cycle 1 / Round 1 / opus-a —— 已存事实 → 预测器可用性映射

角色：**只读调研**。本轮零产品 crate 改动、零 docs 改动、零 git commit，写入面恰为本文件。
读取时点：HEAD `2b0b555`，工作区除 `.agent_workspace/predict/cycle1/`（未跟踪）外干净。
遵 `CONSTRAINTS.md`：不实现 Goal 2、不改 T4D/A0、不把预测模型写进产品 crate。

本文回答一个问题：**今天真的落在盘上的每一条事实，能不能成为 v0.2 浅层预测器的输入或标签？**
不给算法方案（那是后面几轮的事），只给数据面盘点、可数性判断、以及 14 条缺口。

---

## 0. 一句话结论

前台采集这条线**已经落库的明文只有一个时间戳和三个枚举**（`ts` / `kind` / `source` /
`privacy_subject`）——**应用名和时长两样都在密封体里**，SQL 聚合不到；而人脉那条线
（`TieStrength`）落库最厚、字段最全，却**整条是第三人数据、`local_only`、且已冻结**。
换句话说：**最想预测的东西（用什么 app、用多久）目前最难聚合，最好聚合的东西（人脉计数）
最不能出门。** 后面几轮的算法候选必须先在这条不对称上表态。

---

## 1. 主表：每条已存事实 → 能否喂后续预测器

「密封?」= 是否只在 AEAD 密封体内、需 content key 才能读；
「可作 derived aggregate?」= 是否能在不违反产品锁的前提下汇成 derived/aggregate 行
（✅ 可 / ⚠️ 有条件 / ❌ 不可）。

### 1.1 soul-collect —— 前台会话

| 字段 | crate / 位置 | 密封? | 第三人? | 可作 derived aggregate? | 示例 predictand |
|---|---|---|---|---|---|
| `AppIdentity`（`notepad.exe`，已小写、无路径、≤128 字符、须 `.exe/.com/.scr`） | soul-collect `source.rs`；落库形态是 `SealedSessionBody.app` | **是** | 否（本人） | ⚠️ 可，但**必须在解封边界内**算完；聚合行须挂回同一 `content_key_id`（见 G12） | next-app：给定当前 app + 时间桶，预测下一个前台 app |
| `SealedSessionBody.duration_ms` | soul-collect `session.rs` | **是** | 否 | ⚠️ 同上；分桶后可填今天空着的 `duration_bucket`（见 G6） | 本次会话是否 >N 分钟（长专注 vs 扫一眼） |
| `SealedSessionBody` 的**字段集本身**（`FIELDS = ["app","duration_ms"]` + `deny_unknown_fields` + 钉测 `a_body_with_an_extra_field_is_refused…`） | soul-collect `session.rs` | — | — | ❌ 不是数据，是**约束**：往密封体加特征字段会红测，属破坏性改动 | —（供后续轮次评估「加字段」代价） |
| `ForegroundSession.started_at_unix_millis` | soul-collect `session.rs` | 否（**秒级**经 `started_at_unix_seconds()` → `events.ts` 明文列） | 否 | ✅ 无需解封即可按小时/日分桶 | 小时桶事件数（唯一今天就能算的节律量） |
| `ForegroundSession.ended_at_unix_millis` | soul-collect `session.rs` | **未落库** | 否 | ⚠️ 只能由 `ts + duration_ms` 还原 ⇒ 仍需解封 | 会话结束时刻 / 间隙长度 |
| `events.kind = app.foreground` | soul-schema `event.rs`；`events.kind` 明文列 | 否 | 否 | ✅ | 「今天有没有开机使用」二元标签 |
| `events.source = collector.foreground_app` | 同上 | 否 | 否 | ✅ | 来源分层（采集 vs 导入）的分组键 |
| `events.actor_subject` / `privacy_subject`（采集恒 `self`） | soul-schema `common.rs`；明文列 + `events_by_subject` 索引 | 否 | 否 | ✅ | 研究预览的排除键（见 G11） |
| `privacy.purposes = [soul_profile, research]`、`derivation = raw`、`retention = until_forgotten`、`egress` 三档全 deny | soul-collect `collector.rs::collected_privacy` | 否（在 `doc` JSON 里） | 否 | ✅ 读；写新值要重新表态 | 预测器产出的行必须**自己**声明 `derivation`（应为 `aggregate`，今天无写者） |
| `content_key_id`（**每 run 一把**） | soul-collect `collector.rs`；`content_keys` / `sealed_blobs` 明文列 | 否 | 否 | ⚠️ 是**遗忘单元**，不是特征 | —（决定聚合行的生命周期，见 G12） |
| `Tally{events_written, sessions_discarded, consent_refusals, source_errors}` | soul-collect `collector.rs` | **不落库**（进程内） | 否 | ❌ 除 `events_written` 外全部随进程消失 | —（`sessions_discarded>0` 本可解释缺席，但读不到，见 G3） |
| 审计 `collect.start` / `collect.stop` + `ReasonCode` + `AuditCounts.items` | soul-policy `audit.rs` → `audit` 表 | 否（无正文） | 否 | ⚠️ 可读，但**审计行永不被遗忘**，把它当特征等于造了一条绕过遗忘的旁路 | 采集在线时段（可用来区分「没用电脑」和「采集没开」——但见 G3 的代价） |

### 1.2 soul-schema —— 词表里已有、但**今天没有写者**的位置

这一节不是「已存事实」，而是**已冻结的空位**：v0.2 预测器如果要落库，这些是唯一合法的门。

| 字段 | crate / 位置 | 密封? | 第三人? | 可作 derived aggregate? | 示例 predictand |
|---|---|---|---|---|---|
| `SoulInference{statement_key, evidence_ids(非空契约), evidence_band, method, user_verdict, falsifier}` | soul-schema `inference.rs` | 否 | 否 | ✅ **预测输出的正确落点**：`method: Statistical` 已在枚举里，`falsifier` 字段字面上就是「怎么证伪」，`user_verdict` 字面上就是「可纠正」 | 任何预测：预测本身作为一条 inference，用户可 Accepted/Corrected/Rejected |
| `EvidenceKind::AppUsage` | soul-schema `evidence.rs`（也镜像在 soul-algo-trait `types.rs`） | 否 | 否 | ⚠️ **全仓无写者**（grep 确认：只在 soul-algo-trait 的枚举与其测试里出现） | —（见 G14：前台会话今天不产 evidence 行） |
| `EvidenceKind::Aggregate` | 同上 | 否 | 否 | ⚠️ 同样无写者 | 「N 天的小时桶汇总」这类证据的合法 kind |
| `Derivation::Aggregate` | soul-schema `common.rs` | 否 | — | ⚠️ 无写者；采集恒写 `Raw` | 聚合行的 `privacy.derivation` |
| `ExportField::DurationBucket` / `ExportRow.duration_bucket` | soul-schema `export_manifest.rs` | 否 | 否 | ⚠️ **有字段、有 `fields_present` 分支、有 CLI 透传（`soulcore/commands/store.rs`）、无生产者**（`research_preview.rs` 从不写它） | 时长分桶——研究面上唯一为「时长」预留的位置 |
| `EvidenceMethod::{Heuristic, Statistical}` | soul-schema `evidence.rs` | 否 | — | ✅ 可用；今天只有测试用 `Heuristic` | 预测器证据的 method 标注 |

### 1.3 soul-graph —— `TieStrength` 已落库字段（**全部第三人 / `local_only` / T4D 已冻结**）

写入路径：`build.rs` 整边重建后替换，序列化进 `relationships.doc`。**全部字段第三人**，`PersonNode`
与 `TieEdge` 的 `egress_scope` 恒 `LocalOnly`，`SoulGraph::third_party_data_is_local_only()` 有整图断言。

| 字段 | crate / 位置 | 密封? | 第三人? | 可作 derived aggregate? | 示例 predictand |
|---|---|---|---|---|---|
| `band`（`SupportedBand`，锁定时为用户档） | soul-graph `model.rs` | 否 | **是** | ⚠️ 仅本机 | 「这条边下个月是否掉档」 |
| `interaction_count` / `outgoing_count` / `incoming_count` | 同上 | 否 | **是** | ⚠️ 仅本机 | 未来 30 天是否还有往来 |
| `direct_out_count` / `direct_in_count` / `group_out_count` / `group_in_count`（T4D 的分列） | 同上 | 否 | **是** | ⚠️ 仅本机 | 一对一 vs 群聊占比的漂移 |
| `conversation_count` | 同上 | 否 | **是** | ⚠️ 仅本机 | 关系是否只活在单一会话里 |
| `active_day_count` / `direct_active_day_count` | 同上 | 否 | **是** | ⚠️ 仅本机 | 活跃天密度 |
| `first_contact_utc` / `last_contact_utc` / `last_direct_contact_utc` | 同上 | 否 | **是** | ⚠️ 仅本机 | **人脉近因**：下次联系的间隔 |
| `silent_days`（`last_contact_utc` 到 `as_of` 的整 UTC 天） | 同上 | 否 | **是** | ⚠️ 仅本机 | 静默是否会跨过降档阈值 |
| `as_of_utc`（**一次重建一个全库时刻**，不是时钟读数） | 同上 | 否 | **是** | ⚠️ 必须沿用、不可自取时钟（见 G13） | —（评估的时间锚） |
| `algorithm_id` | 同上 | 否 | **是** | ✅ 可作分层键 | 换规则前后的对照分组 |
| `locked_by_user` / `user_band` / `machine_band` | 同上 | 否 | **是** | ⚠️ 仅本机 | **用户纠正本身是标签**：机器档 vs 用户档的分歧率 |
| `TieEdge.types`（`direct/group_only/reciprocal/one_sided`） | soul-graph `model.rs` | 否 | **是** | ⚠️ 仅本机 | 关系形态转变（单向→互惠） |
| `TieEdge.evidence_ids`（非空构造） | 同上 | 否 | **是** | ⚠️ 仅本机 | 可解释性所需的回指 |
| `PersonNode.{contact_id, contact_class, identifier_hashes, forget_state, interaction_count, last_contact_utc}` | 同上 | `label_ref` 是密封指针；其余否 | **是** | ⚠️ 仅本机 | 节点级近因 |
| `InteractionRef.{event_id, peer_contact_id, conversation_ref(SHA256), direction, occurred_at, venue}` | soul-graph `interaction.rs`，存在 evidence 的 `source_refs` | 否（正文在 event 的 `body_ref`） | **是** | ⚠️ 仅本机；`exportable_to_research = Some(false)` 已写死 | **逐条**交互的时间戳——今天唯一带**本地偏移**的时间源（见 G2） |
| `TieScore.detail`（`RawCounts` / `Demoted{band_before}`） | soul-algo-tie `types.rs` | — | — | ❌ **未落库**：`TieStrength` 没有 `detail` 字段 | —（「因静默被降档」只能由 `silent_days`+`machine_band` 反推，`band_before` 丢了） |

### 1.4 soul-profile —— 特质轴（A0，用户锁）

| 字段 | crate / 位置 | 密封? | 第三人? | 可作 derived aggregate? | 示例 predictand |
|---|---|---|---|---|---|
| `TraitAxis.axis_id`（5 个固定 UUID 常量） | soul-profile `axes.rs` | 否 | 否 | ✅ 分组键 | —（跨安装稳定的轴身份） |
| `TraitAxis.position`（`leans_low/mixed/leans_high/unknown`，**四值，无数值**） | soul-schema `profile.rs` | 否 | 否 | ✅ 已在研究预览里作 `self_trait_axis` 输出 | 轴方向是否会被下一次问卷/纠正翻转 |
| `TraitAxis.evidence_band`（`weak/moderate/strong/none`） | 同上 | 否 | 否 | ✅ 已作 `self_trait_band` 输出；`none` 被预览丢弃 | 证据档位的爬升速度 |
| `TraitAxis.evidence_ids` | 同上 | 否 | 否 | ⚠️ 是回指，不是特征 | 可解释性所需 |
| `TraitAxis.locked_by_user` | 同上 | 否 | 否 | ✅ | **用户纠正率**——最干净的「预测被推翻」标签 |
| `clinical_claim`（恒 `false`，反序列化拒真） | soul-schema `common.rs` | — | — | ❌ 常量 | —（D22 的类型级执行） |
| `SoulProfile.voice` / `values` / `boundaries`（`serde_json::Value`） | soul-schema `profile.rs` | 否 | 否 | ❌ 自由形，无结构可数 | — |
| A1 独立性键 `(kind, utc_day)` | soul-algo-trait `types.rs::utc_day` | — | — | ⚠️ 是**规则**不是字段 | —（见 G9：同一 UTC 天的 N 次观察算一组） |

### 1.5 唯一一个已存在的时间聚合

| 事实 | crate / 位置 | 密封? | 第三人? | 可作 derived aggregate? | 示例 predictand |
|---|---|---|---|---|---|
| `EVENT_ROLLUP_SQL`：`(kind, UTC 小时桶, privacy_subject) → count(*)` | soul-store `research_preview.rs` | 否 | 按 `privacy_subject` 打标后**整类排除** third_party/mixed | ⚠️ **只在内存**：D7/D18 说红队未做，`written_to_disk` 恒 false，manifest 只回给调用方 | 小时桶事件计数——**今天唯一现成的节律量** |

注三点：(a) 桶是 **UTC 小时**，`ts` 带偏移时**退化成日期级**桶（`substr(ts,1,10)`）而不是谎称 UTC；
(b) 它数的是**事件条数**，不是**时长**；(c) 它**不分 app**（app 在密封体里，SQL 看不见）。

---

## 2. 缺口（Flags）

`G1`–`G14`，按「会不会让某类预测器直接失效」排。前两条是父代理点名的。

**G1 —— 没有 hour-of-week 直方图。** 全仓唯一的时间分桶是 §1.5 那条 SQL，它是
「UTC 小时 × kind × subject」的**事件计数**，且不落盘、不分 app、不按时长加权。星期几更是
完全没有：`t4d_adapt.rs` 有 `days_from_civil`、soul-policy 有 `civil_from_days`、soul-algo-tie
有 `epoch_day`/`civil_from_epoch_day`——**三处各自实现了民用日历，没有一处导出 day-of-week**。
任何「周一上午通常开什么」的预测器要么自带日历，要么先补这个原语。

**G2 —— 全仓没有本地时区/UTC 偏移。** 采集侧 `soul_policy::clock::rfc3339_utc` 只产
`...Z`，`ForegroundSession` 里没有偏移字段。于是「用户的早上」不可恢复：跨时区出差、夏令时、
甚至只是安装在 UTC+8 的机器，都会让 UTC 小时直方图错位。**唯一带偏移的时间源是导入的
`InteractionRef.occurred_at`**（`t4d_adapt.rs::parse_rfc3339` 解析 `+08:00`），但那是第三人线。
作息预测在补上这一条之前，做出来的是「UTC 作息」，不是用户作息。

**G3 —— 缺席不可解释，而且缺失非随机。** 「没有行」至少对应五种情况：机器关机 / 会话锁定 /
`NothingInForeground`（无焦点或不肯识别的进程）/ 采集未开 / **同意被撤销**。第五种尤其毒：
`poll_once` 遇到 `ConsentWithheld` 会 `discard_open()` 把在飞会话**丢弃**，`finish()` 在无同意时
也丢弃——这是**由用户行为触发的、系统性的**数据删除，不是随机缺失。而唯一能区分它们的计数器
`Tally.sessions_discarded` **不落库**。后果：直接在事件序列上学节律，学到的是采集器的在线时间，
不是人的作息。审计链的 `collect.start`/`collect.stop` 能补一部分，但把它当特征等于用一条
**永不被遗忘**的表去解释一条**可被遗忘**的表（见 G12 的同类问题）。

**G4 —— 没有转移记录。** 相邻会话是两条独立事件，没有 `prev_app` 字段。排序只能靠
`events.ts`，而 `ts` 是会话**起点**且**只到秒**——同秒两条并列可能（1 秒轮询下，一次
`SessionRecorded` 会同时结束旧会话并以同一 `now` 起新会话）。`event_id` 是 `Uuid::now_v7()`
（生成于写入时刻）可作 tie-breaker，但那是实现细节不是契约。next-app 预测要的
「A→B 转移矩阵」今天必须**在解封边界内、按 (ts, event_id) 重排后**现算。

**G5 —— 时长不可 SQL 聚合。** `duration_ms` 在密封体里，`events` 表只有
`event_id/ts/source/kind/actor_subject/privacy_subject/body_blob_id/doc`。任何「按时长加权」的
统计（而不是「按会话条数」）都必须持 content key 逐条解封。这直接决定了预测器只能是
**本机、在线、持钥**的组件——不能是一个跑在导出数据上的离线脚本。

**G6 —— 有词无写者的三个空位。** `Derivation::Aggregate`、`EvidenceKind::{AppUsage, Aggregate}`、
`ExportField::DurationBucket` / `ExportRow.duration_bucket` 全部在词表里、有序列化、有读取分支
（`fields_present` 判存、`soulcore` CLI 结构体透传），**但全仓无一处赋非空值**（grep 确认：
`crates/*/src` 下 `duration_bucket` 的四处命中依次是字段声明、`is_some()` 判存、CLI DTO 声明、
CLI `clone()` 透传，没有构造点）。同时**没有 aggregates 表**：`sql.rs` 的 14 张表里没有任何
一张是给派生汇总用的。v0.2 若要持久化直方图，是**新增持久化面**，而不是填一个已有的洞。

**G7 —— 无 app 内活动信号，同名不可分。** 标题、文件元数据、键鼠全在「不做」列，
`window_titles_are_not_collected.rs` 会读回源码检查。于是 `code.exe` 写代码和 `code.exe` 看日志
是同一件事，`chrome.exe` 里的一切都是同一件事。**可执行名是一个粗到会掩盖大部分意图的特征**，
这不是缺陷是产品锁——但预测器的天花板由它决定，评测集必须按这个粒度设计，不能拿
「预测用户在做什么任务」当 predictand。

**G8 —— 采样量化与时钟伪影。** 默认 `poll_interval = 1s`；短于一个 tick 的切换可能整段丢失
（`SessionStarted` 从未变成 `SessionRecorded`）；`SameApp` 合并意味着 `alt-tab` 到同一 exe 的另一
窗口不产生新会话；`duration_ms()` 对回拨时钟做 `saturating_sub`，于是库里**存在合法的 0ms 会话**。
时长分桶的最低桶必须能容纳这三种伪影，否则模型会去学时钟同步。

**G9 —— A1 的独立性键会压平高频行为数据。** `(kind, utc_day)`：同一 UTC 天的 N 次观察算**一组**。
前台采集一天能产几百条会话，但对轴的证据档位而言仍是**一组**。任何「用行为数据把某条轴推到
strong」的想法在 A0/A1 下不成立，且 `reachable_for_axis_in_v01()` 明写 v0.1 只有
`Questionnaire | UserCorrection` 能对轴产生证据。**不改 A0** 的前提下，行为预测不能改轴。

**G10 —— 不能显示概率。** D22 禁量表/百分位，`Band` 甚至**故意不实现 `Ord`**、只给 `rank()`
方法，注释写明「不让任何调用方偷偷做算术」。于是预测器可以内部算 logit，**但用户面只能是
`weak/moderate/strong` 三档**。这不是 UI 偏好，是必须在算法设计阶段就吃下的约束：任何需要
校准概率才有意义的输出（「73% 概率你接下来开 Excel」）在这个产品里没有合法呈现方式。

**G11 —— 混主体行会被整行排除。** `SubjectClass::parse` 把 `mixed` 当 `third_party` 处理。
把 `TieStrength` 特征和 app 特征拼进同一行，`privacy.subject` 就是 `mixed`，研究预览会整行剔除
并计入 `third_party_rows_excluded`。**「人脉 × 前台」的联合模型天然是纯本机的**，永远不可能有
研究出口——这一点要在候选算法里明说，不要事后发现。

**G12 —— 聚合行与遗忘单元的错配。** content key 是**每 run 一把**，「遗忘采集期间的数据」就是
删这把 key。但一个派生直方图如果不挂在同一 `content_key_id` 上，**删了 key 之后直方图还在**——
用户以为忘了，其实统计量留着。任何持久化聚合都必须要么进 `memory_content_keys` 那种依赖表，
要么在 forget 预览里可见。这是 v0.2 落库设计的第一约束。

**G13 —— T4D 的 `as_of` 已冻结，且边是整体替换。** `as_of_utc` 是「一次重建一个全库时刻」，
`as_of_discipline.rs` 有钉测。预测器**必须沿用传入的 as_of，不可读时钟**；也**不能在
`TieStrength` 上就地追加预测字段**——`build.rs` 每次重建整边替换，追加的字段下次重建就没了。
（`#[serde(default)]` 让旧行能读回，但那是向后兼容，不是给外部写字段的口子。）

**G14 —— 前台会话今天不产 evidence 行。** 产品锁写「无证据不得落库」，`SoulInference.evidence_ids`
是**非空契约**（schema 与 store 双重把关）。但 `app.foreground` 只写 event，**不写 evidence**；
全仓 `EvidenceKind::AppUsage` 无写者。所以「基于 app 用量的预测」今天**没有合法的 evidence_id 可引**。
在写任何预测 inference 之前，必须先补一条 AppUsage/Aggregate 证据链——**这是 v0.2 的第一个前置件，
不是可选项**。

---

## 3. 与产品锁的冲突清单（后续轮次必须逐条表态）

| 锁 | 冲突面 | 已知出路（不在本轮拍板） |
|---|---|---|
| 采集仅「exe 名 + 时长」 | 预测精度天花板由 exe 粒度决定（G7） | 接受天花板；predictand 只到 app 级 |
| 密封体只有两字段 + `deny_unknown_fields` + 钉测 | 想加 `duration_bucket` 之类的派生字段会红测（§1.1 第 3 行） | 派生量放在**聚合行**里，不进密封体 |
| 无证据不得落库 | 无 AppUsage evidence（G14） | 先补证据链，再谈 inference |
| 第三人行数恒 0 | 人脉 × 前台联合特征恒 `mixed`（G11） | 联合模型标注为纯本机、无研究出口 |
| 禁量表分数/百分位（D22） | 概率输出无合法呈现（G10） | 内部数值 → 三档 band；`Band` 无 `Ord` 是硬约束 |
| 输出可解释、可纠正 | 需要 `falsifier` + `user_verdict` + 可解析 `evidence_ids` | `SoulInference` 已具备全部三样，是唯一正确落点 |
| 遗忘语义 | 聚合行可能比 content key 活得久（G12） | 聚合行挂 content key 或进 forget 预览 |
| 审计无正文 | 用审计链解释缺席等于造不可遗忘旁路（G3） | 谨慎；至少不能把审计当训练特征 |
| 不改 T4D/A0 | 不可加 `TieStrength` 字段、不可自取时钟（G13）、不可用行为数据推轴（G9） | 预测器只读消费，另立输出面 |
| E0 无代码路径 / 联邦学习 v0.4 前非默认 | 任何跨机协同过滤出局 | v0.2 候选一律单机 |

---

## 4. 后续轮次可直接接手的三件事

1. **补 day-of-week / 本地偏移原语**（G1+G2）。现状是三处各写了一份民用日历、无一导出 DOW，
   且采集侧完全没有偏移。这是所有节律类算法的共同前置件，值得单独一轮。
2. **定义 AppUsage 证据链的形状**（G14+G6）。`EvidenceKind::AppUsage`、`Derivation::Aggregate`、
   `ExportField::DurationBucket` 三个空位是否一起填、聚合行落在哪张表、怎么挂 content key（G12）。
3. **给每个候选算法标注「缺席假设」**（G3）。同一个 next-app 模型，在「缺席=闲置」和
   「缺席=不可观测」两个假设下是两个模型。本轮的结论是：**在 `sessions_discarded` 不落库之前，
   只有后一个假设是诚实的。**

## 5. 后续怎么证伪本文

本文全是可查事实，逐条给出证伪方式：

- 「app 与 duration 都在密封体」→ 读 `session.rs::SealedSessionBody::FIELDS` 与
  `sql.rs` 的 `events` 建表语句；若 `events` 出现 app 或 duration 列，本文第 0 节作废。
- 「三个空位无写者」→ `rg 'EvidenceKind::AppUsage|EvidenceKind::Aggregate|Derivation::Aggregate' crates/*/src/`
  今天只命中 `soul-algo-trait/src/types.rs` 的四行（枚举成员与 `as_str`），无构造点；
  `rg 'duration_bucket' crates/*/src/` 今天命中四行且全为声明/判存/透传。任一处出现赋非空值即为反例。
- 「唯一时间聚合是 UTC 小时事件计数」→ `rg -i 'GROUP BY' crates/*/src/` 今天只有两处命中：
  `research_preview.rs::EVENT_ROLLUP_SQL` 与 `a1.rs` 里一句注释（A1 的 `(kind, utc_day)` 分组是
  内存里做的，不是 SQL）。出现第三处即为反例。
- 「全仓无 day-of-week」→ `rg -i 'day_of_week|weekday|\bdow\b' crates/ apps/` 今天为空。
- 「采集侧无本地偏移」→ `clock.rs::rfc3339_utc` 只产 `Z`；`ForegroundSession` 三字段无偏移。
- 「`Tally.sessions_discarded` 不落库」→ `collector.rs` 里 `Tally` 只经 `tally()` 返回，
  审计只写 `AuditCounts.items = events_written`。
- 「`TieScore.detail` 未落库」→ 比对 `soul-algo-tie::TieScore` 与 `soul-graph::TieStrength` 字段集，
  差集含 `detail`。
- 「`Band` 无 `Ord`」→ `soul-algo-trait/src/types.rs` 的 derive 列表与其注释。
