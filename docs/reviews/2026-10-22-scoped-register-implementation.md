# D109 精确范围语域偏好独立评审

日期：2026-10-22，America/Los_Angeles。评审者：独立 `scoped_review`（`soul_reviewer`，gpt-6.1-sol/high）；主线程按其只读报告落盘，实际运行验收另见[实施回执](../design/2026-10-22-core-framework-v1.3-execution.md#d109-实施回执2026-10-22)。版本：`codex/soul-rebuild-20261019` / `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46` 加本次 dirty 修改，不以 HEAD 冒充完整候选。

## 结论与范围

**独立静态评审无剩余阻塞项。** 本结论覆盖 D109 的数据、Session、Tauri 接线、React 生命周期和必要上下游；不签运行验收、合入批准或 G-L/G-W/G-M。评审先读 AGENTS、STATUS、PLAN_INDEX、D109、任务范围和实际 diff/新增源码，再看实现说明与原始运行记录。评审者未运行 cargo/pnpm、未写实现、未派生 agent、未读取真实库或凭据。主线程拥有所有构建与集成通道。

## 发现、反例与修复

| 发现 | 影响与反例 | 修复与定向复审 |
| --- | --- | --- |
| 五个新命令遗漏于 `COMMAND_NAMES` | 实际 handler 已注册使 IPC 能通过，但既有 command_surface 双向清单一致性检查会失败 | [commands.rs](../../apps/desktop/src-tauri/src/commands.rs) 补齐五名，handler/core.ts/清单一致；评审复核源码，主线程随后 command_surface 6 项通过 |
| confirm 回执晚到时忽略额外来源变更 | confirm 已提交但回执未到，普通 Memory 编辑使规则失活，旧 UI 仍无条件承诺未来使用。`ui-race-red.log` 7 通过/1 失败，退出 1，命中缺少复核提示 | [DraftScopedVoice.tsx](../../apps/desktop/src/routes/DraftScopedVoice.tsx) 的回执只陈述历史提交；额外 source 通知提示重新起草核对，不推断一定失活，也不丢成功回执。源码与反例一致 |
| 已撤销的旧提案被标为未知 IPC | pending proposal → updateMemory → late proposal → targeted discard 已获回执，`DraftSourceChangedError` 却使共享 lane 不可再起草。`ui-source-red.log` 8 通过/1 失败，退出 1 | Scoped local/propose 排除已知来源失效；[core.ts](../../apps/desktop/src/core.ts) 对真正取消失败使用固定普通 Error，不能把结构化拒绝冒充撤销成功。最终分类修复已定向复审 |

没有另提范围外产品扩展。后续主线程新增取消请求失败的第十项 UI 反例及运行修正，记录在实施回执；不将主线程补测伪装成评审者亲自运行。

## 核验的关键边界

- [存储](../../crates/soul-store/src/scoped_preferences.rs) 精确匹配 owner/人物/场合，并验证有效人物和可解密的 owner Preference。旧普通 Preference 不解析为规则。schema 8 沿现有更高版本拒绝机制，没有新 hash/冻结 contract/gate。
- `confirm_scoped_register` 自持事务，检查原 expected revision，再写 Memory/审计和激活关联。普通 `put_memory` 在保存点内失活并更新 revision；禁止改 type/CK 逃逸。真实审计触发器失败测试已静态核对其快照和一次提案消费断言。
- [遗忘](../../crates/soul-store/src/forget.rs) 的精确预览包含关联原始行；Memory、CK、联系人（包括无姓名）销毁路径同事务清 register。遗忘后新确认使用新 Memory/CK；同数量替换不会沿用旧预览。
- [Session](../../crates/soulcore/src/commands/session_scoped_preferences.rs) 同一 store guard 覆盖读取、应用、生成与审计，只改新 brief 副本，不改全局 voice/锁。普通、端点、Memory、Insight 入口清 scoped 持有状态；确认一次消费，重启不恢复提案。
- 用户显式选择现有三种语域，不上传编辑正文；只密封固定本人确认说明，审计无草稿/编辑正文。普通本地模板链没有新增派发或出网入口；其他模式不声称已应用。
- UI 修改/取消同步撤销 ownership，等待撤销回执后再应用编辑，晚到提案定向清理，确认中锁编辑/范围。普通 Memory 编辑处已显示停用说明。

## 证据与未覆盖

本机证据目录：`E:/SoulSyntheticLab/scoped-register-implementation-20261022/`。评审者亲读 `data-green.log`（10）、`session-green.log`（3）、`ipc-red.log`→`ipc-green.log`（未注册失败→2 通过）、`core-regression.log`（20+3+5+5）、`session-context-regression.log`（11+10+4）、`ui-green.log`（22）、两份 UI RED 和首次 lint 失败记录。主线程统一运行表提供命令/退出码/适用候选，不能只凭绿色 summary。711 项全前端回归是在最后来源错误分类修改之前运行；最终局部复验单独记，不改写原覆盖。

评审自行执行的仅为 pwsh Git 状态/diff、UTF-8 读取与定向 rg；末次 `git diff --check` 退出 0，含 PRODUCT_LOCK 换行提示。两次取证退出 1：D109 标题正则无匹配、猜测测试路径不存在；后续已按真实表格/文件定位。非命令源码审查完成，无虚构退出码。代理返回报告及上述原始日志为证据；评审工具读取过程未另保存完整逐条终端副本，故不声称具备全量审查命令日志。

真实 Windows 页面操作、剪贴板、窗口生命周期、安装、真人效用和正式平台门禁未由本评审验证。源代码评审、合成 SQLCipher、mock IPC 和 jsdom 不能替代这些观察。主线程负责最终运行结果、文档状态与未完成项。

## 后续一：当前偏好核对与精确记忆导航（2026-10-22）

**独立静态评审完成，无剩余静态阻塞。** 评审仍为 `scoped_review`（soul_reviewer，gpt-6.1-sol/high），由主线程据其最终只读报告落盘。版本为同分支、同 HEAD 加本轮 dirty 增量；以本轮 `initial-candidate.patch` 和上一片最终源码区分已有修改，没有重审未变动的全部存储/遗忘实现。范围和实际运行验收见[后续实施回执](../design/2026-10-22-core-framework-v1.3-execution.md#后续一核对当前偏好与定位记忆2026-10-22)。

| 发现与归因 | 具体反例 | 最小修复与复核 |
| --- | --- | --- |
| 新增同页导航后，旧“用于本次起草”按钮仍可能选入 A | A 已打开，同批事件将地址改为 B，再点击尚未被 React 移除的旧按钮 | `ownsOpen()` 同时检查地址与来源，候选/保存按钮均使用；同批点击测试覆盖。实现者也在本轮独立静态检查发现该点 |
| 保存入口与 IPC 派发间的来源变化窗口；原 Memory 已有该缺口 | 点击保存 A，派发微任务前更新 B；实际请求为 B、A，旧 A 仍被发出 | `invokeOwned` 在真实派发前复核捕获来源；保留已发操作真实结束，不伪称取消 |
| 旧写回执被标成当前内容；原 Memory 已有晚到交付缺口 | A 的首笔写入回执延迟，第二笔 A 修改已完成，再收到首笔旧正文 | 额外来源变化后清除可编辑旧详情，保留“写入已收到核心回执”的历史事实；不误报回滚，不丢物理 finish |
| 只读最终投递错误复用写入自身通知额度；本轮交付逻辑问题 | 内层读取 await 检查后，外层交付前收到两次来源通知，旧正文复活 | 只读模式在最终投递再次比较原 source；写入保留自身 start/finish 语义 |
| 新链接遇起草离页清理时缺少恢复入口 | 真实 App 路由中首次读取因来源变化拒绝，detail 0 次，清理后无精确目标重读按钮 | 用户点击“重新读取此地址”才重读当前目标；没有自动重试、保存或遗忘 |

两项 in-flight 导航测试最初的额外读取经诊断是操作前已有 `[A,A]`，来自测试初始化 hash 重复事件，不是提前读取 B。修正事件时序后仍严格断言 pending 时 `[A]`、真实结束后 `[A,B]`，没有放宽次数或跳过断言。所有最终源码修复均定向复核；相同覆盖面未追加固定轮数全库扫描。

本轮核验：五态接口固定 owner 与精确人物/场合，读取无审计写入且不消费提案；absent 仅凭确切缺席，forgotten 要求销毁记录/墓碑/inactive/空 register 一致，缺失或损坏不冒充遗忘，底层错误不泄漏内容。active 复用既有有效性验证，其他状态不返回旧语域。前端查询为手动快照；scope/source/address/lifetime 变化拒收旧响应。UUID 深链拒绝非法、重复及未知参数，响应 ID 不符不展示正文。同页导航复用唯一 MemoryRequests，保留已发写入的物理占用，真实完成后再处理新地址。

评审亲读的本机日志位于 `E:/SoulSyntheticLab/scoped-register-inspection-20261022/`：`data-status-red.log`、`data-status-green.log`（11）、`ipc-status-green.log`（1，75 filtered）、`core-regression.log`、`desktop-regression.log`、`ui-status-green.log`、`ui-navigation-diagnostic.log`、`ui-status-green2.log`（71，后续修复前）、`ui-delivery-red.log`、`ui-link-recovery-red.log`、`ui-read-delivery-red.log`。最后一份报告交付时全前端回归仍在运行；最终运行结果由主线程记录在实施回执，不伪称评审者亲跑或亲读尚未完成的输出。

评审者只执行定向 Git/UTF-8 读取/rg，末次分支与 HEAD 一致、`git diff --check` 退出 0，保留已有 PRODUCT_LOCK 换行警告。非命令源码评审状态为完成，无虚构退出码；未写实现、未运行构建、未修改暂存区或配置。没有新增 schema、依赖、出网或扩大保存/遗忘语义。真实 Windows 手工交互、安装、真人效用及 G-L/G-W/G-M 仍未验证。

收口补充：最终全前端首次为 743 通过/1 失败，唯一旧 Graph 路由测试仍要求 Memory 查询回首页。主线程只更新该测试为 Memory 路由并显式断言 `memoryTarget("#/memory?contact=x") === "invalid"`；评审者随后仅定向核对这项测试及两文件 21 项通过日志，确认未放松非法参数拒绝、其余未扩展路径仍回首页。未重复扫描相同生产实现。

## 后续二：Windows 合成交互初段证据核对

由 `scoped_review` 独立只读检查本机 `E:/SoulSyntheticLab/scoped-register-windows-20261023/`，同分支/HEAD 加 dirty，主线程据实际报告落盘。没有修改实现或操作窗口。已核对当前 release build 退出 0、构建期间 32 个源码文件未变、第二次启动 PID 948/HWND 67492 及 6 个 WebView 合成缓存路径匹配。首次启动没有最终退出回执，原因未知，不能将启动 JSON 等同于外层脚本完成。检查器接受根目录或其精确 EBWebView 子目录，仍保留 PID/启动时间/可执行路径和后代归属检查。

逐步证据支持：033 导入 committed 新增 5；054 提案→056 取消→058 absent；062 保存→063 active→064 同 X/personal 草稿“嗨”并引用同 Memory/revision；067 真正切 work 清旧显示→068 absent→069 “您好”无 applied。022 全局选择的快照仍平实，023 才显示正式和用户锁。066 只是 native select 弹层突出 work，067 Return 后才代表实际切换。没有据前一时点误判跨范围泄漏。

记录缺陷：`ui/actions.jsonl` 前导 166 个 NUL 会使严格 JSONL 解析失败；保留原文件，使用独立逐步 JSON 追溯，不悄悄改写日志。评审取证曾误读根目录 actions 文件退出 1，随后找到 ui 下真实路径；其余结论为静态证据核对，未亲跑业务测试。主线程之后完成的 Y/personal 与复制成功提示不冒称已经由此初段评审覆盖。作者停止 Computer Use 后本轮原生矩阵未完成，不签 G-W/G-M 或完整交互验收。
