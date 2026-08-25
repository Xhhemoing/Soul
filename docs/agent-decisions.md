# 编排拍板（BeadFlow 专属线）

本文件记录本父代理线的架构决定。Soul 拍板仍在 `docs/DECISIONS.md`（D1–D31），此处不覆盖。

| ID | 问题 | 决定 | 理由 |
|---|---|---|---|
| BD1 | 用户确认的拼豆规划是否改写 Soul | **否**。Soul 锁不动。拼豆是兄弟产品 | D27；改产品方向先改锁，用户未授权改 Soul 锁 |
| BD2 | 代码落点 | `apps/bead` + 隔离 `crates/bead-core` | 不砸向导/图谱/store |
| BD3 | bead-core 是否进根 Cargo workspace | **否** | 保护 Soul `just ci` / deny / e0-audit |
| BD4 | 根 pnpm 脚本 | 仍只 filter `@soul/desktop` | 避免未完成的 bead 打断 Soul 门禁 |
| BD5 | 专属分支 | `cursor/beadflow-integration-c441` | 云端分支策略要求 `cursor/*-c441`；11.10 的 `agent/<project>-integration` 用此名落地 |
| BD6 | PR 底 | 先对 `cursor/first-test-candidate-c441` 开 PR，**不合** `cursor/soul-goal1-7b1c` | 唯一主干与拼豆线分离 |
| BD7 | v0 社交 | fixture 画廊，无真实后端/OAuth | 先闭环本机流程 |
| BD8 | 算法权威 | Rust `bead-core` 为 oracle；浏览器 TS 复刻同一 fixture | 可单测、可对照 |
| BD9 | 语音/手势/真实品牌授权色板 | 推迟 | 计划标为可选 / 易侵权 |
| BD10 | 父代理直改 | 仅文档/注释/配置措辞，或 ≤10 行且不碰业务/权限/数据 | 用户 11.3 |
| BD11 | 实现模型 | 写码/修/测：`claude-opus-5-thinking-high-fast`；扫描/拆/Review：`claude-fable-5-thinking-xhigh`；探针：`gpt-5.6-sol-xhigh-fast` | 用户 11.2 / 11.13 |
| BD12 | 静默降级 | 禁止。同系列不可用则暂停询问 | 用户 11.4 |
| BD13 | Soul Goal 1 / PR #4 #7 #10 / AC-27 | 本线不处理 | 已冻结禁令 |
| BD14 | O3 / WP-B03 落点 | v0 只写 `apps/bead/src/algo/`，**不**建 `packages/bead-algo`，不改 `pnpm-workspace.yaml` | F1 地图：避免和第二份 workspace glob 打架；等 O2 壳落地后再派 O3 |
| BD15 | bead 树与 Soul 审计 | bead 源码/清单**禁止**外网 URL 字面量（含 `$schema`、`repository`/`homepage`）；`crates/**/*.rs` 禁用 denylist 词 `score`/`得分`/`分数`/`评分`，置信度字段叫 `confidence` | `xtask e0-audit` 与 `denylist-audit` 跨界全树扫 |
| BD16 | `apps/bead` 落地提交 | 同一提交更新根 `pnpm-lock.yaml`，`.gitignore` 加 `apps/bead/dist/` | `just ci` 用 `--frozen-lockfile` |
| BD17 | e0 URL 与 Soul CI 触发 | 注释里的 URL 也算字面量（e0 不跳注释）；出处只进 `.md` 或 `fixtures/`/`tests/`。禁止从 bead ref 对 Soul `CI` 做 `workflow_dispatch` | F5 `round1-cicd.md` I-4 / CI-6 |
| BD18 | TS 与 rust 谁听谁 | **rust `bead-core` 仍是 oracle**（BD8）。`round1-algo-ts-review` 的 AT-1 不得改写成「以 TS/contract.md 重写 bead-core」。TS 对齐 `crates/bead-core/fixtures/parity` 与合同后的 rust 语义。AT-2（末格采样）在 TS 修 | 审查时 rust 尚未收合同补丁 `1880487` |
| BD19 | 何时上 IndexedDB | v0 方板+步进游标继续 localStorage。**第一次持久化 `Grid` 的 PR**（B03 接线或 B07 导入）必须同时上 `bead-v1` IDB 契约（见 `round2-data.md` §5）。进度只存 `mode`/`stepIndex`/`elapsedMs`，不存整份 `Step[]` | ROUND 2 数据审查；避免 512² 撑爆 quota |
