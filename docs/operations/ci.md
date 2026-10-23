# 合成 CI

本页说明 D110 第一阶段的 [GitHub Actions 配置](../../.github/workflows/ci.yml)。代码及配置已按作者的同步指示推送为 [`79a0aa0`](https://github.com/Xhhemoing/Soul/commit/79a0aa06e2615346d6c06dcf72efd6b8bfb60a08)；2026-10-23 推送后查询时 Actions 仍关闭，该 SHA 没有远端 run/check。本次未修改 Actions 设置或分支保护；既有 G-L 暂缓、G-W/G-M 与安装验收边界仍适用。

## 运行范围

`Synthetic CI / Windows synthetic checks and Tauri compilation` 在 `windows-2022` 的单个 job 中顺序执行，任一步失败即阻断后续步骤。同一 PR/分支的新运行取消旧运行。事件仅为普通 `pull_request` 与 `main`、`codex/**` 的 `push`；只有 `contents: read`，checkout 不保留凭据，不使用 `pull_request_target`、用户凭据或外部模型。

| 次序 | 检查 | 实际范围 |
| --- | --- | --- |
| 1 | Rust 格式 | 根与独立桌面 Cargo workspace 分别执行 `cargo fmt --all -- --check` |
| 2 | 严格 Clippy | 两个 workspace 分别 `--all-targets --locked -- -D warnings`；根额外 `--workspace` |
| 3 | 既有静态审计 | `xtask schema-freeze --check`、`e0-audit`、`denylist-audit`；不新增 schema 冻结或放宽 allowlist |
| 4 | 依赖策略 | 固定的 cargo-deny 0.18.2 Windows 官方预构建二进制；两份 Cargo.lock 分别套用既有根 `deny.toml`，保留现有忽略项及来源/许可/HTTP 依赖限制 |
| 5 | 根合成测试 | `cargo test --workspace --all-targets --locked -- --skip windows::runs_on_windows` |
| 6 | 桌面合成测试 | 独立 manifest 的 `--all-targets --locked`；排除 `windows::runs_on_windows`、`collection_can_be_granted_and_taken_back_over_the_ipc`、`a_restart_finds_collection_off_again` |
| 7 | 前端 | pnpm 10.15.0，`pnpm install --frozen-lockfile`；`lint` 已包含 `tsc --noEmit && eslint .`；`vitest run` 沿用现有 2 workers |
| 8 | Windows 编译 | `pnpm --filter @soul/desktop tauri build --no-bundle -- --locked`；production 前端嵌入 release 应用，不打安装器、不启动应用 |

普通 `cargo test` 不启用 ignored 测试；本配置不启动暂停的研究、外部模型、真实采集或真实用户库。测试中的获准 loopback mock 仍用于验证真实请求边界，不能据此声称零 socket。过滤后的结果必须称为限定合成覆盖，不能称未过滤全量验收。

Rust 保持仓库的 **1.83**，安装对应 rustfmt/Clippy，不升级生产工具链。前端使用 Node **>=24.15.0 且 <25**：根 `engines.node >=22` 只是较宽范围，锁定的 Vite 8.2.2 要求 `^20.19.0 || >=22.12.0`，jsdom 30.0.1 进一步要求 `^22.22.2 || ^24.15.0 || >=26.0.0`。不能仅凭“Node 22”判断兼容。workflow 记录实际 Rust/Node/pnpm/cargo-deny 版本，失败保留在对应 run 日志。

现有 `forbid(unsafe_code)` / Windows 根 `deny(unsafe_code)` 与 Win32 小模块的局部 `allow(unsafe_code)` 继续由源码及现有边界测试约束。Clippy 使用既有 `-D warnings`，没有为全仓测试额外设 `unwrap_used`，也没有增加宽泛豁免。若后续要新增生产代码 unwrap 策略，应先界定生产/测试范围并实际验证候选；本次配置不把未存在的规则宣称为已检查。

## 工具与网络

cargo-deny 使用 [0.18.2 官方 Windows release](https://github.com/EmbarkStudios/cargo-deny/releases/tag/0.18.2)，不使用 `cargo install ... latest`。预构建程序自身的构建 Rust 版本与项目编译器分离；其 `--locked` metadata 调用仍由仓库 Rust 1.83 执行。它会拉取当前漏洞数据库；数据库或依赖策略拒绝必须保留为失败，不能为了绿灯增加忽略项。新库格式若不再受此固定版本支持，应在单独工具变更中验证后升级，不能改项目 toolchain 绕过。

暂未引入 nextest：现有 Cargo/libtest 已能执行这次限定合成覆盖，不需要额外 runner 才能实现目标；未验证的 latest 工具 MSRV、不同隔离/并发语义不进入首片。现有普通 Cargo 测试入口继续有效。

工作流依赖 GitHub runner 预装的 MSVC/SDK/Perl，并验证 OpenSSL 所需 Perl 模块；缺少 NASM 时在临时 hosted runner 用 Chocolatey 安装。下载 Rust/Node/pnpm、包依赖、cargo-deny 与漏洞数据库属于 CI 工具网络活动，不能与产品出站或 E1 授权验证混同。不提供任何用户 endpoint、令牌、正文或真实数据。

Action 用法按 2026-10-23 官方资料核对：[checkout](https://github.com/actions/checkout)、[setup-node](https://github.com/actions/setup-node)、[Windows 2022 image](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md)。两个 Action 当前文档示例均为 v7；使用 hosted runner，不承诺旧 self-hosted runner 兼容。Tauri 的 `--no-bundle` 与 `--` 后传 Cargo 参数按 [官方 CLI 源码](https://github.com/tauri-apps/tauri/blob/dev/crates/tauri-cli/src/build.rs)核对。包版本要求直接来自当前 [pnpm-lock.yaml](../../pnpm-lock.yaml)。

## 证据与验收边界

N1 配置调查起点为 `codex/soul-rebuild-20261019`，HEAD `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46` 加当时的 D109 及规范化 dirty 候选。调查原始资料与首次路径查询失败保存在本机 `E:/SoulSyntheticLab/repository-normalization-20261023/ci-worker.log` 及同目录官方来源副本；它们未随仓库发布。调查未运行构建/测试、未安装工具，实际本地验证和后续同步分别见[所属实施记录](../plans/2026-10-23-repository-normalization.md)。

本地语法检查、命令帮助或既有历史绿灯不能证明这个 workflow 已通过 hosted 执行。本次已回读远端代码 SHA `79a0aa06e2615346d6c06dcf72efd6b8bfb60a08`，Actions runs 与 check-runs 查询均返回空集合，原始结果在 `E:/SoulSyntheticLab/local-api-sync-20261023/`，见[同步回执](../plans/2026-10-23-repository-normalization.md#同步与工作区进度回执)。没有 run URL 或执行事件/步骤结果；后续恢复 hosted 执行时须记录实际目标 SHA（PR 可能是 merge ref）、run URL 与各步骤结果，保留失败、取消及 filtered/ignored 范围。

这条 Windows 自动检查不等于作者 Windows 11 的完整 G-W、G-M、安装/卸载、真实 WebView 交互或用户效用验证；也不是 Linux `just ci-full`，不解除 G-L 暂缓。它不发布、不签名、不安装应用、不自动合并，不产生正式平台 PASS。分支保护和 required checks 按本次规范化方向推进，依赖候选先完成审阅/同步且实际 hosted 检查可核对；当前尚未执行远端设置变更。
