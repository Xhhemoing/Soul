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
| 已检查模块 | 仓库地图（Soul 桌面 + 无 bead 代码） |
| 当前任务 | ROUND 1：3 个 Fable 云端在跑；其余 7 席等 VM 空位 |
| PR | https://github.com/Xhhemoing/Soul/pull/16 （draft → first-test-candidate） |
| Merge | 未合；不合 unique trunk（11.11 跳过） |
| Blocked | 云端 async new-VM limit = 3。F4/F5/O1/O2/O3/S1/S2 排队，不停止 Goal |

## 已检查模块（11.5）

| 面 | 现状 | 下一动作 |
|---|---|---|
| 前端 | 仅 `apps/desktop`（Soul） | WP-B02 壳 |
| 后端 | 无 bead 服务 | v0 本机，不造第二灵魂核 |
| API | 无 bead API | 本地 store 契约 |
| 数据库 | Soul 加密 SQLite，与 bead 无关 | WP-B09 IndexedDB |
| 登录与权限 | Soul HITL/E1；bead 无账号 | 单机；图纸默认不出网 |
| Storage | 无 bead | WP-B09 |
| Cache | 无 | 后置 |
| 第三方 | 无品牌色板授权 | fixture `generic-5mm` |
| 核心业务 | 不存在 | B01–B05 |
| 测试 | 仅 Soul | B01/B02/B10 |
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
Commit        待写入
PR            待创建 → first-test-candidate
Merge状态     跳过（11.11：先保存成果）
下一轮重点    功能地图精化；落地 B01/B02/B03
```

## ROUND 1 派单

| # | 模型 | 方向 | 状态 |
|---|---|---|---|
| F1 | claude-fable-5-thinking-xhigh | 全库地图 + WP 精化 | **在跑** [bc-0a27702c](https://cursor.com/agents/bc-0a27702c-9927-5f5d-bb10-ac67dcbe5136) 分支 `cursor/bead-r1-map-c441` |
| F2 | claude-fable-5-thinking-xhigh | 前端 IA | **在跑** [bc-602028ee](https://cursor.com/agents/bc-602028ee-feb0-5ee1-86a0-f029f714b33d) 分支 `cursor/bead-r1-ui-c441` |
| F3 | claude-fable-5-thinking-xhigh | 算法契约 | **在跑** [bc-2ba7b602](https://cursor.com/agents/bc-2ba7b602-beb5-5d6d-a3da-77112eb9fd1e) 分支 `cursor/bead-r1-algo-c441` |
| F4 | claude-fable-5-thinking-xhigh | 数据 / 存储 / 权限 | **BLOCKED** 等 VM |
| F5 | claude-fable-5-thinking-xhigh | 测试 / CI 隔离 | **BLOCKED** 等 VM |
| O1 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B01 `crates/bead-core` | **BLOCKED** 等 VM |
| O2 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B02 `apps/bead` | **BLOCKED** 等 VM |
| O3 | claude-opus-5-thinking-high-fast | **唯一实现** `packages/bead-algo` | **BLOCKED** 等 VM |
| S1 | gpt-5.6-sol-xhigh-fast | 覆盖缺口探针 | **BLOCKED** 等 VM |
| S2 | gpt-5.6-sol-xhigh-fast | 构建/CI/性能探针 | **BLOCKED** 等 VM |

实现互不覆盖：O1 只碰 `crates/bead-core`；O2 只碰壳/路由/主题；O3 只碰 `apps/bead` 的转换模块，避开 O2 的导航骨架文件除非必要。

## 已完成任务

- [x] 确认 Soul 锁不改写为拼豆（BD1）
- [x] 专属分支 `cursor/beadflow-integration-c441` @ `541d0dc`+
- [x] PR #16 draft
- [x] ROUND 1 先派 3 个 Fable 云端（上限 3）
- [ ] 空位后立即派 O1/O2/O3 与 F4/F5/S1/S2
- [ ] 第一份可运行转图 + 壳

## 已知问题 / 禁令

- 不要合 PR #4 / #7 / #10 / `main` 进 unique trunk。
- 不要做 AC-27。
- 不要把 bead 加进根 Cargo workspace。
- hosted Soul CI 空 runner（Billing）与本线无关，不要 empty-commit 去「修」。

## 下一轮重点（ROUND 2 预告）

Review O1–O3；补 B04 沉浸指引或 B05 库存（看哪块先可接）；未覆盖面优先。

### ROUND 1 · F1 交付（全库地图）

- 交付：`docs/bead/reviews/round1-map.md`（15 面地图、共存硬约束、WP 精化、ROUND 2 建议）。分支 `cursor/bead-r1-map-c441`，基线 `75e3c70`。
- 最要紧的三条共存约束（WP 原文没写）：① `xtask e0-audit` 扫 `apps/**` 与 `crates/**` 的 URL 字面量，bead 代码不得含外网 URL（含 tsconfig/package.json 的 `$schema`、bead-core Cargo.toml 的 `repository`/`homepage`）；② `denylist-audit` 扫 `crates/**/*.rs`，bead-core 禁用 `score`/`得分`/`分数` 等词（置信度叫 `confidence`）；③ `apps/bead` 落地必须同一提交重生成 `pnpm-lock.yaml` 并给 `.gitignore` 加 `apps/bead/dist/`，否则本线 `just ci` 的 `pnpm install --frozen-lockfile` 红。
- 待父代理拍板（建议记 BD14）：派单表把 O3 写成 `packages/bead-algo`，与 WP-B03 的 `apps/bead` 内嵌矛盾；建议 v0 收敛为 `apps/bead/src/algo/`，免改 `pnpm-workspace.yaml`。
