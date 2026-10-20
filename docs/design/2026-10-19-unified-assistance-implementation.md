# 统一辅助实施与验证

本轮按[原计划 U1–U7](2026-10-16-unified-assistance.md)重建。生产起点为 `600f309`，原计划恢复检查点为 `1433951`；此前被工作区维护清理的未同步实现与测试结论不能作为本轮证据。产品、算法和正式验收权威仍由 PLAN_INDEX 指向既有文档。

## U1/U2：统一事项与显式目标

编码前决定见 D97，方案及实际 dirty 源码由独立只读 reviewer 复核。当前实现：

- schema 4 只添加事项元数据；手工目标复用 Commitment Memory 及其内容密钥，创建和修改与事项更新同事务。观察项以真实 namespace/contact 稳定去重，固定 bundle 的替换由用户明确核对版本。
- 截止只接受明确带偏移的绝对 RFC3339 时间；不解析导入文本中的旧“明天”，不补午夜。完成只记自报，取消独立，旧版本动作不能重开终态。
- 可选来源关联不改变手工目标的独立遗忘单位。来源遗忘使其失效，不删除目标正文；目标自身遗忘照旧销毁 CK。遗忘确认比较完整 item/revision 集合，辅助字段损坏不能阻止密钥销毁。
- 事项列表只有元数据，明确详情才读取目标或可信统计。事项跨页交接只携带 ID/revision，core 创建实际绑定事项与来源的 held context，绑定延续到 pending approval、owned worker 和 completion。已发生 E1 请求先记真实审计，再拒收过期回复；实际 worker 结束前不释放槽。
- 前端增加事项与洞察独立入口。地址代次、payload 所属地址/来源、挂载和选择共同拒绝旧结果；未知写入先核对，不自动重放。普通起草及既有本次材料选择保留。

独立复审和实际测试促成的修正：读取/解密故障不能永久改成遗忘；带观察来源的目标也验证自身 CK；重复关联不能把返回旧固定 bundle 的已知结果误标 unknown；新地址 render 不能继续显示或操作旧 state。最后一项用 layout effect 在 DOM commit 后、passive effect 清理前检查，不只验证旧闭包。

### 实际验证（Linux，Rust 1.83.0 / pnpm 10.15.0）

| 检查 | 实际结果与范围 |
| --- | --- |
| `cargo test -p soul-store -p soul-store-api --locked` | 20 个测试输出套件，97 通过、0 失败；含两个无用例 doc-test 套件 |
| `cargo test -p soulcore --test session_assistance --test session_memory_context --test session_insight_context --test session_draft_work --test session_forget_scope --test session_atomic_edits --test session_workflow_continuity --test session_screens --test session_e1 --locked` | 9 套件 / 80 通过；新增 12 项真实 Session/SQLCipher 用例包含其中，不重复累计 |
| `pnpm --filter @soul/desktop test` | 47 文件 / 561 通过；最后仅调整 Assistance effect 的 state 更新位置后，定向 14 项再次通过 |
| `pnpm --filter @soul/desktop lint`、`build` | TypeScript、ESLint、明确 production 构建均退出 0 |
| 现有 `command_surface.rs` 的纯源码断言 | 6 项通过，核对实际 66 个 IPC 名称、注册及薄转发；独立 rustc 适配只替换 crate 常量导入和 include 路径，不执行 Tauri handler |
| 根/桌面格式、schema-freeze、E0、禁词 | 均退出 0；E0 扫描 16 个交付 crate / 289 个文件，禁词扫描 94 项 / 136 个文件 |
| 原生 desktop / IPC / Windows | 未运行；新增真实 handler 场景目前只有源码复核，不能用静态断言代替 |

首次 Rust 编译出现零长度构建缓存对象；限定清理对应 package 缓存后恢复。首次三包全量测试又遇既有测试可执行文件缺执行位，随后重编译出现两个空 ELF 产物；未把这些中断计通过，仅清理已确认的空缓存文件。上表记录后来完整退出 0 的存储检查与受影响核心定向检查，不声明 `soulcore` 全包已通过。前端首轮 9 项失败已逐项修正；没有放宽安全断言或关闭 lint 规则。

## U3：持久触发、恢复与采集导入交接

编码前决定见 D98。领域修改和合并待办同事务；每事项分别保存 pending revision、实际 claim UUID 和精确完成回执。已开始执行但提交未知时只协调原 claim，不重新计算；执行前直接丢弃的 owned work 必须等物理 lease 结束才能重排。失败保存固定代码，只有明确重试才能再次执行。同一新持有期的恢复复用原规范路径 OS lease，不恢复批准或采集同意。

新导入在原事务内创建缺失的稳定观察事项，已有事项保留原固定 bundle。Graph 领域事务尾复验实际引用的完整来源；原 W5 对证据、事件、密钥和导入账本的操作错误现严格传播，数据库故障不再被当成来源消失而永久失效。schema 4→5 只在原迁移事务里初始化一次待办，正常重启不会把已处理结果重新排入。

导入前的 collector 交接使用可重建 source factory 和授权代次：owned worker 在 Session/store 长锁外真实 stop、flush、join；完成时只有同一代次仍获同意且审计已协调才重建 source。撤回立即改变共享同意并可延后记审计，旧代次不能恢复。直接丢弃未执行导入也先完成 collector join，再释放物理 lease。

桌面生产线程使用同一个 Session，周期只调度持久待办。新认领前再次检查 stop；执行在 Session 锁外，已开始的工作在退出时仍完成并真实 join。前台 held/pending 准备优先，后台不丢弃用户材料。所有长存储工作使普通存储入口快速 busy；状态查询缺计数时返回 null，不能假称为零。三个新增 IPC 仅提供元数据查询和显式失败重试，实际桌面命令数为 69。

独立复审闭合了三项并发问题：读状态间接持 Session 等后台 store；未知发布被错误重排而每秒重算；未执行 Import Work 的字段析构先释放 lease、再 join collector。对应真实 Session/SQLCipher 和阻塞 source 反例已通过。

### 实际验证（Linux，同一工具链与锁文件）

| 检查 | 实际结果与范围 |
| --- | --- |
| `cargo test -p soulcore --test session_assistance_work --test assistance_runtime_driver --test session_assistance --locked` | 3 套件 / 28 通过：11 项新工作/交接、5 项实际生产 driver/thread、12 项既有 U1/U2 |
| `cargo test -p soul-store -p soul-store-api --locked` | 20 个输出套件 / 101 通过；新增 4 项真实存储协议与迁移用例包含其中 |
| 核心受影响回归 `session_collect`、`session_import`、`session_import_work`、`session_insight_context`、`session_draft_work`、`session_forget_scope`、`session_atomic_edits`、`session_workflow_continuity`、`session_e1` | 9 套件 / 94 通过；没有启动完整 G-L |
| `pnpm --filter @soul/desktop lint`、`test`、production `build` | 均退出 0；完整 49 文件 / 585 测试通过 |
| 根/桌面格式、schema-freeze、E0、禁词；命令静态断言 | 均退出 0；现有 6 项静态断言核对实际 69 个 IPC 的两端名称、注册及薄转发，不代表原生 handler 执行 |
| 原生 desktop / IPC / Windows | 未运行；新增的 IPC roundtrip 场景只有源码复核，根 workspace 实际生产线程测试不代表 Tauri handler 或 Windows 通过 |

首次 Store 回归的既有 CREATE-only DDL 断言失败；已把待办初始化移入一次版本迁移，保留原安全断言。前端首次 lint 发现 hook 返回对象中的 ref 赋值；改成 hook 内封装同步 begin/owns/finish，保留旧地址与旧挂载拒收条件后完整复验通过。

最后仅把迁移用例强化为真实 schema 4 缺工作表后，定向用例本身通过；首条过滤命令随后尝试运行无匹配用例的既有 `forget` 二进制时遭缓存执行位错误，因此整条命令不计成功。改用精确 `--lib` 范围复验，不以该环境问题放宽任何断言。

## U4/U6：提醒、通知与明确反馈

编码前决定见 D99，独立复审覆盖实际存储、Session、Windows 绑定、生产调度与界面 diff。schema 6 增加提醒、attempt、撤回代次、类别偏好与反馈元数据，原有事项/待办不自动开启提醒。每项提醒独立 CAS，用户必须给出明确的未来绝对时间；高于毫秒精度的时间向上取整，不提前触发。截止时间不启用提醒，改期、完成、取消、来源失效和遗忘与失效/撤回意图同事务。

严格 W5 检查在既有物理槽的 owned work 中执行；同一事务在检查完成后再次核对时钟、事项/来源/类别/schedule 版本，才能创建一次 dispatch attempt。系统恢复、调度间断、过期、频率限制或新持有期恢复都不会补投旧提醒。每分钟最多一个新 attempt，超额仅留收件箱。操作读取错误记录固定 check_failed，不冒作来源消失或后台热重算。

通知传输仅收到 attempt UUID 和固定通用文字；Windows 自有 HWND/线程的资源容量、队列与回执分别管理。提交、展示、打开、隐藏、超时、资源跟踪到期及撤回分开记录，系统提交成功不代表展示或已读。等待 5 秒后结果仍未知时保留实际 owner 和精确回执跟踪；迟到的真实结果能修正原 attempt/WID，不重发，也不覆盖新撤回代次。图标 ID 到进程退出前不复用；每种系统恢复控制只保留最新代次，业务回执写入失败不会遮挡安全时钟复位。

撤回认领先在 Session 留存精确 WID/attempt/claim token，再提交 SQL CAS。提交结果不确定时，只把这一代认领协调为 unknown；不会重新执行撤回或误改其他正在执行的认领。退出先给两个调度线程和原生 owner 发停止信号，再逐一真实 join；退出回收仍记录已发生事件，但不再打开窗口。非 Windows 如实报告 unsupported，到期提醒仅留收件箱。

类别静默/恢复仅作用于明确目标或本地观察两种范围，恢复不补积压。helpful/not_relevant/clear 有独立 revision，不改变事实、语气、人格、目标 revision、批准或完成。界面显式读取后按精确版本操作；未知写入只能由相应记录在写入实际结束后发起的新读取解除，其他面板或较早的迟到查询不能清除。通知打开只导航到当前事项，仍需用户明确读取和操作。

### 实际验证（Linux，同一工具链与依赖锁）

| 检查 | 实际结果与范围 |
| --- | --- |
| `cargo test -p soulcore --test session_reminders --test notification_runtime_driver --test session_assistance_work --test assistance_runtime_driver --test session_assistance --locked` | 5 套件 / 68 通过，含 14 项新提醒业务、实际生产 bridge/thread 的 7 项、原生传输状态机的 19 项；后两组使用合成 OS，不计 Windows 通过 |
| `cargo test -p soul-store -p soul-store-api --locked` | 20 个输出套件 / 101 通过，含两个无用例 doc-test 套件 |
| 与 U3 相同的 9 个受影响核心回归目标 | 94 通过，覆盖采集、导入、owned work、上下文、遗忘、原子编辑、贯通流程及 E1；与前两行共 263 项，不重复累加诊断首轮 |
| `pnpm --filter @soul/desktop lint`、`test`、production `build` | 均退出 0；完整 51 文件 / 630 项通过 |
| 根/桌面格式、schema-freeze、E0、禁词；既有命令静态断言 | 均退出 0；E0 扫描 16 个交付 crate / 311 源码文件，禁词扫描 94 项 / 142 文件；6 项静态断言核对 77 个 IPC 的两端名称、注册及薄转发 |
| 原生 desktop / IPC / Windows | 未运行；新增真实 handler 的 8 命令场景只有源码复核，实际命令数 77；Windows Shell_NotifyIconW 只完成源码/ABI 复核 |

首次核心调用被自动审批因 PowerShell 启动遥测外联拦截，没有计为通过。改为在启动进程前设置官方遥测退出环境变量、关闭更新检查后，定向及完整受影响回归均实际退出 0；没有改用户全局设置、放宽测试或给遥测新增授权。此前诊断首轮 38 项已包含在上表最终结果内。

独立复审发现并修正的关键反例包括：未知撤回 claim 永久卡住；旧查询错误解除新未知写入；业务回执前缀阻塞系统恢复，以及同一种恢复事件尚未 ack 时第二次真实恢复丢失；退出等待第一个线程期间另一个仍可提交；最后 flush 在退出时重新打开窗口。真实重启恢复和最终时钟过窗分别独立测试，不用同一测试中的其他拒绝条件代替。

## U5：默认关闭的本地准备

编码前决定见 D100。仅明确手写目标能按事项 revision 与独立偏好 revision 启用，默认关闭。后台复用 U3 持久工作与实际物理槽；pref revision 只限制模板发布，不把基础核对误标失败。关闭只撤销可见模板并更新偏好，保留共用 pending/claim；运行中的旧 R1 遇到新业务 R2 和关闭操作时，R2 基础处理不会丢失。重新启用或明确重新准备产生新偏好 revision，拒绝旧结果。

准备严格读取真实 OWNER_PROFILE_ID 和四项语气枚举，调用既有本地开场模板；正文占位保留，不拿目标 summary 假作收到的消息。只保存模板 UUID、事项/偏好版本、完整固定来源、owner、四项语气、时间和 opening_v1 类型，没有持久模板正文字段。缺失、损坏或暂时不可读的档案明确失败，不静默换成 neutral；模板分支失败不破坏已经完成的基础 U3 核对。

真实 profile 保存的四项语气改变，与撤销相关模板、合并已启用目标待办在原 savepoint 内完成；人格轴、锁定标记或另一个 owner 的变化不误触发。source hook 在原写事务内保守撤销依赖模板，失败整体回滚；模板撤销本身不把资料标成永久失效，也不改变提醒/事项 revision，真实领域失效仍沿既有事务链传播。schema 6→7 添加两张空表，不回填启用、不重排旧工作。模板和偏好纳入精确遗忘范围；损坏辅助 UUID 不得阻止密钥销毁，遗忘固定来源仍保留独立目标的 Memory 和偏好。

显式预览传 item/revision/pref revision/template UUID，使用新的 LocalPreview owned 工作占既有槽。begin/complete 只短持 Session，execute 在锁外完成全 W5、真实 owner 与语气核验后临时渲染；complete 再核精确元数据，来源写事务撤模板使中间变化拒收。失配先同事务排入刷新，提交后在事务外返回拒绝；操作读取错误回滚，不永久标 stale。预览不清 held/pending 草稿上下文，不消费 E1 许可，物理工作实际结束后才释放容量。

界面只在明确读取后操作，正文绑定当前地址、来源、挂载、事项、偏好及模板身份。重新核对或启停起跑就隐藏旧正文；迟到回包不能恢复它。失败可明确按新的偏好 CAS 重新准备，未点击不重试。metadata、准备和预览都不把文字自动送入 Draft，不自动采用、反馈、完成或批准模型调用。三个新增 IPC 后总计 80 个；原生 handler 场景仍只列为测试源码。

## U7：集成一致性与尚未执行的验收

两条新合成轨迹使用真实 Session/SQLCipher、实际 U3 production driver 和本地 MockHTTP，编码前方案和实际测试源码均经过独立复审：

| 轨迹 | 实际断言 | 不能由此推出 |
| --- | --- | --- |
| 导入到目标全流程，1 条合成轨迹 | 确认导入、固定来源目标、逐项启用、精确本地预览；此前零 E1。合成系统回执只定位同一事项；明确批准后 MockHTTP 恰一次，反馈/返回草稿仍未完成，自报完成才撤回提醒。先忘来源仍能读取目标 Memory、保留自报完成状态；再忘目标销毁自身内容。独立未变化目标上另一个从未执行的批准与上下文在重启后拒绝 | 真实 Windows 投递、剪贴板或外部发送成功、草稿内容质量、使用者满意度 |
| 同时积压 100 项，首项连续编辑 20 次 | 合并后仍 100 项待办；held 前台材料保持可用并阻止后台认领。实际 local draft 消费材料后，每项按最新 revision Ready，最终无 pending/claim。同一到期窗口仅一个 attempt，其余只留收件箱，零 E1 | 吞吐、耗时上界、海量资料性能；一分钟频率边界由 U4 独立用例验证 |

U7 中可在当前环境执行的合成故障、恢复、规模样本和跨层一致性检查与平台/效用验收分开记录：

| 验收面 | 当前范围 |
| --- | --- |
| 工程一致性 | 按本轮实际 Rust/前端检查记录，不沿用丢失工作区或旧提交的绿色结果 |
| Windows 原生通知与桌面 IPC | NOT RUN；当前 Linux 环境缺原生 GTK/WebKit，Windows HWND、系统权限、真实展示/点击/撤回、休眠恢复与退出需在真实平台验收 |
| G-L / G-W / G-M | 完整 G-L 继续按既有指示暂缓；G-W/G-M 与安装/卸载未运行，不新设替代门禁或勾选人工项 |
| 真实任务帮助效果 | NOT RUN，真实任务样本数 0；没有完成率、节省时间、准确率或满意度结论。需经许可的代表性任务比较，分列自报/核实/未知结果，未回应不当负反馈 |

因此本轮源码可以标 REVIEWED，不能把整个 U7、Goal 1、正式 Goal 2 或 Wave 1 标为关闭、GATED 或 INTEGRATED。保留待验收事实，不以合成模型或 Linux 状态机测试替代。

### U5/U7 最终实际验证（Linux，同一工具链与依赖锁）

| 检查 | 实际结果与范围 |
| --- | --- |
| `cargo test -p soulcore -p soul-store -p soul-store-api -p soul-profile -p soul-draft --locked` | 完整退出 0；92 个测试结果输出，615 通过、0 失败、1 项既有 Windows 专项测量忽略，含 7 个空套件。新增 U5 11 项与 U7 2 项均包含其中；不重复累计先前定向结果 |
| `pnpm --filter @soul/desktop lint`、`test`、production `build` | 全部退出 0；完整 53 文件 / 668 项通过，包含新增 wrapper 和实际 React 交互竞态用例 |
| 根与独立 desktop 格式、schema-freeze、E0、禁词 | 均退出 0；E0 扫描 16 个交付 crate / 317 源码文件，禁词扫描 94 项 / 145 文件。依赖锁与冻结 JSON schema 未改；内部 SQL schema 为 7 |
| 既有 `command_surface.rs` 的纯源码断言 | 6 项通过，核对实际 80 个命令的两端名称、注册、薄转发与异步调度；适配方式同 U1，不执行 Tauri handler |
| 原生 desktop / IPC / Windows、正式平台门禁、真实任务效用 | 未运行，范围与原因见上表；根测试包含的合成 OS 状态机与 production driver 不证明这些验收通过 |

U5 首轮专项有一项测试 fixture 在故意移除目标密封 blob 后错误地调用需要正文的详情读取来断言元数据；改为真实 store 的元数据读取，仍保留预览拒绝、精确模板/事项不变和恢复同一 blob 后可预览的断言。前端首轮仅一项路径测试失败：Vite 将 `new URL` 的源码路径改写为 HTTP 资源；按既有契约测试使用 `fileURLToPath` 与路径拼接，保留真实 Rust BODY_SLOT 常量断言，随后完整通过。

五包全量首轮在已有 `session_matrix_replay` 二进制启动前因缓存缺执行位中断，不能计为通过。确认其为非空 ELF 且是该目录唯一缺执行位的有效测试产物后，仅增加该文件的用户执行权限，再以原五包命令完整重跑退出 0；没有修改源码或放宽断言来绕过。

命令静态断言首轮发现新增异步预览未列入既有调度分支，错误按同步函数查找。已加入精确 begin/execute/complete 检查，仅忽略 rustfmt 空白差异；spawn_blocking、禁止持有 guard 跨 execute 及禁止壳层策略构造的既有断言保留。修正经独立只读复审，desktop 格式及原 6 条测试复跑通过；产品实现未因该检查改动。

## GitHub 同步与后续阶段

作者 2026-10-19 明确要求定期同步 GitHub。本轮按完成复审及必要验证的工作包同步到 `codex/soul-rebuild-20261019`。命令行没有推送凭据，改用已连接 GitHub 的 Git 对象接口；同步后核对 Git tree，不能把不同提交 SHA 冒称同一提交。

U1/U2 本地提交 `0c9288625dbec374b000b17a043af031b272410b` 对应远端 `a8399e5ba99c96c778a0c1b154f0bdda527891f7`，两者文件树均为 `997f5d9aab9421c6a1f66a76f3ad9ac907f60f54`。远端提交包含原计划与本次实现，不包含当时尚未验收的 U3 dirty 修改。

U3 本地提交 `1c8fca96e724c3c88dfcadd61ee258765f3b1bc9` 对应远端 `ba2b4f6f77305b2cc7e1ead5ad82adff860aed14`，两者文件树均为 `a75f1888e244793b21df2a4caa2e96c95476e718`。同步只推进当前开发分支，没有合并。

U4/U6 本地提交 `293d1c6d2fffb8e06eab644b3ed553bdafb2371c` 对应远端 `8ee2675e39f7d19bb26c290506e601b693198fba`，两者文件树均为 `aa396f2426068eca0b3dd8d62c2a94ed17f44d3a`。U5 编码从这一已复审、验证、同步的检查点继续。

U1–U6 均已实现并完成独立源码复审（REVIEWED），U7 当前环境可执行的合成集成验证已完成。本提交包含 U5/U7 源码、最终实际结果及当前文档，继续按同一方式同步并核对文件树。完整 G-L 仍按已有指示暂缓，G-W/G-M、原生桌面/Windows 通知与真实任务效用未执行；完整 U7、Goal 1、正式 Goal 2、Wave 1 没有因此关闭，未合并任何分支。

## 核心闭环补强（2026-10-19，Windows）

作者最新指示为核心优先，具体系统功能后期打磨；完整平台及真实效用验收仍保留缺口。起点 `b1147e3` 已有未提交的 G-M 文档，保留这些修改。本轮沿既有 D97 / U7 补充核心跨层组合，不新增生产 API、数据迁移、授权或算法规则，亦未发现需要修补的生产缺陷。

| 新轨迹 | 实际结果与覆盖 |
| --- | --- |
| 来源遗忘在发送前 | 同一 Session/SQLCipher 经导入、固定来源关联、成功模型起草控制，再准备新的未消费批准；通过现有存储 API 遗忘来源，不调用 Session 清理。旧批准被拒绝，loopback 请求数仍为 1，不新增接受记录；目标自身与独立目标 Memory 保留。 |
| 来源遗忘在 execute 后 / accept 前 | 第二次真实 loopback HTTP 已执行，再遗忘来源；接收旧正文被拒绝，审计仍有实际 2 次 E1，`draft.create` 仍只有控制组的 1 次。重启不恢复旧批准；明确清除失效来源后，同一目标重新以自身 Memory 本地起草；最后明确遗忘目标才删除其自身内容。 |
| 公开 IPC 目标遗忘 / 重启 | 经生产注册 handler 导入、关联目标、成功 E1 控制，反馈不完成目标。准备新计划后，经公开 Memory 影响面预览与确认遗忘目标，旧批准拒绝且不新增请求；重启后正文仍拒读、目标为 forgotten，独立目标及导入来源保留，审计有效且无正文。 |

源码为 `crates/soulcore/tests/unified_assistance_source_forget.rs`（两个核心轨迹）、`apps/desktop/src-tauri/tests/ipc_roundtrip/assistance_forget_flow.rs`（一个 IPC 轨迹）；原 `ipc_roundtrip.rs` 仅增加模块注册，避免继续扩张大文件。测试使用合成 fixture、临时加密库及 loopback MockLlm。联系人/来源遗忘属于核心存储 API，未伪称新增桌面入口；IPC 中只调用既有公开 Memory 遗忘命令。

| 最终实际命令 | 退出 / 结果 |
| --- | --- |
| `cargo test -p soulcore --test unified_assistance_source_forget --test unified_assistance_flow --locked --offline` | 0；4 通过（新增 2、既有 2），0 失败 / 忽略 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip --locked --offline` | 0；完整 IPC 68 通过，包含新增 1，0 失败 / 忽略 |
| 根 / desktop `cargo fmt --all -- --check`（desktop 指定独立 manifest）、`git diff --check` | 均退出 0；六份本轮文档的相对链接目标存在 |

运行环境沿用本机 Rust 1.83 / MSVC，进程内 `CARGO_BUILD_JOBS=4`、DEV/TEST DEBUG=0、INCREMENTAL=0，复用本机已构建 OpenSSL 静态库；未改变两个 target 或全局环境。原始输出与退出码位于 `E:\Project\Soul-checks-20261019-win\core-continuation`：`source-forget-core-final.log` / `*-result.json`、`ipc-forget-core-final.log` / `*-result.json`。初轮 `source-forget-core.log` 因测试误写 `Session.audit_chain` 编译退出 101，修正为真实 `Session.audit`；不将其计为已复现业务缺陷，亦不重复累计中间运行。

实际测试 diff 及增强断言已独立只读复核，无阻塞问题；覆盖固定来源整体复验，不能单独证明每个内部防线分别命中。确定性轨迹在 HTTP execute 完成后、交付前遗忘，不声称覆盖 HTTP 仍在途时遗忘、真实 WebView 旧正文消失、通知展示或任务质量提升。生产逻辑/命令/锁/依赖/schema 均未修改；没有运行本轮完整 G-W 或 G-L，没有安装、卸载或合并。原执行阶段尚未提交或推送；后续接管复验与阶段提交见下节。

### 接管复核与阶段提交

作者随后明确授权接管以上既有修改，复核后分阶段提交。本次先读实际 diff，由独立只读 reviewer 核对测试的正向控制、同库来源失效、真实 IPC、请求与接受审计和重启断言，无阻塞发现。主会话重新执行上表两条测试命令：核心 4 通过、独立 desktop IPC 68 通过，均退出 0；定向 `rustfmt --edition 2021 --check` 检查核心测试和 `ipc_roundtrip.rs`（含新增子模块），以及相关路径 `git diff --check` 均退出 0。重复执行不累计为新增测试。

本次仍为 Windows / pwsh 7.6.5 / Rust 1.83.0，沿用上述仅进程内构建参数及已存在的 OpenSSL 静态库。新证据为 `E:\Project\Soul-checks-20261019-continuation\initial-core.log`、`initial-ipc.log` 与各自 `*-exit.txt`。以上测试和记录随本节提交；原 G-M 观察单独收录，不代签人工验收，不扩称新 G-W、真实效用、推送或合并。

## U7：真实 HTTP 在途来源遗忘

基于 `7814fe3`，独立规划复核定位原轨迹只覆盖发送前、execute 后/accept 前的差集。新增 `crates/soulcore/tests/unified_assistance_inflight_forget.rs`，沿用 D97 与现有 store 遗忘 API，不改生产接口或授权。

真实 Session/SQLCipher 先成功起草作为正控制；第二个 loopback 服务收到完整 HTTP body 后通过 channel 通知，只有显式释放才回复。直接同库 Contact 遗忘在回复前完成，worker 完成 channel 仍空，普通新准备精确返回 Busy。释放后旧结果为 `PLAN_HASH_MISMATCH`，实际 `egress.request=2`、`draft.create=1`；槽恢复后可重新准备。经真实导入索引选择该联系人消息，核对 event body 与索引 CK 一致、遗忘前可读，遗忘后及重启后均精确拒绝该 CK；独立目标仍可读，独立未消费批准也不能跨重启使用。审计无合成正文。

主会话实际执行 `cargo test -p soulcore --test unified_assistance_inflight_forget --test session_draft_work --test unified_assistance_source_forget --locked --offline`：**13 通过、0 失败，退出 0**，含新增 1、既有 12。环境沿用上节，证据为 `E:\Project\Soul-checks-20261019-continuation\inflight-forget-final.log` / `inflight-forget-final-exit.txt`。初轮 `inflight-forget.log` 退出 101：fixture 没有可选联系人标签，测试在构造 CK 控制时失败；修正测试选择后重跑，不称业务缺陷红绿修复。最终新文件 `rustfmt --edition 2021 --check` 退出 0，实际源码经独立只读复核无阻塞。

同步依赖完整请求及显式响应 channel，不以 sleep 推测 HTTP 阶段；accept/read/release 均有界。成功路径 join worker；断言失败路径的部分 worker 可能短暂 detach，端点 Drop 释放响应并 join 服务线程。该验证不替代真实 WebView 旧正文清除、OS 通知、外部服务效用或完整平台门禁；没有修改生产逻辑、数据迁移、依赖锁和算法。

## U7：无 Drop 进程退出后的持久恢复

新增 `crates/soulcore/tests/unified_assistance_crash.rs`，沿用 D98。既有正常 Drop/reopen 测试不足以证明进程死亡后 OS lease 与 Running claim 的组合恢复；本片使用当前测试二进制的精确子入口、临时 SQLCipher 库和生产 driver，不加生产 failpoint 或恢复机制。

- 子进程持有未 execute/Drop 的 owned work，真实读取 Running/claim/revision；经既有 Memory 更新 API 在同库产生不同的最新 pending revision，再回读确认。最后只写 UUID manifest 并同步，以 `process::exit(86)` 结束；父进程等待真正退出，只接受 86，重新打开同目录，验证最新 Pending→Ready、旧 claim 不算完成，后续两轮及再次重启均 Idle。
- 生产物理槽不允许前台准备与后台 claim 同驻，因此批准恢复另用一条子进程轨迹：保留真实未消费批准、held memory context 及 FakeForegroundSource 的采集同意后无 Drop 退出。重启先核对同意和前台材料清零，再恢复同一 loopback 端点，旧批准与上下文拒绝，端点请求数仍为零、审计无正文及出网/起草接受记录。

主会话执行 `cargo test -p soulcore --test unified_assistance_crash --test assistance_runtime_driver --test session_assistance_work --locked --offline`：**20 项输出通过、0 失败，退出 0**。分别为既有 driver 5、Session work 11、新文件 4；新文件含 **2 条实质父轨迹与 2 个普通执行时 noop 的子入口**，不报告为四个新场景。日志及退出码：`E:\Project\Soul-checks-20261019-continuation\crash-recovery.log` / `crash-recovery-exit.txt`。环境同上，实际源码和计数已独立只读复核，无阻塞发现；新文件格式与提交 diff 检查退出 0。

父进程等待期限 20 秒，异常路径 guard 执行 kill/reap，避免遗留仍持 lease 的子进程；未故障注入 kill 失败，底层 wait 没有第二层独立超时。该结果只证明已持久 Running 状态下无 Drop 退出恢复，不证明事务中途断电、真实采集或 Windows UI。生产逻辑、权限、依赖/schema/算法均未改变，完整 G-L 和人工/效用验收仍保留原边界。
