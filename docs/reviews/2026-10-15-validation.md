# 本轮架构改动验证记录

日期：2026-10-15（Asia/Shanghai）。基线：`Xhhemoing/Soul` PR #66，`9d3cd300c19c996f9edabf337c9d359fcb594bc3`。测试针对该基线上的本轮文件差异执行，随后用户授权将同一业务代码及新增架构文档同步到原工作分支。测试后仅增改文档与图，未修改业务代码；提交身份以包含本记录的 Git 历史为准。完整设计见 [架构报告](2026-10-15-architecture-review.md)。

## 环境与执行边界

- Linux x86_64；Node v24.19.0、pnpm 11.25.0、Rust/rustfmt 1.83.0。
- Rust 工具链安装在任务临时目录，未修改用户全局工具配置；`RUSTUP_HOME`、`CARGO_HOME` 和 PATH 均按进程设置；`CARGO_BUILD_JOBS=1`。
- `pnpm install --frozen-lockfile --ignore-scripts` 退出 0；根 Cargo.toml、两个 Cargo.lock 和 pnpm-lock.yaml 均与固定基线逐字节相同。
- 仓库要求 PowerShell，但本执行器不支持指定 pwsh，且未提供该程序。本轮使用 Bash 诊断并检查真实退出码，不将其登记为正式 G-L/G-W。
- 测试只用合成 fixtures、临时库及已有 mock。本轮没有读取或上传真实聊天数据、数据库或密钥。

## 已执行的检查

| 命令/范围 | 结果 | 交付包日志 |
| --- | --- | --- |
| `pnpm --filter @soul/desktop test` | 退出 0；23 文件 / 338 测试通过 | `logs/frontend-all-final.log` |
| `pnpm --filter @soul/desktop lint` | 退出 0；完整 TypeScript 与 ESLint 检查通过 | `logs/frontend-lint.log` |
| `NODE_ENV=production pnpm --filter @soul/desktop build` | 退出 0；TypeScript + Vite 生产构建成功 | `logs/frontend-build.log` |
| `pnpm --filter @soul/desktop exec vitest run src/routes/Import.test.tsx src/routes/Import.lifecycle.test.tsx src/routes/Import.race.test.tsx src/routes/importRequests.test.ts src/routes/Memory.write.test.tsx` | 退出 0；5 文件 / 85 测试通过，已包含在上述 338 中 | `logs/frontend-regressions.log` |
| `cargo +1.83.0 test -p soul-store --test forget_scope --test forget --lib --locked` | 退出 0；28 单元 + 7 既有遗忘 + 2 范围测试，共 37 项通过 | `logs/rust-store-final.log` |
| `cargo +1.83.0 test -p soulcore --test session_forget_scope --test session_screens --test profile_memory_commands --lib --locked` | 退出 0；13 单元 + 3 profile/memory + 2 新范围 + 10 Session，共 28 项通过 | `logs/rust-core-final.log` |
| 对 7 个改动 Rust 文件执行 `rustfmt --edition 2021 --check` | 退出 0；没有执行整仓格式重写 | `logs/rust-format-final.log`（成功无输出） |

Rust 格式检查涉及 `soul-store/src/forget.rs`、`soul-store/tests/forget_scope.rs`、`soulcore/src/commands/session.rs`、`soulcore/tests/session_forget_scope.rs`、桌面 `commands.rs`、`tests/one_store.rs` 和 `tests/shell_is_local_only.rs`。存储层测试使用项目现有 bundled SQLCipher 与字段加密实现，不是对存储替身的模拟。

合计 338 项前端测试、65 项定向 Rust 测试通过。两个 Rust 命令覆盖不同测试二进制，不把重复运行次数算作新增覆盖。最终补丁另以 `git apply --check` 对固定 SHA 的原始 archive 检查可应用性。

## 失败如何处理

1. 首次完整前端运行：335 项中 331 通过、4 失败，全部位于 Import 清空选择后的迟到读取/预览结果。保留原断言，修复页面请求归属，并新增 3 项反例后全量 338 通过。原始失败记录保留为 `logs/frontend-all-before-fix.log`。
2. lint 首先指出 Memory 在 render 中读取 ref；改为 lazy state 保存稳定协调对象后通过，没有禁用规则。
3. Rust 首次命令返回 101，日志只留下依赖下载。随后直接重跑时 28+7 项通过，但新增 forget_scope 测试二进制无法启动（Permission denied）。检查确认该单个生成产物权限为 0644、文件头为零且不是 ELF。只删除该无效生成产物并让 Cargo 重新链接；后续完整存储命令退出 0。没有通过 chmod 伪装修复，也没有更改源码行为来绕过测试。
4. 定向 rustfmt 首次发现新增测试排版差异；仅在改动文件中应用格式修正，最终退出 0。存储测试最终执行后的一次改动仅为同一测试的空白排版。

## 新增反例与独立评审

新增 5 项 Rust 测试覆盖：相同数量的 evidence/inference 替换；blob ID 替换；无关记忆不使报价失效而共享密钥范围扩大使其失效；Session 在销毁前拒绝旧报价且拒绝审计不死锁、新报价可执行并在重启后仍不可读；编辑后旧报价被拒绝且新内容保留。

新增 12 项前端测试覆盖 Memory 写入拒绝/未知结果/准入失败、等待确认、编辑代次与卸载，以及 Import 清空选择后的旧成功/失败和提交保护。定向与全量测试存在包含关系，不能将其数量相加声称覆盖了更多独立测试。

独立只读评审检查了实际源码差异、锁释放路径、对象身份比较、页面归属和新测试，未发现阻塞问题。评审者未运行构建，不以静态评审代替上述真实执行。复审提出的撤权设计边界措辞已修正为“派发许可发放前撤权”，避免承诺撤回已经派发的网络数据。

## 架构图与 GitHub 同步检查

用户追加要求后新增四张架构图、对应 Mermaid 源和 SVG，并更新 README、计划索引与当前状态。四个文内 Mermaid 块与 `.mmd` 逐字一致，全部通过 Mermaid 11.12.2 的真实浏览器渲染，页面错误为零；SVG 已逐张目视检查并修正长标题与目标图横向过宽的问题。架构文档的本地相对链接全部可解析，独立只读复审确认当前/未来能力、字段归属及信任边界表述准确。

本次同步前，将上一轮交付的 13 个 Rust/TypeScript 修改文件与当前源码逐字节比较，全部一致。本轮仅增改文档与图，未重新累计或重复宣称业务测试数量。GitHub 更新以 `9d3cd30` 为父提交并对分支旧 SHA 做条件校验，避免覆盖并发更新；未合并 main。

## 未运行与仍待完成

- 未运行整个 Rust workspace、独立 Tauri workspace 的编译/测试、原生桌面 IPC、Windows G-W、NSIS 安装/卸载或人工 G-M。
- 未运行完整 G-L；既有暂缓指示未解除。局部真实 Rust/SQLCipher 和 React/Vitest 通过不等于正式门禁通过。
- Session 长任务占锁与独立撤权仍是 P0 设计待实施事项；本轮没有修改授权、取消、迁移或 D55 重导语义。
- 初次交付未进行 commit、push、merge；随后用户明确授权同步代码及架构图到 GitHub，提交到原工作分支并保留草稿 PR，未合入 main。Goal 1、正式 Goal 2 与 Wave 1 均未据此关闭。
