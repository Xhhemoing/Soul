# VAL-01 合成评测适配设计与实施

状态：**D101 的 V1–V3 合成适配已实现，分片结果见本页后续记录**。作者于 2026-10-19 先选择保留门禁边界、推进设计，随后要求按核心计划逐部分分析、编码、review 和提交。本页细化 [ROADMAP 7.2](../ROADMAP.md#72-首轮个性化试验) 与 [GOAL2_PLAN 的 VAL-01 交接](../GOAL2_PLAN.md#val-01-评测适配交接非-q2-关闭包)；D101 仅提前测试专用适配，正式阶段及真实试验前置、样本登记和指标口径不变，不新增产品功能、冻结 contract、baseline 或 gate。

目标是让后续评测如实回答：实际用了哪种 brief、哪些候选失败或降级、哪些配对可比较、结果分母是什么。没有采集真实反馈。原设计源码可行性核于 `629c739`，D101 实施起点为 `5821f29`；设计中的手算数字不作为执行结果。

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

合成适配在生成前保留普通预分配槽位记录，顶层验证按 Run/task_slot 核对 Pair 的身份、任务、左右映射、种子和条件；不是新增冻结 contract 或 hash。发生安全暂停的 Pair 保留历史流程、生成和安全计数，同时单列安全排除数；该 Pair 双方不进入本次待汇总的偏好、耗时、首稿接受和事实正确效用分母，避免将受影响配对的对侧当正常效果。此规则不删除同 Run 无关历史数据，也不授权继续暂停路径；真实试验仍须另行满足原开始与撤回安排。

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

## 分阶段实施拆分（D101）

每一片先写可失败的行为测试、运行、实现最小测试适配，再独立 review 和提交。若首次运行已绿，只算已有能力的覆盖，不编造业务红绿修复。协调者独占构建与提交，实现者只改认领路径；沿用现有干净开发分支及缓存，不另建流程目录。

| 片 | 拟写路径与职责 | 消费 / 产出 | 最小反例与通过证据 |
| --- | --- | --- | --- |
| V1 记录一致性与汇总 | `crates/soulcore/tests/val01_evaluation.rs` 入口；`tests/val01_evaluation/records.rs`、`pairs.rs`、`aggregate.rs`、`fixtures.rs`、`record_tests.rs` | 消费 ROADMAP 登记和固定合成元数据；`validate` 产出已验证记录，`summarize` 产出无正文分层计数；V2/V3 填同一 Candidate | 上表逐边手算、重复/缺失关联拒绝、完成状态一致、左右交换、unrated/not_comparable/broken/withdrawn、A=0与缺测分开、模型变更不合并 |
| V2 brief 与 T1 适配 | `tests/val01_evaluation/briefs.rs`、`local.rs`、`local_tests.rs` | 消费公开 ProfileBrief API、合成 ProfileView 和 V1 typed Candidate；产出显式 brief 身份与本机结果 | B 失败不回退、C 无 readings、T1 同枚举不可比较、模板正文槽不伪称首稿接受 |
| V3 T2/T3 与 Session 桥接 | `tests/val01_evaluation/endpoint.rs`、`endpoint_tests.rs`、`endpoint_transport.rs`、`session_bridge.rs`；入口只注册模块 | 消费 V1/V2；产出实际 source/degraded/refusal/privacy shape | 未批准零请求、错批准/重放拒绝、T2 占位、T3 仅单次原文、降级不当端点成功；一条真实 Session/SQLCipher 桥接 B 的资料来源和审计边界 |

V1→V2→V3 串行集成，先独立核验记录和算术，再连接生成路径，避免每次分母修订都触发 HTTP 或数据库测试；测试模块有一个实现 owner，独立 reviewer 只读。现有 q2_continuity/q2_recovery 的功能不移植或删除，不为评测便利修改 Session、IPC、schema、锁文件或已有算法。若既有公开 API 无法满足任一断言，先回报具体差距并形成新的认领，不能在测试适配里绕过安全边界。

各片实施后运行下列定向命令，保留实际输出与退出码；计划不作为通过证据：

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
- D101 仅允许 V1–V3 合成测试适配提前实施；正式阶段仍需要 Goal 1 按 ACCEPTANCE/STATUS 关闭、Q2-00 给出可比较实现输入以及可用构建。用户没有解除 G-L 暂缓、系统 G-M 后置和真实效用未测状态。
- 真实阶段需要单独的产品评测同意、开始/截止、撤回及汇总处理说明、固定任务槽与人工盲评安排；外部端点每次仍按既有授权。尚未提供这些信息时只做合成适配，不联系参与者、不启动真实试验。
- v0.2 DATA/MODEL、可逆文件动作、多端与云端保持 ROADMAP 各自的设计与前置，不因为本页的评测类型而获得数据、权限或产品实现授权。

本页的示例数字是独立手算的测试期望，不是测量结果。实施结果按 V1/V2/V3 分片追加，真实任务样本数仍为 0。

## V1 实施与验证（2026-10-19）

候选为 `b483f80` 加 V1 dirty 源码，随后由包含本节的提交收录。新增入口及 records/pairs/aggregate/fixtures/record_tests 六文件，普通类型记录 Run、预分配槽、Pair 与 Candidate；顶层验证关联、阶段、实际 brief/source/拒绝/安全形状后才汇总。P1–P8 的 16 候选与独立 30 槽作者计划分开；缺测保留原因，撤回/破盲/不可比较/降级不伪造正常效果，失败不从原计划完成率分母移走。

初始 15 条行为测试因未实现而失败，完成后通过；槽位绑定再经 1 条行为红绿。独立规格/质量 review 找到失败/安全状态可混入效用、槽位 helper 未被顶层强制消费两项，新增 4 条反例实跑 **17 通过 / 4 失败**；修复后 **21 通过、0 失败**，严格定向 Clippy 退出 0，并已独立复审修复，无阻塞发现。Clippy 最初两个命名告警用等价重命名修复，未添加 lint 豁免；仅未使用的登记枚举变体有带解释的 dead-code 属性。

实际命令为 `cargo test -p soulcore --test val01_evaluation --locked --offline` 与 `cargo clippy -p soulcore --test val01_evaluation --locked --offline -- -D warnings`；根格式及 diff 检查退出 0。Rust 1.83 / MSVC / Windows，既有静态 OpenSSL、jobs=4、debug=0、incremental=0；未改全局环境、target、依赖锁或生产代码。原始证据在仓库外 `E:\Project\Soul-checks-20261019-core-plan\`，包括 `environment.json`、`v1-red`、`v1-slot-red`、`v1-review-red`、`v1-review-green`、`v1-review-clippy` 的日志及退出文件。前期和最终重复运行不累计为更多测试。

本片只证明记录与合成算术，尚不证明真实 brief、模板、HTTP/Session 接线或真人效果；V2/V3 继续按计划串行推进。不签发完整平台门禁，不关闭 Goal 1、正式 Goal 2 或 U7。

## V2 实施与验证（2026-10-19）

起点 `068295c`，新增 briefs/local/local_tests 三模块并注册入口。A 用 neutral，B 显式 `from_view`，失败不回退；C 仅四个预先选择的语气枚举，不带 readings。配对借用同一 ProfileView，T1 比较四个生效枚举而非 `user_set` 或轴。准入检查已登记槽位、Run 的许可/起止、重复身份及同 Run/path 的安全暂停；拒绝发生在分配、构造和真实 `DraftSession::draft_offline` 调用之前。

真实本机返回被映射进 V1 Candidate，并通过顶层验证/汇总；输出正文只在当前断言中，不进入记录。生成仍为 incomplete，偏好 unrated，首稿接受、耗时和帮助次数保持缺测。本机模板以 request.context 的入站轮次选择话术，但返回默认 BodyFacts，出口 `third_party_turns/placeheld_turns=0`；初次实跑的 1 项失败来自新测试误将两种计数混同，修正预期与实际映射断言，未修改生产逻辑。

新增 10 项先得到 **21 通过 / 10 TODO 失败**，实施后首次 **30 通过 / 1 失败**，最终 **31 通过、0 失败**；严格定向 Clippy、根格式和 diff 检查均退出 0。命令与环境沿 V1，证据在同目录 `v2-red`、`v2-green`、`v2-verified`、`v2-clippy` 日志及退出文件。完成独立只读规格/质量审查，无阻塞发现。该时点 V3 的 HTTP/Session 桥接尚未完成，后续结果见下一节；真实同意收集、原生 UI 和真人效果均未测。

## V3 实施与验证（2026-10-20）

起点 `8d872e3`，新增 endpoint、endpoint_tests、endpoint_transport、session_bridge 四模块，入口注册及记录枚举作最小扩展。复用真实 DraftSession/PolicySession，配对固定资料快照、标识脱敏配置与 endpoint origin；每候选准备和派发均检查 Run、登记身份和同路径安全暂停。未批准、错批、跨候选批准、重放及改址用实际请求数核对；T3→T2 保留同一组会话与令牌记录，核对一次原文授权不会延续。拒绝、真实 HTTP 503、跨 origin 重定向、不可读/空/临床回复的模板降级分别入记录，不补人工评分。

合成端点只接收字面 loopback HTTP 地址，不做外部 DNS；补充端点采用完整 Content-Length、总读取期限和退出唤醒。退出时可能在唤醒前关闭 listener 的竞争是静态发现并修正，未声称实跑复现。Session 桥接使用临时 SQLCipher 与合成导入，显式 `ProfileBrief::from_view` 的 B 与实际 Session 请求一致；语气纠正重开保持，审计不带正文。未修改生产代码、产品 UI/IPC、依赖或锁文件。

初始 `v3-red` 为 **31 通过 / 8 TODO 失败**；首轮实现编译因 ForgetState 导入路径错误失败，未执行测试，修正后 `v3-green2` **43 通过**。独立 review 发现不同 endpoint 可继续生成可比较配对、失败候选可重试覆盖原失败两项，三反例 `v3-review-red` 实跑 **45 通过 / 3 失败**。修复后 `v3-final-tests` 为 **48 + 34 通过**；针对性复审继续找到截止/撤回准入能覆盖已终止失败，`v3-terminal-red` 两条定向反例均失败。终止检查现集中在已验证身份之后、任何准入记录修改之前；重试需新登记槽位，错批后仍可使用正确批准，成功后的重放仍由真实 DraftSession 拒绝。

最终开发验证（`8d872e3` 加本片 dirty 源码）：

| 实际命令 | 结果 | 仓库外证据 |
| --- | --- | --- |
| `cargo test -p soulcore --test val01_evaluation --test q2_continuity --test q2_recovery --test session_e1 --locked --offline` | **50 + 4 + 3 + 27 = 84 通过**，退出 0 | `v3-verified-tests.log` / `v3-verified-tests-exit.txt` |
| `cargo clippy -p soulcore --test val01_evaluation --locked --offline -- -D warnings` | 退出 0 | `v3-verified-clippy.log` / `v3-verified-clippy-exit.txt` |
| `cargo fmt --all -- --check` | 退出 0 | `v3-verified-fmt.log` / `v3-verified-fmt-exit.txt` |

日志均在 `E:\Project\Soul-checks-20261019-core-plan\`；沿 V1 的 Rust 1.83 / Windows MSVC / 既有静态 OpenSSL 环境，实际调用与退出记录补入 `environment.json` 的 `final_verification`。50 项合成适配由 V1 21、V2 10、V3 19 项组成；重复运行不累加。独立规格/质量 review 及针对性修复复审已完成，无阻塞或非阻塞缺陷发现；最终 reviewer 只读核验原始红绿输出与必要上下游，未自跑构建。side 索引安全有前置 `.get(side)` 静态证据，未另设专门越界运行例。

本片只交付合成适配与开发验证：真人样本仍为 0，未运行真实外部模型、原生 UI、Linux G-L、完整 G-W/G-M 或安装卸载；未改变正式 Goal 1/Q2/U7 的关闭状态。DATA L1 仅完成设计收敛，生产恢复器仍不在本轮范围内。

## 任务比较材料与有限干跑准备（2026-10-20）

本段对应[下一阶段计划 3.2](2026-10-20-next-stage-plan.md#32-val让比较过程能够被执行和解释)的已认领材料工作。核对起点为 `6a56d852d23a051c7c4e9845642791295c541105`，工作区已有配置/文档整理 dirty；本次仅追加本页。指标、字段和正式启动条件仍以 [ROADMAP 7.2](../ROADMAP.md#72-首轮个性化试验) 为唯一口径。下列新卡是合成材料，**尚未作为新输入执行，未开始真人试验**。

### D102 时点的入口核对与不新增胶水的理由

已有 `local_tests::real_template_observations_validate_and_summarize_without_human_scores` 直接调用 `run_local_pair → validate → summarize`，真实本机输出与未评分状态在同一测试内核对；`endpoint_tests::endpoint_t2_pair_builds_actual_briefs_and_records_unrated_outputs` 对 loopback 端点做对应核对。`record_tests` 使用独立的预设反馈 fixture 检查分母及排除。它们已经覆盖本轮需要的有限干跑，不新增 runner、统计副本或 `dry_run.rs`。

该时点入口是 Cargo integration test `val01_evaluation`，不是产品按钮、通用命令行录入工具或人工评分 UI。该时点 `Summary` 是内存结果，没有报告文件导出器；后续离线元数据入口见[计时缺测与人工元数据入口](#计时缺测与人工元数据入口)。测试通过日志证明断言被执行，下面表格提供不带正文的可重算解释。不能把测试通过日志称为已经完成了人工比较。现有 loopback 固定回复也不证明下面 T3 卡的语义任务被完成。

| 核验目的 | 已有测试名（同一 target 内） |
| --- | --- |
| 本机真实生成、未评分与汇总 | `local_tests::real_template_observations_validate_and_summarize_without_human_scores` |
| 端点真实生成、未评分 | `endpoint_tests::endpoint_t2_pair_builds_actual_briefs_and_records_unrated_outputs` |
| T1 四项 voice 相同 | `local_tests::identical_effective_voice_marks_the_executed_t1_pair_not_comparable` |
| 未配置/改址拒绝及实际降级/传输失败 | `endpoint_tests::endpoint_unconfigured_and_changed_origin_are_distinct_refusals`、`endpoint_tests::endpoint_reply_defects_keep_actual_degradation_and_transport_failure` |
| 预设反馈、缺测与零秒区分 | `record_tests::full_fixture_retains_failures_and_literal_pair_denominators`、`record_tests::zero_seconds_remains_observed_while_incomplete_pair_is_excluded_from_time` |
| 撤回与安全暂停 | `record_tests::withdrawn_after_scoring_keeps_flow_but_excludes_all_pending_utility`、`local_tests::safety_stop_pauses_the_same_run_path_without_globally_pausing_other_paths` |

主线程按唯一构建通道运行下列现有 target，一次覆盖上述入口；本段初稿编写时未运行，随后实际回执见本节末尾。

```pwsh
$ErrorActionPreference = 'Stop'
cargo test -p soulcore --test val01_evaluation --locked --offline
if ($LASTEXITCODE -ne 0) { throw 'VAL 有限合成干跑失败' }
```

本轮不改生成/授权代码；因此材料本身不要求另增端点或 Session 测试。新卡若将来需要自动执行，须另认领测试适配输入；当前 UI 无任意 A/B/C brief 注入面，不能由人工操作现有 UI 假称已经控制了 A/B/C。

### 可交给执行者的五张合成卡

共同约束：卡中所有消息均虚构，不含普通姓名、真实账号、物理地址或联系人资料；同一 Pair 左右使用完全相同的卡和 paste。A/B/C 唯一差异按既有 brief 定义。评分者可以看任务卡；**生成器允许看到的材料**按下表限制，不能把评分提示偷加到 T1/T2 的输入中。均只起草不发送。截止时间由 Run 登记，未完成保留原因，不另设单卡通过阈值。

| 卡 / 类型 | 固定输入 paste | 生成器允许材料 | 必需事实或限制、完成定义 | 不适用评分 |
| --- | --- | --- | --- | --- |
| S1 / T1 / `not_applicable` | `合成消息：请先确认是否收到，具体安排之后再讨论。` | 四项生效 voice 与入站 turn 数；本机模板不消费正文或轴 readings | 评分者补全正文槽为通用确认，保留“安排尚未确定”，然后认可整体语气。衡量语气和补全成本；正文槽未补全不得接受首稿 | 不评生成器理解原文、事实正确或内容相关性；`unsupported_claim`、`required_fact_error_count` 均 N/A |
| S2 / T2 / `not_applicable` | `合成消息：请在周五之前确认是否参加周六的讨论，时段尚未确定。` | 相应 brief 和第三人占位符；不见这段 paste | 认可一条通用确认/占位回复；首稿不得自行给出日期、参加决定、具体时段或承诺。出现任何消息特定事实/承诺记 `unsupported_claim=true`，本候选不完成，不能借评分者知道原文判其正确 | 原文事实正确率与消息特定内容相关性 N/A；`required_fact_error_count` N/A |
| S3 / T3 / `coordination` | `合成任务：拟回复协调邀请。周五下午不能参加；周六上午可以；请对方确认是否方便。地点还没有确定，不要承诺地点。` | 相应 brief；仅该候选经二次确认和 exact-plan 单次批准的脱敏原文 | 必需项：周五下午不可、周六上午可、请求确认、地点未定/不承诺地点。首稿逐项核对；人工修正后可完成，但首稿错误计数保持。不得声称双方已经约定 | 只有未降级 endpoint 进入事实正确率；模板降级不评 T3 正确率 |
| S4 / T3 / `boundary_refusal` | `合成任务：婉拒临时协助。今晚不能接手这项工作；本周也不能承诺代办；希望礼貌说明边界，不提出替代完成日期。` | 同 S3，一次授权只覆盖此候选 | 必需项：今晚不接手、本周不承诺代办、无替代完成日期；礼貌但不把拒绝改成答应，不虚构原因。评分者认可内容与语气或修正到认可 | 同 S3；不把婉拒任务误记为系统 `refusal_code` |
| S5 / T3 / `relationship_followup` | `合成任务：跟进一次未完成的讨论。上次讨论暂停了，目前没有确定下一步；想询问对方是否愿意继续；如果暂时不想继续，也可以稍后再说。不要推断对方情绪或关系状态。` | 同 S3 | 必需项：讨论曾暂停、下一步未定、询问继续意愿、允许暂缓；不新增期限、情绪判断或关系结论。评分者认可或在截止前编辑到认可 | 同 S3；不评价人格、心理或关系真值 |

T3 的必需事实错误逐项计数：一项遗漏或反转计该项错误一次；重复表述同一项错误不重复累计。额外捏造按 `unsupported_claim` 另记，不把新增承诺藏进零错误结论。计数评价首稿，完成状态评价最终经人工认可的草稿；记录只保留计数与类别，不保存首稿或编辑正文。T1/T2 的人工补全不能追溯成模型已经理解正文。

### 配对前登记与人工执行顺序

以下是未来满足正式前置后的执行说明，不是当前真实试验授权。测试适配不能直接拿去访问真实 endpoint；仍拒绝一切非字面 loopback origin。真实比较必须先另行准备能够遵守相同授权和记录边界的执行入口；缺少该入口时 T2/T3 写 NOT RUN，不修改测试安全限制。后续 D104 的[合成生成与盲呈现接线](#d104-合成受控生成与盲呈现入口)仅演练该协议，不替代真人界面或正式前置。

1. 在首个输出前，填 Run 的匿名 ID、实际开始/截止、参与范围、产品评测用途和主动同意状态；先向参与者说明撤回后停止任务、保留哪些最小流程计数，以及尚未汇总的效用评分如何排除。没有这项说明与同意不启动。作者方案为连续 14 个自然日、至少 30 槽；其他志愿者按 ROADMAP 的 5–8 位成年人、每人 10–15 对执行，均为探索假设。
2. 作者的最小 30 槽可直接复用 `pairs::author_slots(seed)` 并经 `validate_author_slots` 核验，登记整数 seed 和每槽实际左右映射。现有算法产生 AB=15、AC=8、BC=7，T1/T2/T3 各 10；每个 contrast 的左右数量相差至多 1。按 T3 出现顺序固定 S3/S4/S5 循环，得到 4/3/3 层，T1 用 S1、T2 用 S2。`author_slots` 不负责 semantic stratum 或卡选择；这些字段须在开始前补到登记中，不在看过输出后换卡。该重复卡安排只作有限流程准备；不能把重复任务当独立人群样本。
3. 每 Pair 固定 B 的一个资料快照和 C 的四个枚举值；C 可填 `register=__ / directness=__ / warmth=__ / emoji_use=__`，仅用 ROADMAP 允许值，不含自由文本或 readings。登记相同路径、endpoint、脱敏方式、模型/版本/参数和上下文上限；未知保持 `unknown/not_exposed`。保存实验映射的操作者与盲评分者分开；作者无法保持盲法时如实记 `broken`。T1 四项相同直接记 `not_comparable`，不事后改 voice。
4. 构造前建 Pair 与双方 Candidate 行。操作者按登记逐边构造、准备、核对 exact plan 并各自批准；批准不可交叉或重用。B 构造失败保留 unknown/failed；不借 neutral 回退冒充 B。逐边记录实际 source、degraded、refusal 与隐私形状。正文仅临时呈现，映射和来源不提前给评分者；不在日志或仓库留正文。
5. 每次只呈现登记的一对“左/右”，先看原始候选并填写 `left/right/tie/unrated`，记录是否破盲，之后逐边编辑到认可。耗时按该边开始展示到认可记录实际秒数，另一边等待和暂停分别排除，暂停秒数与帮助次数独立记；没有计时就保留缺测。T1 必须补全正文槽，T2 检查 unsupported claim，T3 按卡核首稿事实与额外捏造。没有人工评分，生成结果保持 `unrated/incomplete`。
6. 评分与完成记录固定后才揭盲，按左右映射转换到 AB 的 B、AC 的 C、BC 的 B；先按每位参与者/Run、任务、来源和模型条件计算，再报告参与者中位数。揭盲后不重填偏好以追阈值。盲法破坏排除偏好但不删除已观察耗时和流程，具体排除仍由现有汇总器执行。
7. 到截止立即收口。拒绝、降级、单边失败、未开始/未完成均保留原槽与原因；没有端点记 NOT RUN，不补造评分、不延长或补样本追线。撤回即停后续候选，历史流程留存、待汇总效用按预先说明排除。隐私越权、泄漏、纠正静默覆盖或遗忘后可读立即暂停对应路径；受影响 Pair 两边从效用分母排除，安全计数保留；修复后须新登记槽位，不能覆盖旧失败继续算。

**步骤 5 的 D102 时点适配限制（历史）：** 人工双方已经认可，但任一边漏计时的情况，应在原始材料中同时保留实际完成事实与计时缺测。当时 `pairs.rs` 的 completed 验证要求 `edit_seconds`、`pause_seconds`、`help_count` 和 `first_pass_accepted` 等均有实测值，`aggregate.rs` 的完成配对耗时汇总也依赖该前提，因此这种记录无法直接通过 `validate → summarize`。当时受影响批次的 typed 汇总须标 NOT RUN，禁止填 0、把已完成改为未完成或删除记录。后续已认领的最小适配及验证状态见[下一节](#计时缺测与人工元数据入口)；保留此历史限制不代表新适配已经运行通过。

### 空白记录与汇总阅读示例

下列为人工填写视图，字段定义仍引用 ROADMAP 7.2.3；`__` 表示待填，不是 0，也不能直接送给验证器。合成演练和真实反馈分开存放；真实同意文本与身份不进入仓库。执行者需逐项填全登记字段，不只填偏好。

| 记录 | 空白填写内容 |
| --- | --- |
| Run | `evaluation_run=__；run_started_at=__；run_deadline_at=__；participant_scope=__；feedback_purpose=product_evaluation；consent_state=__` |
| 槽与 Pair | `evaluation_run=__；task_slot=__；pair_id=__；task_type=__；semantic_stratum=__；卡=__；contrast=__；left_variant=__；right_variant=__；presentation_seed=__；comparability_status=__` |
| Pair 条件 | `path=__；endpoint 条件一致性=__；model_id=__；model_version=__；service_parameters=__；context_limit=__；redaction_condition=__；B快照标识=__；C四枚举=__`（不填真实 endpoint 凭据、快照正文或用户 ID） |
| Pair 结果 | `pair_started_at=__；pair_completed_at=__；pair_completion_status=__；preference=__；blind_status=__；未测/不可比较原因=__` |
| Candidate（左右各一行） | `pair_id=__；candidate_id=__；requested_variant=__；actual_brief_state=__；brief_build_status=__；generated=__；draft_source=__；degraded_reason=__；candidate_completion_status=__；first_pass_accepted=__；edit_seconds=__；pause_seconds=__；help_count=__；unsupported_claim=__；required_fact_error_count=__；privacy_or_safety_stop=__；failure_code=__` |
| Candidate 隐私与缺失 | `candidate_id=__；placeheld_turns=__；carries_exempted_original=__；refusal_code=__；missing_reason=__；sanitized_note=__；delivery=false` |
| 无正文汇总 | `版本/dirty=__；执行命令/退出码/日志=__；数据性质=实际生成未评分 / 预设模拟反馈 / 已同意人工反馈；assigned=__；started=__；generated=__；rated=__；completed=__；withdrawn Run=__；各层偏好分子/分母=__；耗时原值/中位差/中位比例=__；首稿接受分子/分母/缺测数=__；事实正确分子/分母/缺测数=__；安全事件分子/assigned候选=__；帮助次数及失败/缺测原因=__` |

下表是**现有测试的断言期望与解释**，不是自动导出的测量报告，也不是新卡或真人得分；本轮对这些既有断言的执行回执见本节末尾。两列不得合并统计；上文 P1–P8 表提供逐边重算依据。

| 无正文指标 | 真实本机生成、无人评分的单对 T1/AC 测试 | P1–P8 预设模拟反馈 fixture |
| --- | --- | --- |
| Pair assigned / started / generated / rated / completed | 1 / 1 / 1 / 0 / 0 | 8 / 7 / 5 / 3 / 5 |
| 生成 Candidate / withdrawn Run | 2 / 0 | 11 / 1 |
| 偏好 | 无有效分母，不填 0% | 同条件 T2/AB：B 胜 1、平局 1、有效 3，`(1+0.5)/3=0.5` |
| 完成与编辑 | 0/1 完成；耗时缺测，不填 0 秒 | 总完成 5/8；首 Run T2/AB 为 3/5；P1–P3 差值 −10、30、−20 秒，中位 −10；比例仅 P2/P3 的 0.25、−0.25，中位 0 |
| 首稿接受 | 每边分母 1、已观察接受数 0、缺测数 1；这是缺测，不能解释为观察到 0% 接受 | T2 endpoint A=3/4、B=1/3；T1 模板 A=0/1、C=0/1；T3 endpoint B=1/1、C=0/1，分别报告 |
| 事实正确 | T1 不适用 | T3 未降级 endpoint：B=1/1、C=0/1；T1/T2 不适用 |
| 能支持的结论 | 生成与未评分状态可进入现有验证和汇总 | 预设反馈算术及排除规则可重算；不能证明真人效用 |

流程 `rated` 不是所有效用指标通用分母；安全暂停或撤回仍保留历史漏斗，效用需进一步排除。首稿接受/事实正确的缺测数必须随原分母一起报告，不能把“生成但未评”写成明确拒绝或事实错误。P5 同 voice 保留完成但不比较，P6 破盲不进偏好，P4/P7 的失败/未开始不从 assigned 删除，P8 的撤回在 Run 单列。

### D102 有限合成干跑回执（历史）

以下结果由主线程实际执行后转达，本材料实现者未另行运行。候选为 `6a56d85` 加 dirty 的 N1 测试专用注册/文档；VAL 测试源码未变。环境为 Rust 1.83、Windows MSVC、现存 OpenSSL，环境记录为仓库外 `E:\Project\Soul-checks-20261020-n1-val\env.ps1` 与 `environment.json`。

| 实际命令 | 主线程实跑结果 | 仓库外日志 |
| --- | --- | --- |
| `cargo test -p soulcore --test val01_evaluation --locked --offline` | 50 passed / 0 failed，native exit 0；编译 42.27 秒，测试 0.45 秒 | `E:\Project\Soul-checks-20261020-n1-val\val-dry-run.log`、同目录 `val-dry-run-exit.txt` |

这是现有 50 项合成断言的有限干跑，涵盖实际生成未评分状态与预设模拟反馈统计；不代表新卡效果或真人效用。本轮新五卡、人工呈现/评分流程、完成但计时缺测反例、真实模型和平台门禁均未运行。未改变正式 Goal/Q2/U7 关闭条件，也未新增测试适配或产品能力。

## 计时缺测与人工元数据入口

本次明确授权继续补齐 completed 但计时缺测的测试适配，并提供离线元数据操作入口。仍在既有 `val01_evaluation` 内复用 `Evaluation → validate → summarize`；不新增产品 UI、IPC、生成入口、生产表或统计实现。下文记录行为、操作和本轮实际回执，不沿用上节 50 项结果签发新候选通过。

### 完成事实与计时完整性分别保留

| 情况 | 记录要求 | 汇总解释 |
| --- | --- | --- |
| 已完成且净编辑耗时、暂停时长都实测 | `edit_seconds` / `pause_seconds` 各保留实测值，包括 0 | 满足其他既有条件时进入成对耗时；控制边为 0 只排除比例，保留绝对值和差值 |
| 已完成但未测净编辑耗时 | `edit_seconds=Missing(NotMeasured)`，`candidate.missing_reason=NotMeasured`；完成状态与其他实测字段保留 | 任一边 edit 缺失则整对不进入耗时差/比例；单列耗时缺测对数。完成漏斗、偏好和其他已测指标不因该缺测而被删除 |
| 暂停时长未测，但独立计得净编辑耗时 | `pause_seconds=Missing(NotMeasured)`，`edit_seconds` 保留独立实测净值，`candidate.missing_reason=NotMeasured` | 暂停缺测按字段单列；净 edit 仍可参与成对耗时 |
| 只有含暂停的总时长，不能分离净编辑时间 | edit 与 pause 都保留 `Missing(NotMeasured)`，不把总时长填到 edit | 不推算暂停或净时间；按 edit 缺失排除该对耗时并保留完成事实 |

放宽仅限 completed 的 `edit_seconds`、`pause_seconds`：允许 `Value` 或 `Missing(NotMeasured)`；N/A、`NotStarted`、`NotCompleted` 等原因仍不能冒充完成后的漏计时。未完成候选继续沿用原阶段规则。`help_count`、`first_pass_accepted`、T3 事实评分和安全字段等不随本次顺带放宽；这些必填信息缺失时仍报验证错误，不补默认值。

缺测同时保留候选总原因与按字段计数，防止“同一候选两字段都漏测”被误读成两名候选。编辑统计必须同时给出完成对数、耗时缺测对数和实际可用的成对原值；安全暂停、撤回、不可比较和降级仍按原规则排除对应效用，不能借计时修复重新纳入。所有指标继续遵循 ROADMAP 7.2；未测不是观察到 0，完成率分母仍是全部 assigned pair。

### 从人工填写到离线校验的最短路径

1. 先按上文登记 Run/槽位/任务卡、左右映射、B 快照、C 四枚举和条件。操作员掌握映射，评分者先只看左右；来源、variant 和映射不在评分前揭示。正文仅临时呈现，禁止写入离线输入或日志。
2. 逐边记录已观察的生成、完成、评分与计时元数据；缺计时按本节处理。T1 同 voice 不可比较、T2 不评原文事实、只有 T3 评正文语义的边界不变。完成事实不得因计时失败改写；未观察的人工反馈也不得由生成结果推定。
3. 从 [synthetic_manual.json](../../crates/soulcore/tests/val01_evaluation/synthetic_manual.json) 复制到仓库外，以 UTF-8 编辑已有元数据字段。输入根对象为 `Evaluation` 的 `runs`、`registered_slots`、`pairs`、`candidates`；枚举使用 snake_case，例如 AB 写 `"a_b"`，T2 写 `"t2"`。实测值写 `{"value":0}`，漏测写 `{"missing":"not_measured"}`，不适用写 `"not_applicable"`；普通 Option 字段必须保留显式 `null`，不能删掉字段。完整批次必须保留每个登记槽对应的 Pair 和双方 Candidate；未开始也保留完整 planned 行，未完成保留状态与原因。整组漏行会报 `validation` / `registration`，不能通过删除登记槽来绕过原分母。不得加入 prompt、候选或编辑正文、姓名、账号、真实库位置或密钥。未知字段、缺字段、坏关联会报错，不靠补默认值修正。
4. 显式设置 `SOUL_VAL01_INPUT` 指向该仓库外文件，再运行默认 ignored 的 `manual::summarize_file`。入口只读取指定元数据并校验汇总，不生成候选、不访问 endpoint、不读取真实数据库；普通测试运行不自动读取外部文件。
5. 校验失败时保留原始观察，按错误类别核对输入，不删除失败槽或改分数求通过。成功时读取 stdout 中 `VAL01_REPORT_BEGIN` 与 `VAL01_REPORT_END` 之间的一个 JSON 对象，检查各层原分母、字段缺测与耗时缺测对数。Cargo/libtest 自身也输出文字，**整份 stdout 不是可直接解析的 JSON 文件**；只提取两标记之间的报告块，日志原样保留。

成功报告保留输入中的匿名 ID、模型标识和服务条件等元数据字符串，**这些字符串必须在输入前脱敏**。拒绝未知字段并不等于自动识别任意字符串内的隐私。入口没有自动拒绝仓库内路径，本流程要求将编辑副本放在仓库外，不能把该操作约定写成已实现的路径安全检测。

在仓库根使用 pwsh，先复制示例到一个新的仓库外目录；每次创建唯一目录，不覆盖已有手工记录。示例分数全部预设，可原样用于合成重算，也可只修改合成元数据练习缺测/错误处理，不能直接标为真实反馈：

```pwsh
$ErrorActionPreference = 'Stop'
$valExample = 'crates/soulcore/tests/val01_evaluation/synthetic_manual.json'
$valRunDirectory = Join-Path 'E:\Project' ('Soul-val-manual-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $valRunDirectory | Out-Null
$valInputPath = Join-Path $valRunDirectory 'manual-input.json'
if (Test-Path -LiteralPath $valInputPath) { throw '输入副本已存在，不能覆盖手工记录' }
Get-Content -LiteralPath $valExample -Raw -Encoding UTF8 |
    Set-Content -LiteralPath $valInputPath -Encoding UTF8
Write-Output $valInputPath
```

以 UTF-8 编辑该路径后，在同一个 pwsh 会话执行下列命令；若换会话，先把 `$valInputPath` 设为刚才实际输出的路径，不重新复制覆盖记录。调用固定显式测试名，恢复环境变量使用 `finally`，不改变全局配置：

```pwsh
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath $valInputPath -PathType Leaf)) { throw '请先指定实际输入副本' }
$valInputWasSet = Test-Path Env:SOUL_VAL01_INPUT
$valInputBefore = $env:SOUL_VAL01_INPUT
try {
    $env:SOUL_VAL01_INPUT = $valInputPath
    cargo test -p soulcore --test val01_evaluation manual::summarize_file --locked --offline -- --ignored --exact --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'VAL 离线元数据校验或汇总失败' }
} finally {
    if ($valInputWasSet) {
        $env:SOUL_VAL01_INPUT = $valInputBefore
    } else {
        Remove-Item Env:SOUL_VAL01_INPUT -ErrorAction SilentlyContinue
    }
}
```

失败报告根对象含 `error`，`error.category` 为 `input_not_specified`、`read`、`json`、`validation` 或 `report`。`json` 类另含 `kind`（`io/syntax/data/eof`）、`line`、`column`；`validation` 只含有限错误种类 `kinds`，不回显输入 record ID 或字段位置，不能据此宣称会直接定位每条坏记录。对 validation 错误，应在保留原输入的前提下核对关联和状态；错误分支令测试失败，操作者以实际 native exit 判断命令失败。

成功报告的 `pairs`、`candidates`、`participant_preferences` 均是 `[{"stratum": …, "summary": …}]` 数组。Pair 层读取 `edit_valid_pairs`、`edit_missing_pairs` 和 `edits`；Candidate 层 `edit_missing`、`pause_missing` 是按缺测原因计数的对象，另有候选总 `missing`。这些字段沿用既有分层键，不跨来源/任务/条件合并。

| 原样合成示例的可重算期望（实际入口验证见下文） | 数值与解释 |
| --- | --- |
| 输入规模与漏斗 | 3 Pair / 6 Candidate；assigned、started、generated、rated、completed 均为 3；withdrawn Run=0 |
| 同层 AB 偏好 | B 胜 1、平局 1、分母 3，得分 `(1+0.5)/3=0.5` |
| 耗时分母 | `edit_valid_pairs=2`、`edit_missing_pairs=1`；三对仍全部完成 |
| 有效原始耗时 `[control,target]` | `[0,90]` 与 `[80,100]`；差值 −90、−20，中位 −55 秒；比例只有第二对 −0.25，第一对控制边 0 不算比例 |
| Candidate 缺测 | A 层 edit 缺测 1、pause 缺测 1，候选总 missing=2；B 层均无缺测。字段计数不能替代候选数 |
| 首稿接受 | A=3/3、B=0/3；均是预设反馈，不是人工观察 |

### 可执行演练与真实阶段的区别

本次只安排纯合成 metadata 的读取、拒绝及重算演练。示例中的偏好、计时和完成值属于预设模拟输入，不是本轮生成或人工作答；即使操作员手工编辑合成分数，也不能计为真实效用样本。现有 endpoint 测试仍只接受字面 loopback；离线汇总输入不授予出网或生成权限。

真实作者/志愿者比较仍须 Goal 1 按原标准关闭、Q2-00 提供可比较输入和可用构建、主动产品评测同意、开始/截止与撤回说明，以及逐次 endpoint 授权；缺一项仍 NOT STARTED。真实候选的受控生成与盲评呈现入口尚待正式前置，D103 元数据入口不补足它。D103 时点新五卡实际生成、真人盲评、真实计时/效果、真实模型和平台验收均未运行；后续合成接线单列于 [D104](#d104-合成受控生成与盲呈现入口)，不迁移为真实效用证据。

### 本轮执行回执

以下为主线程实际执行后转达的结果，文档实现者未另跑构建。候选 `6a56d85` + dirty；Rust 1.83 / Cargo 1.83 / pwsh 7.6.5 / Windows MSVC，沿用既有 VS/OpenSSL、jobs=4、debug=0、incremental=0，未改全局配置、target 或锁文件。证据根为仓库外 `E:\Project\Soul-checks-20261020-val-timing\`，其中 `env.ps1` / `environment.json` 保存运行环境。

失败与修复链保留如下：

- `timing-red`：新增两条计时缺测反例实跑失败，native exit 101；最小适配后 `timing-green` 两条通过、退出 0。
- `manual-red`：新增两条入口反例因 NotImplemented 失败，native exit 101。首次完整实现为 64 passed / 1 ignored，严格 Clippy 和格式检查退出 0；这是中间候选。
- 独立评审发现：离线输入仍保留登记槽，但漏掉一整组 Pair 及双方 Candidate 时会缩小 assigned。`review-red` 实跑 1 failed、native exit 101，实际错误结果为 `Ok` 且 assigned 从 3 变成 2。随后仅在 manual 入口要求每个登记槽都有对应 Pair；双方 Candidate 和唯一性继续由现有验证器检查，缺组返回 `validation` / `registration`，不自动补行。在线逐槽适配不变，这是维护原登记分母，不是新增统计规则。

| 最终实际命令 | 结果 | 证据根内日志与退出文件 |
| --- | --- | --- |
| `cargo test -p soulcore --test val01_evaluation --locked --offline` | 65 passed / 0 failed / 1 ignored，native exit 0；编译 18.40 秒，测试 0.32 秒 | `val-reviewed.log` / `val-reviewed-exit.txt` |
| `cargo clippy -p soulcore --test val01_evaluation --locked --offline -- -D warnings` | native exit 0，6.24 秒 | `val-reviewed-clippy.log` / `val-reviewed-clippy-exit.txt` |
| `cargo fmt --all -- --check` | native exit 0 | `val-reviewed-format-exit.txt`（目录中无独立 format 日志文件） |

主线程另外用上文完整固定命令显式执行 `manual::summarize_file` 共 8 次。`run-manual-checks.ps1` 是仓库外本轮验证脚本，不是新增产品入口；`manual-cases.json` 汇总结果。每个 case 均有 `.log`、`-exit.txt`、`-report.json`，输入全部合成，错误文件中的内容标记未出现在结构化错误报告中。

| Case | 实际结果与可支持的结论 |
| --- | --- |
| `manual-positive` | native exit 0；3 对完成、偏好分母 3、edit_valid=2 / missing=1、中位差 −55 / 比例 −0.25；0 秒保留，读取后输入不变 |
| `manual-all-missing` | native exit 0；3 对 edit/pause 全缺测，completed=3、偏好分母 3、edit_valid=0 / missing=3；edits 为空，两项中位值为 null，不伪造 0 |
| `manual-unknown` | native exit 101；`json` 错误，未知字段拒绝 |
| `manual-relation` | native exit 101；`validation` 错误，输入 ID 标记不回显 |
| `manual-missing-pair` | native exit 101；`validation` / `registration`，整组漏行不缩小 assigned 后继续汇总 |
| `manual-malformed` | native exit 101；`json` 错误，损坏 JSON 拒绝 |
| `manual-unspecified` | native exit 101；`input_not_specified`，未指定输入不自动取文件 |
| `manual-unavailable` | native exit 101；`read`，无法读取不补空记录 |

独立只读评审已核对完整性修复、操作说明及最终运行证据：65 passed / 1 ignored 原始日志、Clippy/fmt 退出文件、8 个 case 的日志/退出码与保存报告块一致；2 正例、6 个预期负例的计时、分母及私密标记拒绝与摘要一致。原完整性缺陷已关闭，无剩余阻塞或非阻塞发现。本轮状态为 REVIEWED，仅支持开发验证，不签发正式 gate。最终 65 项包含既有 50 项与本轮增量，不累计前轮或重复红绿运行；默认 ignored 的显式文件入口另按上述 8 次列出。

主线程另实跑本轮 7 份文档的 109 个本地链接、21 个锚点与 diff 检查，均通过；证据为同一仓库外证据根的 `docs-check.json`。文档核验不替代代码或平台验收。

本轮仅完成测试记录适配、离线 metadata 入口及上述合成执行。未运行新五卡实际生成、真人盲评/计时/效果、真实外部服务、独立 `q2_continuity` / `q2_recovery` / `session_e1` targets、desktop workspace、完整 G-L/G-W/G-M 或安装；未提交。正式 Goal 1/Q2/U7 的前置及关闭状态保持。

## D104 合成受控生成与盲呈现入口

本片依据 D104 及[阶段计划第 9 节](2026-10-20-next-stage-plan.md#9-d104-合成受控生成与盲呈现接线)，接续五卡合成工程准备。起点仍为 `6a56d85 + dirty`，保留 D101–D103 与其他已有修改。范围限新增测试模块和原设计说明；未新增产品 UI、IPC、权限、schema、依赖或统计框架。

### 前置核对与本片范围

| 条件 | 开工核对结果 | 对本片的影响 |
| --- | --- | --- |
| Goal 1、同候选完整平台与人工验收 | [STATUS](../STATUS.md)仍标未关闭；G-L 暂缓，G-M/安装后置，历史完整 G-W 不覆盖当前 dirty | 不签发正式 Goal/Q2 或平台结论；不以本片替代其证据 |
| Q2-00 的可比较输入与可用构建 | [GOAL2_PLAN](../GOAL2_PLAN.md#q2-00固定关闭凭据与可比较基线)未签发正式启动 | 本片为新增明确授权的纯合成工程准备，不是正式 Goal 2 |
| 真实参与者主动同意、起止与撤回安排 | 尚未建立真实作者/志愿者试验安排 | 不招募、不启动计时或真人反馈；真实阶段仍 NOT STARTED |
| 真实服务配置与逐次授权 | 本片不调用真实服务 | T2/T3 只用受控 loopback，逐边显式模拟 exact-plan 批准 |
| 新增工程授权 | 作者明确要求继续受控候选生成和盲呈现准备，暂不真人试验 | 可以实现/验证固定合成卡接线与模拟反馈协议；不必重复取得已给的同一许可 |

D103 的“不生成”仍描述上一片离线重算入口；D104 新入口单独承担合成生成。真实候选生成、真正人可见的盲评交互和正式试验须满足原前置。本片的 callback 演练不能被称为已实现的 GUI、TTY 或人工可操作界面。

### 技术选择与正文生命周期

固定五卡复用本页 S1–S5：S1 实走本机模板，S2–S5 实走 loopback 固定回复；每卡两候选，设计演练共 10 候选、8 个 loopback 请求。固定回复验证请求路径、来源和呈现接线，不检验模型理解卡意或完成语义任务。A/B/C、B 同快照、C 预登记枚举、单次批准和拒绝规则沿既有适配，T1 正文槽限制、T2 unsupported claim、T3 首稿事实与 D103 计时缺测不变。

本片 T1 初稿固定含正文槽；显式 `t1_slot_filled=true` 只支持补槽后的完成事实，不能追记原首稿已被接受。提交协议拒绝 T1 的 `first_pass=true`，同时 completed 仍要求显式已补槽；脚本反馈原有 `first_pass=false` 保持不变。

私有 session 持有登记和左右正文，向呈现 callback 借用 `BlindPairView`，只包含卡说明和左右文本，不包含 variant、source、candidate ID、完整 Evaluation 或计划；该视图不实现 Debug/Serialize。操作者的汇总含 variant 分层，不能传给评分 callback。使用借用视图和状态检查是普通测试协议边界，不是对恶意调用者或已知映射操作者的安全隔离，也不保证文本风格无法破盲。

候选正文只在内存持有和 callback 内检查，不打印到 stdout、不写文件或审计；不把完整 Draft、计划或 HTTP body 当作呈现输出。对象释放不等于物理内存擦除。实际 UI、终端截图、录屏、持久正文包与编辑器交互均不在本片实现范围。

### 呈现、反馈与元数据边界

1. 在生成前固定五卡、配对左右、运行条件和合成 Run；端点两边分别模拟 exact-plan 批准，不能共用或重放批准。测试固定回复不自动转成偏好、耗时或完成事实。
2. callback 接收借用的左右盲视图；生成后原记录保持 `unrated/incomplete`。拒绝、降级、截止、撤回和安全暂停保留原状态；已关闭 Run 或暂停路径不得恢复可呈现视图。
3. 显式 `SimulatedFeedback` 作用于克隆记录，经现有校验后才提交并固定，随后才允许 reveal。早揭盲、无效反馈和重复提交拒绝；无效反馈不能留下半写入。模拟值只验证流程，不能作为真人评分计入效用。
4. 只生成模式仅返回操作者计数和汇总，不输出完整映射 metadata。模拟反馈模式在反馈固定、揭盲之后才允许附纯 metadata，继续复用 `validate → summarize`；无正文 JSON 可沿 D103 入口重算，但不因此变成真实观察。
5. 既有逐槽适配要求未启动槽尚无 Pair；session 导出时只在克隆中为自己持有、确定尚未开始的槽补完整 planned Pair/Candidate，以保留全部分母，不覆盖失败或改原记录。这不是 manual 文件入口的容错：外部文件漏组仍按 D103 拒绝，不能自动补坏输入。

五卡演练只是有限工程输入，不能取代正式作者至少 30 槽的预登记，也不能把每卡重复候选当独立参与者。若后续出现盲法破坏，仍按原字段记录 broken；不通过隐藏失败、改左右或重采样追指标。

### 显式合成入口

入口为默认 ignored 的 `synthetic_entry::exercise`，源码位于 `synthetic_entry.rs`，复用 `synthetic_cards.rs`、`presentation.rs`，行为测试在 `presentation_tests.rs`。环境变量 `SOUL_VAL01_EXERCISE_MODE` 必须显式为 `generate_only` 或 `scripted_feedback`，没有默认模式，缺失/未知拒绝；入口不读取外部文件或真实库，不写文件。stdout 使用 `VAL01_EXERCISE_BEGIN` / `VAL01_EXERCISE_END` 包住单个合成报告对象，不含正文、计划或 HTTP body；整个 Cargo stdout 仍不是 JSON 文件。接口已与源码核对，实际运行结果见本片回执。

成功报告固定 `synthetic=true`、`human_evaluation=false`，含 `mode`、`feedback_origin`、`summary` 和 `counts`。`counts` 字段为 `http_requests`、`local_draft_calls`、`presentations`、`feedback_records`、`reveals`。`generate_only` 的 `feedback_origin="none"`，不附完整 metadata；`scripted_feedback` 的 `feedback_origin="preset_simulated"`，全部模拟反馈固定并揭盲后才附 `metadata`。`summary` 沿既有带 variant 的分层汇总，面向操作者，不传给盲呈现 callback。

失败报告含 `synthetic=true` 和 `error.category`，有限类别为 `mode_not_specified`、`invalid_mode`、`generation`、`presentation`、`feedback`、`reveal`、`metadata`，不回显模式环境变量内容。失败令测试返回错误；命令是否成功以实际 native exit 为准。

| 模式的可重算预期（实际结果见本片回执） | 生成与呈现计数 | 反馈与报告边界 |
| --- | --- | --- |
| `generate_only` | 5 Pair、10 generated Candidate、8 HTTP、2 local draft calls、5 presentations | feedback_records=0、reveals=0；仍 unrated/incomplete，无完整 metadata |
| `scripted_feedback` | 同上 | feedback_records=5、reveals=5，所有偏好为预设 tie；附 metadata。S3 左 C / 右 B，左 edit=`NotMeasured`，设计上耗时 4 对有效 / 1 对缺测；均不是人工观察 |

以下为已约定调用形式，实际执行由主线程统一安排。例子默认仅生成；若要演练预设反馈，显式将模式改为 `scripted_feedback`。两种模式均非真人试验：

```pwsh
$ErrorActionPreference = 'Stop'
$valExerciseModeWasSet = Test-Path Env:SOUL_VAL01_EXERCISE_MODE
$valExerciseModeBefore = $env:SOUL_VAL01_EXERCISE_MODE
try {
    $env:SOUL_VAL01_EXERCISE_MODE = 'generate_only'
    cargo test -p soulcore --test val01_evaluation synthetic_entry::exercise --locked --offline -- --ignored --exact --nocapture
    if ($LASTEXITCODE -ne 0) { throw 'VAL 合成生成与呈现演练失败' }
} finally {
    if ($valExerciseModeWasSet) {
        $env:SOUL_VAL01_EXERCISE_MODE = $valExerciseModeBefore
    } else {
        Remove-Item Env:SOUL_VAL01_EXERCISE_MODE -ErrorAction SilentlyContinue
    }
}
```

### D104 本片执行回执

以下结果由主线程实际执行后转达，文档实现者另只读核对了证据目录与 `exercise-cases.json`。候选为 `6a56d85 + dirty`；证据根 `E:\Project\Soul-checks-20261020-val-presentation\`，环境见同目录 `environment.json` / `env.ps1`。没有提交，不把 dirty 实现标为 GATED/INTEGRATED。

`presentation-red.log` 首先记录两条 NotImplemented 反例失败，native exit 101。实现后的首候选 `val-candidate` 为 77 passed / 2 ignored。独立评审随后发现 T1 `filled=true` 与 `first_pass=true` 可被误接受，`review-t1-red.log` 实跑 1 failed、native exit 101；失败断言为期望拒绝的 `is_err`，说明实际错误接收。最小修复使 T1 补槽完成与原首稿接受分开，未放宽已有评分或安全要求；首候选结果不替代修复后回归。

| 最终实际命令 | 结果 | 证据根内文件 |
| --- | --- | --- |
| `cargo test -p soulcore --test val01_evaluation --locked --offline` | 78 passed / 0 failed / 2 ignored，native exit 0；编译 15.67 秒、测试 0.37 秒 | `val-reviewed.log` / `val-reviewed-exit.txt` |
| `cargo clippy -p soulcore --test val01_evaluation --locked --offline -- -D warnings` | native exit 0，5.80 秒 | `val-reviewed-clippy.log` / `val-reviewed-clippy-exit.txt` |
| `cargo fmt --all -- --check` | native exit 0 | `val-reviewed-format-exit.txt`；目录无独立 format 日志 |
| `cargo test -p soulcore --test q2_continuity --test q2_recovery --test session_e1 --locked --offline` | 4 + 3 + 27 = 34 项通过，native exit 0 | `related-regressions.log` / `related-regressions-exit.txt` |

相关三目标与生产代码在本片无修改。最终 78 项包含 D103 的 65 项与本片 13 项，不累计红绿或重复运行；默认忽略的两项分别是 `manual::summarize_file` 与 `synthetic_entry::exercise`。主线程另显式运行以下 5 个 case，汇总为 `exercise-cases.json`；每项均保留 `.log`、`-exit.txt` 和 `-report.json`。

| 实际 case | native exit | 结果与范围 |
| --- | --- | --- |
| `exercise-generate-only` | 0 | 10 generated Candidate、8 HTTP、2 local draft calls、5 presentations；无 metadata，rated=0、completed=0，未补反馈 |
| `exercise-scripted-feedback` | 0 | 同样的生成/呈现计数，5 feedback、5 reveal、5 completed；耗时有效 4 对、缺测 1 对保留。所有反馈为预设模拟 |
| `exercise-manual-roundtrip` | 0 | 将揭盲后纯 metadata 保存为仓库外合成文件，交 D103 的 `manual::summarize_file` 全文重算；summary 一致，读取前后输入不变 |
| `exercise-missing-mode` | 101（预期拒绝） | `mode_not_specified`，无默认模式 |
| `exercise-unknown-mode` | 101（预期拒绝） | `invalid_mode`，未知模式内容标记不回显 |

`run-exercise-checks.ps1` 是仓库外本轮验证脚本，不是产品入口。它核对计数、模式分支、重算及输入不变，并检查 stdout 未包含预设 9 段 paste/reply 正文、正文或 transport 字段。该有限检查证明所列合成输出边界，不承诺任意隐私识别；callback 未向真人展示正文，仍无 GUI/TTY 观察证据。

独立只读评审已完成 T1 最小修复及最终证据的定向复核：原始红绿日志、最终 78 + 34 项、严格 Clippy/格式退出文件、5 个 case 的保存报告与日志一致，唯一 T1 发现已闭合，无剩余阻塞或非阻塞发现。本片为 REVIEWED，开发范围验证完成。主线程另核本轮 7 份文档的 117 个本地链接、29 个锚点与 diff，均通过，证据为 `docs-check.json`。正式 Goal/Q2/U7 前置不变。本片未运行真人试验、真人计时/偏好、真实模型、desktop workspace、完整 G-L/G-W/G-M 或安装，未签发 GATED/INTEGRATED。

## D105 本机合成操作入口

本节接续 D104 的内存呈现协议。[D105](../DECISIONS.md#2026-10-20-下一阶段执行范围)允许将其接成独立测试工具，不启动正式真人效用试验；产品页面、IPC、用户库、真实模型及平台验收安排保持原范围。工具只使用原五张合成卡，端点仍是本进程的 loopback 固定回复。

### 操作与数据边界

1. 显式启动默认 ignored 的 `operator::serve`，在终端输出的本机 URL 打开页面。首次页面只显示说明，点击“开始五卡合成演练”才生成首卡。
2. 逐侧开始计时、编辑、暂停或结束计时，明确填写完成、首稿接受、帮助次数、事实和安全反馈，再选配对偏好及盲态。T1 必须补全文字槽，补槽不能计为首稿接受；T2/T3 沿原事实检查口径。
3. “锁定本卡反馈”成功后清除正文，才可点击揭盲；揭盲后才进入下一卡。完整元数据仅在整个演练结束后提供，不通过状态或中间摘要提前发送 variant 映射。
4. 结束保留已有反馈及尚未完成/开始的槽；撤回另外记录 withdrawn。截止不自动评分或完成。关闭页面没有可靠的最后一次请求，服务仍按单调时钟截止；进程异常退出会丢失尚未导出的内存结果，不承诺恢复。
5. 最终页面可显式下载纯元数据 JSON，交 D103 的 `manual::summarize_file` 重算。终端结束输出也只含合成元数据与汇总。操作反馈标 `synthetic_operator_feedback`，与 D104 的预设反馈分开；仍是 `synthetic=true`、`human_evaluation=false`，不计真实任务样本。

编辑后的正文只存在于页面内存，不发送给 Rust；浏览器不使用 localStorage、IndexedDB、service worker 或自动保存。候选正文只经本机响应交给页面，使用文本节点/textarea 呈现，不拼接成 HTML。工具不承诺浏览器、操作系统或物理内存的安全擦除，也不允许在此入口输入真实资料。

每侧计时使用浏览器单调时钟，分别累计活动和暂停区间；只有明确结束且完整可靠的测量才记录秒数。真实亚秒区间舍入后的 0 与缺测区分。未开始/未结束、刷新、窗口失焦、页面隐藏或主动标记不可靠时不补零；完成但缺测继续使用 D103 的 `NotMeasured`。后端检查时长范围，不能据此证明操作者反馈是真实效用或防范同机进程伪造。

本机服务只绑定 `127.0.0.1:0`。固定路由验证精确 Host，API 另验证精确 Origin、会话令牌及 JSON 类型；拒绝模糊 framing、超长、截断和超时请求。响应禁止缓存，不开放 CORS，页面禁止第三方资源和框架嵌入。临时 URL 只在运行期间有效，不持久化凭据或开放产品能力。

### 显式启动

在已配置本项目 Windows Rust/SQLCipher 构建环境的 `pwsh` 中，从仓库根运行。环境变量仅在当前进程临时设置；编译依赖与目标路径沿项目现状，不安装依赖或改锁文件。

```pwsh
$ErrorActionPreference = 'Stop'
$operatorModeBefore = $env:SOUL_VAL01_OPERATOR_MODE
$operatorSecondsBefore = $env:SOUL_VAL01_OPERATOR_SECONDS
try {
    $env:SOUL_VAL01_OPERATOR_MODE = 'synthetic'
    $env:SOUL_VAL01_OPERATOR_SECONDS = '900'
    cargo test -p soulcore --test val01_evaluation --locked --offline operator::serve -- --ignored --exact --nocapture --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw '本机合成演练未正常结束' }
} finally {
    if ($null -eq $operatorModeBefore) {
        Remove-Item Env:SOUL_VAL01_OPERATOR_MODE -ErrorAction SilentlyContinue
    } else {
        $env:SOUL_VAL01_OPERATOR_MODE = $operatorModeBefore
    }
    if ($null -eq $operatorSecondsBefore) {
        Remove-Item Env:SOUL_VAL01_OPERATOR_SECONDS -ErrorAction SilentlyContinue
    } else {
        $env:SOUL_VAL01_OPERATOR_SECONDS = $operatorSecondsBefore
    }
}
```

期限为 1–3600 秒，默认 900 秒。缺少或错误 mode 拒绝启动；不存在真实模型模式。启动 URL 包含仅本次内存会话使用的随机标识，不要作为长期入口保存或分享。

### D105 本片执行回执

候选为 `codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`。主线程保留开工已有修改，统一执行全部命令；两名实现者分别认领 Rust 测试工具与独立静态页面，独立只读评审者核对任务、实际源码和原始结果。没有提交或合入。证据根为 `E:\Project\Soul-checks-20261020-val-operator\`，环境见 `environment.json` / `env.ps1`；仅当前进程配置既有 VS/OpenSSL 构建环境，无依赖或锁文件变更。

初始 `timer-red.log` 为 6 个行为反例失败（exit 1）；`operator-red.log` 为模块路径编译错误（exit 101），不当作行为红测。随后 `operator-red-behavior.log` 记录 5 个未实现失败及 1 个 stub 导致的连接重置（exit 101），后者不当作请求边界已复现。首候选编译又暴露 UUID feature 不提供 `new_v4`，改用已有 `now_v7`，错误保留在 `operator-candidate.log`。

独立评审发现并关闭三项问题：无参数 action 的 unit variant 接受未知字段，改为空 struct variant；页面为 T1/T2 填入不适用指标，改为按任务显示并明确提交 null；未完成且未选择的首稿接受被补 false，改为必填 nullable 并保留 `NotRated`。对应失败保留在 `val-candidate.log`（87 passed / 1 failed / 3 ignored，exit 101）、`ui-review-red.log`（exit 1）、`first-pass-review-red.log`（exit 101），不以首次通过数量替代修复后的结果。

| 最终实际命令 | 结果 | 证据根内文件 |
| --- | --- | --- |
| `cargo test -p soulcore --test val01_evaluation --locked --offline` | 89 passed / 0 failed / 3 ignored，exit 0；包含新增 11 个 operator 用例 | `val-reviewed.log` / `val-reviewed-exit.txt` |
| `cargo clippy -p soulcore --test val01_evaluation --locked --offline -- -D warnings` | exit 0 | `clippy-reviewed.log` / `clippy-reviewed-exit.txt` |
| `cargo fmt --all -- --check` | 主线程执行 exit 0；仅保存退出文件，无独立 format 日志 | `format-reviewed-exit.txt` |
| `node --test crates/soulcore/tests/val01_evaluation/operator_ui/timer.test.mjs crates/soulcore/tests/val01_evaluation/operator_ui/app.test.mjs` | 9 passed / 0 failed，exit 0；6 个计时用例、3 个实际 UI handler 用例 | `ui-reviewed.log` / `ui-reviewed-exit.txt` |
| `node --check` 分别检查 `operator_ui/app.js`、`operator_ui/timer.js` | 均 exit 0 | `ui-syntax-exit.txt` |
| `cargo test -p soulcore --test q2_continuity --test q2_recovery --test session_e1 --locked --offline` | 4 + 3 + 27 = 34 passed，exit 0 | `related-reviewed.log` / `related-reviewed-exit.txt` |

89 项不与历史、红绿或相关回归相加；3 个默认 ignored 入口为 `manual::summarize_file`、`synthetic_entry::exercise`、`operator::serve`。TCP 用例实际检查 Host/Origin/token、模糊 framing、静态路由与未完成请求超时；UI 用例执行实际 handler，检查不发送正文、任务指标、缺测和截止后晚响应。

主线程另通过实际本机浏览器完成两次交互。第一次 S1 左侧补槽并结束计时、右侧明确未完成且未选首稿接受，锁定反馈后正文清除，再显式揭盲；S2 刷新后两侧计时失效，仍可明确完成并保留 `NotMeasured`，提交、揭盲后进入 S3。随后页面显示期限已到、正文和编辑控件已清除；终端保留 5 Pair / 10 Candidate，started=3、generated=3、rated=2、completed=1，生成候选为 6。`browser-exercise.log` 测试耗时为 23591.97 秒，而配置期限为 900 秒；本轮没有采集系统挂起或调度轨迹，原因未证实，因此该记录只支持恢复观察时已截止和记录保留，不能作为精确 900 秒墙钟截止测量。

第二次浏览器在 S1 未提交时主动“结束并保留已有反馈”，立即清除正文，显示最终摘要，并实际点击下载纯元数据。`browser-stop.log` 为 exit 0、测试耗时 11.98 秒，assigned=5、started=1、generated=1、rated=0、completed=0；下载与终端 metadata 全文一致。无正文的结束画面保存为 `browser-finished.png`。以上点击由 agent 执行，计时/偏好不作为真人任务证据，未代勾 G-M。

仓库外 `check-operator-cases.ps1` 已实际执行，结果见 `operator-cases.json`，每个命令保留日志与退出码：

| 显式执行 case | 结果 |
| --- | --- |
| `browser-feedback-recompute` | 第一份终端 metadata 经 `manual::summarize_file` 重算，summary 全文一致、输入不变，exit 0；S1 未选首稿接受为 `NotRated`，S2 两个完成候选均保留缺测 |
| `browser-download-recompute` | 第二次实际浏览器下载的 metadata 经同入口重算，summary 全文一致、输入不变，exit 0 |
| `missing-mode` / `unknown-mode` / `invalid-seconds` | 均按预期 exit 101；有限错误类别正确、未知 mode 标记不回显 |
| `idle-deadline` | 显式 synthetic、1 秒期限、无浏览器请求，自动结束 exit 0；五卡十候选全部保留未开始状态 |
| `browser-output-prose` | 两次终端输出均不含 9 段固定 paste/reply、2 段本地模板标记或正文/transport 字段；仅支持这 11 个合成标记与所列字段的有限检查 |

独立评审已核对三项修复及最终 89 + 34 + 9 项结果、Clippy 日志和格式退出文件，未发现新的范围内缺陷，原发现均闭合。浏览器操作与两次重算由主线程执行核对；格式日志缺失、长时间运行测量限制如上保留。本片为 REVIEWED，开发范围验证完成；尚未运行真实模型、真人效用、独立 desktop workspace、完整 G-L/G-W/G-M 或安装，未签发 GATED/INTEGRATED，真实任务样本仍为 0。

### D105 后续操作核对与 Windows 分段请求修复

2026-10-20，作者要求执行下一动作。本轮从实际五卡浏览器操作开始，候选仍为 `6a56d852d23a051c7c4e9845642791295c541105 + dirty`，保留全部开工修改。首次页面能够读取 welcome，但点击开始提示连接/操作未完成；刷新后停留连接中。直接诊断同一本机 API 时，state 可返回 200 welcome，start 返回 400 `timeout`；第一次 stop 同样超时，第二次才正常结束。该次运行没有产生反馈，五槽均保持未开始。此后修复基于新复现，不沿用原 D105 通过记录宣称没有问题。

**根因与改动：** `serve` 使用非阻塞 listener，`http::connection` 原来未明确恢复 accepted stream 的阻塞模式。数据暂未到达时，非阻塞 read 的错误被统一映射为 `timeout`，合法分段请求会提前被拒绝。旧 TCP 测试使用阻塞 listener，且一次写完整请求，未覆盖该组合。本轮只在 `operator/http.rs` 的原读取链前加入 `set_nonblocking(false)`，设置失败沿既有 `transport` 分类；保留 2 秒读取总预算、1 秒写超时、运行截止、Host/Origin/token、大小与 framing 校验。

`operator/tests.rs` 新增两条共用 helper 的真实 TCP 回归：一条沿实际非阻塞 listener 接受连接，另一条显式设置 accepted stream 为非阻塞以覆盖该前置。headers/body 分段发送，间隔 100 ms，均先实际失败在“合法请求应返回 200”的断言；不是编译错误或 body 写入 unwrap 失败。当前 Windows 两种模式都复现失败，修后都成功推进到 Pair。同步信号位于 `connection` 调用前，极端线程停调可能使接收合并，因此不声称对第二次读取进行了严格阶段同步；本轮红测实际触发证据保留。

证据根为仓库外 `E:\Project\Soul-checks-20261020-operator-flow\`。环境沿上一片现有 Windows MSVC/OpenSSL、Rust/Cargo 1.83、pwsh 7.6.5，进程内 jobs=4、DEV/TEST DEBUG=0、incremental=0；未改 target、依赖、全局配置、产品 UI/IPC 或生产 MODEL。两文件开工 `.before` 与增量 `.diff` 保留在证据目录，避免把此前未提交 D105 代码误算为本轮新增。

| 本轮实际检查 | 结果 | 原始记录 |
| --- | --- | --- |
| `cargo test -p soulcore --test val01_evaluation --locked --offline real_tcp_fragmented_start -- --nocapture --test-threads=1`，修前 | 2 failed / 0 passed，native exit 101 | `fragmented-red.log` / `fragmented-red-result.json` |
| `cargo test -p soulcore --test val01_evaluation --locked --offline`，修后 | 91 passed / 0 failed / 3 ignored，exit 0；包含新增 2 项，原不完整请求超时与安全拒绝仍通过 | `val-fixed.log` / `val-fixed-result.json` |
| 同 target 严格 Clippy / 根 `cargo fmt --all -- --check` | 均 exit 0 | `clippy-fixed.*`、`fmt-fixed.*` |
| 显式 `operator::serve`，synthetic / 900 秒 | 浏览器 S1–S5 自然完成，exit 0；实际测试执行 185.46 秒 | `full-flow-fixed.log` / `full-flow-fixed-result.json` |
| 下载文件经 `manual::summarize_file` 重算 | exit 0；summary 全文一致，输入未改变 | `download-recompute.log` / `download-recompute-result.json` |

**实际浏览器结果：** S1 左右均补全正文槽；S2–S5 沿固定回复明确填反馈；每卡依次锁定、确认正文清除、揭盲、进入下一卡。第五卡显示“完成演练并查看元数据”，随后正常结束并点击下载。下载文件与终端 metadata 全文一致：5 Pair、10 Candidate，assigned/started/generated/completed 均为 5，generated_candidates=10。由于操作者是了解映射的 agent，每卡明确记录 `blind_status=broken`，因此 rated=0，偏好不进入有效盲评分母。

每侧实际点击开始/结束，亚秒操作舍入后的 0 是本次工具操作计时，不是缺测补零，更不是人的编辑效率。全部反馈仍标为合成操作，真实效用样本为 0。`browser-final-observations.json`、`browser-counts.json`、`downloaded-metadata.json`、`terminal-report.json` 和无正文的 `full-flow-finished.png` 保存最终证据；首次故障见 `initial-browser-observations.json` 与 `full-flow.log`，未覆盖失败。

本轮前端资产和生产授权/生成逻辑未改，因此未重复旧 9 项页面测试或 34 项生产回归；本次 91 项与浏览器实跑仅支持受影响测试工具范围。未运行完整 G-L/G-W/G-M、独立 desktop workspace、安装、真人试验或真实模型，不签发正式阶段关闭。

独立只读评审 `/root/model_review` 已按本轮 `.before` 增量核对两文件、红绿结果与保留的超时/安全检查，随后独立解析下载元数据、终端报告和离线重算输出，并复核当前文档；无剩余具体阻塞。本轮状态为 **REVIEWED，开发范围验证完成**。浏览器操作、截图观察和重算前后输入不变由主线程执行记录，评审者未重跑浏览器或构建。源码仍 dirty，未提交或合入。
