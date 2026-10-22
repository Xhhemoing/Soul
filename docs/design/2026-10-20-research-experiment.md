# 实验版阶段实施计划：数据、程序算法与架构研究

日期：2026-10-20。规划起点：`codex/soul-rebuild-20261019` @ `6a56d852d23a051c7c4e9845642791295c541105` + dirty。
本文维护实施方案与实际执行回执；未勾选任务不代表功能完成，定向验证不代表完整门禁。当前事实只维护在 [STATUS](../STATUS.md)。

## 目标与授权边界

目标：先保障软件可靠性，以作者本人的获准行为数据回答可测问题，研究程序算法的反例与架构的运行成本/故障边界，并留下能在后续项目独立读取的研究数据。
技术路径：Rust 采集与加密存储 → Session 用途过滤及生命周期 → 独立 Tauri IPC → React 明确预览和确认；JSONL 是派生留存，不替换主库。
本计划依据 [PRODUCT_LOCK](../PRODUCT_LOCK.md)、[DECISIONS](../DECISIONS.md) D107；D107 的产品文字同步落定后才进入新增落盘实现。

- 仅释放本机显式请求、字段白名单、`self` 且 `derived/aggregate` 的研究留存；现有研究预览继续只读，另设保存动作。
- 采集同意不自动授予研究保存；已有行级研究用途/粒度限制仍须满足。`bucket` 许可不能升级为任意原始事件导出。
- 先作者自己，未来少数知情参与者。本文不授权读取真实库、开启采集、招募、安装或外网调用。
- 排除本人全量导出、raw 聊天、`third_party/mixed/unknown`、标题、路径、密钥、批准状态和队列状态。
- 非目标：完整应用恢复、换机/重装恢复、跨机密钥、整个系统迁移、云训练与模型效果承诺。
- N3、生产恢复、MODEL 扩展不作为本轮必选；保留 D106 N2、既有恢复原型与 VAL 工具，不复制另一套。
- 不新增 hash、冻结 contract、baseline 或 gate；不改冻结 T4D/A0/A1/A2/A3，不降低既有授权、遗忘和验收要求。

## 现有证据与能力差集

| 证据位置 | 当前可确认事实 | 对计划的约束 |
| --- | --- | --- |
| `crates/soul-collect/src/session.rs` | 密封正文只有 `app` 和 `duration_ms`；事件起点另有时间戳，时钟倒退时长饱和到零 | 应用名/时长存在于库不等于研究保存获准；零值可能是时钟异常，不能推断未活动 |
| `crates/soul-collect/src/collector.rs` | 采集事件研究形态为 `bucket` | 首片只使用获准桶粒度，不解封后原样外写 |
| `crates/soul-store/src/research_preview.rs` | 按 kind、时间桶、subject、disposition 聚合计数；非 UTC 表达退化为日期桶 | 小时桶和日期桶不得混称小时精度；现有候选查询不直接等于新保存路径的遗忘过滤证明 |
| `crates/soulcore/src/commands/store.rs` | 预览字段有计数、时间桶、特质档等，`duration_bucket` 为可选字段 | 字段存在于类型不证明生成了值；新路径不能把未知时长补成零 |
| `crates/soulcore/tests/session_research.rs` | 已有本人导入/问卷原始行拒绝、第三人排除与预览不落盘断言 | 保留这些断言；新保存动作需要独立测试，不修改旧断言放宽预览 |

## 首片交付：研究问题到可读取记录

首片先交付字段/来源与覆盖差集，再决定最小代码变更。默认主记录为获准前台活动桶，确实保留本人行为观测，不仅保存程序诊断统计。

| 研究问题 | 记录粒度与候选字段 | 现有来源 | 许可与独立读取断言 |
| --- | --- | --- | --- |
| 本人在已观测时间段的前台活动记录如何分布？ | 每事件类型 × UTC 小时桶；`event_kind/time_bucket_utc/aggregate_count` | collector 事件及受限留存查询 | 仅当前 `self + bucket`、未遗忘且本次研究保存确认；销毁测试 Session 后独立 JSONL 解析得到与合成期望一致的桶和计数；旧屏幕预览的日期降级不用于保存 |
| 哪些区间没有可解释观测？ | 同一观测区间的覆盖/缺失说明，缺失原因仅写实际可证事实 | 既有采集状态与测试控制轨迹；真实历史覆盖能力须核对 | 未记录的历史不补造；未知覆盖写未知，计数空缺不解释为零活动；不从审计拼接个人行为时间线 |
| 是否能比较不同应用的使用时长？ | 本片不输出应用名、精确起止或原始 `duration_ms`；`duration_bucket` 未实际生成时保留未知/不支持说明 | 密封正文有原始值，但现有 rollup 未证明可用时长桶 | 不能借 `bucket` 名义导出原始值；需要新研究问题与合规派生设计时另议，本片明确拒绝请求 |

字段交付物必须逐项给出类型、单位、粒度、缺失含义、来源、许可和排除理由；未支持字段明确拒绝或未知。
首批留存排除 profile/特质轴派生行，避免把现有轴预览的许可张力带入本轮；算法研究可在合成 fixtures 中保留相关反例。
桶计数代表已存储并获准的事件数量，不代表总操作次数、真实活动时长或全天覆盖率。
覆盖率仅在分母可证时计算：区分计划观察时段、实际可观测时段、获准时段和纳入记录；断电/退出期间不得补采或推断。

### 首批字段与来源说明

| 项目 | 类型、单位与粒度 | 来源和许可 | 缺失或排除含义 |
| --- | --- | --- | --- |
| `event_kind` | 字符串，首批仅前台应用事件类型 | 同时限制 `CollectorForegroundApp` 来源、`AppForeground` 类型、本人 actor/subject | 其他来源与 profile 行不进入本次留存 |
| `time_bucket_utc` | 字符串；标准 UTC 时间的小时桶 | 取事件起点，不解封正文；受限查询验证日历合法性及固定 UTC 表达 | 非标准表达与无效时间明确排除，不补零；旧预览的日期降级不进入留存 |
| `aggregate_count` | 无符号整数，单位为纳入的事件条数 | 源行须含研究用途、`research_export=bucket`，且关联正文及内容密钥仍有效 | 不是采样次数、活动时长或全天操作数；未出现的桶不代表零活动 |
| 纳入/排除统计 | 分别命名事件数和输出桶数 | 只对本次明确范围实际检查的候选计数 | 现有预览的排除数是候选组数，不能复用为排除事件数；不推算全历史被遗忘数量 |
| 覆盖与缺失说明 | 有证据的状态说明；分母未知时不提供百分比 | 合成实验可用脚本轨迹作分母，真实历史只用已记录事实 | 采样间变化、未完结会话、程序关闭期和真实休眠期间的活动不可由桶计数恢复 |

采集源行的 `derivation` 当前为 `Raw`；D107 允许的是将获准源行聚合后的输出留存，不能错误要求所有源行已是 `aggregate`，也不能因此导出原始正文。应用名、精确起止与 `duration_ms` 不在输出字段白名单。

### 代码实施顺序与依赖

1. **采集计量证据先行。** 一个实现者认领独立测试文件，主线程统一运行真实临时加密库测试，独立评审者核对断言；只修实际复现的问题。
2. **留存按存储查询 → Session 编排 → Tauri/React 串行接线。** 先落实上述过滤和独立读取格式，再做预览、一次确认、取消、暂存及最终发布；既有预览不承担保存。接口和权限明确后，独立文件的读取器/测试可以并行，Session、IPC、schema 与锁文件始终单写者。
3. **算法与架构实验复用既有入口。** 只认领未覆盖的具体反例或测量；完整构建及平台操作仍由主线程统一排队。每个切片分别交付实际代码、运行记录、解释边界与评审结果，不以一张总 PASS 代替。

实施前待决的范围/粒度、确认消费/取消和完整发布细节，已由[第二批实现选择](#第二批实现选择2026-10-20)及其运行回执解决：保存仅采用标准 UTC 小时桶，三项内容装入单个 ZIP 后完整发布，不覆盖已有目的地，不沿用旧 preview 默认值。采集撤权和研究保存取消是不同动作，现有 `revoke_collect_consent` 不能擅自解释成撤回所有历史研究用途。

保存重验必须核对精确贡献源与当前许可：遗忘 A 后新增同桶 B 即使计数不变，也不能接受 A 的旧候选。现有 Session/store 锁可用于共享句柄的最终核验，但不能据此声称覆盖第二 SQL 连接；实际并发与文件发布边界须由保存切片的测试证明。

## 切片 1：数据测量与可靠采集

- [x] 对照 `soul-collect` 生命周期与 `session_collect/collect_probe/session_research` 现有用例，交付覆盖差集与上述字段映射。
- [x] 用合成轨迹核对默认关闭、撤权、停止、采样长间隔/时钟跳变、短会话与数据库重开；核对“关闭后无新写入”而非只看返回成功。真实 OS 休眠与进程崩溃不在本次结论中。
- [x] 给出已采/排除/未知的分母与误差来源；区分时间饱和零、未采集、未完成会话、被遗忘和真正零计数。真实历史没有可证覆盖分母，不生成覆盖率；保存路径的遗忘过滤已由切片 4 实现并完成第二批合成验证。
- [x] 只有新增反例实际失败才补最小修复及回归；本次五组刻画符合现有语义，生产逻辑 NO-CHANGE，新增的是计量边界测试。

后续任务可认领：`crates/soul-collect/src/{collector,session,runner,consent}.rs`、`crates/soul-collect/tests/`、`crates/soulcore/tests/{session_collect,collect_probe,session_research}.rs`。
依赖：不修改采集授权语义；涉及 Session 接线时由主线程串行认领。验收：合成计数可重算、拒绝路径零新增写入、缺测不伪装为零。

## 切片 2：程序算法的可解释反例与消融

- [x] 时间与聚合：第三批覆盖正逆序、offset、UTC 跨日、小数秒与缺失/非法时间，核对全图 `as_of`；展示排序反例已复现并修复。当时未决的结构缺损语义已由第四批 D108 明确并实施。
- [x] 检索候选与证据：第三批覆盖默认上限下的反证位置交换、遗忘后失效与补位；关键词缺失沿用 MODEL-01 已实测结果，引用与密封正文逐项核对。
- [x] 消融仅用合成材料删除时间字段、改变候选位置或遗忘已选项，记录 8/9 选择及 8/8 剩余可用项、具体反例和不可判定语义；未改生产判档。
- [x] 复用 VAL-01 的缺测、失败、配对与离线重算以及 MODEL-01 关键词反证回执，未复制操作工具或重复运行同覆盖面；合成结果不宣称真人效用。

后续任务可认领：`crates/soulcore/tests/session_memory_context.rs`、`crates/soulcore/tests/val01_evaluation/` 的具体无冲突文件；算法 crate 只读起步。
依赖与验收：先查现有断言差集；无新反例不加代码。发现冻结口径问题只报告证据并走原决定流程，不改常量或加新阈值。

## 切片 3：架构路径、性能与故障

- [x] 已逐层追踪采集 → 加密库 → Session 查询/遗忘 → Tauri command → React，输入、失败传播与责任见第三批路径表。
- [x] 第三批实际测量 100/1000 合成输入下的导入（含 rebuild）、people 查询、正向研究聚合及空载 fake runner 撤权至返回；原始值、机器和未测项见回执，不称一般负载的停止延迟。
- [x] 复用第二批并发/遗忘/文件进程退出/真实 IPC/UI 旧结果回执，本批新增规模下的失效快照拒绝和撤权后真实事件身份集合不变；未重新签发桌面或崩溃验收。
- [x] 本批只修已复现的展示排序错误，未据小样本重写架构或增加性能门槛；已有 `q2_scale` ignored 入口及 clean SHA 条件保持不变，未运行或绕过。

后续任务可认领：`crates/soulcore/tests/q2_scale.rs`（先只读）、既有 Session 故障测试、`apps/desktop/src/routes/{Collect,Research}.test.tsx`。
根 Rust、独立 desktop workspace、React 分别给证据；Windows 真机采集/托盘观察属于后续作者明确执行，不能由合成测试签发。

## 切片 4：本机可机读研究留存

前置：D107 产品边界已同步，前三片的许可、遗忘及来源差集已明确；任何不支持的字段不阻碍先交付窄白名单。
本批已完成独立设计审查、分路径实现及独立代码评审；下列交付约束对应文末第二批实测回执，不代表正式平台或真人验收。

- [x] 单独展示研究保存用途、字段、桶精度、实际行数、时间范围、排除/缺失说明与用户选择的本机目的地，再明确确认。
- [x] 保存 UTF-8 `records.jsonl` 主记录、`metadata.json` 与字段说明于单个标准 ZIP；未另做 CSV 权威副本。
- [x] metadata 记录格式版本、生成程序版本、范围/粒度/单位、纳入及排除计数、缺失语义；不包含真实源路径、密钥或批准/队列状态。
- [x] 只保存当前获准且未遗忘材料；生成到最终提交期间撤权或遗忘拒绝旧结果，不靠预览时检查替代提交检查。
- [x] 不复用前 N 行预览充当完整留存；完整范围保存，超资源上限拒绝而不截断。
- [x] 明确告知这是明文、可机读、独立副本。获准原始数据继续在加密库；不新增密码恢复或跨机密钥工程。
- [x] 原库遗忘不自动撤销已复制文件；用户需自行处理既有副本，不承诺全部副本消失。内部派生缓存仍遵守遗忘。
- [x] 审计只记批准/拒绝与既有理由码，不写目的地、观察小时、正文或来源编号。
- [x] 合成验收覆盖 self/raw/third_party/mixed/unknown、撤权/遗忘竞态、取消、写入失败、已有目标文件、路径变化和重试；已有目标即拒绝，不覆盖。未发布数据由 owned handle 清理，不删除先前成功副本或用户文件；设置删除标记前失败可能留下空文件的限制见下文。
- [x] 已销毁测试 Session 后，独立 Python 进程不访问库、密钥或网络而解析实际生成的 ZIP/JSONL，核对字段、粒度、行数与计数；没有启动或关闭用户正在使用的 Soul。

预期实现面：`crates/soul-store/src/research_preview.rs`、`crates/soulcore/src/commands/store.rs` 先只读复用；新增保存逻辑应另设模块，不使预览模块写文件。
新增模块的具体路径在实施任务卡认领；实际入口涉及 `crates/soulcore/src/commands/session.rs`、`apps/desktop/src-tauri/src/commands.rs`、IPC 类型/注册表，均为主线程串行热点。
React 可认领 `apps/desktop/src/routes/Research.tsx` 与对应测试；schema 不变。根 Cargo 与锁文件的本次窄调整见下述实现选择。
验收同时要求加密库/Session 语义、真实 IPC 与 UI 旧结果保护；没有明确设计和测试的失败写入清理策略不能记完成。

### 第二批实现选择（2026-10-20）

本次用户明确要求推进查询、确认/取消、文件写入与独立读取，验证只用合成材料。范围为来源 `collector.foreground_app`、事件 `app.foreground`、actor/privacy 均为 self、purpose 含 research、disposition 为 bucket 的未遗忘事件；Raw 只作为聚合输入，不解封正文。输出仅事件类型、UTC 小时桶、事件数，不保留应用名、时长、个人编号或 profile。范围采用 UTC 整日半开区间；合法的非标准 UTC 时间格式明确计入排除，不假装已归一化。缺失小时不补零，事件数也不是观察时长或覆盖率。

查询读取完整选定范围，不复用 50 行屏幕预览。超过 100000 个候选或 16 MiB 私有来源元数据即拒绝，完整归档超过 1 MiB 同样拒绝而不截断；这些是本机资源上限，不是算法阈值。元数据中的统计来自准备时刻；发布前重新核验相关来源的身份、政策、blob/key 绑定和存活状态，范围内变更或同数量替换拒绝旧请求，范围外新增不影响已选范围。重新核验到文件发布之间使用既有 `BEGIN IMMEDIATE` 防其他连接改写。

交付为单个无压缩标准 ZIP，内含 UTF-8 `records.jsonl`、`metadata.json`、`README.txt`。三项先有界生成于内存，ZIP 的 CRC32 仅为标准格式字段，不新增 hash、冻结规范或门禁。独立 Python 标准库读取器在不访问 Soul 库、密钥或网络时解析并核对内容，不解压写盘，不提供恢复功能。

Windows 文件边界单独置于 `soul-local-files` 小 crate，以使 core/store 继续禁止 unsafe；根 workspace 因而从 18 增为 19 个 crate，两份 Cargo.lock 只增加此本地依赖，不更换第三方版本。目的地要求普通固定本地卷上的绝对路径，拒绝已有文件、网络/设备路径、SUBST 与 reparse 祖先；持有祖先句柄防路径替换。最终 `create_new` 独占创建，先将 owned handle 标为待删除，再写入并同步，最后清除待删除标记作为发布点。取消与这一发布点串行判断，发布前取消删除本次未提交文件，发布后返回已发布状态且保留文件。普通错误与进程退出由操作系统关闭句柄清理；创建后设置待删除标记失败或该间隙进程退出最多留下空文件，不按路径删除潜在替换目标。这里不宣称断电持久性。

该实现依据 [Windows FILE_DISPOSITION_INFO](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_disposition_info) 的普通句柄删除标记语义，不能使用无法撤销的 `FILE_FLAG_DELETE_ON_CLOSE`；是否可用仍以本机合成写入和子进程退出测试为准。保存后的独立副本不会随原库遗忘而消失。已有一般文件计划保持只读；本次是单独、一次性的研究保存批准，不开放通用 FileWrite token。

审计沿用现有 `ConsentGrant` 记录本次保存批准，`HitlDeny` 记录取消/拒绝；不将批准冒充保存成功，不将目标路径、小时桶或来源标识写入审计。发布成功后数据库事务结束异常仍返回已保存及事务警告，不能误报无输出或删除已发布文件。

## 首批实际验证安排

以下是本轮合成验证入口；实际运行与偏差见文末回执。由主线程持有唯一构建通道，使用现有 target。
在仓库根目录以 `pwsh` 执行，使用合成 fixtures 和临时库；离线依赖不足如实报告，不安装或更新锁文件。
Windows 上必须排除 `windows::runs_on_windows`：其中两项直接调用真实前台源，并非合成用例。该组保留在源码，作者明确授权真机观察后再单独安排，不以其绿色结果充当采集验收。

```pwsh
$ErrorActionPreference = 'Stop'
cargo test -p soul-collect --locked --offline -- --skip windows::runs_on_windows
if ($LASTEXITCODE -ne 0) { throw 'soul-collect failed' }
cargo test -p soulcore --test session_collect --test collect_probe --test session_research --locked --offline
if ($LASTEXITCODE -ne 0) { throw 'collection/research Session tests failed' }
```

切片 2 的既有入口为 `cargo test -p soulcore --test session_memory_context --test val01_evaluation --locked --offline`，运行后立即检查 `$LASTEXITCODE`；ignored 实验不自动启用。
切片 3/4 若触及桌面，另排 `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again` 与 `pnpm --filter @soul/desktop test`，分别检查退出码。两项排除用例在 Windows 会调用真实前台源，本轮不执行、不计为覆盖。
前端 lint、格式和完整门禁按实际修改由主线程安排；根测试不代表 desktop，局部通过不更新 G-L/G-W/G-M 结论。
网络仅受控 loopback mock；日志放仓库外且不含真实正文、路径或凭据。回执记录命令、退出码、环境、SHA/dirty 和未覆盖项。

## 后续实际观察与停止条件

建议在上述可靠性及留存断言有证据后，由作者另行明确开启获准采集，观察 7–14 天；这是观察建议，不是代码工期或通过门槛。
记录数据覆盖、缺失、软件故障和研究问题是否可回答；不以样本天数取代有效覆盖，不把作者单例推断为群体效果。
未来参与者须单独知情同意用途、范围、明文副本、撤回与遗忘限制；当前不招募、不导入其数据。
出现授权/遗忘失败先停止相关留存并修复；研究问题缺乏所需粒度时报告限制，不擅自扩大采集。
每片提交覆盖差集、具体反例、实际路径与验证记录；无新问题即可收口，既有代码保留，不强制产生功能修改。

## 前轮文档回执（代码批准前）

本轮新增本计划，并同步 PRODUCT_LOCK、DECISIONS D107、STATUS、PLAN_INDEX、ROADMAP、GOAL2_PLAN 与前阶段计划的入口；此前 dirty 代码及文档保留，未提交。

主线程对本轮编辑前的仓库外文本副本执行增量比较，检查新增 14 个本地链接与 7 个实际测试文件路径，均存在；新增正文 UTF-8 读取、尾随空白/占位符检查及既有文档 `git diff --check` 均退出 0。新计划尚未跟踪，另直接检查正文，没有将 Git 忽略未跟踪文件的空结果当作检查覆盖。

独立只读评审覆盖本轮 7 个既有文档增量及新计划，发现一处失败清理对象的措辞歧义；已明确只清理本次暂存输出，不覆盖/删除既有文件，针对性复核关闭该项，未发现剩余具体缺陷。结论仅为规划一致性。

未执行构建、产品测试、性能测量、真实数据读取、采集、研究文件保存、安装或平台门禁：本轮交付是已同意方向的计划与权限边界整理，不声称实验能力已实现或当前候选通过。

## 首轮代码与运行回执（2026-10-20，代码批准后）

候选为 `6a56d852d23a051c7c4e9845642791295c541105` + dirty，沿用当前工作区并保留原有改动，未提交。一个实现者新增 [measurement_boundaries.rs](../../crates/soul-collect/tests/measurement_boundaries.rs)，主线程统一执行；两个只读调查分别确认采集覆盖差集与留存源/权限接口，独立评审者核对新增测试和字段/实施依赖。未改生产采集逻辑、Session、IPC、schema 或锁文件。

五项新增实验使用脚本时间/前台源、真实临时 SQLCipher 库及 `BlobStore::open` 解封，不以 `Poll` 返回值代替落库事实：

| 轨迹 | 实际断言通过的结果 | 不能推导的结论 |
| --- | --- | --- |
| A@0 → B@125 → 空@200/500 → A@1000 → 结束@1100 ms | 三段为 125/75/100 ms；同秒事件身份独立，显式空档不并入相邻段 | 空来源期间实际没有活动 |
| 两次采样间 A → B → A | 只保存 A 的 1100 ms 段 | B 的真实使用时长为零、A 连续可见 |
| A@5000 → B@4000 → 结束@6000 ms | A 饱和为 0，B 为 2000 ms，均保留事件 | 0 ms 表示没有活动；当前数据有可区分所有零值的质量标记 |
| A@0 → A@3600000 → B@3601000 → 结束@3602000 ms | 两段为 3601000/1000 ms | 一小时内持续采样，或已完成真实休眠/唤醒测试 |
| A 完结、B 未完结时丢弃同步 Collector 并关闭库，再重开采 C | A 原 ID 及正文可读；B 未保存；C 仅 500 ms | 进程崩溃/WAL 故障验收、未完成段自动恢复 |

已采分母只能指向脚本中实际送入采集器的观测；相邻观测之间、未完成段和关闭期间的活动未知。已有撤权回归证明拒绝后的写入边界，不等于全部历史缺失可分类。源 Raw 行仅作为获准桶聚合输入；新的留存查询、文件生成和独立解析尚未实现。

运行环境：Windows `x86_64-pc-windows-msvc`，Rust/Cargo 1.83.0，现有根 `target`。最终测试仅在子进程设置 `CARGO_BUILD_JOBS=4`、DEV/TEST DEBUG=0、INCREMENTAL=0，并复用既有 OpenSSL 3.6.3 静态库（`apps/desktop/src-tauri/target/debug/build/openssl-sys-8faadaede0bdeb1e/out/openssl-build/install`，NO_VENDOR=1、STATIC=1）；未改全局环境或安装依赖。

| 实际命令 | 结果与范围 |
| --- | --- |
| `cargo test -p soul-collect --locked --offline -- --skip windows::runs_on_windows` | 退出 0；31 项通过（包含新增 5 项），3 项 Windows 源用例过滤；doc-tests 0 |
| `cargo test -p soulcore --test session_collect --test collect_probe --test session_research --locked --offline` | 退出 0；分别 9、3、1 项，共 13 项通过；采集使用 FakeForegroundSource，probe 只执行无同意拒绝路径 |
| `cargo fmt -p soul-collect -- --check` | 首次退出 1，仅新增文件 3 处换行；按输出修正后退出 0 |

最终定向测试共 44 项，不重复累计先前运行。日志位于仓库外 `C:/Users/86080/AppData/Local/Temp/soul-collection-research-0795faf48521410abf21d170fdc0b97f/`：`collect-synthetic-final.log`、`session-final.log`、`collect-format-initial.log`、`collect-format-final.log` 与 `environment.log`。

执行偏差保留：首次无环境覆盖的 crate 命令在重编译 vendored OpenSSL 时由主线程停止，尚未进入测试，进程退出 1；停止命令因一个子进程已退出返回 1，随后确认本次进程树已退出，才复用静态库重试。对应输出为 `collect-initial.log`。第一次重试未加过滤，34 项通过（`collect-final.log`），其中既有 Windows 源的两项测试直接调用前台查询共 6 次；这是本轮合成验证范围控制的疏漏。采样值仅在测试进程中用于断言或丢弃，未写入库/文件，实际通过输出不含应用名，未启动持续采集。已定向核对该组源码，更新本计划命令并实际过滤重跑；不能把第一次重试称为纯合成测试，也不把它列为作者真机采集验收。

独立只读评审未发现新增测试或字段/依赖说明的具体缺陷；评审不代替运行证据。未运行独立桌面 workspace、React、真实休眠/崩溃、性能规模实验、算法新增实验、研究保存、G-L/G-W/G-M、安装卸载；留存仍待实现，正式 Goal 状态不变。

## 第二批代码与运行回执（2026-10-20）

本节更新第二批留存的实现事实；上节“留存待实现”仅描述首批结束时点。候选仍为 `6a56d852d23a051c7c4e9845642791295c541105` + dirty，保留先前工作区修改，未提交、推送、合并、安装或发布。

用户本次明确要求受限查询、保存确认与取消、文件写入、独立读取、测试与独立评审。两个实现者分阶段认领不重叠的存储/归档/前端及 Windows 文件模块，主线程负责 Session、IPC、共享配置、集成和唯一重构建通道；独立评审者只读核验。范围与选择见上文，算法常量、schema、一般文件计划与 E1 许可不变。两个锁文件仅增加本地 `soul-local-files` 包/依赖，未变更第三方版本。

实现入口：

- [受限查询与发布复验](../../crates/soul-store/src/research_retention.rs)：完整范围聚合、互斥排除计数、精确私有来源快照、既有即时事务。
- [Session 保存编排](../../crates/soulcore/src/commands/session_research_retention.rs)：一次确认、取消与发布胜负、拒绝审计、重启/替换不恢复批准。
- [Windows 文件发布](../../crates/soul-local-files/src/windows.rs)：本地固定卷、目录保护、不覆盖创建、错误/取消/进程退出清理。
- [ZIP 编码](../../crates/soulcore/src/commands/research_archive.rs)、[独立读取器](../../scripts/read-research-retention.py)、[研究保存面板](../../apps/desktop/src/routes/ResearchSave.tsx)及实际 Tauri handler。

使用方式：在研究页“保存本人研究数据”填写 UTC 开始日期、结束日期（不含）及普通本机磁盘上已有文件夹中的新 `.zip` 绝对路径；准备后核对用途、范围、计数和目的地，再单独确认。归档内固定三项，记录只含 `event_kind`、`time_bucket_utc`、`aggregate_count`。独立运行 `python scripts/read-research-retention.py <保存文件.zip>`，成功输出版本、行数与计数摘要；错误退出 2，不回显内容或路径。读取器不需要运行 Soul，也不会写回数据。

### 实际执行与覆盖

Windows `x86_64-pc-windows-msvc`、Rust/Cargo 1.83.0、Python 3.12.14，根和桌面使用各自既有 target。Rust 测试/Clippy 子进程设置 `CARGO_BUILD_JOBS=4`、DEV/TEST DEBUG=0、INCREMENTAL=0，并复用既有 OpenSSL 静态库（同首轮目录，NO_VENDOR=1、STATIC=1），未安装依赖或改全局环境。前端沿用 2 workers，生产构建临时设 `NODE_ENV=production` 并在 finally 恢复。

本轮仅合成 fixtures、临时 SQLCipher 库、合成前台源、临时文件与既有受控 loopback mock；没有调用真实前台源或读取真实聊天、用户库、密钥及凭据。实际 DPAPI 操作只针对新建的测试 Session。独立读取用例先 drop 测试 Session，再启动 Python 进程解析该用例刚保存的文件。

| 当前候选实际检查 | 结果与范围 |
| --- | --- |
| `cargo test -p soul-store --lib research_retention --locked --offline`；`cargo test -p soul-store --test research_retention --locked --offline` | 7 unit + 7 integration 通过；含 72 小时桶完整输出、主体/用途排除、无解封、同数量替换、遗忘与 blob/CK 重绑定、两连接事务、发布后事务错误、资源超限 |
| `cargo test -p soul-local-files --locked --offline` | 3 unit + 11 integration 通过；两个 helper/平台特权用例默认 ignored，子进程 helper 由正常测试实际调用，不重复计数 |
| `cargo test -p soul-local-files --test publication --locked --offline -- --ignored --exact reparse_parent_is_refused` | 1 项实际通过；本机具备创建合成符号链接的权限，没有将权限不足当通过 |
| `cargo test -p soulcore --lib research_ --locked --offline -- --include-ignored`，显式提供 `SOUL_TEST_PYTHON` | 14 项通过，包含默认 ignored 的独立 Python 读取；其余覆盖归档边界、取消后写入清理、Session Drop、迟到取消不撤回已发布、确认重放与拒绝审计 |
| `cargo test -p soulcore --test session_research --test session_screens --locked --offline` | 1 + 10 项原有回归通过，旧研究预览仍不落盘 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again` | 130 项通过，明确过滤 2 项会启动真实 Windows 采集的既有测试；新增实际 handler 测试从 FakeForegroundSource 落库到准备、取消和保存，检查 origin、camelCase 参数、一用确认及旧预览只读 |
| Python `-B -m unittest discover -s scripts/tests -p test_research_retention_reader.py` | 7 项通过；正常/空包、损坏、未知版本、缺项/额外项、布尔伪计数、时间/字段/排序/守恒与 CLI 拒绝 |
| `pnpm --filter @soul/desktop lint`；`pnpm --filter @soul/desktop test`；`pnpm --filter @soul/desktop build` | 均退出 0；55 个测试文件、685 项通过；生产 bundle 成功。其中新保存组件 13 项和真实 wrapper 4 项覆盖确认、取消、输入变化、卸载重挂与准备派发顺序 |
| `cargo clippy -p soul-store -p soulcore -p soul-local-files --all-targets --locked --offline -- -D warnings` | 退出 0 |
| 两个 workspace 的 `cargo fmt --all -- --check`；既有 `schema-freeze --check`、`e0-audit`、`denylist-audit`；`git diff --check` | 均退出 0；未新增 gate/hash/freeze |

所有检查由命令后实际退出码判断，重复 RED/GREEN 不累计成新增通过数。原始日志在仓库外 `C:/Users/86080/AppData/Local/Temp/soul-retention-fb0730d6144a49939bfe44d17b402624/`：`store-green.log`（unit，integration 被过滤）、`store-integration-green.log`（实际 integration）、`file-green-final.log`、`file-reparse.log`、`core-candidate-final.log`、`core-regression.log`、`independent-read.log`、`desktop-candidate-final.log`、`python-reader-final.log`、`frontend-{lint,test,build}.log`、`root-clippy-final.log`、`static-checks.log`。部分日志只有原始测试汇总，退出码来自执行工具与脚本检查，不补造日志中不存在的标记。

### 失败、修复与限制

- 测试先写 stub 再实际 RED：存储、归档、Session、文件与 UI 正常路径均先因未实现失败。初次 Session RED 的测试代码误用了不存在的 forget API，编译失败；纠正为真实 CK 遗忘接口后才获得行为 RED，不把编译失败当行为复现。一次存储组合命令的名称过滤导致 integration 运行 0 项，随后单独执行全部 7 项通过，未把 0 项记作覆盖。
- Windows 实测发现仅 `FILE_READ_ATTRIBUTES` 的目录句柄未阻止改名/写句柄。临时合成诊断矩阵实际得到 `0x80` 允许、`0x81` 和 `GENERIC_READ` 拒绝；据此改用 `0x81` + 仅共享 READ，保留反例回归并通过，临时诊断代码已移除。内部路径绑定验证后的 Volume 设备，不在最终写入时重新追随用户盘符映射。
- 独立评审发现拒绝审计缺失、旧 prepare 并发覆盖新预览两项 P2，均先复现后修复并复审闭环，见[评审记录](../reviews/2026-10-20-research-retention.md)。初次桌面运行因转发结构测试尚未认识新 async 命令而失败，补入等价调度断言后通过；严格 Clippy 的布尔简化和测试多余借用已按提示修正，未关闭 lint。
- 写入故障使用部分写入后的注入错误，以及真实 sync 后注入完成错误；不冒称测了真实磁盘写满。进程退出用例实际使用子进程无析构退出，验证发布前删除、发布后保留；未验证整机断电、SSD 物理擦除或任意系统崩溃。
- 当前文件发布支持 Windows 的普通固定本地卷；非 Windows 明确拒绝写入，独立 Python 读取器不依赖 Soul。已有目标不覆盖，路径/reparse/资源上限拒绝；设置删除标记前的极短失败窗口可能留下空文件。明文副本在成功保存后由用户管理，原库遗忘不会删除它。

未运行真实数据研究、真实采集、参与者试用、算法新增实验、规模/性能实验、桌面人工点击/安装卸载、完整 G-L/G-W/G-M。没有代勾人工清单或关闭 Goal 1/U7/正式 Goal 2；本批已完成的是获准的代码与合成验证闭环。

## 第三批实施范围（2026-10-21）

接续前实读 Git 状态，起点仍为 `6a56d85 + dirty`；独立评审核对第二批最终日志，补齐其前端证据并发现一项过时计划措辞，已修正，未发现新的实现阻塞。沿用当前工作区并保留已有修改，不提交或重新签发平台门禁。

覆盖差集限定为以下三组；两个实现者只写互不重叠的新测试文件，协调者维护文档、运行与集成，完成后另作独立只读评审。

1. 时间组合：真实加密库中的三次跨 UTC 日往来，正/逆序、规范 UTC/等价 offset 表达，核对计数、统一 `as_of`、证据和展示 first/last；在现有导入 API 做规范化对照。另分别删除时间字段或改为非法值，记录识别/拒绝行为差异；结构缺损的跳过/拒绝语义未有明确决定时只报告观察，不自行制定新规则。认领 `soul-graph/tests/research_time_boundaries.rs` 与 `soulcore/tests/research_selection_boundaries.rs`，不改算法常量。
2. 候选消融：真实 Session 写入默认选择上限加一条短记忆，以候选中反证的位置变化观察纳入/排除；遗忘已入选项后旧上下文拒绝，新选择补入剩余候选，引用可解封且遗忘正文不可读。复用 MODEL 的关键词反证与 VAL 的既有证据，不复制其工具或把本实验当模型语义判断。与第一组共用已认领的 core 测试文件。
3. 运行成本：独立 ignored 的 `soulcore/tests/research_path_probe.rs`，100/1000 条合成输入，各三次新临时库观测。分别计时 Session 导入（commit 含 rebuild）/people，以及 FakeForegroundSource → Collector 的真实逐事件写入、受限聚合、无文件发布闭包的来源复验事务和遗忘后查询；采集停止仅通过显式 fake source 的 runner。只报告原始值和 min/median/max，不设性能阈值，不运行或绕过要求 clean SHA 的 Q2。初始化、fixture 生成和断言不计入操作计时；预热与样本分开。

本批只用合成材料和临时库；不启真实前台源、外部端点或用户应用，不读取用户数据。普通定向测试与显式 ignored 诊断由协调者串行运行，必要格式/Clippy 及已有相邻回归按改动面执行；若反例实际失败，先定位与保留证据，再判断修复范围。不以将预期改为当前错误行为来伪造通过，不因静态疑点修改生产算法。

### 实际架构路径与边界责任

以下为本批按调用位置追踪所得的静态事实；运行证据按批次另记，不以源码阅读替代测试。

| 层与入口 | 输入及责任 | 失败、遗忘与取消边界 |
| --- | --- | --- |
| [Collector](../../crates/soul-collect/src/collector.rs) / [runner](../../crates/soul-collect/src/runner.rs) | 前台源与采样时刻组成已观测会话；正文 seal 后写事件，统计不等于连续观察 | 写前及取得 store 锁后复查同意；撤权丢弃未完结段，正常 finish 可保存；runner stop 请求退出并 join |
| [受限查询](../../crates/soul-store/src/research_retention.rs) / [遗忘事务](../../crates/soul-store/src/forget.rs) | 查询获准的 self/bucket 来源、时间和 blob/CK 可用性；输出桶与私有来源快照 | 遗忘在事务中删除 CK 并记销毁身份；发布复验在即时事务内核对精确来源，拒绝旧快照。查询本身不解封正文或写文件 |
| [Session](../../crates/soulcore/src/commands/session_research_retention.rs) | 准备时持有范围/目的地/来源；一次批准转为锁外 worker，生成完整归档 | 取消先设状态再记审计，发布点与取消串行；来源失效、文件错误返回拒绝；发布后的事务关闭异常保留成功收据及警告 |
| [Tauri commands](../../apps/desktop/src-tauri/src/commands.rs) | prepare/cancel 用 blocking worker；confirm 的 begin/complete 持 Session 锁，execute 在锁外 | 真实 handler 返回类型化结果，worker 异常传播为拒绝；第二批合成 IPC 已覆盖 origin/参数/批准一用，不能以本批 root 测试替代 |
| [React 保存面板](../../apps/desktop/src/routes/ResearchSave.tsx) / [wrapper](../../apps/desktop/src/core.ts) | 单独准备、明确确认、取消与收据；跨卸载的 prepare 队列保持派发顺序 | 输入改变/卸载取消旧请求，迟到准备不能覆盖新界面；发布成功收据保留原请求事实。第二批组件/wrapper 实测保留，本批未改 UI |

跨层失败策略的实际运行基础沿用第二批的两连接事务、来源变化、worker/store 锁期间取消、Windows owned-handle 清理、真实 IPC 及卸载重挂回归。第三批只新增未覆盖的时间表示、候选默认上限组合和正向聚合规模观测，不重复复制故障框架。

## 第三批代码与运行回执（2026-10-21）

候选仍为 `codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`，此前修改保留。新增两个普通实验测试文件和一个默认 ignored 诊断文件，生产改动仅 [graph build](../../crates/soul-graph/src/build.rs) 的展示时间排序；本批未改依赖、锁文件、schema、IPC、React、冻结 scorer 或算法权威。独立只读评审已检查测试设计及生产差异，未发现具体缺陷；最终运行回执核对另记于[本批评审](../reviews/2026-10-21-research-experiments.md)。

### 算法结果、修复与未决语义

| 实验 | 实际结果 | 解释边界 |
| --- | --- | --- |
| UTC/offset、输入正逆序及跨 UTC 日 | 三条往来落在两个 UTC 日；第二 peer 的 12:15 锚点使所有 edge 共用该 `as_of`，主 peer 最近往来仍为 00:15；引用均可解引用 | 正序与逆序也反转同一 evidence 内 source_refs，避免 SQL 排序使置换失去作用 |
| 带偏移和小数秒的展示极值 | 初次 offset 反例把实际前一天 23:30 排在次日 00:00 之后；新增小数反例把 `.12Z` 排在整秒之前、把 `.9Z` 排在 `.91Z` 之后，实际 RED 为 3 失败/3 通过 | 根因是原始 RFC3339 字串比较。正式导入对照先统一 UTC/整秒，本轮未复现该正常路径受影响；直接证据 API 的 adapter 明确接受 offset/小数，故修展示比较而非拒绝合法输入 |
| 展示排序修复 | 采用 adapter 已验证的 UTC 整秒、小数数字去尾零和原文稳定平局顺序；保留原文，不用浮点或截断到纳秒。first/last/last_direct 六项回归均通过 | 没有改变评分的整秒时钟、`as_of`、档位、冻结常量或用户纠正。负 Unix 秒及任意超长小数的正确性仅有静态推断，未宣称分别运行验证 |
| 删除时间字段 / 保留字段但填非法时间 | 缺字段的一条被跳过，剩两条形成关系；非法值整体拒绝 rebuild，关系和 inference 均未写入 | 不补零/当前时间。识别到 graph 标签但结构缺损应跳过还是拒绝仍缺产品决定，本批只记录现状，不将测试升为新规范 |
| 默认上限的反证位置与遗忘 | 默认 8 条上限下，从 9 条选择前 8 条；末位反证因 SelectionLimit 排除，移到首位即入选。遗忘已选 A1 后旧 context 拒绝，新选择为其余 8 条，反证补入 | 这是候选顺序/容量效应，不能宣称系统识别了语义冲突。旧 CK 已销毁、旧正文不可读；新引用的 blob ID、title/summary 与实际解封值一致 |

本次对后续研究的直接建议：同时检查候选纳入/排除与最终输出；反证缺席可能发生在模型调用之前。对损坏来源的跳过/拒绝语义应在扩展输入格式前明确，而不是把静默减少的观测当作完整数据。现有正式导入先校验规范化的路径继续保留。

### 有界运行成本

最终修复候选上重跑一次小规模预热和每档三次新临时库观测，六个正式样本全部保留。环境：Windows 10.0.26200 X64，AMD Ryzen 7 H 260，16 个逻辑处理器、15.3 GiB 内存；Rust/Cargo 1.83.0，既有 root target，test profile 为 optimized、DEBUG=0、INCREMENTAL=0、JOBS=4，复用第二批 OpenSSL 静态库。未控制系统缓存或后台负载，不把前后两轮测量差值归因于代码。

下表单位为 **ms，最小 / 中位 / 最大**。导入固定 10 个 peer；活动事件每隔一个脚本小时一次，因此 100/1000 条分别形成 100/1000 个完整桶。两条路径使用独立库，不能将不同阶段直接相加解释为用户端到端耗时。

| 操作范围 | 100 条（3 次） | 1000 条（3 次） |
| --- | --- | --- |
| 导入预览 | 5.29 / 5.35 / 8.02 | 16.41 / 17.90 / 19.28 |
| Session 导入提交，含 rebuild | 37.01 / 39.80 / 45.62 | 302.46 / 344.53 / 350.32 |
| people 查询及证据视图 | 1.81 / 1.97 / 3.20 | 24.22 / 27.25 / 32.31 |
| Collector 脚本切换、poll 与逐事件写入（前 N−1 条） | 219.92 / 221.43 / 258.72 | 2233.03 / 2308.34 / 2368.61 |
| finish 写入最后一条 | 1.77 / 2.24 / 2.96 | 1.52 / 1.55 / 2.31 |
| 完整正向研究查询 | 4.24 / 4.67 / 7.56 | 41.43 / 42.47 / 47.21 |
| 来源复验与事务，空发布闭包 | 3.12 / 4.85 / 6.11 | 37.98 / 42.51 / 49.30 |
| 遗忘本次 Collector 共用的 CK | 8.25 / 10.66 / 12.51 | 15.84 / 17.47 / 18.69 |
| 遗忘后旧候选拒绝 | 3.06 / 5.47 / 8.83 | 38.71 / 39.71 / 40.53 |
| 遗忘后重新查询 | 3.42 / 4.81 / 4.95 | 34.92 / 42.79 / 47.13 |

每个样本另开一个空载 fake runner Session，观察至少一次采样后测量撤权调用到返回（含审计/状态处理）。六次为 3.49–11.73 ms，均未形成已完成事件；撤权后继续切换五次 fake 来源，实际 store 事件 ID 集合、采样数及状态计数不再增加。这与上述 100/1000 条负载没有关联，不能将其叫作规模压力下的停止时间。

遗忘会销毁该 collector 所有事件共用 CK：全部 100/1000 个源变为 unavailable、输出为零、旧发布闭包调用次数为零；不是随机遗忘单条的测量。查询未截断到旧预览的 50 行。空发布闭包未编码或写 ZIP；未测文件保存耗时、10k/100k 容量、冷缓存、p95、长期增长、UI 延迟或真实前台活动。这些样本不足以宣布存在/不存在架构瓶颈，未据此增加缓存或重构事务。

### 验证与记录

| 实际命令 | 结果与范围 |
| --- | --- |
| `cargo test -p soul-algo-tie -p soul-graph --locked --offline` | 退出 0；195 个普通测试及 1 个 doc-test 通过，包含新增 graph 6 项；源码未改冻结 scorer |
| `cargo test -p soul-graph --test research_time_boundaries --locked --offline` | 加强第二 peer 锚点后再次 6 项通过，退出 0；不重复计入新增数量 |
| `cargo test -p soulcore --test research_selection_boundaries --test session_memory_context --test session_screens --test session_collect --test session_research --locked --offline` | 退出 0；2 + 4 + 10 + 9 + 1 = 26 项通过；采集仅 fake source |
| `cargo test -p soulcore --test research_path_probe --locked --offline -- --ignored --exact bounded_research_paths --nocapture` | 最终候选退出 0；显式运行 1 项，输出 6 个样本；默认仍 ignored，不是 Q2 |
| `cargo clippy -p soul-algo-tie -p soul-graph -p soulcore --all-targets --locked --offline -- -D warnings` | 退出 0，没有关闭 lint |
| `cargo fmt --all -- --check`；`git diff --check`；新增未跟踪文件空白检查 | 退出 0；仅检查，未整仓改写格式 |

去重后的本批运行范围为 222 个测试（含显式 ignored 诊断）和 1 个 doc-test；新加 8 个普通测试和 1 个显式诊断。最初 offset RED、扩展小数 RED、导入对照 GREEN、修复后完整回归、锚点加强后的定向回归及最终测量分别保留，未累计重复通过。

仓库外日志目录：`C:/Users/86080/AppData/Local/Temp/soul-experiment-20261021-c8902b177ab24d148f91fcf1ee1ef65b/`。主要文件为 `environment.log`、`time-initial.log`、`time-expanded-red.log`、`selection-control.log`、`graph-final.log`、`time-final.log`、`core-regression-final.log`、`path-probe-final.log`、其 `.summary.json` 和 `clippy-final.log`；初测 `path-probe.log` 保留但不作为最终表。操作执行器另记录格式/空白检查退出码，不伪造日志中没有的标记。一次只读追踪误用了不存在的 `session_collect.rs` 路径，命令退出 1，随后定位到实际 `session.rs`，没有据失败查询得出实现结论。

本批未读取真实数据、开启真实采集、联系参与者、外部模型调用、安装或提交；未重跑 desktop/React（未改其接口与实现）、完整根 workspace、G-L/G-W/G-M。第二批桌面与前端证据保留其原时点，不能迁移为本次完整平台验收。源码未提交，未标 GATED/INTEGRATED；Goal 1/U7/正式 Goal 2 状态不变。

## 第四批：缺失与非法时间处理（2026-10-21）

用户明确要求继续实验版计划，先明确规则，再实施、测试和独立评审，仅用合成材料。起点仍为 `6a56d85 + dirty`，保留上批全部修改。第三批关于结构缺损“未决”的文字为当时观察；本批由 [D108](../DECISIONS.md) 明确处理选择，不改变冻结算法或增加验收门禁。

### 处理规则与实施范围

| 使用场景 | 处理规则 | 验证断言 |
| --- | --- | --- |
| 明确声明的 `graph_interaction` | required 结构缺损和时间缺失/null/错误类型/不可解析值一律拒绝；合法 offset/小数继续接受。不跳过坏条目，不填零或墙钟 | 同一 evidence 混合好坏条目也整体拒绝；已有关系和 inference 完整内容不变；无 owner 也校验明确声明的来源 |
| 非 graph source_ref | 继续忽略；分类只识别 Message evidence 中的明确 graph tag，不把坏条目误判成非 graph | 非 graph 引用可与合法 graph 引用共存；分类为 graph 不代表验证成功 |
| 图谱摘要、洞察与出处消费者 | 使用严格解析结果；错误向上传播或整个候选不可用，不能截成部分材料后继续成功 | 旧图引用受损来源后，消费路径拒绝；错误不含坏时间或正文 |
| 正式导入 | v1 继续按 schema 校验并转 UTC 整秒；Telegram 的 date 为必需合法民事时间字符串，实际时刻只由 date_unixtime 决定 | 非字符串 date 不能借合法 Unix 值漏过；不强行比较无时区 civil 时间与 UTC 一致；导入失败无部分持久化 |
| 研究留存 | 保持标准 UTC 格式和既有排除类别 | 合法 offset 仍属于不满足本次留存格式的排除，不能因图谱支持 offset 而扩大留存 |

源证据保持原样，拒绝不自动修复或删除数据。图谱在全部来源解析/适配完成后才写派生结果；Session 正式导入沿用现有事务回滚本次消息、密封内容、证据与图谱变更。这里不声称泛型 graph store 对所有非时间写失败都自动回滚。错误沿用仅携 evidence ID 的 GraphError 或导入字段/位置与静态原因，不转发携原始值的 adapter/serde 错误。

“结构可读”仅指 required 字段存在，类型、枚举和 UUID 可解码，时间另按既有 adapter 验证；不承诺透明字符串包装中的所有值都已做通用语义校验。出处/洞察原有数量、用途、主体、来源及 conversation_ref 等更严格检查保留。摘要必须检查当前候选依赖的全部相关来源，不能因前一条已匹配最近联系而短路跳过后面的坏引用。新持久导入在事务外记录失败状态不属于内容部分提交。

两个实现面分开认领：graph 的严格解析 API、所有消费者和相关测试由一名实现者负责；Telegram parser 与其测试由另一名负责（含该测试对新 graph API 的调用适配）。协调者负责 D108/计划/状态、唯一构建通道和集成；独立评审者核对规则、候选 diff 及实际证据。先写可在旧 API 下编译的行为 RED，再修改生产逻辑；不以 API 编译错误代替行为复现。

### 第四批实现与实际证据

已实现的生产差异：

- [interaction.rs](../../crates/soul-graph/src/interaction.rs) 返回 `GraphResult<Vec<InteractionRef>>`，显式标签反序列化和现有时间验证的任一失败均返回仅含 evidence UUID 的错误；[build.rs](../../crates/soul-graph/src/build.rs) 传播拒绝，无 owner 分支也检查声明的 graph 来源。
- [analysis.rs](../../crates/soul-draft/src/analysis.rs) 验证全部 resolved 证据，再使用匹配的最近联系来源，不再因找到匹配而短路；坏引用使整份摘要拒绝。[insights](../../crates/soulcore/src/commands/insights.rs) / [sources](../../crates/soulcore/src/commands/sources.rs) 显式传播错误或将整个候选标为不可用，保留原有数量、用途、主体、来源和预算检查。
- [Telegram parser](../../crates/soul-import/src/telegram.rs) 拒绝非字符串 date，Unix 时间权威、范围与原转换不变。已迁移所有直接测试调用，正常 fixture 显式 expect，未把 Result 本身误作可迭代的观测集合。

规则审查发现的摘要短路反例已补实际 RED：即使有效的最近联系在前，后续坏证据仍会被旧实现漏过。最终实现无条件遍历解析，回归分别验证坏引用在有效匹配之前和之后。审查还要求限定“结构可读”的含义，已按上文明确，不增加通用摘要字符串验证。Telegram 和主模块的实际差异均完成独立静态评审，没有剩余具体发现；运行证据收尾另记于[第四批评审](../reviews/2026-10-21-time-integrity.md)。

| 实际反例与合成验证 | 结果 |
| --- | --- |
| graph 旧实现 | 新增两项行为 RED：坏时间仍成功构建；损坏引用的 tag 分类错误。旧六项仍通过，命令退出 101 |
| 正式 Session 旧导入入口 | 缺时间来源下仍返回成功收据，实际 RED 退出 101；修复后四种损坏输入均拒绝，完整事件/联系人/证据/关系/推断快照及 blob/CK 数量不变，审计数量不增 |
| 当前持久导入入口 | 实际经过 prepare → begin/execute/complete；已有坏时间使本次内容事务回滚，域数据、blob/CK 数量和审计链快照不变，操作另存为 Rejected、无 receipt |
| 摘要 | 原先有效最近联系使坏引用漏检，实际 RED 退出 101；修复后两种证据顺序均拒绝，沿用静态、无正文错误 |
| Telegram | 旧实现接受 date=null，实际 1 fail / 1 pass，退出 101；修复后缺失、null、数字、布尔、数组、对象及非法字符串均拒绝，合法 civil 对照仍以 Unix 值为准 |
| graph 最终组合 | 8 项通过，包含坏 required 字段矩阵、好坏 sibling 整体拒绝、旧关系/inference 完整值不变、无 owner、未知标签控制及上批 offset/小数/跨日回归 |
| 留存边界 | 原有 7 项 integration 通过；canonical UTC、范围半开及 offset 排除语义保持，未因 graph 放行合法 offset 而扩大留存 |

本批新增 7 项普通测试，并将上批一项缺失时间现状刻画改为 D108 的一致拒绝断言；旧 RED 及第三批历史观察仍保留，不把它们改写成原先就符合新规则。新测试入口为 [research_time_refusal.rs](../../crates/soulcore/tests/research_time_refusal.rs)，其余添加于既有 graph/summary/Telegram 测试文件。密封内容的回滚通过真实临时 SQLCipher 库中的 blob/CK 数量及域数据快照核验，没有新增只为测试服务的生产接口。

环境与日志：Windows `x86_64-pc-windows-msvc`，Rust/Cargo 1.83.0，使用既有根/桌面 target。子进程设置 JOBS=4、DEV/TEST DEBUG=0、INCREMENTAL=0，复用第三批的既有 OpenSSL 静态安装（NO_VENDOR=1、STATIC=1）；未安装依赖或更改全局配置。所有输入均为合成 fixtures/临时测试库，网络仅为既有受控 loopback mock。

原始命令、实际退出码及 RED/GREEN 日志保留于 `C:/Users/86080/AppData/Local/Temp/soul-time-policy-c85d902b5d594767a26bdf2600213893/`：`graph-red.log`、`session-red.log`、`summary-red.log`、`telegram-red.log`、`crates-green.log`、`core-green.log`、`retention-green.log`、`desktop-green.log`、`clippy-final.log`。首次 Telegram RED 的旧 expect_err 会将合成 fixture 的 StagedImport 写入 panic 输出；现已改为静态 case 编号，未把该输出误称产品错误回显或真实数据。

### 第四批最终验证范围

以下命令由协调者串行运行于上述 dirty 候选；独立评审核对证据，不重复启动构建。

| 实际命令 | 结果与范围 |
| --- | --- |
| `cargo test -p soul-algo-tie -p soul-graph -p soul-import -p soul-draft --locked --offline` | 退出 0；364 项普通测试及 1 项 doc-test 通过，包含 graph 时间组合 8 项、people_summary 17 项和 Telegram 17 项 |
| `cargo test -p soulcore --test research_time_refusal --test research_selection_boundaries --test insight_commands --test session_insight_context --test session_evidence_sources --test session_import --test session_import_work --test import_and_graph_commands --locked --offline` | 退出 0；69 项通过，包含旧导入和持久导入各一项时间拒绝/回滚测试 |
| `cargo test -p soul-store --test research_retention --locked --offline` | 退出 0；7 项通过，留存格式与排除边界保持 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again` | 退出 0；130 项通过，2 项调用真实采集源的 IPC 用例明确 filtered；包括真实 command surface/handler 回归，不是未过滤的全量桌面验收 |
| `cargo clippy -p soul-algo-tie -p soul-graph -p soul-import -p soul-draft -p soulcore --all-targets --locked --offline -- -D warnings` | 退出 0；没有关闭 lint |
| `cargo fmt --all -- --check`；`cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check`；`git diff --check` | 均退出 0；仅检查，未整仓重写格式；格式/空白检查的实际退出码由执行器记录 |

去重后的本批最终执行范围为 **570 项普通测试和 1 项 doc-test**；不累计 RED 重试、局部重复运行或第三批历史数量。定向确认 `t4d_adapt.rs`、冻结算法 crate 与算法权威无 diff；未改 schema、依赖、锁文件、IPC 定义或前端实现。

本批未读取真实数据、启用真实采集、调用外部模型、联系参与者、安装应用或提交代码。没有运行完整根 workspace、前端测试、完整 G-L/G-W/G-M、安装卸载或新的性能测量；旧证据保留原版本，不迁移为本次全量验收。源码仍 dirty，未标 GATED/INTEGRATED，Goal 1/U7/正式 Goal 2 状态不变。

## 第五批：增长、ZIP 与停止压力（2026-10-21）

本批由用户明确要求继续实施、实测、必要修复、独立评审和记录；仅使用合成数据。起点仍为 `codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`，既有工作全部保留。复用原实验计划与测试框架，不新增门禁、性能阈值、依赖或生产测量接口。

### 实施与测量方法

两个实现者各认领独立测试文件：`soulcore/tests/research_growth_probe.rs`（必要 helper 放同名子目录）负责增长和真实文件保存；`soulcore/tests/research_stop_pressure.rs` 负责停止压力。不得互写、修改共享锁协议或默认扩大生产实现。协调者串行运行所有编译/实验，独立评审者检查实际 diff、断言、采样分母与原始结果；若复现具体功能问题，先保留失败证据再安排最小修复。

1. **同库累积增长。** 以累计 100 → 1000 → 10000 条为检查点，每阶段仅新增差量；每种场景独立三次临时库重复，同一次重复中不重置库。导入与前台活动分库，避免混合增长来源。导入时间生成正确处理秒/分钟/日期进位，来源 ID 全局递增；每阶段核对累计事件、证据、联系人、关系计数。活动用显式 fake source 或合成密封事件，分别测密集桶与一事件一小时的稀疏桶。时间范围覆盖全体生成数据，不沿用旧 probe 的固定两个月范围。记录新增成本、完整查询、重开成本和主 DB/WAL/SHM 大小；同时记录打开与全部连接关闭后的文件大小，不把主 DB 独自当作全部占用。
2. **ZIP 保存成本。** 通过真实 Session `prepare → begin → execute → complete`，每次新临时目的地；分别测各阶段和整体时长、实际 ZIP 字节数、桶数与事件数。prepare 包含查询及编码预检，execute 包含再次编码、锁等待、事务复验和真实 write/sync/publish，不能将它们标成纯编码或纯磁盘耗时。断言预览后文件不存在，成功后 receipt 与实际文件一致。释放 Session/所有旧 store 句柄后运行独立 Python 读取器，核对行数与计数，随后重开原临时库验证内容保持、采集仍关闭。读取器启动成本单列，不混入保存时间。用有界稀疏扩展观察既有 1 MiB 拒绝边界，拒绝时不产生目标文件、不截断；不提高任何上限来使样本通过。
3. **压力下停止。** 在含合成历史的临时库上启动真实 runner + fake source；先证明采样和完成事件确已发生。并发线程执行真实研究查询及合成密封写入，记录共享锁的获取/持有区间和请求/返回时间。只将与撤权请求真实重叠的操作计入竞争样本；必要握手用于确定持锁和请求次序，不在锁内 sleep 制造成本。停止返回后继续有界负载及 fake 切换，核对采样数及 runner 专属事件集合不再增长；背景 writer 的事件独立识别。记录重复样本、吞吐和锁等待，区分撤权生效、停止返回、最后事件。既有 STOP_BUDGET 按原含义对照，不新增速度门槛，不把请求等待自动解释为撤权失效。受控阻塞若另测，必须与自然负载分开，不代替真实 ZIP 保存。

公开研究查询会扫描全库 collector 候选，100000 条/16 MiB 元数据上限先于日期排除；缩小范围不保证降低候选扫描成本。完整 ZIP 上限仍为 1 MiB。上述是待实测的成本和边界，不据静态推断宣称瓶颈或通过。长期墙钟运行、冷缓存、系统后台负载控制、任意规模外推、真实数据和真实前台采集不在本批范围。

### 执行安排与验收断言

- 新测量入口默认 ignored，显式运行；先运行一轮检查设计和实际成本，再完成三次独立重复。fixture 构造、断言、日志输出和清理不计入被测操作；报告具体阶段而非总测试进程时间。
- 先验证候选/纳入/桶数/ZIP 字节及独立读取一致，再讨论耗时；没有有效输出的样本不记作成功保存速度。保存拒绝、未重叠或未完成的样本分别记录。
- 稀疏上限对照固定为一次从 10000 增至 12000 条，不无限扩展；先证明 store 查询仍成功，再核对 Session 拒绝来自完整 ZIP 上限，其他原因不得冒称 ZIP 边界。停止实验明确总尝试/重叠/未重叠/完成数，以及查询与 writer 操作身份和计数区间；公开 API 无法直接观测撤权内部获锁时刻时，只报告请求至竞争锁释放、释放至返回的可观测区间，不将它们称为纯锁等待或内部阶段。
- 原始测量仅输出计数、字节、耗时和状态。日志及开工快照置于仓库外 `C:/Users/86080/AppData/Local/Temp/soul-architecture-a9dadd6a760a477cb27e4924cdf07342/`；不读取真实库、密钥、导出或终端历史。
- 主线程运行新入口与必要相邻回归、严格 Clippy/格式检查。仅合成采集继续过滤 `windows::runs_on_windows`；如果触及桌面，排除 AGENTS 指定的两项真实采集 IPC 用例。不把局部运行称为 G-L/G-W/G-M 或正式 Q2。
- 结束后在本节记实际命令/退出码、所有有效样本及未覆盖项，更新 STATUS 并记录一次独立只读评审。不自动提交、合并、安装或启动真实观察。

### 第五批实际结果

新增 [research_growth_probe.rs](../../crates/soulcore/tests/research_growth_probe.rs) 与 [research_stop_pressure.rs](../../crates/soulcore/tests/research_stop_pressure.rs)，两个较重实验默认 ignored，另外有一项时间进位/差量 fixture 检查。本批没有复现需要修改生产逻辑的功能故障，生产实现、冻结算法、授权/事务协议和资源限制保持。必要修正集中于实验代码：移开停止请求后的额外快照扫描；依独立评审删除把请求标记误当撤权生效时刻的过强断言；将过大的 JSON 日志宏拆分，解决首次 Cargo 编译退出 101。编译失败没有进入行为运行，不作为 RED。静态反例、修改及复核见[第五批独立评审](../reviews/2026-10-21-architecture-growth.md)。

最终增长样本为三个场景各 3 次独立重复、每次同库 3 个检查点，共 **27 条成功检查点记录，其中 18 条实际 ZIP 保存**；另有 1 条固定 12000 行拒绝记录。最终停止样本为两档历史各 3 次，**6 次尝试、6 次完成、6 次确有同步持锁重叠、0 次未重叠**。单轮试跑另存，不累计进这些数字，也不把测量记录数当测试用例数。

环境：Windows 11 10.0.26200 X64，AMD Ryzen 7 H 260 / 16 个逻辑处理器，16004104 KiB 可见内存；Rust/Cargo 1.83.0、Python 3.13.14。沿用既有 target 和 OpenSSL 静态安装，仅子进程设置 JOBS=4、DEV/TEST DEBUG=0、INCREMENTAL=0；没有改全局缓存、安装依赖或控制系统缓存/后台负载。没有以不同批次的时间差判断代码收益。

#### 同库增长和实际保存

时间单位 **ms，最小 / 中位 / 最大**，每格 n=3。导入查询是完整 people 查询，提交成本包含该阶段差量导入及全图重建；活动查询是完整研究聚合，保存成本是 prepare/begin/execute/complete 四段操作时间之和，不含用户等待、段间断言或独立读取器启动。

| 场景 | 累计事件 | 本次新增 | 完整查询 | 导入提交或四阶段保存操作之和 | 关闭后的库字节 |
| --- | ---: | ---: | --- | --- | ---: |
| 导入 | 100 | 100 | 1.84 / 2.57 / 2.74 | 45.86 / 49.92 / 54.18 | 724992 |
| 导入 | 1000 | 900 | 29.30 / 32.13 / 40.38 | 317.40 / 333.98 / 361.10 | 3305472 |
| 导入 | 10000 | 9000 | 692.94 / 849.65 / 1008.97 | 3815.34 / 4019.35 / 4031.67 | 29052928 |
| 稀疏，每小时一事件 | 100 | 100 | 3.79 / 4.34 / 6.88 | 14.08 / 14.90 / 23.71 | 540672 |
| 稀疏，每小时一事件 | 1000 | 900 | 41.06 / 41.81 / 48.59 | 93.23 / 100.46 / 110.36 | 1695744 |
| 稀疏，每小时一事件 | 10000 | 9000 | 498.06 / 522.15 / 552.52 | 980.70 / 1072.81 / 1189.63 | 13471744 |
| 密集，每分钟一事件 | 100 | 100 | 4.26 / 5.12 / 6.17 | 14.56 / 16.67 / 17.72 | 540672 |
| 密集，每分钟一事件 | 1000 | 900 | 44.94 / 46.69 / 48.07 | 85.54 / 87.60 / 105.04 | 1695744 |
| 密集，每分钟一事件 | 10000 | 9000 | 491.17 / 528.75 / 560.90 | 1002.18 / 1030.62 / 1043.62 | 13471744 |

一万活动事件时，两个场景的 prepare 分别为稀疏 **465.44 / 540.06 / 567.77 ms**、密集 **480.51 / 480.58 / 523.60 ms**；execute 分别为 **513.35 / 530.97 / 597.87 ms**、**506.12 / 517.74 / 540.40 ms**。两者都包含规定的组合工作，不能用阶段差值拆出纯编码、纯写盘或纯锁等待。该阶段新增 9000 条的真实 Collector poll/switch/write 加 finish 成本分别为 **20596.94 / 21640.40 / 25618.73 ms**、**21000.18 / 21699.37 / 23138.51 ms**；这是逐事件持久写入，不是事务批量写入基准。

| 场景 | 100 事件：桶 / ZIP 字节 | 1000 事件：桶 / ZIP 字节 | 10000 事件：桶 / ZIP 字节 |
| --- | ---: | ---: | ---: |
| 稀疏 | 100 / 10731 | 1000 / 94434 | 10000 / 931437 |
| 密集 | 2 / 1617 | 17 / 3030 | 167 / 17133 |

所有保存均无排除事件，预览后目标不存在，发布成功且事务正常结束。每次关闭原 Session 和全部 store 句柄后，独立 Python 进程均校验行数、候选、纳入及排除计数一致，再重开同库核对数据和采集关闭。一万事件时独立读取器的进程启动加读取成本为稀疏 **550.39 / 629.89 / 642.47 ms**、密集 **366.51 / 409.78 / 493.63 ms**；重开成本分别 **14.69 / 15.68 / 17.24 ms**、**12.83 / 16.82 / 30.83 ms**。它们没有计入保存操作之和。

一万导入事件检查点打开时 DB/WAL/SHM 长度分别为 **29052928 / 26644072 / 65536 字节**，合计 **55762536**；全部关闭后为 **29052928 / 0 / 0**。一万活动事件为 **13414400 / 4169472 / 32768**，合计 **17616640**；关闭后为 **13471744 / 0 / 0**。原始日志包含所有检查点。关闭引起的 WAL 整理会改变主文件长度，这些数字仅是检查点占用，不是峰值、写入量或可回收空间保证。

第三轮稀疏库一次扩至 12000 事件/桶：store 查询完整纳入 12000，耗时 **645.12 ms**；Session prepare 在 **738.18 ms** 后返回既有 1 MiB 拒绝说明，目标文件不存在，重开后事件仍为 12000。此结果只验证该输入超限而不截断，没有搜索精确最小拒绝行数，也没有测到 100000 候选或 16 MiB 元数据上限。

#### 查询和写入竞争中的停止

历史通过单独合成密封事务准备，不计入被测负载；runner 在压力前已有采样和完成事件。负载按循环执行真实密封写入及全候选研究查询，带 20 ms 锁外 fake 切换间隔。每轮第三次查询持锁后发出通知，停止线程记录请求，查询随后实际运行；这使样本有确定重叠，但不是随机到达的自然停止流量。额外验证扫描在通知前完成，不进入停止请求计时。

| 历史事件 | 请求至返回 ms | 同步查询实际工作 ms | 请求至竞争锁释放观察 ms | 竞争锁释放观察至返回 ms |
| ---: | --- | --- | --- | --- |
| 1000 | 36.46 / 39.88 / 67.27 | 33.41 / 36.12 / 63.53 | 33.44 / 36.15 / 64.37 | 2.91 / 3.02 / 3.73 |
| 10000 | 479.65 / 501.67 / 524.95 | 472.60 / 493.77 / 517.74 | 472.63 / 493.85 / 517.77 | 7.03 / 7.19 / 7.82 |

六次返回均小于既有 1 秒参考值，且返回后继续八次真实写入、八次查询及 fake 切换，**runner 专属事件与采样新增均为 0**；背景 writer 正常新增，历史事件不变，审计链通过。请求前快照变化只作为观察量，没有将请求标记等同内存撤权生效点；本批没有直接测得内部撤权时刻或最后一次写入时刻。

每个样本的负载观察区间内各完成 11 次写入和 11 次查询。1000 历史时观察窗口为 **694.54 / 756.51 / 835.57 ms**，每种操作的窗口平均频率为 **13.16 / 14.54 / 15.84 次/秒**；10000 历史为 **6187.86 / 6256.33 / 6523.28 ms**、**1.69 / 1.76 / 1.78 次/秒**。该窗口包括锁外间隔、握手和调度，并非连续饱和吞吐。原始操作时间线区分查询/写入及各自锁等待；表中两段观察间隔不冒称撤权内部的纯 mutex 等待或审计成本。

#### 本批能支持的判断

完整保存路径在一万候选下，即使输出只有 167 桶/17133 字节，也仍需约一秒的组合操作时间；同量候选的完整查询本身约半秒。结合既有 prepare/发布复验都查询来源的实现，**后续优化研究应先定位全候选扫描与复验持锁成本**，而不能仅根据 ZIP 文件大就先改压缩。同步竞争样本也实测停止返回会等待查询占用；当前六次未超一秒参考值且返回后无采集新增，不代表更大库或慢磁盘下有同样上界。

这些是有限数据增长的实测，不是长时间运行、内存峰值、p95、冷缓存、完整 ZIP 发布与停止同时发生或用户可见 UI 延迟的验收。没有据此提前重构缓存、事务或取消语义；只有新的具体反例才继续修复对应路径。

#### 第五批命令与证据范围

全部命令由协调者串行运行。两个最终 ignored 实验显式设置 `SOUL_RESEARCH_TRIALS=3`；单轮 pilot 的环境为 1，日志分开，不累计。下列最终命令均退出 0。

| 实际命令 | 结果及限制 |
| --- | --- |
| `cargo test -p soulcore --test research_growth_probe --locked --offline -- --ignored --exact cumulative_growth_and_zip_costs --nocapture` | 1 项显式诊断通过、1 项普通测试 filtered；27 个成功检查点和 1 个拒绝，18 次真实保存均独立读取 |
| `cargo test -p soulcore --test research_stop_pressure --locked --offline -- --ignored --exact bounded_stop_under_query_and_write_pressure --nocapture` | 1 项显式诊断通过；6 个 attempt 和对应 6 个有效 sample，不能算成 12 个样本 |
| `cargo test -p soulcore --test session_collect --test collect_probe --test session_research --test research_growth_probe --test research_stop_pressure --locked --offline` | 14 项普通测试通过，2 项新诊断默认 ignored |
| `cargo test -p soulcore --lib research --locked --offline` | 13 项通过，1 项既有跨语言专用检查 ignored，36 项无关用例 filtered；该 ignored 不冒称本轮执行，新增增长探针的 18 次独立读取另有证据 |
| `cargo test -p soul-collect --locked --offline -- --skip windows::runs_on_windows` | 31 项通过，3 项 Windows 源模块测试 filtered；未调用真实采集源 |
| `cargo test -p soul-store --test research_retention --locked --offline` | 7 项通过 |
| `cargo test -p soul-local-files --locked --offline` | 14 项通过，2 项默认 ignored；其中 synthetic child helper 由父测试实际启动，不另外累计为独立通过用例，reparse 特权用例未运行 |
| `cargo clippy -p soulcore --all-targets --locked --offline -- -D warnings` | 严格检查通过，没有关闭 lint |
| `cargo fmt --all -- --check`；`cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check` | 根与独立桌面格式检查通过，仅检查未整仓重写 |

最终去重为 **79 项普通测试 + 2 项显式诊断，共 81 项**；本批新增 1 项普通 fixture 检查和 2 项默认 ignored 实验。没有把相同测试的 pilot 或上批数量重复加入。生产相关路径与开工快照内容相同，旧 probe 保留原范围。

原始记录目录仍为 `C:/Users/86080/AppData/Local/Temp/soul-architecture-a9dadd6a760a477cb27e4924cdf07342/`。主要文件：`environment.log`、`growth-pilot.log`、`growth-final.log`、`stop-pilot.log`（首次编译失败）、`stop-pilot-retry.log`、`stop-final.log`、`core-regression.log`、`archive-regression.log`、`collect-regression.log`、`store-regression.log`、`files-regression.log`、`clippy-final.log`、`format-final.log`。仓库外 `summarize.ps1` 仅读取这些合成原始测量，重算样本完整性和最小/中位/最大至 `summary.json`；没有成为新门禁或第二份当前状态。

本批未读取真实数据、启用真实采集、外网调用、联系参与者、安装或提交。未运行完整根 workspace、桌面行为测试、前端、完整 G-L/G-W/G-M 或新的正式 Q2；格式检查不等于桌面测试，历史平台结果不迁移。源码仍 dirty，未标 GATED/INTEGRATED，正式 Goal 状态不变。

### 研究保存取消补充实测（2026-10-21）

用户在进度核对与三项实验计划后要求执行。本聊天开工发现“规划 Soul 实验版收尾”聊天已认领并执行上面的增长/ZIP/采集停止实验，因此保留其单写者与构建安排，读取并重算该批原始结果，不重复实现或运行同覆盖面。本聊天只补原计划中尚未覆盖的**研究保存取消响应**。在对方实测与构建完成后串行运行补充入口；待其结束并释放文档写入后，再追加本节和更新 STATUS。

新增 [research_save_cancel_probe.rs](../../crates/soulcore/tests/research_save_cancel_probe.rs)，Windows 限定、默认 ignored，不改生产逻辑、依赖、授权、取消协议或算法。100/1000/10000 条本人获准的合成密封事件，每档三次独立临时 SQLCipher 库，分别形成 2/17/167 个小时桶；fixture 在事务内生成，其成本不进入被测操作。

真实 `prepare_research_save → begin` 后，查询线程持有同一 store 锁，握手允许 worker 的 `execute` 与 Session 的 `cancel_research_save` 公告请求；收到两个公告后执行真实完整研究查询，锁内无 sleep。随后核对 `complete`、最终文件、事件计数及审计链。公告后线程仍可能被抢占，因而只记录**请求公告与持锁观察区间重叠**，不声称已测得方法内部进入、实际 mutex 等待或取消 flag 生效。取消返回包含可能的审计等待；execute 返回也可能包含来源复验、清理和失败审计，两个区间不是可相加的独立阶段。

最终 9 条 attempt 对应 9 条 sample，三档各三次；取消与 execute 公告均 9/9 重叠，**9/9 取消获胜、0 已发布、9/9 completion 拒绝且目标文件不存在**。事件总数保持，审计链断言通过。没有为了得到取消分支静默重跑或丢弃样本。

以下单位为 **ms，最小 / 中位 / 最大**，来自原始时间点重算：

| 合成历史事件数 | 真实查询操作 | 取消公告至返回 | execute 公告至返回 |
| ---: | ---: | ---: | ---: |
| 100 | 4.54 / 4.63 / 5.07 | 5.80 / 5.85 / 6.53 | 8.22 / 10.86 / 10.91 |
| 1000 | 38.53 / 52.61 / 58.73 | 42.32 / 55.52 / 67.80 | 57.78 / 71.40 / 103.39 |
| 10000 | 526.48 / 566.89 / 590.29 | 542.80 / 581.48 / 602.57 | 557.26 / 614.32 / 1091.87 |

10000 第三次的取消公告至返回为 581.4804 ms，execute 公告至返回为 1091.8695 ms，最终仍取消且无目标文件。该观察证明任务返回可能晚于取消返回；时间点不足以定位差值属于哪一内部阶段，也不适用采集 `STOP_BUDGET`。本次没有证明“点击取消后所有后台工作立即结束”，没有把较长返回认定为授权失效或未经测量就修改生产路径。

| 补充实际命令 | 结果 |
| --- | --- |
| `cargo test -p soulcore --test research_save_cancel_probe --locked --offline -- --ignored --exact bounded_save_cancel_under_query_pressure --nocapture` | 退出 0；1 项显式诊断通过，9 个完整样本，测试阶段 10.15 秒 |
| `cargo clippy -p soulcore --test research_save_cancel_probe --locked --offline -- -D warnings` | 退出 0，无 lint 降级 |
| `rustfmt --edition 2021 --check crates/soulcore/tests/research_save_cancel_probe.rs` | 最终退出 0；仅检查新增文件 |

环境：Windows、Rust/Cargo 1.83.0、沿用既有根 target、JOBS=4、DEV/TEST DEBUG=0、INCREMENTAL=0；复用上述 OpenSSL 静态安装，NO_VENDOR=1、STATIC=1，没有安装依赖或改变全局配置。原始 `cancel-final.log`、`clippy-final.log`、`format-final.log`、环境记录及实际执行脚本位于 `C:/Users/86080/AppData/Local/Temp/soul-save-cancel-67828c71dfb645a9844c633e8d95e18c/`。`summary.json` 只是原始样本的派生统计，不是新状态权威或门禁。

独立只读评审发现并修正两项测试问题：公告时间命名夸大内部观测，以及发布获胜分支的 Python 读取等待无界。随后定向静态复审和最终原始日志核对均完成，九组统计一致，没有剩余具体发现，见[补充评审](../reviews/2026-10-21-research-save-cancel.md)。新增读取器采用临时文件捕获、120 秒截止、超时终止后 5 秒有界回收和 64 KiB stdout 读取限制；本轮没有发布获胜样本，因此该读取器和超时清理分支仅静态检查，未运行。第五批 18 次成功保存独立读取仍是上方原回执，不移作这个新 helper 的覆盖。

本补充只增加 1 项显式诊断，与上方 79 项普通测试 + 2 项显式诊断分别保留执行来源，不将跨聊天结果称为一次完整门禁。未新增生产修复；未读取真实数据、开启真实采集、调用外部服务、提交或安装。未测 UI 延迟、内部取消生效时刻、自然竞争分布、已进入 ZIP 磁盘写入阶段的精确取消时间、发布获胜或失败线程的故障注入；正式 Goal/Q2 和平台验收状态保持。
