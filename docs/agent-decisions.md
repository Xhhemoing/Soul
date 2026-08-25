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
