# STATUS

本文件是当前状态入口。历史见 [2026-09-28 快照](STATUS_HISTORY_20260928.md) 与[遗忘阶段 1–4 记录](SOUL_FORGET_PROGRESS_2026-09-29.md)。历史记录中的“尚未接入”描述其当时版本，不覆盖本页的新进展。

## Windows 密钥读取竞争修复（2026-10-19）

完整 G-W 在干净候选 `629c739` 的并发密钥测试中实际失败，见[失败记录](gates/20261019-629c739-win.md)。受控反例先持独占锁写入合成文件前半段，再调用真实读取：TestKeyProvider 和临时 DPAPI provider 均提前返回 os error 33，settling 路径提前返回 None，新增三项均实跑失败；原并发测试单独运行通过，不能消除已复现竞争。

三个读取入口现共用只读句柄与 shared lock，在同一持锁句柄读取完整字节，再释放锁；真实 open/lock/read 错误传播。保留 create_new、独占发布、空文件恢复及非空损坏拒绝，不 unlink 或覆盖已有密钥，不改变 DPAPI/KEK/DEK 格式。新增损坏 seed 不覆盖控制。修复后 `cargo test -p soul-store --lib --locked --offline` **49 通过、0 失败**，根 workspace 格式及 diff 检查退出 0；证据为仓库外 `E:\Project\Soul-checks-20261019-continuation\keys-claimed-red.log`、`keys-lib-green.log` 与对应退出码文件。Linux advisory lock 半写风险经源码分析，尚未在 Linux 实跑。

已完成独立只读源码与红绿日志复核，无阻塞发现。共享锁获取沿用阻塞式 OS 锁；既有 settle timeout 只限制空文件轮询，不承诺活写者永久持锁时的读取超时。本项局部验证不替代完整 G-W；新干净候选需复跑后另记结果。G-L 暂缓、系统 G-M 后置，Goal 1、正式 Goal 2 与 U7 未关闭。

## 后续评测设计（2026-10-19，实施未启动）

按作者“保留门禁边界，先推进后续方案设计”的选择，完成 [VAL-01 合成评测适配设计](design/2026-10-19-val-01-evaluation-design.md)。基于当前公开 brief / DraftSession / policy API，独立只读复核可行性与逐边算术，明确先记录/汇总、再 brief/T1、最后 T2/T3 和真实 Session 桥接。未新增产品或测试适配源码，示例是合成手算期望，未测用户偏好。正式实施仍依赖 Goal 1 关闭与 Q2-00；G-L 暂缓、系统 G-M 后置与真实样本为 0 的边界不变。

## U7 故障边界续验（2026-10-19）

作者已明确授权接管既有修改、持续按计划推进并逐部分提交。历史 G-M 观察收录于 `9ff02b9`，来源遗忘发送/接收边界及公开 IPC 遗忘回归收录于 `7814fe3`；它们没有改变生产行为或关闭正式验收。

- 在途来源遗忘：新增真实 Session/SQLCipher 与阻塞 loopback HTTP 轨迹。完整请求到达、回复尚未发送时，直接同库来源遗忘可完成；物理工作槽继续准确返回 Busy。放行回复后拒收旧结果，真实请求审计保留、草稿接受数不增加；独立目标保留，来源消息 CK 重启后仍已销毁。额外使用独立目标的未消费批准验证重启拒旧。
- 已完成独立只读源码复核；`cargo test -p soulcore --test unified_assistance_inflight_forget --test session_draft_work --test unified_assistance_source_forget --locked --offline` 实跑 **13 通过**（新增 1、既有 12），退出 0。初轮 fixture 无可选联系人标签导致测试失败，按真实导入索引改取该联系人消息密文后通过；不是生产缺陷修复。
- 进程退出恢复：新增两条真实子进程轨迹，分别在持久 Running claim 带更新 pending revision、以及未消费批准/上下文和合成采集同意存在时，不运行 Rust Drop 直接退出。父进程只接受约定退出码，重新打开同库，经生产 driver 只发布最新版本，后续及再次重启均 Idle；旧批准/上下文和采集同意不恢复。源码已独立只读复核；定向 `unified_assistance_crash`、`assistance_runtime_driver`、`session_assistance_work` **20 项输出通过**，包含新增 2 条实质父轨迹及 2 个普通运行 noop 子入口，退出 0。该测试不证明写事务中途断电。

详细命令、同步边界和未覆盖项见[实施补充](design/2026-10-19-unified-assistance-implementation.md)。当前仅增加原 U7 的合成故障验证，未新增功能、授权或算法。干净候选 `629c739` 随后实际运行完整 G-W，根 workspace 的并发密钥读取在 Windows 返回 os error 33，脚本正确停止，后续桌面/前端/打包未运行，见[失败记录](gates/20261019-629c739-win.md)。G-L 暂缓，系统 G-M 后置，真实效用样本仍为 0。U7 / Goal 1 / 正式 Goal 2 / Wave 1 均未关闭，未推送或合并。

## 核心闭环续建（2026-10-19，系统功能验收后置）

作者最新要求先构建程序核心，通知、托盘等具体系统功能后期再打磨；不再等待作者排查通知设置。Linux 门禁继续暂缓，不安装或卸载应用。此前 09:44 真实通知“没有看到”的未通过观察保留，不改为通过，也不据此冻结独立核心工作。

以 `b1147e3` 加现有 dirty 文档为起点，在原 U7 范围新增三条跨层组合验证，没有重做已实现模块，也没有新增生产命令、授权、数据迁移或算法规则：

- 核心新增两条：同一真实 Session/SQLCipher 库在请求发送前、以及真实 HTTP 执行后/结果接收前发生来源遗忘。直接经过已有存储遗忘 API，避免仅靠 Session 清空准备状态假通过；前者不新增请求，后者保留已发生的请求审计但拒收旧结果，均不增加 `draft.create` 接受记录。
- 遗忘来源后仍保留独立目标及自身 Memory；重启不恢复旧批准。用户明确清除失效来源关联后，同一目标可重新以自身 Memory 本地起草；最后明确遗忘目标才销毁该目标内容。来源遗忘是核心 API，未虚构为桌面公开命令。
- 桌面新增一条：通过生产注册 handler 完成真实导入、来源关联目标、成功起草控制、新未消费计划、公开记忆遗忘、旧批准拒绝及重启；重启后正文仍不可读，独立目标与导入来源仍保留。反馈和生成均不冒充任务完成，审计无正文。新场景放独立子模块，原大测试文件仅增加模块注册。

最终实跑：`cargo test -p soulcore --test unified_assistance_source_forget --test unified_assistance_flow --locked --offline` **4 通过**（新 2 / 既有 2）；独立 desktop `--test ipc_roundtrip --locked --offline` **68 通过**（新 1），均退出 0。开发反馈不重复累计；初轮因测试调用了不存在的 `Session.audit_chain` 编译退出 101，按真实 `Session.audit` 修正后重跑，不记为产品红绿修复。实际源码与增量均已独立只读复核，无阻塞发现；本轮未发现需修改生产逻辑的缺陷。

原始证据位于 `E:\Project\Soul-checks-20261019-win\core-continuation`。作者随后明确授权接管既有修改、复核后分阶段提交；本次独立只读复核无阻塞发现，重新实跑上述核心 4 项及独立 desktop IPC 68 项均退出 0，当前复验日志为 `E:\Project\Soul-checks-20261019-continuation\initial-core.log` / `initial-ipc.log`，对应 `*-exit.txt` 保存退出码。详细范围见[实施与验证补充](design/2026-10-19-unified-assistance-implementation.md)。测试及记录随本节纳入阶段提交；未运行新的完整 G-W，原 `237ef18` 的 G-W 不迁移给本轮候选。系统缺口和真实任务效用仍未关闭，不宣称完整 U7 / Goal 1 / Goal 2 / Wave 1 通过，未推送或合并。

## Windows G-M 与原生通知续验（2026-10-19，部分自动观察，已后置）

从干净文档 HEAD `b1147e3` 继续运行 `237ef18` 的现有 release exe，生产/构建源码未改变。未运行安装器，库与 WebView2 缓存分别隔离到仓库外测试目录；初始没有 Soul 进程，随后只操作测试 PID `31532`。真实 Windows 向导与页面可渲染，WebView2 `155.0.4283.45`；启动者未提权。窗口 × 隐藏后进程保留，第二次启动同一 exe 的新进程退出 0，原 PID / 窗口恢复同一事项；不冒称开始菜单或托盘菜单验收。

- 真实云开关五次点击后均为“尚未启用”。08:59:53.706–09:04:51.980（UTC−07:00）取得 Soul / 后代 WebView2 的 143 次 TCP / UDP 端点快照，未观察到非回环 TCP 或 UDP 端点；不覆盖样本间隙、UDP 包、启动首样本前或稍后通知阶段，未连续抓包。
- 真实页面创建合成目标，初始提醒关闭，明确设置未来时间后收到原生系统提交、展示与超时回调；打开未观察。明确请求撤除并重新读取后为“系统已接受撤除”。到期前 A→B 改期，在 A 后/B 前仍为 B 且仅显示上一轮历史投递，B 到期再出现当前轮系统回调。未注入模拟 OS，亦未取得可点击系统气泡窗口；气泡文字、肉眼展示、点击导航、视觉撤除与完整无重复观察仍待人工。
- 作者随后要求安排两分钟后的观察轮：已在同一合成事项实际提交本地 09:44:00（`16:44:00.712Z`）提醒。作者回报“没有看到通知”，本轮肉眼展示未通过，点击/导航未执行。事后重读确认本轮原生提交/展示/超时回调、打开未观察；服务运行中/错误无，两类别允许提醒。自动关闭/最小化输入后的收起状态仍未核实，不把输入返回成功记成隐藏通过。当前尚无 flags/ABI 或关闭路径的可证生产缺陷；已请作者只读回报通知总开关/勿扰和窗口状态，不重投、不改系统设置，根因待定位。
- 现有 headless `config` 退出 0，测试配置只有两个既有键；无 `--i-consent` 的 `collect-probe` 实际退出 1 并要求同意。带同意的两段采集测量未运行。作者随后明确回报已停止；主会话核实同一测试 PID、真实页面“没有在采集”且停止按钮禁用，主动刷新后仍为 8 条（09:37:20.271，UTC−07:00）。只确认本次停止及离散计数，不代填十次切换、1 秒撤销测量或完整 AC-09/10。
- 记录、原始证据和具体未覆盖项见 [G-M 补充记录](gates/20261019-237ef18-win.md)；[作者清单](../scripts/author-manual-checklist.md)已纠正旧历史 PASS，并补充第 11 节通知步骤。所有人工复选框保留未勾选。

**Windows 手动缺口：**托盘图标/tooltip/左右键与正常退出，标准用户与任务管理器提权列，150% DPI，资源监视器/系统连续流量，真机两段采集与撤销，通知肉眼展示/点击/视觉撤回，退出跨到期重开与真实休眠恢复。无作者实测回报，不记通过。NSIS / 安装卸载继续 NOT RUN，G-L 继续暂缓；完整 G-M、U7、Goal 1、正式 Goal 2 与 Wave 1 均未关闭。本轮仅修改验收文档，不自动提交或推送。

## Windows 独立桌面复核（2026-10-19，完整 G-W 执行通过）

起点为 `codex/soul-rebuild-20261019` 的 `b0e2334`，修复候选为本地提交 `237ef18f71de04b73abdc1a93baec50d87d1fc09`。业务与测试改动已完成独立只读复审；下列实跑均在 Windows、pwsh 7.6.5、Rust 1.83.0 / MSVC x64 下完成，仅使用合成资料、临时库和 loopback endpoint。该干净候选已完整执行 G-W，前后 HEAD / 空状态一致；完整记录见 [20261019-237ef18-win](gates/20261019-237ef18-win.md)。当前不是 GATED / INTEGRATED，Goal 1、正式 Goal 2 与 Wave 1 均未关闭。

- 首次独立 desktop `--all-targets --locked` 编译退出 101：通知 runtime 测试缺少 `soul-policy` / `tempfile` 的直接 dev 依赖。已补齐并删除冗余导入；桌面锁仅增加两个 dev 根依赖边和 `tempfile 3.27.0`，未升级已有包，完整 `metadata --locked --offline` 验证通过。
- 实跑发现公开“失败重试”也接受 Pending / Ready。先新增两个存储回归并实际观察到错误成功，再在原事务内、保留 CAS / Running 拒绝后限定 Failed。拒绝不改工作元数据或已完成 claim 回执；原 Session 测试改用真实记忆修改触发下一轮工作，保留未知结果、新旧待办和旧 revision 断言。
- 补齐两条 U5 成功预览 IPC：真实 Session / SQLCipher / owned work 生成模板，同一 Session 交给生产注册 handler；先成功预览，再分别关闭准备或修改语气，精确拒绝旧模板。核对正文占位、身份版本、元数据无正文及指定 loopback 模型端点零请求，不将其扩称所有 TCP / UDP 零出站。
- 修复 Tauri mock 重启夹具未释放管理状态的问题，仅在显式重启且所有调用返回后释放旧 Session，再正常打开同目录，不放宽库租约；修复旧静态托盘检查对 `build(context).run(callback)` 的误报，关闭拦截、隐藏及托盘前提断言保留。这些测试不证明生产窗口或真实通知交互。

| 本轮实际命令 | 结果 |
| --- | --- |
| `cargo test -p soul-store --lib retry_rejects_ --locked --offline`（修复前） | 0 通过 / 2 失败，退出 101，因非 Failed 工作错误返回成功 |
| `cargo test -p soul-store --lib assistance_work::tests --locked --offline` | 6 通过，退出 0 |
| `cargo test -p soulcore --test session_assistance_work --locked --offline` | 11 通过，退出 0 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline` | 7 个输出套件（含一个空套件），130 通过 / 0 失败；IPC 67 项包含新增 U5 两项，退出 0 |
| 根 / desktop `cargo fmt --all -- --check`、`git diff --check` | 均退出 0 |
| `pwsh -NoProfile -File scripts/gate-win.ps1`（候选 `237ef18`） | 完整退出 0，无跳过 workspace；根 1305 通过 / 1 项既有测量忽略，desktop 130 通过，前端 668 通过，生产包与两个 release 产物通过，manifest / smoke 通过 |

原始日志位于本机 `E:\Project\Soul-checks-20261019-win`：`desktop-initial.log`、`desktop-after-fixes.log`、`retry-red.log`、`retry-green.log`、`session-assistance-work.log`、最终 `desktop-verified.log`，退出码由各对应 `*-exit.txt` 或初轮 JSON 记录；中间失败不累计为通过。首次完整构建生成本机 OpenSSL 3.6.3 静态库，后续命令仅在子进程中通过 `OPENSSL_DIR` / `OPENSSL_NO_VENDOR=1` / `OPENSSL_STATIC=1` 复用该实际产物；根 / desktop 的 target 路径不变，已核对 build 输出的实际链接目录。此复用不证明 release 默认 vendored 冷构建路径。

历史 `ebff0c9` 的原始证据目录本机再次确认缺失，[计划索引](PLAN_INDEX.md)与[历史记录](gates/20260925-ebff0c9-win.md)已恢复 `NOT VERIFIED`。当前候选 G-W 的 15 项 smoke 检查通过，包含一次成功的 Windows TCP 表观察；headless 自身的 `observed=false` 不计网络观察通过。G-L 继续暂缓，安装 / 卸载未运行；当次 G-W 未执行 G-M、真实通知或任务效用验收，后续部分桌面续验见本页顶部。本次 G-W 结论只属于 `237ef18`，后续门禁文档提交不冒称为另一次新源码门禁运行。

## 统一辅助重建（2026-10-19，U1–U6 已实现，U7 部分验收）

工作区维护清理后从可取得的生产提交 `600f309` 和原计划 `64faa56` 继续，当前开发分支 `codex/soul-rebuild-20261019`。原计划已恢复为 `1433951`；此前未同步代码及其历史测试结果不能作为本轮交付。按[原计划 U1–U7](design/2026-10-16-unified-assistance.md)逐块调研、编写、独立复审和验证，工程选择从 D97 记录。

- U1/U2 已实现并完成独立只读复审（REVIEWED）：稳定事项、显式 Commitment 目标、固定观察来源、绝对时间、CAS 编辑/取消/自报完成、同事务 Memory 联动与遗忘范围、core 持有的事项草稿上下文。九个新桌面命令已接线，实际命令数 66；新增真实 handler 测试源码不代表原生运行通过。
- U1/U2 阶段验证：存储两包 97 项、受影响核心 9 套件 / 80 项、完整前端 47 文件 / 561 项通过；最后 Assistance effect 调整后定向 14 项复验通过，均不重复累计。当时 Rust 全三包检查因缓存产物故障中断，最终五包完整结果见下；各阶段命令、范围和失败修复见[实施与验证](design/2026-10-19-unified-assistance-implementation.md)。工具链为 Rust 1.83.0、pnpm 10.15.0，依赖锁未改。
- U3 已实现并完成独立只读源码复审：持久合并待办、精确 claim 完成回执、未知提交不重算、一次持有期恢复、严格来源读取错误、Graph/导入事务触发、collector 的真实 stop/flush/join 与同授权代次恢复。生产线程共用一个 Session，前台准备优先，普通读 busy 快返；新增 3 IPC 后共 69 个命令。定向核心 3 套件 / 28 项、存储 101 项、受影响核心回归 9 套件 / 94 项、完整前端 49 文件 / 585 项均通过，lint/production build 通过；具体失败修复与边界见实施文档。
- U4/U6 提醒与明确反馈已实现并完成独立源码复审（REVIEWED）：默认关闭的独立提醒 CAS、一次持久 dispatch、精确撤回代次与未知提交协调、隔离 Windows 通知 owner、类别静默/恢复及独立反馈。生产后台已接线，退出先停止所有 owner 再 join；实际命令数 77。核心定向 5 套件 / 68 项、存储 101 项、既有核心回归 94 项、完整前端 51 文件 / 630 项均通过，lint/production build 通过。合成原生传输和 Linux bridge 测试不代表 Windows 真机通知通过。
- U5 已实现并完成独立源码复审（REVIEWED）：默认关闭、明确目标逐项启用、真实 owner 语气模板、仅元数据持久化、资料/来源变更同事务撤销与重排、锁外 owned 预览及精确遗忘。关闭模板准备保留共享 U3 待办，避免丢失新的业务处理；准备与预览零 E1，不自动采用草稿。实际 IPC 总数 80。
- U7 当前环境可执行的集成检查已完成：真实 Session/SQLCipher 与生产 driver 覆盖导入→目标→准备→提醒回执→明确批准→草稿→反馈→完成→遗忘→重启，以及 100 项积压与前台材料优先。通知回执和 HTTP 为合成输入；真实任务样本数 0，未声明平台行为或帮助效果通过。
- 最终五包 `soulcore` / `soul-store` / `soul-store-api` / `soul-profile` / `soul-draft` 完整测试：92 个输出套件（含 7 个空套件），615 通过、0 失败、1 项既有 Windows 专项测量忽略。新增 U5 11 项和 U7 2 项包含其中；完整前端 53 文件 / 668 项、TypeScript、ESLint、production build 通过。根/桌面格式、schema-freeze、E0（16 crates / 317 源码文件）、禁词（94 项 / 145 文件）均通过；既有 6 项静态断言核对 80 个 IPC 名称、注册与调度，不代表原生 handler 执行。
- 作者已要求定期同步 GitHub：U1/U2 本地 `0c92886` 对应远端 `a8399e5`；U3 本地 `1c8fca9` 对应远端 `ba2b4f6`；U4/U6 本地 `293d1c6` 对应远端 `8ee2675`，均核对文件树完全相同，分支 `codex/soul-rebuild-20261019`。本提交收录 U5/U7 实现和最终结果，沿同一分支同步并核对 Git tree；未合并。
- 完整 G-L 继续按既有指示暂缓；当前环境无原生 GTK/WebKit 开发依赖，desktop 原生 IPC、Windows 真机通知与 G-W/G-M 未运行。Goal 1、正式 Goal 2 与 Wave 1 均未关闭。

## 导入后的来源选择连续性（2026-10-16）

起点 `760c269f4e0adcab2886bf4ffe30b5f4564166e4`。按 D68 对既有导入链路补最小缺陷修复；[具体方案与结果](design/2026-10-16-import-source-refresh.md)先获独立规划复核，实际源码和回归完成独立只读复审（REVIEWED）。包含本页的提交标识本轮源码，继续沿用 PR #66 草稿。

- 首次导入提交前，新来源尚未持久化；此前页面只在挂载或查询回执时读取列表，提交成功后仍不能直接选回新来源。现由共享回执变化刷新选项，切页期间完成的提交也能更新当前页面；不自动选择来源，后续准备仍需用户明确选定 UUID。
- 初读、提交后刷新及显式核对共用同一读取路径。挂载身份、回执身份和显式读取代次拒收迟到成功/失败；列表读取失败单独提示重读，不改变已提交回执、unknown 或工作槽，不重发导入。
- 先实跑新增反例，因提交后找不到新来源选项而失败；修复后完整前端 **44 文件 / 524 测试**、TypeScript、ESLint、production build 全部退出 0。新增 **7 项**真实 core wrapper + React / 合成 native 回归包含其中，首轮 6 项定向不重复累计。
- 首次全量验证因 PowerShell 启动遥测外联被自动审批中断，未计通过；按官方要求在进程启动环境禁用遥测后完整重跑通过。Rust/独立 desktop/原生 IPC、本轮完整 G-L 与 G-W/G-M 均未运行，不借用下方旧 Rust 数量。核心、57 个 IPC、算法、schema 与依赖锁未变；Goal 1、正式 Goal 2 与 Wave 1 未关闭。

## 单条导入消息来源核对（2026-10-16）

起点 `1b41f1dcc0df8ea4929ede94daa89fbc77352148`，已审规划检查点 `cc32eb29e6a76c32109c5e4b33d531344b9f2389`。D77 与[具体方案](design/2026-10-16-local-evidence-view.md)先完成独立规划审查，核心/IPC、存储与界面实现及后续修复均完成独立源码复审（REVIEWED）；相关实际检查已完成，包含本页的提交标识最终源码。继续同步 PR #66 草稿，未合并 main。

- 两步纯本地核对：先取得精确证据元数据，再明确选择一条消息；元数据不打开正文，正文只读选中项。每次按事件唯一索引核对真实 namespace / sender / 会话、当前 self↔peer 关系、Graph 用途和 CK，不能继承 D76 bundle 的来源或 E1 许可。
- 身份、account 和正文解封前检查真实密封行归属、nonce 与长度；大 identity 保持原导入/去重，只在本地查看不可用。owner 群消息拒绝，本人直接消息还需同关系、同真实来源/会话的有效 peer 发言；这是既有 actual-speaker 归属核验，不证明收件人。整次 evidence 行、refs 和唯一事件容量共享，不能每条重置。
- 单条 React 纯文本显示；关闭、来源、地址、选择或卸载清除并拒收迟到结果，旧 DOM 在微任务发起前也复验。不增加起草、端点、记忆、研究、导出用途或伪读取审计；清状态不宣称安全擦除 JS 内存。旧数据、遗忘或无法精确核对时拒绝，不猜另一来源。
- 最终 `soulcore` / `soul-import` / `soul-store` 三包全部目标 **67 套件 / 473 通过、0 失败、1 项既有 Windows 专项测量忽略**。包含新增真实 Session 15 项和存储 5 项；定向存储 5 项与单元 24 项不重复累计。完整前端 **43 文件 / 517 测试**、TypeScript、ESLint、production build 均通过，新增 UI 20 项和 wrapper 8 项包含其中。
- 根/桌面格式、实际 **57 命令**的 6 条静态断言、schema-freeze、E0（16 crates / 277 源码文件）、禁词（94 项 / 133 文件）及文档静态检查通过。测试初始 API/空 caption 假设、外部合成 URL 与缓存执行位问题均按实际失败修正并重跑，详见方案实际结果；未放宽断言、审计规则或冻结 schema。
- 原生 desktop/IPC 仍未构建或执行，新增 native handler 场景仅源码复核，静态命令检查不替代原生。完整 G-L 继续暂缓，G-W/G-M 未运行；依赖锁、持久 schema、T4D/A0–A3 未改。Goal 1、正式 Goal 2 与 Wave 1 未关闭，不标 GATED/INTEGRATED；真实模型效用仍未测。

## 洞察统计参与本次起草（2026-10-16）

起点 `f6972e8efa96aee65452c5b7e05a82c8c4c01f5a`，已审契约检查点 `7d4421e253e7493519b5762de12007e6898845bd`。D76 与[具体方案](design/2026-10-16-insight-draft-context.md)先完成独立规划审查，核心/IPC 与界面分别实现并经独立源码复核（REVIEWED），相关检查已实际完成；包含本页的提交标识最终源码。继续阶段同步 PR #66 草稿，未合并 main。

- 选择有效 bundle 的一个联系人，核心生成完整可信 A2 统计和准确引用，使用独立一次上下文，与手写 memory 互斥。引用是统计材料出处，不能给整篇生成草稿逐句事实背书；统计描述全库当前有效关系，不限于触发它的那次导入。
- 默认只供本地核对，本机语气模板不变。另行选择 includeCounts 才把匿名派生计数纳入既有 E1 最终预览与单次批准，不从 UI 接收统计文本，不把原事件或证据改为 Owner，不向页面/端点交付来源原文。paste 豁免只交给 paste。
- exact bundle 读取不受 recent 窗口限制，准备/派发/接收均复验 W5 完整快照。忽略、Session 重算、遗忘和替换清除旧用途；直接 store 新建 bundle 本身不自动撤旧行。已发 HTTP 先审计再拒收失效回复，保持 D69 余量边界；重启不恢复上下文或批准。
- 单项统计按实际 final JSON serializer 的既有 64 KiB 预算完整保留或整体拒绝，不截掉部分 points/引用。界面只持 UUID 选择意图，切换走 shared discard drain，旧 DOM/预览/批准受来源、选择、paste 和挂载代次约束。忽略/重算后的新读取重新绑定来源，外部变更仍拒收旧结果。
- 新增三个独立 IPC，实际命令表 **55 项**。最终 `soulcore` / `soul-import` 两包全部目标 **52 套件 / 370 通过、0 失败、1 项既有 Windows 专项测量忽略**；完整前端 **41 文件 / 489 测试**、TypeScript、ESLint、production build 通过。10 项新真实 Session/MockHTTP 用例包含在上述统计中，定向 42 项与单元 24 项不重复累计。
- 根/桌面格式、55 命令的 6 条静态断言、schema-freeze、E0（16 crates / 273 源码文件）、禁词与文档静态检查通过。最终全部目标首轮因一个缓存 ELF 无执行位中断，修复构建缓存权限后以同命令重跑退出 0；没有改测试断言或产品源码绕过。冻结 schema、算法与依赖锁未变。
- 原生 desktop/IPC 仍未构建或执行，新增 mock handler 调用源码不替代它。完整 G-L 继续暂缓，G-W/G-M 未运行；原文受控解引用与真实模型效用未在本轮交付。Goal 1、正式 Goal 2 与 Wave 1 未关闭，不标 GATED/INTEGRATED。下方 360 / 463 等属于父检查点，不能覆盖本节新结果。

## 工作流连续性补齐（2026-10-16）

起点 `413790daaadeab933879406b8d6c01c94c5053b9`。本轮三个模块及复审发现的导入审计遗漏均先具体规划、独立核验，再实现与源码复审（REVIEWED）；包含本页的提交记录最终版本。开发线继续为 PR #66 草稿，未合并 main。

- 洞察可定位到唯一当前关系和指定证据元数据，显示已有标识提示并聚焦标题。未知、遗忘、缺失或歧义目标不落到另一人；64 个唯一引用、8192 字符的导航上限不裁剪整份依据，超出时明确提示并提供普通图谱。不会自动请求摘要、解密材料或发起纠正。
- 地址、来源及挂载时序均核对；普通初读中断后可以明确重读，旧 DOM 按钮不能越过地址切换发起纠正或摘要，旧写入结束后新页面仍可正常操作。已执行写入的反馈与旧证据定位失效分开表达，不能把失效显示成写入未发生。
- 新增真实 Session / SQLCipher 组合回归：不可变导入及幂等回执、去重、本地洞察 ignore/correct/recompute、手写记忆本次上下文、本地起草和单次 Owner 节选端点起草、遗忘与重启。核对实际 wire、计数和无正文审计，不用旧 AC-21 低层轨迹冒充新 Session 链路；联系人的遗忘沿既有 store 领域接口，并非新增 Session/IPC 功能。
- 现代准备路径恢复 `injection.blocked` 聚合尝试审计。真实 RED 已复现缺行；owned worker 在实际解析且两层预算通过后、身份/索引预检及 reserved 之前记录非零计数。迟到拒收或完成对象丢弃不抹掉已经读取的尝试；未执行、解析失败、预算拒绝与零标记不造记录，确认导入领域事务未改。
- 原生 IPC 测试源码迁移到现代准备协议，移除正向用例任意拒绝即返回的假通过，补新增命令真实调用、有效身份的错误参数对照及旧入口固定拒绝。原生测试尚未编译或运行，不能把源码修复、格式或静态断言当作原生通过。
- 最终 `soulcore` / `soul-import` 两包全部目标 **51 套件 / 360 通过、0 失败、1 项既有 Windows 专项测量忽略**；完整前端 **39 文件 / 463 测试**、TypeScript、ESLint 与 production build 通过。根/桌面格式、6 条命令静态断言、schema-freeze、E0（16 crates / 268 源码文件）与禁词检查通过。实际命令和范围见[本轮计划与验证](design/2026-10-16-workflow-continuity.md)；下方 720 / 437 是父检查点的历史验证，不迁移为新源码的结果。
- 定位只核对当前元数据存在，不认证旧洞察陈述或完整快照仍最新；导入统计尚不能直接选入起草上下文。完整 G-L 继续暂缓；缺原生 GTK/WebKit 开发依赖，原生桌面/IPC、G-W/G-M 未运行。Goal 1、正式 Goal 2 与 Wave 1 未关闭，不标 GATED/INTEGRATED。

## 剩余框架续写（2026-10-16，D71–D75）

起点 `b66f093a1407887eb873fe33defe984906252d29`，扫描/选择库检查点 `a3bd7c709bf7f95c15b3145e32d7317631c1f06e`。作者要求完成现有计划的剩余本地用例；W1–W5 均已逐模块规划、独立复核、实现并完成源码复审（REVIEWED），本轮相关回归完成。开发线为 PR #66 草稿，未合并 main；包含本页的提交记录最终源码版本。

- W1 扫描采用 Session 内登记、锁外只读扫描、归还唯一 file-plan ledger、审计及来源校验；与 E1/导入共用物理慢工作槽。来源修改不提前释放在途槽，生产 Tauri worker 持有完整接收处理责任；页面拒收旧输入、旧挂载与旧来源结果。
- W2 接入明确候选 UUID 内的关键词/类型选择，缺事实有效时间返回 Clarify；默认仅本地参考。手写记忆与实际密封 blob 组成出处，Owner 另行勾选才进入现有 E1 批准，paste 豁免不扩大到 memory。按真实最终 JSON serializer 的 64 KiB 字节预算整条裁剪，界面引用、Owner 项、body preview 与实际保留材料一致；编辑、遗忘、用途消费及迟到结果均复验。
- W3/W4 使用核心持有的不可变准备、ID-only 确认、OS 持有期 lease、持久固定状态/计数回执与显式查询恢复。领域、图谱、原导入审计、成功回执同事务；长导入期间其它 Session store 入口立即拒绝，清除端点/丢弃批准/撤回仍可达，延迟撤回审计先于再授权。schema 3 原子迁移；外部自然键封存在现有 CK 下，同键相同跳过、内容或元数据冲突整次拒绝。旧历史缺 ID 不猜，明确 forward-only 后持续显示历史可能重叠。遗忘同事务删除关联索引并撤销 namespace，历史计数不恢复旧内容或批准。
- W5 导入提交事务内生成现有 A2 的本地候选，不调用端点、不新增评分。持久依赖包含 UUID、数值、固定状态及既有 conversation_ref 标识的规范编码，不存摘要正文；读取/重启复验实际来源。ignore 只影响指定 bundle，纠正沿用既有用户锁，recompute 创建新 UUID；来源缺失/遗忘/撤销时拒绝重算。
- 新增命令采用薄转发或精确的 begin/execute/complete 调度，实际命令表为 **52 项**；原四个同步导入 IPC 入口保留名称但固定拒绝，要求改用不可变准备。JSON schema、依赖锁、T4D/A0–A3 不变，内部数据库 schema 从 2 升为 3。静态命令表/转发断言不是原生构建或 IPC 执行。
- Linux / pwsh 7.4.6 / Rust 1.83.0 实际检查：九个相关 Rust 包 `--all-targets --locked --offline` 共 **103 套件 / 720 项通过、0 失败、1 项既有 Windows 专项测量忽略**；完整前端 **37 文件 / 437 测试**通过，TypeScript、ESLint、production build 通过。根与桌面格式检查、52 命令的 6 项静态源码断言、schema-freeze、E0 审计（16 shipped crates / 265 源码文件）及禁词审计均通过。未把矩阵分支或重复运行累计为额外测试。
- 完整 G-L 继续按既有指示暂缓；当前缺 pkg-config、GTK/WebKit 开发依赖，原生桌面构建/IPC 未运行，G-W/G-M 未执行。不标 GATED/INTEGRATED，不关闭 Goal 1、正式 Goal 2 或 Wave 1；Android、多端同步、自动代发、文件写执行与真实模型效用实验仍属独立范围。

见 [剩余模块计划与实际结果](design/2026-10-16-remaining-implementation.md)。下列 D70 内容是此前切片的验证记录。

## 导入交付与人物摘要执行（2026-10-16，D70）

起点为 PR #66 的 `c1d4f8fc82f9535e12c9e9a07823df1599581432`。按作者最新框架要求，两模块均先具体规划、独立复核方案，再编码与只读 diff 复审；状态为 REVIEWED，未标 GATED/INTEGRATED。Goal 1、正式 Goal 2 与 Wave 1 未关闭。

- 导入页在同一 WebView 的路由挂载之间共享真实提交槽。切页不释放正在执行的提交，成功回执可在返回后查看；只保留计数和格式，不缓存原文/文件名。结构化核心拒绝可显式重试；非结构化失败或畸形回执进入 unknown，暂停继续导入，不伪称回滚或自动重发。重启不证明之前未提交。
- 回执提供图谱/证据与审计入口；关系数准确描述为本轮按有效证据重算写回量，包含已有关系的重算，不伪称新增或图中现存总量。D55 重复导入语义不变。
- 人物摘要沿原点击触发与统计回退路径，改为 Session 内准备、锁外 HTTP/解析、Session 内审计和来源校验。与起草共享一个真实 E1 worker 槽；失效不提前释放槽，counts 无端点时不占网络槽。资料/端点变化后拒收旧结果，WebView 补充交付微任务校验并清除已显示摘要。
- 完整前端 **30 文件 / 428 测试**、TypeScript、ESLint、production build 通过；Rust 1.83 / SQLCipher **15 套件 / 162 测试**全部通过；5 文件定向 Rust 格式及 6 项原壳命令静态断言通过；E0 审计扫描 16 个 shipped crate / 243 个源码文件，clean。16 类来源变化是一个矩阵测试的分支，不重复累计。
- 未修改持久化 schema、依赖锁、算法、39 个 IPC 命令的名称/参数/返回形状；person_summary 壳调度改为 async。原生桌面构建/IPC 未运行，缺 pkg-config 与 GTK/WebKit 开发依赖；静态断言不替代它们。完整 G-L 继续暂缓，G-W/G-M 未补跑。
- 仍未实现消息去重、持久回执、崩溃任务恢复、自动洞察与反馈学习、记忆用途/自动检索；扫描和导入长锁仍待迁移。生产 Tauri 闭包负责完整请求审计，不扩称任意底层调用方或 transport 内部 panic 均有相同保证。

详见 [模块计划、反例和实际验证](design/2026-10-16-import-summary-continuation.md)。同步沿用当前草稿分支，未合并 main。下方起草专用记录是上一切片范围，不覆盖本节新增摘要执行能力。

## 生产框架接入（2026-10-16，起草切片）

按使用框架完成前两步生产调整，基于 PR #66 工作分支的 `258c0162ca28c44c42dcb3f1c448e01a5bc5c7b6`。已独立逻辑复审；不是 GATED / INTEGRATED，Goal 1、正式 Goal 2 与 Wave 1 均未关闭。

- 起草页已接同步准入、输入/生命周期归属、预览和原文豁免绑定、跨挂载任务槽与 discard drain。资料变更期间拒绝新起草，成功 IPC 交付前校验代次，清除已显示旧结果；未知端点结果不自动重试。
- 生产起草采用 Session 内准备与发许可、锁外 HTTP/解析、Session 内审计与接收校验。Tauri worker 持 Arc，共享同一 Session/加密库，IPC 等待方消失仍完成审计。来源变化和撤回使旧结果失效，不提前释放在途槽。
- 撤回边界与两分钟单调期限见 [D69](DECISIONS.md)。已发许可可能继续出网；只迁移了起草 HTTP，摘要/扫描/导入仍可能占锁。记忆用途与自动检索、D55、迁移/遗忘重导未接入。
- 最终前端：28 文件 / 405 测试、TypeScript/ESLint、生产 build 均通过。含 6 项真实 core.ts + React 联合时序回归。HTML 13 项离线交互/下载/手机宽度检查通过。
- Rust 1.83.0：soulcore 定向 10 个 suite / 119 项全部通过（含新工作生命周期与期限的 14 项测试，两个来源矩阵各遍历 14 类修改）；7 个改动 Rust 文件格式检查通过；E0 审计扫描 16 个 shipped crate / 240 个源码文件，clean。
- schema、依赖锁、T4D/A0–A3 未改；原 39 个 IPC 命令名/参数/返回 schema 不变，起草的 Rust 壳函数改为 async。桌面原生构建与 IPC 未运行：当前 Linux 无 pkg-config / GTK / WebKit 开发依赖。完整 G-L 继续暂缓，NSIS / G-M 跳过不等于豁免；本环境 Bash 诊断不能替代仓库的 pwsh 平台门禁。

最新 [HTML 框架页](framework-design.html)、[生产起草流程图](design/draft-runtime-flow.svg)、[实现与验证](design/2026-10-16-draft-implementation.md)。代码/设计同步当前工作分支，PR 保持草稿，未合并 main。以下记录为历史，不能用其中“尚未接入起草”覆盖本节。

## 后续设计更新（2026-10-15，使用闭环与参考算法）

基于 PR #66 的 `6e8f3c06b7fd0cbc7920766337572301319b021a`，完成使用场景/源码差距、8 项一手研究依据、4 种路线比较、准备/执行/接收框架，以及页面归属、任务控制、证据选择、纠正优先与未知结果恢复设计。独立评审的槽位回收、字段类型、冲突范围及 origin 反例已修正。

新增纯 JS 非生产参考算法，37/37 合成测试（含一个枚举 120 种事件顺序的用例）、6/6 定向变异检出、13/13 离线 HTML 检查。没有改生产 Rust/React/IPC/schema/锁文件，未把模型结果视作原生/平台验证。研究、图示与实验见 [交互 HTML](framework-design.html)、[设计](design/2026-10-15-usage-framework.md)和[验证记录](design/2026-10-15-framework-validation.md)。用户既有 GitHub 同步授权继续用于当前草稿分支，未合并 main。

**下方生产修复与其测试仍是前一阶段证据。** 独立撤权与记忆参与起草尚未接入；取消、用途许可、迁移/遗忘重导仍需明确决定。Goal 1、正式 Goal 2 与 Wave 1 均未关闭。

## 当前状态（2026-10-15，架构改进与真实定向验证）

**Goal 1、正式 Goal 2 与 Wave 1 均未关闭。** 本轮状态为已实现并独立复审，附有局部测试证据；不标记 GATED 或 INTEGRATED。

源码基于 PR #66 / `codex/soul-forget-outcomes-20260929` 的 `9d3cd300c19c996f9edabf337c9d359fcb594bc3`。GitHub main 核对为 `8aae8f3877bc93529d0a0104c277a85ded9c43aa`。用户于 2026-10-15 明确要求将本轮代码及架构设计图同步 GitHub；交付落在该工作分支，PR 保持草稿，未合入 main。提交身份以包含本页的 Git 历史为准。

- 遗忘预览增加不可由 IPC 构造的精确范围记录，确认时在同一 store guard 下比较主键集合与影响统计，再执行原有销毁和 cleanup/audit 流程。影响扩大或等数量对象替换在销毁前拒绝；拒绝审计前释放 guard，防止递归锁死。此保证限定于现有单 writer 模型，不是跨连接授权协议。
- Memory 写入获得成功确认且编辑代次仍相同时才清空输入；拒绝、未知结果、新编辑和卸载不被旧回调清空。请求协调对象初始化通过真实 lint。
- Import 清空文件选择使旧 read/preview 结果失效，保留 commit 期间的保护；修复了完整前端测试中复现的 4 个既有失败。
- Linux 实际通过：前端 23 文件 / 338 测试、完整 TypeScript/ESLint、生产 build；Rust 1.83.0 存储 37 项 + soulcore 28 项，共 65 项；7 个改动 Rust 文件定向格式检查。独立只读复审未发现阻塞问题。环境使用 Bash，不能当作仓库要求的 PowerShell 平台门禁。
- schema、算法、依赖锁、IPC 命令签名与注册未改。Session 长任务占锁与独立撤权仍是 P0，已给出准备／锁外执行／结果校验的设计，尚未实施。D55 幂等与迁移/重导语义保持原决议。

图示入口见 [系统架构与设计图](ARCHITECTURE.md)，详细设计见 [架构报告](reviews/2026-10-15-architecture-review.md)，命令、退出码、失败修复与边界见 [验证记录](reviews/2026-10-15-validation.md)。后续按报告迁移一个起草用例并验证独立撤权，再扩展到其他长任务。

未运行整个 Rust workspace、独立桌面 workspace、原生 IPC、G-W 或 G-M；完整 G-L 仍按原指示暂缓，NSIS 安装/卸载与 G-M 跳过不等于豁免。以下 2026-09-29 记录保留为历史，不覆盖上述新证据。

## 历史状态（2026-09-29，阶段 5：桌面接线）

**Goal 1、正式 Goal 2 与 Wave 1 均未关闭。** 本轮把遗忘结果接到桌面源码并完成局部诊断，不是原生端到端通过、独立评审通过或发布声明。

工作分支：`codex/soul-forget-outcomes-20260929`，PR #66 保持草稿。直接父提交 `631e9ed16fcb7426cbd1f7e8df2132a6efefc211`。本轮核对 main 仍为 `8aae8f3877bc93529d0a0104c277a85ded9c43aa`，未修改或合并 main。前轮分支起点仍是 `codex/soul-followup-20260928` @ `3cd0c73`。

## 本轮完成的源码对齐

- 核心 `commands::memory::forget` 改用 `forget_with_outcome`，保留原 unit/impact 访问，增加清理与审计确认；已遗忘记忆不再次执行销毁。既有 Session 调用通过这个桥接进入新服务，没有修改 Session 的整体锁模型。
- `ForgetReceiptView` 增加 `logical_committed`、`cleanup`、`audit`。新 `retry_forget_cleanup` 命令复用 Session 的同一个加密库连接，不接收记忆 id 或销毁确认，不执行销毁或补写审计。
- TypeScript 包装、原生命令声明、原生命令名表、Tauri 注册表静态核对为同一组 **39 个命令**。原有命令未删除，新增一个清理命令。
- `Memory.tsx` 接入三种独立结果和“仅重试日志清理”按钮；重试保留原回执数字及审计不确定性。重启后已有墓碑也可发起清理，但不重建历史回执，不把墓碑当清理／审计确认。
- 页面请求在 IPC 前同步准入，卸载前后与旧预览回调不得重入；在实际启动 IPC 的微任务内再次检查归属。只有明确的预览编号不匹配保留确认面板；未知 IPC 结果不冒称回滚，不自动重发销毁。
- 旧版回执缺少新字段时不默认成功。既有 `fakeCore` 保留为旧形状测试材料；新增组件测试分别覆盖当前与旧形状。没有修改旧回归来掩盖不兼容。
- 原 `FORGET_NOTICE` 常量及其旧测试双胞胎未改；页面另行明确其永久不可读描述以日志清理确认为前提。旧底层遗忘 API 保留供旧调用者使用，本轮并非迁移所有旧 API 调用方。

## 本轮实际检查（不是完整门禁）

环境为 Linux 外部部分工作副本，Node v22.16.0、TypeScript 5.8.3。没有 cargo、rustc、pnpm、pwsh 或完整项目 React/Vitest 依赖；原始下载域名访问失败。诊断未使用仓库要求的 PowerShell，不能当作 G-L/G-W。

| 检查 | 实际结果 | 证据边界 |
| --- | --- | --- |
| `node /mnt/data/soul-checks/run-pure.cjs` | 退出 0，17/17 | 新 `memoryRequests.test.ts` 只替换 runner 导入为 node:test；断言正文不改；不是 Vitest |
| `node /mnt/data/soul-checks/probe-memory-ui.cjs` | 退出 0，12/12 | 实际 Memory 源码与 core.ts 包装，hook/JSX 和 invoke 替身；不是 React、DOM 或 Tauri 原生执行 |
| `node /mnt/data/soul-checks/check_sources.cjs` | 退出 0 | 7 个 TS/TSX 单文件转译无语法诊断；四处命令表一致、39 项；原生命令仍为单行转发 |
| `tsc --noEmit --strict --noUnusedLocals --noUnusedParameters --exactOptionalPropertyTypes --noUncheckedIndexedAccess --target ES2022` | 退出 0 | 仅 memoryRequests.ts 和 forgetOutcome.ts 两个独立模块 |
| Git blob 字节核对 | 通过 | 五个修改前完整文件匹配来源 blob；提交源码匹配工作副本；不是编译正确性证明 |

新增 **10 个 React/Vitest 用例、4 个 Rust 用例已编写但未运行**。前轮 14 个 Rust 用例也仍未运行。前轮 SQLite 6/6 和呈现断言 14/14 属于前轮局部证据，本轮没有重新计为 PASS。

没有运行完整应用类型检查、lint/build、旧回归、真实组件、Rust 格式／编译／测试、桌面 IPC、独立只读评审或平台门禁。受控地重复旧回调不等于真实浏览器正常双击复现；页面结果忽略不等于取消或回滚核心操作。

## 未闭环与后续顺序

1. **遗忘预览的影响面变化仍是 P0。** 当前 Session 校验预览编号／对象后执行，matched_preview 在返回回执时比较；本轮没有把影响面比较移动到销毁前，不能宣称阻止了预览后内容变化导致的超范围销毁。需在同一存储锁保护下重新比较再执行，并增加真实事务回归。
2. **独立撤权仍是 P0。** Session 长任务的准备／锁外执行／提交、撤权优先路径、取消与来源变化后的提交拒绝尚未实现。新清理命令仍受现有锁模型约束。
3. 补真实 Rust／SQLCipher、现有回归、React／Vitest、完整构建与原生 IPC 测试，再做独立评审。源码已接线不等于产品已验收。
4. 继续按原许可处理 G-L/G-W/G-M 与主干集成。G-L 暂缓，NSIS 安装／卸载和 G-M 跳过；没有据此勾选或免除任何关闭条件。

## 整体范围

已有 v0.1 路径包括问卷与档案纠正、导入、图谱、记忆 CRUD、Windows 前台时长采集、起草、授权目录只读计划、审计与内存研究预览；各自当前完整验证状态不得从旧版本绿迁移。

获准记忆自动检索参与起草、消息级幂等、行为预测、恢复包与受控文件执行、多设备／Android、可选云深度分析仍按 [ROADMAP](ROADMAP.md) 和 [Goal 2 计划](GOAL2_PLAN.md) 推进，不算本轮完成。

冻结 schema、算法、依赖锁文件、权限和采集能力没有扩展；未恢复托管 CI。正式关闭仍以 [Wave 1 计划](SOUL_WAVE1_IMPLEMENTATION_PLAN_2026-09-27.md)、[ACCEPTANCE](ACCEPTANCE.md)、[gates/README](gates/README.md) 与[作者清单](../scripts/author-manual-checklist.md) 为准。
