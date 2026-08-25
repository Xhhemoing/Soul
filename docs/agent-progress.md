# 编排进度（BeadFlow / LOOP20）

父代理：https://cursor.com/agents/bc-4efce4bb-1286-4d06-badf-5c61280bc441  
专属分支：`cursor/beadflow-integration-c441`  
产品原则：`docs/bead/PLAN.md`  
工作包：`docs/bead/WORK_PACKAGES.md`  
拍板：`docs/agent-decisions.md`

## 总览

| 项 | 值 |
|---|---|
| 当前轮次 | ROUND 1（启动中） |
| 目标轮次 | ≥20，之后继续，除非用户停止 |
| 每轮编制 | 5× Fable-xhigh + 3× Opus-fast + 2× gpt-5.6-sol-xhigh-fast |
| 已检查模块 | 壳 + bead-core 已合入；CI/IA/算法/地图审查齐 |
| 当前任务 | O3 已合入；SH-1..4 仍在修；TS 管线 Fable Review 在跑 |
| PR | #16 专属线；#20–#24（#24 转图管线） |
| Merge | 壳、core、壳 Review 文档已合进专属线；不合 unique trunk |
| Blocked | F4/S1/S2 与 core Review 等 VM |

## 已检查模块（11.5）

| 面 | 现状 | 下一动作 |
|---|---|---|
| 前端 | 壳已合入；Review 指出 SH-1..4 | Opus 跟进中 |
| 后端 | 无 bead 服务 | v0 本机，不造第二灵魂核 |
| API | 无 bead API | 本地 store 契约 |
| 数据库 | Soul 加密 SQLite，与 bead 无关 | WP-B09 IndexedDB |
| 登录与权限 | Soul HITL/E1；bead 无账号 | 单机；图纸默认不出网 |
| Storage | 无 bead | WP-B09 |
| Cache | 无 | 后置 |
| 第三方 | 无品牌色板授权 | fixture `generic-5mm` |
| 核心业务 | rust oracle + TS 管线都已合入 | Fable 对照色号；B04/B05 |
| 测试 | 壳 43 + bead-core 全绿 | B10 独立 workflow 仍缺 |
| 构建 | Soul just/pnpm | 隔离，勿改根脚本 |
| CI/CD | Soul ci.yml | B10 独立 job |
| 性能 | 无转图基准 | 后置大图 |
| 安全 | Soul 锁不适用于 bead UI | 本地 XSS/文件导入审查 |
| 可靠性 | 无项目持久化 | B09 |

## ROUND 0（本父代理，文档 only）

```text
ROUND 0
子代理任务    无（11.3 直改文档）
发现问题      仓库是 Soul；用户确认拼豆规划；无 apps/bead
修复问题      专属分支 + PLAN / WP / 拍板 / 本进度
测试结果      文档变更，无产品测试
Commit        541d0dc docs: start BeadFlow exclusive line and work packages
PR            https://github.com/Xhhemoing/Soul/pull/16
Merge状态     跳过（11.11：先保存成果；不合 unique trunk）
下一轮重点    功能地图精化；落地 B01/B02/B03；补派被 VM 上限挡住的 7 席
```

## ROUND 1 派单

| # | 模型 | 方向 | 状态 |
|---|---|---|---|
| F1 | claude-fable-5-thinking-xhigh | 全库地图 + WP 精化 | **完成** `0c59fb9`。`docs/bead/reviews/round1-map.md` 已收进专属线。拍板 BD14–BD16 |
| F2 | claude-fable-5-thinking-xhigh | 前端 IA | **完成** `7dcc8ad` → 已 merge 进专属线。`docs/bead/reviews/round1-frontend.md`。子代理 GitHub token 只读，PR 由父代理补 |
| F3 | claude-fable-5-thinking-xhigh | 算法契约 | **完成** `e3cb62d` → 已 merge。`docs/bead/reviews/round1-algorithms.md`。O1 仍在跑，契约 follow-up 等它空闲再 resume |
| F4 | claude-fable-5-thinking-xhigh | 数据 / 存储 / 权限 | **BLOCKED** 等 VM |
| F5 | claude-fable-5-thinking-xhigh | 测试 / CI 隔离 | **完成** `814250f` → #20 **MERGED**。门禁升为 BD17 |
| O1 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B01 `crates/bead-core` | **完成并合入** `5d62896` #23。本机 `cargo test --manifest-path crates/bead-core/Cargo.toml` 绿 |
| O2 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B02 `apps/bead` | **完成** `dc35ef0` → 已 merge。43 tests（子代理）。#21 |
| O3 | claude-opus-5-thinking-high-fast | **唯一实现** `apps/bead/src/algo/`（BD14） | **完成并合入** `5ca90c2` #24。本机 `@bead/app` 20 文件 217 绿。`/create` 上传仍故意不接线 |
| S1 | gpt-5.6-sol-xhigh-fast | 覆盖缺口探针 | **BLOCKED** 等 VM |
| S2 | gpt-5.6-sol-xhigh-fast | 构建/CI/性能探针 | **BLOCKED** 等 VM |

壳 Review：**完成** `4e7b990` #22。SH 修复：[bc-4bea8264](https://cursor.com/agents/bc-4bea8264-3f2b-546b-a858-2d7bd0e9ac13)。core Review 因 VM 上限排队。

实现互不覆盖：O1 只碰 `crates/bead-core`；O2 已完成勿再改壳骨架；O3 只碰 `apps/bead/src/algo/`。

## 已完成任务

- [x] 确认 Soul 锁不改写为拼豆（BD1）
- [x] 专属分支 `cursor/beadflow-integration-c441` @ `541d0dc`+
- [x] PR #16 draft
- [x] ROUND 1 先派 3 个 Fable 云端（上限 3）
- [x] F2 前端 IA 合入专属线
- [x] O2 壳合入专属线（#21）
- [x] O1 bead-core 合入（#23），本机测试绿
- [x] F3 算法契约合入（#18）
- [x] F1 地图收进专属线；BD14–BD16 已拍
- [x] F5 CI 清单合入（#20）
- [ ] F4/S1/S2
- [x] 壳 Fable Review 合入（#22）；SH-1..4 已派 Opus
- [x] O3 转图管线合入（#24），本机 217 绿
- [ ] TS 管线 Fable Review 在跑；core Review 仍等 VM
- [ ] `/create` 上传接线（独立变更，勿打坏 B02 文案测试）
- [x] 可导航的应用壳（转图仍 stub，O3 在换）
- [x] 本机复核：`@bead/app` 9 文件 43 绿；`@soul/desktop` 14 文件 172 绿；lockfile frozen 一致

## 已知问题 / 禁令

- 不要合 PR #4 / #7 / #10 / `main` 进 unique trunk。
- 不要做 AC-27。
- 不要把 bead 加进根 Cargo workspace。
- hosted Soul CI 空 runner（Billing）与本线无关，不要 empty-commit 去「修」。

## 下一轮重点（ROUND 2 预告）

审 O1（CIEDE2000 + 并列规则 + golden）；审 O2（lockfile 同提交、no-egress）；O3 打通 `apps/bead/src/algo/` 与 golden 色号对照；B09 存储契约。

### ROUND 1 · F1 交付

`docs/bead/reviews/round1-map.md` @ `0c59fb9`。后端 / 登录 / 缓存三面 NO_HIGH_VALUE_CHANGE_FOUND。共存三约束已升为 BD15/BD16；O3 落点为 BD14。
