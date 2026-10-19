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

## GitHub 同步与后续阶段

作者 2026-10-19 明确要求定期同步 GitHub。本轮按完成复审及必要验证的工作包同步到 `codex/soul-rebuild-20261019`。命令行没有推送凭据，改用已连接 GitHub 的 Git 对象接口；同步后核对 Git tree，不能把不同提交 SHA 冒称同一提交。

U1/U2 本地提交 `0c9288625dbec374b000b17a043af031b272410b` 对应远端 `a8399e5ba99c96c778a0c1b154f0bdda527891f7`，两者文件树均为 `997f5d9aab9421c6a1f66a76f3ad9ac907f60f54`。远端提交包含原计划与本次实现，不包含当时尚未验收的 U3 dirty 修改。

U3 本地提交 `1c8fca96e724c3c88dfcadd61ee258765f3b1bc9` 对应远端 `ba2b4f6f77305b2cc7e1ead5ad82adff860aed14`，两者文件树均为 `a75f1888e244793b21df2a4caa2e96c95476e718`。同步只推进当前开发分支，没有合并。

U4/U6 已完成实现与独立源码复审（REVIEWED），本次提交后继续按同一方式同步。U5 自动本地准备已完成方案复核，见 D100；关闭模板准备保留共用 U3 待办，避免取消模板时丢失新业务版本。U7 本轮集成与效用验证仍未完成。完整 G-L 仍按已有指示暂缓，G-W/G-M 与真实任务效用未执行；Goal 1、正式 Goal 2、Wave 1 没有因本轮局部验证关闭，未合并任何分支。
