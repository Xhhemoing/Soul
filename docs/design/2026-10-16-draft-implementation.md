# 起草框架接入与验证 · 2026-10-16

基线：PR #66 工作分支 `codex/soul-forget-outcomes-20260929` 的 `258c0162ca28c44c42dcb3f1c448e01a5bc5c7b6`。本轮按用户“按照这个框架进行项目的调整”指示实施前两步。生产代码与架构文档一起同步，PR 保持草稿，未合并 main。

## 实际变化

| 层 | 调整 | 使用效果 |
| --- | --- | --- |
| React 起草页 | 同步准入、输入代次、预览/原文豁免绑定、卸载失效 | A→B→A 不复活旧批准；旧回复不覆盖新编辑 |
| core.ts | 修改前/后来源失效、在途修改计数、交付检查与订阅 | 修改未完成时不发新起草 IPC；已经显示的旧结果会清除 |
| DraftSession / PolicySession | 冻结正文、按值消费的工作体与单次许可 | 不重建获准正文，不重用已花费批准 |
| Session | begin / execute / complete、来源代次、单槽、单调期限 | 出网结束后先审计，再判断是否可接收；失效不记 draft.create |
| Tauri | Arc 共享同一 Session，spawn_blocking 内负责完整生命周期 | 起草 HTTP 不长期占会话锁；IPC await 丢弃不放弃完成审计 |

仍复用原 39 个 IPC 命令名、参数和返回 schema；`generate_draft` 的 Rust 壳函数改为 async。未变更依赖锁、持久化 schema、T4D/A0–A3 规则或默认权限。

## 边界与恢复

- [D69](../DECISIONS.md)：撤回之后不发新的派发许可，拒收旧代次结果。已发许可可能仍出网，不能承诺“点击撤回后绝无首字节”。
- 一个真正执行中的工作仍占唯一槽。编辑或资料变化仅失效其结果；不能通过反复取消增加并发。
- 准备和回复接收从准备起两分钟有效，使用单调时钟和既有能力令牌默认 120 秒值。过期需重新准备和核对。
- 未知 IPC 结果不视作撤回成功，不自动重发无 token 的 discard；当前端点路径需完整重启应用恢复，本机模板可继续。明确的来源变化显示普通提示，不冒称后端拒绝或要求重启。
- Core 保证覆盖 Session 入口；WebView 补充校验覆盖当前单窗口且通过 core.ts 的调用。任意直接共享 store 写入、其他原生调用者、多窗口同步不在此证明范围。
- 仅迁移起草 HTTP。摘要、目录扫描、导入仍可能长期占锁。手选/自动记忆上下文、用途授权、D55 操作回执、迁移及遗忘后重导未接入。

## 独立评审修订

真实 diff 评审修正：公开工作体缺少 Debug（改为不含正文的手写 Debug）、旧桌面检查写死同步函数/裸 Mutex、测试字段名错误、变更尚在等待时新预览先返回的反序、测试 URL 与既有 E0 扫描冲突。

撤权测试的 mock 不再与控制线程使用相同自动放行超时：现在必须明确释放 HTTP 回复，避免原来的长锁实现也能通过。另增加真实 core.ts + React 的联合微任务调度回归，不只分别测两个替身。

## 本轮实际验证

所有下列成功结果都来自本轮实际执行，非参考模型或旧版证据。

| 检查 | 最终结果 | 范围 |
| --- | --- | --- |
| 完整前端 Vitest | **405 / 405，28 文件，退出 0** | 原 338 项与新增 67 项；包含 6 项真实 wrapper + React 联合时序 |
| TypeScript + ESLint | 退出 0 | 完整桌面前端 |
| Vite 生产 build | 退出 0 | 43 modules，生成 dist；不是原生安装包 |
| Rust 1.83.0 定向回归 | **119 / 119，10 个 suite，退出 0** | lib 17、draft 20、policy 7、atomic edits 4、draft work 10、E1 27、forget scope 2、graph correction 7、import 15、screens 10 |
| E0 静态出网审计 | 退出 0，clean | 16 个 shipped crate / 240 个源码文件；无新增未受控出网路径 |
| Rust 定向格式 | 7 个改动文件，退出 0 | 含 Tauri 源码与测试；不是 Tauri 编译 |
| 离线 HTML 浏览器 | **13 / 13，退出 0** | 四视图、六任务情景、记忆实验、下载字节一致、导航、390px 无整页溢出、无 JS 异常/自动联网；新 SVG 另经目视检查 |
| 独立只读复审 | REVIEWED | 未发现剩余阻塞逻辑问题，不替代平台验证 |
| 桌面原生构建 / IPC | **NOT RUN** | Linux 缺少 pkg-config、GTK/WebKit 开发依赖；新增共享状态测试尚未执行 |

核心新增 14 项测试，其中两个矩阵各验证 14 类来源修改在准备后/派发后失效；这 28 个矩阵分支没有重复计入“119 个测试”。阻塞 HTTP 用例在收到真实请求后，先要求 clear 返回，再明确释放响应；已发许可余量、单次请求计数、未结束的槽、错误会话 completion、无效地址解析、审计写入失败及 panic/过期均有具体断言。

初轮真实运行中两个矩阵因合成测试错误地把轴名当作 UUID 而失败；改为读取实际 profile 的 axis_id 后，完整定向集合重跑通过，没有放宽生产校验。前端首次 399 项后新增 6 项联合回归，最终完整重跑为 405 项；不把早期结果重复累计。

复现命令（仓库根；工具在 PATH，未修改锁文件）：

```sh
pnpm --filter @soul/desktop lint
pnpm --filter @soul/desktop exec vitest run --maxWorkers=2
pnpm --filter @soul/desktop build
CARGO_BUILD_JOBS=1 cargo run --locked -p xtask -- e0-audit
CARGO_BUILD_JOBS=1 cargo test --locked -p soulcore --lib \
  --test session_draft_work --test session_e1 --test draft_commands \
  --test policy_commands --test session_atomic_edits --test session_import \
  --test session_forget_scope --test session_graph_correct --test session_screens \
  -- --format terse
```

E0 首次运行因工作区生成的 anyhow build-script 丢失执行位而在运行检查前退出 101；仅恢复 target 中该生成物的执行位后重跑，审计正常完成。未调整检查规则。

实际工具版本：Rust 1.83.0、Node 24.19.0、环境提供的 pnpm 11.25.0；pnpm 使用 `--frozen-lockfile` 安装，仓库仍声明 pnpm 10.15.0，本轮未更新 packageManager 或依赖锁。前端与 Rust 测试使用合成资料、临时 SQLCipher 库和回环 mock HTTP，没有使用用户资料或配置真实模型端点。

环境为 Linux / Bash。仓库要求的 pwsh 不在此环境，本轮为局部诊断，不冒称正式 G-L/G-W/G-M。完整 G-L 继续暂缓，NSIS / G-M 跳过不等于豁免；Goal 1、正式 Goal 2、Wave 1 均未关闭。
