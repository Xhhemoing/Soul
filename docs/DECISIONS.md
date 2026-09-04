# 自主拍板记录

作者 2026-08-24 声明：除已给出的产品方向外，其余问题可自主决定。本文件记录拍板，避免后续重开方向战。

| ID | 问题 | 决定 | 理由 |
|---|---|---|---|
| D1 | Soul 是助手还是数字复刻 | 数字复刻为灵魂层，助手为代理层，复刻是本体 | 作者定义是电子版的你 |
| D2 | 要不要默认云端 | 默认全本地；v0.1 无云端 HTTP | 本地优先 |
| D3 | 社交数据怎么进来 | 官方导出或手动导入；v0.1 无 OAuth；禁止爬虫 | 合法可审计 |
| D4 | 能否自动回消息 | v0.1 只起草不发送 | 代发不可逆 |
| D5 | 心理模型是否临床 | 否。可替换特质轴 + 证据档 + 用户纠正 | 避免医疗声称 |
| D6 | 手机何时做 | Android 在 v0.3 | Windows 先闭环 |
| D7 | 研究数据如何用 | v0.1 只预览不写文件；与助手出网分离 | 防偷渡 |
| D8 | v0.1 是否要真预测模型 | 只要 schema 与离线入口 | 先有数据契约 |
| D9 | 技术栈 | Tauri2 + Rust + React | 常驻与打包 |
| D10 | 文件整理是否产品本体 | 否。v0.1 只做只读计划预览；执行在 v0.1.1 | 代理层证明不靠真写文件 |
| D11 | 出网怎么分级 | E0/E1/L，全部经 net_guard | 否则「零出网」不可测 |
| D12 | v0.1 的 E0 | 无实现、无域名、无 HTTP client | 关开关不够 |
| D13 | 第三人正文进 E1 | 默认占位；单次整条豁免；研究无豁免 | 第三人未同意 |
| D14 | 主库 | 加密 SQLite；JSONL 非存储 | 2026-08 计划评审共识 |
| D15 | 遗忘 | 销毁内容密钥；不承诺物理擦除 | 可测 |
| D16 | 遗忘 vs 审计 | 审计无内容，可留孤立 UUID | 两承诺共存 |
| D17 | v0.1 导入 | soul-import-v1 + Telegram result.json | 具名才可测 |
| D18 | 研究导出形态 | 预览 only | 脱敏未红队前不落盘 |
| D19 | WP11 写执行 | Goal 1 不做 | 防误读成产品本体 |
| D20 | 记忆是否门禁 | 是 | 作者原意第 6 条 |
| D21 | 人事分析是否门禁 | 是；无 key 则统计降级 | 作者原意第 5 条 |
| D22 | 大五怎么写 | 方向轴 + 弱中强，禁 score/percentile | 非量表 |
| D23 | 采集面 | 仅前台应用时长 | 同意闭环一项即可 |
| D24 | HITL | 未知拒绝；plan_hash 变拒绝；令牌一次性 | 可测 |
| D25 | 注入 | 外部内容永不进指令位；红线进门禁 | 2026-08 计划评审共识 |
| D26 | 写码前文档 | 锁、拍板、九 schema、SECURITY、STATUS | 14 份会空转 |
| D27 | 第二份 PRODUCT.md | 禁止 | 防双源 |
| D28 | Goal 2 | 单独规划，Goal 1 关闭后再排（2026-09-04 改指 `docs/GOAL2_PLAN.md`，原文件已移除，见 D64） | Goal 1 必须有终点 |
| D29 | 验收权威 | `docs/ACCEPTANCE.md` 中 Given/When/Then 矩阵（2026-09-04 自开工合同迁出，AC 文本未改，见 D64） | 散文不作门禁 |
| D30 | 工作包增减 | 计划冻结后只减不增 | 防回弹 |
| D31 | 只读整理预览是否留在 Goal 1 | 留。执行仍在 v0.1.1。这是代理层只读证明，不是产品卖点 | 否决评审中删除 AC-18 的提案 |

> D32–D49 已由 `main` 上的算法拍板占用（T4D / A0 / A2 / 导入口径），本分支不复用这些编号；合回 `main` 时两表直接拼接。

| ID | 问题 | 决定 | 理由 |
|---|---|---|---|
| D50 | 门禁在哪跑 | **不用 hosted CI**。GitHub Actions 已在仓库设置里关闭（`actions/permissions.enabled=false`），`.github/workflows/ci.yml` 删除。GitHub 只做代码存储与同步。门禁 = G-L（Linux 开发机 `just ci-full`）+ G-W（作者 Win11 `scripts/gate-win.ps1`）+ G-M（`scripts/author-manual-checklist.md`），证据落 `docs/gates/<yyyymmdd>-<sha7>-<linux|win>.md` | 作者 2026-09-03 明令；私库账本不可控（8/25 起五门 0 step）；hosted 本来就证不了托盘 / UAC / DPAPI / WebView2 真机行为。AC-26 的「CI」自此读作本地门禁 |
| D51 | Linux 门禁机 | 本项目指定的 Linux 门禁机（4 vCPU / 3.9 GB）。首份 G-L：`docs/gates/20260903-e2b4e48-linux.md`。这台机没有 webview 开发库也没有 sudo，桌面壳 mock-runtime 测试（`just desktop-shell-test`）不在 G-L 内，由 G-W 的 `desktop-test --all-targets` 覆盖 | 冷编 35 分钟可接受；装 GUI 栈要 root |
| D52 | 没有门禁文件的提交能不能合 | 不能。合到 `main` 或 Goal 分支的每个 sha 必须在同一提交序列里带对应 G-L 记录；改到 `apps/desktop`、`soul-collect/windows.rs`、`soul-win-dpapi`、NSIS 的还要 G-W | 本地门禁唯一弱点是「谁跑的、跑了没」，用落盘记录堵上；D29 同一条纪律 |
| D64 | 开发流程文档 | 作者 2026-09-04 指示：仓库只保留代码与规划。移除 `FORMAL_WORK_PROMPT.md`、`GOAL2_POLISH_PROMPT.md`、`PLAN_VERIFY_PROMPT.md`、`templates/THREE_ROUND_DUAL_SCAN.md`、`scan-rounds/`。验收矩阵、工作包、红线迁至 `docs/ACCEPTANCE.md`（AC 文本未改）；Goal 2 方向迁至 `docs/GOAL2_PLAN.md`。**合并规则**（为落实上述指示而拟，非作者原话，待作者确认）：这些文件在任何后续合并中一律保留删除；来源分支对验收矩阵的实质变更（如 `main` 的 AC-28–34）移植进 `ACCEPTANCE.md`，不恢复原文件。评审轮次溯源标签（D14/D25/D31）已归一化为「2026-08 计划评审」。同理，`.agent_workspace/`、`docs/agent-*.md`、`.github/workflows/` 在任何后续合并中一律不带入。历史见 tag `backup/pre-cleanup-20260904`（= `3eb0d8f`，标签仅在本机） | 流程配置不是产品；写下规则防止合并时静默复活或静默丢失 |
| D65 | 平台定位 | 作者 2026-09-04 原话：「本项目不是 windows only，只是前期现在 Windows 系统上进行开发和测试，后期支持核心部署到 linux mac windows 多端，同时可以接入 mac linux windows Android 多端进行数据获取和反馈呈现」。归纳：Windows 是 v0.1 的开发与测试平台，不是唯一目标。核心后续部署 Linux / macOS / Windows；接入端（数据采集与结果呈现）含 macOS / Linux / Windows / Android。v0.1 边界表不变。**不变量**：非测试代码中 `cfg(windows)` 目前只在 6 个文件（`soulcore/src/netwatch.rs`、`soul-win-dpapi/src/lib.rs`、`soul-store/src/keys.rs`、`soul-collect/src/lib.rs`、`soul-collect/src/source.rs`、`apps/desktop/src-tauri/src/instance.rs`）；此外新增 Windows 绑定须先记一条决策 | 多端是产品方向；移植面现在有界，不让它扩大 |

> 注（2026-09-04）：本分支 D50–D52 与 `main` 上同号决策语义不同（`main` 的 D50–D52 是算法 crate 事项）。合并时不能直接拼接两表；以「全部 refs 最大号 + 1」为本分支门禁三条重新取号，并同步修正引用它们的文件。
