# Soul

灵魂级个人软件。根据你授权的社交档案、电脑操作和日常记录，在你自己的设备上复刻电子版的你——人格、记忆、心理倾向、人脉图——并辅助处理电脑事务、起草回复、分析人与事。本机采集与浅层处理；云端深度分析默认关闭。行为数据可在授权下用于行为预测研究，采集许可不自动授予研究保存许可。

桌面端以 Windows 为当前开发与验收平台；跨平台方向与产品边界见 [PRODUCT_LOCK](docs/PRODUCT_LOCK.md)。开发、局部检查、平台验收、合入和发布分别记录，当前完成状态只见 [STATUS](docs/STATUS.md)。

## 原则与入口

- 数据留在获准范围内：无 E0，E1 需单次批准，只起草不发送；审计不存正文，遗忘销毁内容密钥。
- 从当前工作树开始：[AGENTS](AGENTS.md) 是协作入口，[PLAN_INDEX](docs/PLAN_INDEX.md) 按任务导航；规范与决定见 [ACCEPTANCE](docs/ACCEPTANCE.md)、[SECURITY](docs/SECURITY.md)、[DECISIONS](docs/DECISIONS.md)。
- 结论跟随实际版本和执行证据：[协作与记录](docs/operations/repository-workflow.md)、[远端 CI](docs/operations/ci.md)、[平台门禁](docs/gates/README.md)。历史展示页和回执只描述其原版本。

## 本地启动

需要 [rust-toolchain.toml](rust-toolchain.toml) 指定的 Rust、[package.json](package.json) 指定的 Node.js/pnpm，以及 Windows Tauri 构建所需的 C++ 工具和 WebView2。已有依赖可直接运行；需要准备依赖时在仓库根目录执行：

```pwsh
$ErrorActionPreference = 'Stop'
pnpm install --frozen-lockfile
if ($LASTEXITCODE -ne 0) { throw 'pnpm install failed' }
```

桌面启动会打开数据目录。开发时使用仓库外的新合成目录，并通过 `WEBVIEW2_USER_DATA_FOLDER` 使用独立 WebView profile；`SOUL_DATA_DIR` 只隔离库，不隔离同一登录会话的单实例。先退出自己已有的 Soul 实例，以下命令发现实例时会停止，不会终止进程。启动不批准真实数据导入、采集、研究保存或外部请求，仅使用合成材料：

```pwsh
$ErrorActionPreference = 'Stop'
if (Get-Process | Where-Object { $_.ProcessName -eq 'soul' }) {
    throw 'Exit the existing Soul instance before starting an isolated development session.'
}
$previousSoulDataDir = $env:SOUL_DATA_DIR
$previousWebViewDataDir = $env:WEBVIEW2_USER_DATA_FOLDER
try {
    $env:SOUL_DATA_DIR = Join-Path ([System.IO.Path]::GetTempPath()) ('Soul-synthetic-' + [guid]::NewGuid())
    $env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $env:SOUL_DATA_DIR 'webview-profile'
    New-Item -ItemType Directory -Path $env:SOUL_DATA_DIR | Out-Null
    New-Item -ItemType Directory -Path $env:WEBVIEW2_USER_DATA_FOLDER | Out-Null
    Write-Output ('Synthetic data directory: ' + $env:SOUL_DATA_DIR)
    pnpm dev
    if ($LASTEXITCODE -ne 0) { throw 'Soul development startup failed' }
} finally {
    $env:SOUL_DATA_DIR = $previousSoulDataDir
    $env:WEBVIEW2_USER_DATA_FOLDER = $previousWebViewDataDir
}
```

`pnpm dev` 从根目录启动 Tauri 与前端；关闭窗口可能只隐藏到托盘，结束隔离会话时使用托盘“退出”。上例目录用于临时开发，不自动删除；需保留验证证据时，将安全日志和合成复现输入保存到仓库外持久目录，记录实际位置。未设置 `SOUL_DATA_DIR` 时应用使用平台默认目录，不适合限定合成的开发验证。

## 验证

按改动面运行必要检查，执行后立即检查退出码。例如授权与出网及前端检查：

```pwsh
$ErrorActionPreference = 'Stop'
cargo test -p soul-policy -p soul-egress --locked
if ($LASTEXITCODE -ne 0) { throw 'Policy and egress tests failed' }
pnpm --filter @soul/desktop lint
if ($LASTEXITCODE -ne 0) { throw 'Desktop lint failed' }
pnpm --filter @soul/desktop test
if ($LASTEXITCODE -ne 0) { throw 'Desktop frontend tests failed' }
```

根 Rust workspace 与 `apps/desktop/src-tauri` 分别验证。桌面合成测试必须排除真实采集用例，具体命令、生产构建和平台验收边界见[检查入口](docs/operations/repository-workflow.md#检查与失败处理)。失败保留原始结果，修复后重跑原命令。合并前核对目标 SHA 的[远端 CI 实际结果](docs/operations/ci.md)；局部通过不代表完整平台验收。

## 项目结构

```text
Soul/
├── AGENTS.md          协作入口
├── .codex/            本项目实现与只读评审角色
├── crates/            根 Rust workspace
├── apps/desktop/      React 界面；src-tauri 是独立 Cargo workspace
├── fixtures/          合成测试材料
├── scripts/           检查与人工验收入口
└── docs/
    ├── STATUS.md      当前状态与下一步
    ├── PLAN_INDEX.md  按任务查阅
    ├── decisions/     架构与治理 ADR
    ├── plans/         实施计划
    ├── operations/    开发、记录与 CI 操作
    ├── design/        具体设计、调研与执行回执
    ├── architecture/  架构方案、图源与导出图
    ├── algorithms/    算法权威
    ├── schemas/       数据契约与既有 lock
    ├── reviews/       独立审查与修复记录
    ├── gates/         版本和平台验收证据
    └── history/       原版本历史快照
```

当前架构见 [ARCHITECTURE](docs/ARCHITECTURE.md)；共享 runtime/types、合成 API、预览确认与事件刷新的启动及边界见[本地 API](docs/architecture/local-api.md)。[离线架构地图](docs/architecture.html)、[使用闭环与算法展示](docs/framework-design.html)和[历史总览](docs/project-overview.html)保留各自页内版本，不实时代表当前客户端。

## 许可

Cargo 包声明 `LicenseRef-Soul-Proprietary`，前端包指向本 README。仓库尚未提供独立 `LICENSE` 文件或可据以授予使用、复制、修改、分发权利的许可条款；不要将源码可见或这些标识解释为开源授权。许可需由权利人另行明确。
