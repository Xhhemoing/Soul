# docs/gates/ — 本地门禁记录

> 依据 DECISIONS **D61–D63**：本项目没有 hosted CI（GitHub Actions 已关闭，`ci.yml` 已删）。
> 所有质量门禁在作者控制的机器上本地运行，证据落在这个目录。
> 合到 `main` 或 Goal 分支的每个 sha 都必须有对应记录；没有记录的提交不合（D63）。

## 三道门

| 门 | 在哪跑 | 命令 | 覆盖 |
|---|---|---|---|
| **G-L** | Linux 开发机（D62：本项目指定的 Linux 门禁机，4 vCPU / 3.9 GB） | `just ci-full` | lint / schema / e0 / denylist / fixtures-verify / `cargo test --workspace --all-targets` / smoke-lint / sbom / ui-lint / ui-test / `cargo deny check` / `cargo deny list` |
| **G-W** | 作者 Win11 真机 | `pwsh -NoProfile -File scripts/gate-win.ps1` | `cargo test --workspace --all-targets`（含 SQLCipher、DPAPI）/ 桌面壳 `--all-targets` / 前端 lint、test、bundle / release 二进制 / `soul.exe` 内嵌 `asInvoker` / `install-smoke.ps1 -SkipInstall` |
| **G-M** | 作者 Win11 真机 | `scripts/author-manual-checklist.md` | `tauri build`、真装真卸、托盘、UAC、进程名、WebView2 抓包、真机采集（没有任何自动化能替） |

G-W 的 Windows TCP 观察必须记录至少 1 次成功查询且未观察到该子进程的非回环连接；监测器缺失或任一次查询报错必须失败，不能记成 skipped success。`0 sample(s)` 不是观察通过；旧脚本即使打印 green，也不能用该行补齐 AC-21。TCP 表轮询不覆盖 UDP，也不替代 G-M 的 WebView2 出网观察。

不在 G-L 里的：桌面壳 mock-runtime 测试（`just desktop-shell-test`：`ipc_roundtrip` / `command_surface` / `no_egress_path`）需要 webkit2gtk 开发库，门禁机没有也装不了（无 sudo）。G-W 的 `desktop-test --all-targets` 覆盖同一批文件；有 GUI 栈的 Linux 机器跑了也写进记录。

哪些改动必须带 G-W：`apps/desktop/**`、`crates/soul-collect/src/windows.rs`、`crates/soul-win-dpapi/**`、`installer-hooks.nsh` / `tauri.conf.json`。

## 开发反馈与正式验收

- 定向 crate、前端单文件或脚本受控测试是开发反馈，不自动获得 G-L/G-W 结论。正式门禁和人工观察仍按 D61–D63，项目协作分工见 [AGENTS.md](../../AGENTS.md)。
- 同时只安排一份完整构建/门禁/打包；由集成人协调，不能靠多个 agent 各自跑全套争抢同一机器。不要全局覆盖 `CARGO_TARGET_DIR`，当前 G-W 仍按固定 root/desktop target 路径读取产物。
- 传入 `-SkipWorkspaceTests` 是局部复跑；无论脚本末尾如何着色，都不能记录为完整 G-W 通过。逐项记录 SKIPPED / NOT RUN 与原因，不能沿用旧源码绿灯补齐。
- 原始日志和执行结果是证据；摘要不代替实际运行。源码、依赖或脚本变动后重新确定候选和必要验证，不用更新记录中的 SHA 代替重测。

## 命名规则

```
<yyyymmdd>-<sha7>-linux.md   # G-L
<yyyymmdd>-<sha7>-win.md     # G-W（G-M 的七条也填在同一文件的手动节）
```

sha7 是跑门禁时 `git rev-parse --short=7 HEAD`。工作区不干净就先提交，门禁记录对着的必须是一个提交。

## 证据保留

- 原始日志、结果 JSON 和审查回执不能只保存在会随 worktree、缓存或临时目录消失的位置。体积小且不含密钥、个人正文或其他敏感数据的证据，可以随对应记录保存到版本化目录；安装包、release 二进制和大型日志不入 Git，只记录稳定保存位置、字节数和重新计算得到的哈希。
- 删除或注销 worktree 前，先核对其忽略目录和外部证据目录是否仍包含当前记录依赖的唯一副本。需要保留的内容先复制到稳定位置，再执行清理。
- 证据字节已经缺失时，只能写 `NOT VERIFIED` / `NOT RUN` 并重新执行；历史文字、旧哈希和文件名不能恢复一个 PASS。
- 本节规范证据保留方式，不新增 G-L、G-W、G-M 之外的发布 gate，也不改变现有验收条件。

## G-L 记录模板

```markdown
# G-L 门禁记录 — <yyyy-mm-dd> — <sha7> — linux

## 环境
| 项 | 值 |
|---|---|
| 主机 | <OS / CPU / RAM> |
| 分支 / sha | `<branch>` / `<sha7>` |
| rustc / cargo | |
| node / pnpm / just | |
| cargo-deny | |
| 运行人 | |
| 干扰 | <同机并行负载等> |

## 结果（`just ci-full` 各门）
| 门 | 命令 | 结果 | 备注 |
|---|---|---|---|
| lint | `just lint` | ✅/❌ | |
| schema | `just schema` | ✅/❌ | |
| e0 | `just e0` | ✅/❌ | |
| denylist | `just denylist` | ✅/❌ | |
| fixtures-verify | `just fixtures-verify` | ✅/❌ | |
| test | `just test` | ✅/❌ | <binaries / passed / failed / ignored> |
| smoke-lint | `just smoke-lint` | ✅/❌ | 有无 pwsh |
| sbom | `just sbom` | ✅/❌ | 组件数 |
| ui-lint | `just ui-lint` | ✅/❌ | |
| ui-test | `just ui-test` | ✅/❌ | <文件数 / 通过数> |
| deny | `just deny` | ✅/❌ | advisories / bans / licenses / sources |
| deny-list | `just deny-list` | ✅/❌ | |

**结论：** G-L 绿 / 红（红的写明哪一门、日志在哪）。

## 耗时
| 场景 | 耗时 |
|---|---|
```

## G-W 记录模板

`scripts/gate-win.ps1` 跑完会把结果表打印出来；核对实际执行范围、退出码和跳过项后记录，再补环境表与手动节。绿色文字本身不是完整验收证明：

```markdown
# G-W 门禁记录 — <yyyy-mm-dd> — <sha7> — win

## 环境
| 项 | 值 |
|---|---|
| 主机 | Windows 11 <build> / CPU / RAM |
| 分支 / sha | |
| rustc / node / pnpm | |
| WebView2 Runtime 版本 | |

## 结果（`scripts/gate-win.ps1`）
<脚本打印的表>

## 手动节（G-M，`scripts/author-manual-checklist.md`）
| 节 | 结果 | 备注 |
|---|---|---|
| 0 打包 | | |
| 1 静默装 → smoke → 卸载 | | |
| 2 托盘 | | |
| 3 不提权 | | |
| 4 进程名 | | |
| 5 云开关无流量 | | |
| 6 真机采集 | | |
| 7 环境相关 | | |
```

## 已有记录

- [20260927-6fa479f-win.md](20260927-6fa479f-win.md) — 唯一候选 Git HEAD `6fa479f` 的 2026-09-27 完整 G-W 实测 exit 0：根 1052 通过/1 ignored、桌面 88 通过、`-SkipInstall` smoke 15 项通过；原始日志、回执与 release 产物保全，四次尝试逐次记录。当前 NSIS / 作者 G-M 0 / 真实安装卸载仍 `NOT RUN`，G-L 暂缓，Goal 1 未关闭。

- [20260925-ebff0c9-win.md](20260925-ebff0c9-win.md) — 四包独立集成 G-W 的根1052、桌面88、前端202、时间、哈希与exit 0仅保留为历史文字记录；原集成 worktree、`target/q2` 与六个关键原始证据文件当前缺失，因此该历史 G-W 保持 `NOT VERIFIED`；后继候选的 2026-09-27 新运行单独记录于上条，不倒填本历史记录。历史 `ebff0c9` 同源码代理 NSIS 构建和配对 `-SkipInstall` 的声称为 `NOT VERIFIED`；当前唯一候选 NSIS 为 `NOT RUN`，作者 G-M 0 与真实安装/卸载为 `NOT RUN`，G-L 为 `NOT RUN` / 暂缓，G-M 1–7 未完成。

- [20260925-95ff7d6-win.md](20260925-95ff7d6-win.md) — 安装修复的隔离 G-W 与独立受控回归通过；首次真实安装失败、恢复有残留，真实 NSIS 副本验证被执行审核阻断。G-M 未完成、G-L 暂缓。

- [20260924-e2fdf16-win.md](20260924-e2fdf16-win.md) — 该历史固定源码的 G-W、NSIS / 打包后 smoke 通过；后续安装与修复状态见 9 月 25 日记录。

- `20260904-6f6a259-linux.md` — G-L 绿（含 deny-list）；无 G-W

| 文件 | sha | 门 | 结果 |
|---|---|---|---|
| [20260903-e2b4e48-linux.md](20260903-e2b4e48-linux.md) | e2b4e48 | G-L | 11/11 绿（`deny-list` 当时尚未入 `ci-full`；ui-test 全量并发下 1/161 偶发超时，单文件重跑过） |
