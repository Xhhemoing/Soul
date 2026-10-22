# 程序、算法与架构综合测试和结果分析

日期：2026-10-21。用户要求主会话统筹思考、分派测试、分析结果并记录全过程，子 Agent 使用 `gpt-6.1-sol`、`high`。开工候选为 `codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`；既有全部修改保留。当前结论入口仍为 [STATUS](../STATUS.md)，本文件保存本次覆盖、执行与分析依据，不另立常驻任务账本。

## 目标与安排

在当前 Windows 合成测试条件下，建立程序正确性、冻结算法、桌面接口与架构行为的横向证据，再由实际失败或覆盖差集决定是否增加最小验证。根 19-crate workspace、独立 Tauri workspace、React、Python 读取器分开记录；不能用单层通过替代跨层结果。既有第五批性能观察保留原样本与版本，不机械重复，也不把本次普通回归称为长期性能测量。

主会话持有唯一完整构建/测试调度通道，负责整合记录和最终结论。三名指定模型的子 Agent 先独立核对不同覆盖面，再解析各自原始日志和反例；若需写代码另明确认领，最多两名写者，单文件一名写者。修复由其他 Agent 独立只读评审，不能自签。授权、事务和 schema 共享协议的变更仍串行并遵守原决定流程。

| 负责人 | 检查面 | 必须解释的结果 |
| --- | --- | --- |
| `program_test_lead`（6.1-sol/high） | 加密库、事务/遗忘/重启、导入/Session/证据、策略/脱敏/E1 | 数据和授权断言是否经过真实临时库与受控 loopback；拒绝时的副作用；剩余反例 |
| `algorithm_test_lead`（6.1-sol/high） | T4D/A0/A1/A2/A3、as_of、用户纠正、候选与反证、VAL 分母 | 判据来自既有权威；测试 oracle 是否独立；正确性、能力限制和真人效用分开 |
| `architecture_test_lead`（6.1-sol/high） | 独立 Tauri、React/IPC/异步旧结果、构建、独立读取器、已有规模实验 | 接口到实际 handler 的证据、可发布构建与安装的区别、成本/故障结论的适用范围 |
| 主会话 | 调度、必要集成、实际命令/退出码、跨层分析 | 通过/失败/忽略/排除去重；失败性质与修复后回归；实际剩余风险 |

## 执行边界与覆盖矩阵

只使用合成 fixtures、临时加密库和受控 loopback，不读取真实库、导出、密钥或凭据，不启动真实采集、调用外部模型、安装应用或提交代码。G-L 继续暂缓；本次 dirty 候选的筛选测试和前端生产构建不签发 G-W/G-M、正式 Q2 或 Goal 关闭。所有 ignored 入口先辨明用途，不使用笼统 `--include-ignored`。

| 检查面 | 本次既有执行入口 | 排除与解释 |
| --- | --- | --- |
| 根 Rust workspace 普通测试 | `cargo test --workspace --all-targets --locked --offline -- --skip windows::runs_on_windows` | 排除 Windows 真实源模块；ignored 保持原状态，默认测试实际安全性由程序负责人先核对 |
| 根文档示例 | `cargo test --workspace --doc --locked --offline` | 与 all-targets 分开，不把未执行 doc-test 计作通过 |
| 独立桌面 workspace | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again --skip windows::runs_on_windows` | 两项真实采集 IPC 明确排除；真实 command surface/handler 的合成测试仍执行 |
| React 静态及交互 | `pnpm --filter @soul/desktop lint`；`pnpm --filter @soul/desktop test` | 使用既有 2 workers；UI mock 与 Tauri 实际 handler 证据分开 |
| 前端生产构建 | `pnpm --filter @soul/desktop build` | 子进程临时 `NODE_ENV=production`，finally 恢复；不等于安装包或完整桌面发布 |
| 独立研究读取器 | `python -B -m unittest discover -s scripts/tests -p test_research_retention_reader.py` | 仅合成 ZIP；独立于 Soul 密钥与运行态 |
| 既有架构约束 | `cargo run -q -p xtask --locked --offline -- schema-freeze --check`、`e0-audit`、`denylist-audit` | 保留已有检查，不新增 gate；预检不能替代以上真实测试 |
| 编译质量 | 根与桌面 `cargo clippy ... --all-targets --locked --offline -- -D warnings`，两 workspace fmt check | 不降级 lint；不整仓重写格式或更新锁文件 |

Rust 沿用工具链 1.83、已有 root/desktop target、JOBS=4、DEV/TEST DEBUG=0、INCREMENTAL=0，以及已存在的 OpenSSL 静态安装；不改变全局配置或安装依赖。实际命令会记录每项启动/结束、退出码、标准输出错误及环境。日志和临时任务材料位于仓库外 `C:/Users/86080/AppData/Local/Temp/soul-comprehensive-8fba6660c53343d9ba4c0f06ad43743b/`。

实际环境：Windows 11（10.0.26200），AMD Ryzen 7 H 260（8 核/16 线程），可见物理内存 16,004,104 KiB；pwsh 7.6.5、rustc/cargo 1.83.0、Node 24.18.0、pnpm 10.15.0、Python 3.13.14。硬件信息只用于界定本次运行环境，构建总耗时不作为程序响应性能。

## 分析方法与停止条件

1. 每个检查面记录其输入、关键断言和实际执行范围；测试数量用于核验执行，不替代覆盖解释。
2. 失败区分编译/环境、测试假设问题、产品功能、能力限制。保留首次失败；先复现和定位，再修改最小认领路径，不通过降低断言、扩大权限或重试挑样获得通过。
3. 对算法优先利用既有权威 fixture、置换/时间平移/用户覆盖等交叉校验；有具体缺口时才补最多必要的反例。mock 模型只支持请求/协议结论。
4. 新源码或修复后重跑受影响范围；此前失败的完整命令必须有修后结果才能记该范围通过。无新变化时不重复全库扫描。
5. 结果分析逐项写明事实、可能解释、证据限制和下一动作。成本结论保留量纲、负载形状及样本数，不从三次样本推导 p95 或自然使用保证。
6. 本次结束条件为既有可执行检查面完成，实际失败完成处理或明确阻塞，关键结果经独立核验，分析记录和 STATUS 更新。平台/真实效用等未覆盖项如实保留，不无限扩展测试。

## 执行与分析记录

已完成开工 Git 核对、职责分派和测试入口安全性调查。以下先保留覆盖调查，再记录本轮实际运行；源码中的断言与日志中的执行证据分开。

### 覆盖调查与分析判据

| 工作面 | 已存在的高价值验证 | 分析中必须保留的边界 |
| --- | --- | --- |
| 存储/导入/遗忘 | `no_plaintext_at_rest` 写后扫描并重开正控制；`crash_recovery` / `session_crash` 子进程死亡；`session_import_work` 审计失败整事务回滚；`indexed_import` 重叠/冲突/遗忘重导；N1/N2 两连接与迟到结果反例 | 合成临时 SQLCipher 与 DPAPI 是真实平台调用，不等于真实用户数据或断电实验；测试专用恢复不转为生产恢复 |
| 授权与证据 | `session_e1` / `e1_origin` 的请求计数与 wire 检查；`session_evidence_sources` 精确锚/主体/用途/共享预算；研究保存单次消费、源身份替换、取消/发布先胜 | dispatch 前拒绝才可称零出站；`session_draft_work` 已颁发 permit 后撤回仍可能有请求，必须拒绝迟到 completion，不能改写为零 socket |
| T4D 与图谱 | `ablation` 冻结语料、`direct_gate` 有限固定种子性质；`t4d_fixture_parity` 经产品 SQLCipher/rebuild 对照原始计数与档位；全库 as_of、时间置换与坏来源回滚 | `goal1_fidelity` 是 T0 历史 oracle，不能充当 T4D 证明；规则一致不是关系预测效果 |
| A0/A1/A2/A3 | intake/replay 对照、用户锁值与机器值分离、多来源独立性；A2 消费档位和证据；A3 文本推断拒绝 | A1 在 v0.1 来源条件下空转是既定行为；不新增评分阈值或人格效果承诺 |
| 候选与 VAL | 选择顺序/上限影响反证、遗忘后旧 context 拒绝；VAL 缺计时、失败、撤回、左右互换与各自分母 | 关键词/顺序过滤限制必须报告；固定回复和合成元数据不计入真人效用样本 |
| 桌面/前端 | 真实 Tauri 注册 handler 的 origin/参数/单次确认/文件断言；React wrapper 队列、卸载重挂、迟到结果与取消后真实收据 | jsdom/mock invoke/Tauri mock runtime 不是原生 WebView、通知、安装或 UI 响应时延的证据 |

根测试入口安全性经调用链定向核查：Windows 真实 sample 位于明确排除的模块；其余默认来源创建只 describe、受 cfg 限定或在 factory 前 busy 拒绝。默认 HTTP 使用受控 loopback 且客户端禁用系统代理。普通测试中仍有真实 DPAPI、socket 观察、PowerShell 和临时 rustc 子程序；安装脚本回归只针对临时假安装，不调用 NSIS 或用户 Soul 安装。ignored 的 Q2、手工操作、真实/特权辅助入口未整体启用。

程序与算法两线均未提出需要立即新增测试的具体反例，因此本轮先运行既有高价值覆盖。架构线保留两个差集：完整 ZIP 发布与采集停止同时发生，以及构建包装器非零退出的实际传播；后者本轮用下面的合成实验补证，前者不由既有查询压力和全取消样本推导。

### 独立 VAL 算术重算

算法子 Agent 只读 `crates/soulcore/tests/val01_evaluation/synthetic_manual.json`，使用 PowerShell 独立计算，没有调用 Rust validator/aggregate。实际脚本与日志为同一仓库外目录的 `val-independent.ps1`、`val-independent.log`；脚本及调用进程退出码均为 0，12 项既有断言和 2 项补充核计符合预期。

登记/assigned/completed 均为 3；偏好 B 胜 1、平局 1、分母 3，分值 0.5。缺 edit 的 M1 仍计入完成和偏好；M2 control=0、target=90，差为 -90 且比例不可算；M3 的 80→100，差为 -20、比例 -0.25。因而 edit_valid=2、missing=1，差中位数 -55、比例中位数 -0.25，与 `manual_tests.rs` 既有断言一致。该独立重算只支持这个固定合成样本的算术和分母解释，不证明一般输入验证或真人效用。

### 构建包装失败传播实验

架构子 Agent 从当前 `scripts/gate-win.ps1` 的 PowerShell AST 原样提取 `Step` 和 `With-NodeEnv`，只执行函数片段，不运行原脚本主体。使用原生 pwsh 子进程 stub 返回 0/37；原 NODE_ENV 为缺省、test、probe-original 三种，共六项组合。六次子进程日志均显示 production；三次 0 正确记录 OK，三次 37 正确记录 FAIL 并抛异常，六次均恢复原变量存在性和值。纯探针最终退出 0。

原始脚本/子进程 stub/函数片段/日志/元数据均在本次仓库外日志目录，文件名分别为 `gate-wrapper-probe.ps1`、`build-stub.ps1`、`Step.actual.ps1`、`With-NodeEnv.actual.ps1`、`gate-wrapper-probe.log`、`gate-wrapper-probe-meta.json`。首次探针本身退出 0，但附带缺失可选路径检查使宿主退出 1；最终移除该附带检查重跑纯探针，保留此区别。程序负责人独立重取 AST 对比两函数、核对六条日志，确认定义一致且没有运行完整构建/安装主体。

本次没有覆盖真实 pnpm shim 的失败、With-NodeEnv test 模式、空字符串环境、PowerShell body 自身异常或真实发布。探针的 pass 布尔未自动断言子进程 production 文本，六条原始文本已独立逐项核对；不把该有限实证签发为完整 G-W。

另发现 `justfile` 全局 NODE_ENV=test，而 ui-build 未在配方中覆盖生产值。这是具体静态配置风险；本机 PATH 没有 just，本轮没有执行该 recipe，也没有用替代 shell 假称复现。本轮实际生产构建使用显式 production 的 pnpm 入口。该风险与 Windows gate 已验证的环境包装分别记录。

### 根工作区实际结果

本轮 `root-tests.log` 的命令与上表一致，UTC 11:34:05 至 11:41:21，含编译约 436.05 秒，末尾 `EXIT=0`。程序负责人独立解析 179 个顶层 binary 与 179 条 summary，累计 **1490 passed、0 failed、12 ignored、3 filtered**；再按 `test … ok` 行核算同为 1490。`publication` 内两次子进程 helper 的 `running 1 test` 不另算完成用例。未见编译 warning/error、FAILED 状态或未捕获 panic 输出。

| crate | passed | ignored | filtered |
| --- | ---: | ---: | ---: |
| soul-algo-tie | 134 | 0 | 0 |
| soul-algo-trait | 114 | 0 | 0 |
| soul-collect | 31 | 0 | 3 |
| soul-draft | 94 | 0 | 0 |
| soul-egress | 12 | 0 | 0 |
| soul-fileplan | 66 | 0 | 0 |
| soul-graph | 63 | 0 | 0 |
| soul-import | 73 | 0 | 0 |
| soul-local-files | 14 | 2 | 0 |
| soul-memory | 25 | 0 | 0 |
| soul-policy | 90 | 0 | 0 |
| soul-profile | 26 | 0 | 0 |
| soul-schema | 21 | 0 | 0 |
| soul-store | 123 | 0 | 0 |
| soul-store-api | 9 | 0 | 0 |
| soul-testkit | 26 | 0 | 0 |
| soul-win-dpapi | 8 | 0 | 0 |
| soulcore（含 headless binary） | 525 | 10 | 0 |
| xtask | 36 | 0 | 0 |

12 项 ignored 为：local-files reparse 权限用例/子进程 helper 2 项，核心跨语言 ZIP reader 1 项，MODEL 显式报告 1 项，Q2 显式规模 1 项，research growth/path/save-cancel/stop 4 项，VAL manual/operator/exercise 3 项。3 项 filtered 均来自 `soul-collect::windows::runs_on_windows`，并非失败。没有将此前显式实验搬入本轮数量。

根 doc-tests 独立命令亦退出 0，耗时约 12.96 秒。19 个 crate 段中只有 `soul-algo-tie/src/lib.rs:11` 有实际用例，结果 **1 passed**；其余 18 段为 0，不计作额外 19 项测试。原始结束证据分别为 `root-tests.log:2602` 与 `root-doc-tests.log:119`。

### 程序与算法结果解释

| 本轮实际证据 | 可以得出的结论 | 不能外推的结论 |
| --- | --- | --- |
| 根日志 1434–1435、1450、1462、1486–1487、1982–1983、2026–2028、2079/2081/2083 | 加密落盘/无 key 读取、遗忘重启、迁移与 receipt 事务、坏时间整单回滚、审计失败回滚、Session 子进程中断均实际通过 | 未测试物理断电、真实用户库或通用生产恢复；N1/N2 仍为 test-repair 原型 |
| 根日志 2116–2120、2133–2134、2148/2160、2420–2421、2436、2443–2444 | E1 拒绝与请求计数、origin、证据失效、重启许可、在途旧结果拒绝符合当前边界 | 撤回已经发出的许可不撤销已经发生的网络副作用；不能一概称撤回后零出网 |
| algo-tie 134、graph 63；其中冻结 fixture→SQLCipher 产品对照 3 项，T4D product 12 项 | 冻结锚、时间衰减边界、私聊门槛、全局 as_of、用户锁值/机器值分离及实际产品适配一致 | 固定种子性质不是数学穷尽证明；旧 T0 fidelity 5 项不能替代 T4D oracle；未证明真人关系推断效果 |
| algo-trait 114、profile 26、draft 94、memory 25 | A0 intake/replay 和纠正锁、A1 来源独立性、A2 纯渲染、A3 文本拒绝及证据引用通过 | A1 v0.1 空转是既定能力边界；无新增阈值、人格判断效果或生产模型效用证据 |
| `research_selection_boundaries` 2、`research_time_refusal` 2 | 候选顺序/上限、遗忘反证、引用回指、official-import offset 等价与坏时间回滚实际通过 | 关键词/顺序筛选仍不等于语义冲突识别 |
| VAL 91 passed / 3 ignored；独立 PowerShell 重算一致 | 缺测、配对方向、各指标分母、撤回和安全记录的合成断言通过 | 任务数不等于参与者数；固定合成卡不计真人效用样本，MODEL ignored 不计新实验 |

VAL 91 项按实际 test 行再拆分为 endpoint 18、local 10、manual 7、missing_timing 8、operator 13、presentation 13、record 21、session_bridge 1。两名子 Agent 分别核验程序/算法原始结果，均未提出新的已复现产品缺陷；这只描述本次覆盖范围，不能理解为缺陷不存在。

### 桌面、前端与独立读取器实际结果

| 检查 | 本轮结果 | 原始证据 |
| --- | --- | --- |
| 独立 Tauri workspace | 130 passed / 0 failed / 0 ignored / 2 filtered；exit 0 | `desktop-tests.log:176`；含编译约 12.30 秒 |
| 前端类型与 ESLint | `tsc --noEmit && eslint .`，exit 0 | `frontend-lint.log:5`、`:7`；约 26.80 秒 |
| 前端 Vitest | 55 文件 / 685 tests passed，exit 0，NODE_ENV=test | `frontend-tests.log:12`–`:17`；Vitest 总 123.37 秒，外层约 126.75 秒 |
| 前端生产包 | `tsc --noEmit && vite build`，production，exit 0，无 warning/error | `frontend-production-build.log:3`、`:6`、`:8`、`:18`；外层约 11.11 秒 |
| Python 独立读取器 | 7 tests / OK / exit 0 | `python-reader.log:5`、`:7`、`:8`；unittest 0.890 秒，外层约 1.75 秒 |

桌面 130 项分为 lib 29、command_surface 6、ipc_roundtrip 67、no_egress_path 3、one_store 3、shell_is_local_only 22；main 为 0。两项 filtered 是已明确排除的真实采集 IPC。日志 49/53/87/113 行分别确认两端命令名一致、每个命令实际注册、真实 Session/request shape、研究保存在真实 handler 与合成 collection 下通过。Tauri mock runtime 实际调用 handler，但未使用真实 WebView。

前端日志为精简总计，没有逐 suite 打印；不伪造单项测试的日志行。源码覆盖调查中的异步队列、卸载重挂、迟到结果与取消后收据属于该测试集合；本轮实际证据是完整 55 文件/685 项通过。生产构建转换 61 个模块，输出 HTML 0.39 kB、CSS 3.35 kB、JS 362.19 kB（gzip 108.87 kB）。Vite 阶段 336 毫秒不含前置 TypeScript，不写成完整构建耗时。

读取器的 7 个 unittest 以合成 ZIP 覆盖 CRC/截断、体积、重复键/额外字段、压缩/加密标志、时间/排序/计数守恒和命令行拒绝；不依赖 Soul 运行态和内容密钥。其通过不能替代跨语言保存入口 ignored 用例的新执行，也不能声称取消探针的新超时清理分支被执行。

上述三份最终日志由架构负责人独立核对。证据支持接口逻辑、前端回归和可构建性；真实窗口响应、通知、发布模式嵌入、NSIS/安装及真人数据仍不在本轮覆盖内。

### 桌面严格 Clippy 首次失败与处理

根严格 Clippy 已退出 0；独立桌面相同严格级别首次退出 **101**，原始日志 `desktop-clippy.log` 保留，主 runner 立即中止，没有继续把后续检查记为通过。错误为 `apps/desktop/src-tauri/src/lib.rs:189` 的 `window.eval(&format!(…))` 触发 `needless_borrows_for_generic_args`，普通 lib 与 lib-test 均受影响。该语句不属于本轮新增的研究保存注册三行；原有 dirty 修改保留。

根因调查已对照实际诊断与调用位置：API 泛型接受格式化字符串，多余借用导致 `-D warnings` 失败。按系统化调试流程，将单文件最小修复分派给 `desktop_lint_fix`（同为 gpt-6.1-sol/high），不改变路由文本、typed UUID 来源、通知时序或 lint 级别。首次失败是已执行检查发现的代码质量问题，不写成环境失败，也不等同运行时通知故障。

实现者只去掉多余 `&`，修改前快照为同一日志目录的 `desktop-lib-before-lint.rs`。程序负责人先比较实际 diff 与快照，再看说明，独立核对本地 Tauri 2.11.5 的 `eval(&self, js: impl Into<String>)`，并确认回调仍为 `AssistanceNotificationId = uuid::Uuid`，格式串和调用顺序不变，无新增 allow/依赖/覆盖。既有三行研究保存 command 注册已在修改前快照中，不归入本轮修复。

相同完整桌面 Clippy 命令重跑退出 0（`desktop-clippy-recheck.log`）；相同过滤桌面测试再次 **130 passed / 0 failed / 0 ignored / 2 filtered**（`desktop-tests-recheck.log`），约 40.39 秒含重新编译。随后普通 core 库 check 与根 fmt 退出 0。桌面 fmt 首次退出 **1**，只要求刚修正的表达式改为指定两行；`desktop-format.log` 保留。实现者仅调整该表达式换行，快照为 `desktop-lib-before-format.rs`，非空白内容不变。最终桌面 fmt 与完整严格 Clippy 又均退出 0，原始日志为 `desktop-format-recheck.log`、`desktop-clippy-final.log`。

桌面 130 项在去借用后执行，在最终仅换行后未重复运行；最终 Clippy 检查的是格式收尾后的源码。根 1490 项在此次桌面修改前执行，根 workspace 源码未受影响，不宣称它们也进行了修后重跑。初次与复验的桌面 130 不累计为 260 项独立用例。调查中一次 `rg | Select-Object -First 12` 的只读搜索因管道截断返回 1，随后改用 UTF-8 文本读取成功；它与上述两次产品检查失败分开记录。

### 最终检查汇总

执行记录 `execution.jsonl` 共 19 条命令回执，17 次 exit 0、桌面 Clippy 首次 101、桌面格式首次 1；两项失败均有保留原日志及修复后的成功回执。以下按最终覆盖面列示，不把审计/编译当成功能测试，也不重复累计复验。串行命令执行及修复窗口为 UTC 11:34:05–11:51:53，不是程序运行性能或本任务全部调查耗时。

| 检查面 | 最终结果 | 最终日志（本次仓库外目录） |
| --- | --- | --- |
| 根普通测试 | 1490 通过 / 12 ignored / 3 filtered，exit 0 | `root-tests.log` |
| 根 doc-tests | 1 通过，exit 0 | `root-doc-tests.log` |
| 独立桌面测试 | 130 通过 / 2 filtered，修后重跑 exit 0 | `desktop-tests-recheck.log` |
| 前端 TypeScript/ESLint | exit 0 | `frontend-lint.log` |
| 前端测试 | 55 文件 / 685 通过，exit 0 | `frontend-tests.log` |
| 前端 production 构建 | exit 0 | `frontend-production-build.log` |
| Python 独立读取器 | 7 通过，exit 0 | `python-reader.log` |
| 既有 schema-freeze | 与已有 lock 一致，exit 0 | `schema-audit.log` |
| 既有 E0 audit | 17 个 shipped crate / 333 源文件，clean，exit 0 | `egress-audit.log` |
| 既有 denylist audit | 94 词 / 157 文件，clean，exit 0 | `denylist-audit.log` |
| 根严格 Clippy | 全 workspace/all-targets、`-D warnings`，exit 0 | `root-clippy.log` |
| 桌面严格 Clippy | 最终格式源码、all-targets、`-D warnings`，exit 0 | `desktop-clippy-final.log` |
| 普通生产配置核心库 | `cargo check -p soulcore --lib --locked --offline`，exit 0；非 test-repair 入口 | `core-production-check.log` |
| 根 Rust 格式 | exit 0 | `root-format.log` |
| 桌面 Rust 格式 | 修正后 exit 0 | `desktop-format-recheck.log` |

常规测试按入口合计 2313 项通过（1490 + 1 + 130 + 685 + 7），不宣称是 2313 个互不重叠的业务性质；12 项 ignored、根/桌面合计 5 项真实采集 filtered 保留。另有独立 VAL 算术核算与六组合构建包装实验，分别按其原断言解释，不加进上述 Rust/前端/Python 用例总数。

本轮实际源码改动仅为桌面该表达式去除多余借用及符合 rustfmt 的换行；不改变既定产品、算法、授权或数据语义。主会话记录计划/运行/分析并维护 STATUS/PLAN_INDEX，各子 Agent 使用指定 6.1-sol/high；过程未 commit/push、安装应用、读取真实数据、开启真实采集或联系外部模型。最终差异、19 条回执、去重计数及结论限制已完成[独立只读核验](../reviews/2026-10-21-comprehensive-validation.md)，无剩余具体发现。结论为本次选定的 Windows 合成检查通过，正式平台/人工/真人效用结论仍按下节保留。

### 风险、能力限制与后续优先级

1. **构建入口一致性，静态风险。** `justfile` 的 ui-build 继承 test 环境与生产构建要求不一致，Windows gate 的生产包装已有限实测通过。后续应在可运行 just 的环境验证该入口并明确生产环境；本轮没有运行 recipe、修改其行为或声称已修复。
2. **规模与锁等待，已有测量支持的调查方向。** [第五批原回执](2026-10-20-research-experiment.md#第五批增长zip-与停止压力2026-10-21)的 10000 事件场景中，追加 9000 导入/重建中位约 4019 ms；保存阶段约 1 秒，停止及取消在查询竞争下达到数百毫秒。它们是先前同日有限样本，本轮不重新计样、也不宣称 p95。优先定位全候选扫描、复验和持锁耗时，再由测量决定优化；不能仅凭耗时相关性认定某个函数是唯一根因。
3. **语义与模型能力，已知限制。** 候选顺序/关键词会影响反证可见性；当前测试证明确定性选择和遗忘边界，不证明系统能理解材料冲突。A1 v0.1 空转与 A3 拒绝按现有权威保留。后续能力改动须先定义可验证目标，不能为了让演示更强而放宽现有规则。
4. **并发和长期观察，未验证。** 完整 ZIP 发布与采集停止同时发生、写盘中精确取消时刻、长期墙钟运行、峰值内存、自然尾延迟仍缺数据；此前全取消样本不填补发布先胜的实际窗口。新增实验应针对这些具体差集，不重复整套已通过的同覆盖面回归。
5. **产品与平台验收，另有条件。** 当前研究实验版已具有数据流、规则和受限研究留存的工程证据。真实任务效用样本仍为 0；G-L 暂缓，G-W/G-M/安装不在本轮。不能标 Goal 1、U7、正式 Goal 2 或 Wave 1 已关闭。
