# VAL-01 合成评测适配设计草案

状态：**设计草案；实施保持 BLOCKED_BY_GOAL1_AND_Q2-00**。作者于 2026-10-19 选择保留门禁边界、先推进后续方案设计。本页细化 [ROADMAP 7.2](../ROADMAP.md#72-首轮个性化试验) 与 [GOAL2_PLAN 的 VAL-01 交接](../GOAL2_PLAN.md#val-01-评测适配交接非-q2-关闭包)，不改变其进入条件、样本登记或指标口径，不新增产品功能、冻结 contract、baseline 或 gate。

目标是让后续评测如实回答：实际用了哪种 brief、哪些候选失败或降级、哪些配对可比较、结果分母是什么。本页没有采集真实反馈，也没有执行评测适配代码。源码可行性核于 `629c739`。

## 已有能力与需要补的差集

| 已有入口 | 已证明或支持的能力 | 后续适配应补的内容 |
| --- | --- | --- |
| `soul-draft/src/brief.rs` 的 `ProfileBrief::neutral/from_view/new/set_by_user` | 能显式构造 A/B/C；`from_view` 会返回错误；手工 voice 可不携带轴 readings | 保留 requested / actual brief 身份，B 构造失败不得标为 neutral 成功 |
| `soulcore/tests/q2_continuity.rs` | 多次重开、独立库、三路模板对照及纠正连续性 | 复用为接线证据，不重复抄写；不从相同模板推断用户偏好 |
| `soulcore/src/commands/draft.rs` 的 `DraftSession` | `DraftRequest` 可带明确 brief；prepare 与 generate 分离，现有单次批准、redactor 和 exact origin 可复用 | A/B/C 的合成领域适配，不为实验新增 Session/UI 任意 brief 注入接口 |
| Session 及已有 E1/恢复测试 | 实际库、来源失效、审计、工作槽和重启边界 | 最小桥接检查证明 B 来自实际 profile；领域适配不冒充完整 Session 或真实 UI |
| ROADMAP 的 Run / Pair / Candidate 登记 | 已规定用途、状态、分母和排除规则 | 类型化的测试数据、合成槽位及可重算汇总；缺测仍有原因，不填零 |

当前 Session 的方便起草入口不是 A/B/C 实验注入面。适配必须显式构造 B，并保留失败，不能从最终返回了模板倒推 brief 成功。T1 只读取四项 voice 和 turn 数；T2 不见原始 paste；只有 T3 可用于依赖消息正文的内容相关性评价。新测试不得把这三种任务混为同一种生成能力。

## 路线比较与推荐

| 方案 | 收益 | 成本与限制 | 本草案选择 |
| --- | --- | --- | --- |
| 仅在现有测试旁记录人工汇总 | 无产品改动，准备成本低 | 无法稳定重算排除条件，容易把 B 回退和降级当成功 | 保留人工实际评分，但不单靠手工算表 |
| 测试专用领域适配 + 少量 Session 桥接 | 复用真实 brief、policy、redactor、HTTP 和持久化，能核验分母 | 不提供实际盲评 UI，不证明真人效果 | 推荐首片；限合成输入和 loopback |
| 新增产品评测页面、任意 brief 注入或独立评测服务 | 可直接组织真人流程 | 增加 UI/IPC/持久化/许可面，越过当前交接范围 | 本阶段不选 |

首片不引入依赖、业务表或另一套算法。类型只存在于测试模块；枚举沿用 ROADMAP 登记，既有产品值通过明确转换映射。没有真实用户时，仅测试预设合成反馈的算术与流程，不能将其写成“用户喜欢 B”。

## 候选构造与执行边界

1. 分配槽位前读取合成 Run 的状态；不满足 granted 或已到预先截止时不启动新的候选。真实 Run 的同意收集不由此自动实现。
2. 在看到候选前固定 Pair 的 contrast、左右 variant、任务类型、输入卡和运行条件。A 使用 neutral；B 显式 `from_view`；C 通过四个已登记的 `VoiceSetting` 构造，禁止自由文本、轴 readings 或事后调参。
3. B 错误保存 `brief_build_status=failed`、`actual_brief_state=unknown`，不调用生成。T1 同 voice 的配对标 `not_comparable`；不通过微调语气制造可比较样本。
4. T1 走真实本机模板。T2/T3 使用 `DraftSession`、`PolicySession` 与 loopback MockLlm，分别准备、检查 exact plan、批准和生成。`DraftSession` 与 `PolicySession` 使用同一 `KnownIdentifiers` 集合；T3 合成卡默认不含普通姓名，需测试姓名时按既有规则登记并验证脱敏。T3 的一次性豁免只属于对应候选；下一次 T2 必须恢复占位。不绕过 policy 直接发 HTTP。
5. 从实际 `DraftSource` 和已存在的降级结果映射 Candidate。拒绝、网络错误、不可读/空/禁止回复所致降级单列，不纳入正常端点胜负。每个候选都断言 `delivery=false`。
6. 只把计数、枚举和原因送给汇总器。合成输出正文只供当前测试断言；不写产品审计或提交到评测结果文件。真实任务的内容评分必须人工提供，不能从模型自评、点击或“生成成功”补造。

配对左右使用两个独立的 `DraftSession` / `PolicySession` 和相同运行配置；各自的批准不能交叉使用。同一用户 profile 读取成一个明确快照，构造失败影响对应 Pair 的可比较性，不能让另一边悄悄使用不同时间的状态。T1 判断 voice 相同只比较四个生效枚举，不能把 `user_set` 元数据差异当作输出差异。

Run、Pair、Candidate 字段仍以 ROADMAP 表为唯一登记。本页不复制完整字段表，也不额外生成 schema lock。具体实现采用测试内普通 Rust 类型和唯一 Pair/Candidate 标识关联；若关联缺失、重复或左右 variant 与 contrast 不符，返回结构化验证错误，不从缺失记录生成零值或默认候选。验证器还核对 Pair completed 与双方 Candidate completed 一致；T2 的 unsupported claim、B 构造失败不能手工标为正常完成，端点降级必须保留实际来源、原因并与正常端点评分分层。P7 等尚未开始的计划候选允许按 missing_reason 保留未填字段，不当作损坏记录；只有实际进入构造/生成/评分阶段才要求对应字段。

## 汇总与反例

Pair 漏斗、Run 撤回、偏好及耗时分别重算。`assigned` 不等于已开始；生成成功不等于评分或完成。模板与端点、T1/T2/T3、模型条件和参与范围分层；不把单作者多任务当多个参与者。

合成汇总夹具采用下表的预设元数据，**不是实际效果数据**。P1–P4、P7 属于同一个合成 Run 的同条件 T2/AB；P5 是 T1/AC；P6 是 T3/BC；P8 属于另一已撤回的合成 Run。

| Pair | 开始 | 两边生成 | 评分及可比较性 | Pair 完成 | 需要保留的解释 |
| --- | --- | --- | --- | --- | --- |
| P1 | 是 | 是 | B 胜，可比较、盲法完整 | 是 | A=0 秒、B=10 秒；零值保留，但不计算比例 |
| P2 | 是 | 是 | 平局，可比较、盲法完整 | 是 | A=120 秒，B=90 秒，改善比例 0.25 |
| P3 | 是 | 是 | A 胜，可比较、盲法完整 | 是 | A=80 秒，B=100 秒，改善比例 -0.25 |
| P4 | 是 | 否 | unrated | 否 | A 生成且完成，B 网络失败；B 完成耗时缺失，不填 0 |
| P5 | 是 | 是 | 已填写偏好，但 T1 两边 voice 相同 | 是 | not_comparable；仍保留完成与流程计数 |
| P6 | 是 | 是 | 已填写偏好，但盲法 broken | 是 | 不进偏好分母；T3 事实错误计数单列 |
| P7 | 否 | 否 | unrated | 否 | 已分配但未开始 |
| P8 | 是 | 否 | unrated | 否 | Run withdrawn，停止后续候选 |

八对有十六个计划 Candidate，逐边元数据如下。下表 `—` 是未测/未到该阶段，并带最后一列原因，不能转换成 0；`N/A` 是该任务明确不适用。已生成 T2/T3 均为未降级 user_endpoint，T1 为 tone_template；已生成项均 brief built、delivery=false、privacy_or_safety_stop=false。未生成项按既有登记为 source=none、degraded=not_generated，failure_code 保存脱敏原因；尚未开始的 brief 字段保留阶段性缺失。

| Pair / variant | generated | Candidate 完成状态 | 首稿接受 | edit_seconds | unsupported_claim | required_fact_error_count | 缺失/失败原因 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P1 A | true | completed | true | 0 | false | N/A | none |
| P1 B | true | completed | false | 10 | false | N/A | none |
| P2 A | true | completed | false | 120 | false | N/A | none |
| P2 B | true | completed | true | 90 | false | N/A | none |
| P3 A | true | completed | true | 80 | false | N/A | none |
| P3 B | true | completed | false | 100 | false | N/A | none |
| P4 A | true | completed | true | 30 | false | N/A | none |
| P4 B | false | system_failure | N/A | — | — | N/A | transport_failure；未生成，未评分 |
| P5 A | true | completed | false | 60 | N/A | N/A | none；模板正文槽已由合成评分者填写 |
| P5 C | true | completed | false | 60 | N/A | N/A | 同上 |
| P6 B | true | completed | true | 20 | false | 0 | none；Pair 破盲只排除偏好 |
| P6 C | true | completed | false | 40 | false | 1 | none；合成评分者已修正事实后完成 |
| P7 A | false | incomplete | N/A | — | — | N/A | not_started |
| P7 B | false | incomplete | N/A | — | — | N/A | not_started |
| P8 A | false | incomplete | N/A | — | — | N/A | consent_withdrawn_before_generation |
| P8 B | false | incomplete | N/A | — | — | N/A | consent_withdrawn_before_generation |

P8 固定为 T2/AB，Pair aborted；两边均未生成。所有已完成 Candidate 的 pause_seconds=0、help_count=0 是显式合成值；未完成项保持缺测及原因。P4 的 B brief 已构造但请求失败，P7/P8 未构造 brief。P6 的事实错误计数评价生成首稿，完成状态评价人工编辑后的任务，两者不相互覆盖。

预期的普通测试断言：assigned=8、started=7、generated=5、rated=3、completed=5；withdrawn Run=1，不能拼到 Pair 漏斗末尾。总体完成率=5/8；首个 Run 的 T2/AB 完成率=3/5。该层有效偏好分母=3，B 得分 `(1+0.5+0)/3=0.5`；P4/P7 仍进入分配/失败分母。定义耗时改善差为 A−B，P1/P2/P3 分别 −10、30、−20 秒，中位差 −10 秒；改善比例为 (A−B)/A，P2/P3 为 0.25/−0.25，中位比例 0。P1 保留绝对秒数但排除比例，P4 不进入成对耗时。缺失秒数与零秒数必须不同。

Candidate 总生成数=11。按未降级 endpoint / T2 / variant 分层，A 首稿接受=3/4、B=1/3；T1 模板 A=0/1、C=0/1；T3 endpoint B=1/1、C=0/1。各层单列，不合并成“11 项用户效果”；破盲 P6 不进偏好分母，但已生成候选仍进入其来源/任务/条件的接受率分母。撤回 Run 无已生成候选，本夹具不替代对“撤回前已评分如何排除”的另一个专门反例。

另增降级反例：端点请求已发生但返回本机模板时，`generated` 与具体结果来源按登记如实保存，不能纳入未降级端点偏好或 T3 正确率；只有一边生成的 Pair 不进 generated。未暴露的模型版本和服务参数保持 unknown/not_exposed，不捏造相等条件或可重复性。

## 后续实施拆分（尚未启动）

所有路径均为未来认领建议；本次不创建这些源文件。每一片先写可失败的行为测试、运行、实现最小测试适配，再独立 review 和提交。若首次运行已绿，只算已有能力的覆盖，不编造业务红绿修复。

| 片 | 拟写路径与职责 | 消费 / 产出 | 最小反例与通过证据 |
| --- | --- | --- | --- |
| V1 记录一致性与汇总 | `crates/soulcore/tests/val01_evaluation.rs` 入口；`tests/val01_evaluation/records.rs`、`pairs.rs`、`aggregate.rs` | 消费 ROADMAP 登记和固定合成元数据；产出无正文的分层计数、分母及缺失原因 | 上表逐边手算、重复/缺失关联拒绝、完成状态一致、左右交换、unrated/not_comparable/broken/withdrawn、A=0与缺测分开、模型变更不合并 |
| V2 brief 与 T1 适配 | `tests/val01_evaluation/briefs.rs`、`paths.rs` 的本机路径 | 消费公开 ProfileBrief API、合成 ProfileView 和 V1 typed Candidate；产出显式 brief 身份与本机结果 | B 失败不回退、C 无 readings、T1 同枚举不可比较、模板正文槽不伪称首稿接受 |
| V3 T2/T3 与 Session 桥接 | `tests/val01_evaluation/paths.rs` 的端点路径；入口只注册模块 | 消费 V1/V2；产出实际 source/degraded/refusal/privacy shape | 未批准零请求、错批准/重放拒绝、T2 占位、T3 仅单次原文、降级不当端点成功；一条真实 Session/SQLCipher 桥接 B 的资料来源和审计边界 |

V1→V2→V3 串行集成，先独立核验记录和算术，再连接生成路径，避免每次分母修订都触发 HTTP 或数据库测试；测试模块有一个实现 owner，独立 reviewer 只读。现有 q2_continuity/q2_recovery 的功能不移植或删除，不为评测便利修改 Session、IPC、schema、锁文件或已有算法。若既有公开 API 无法满足任一断言，先回报具体差距并形成新的认领，不能在测试适配里绕过安全边界。

首次实现才运行下列定向命令；当前未创建测试文件，因此本次不把命令记录为通过：

```pwsh
$ErrorActionPreference = 'Stop'
cargo test -p soulcore --test val01_evaluation --locked --offline
if ($LASTEXITCODE -ne 0) { throw 'VAL-01 合成适配失败' }
cargo test -p soulcore --test q2_continuity --test q2_recovery --test session_e1 --locked --offline
if ($LASTEXITCODE -ne 0) { throw '既有连续性/端点回归失败' }
```

不因为纯文档或测试专用元数据而自动重跑完整打包；正式集成仍使用既有门禁入口。若实现触及产品权限、数据格式、跨系统交互或发行边界，按原规范决定所需验证，不新增“评测通过即发布”的规则。

## 进入实施与真实试用之前

- 本草案可先独立 review；设计完成不把任何 Q2/Goal 或 U7 标为关闭。
- 实施仍需要 Goal 1 按 ACCEPTANCE/STATUS 正式关闭、Q2-00 给出可比较实现输入以及可用构建。用户本次没有解除 G-L 暂缓、系统 G-M 后置和真实效用未测状态。
- 真实阶段需要单独的产品评测同意、开始/截止、撤回及汇总处理说明、固定任务槽与人工盲评安排；外部端点每次仍按既有授权。尚未提供这些信息时只保留合成设计，不联系参与者、不启动真实试验。
- v0.2 DATA/MODEL、可逆文件动作、多端与云端保持 ROADMAP 各自的设计与前置，不因为本页的评测类型而获得数据、权限或产品实现授权。

本页只提交设计和源码可行性分析。拟新增类型、验证器、汇总器和命令均未实施/执行；示例数字是独立手算的测试期望，不是测量结果。
