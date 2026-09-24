# docs/gates/ — 本地门禁记录

> 依据 DECISIONS **D61–D63**：本项目没有 hosted CI（GitHub Actions 已关闭，`ci.yml` 已删）。
> 所有质量门禁在作者控制的机器上本地运行，证据落在这个目录。
> 合到 `main` 或 Goal 分支的每个 sha 都必须有对应记录；没有记录的提交不合（D63）。

## 三道门

| 门 | 在哪跑 | 命令 | 覆盖 |
|---|---|---|---|
| **G-L** | Linux 开发机（D62：本项目指定的 Linux 门禁机，4 vCPU / 3.9 GB） | `just ci-full` | lint / schema / e0 / denylist / fixtures-verify / `cargo test --workspace --all-targets` / smoke-lint / sbom / ui-lint / ui-test / `cargo deny check` / `cargo deny list` |
| **G-W** | 作者 Win11 真机 | `pwsh -File scripts/gate-win.ps1` | `cargo test --workspace --all-targets`（含 SQLCipher、DPAPI）/ 桌面壳 `--all-targets` / 前端 bundle / release 二进制 / `soul.exe` 内嵌 `asInvoker` / `install-smoke.ps1 -SkipInstall` |
| **G-M** | 作者 Win11 真机 | `scripts/author-manual-checklist.md` | `tauri build`、真装真卸、托盘、UAC、进程名、WebView2 抓包、真机采集（没有任何自动化能替） |

G-W 的 Windows TCP 观察必须记录至少 1 次成功查询且未观察到该子进程的非回环连接；监测器缺失或任一次查询报错必须失败，不能记成 skipped success。`0 sample(s)` 不是观察通过；旧脚本即使打印 green，也不能用该行补齐 AC-21。TCP 表轮询不覆盖 UDP，也不替代 G-M 的 WebView2 出网观察。

不在 G-L 里的：桌面壳 mock-runtime 测试（`just desktop-shell-test`：`ipc_roundtrip` / `command_surface` / `no_egress_path`）需要 webkit2gtk 开发库，门禁机没有也装不了（无 sudo）。G-W 的 `desktop-test --all-targets` 覆盖同一批文件；有 GUI 栈的 Linux 机器跑了也写进记录。

哪些改动必须带 G-W：`apps/desktop/**`、`crates/soul-collect/src/windows.rs`、`crates/soul-win-dpapi/**`、`installer-hooks.nsh` / `tauri.conf.json`。

## 命名规则

```
<yyyymmdd>-<sha7>-linux.md   # G-L
<yyyymmdd>-<sha7>-win.md     # G-W（G-M 的七条也填在同一文件的手动节）
```

sha7 是跑门禁时 `git rev-parse --short=7 HEAD`。工作区不干净就先提交，门禁记录对着的必须是一个提交。

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

`scripts/gate-win.ps1` 跑完会把结果表打印出来，贴进来即可；再补环境表与手动节：

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

- [20260924-e2fdf16-win.md](20260924-e2fdf16-win.md) — 当前收尾源码 G-W 通过，独立审查 PASS；NSIS / 打包后 smoke 通过，G-M 未完成，G-L 按用户指示暂缓。

- `20260904-6f6a259-linux.md` — G-L 绿（含 deny-list）；无 G-W

| 文件 | sha | 门 | 结果 |
|---|---|---|---|
| [20260903-e2b4e48-linux.md](20260903-e2b4e48-linux.md) | e2b4e48 | G-L | 11/11 绿（`deny-list` 当时尚未入 `ci-full`；ui-test 全量并发下 1/161 偶发超时，单文件重跑过） |
