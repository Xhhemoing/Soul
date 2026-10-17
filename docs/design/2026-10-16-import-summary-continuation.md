# 导入交付与人物摘要：计划和验收

起点：PR #66，`c1d4f8fc82f9535e12c9e9a07823df1599581432`。依据最新端到端框架和仓库使用框架设计，先实现两个可验证切片。本文先于生产修改建立；文末记录实际验证，不预报通过。

## A. 导入交付

输入仍是用户已预览、已确认的两种导出文本；输出仍是既有计数回执或结构化核心拒绝。新增的是当前 WebView 中导入页的交付状态，不是数据库操作回执或消息去重。

- 一个共享协调器在调用 commit factory 前同步占槽。页面卸载只放弃本页输入，不释放实际提交；重挂载订阅同一状态。
- 状态为 ready → pending → ready（成功或明确拒绝），或 pending → unknown（非结构化失败、回执格式/来源不符）。pending/unknown 不允许另一个提交；只有真实提交 settle 才离开 pending。
- unknown 不声称回滚、不展示原始异常、不提供重复提交/清除按钮；允许用户查看图谱与审计。重启会丢失该内存状态，但不是未提交的证明，页面明确重复导入风险。
- 成功仅保存既有计数/说明/格式，标为“本次运行最近收到的回执”；pending/unknown 隐藏旧成功。原文、文件名、路径不进入协调器或日志。
- 关系重算写回量与本次事件写入量分开表述；前者可能包含已有关系，也不等于图中保留的全部关系数。继续保留重复导入会重复事件/证据的既有告知；结果卡链接已有图谱/审计，不推断关系变化，不自动启动 E1。

验收反例：同回调重入仅一次调用；提交后卸载重挂仍阻止第二次调用；成功回包可在新页面显示；明确拒绝可显式重试；未知失败跨重挂保留且不泄露异常；旧成功不伪装成新提交完成；回执来源/计数不符按未知处理；读取/预览失败不锁提交；D55 重复导入语义和原预览时序回归继续通过。

覆盖仅当前单 WebView 导入入口；不宣称 native/direct wrapper、多窗口、进程崩溃后的幂等或恢复。操作 ID、持久回执、迁移和遗忘后重导依然待独立设计，不新增导入确认期限。

## B. 人物摘要

目标是让用户触发的人物摘要 HTTP 不再占 Session/Store 长锁，保留统计回退、原有证据算法、现有 IPC 数据形状与审计动作。后端实现者已在会话中先提交以下方案，经独立规划评审后进入编码。

- begin 在 Session/Store 锁内校验 AnalysePeople/User、解析图和全部证据、构建 counts 并准备既有 PersonSummary 用途许可；Store guard 仅覆盖准备，不跨入 execute。只读 counts 无端点时不占网络槽。
- 起草和端点摘要共用一个真实 E1 worker 槽，以内部 kind/id 区分，不增加并发；来源变动只失效结果，不提前释放物理槽。counts completion 不取走另一个 worker 的槽。
- owned execute 不借用 Session/Store，锁外 HTTP 后继续使用原 phrase_with 的 grounded/clinical/fallback 检查。准备材料与解析材料在派发前逐字核对；不重发 HTTP。保存出网审计后再隔离解析 panic，避免丢失已发生的请求记录。
- complete 仅接受原会话对应 kind/id，先记录请求审计，再校验来源代次；过期来源拒返整个旧结果。审计失败也不交付。仍无独立 summary audit 动作、正文日志或摘要新期限。
- Tauri person_summary 改 async，由持有同一 Arc Session 的 spawn_blocking 闭包完整执行三阶段；保留 IPC 名、参数和返回形状以及同步 Session 兼容入口。

验收反例：门控回环 HTTP 期间撤回/读库能完成；同址设置、端点 A→B→A、纠正/导入/遗忘等来源修改后迟到摘要拒收且请求审计仍在；摘要与草稿互斥、失效和网络完成未接收都不提前放槽；counts 不占槽、外来 completion 不偷槽；传输/跑题/临床输出回退原统计；审计失败拒返；准备材料不一致零出站；解析 panic 保留请求审计。

前端配合接收校验：资料/端点变更时清除已展示摘要，变更进行中不请求新摘要；旧回包不覆盖最新对象。保留用户主动点击，不后台出网。

## 验证记录

两个模块均先提交具体方案，经独立规划复核后编码，再接受独立只读 diff 评审。复审修正了“释放 Store guard 后才发许可”的文档描述，以及把关系重算写回量误称现存关系总量的文案；修订后未发现剩余阻塞逻辑问题。REVIEWED 限定静态逻辑范围，不等于平台验收。

| 检查 | 实际结果 | 范围 |
| --- | --- | --- |
| 最终完整前端 Vitest | **428 / 428，30 文件，退出 0** | 真实 React/core wrapper 与合成 IPC；含导入 unknown 跨重挂、交付微任务和来源变更 |
| TypeScript / ESLint | 退出 0 | 完整桌面前端 |
| Vite production build | 退出 0 | WebView 静态资源，不是原生安装包 |
| 定向 Rust 格式 | 5 文件，退出 0 | core、测试与 Tauri 源码；不是 Tauri 编译 |
| IPC 静态对应 | **6 / 6，退出 0；39 命令** | 执行原 command_surface 断言正文，将库导入替换为实际源码声明的命令常量；不编译/调用 Tauri |
| Rust 1.83 / SQLCipher 关联回归 | **162 / 162，15 套件，退出 0** | lib 19、draft 20、policy 7、atomic edits 4、commands 20、draft work 10、E1 27、forget scope 2、forget summary 3、graph correction 7、import 15、projection 4、screens 10、summary 4、summary work 10 |
| E0 静态出网审计 | 退出 0，clean | 16 个 shipped crate / 243 个源码文件 |
| 原生桌面构建 / IPC | **NOT RUN** | 当前环境缺 pkg-config 与 GTK/WebKit 开发依赖 |

本轮工具为 PowerShell 7、Rust 1.83.0、Node 24.19.0 与已有锁文件对应的前端依赖。pnpm 的当前包装拒绝使用复用依赖目录的符号链接；改为直接执行相同已安装 Vitest/tsc/ESLint/Vite CLI，未更新依赖锁。测试仅用合成资料、临时加密库和回环模型端点。

首轮 Rust 新来源矩阵在成功导入分支失败：测试混用了两个不同“本人账号”的导出，触发既有单本人校验。改为同一 Telegram 本人再次导入后，完整定向集合重跑全部通过；没有放宽生产校验或删减来源拒收/审计/槽位断言。新增摘要工作 10 项与内部 2 项测试中，16 类来源变化计作一个矩阵测试，未重复累计。最终格式化后又单独核验摘要工作集合，不另计测试总数。

实际命令（PowerShell；原生程序每次非零退出立即检查；不代表完整平台门禁）：

```pwsh
# apps/desktop
node node_modules/vitest/vitest.mjs run --maxWorkers=2
node node_modules/typescript/bin/tsc --noEmit
node node_modules/eslint/bin/eslint.js .
# NODE_ENV 临时设为 production，结束后恢复原值
node node_modules/vite/bin/vite.js build

# 仓库根，CARGO_BUILD_JOBS=1
cargo test --offline --locked -p soulcore --lib `
  --test session_summary_work --test session_summary --test session_draft_work `
  --test session_e1 --test draft_commands --test policy_commands `
  --test session_atomic_edits --test session_import --test session_forget_scope `
  --test session_graph_correct --test session_screens --test session_forget_summary `
  --test session_projection --test session_commands -- --format terse
cargo run --offline --locked -p xtask -- e0-audit
```

审计保证限定生产 Session/Tauri 完整三阶段路径：直接丢弃 owned completion 仍可能放弃调用方的审计责任；旧低层 summarize_person 的 Result 类型不能在最终错误时携带 attempt audit。transport 内部 panic 若尚未形成 outcome，也不证明请求零出站。本轮没有扩大这几项保证。

不标 GATED/INTEGRATED，不关闭 Goal 1、正式 Goal 2、Wave 1。完整 G-L 暂缓、G-W/G-M 与原生平台边界沿用既有指示。消息去重、持久操作回执、任务恢复/洞察反馈、记忆用途/自动检索，以及扫描/导入长锁仍未在本轮实现。
