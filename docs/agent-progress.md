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
| 当前任务 | F2/F3 审查已合入；F1 仍在跑；O1 core + O2 shell 在写 |
| PR | https://github.com/Xhhemoing/Soul/pull/16 （draft → first-test-candidate） |
| Merge | F2/F3 审查已合进专属线；不合 unique trunk |
| Blocked | VM 上限仍在。O3/F4/F5/S1/S2 排队；O1 的契约 resume 等空闲 |

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
Commit        541d0dc docs: start BeadFlow exclusive line and work packages
PR            https://github.com/Xhhemoing/Soul/pull/16
Merge状态     跳过（11.11：先保存成果；不合 unique trunk）
下一轮重点    功能地图精化；落地 B01/B02/B03；补派被 VM 上限挡住的 7 席
```

## ROUND 1 派单

| # | 模型 | 方向 | 状态 |
|---|---|---|---|
| F1 | claude-fable-5-thinking-xhigh | 全库地图 + WP 精化 | **在跑** [bc-0a27702c](https://cursor.com/agents/bc-0a27702c-9927-5f5d-bb10-ac67dcbe5136) 分支 `cursor/bead-r1-map-c441` |
| F2 | claude-fable-5-thinking-xhigh | 前端 IA | **完成** `7dcc8ad` → 已 merge 进专属线。`docs/bead/reviews/round1-frontend.md`。子代理 GitHub token 只读，PR 由父代理补 |
| F3 | claude-fable-5-thinking-xhigh | 算法契约 | **完成** `e3cb62d` → 已 merge。`docs/bead/reviews/round1-algorithms.md`。O1 仍在跑，契约 follow-up 等它空闲再 resume |
| F4 | claude-fable-5-thinking-xhigh | 数据 / 存储 / 权限 | **BLOCKED** 等 VM |
| F5 | claude-fable-5-thinking-xhigh | 测试 / CI 隔离 | **BLOCKED** 等 VM |
| O1 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B01 `crates/bead-core` | **在跑** [bc-701ecb28](https://cursor.com/agents/bc-701ecb28-1059-5f1e-84c2-e85f513717ce) 分支 `cursor/bead-r1-core-c441` |
| O2 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B02 `apps/bead` | **在跑** [bc-f8aa23d9](https://cursor.com/agents/bc-f8aa23d9-4b9d-5991-bcb2-b64ca1b2a72c) 分支 `cursor/bead-r1-shell-c441` |
| O3 | claude-opus-5-thinking-high-fast | **唯一实现** `packages/bead-algo` | **BLOCKED** 等 VM |
| S1 | gpt-5.6-sol-xhigh-fast | 覆盖缺口探针 | **BLOCKED** 等 VM |
| S2 | gpt-5.6-sol-xhigh-fast | 构建/CI/性能探针 | **BLOCKED** 等 VM |

实现互不覆盖：O1 只碰 `crates/bead-core`；O2 只碰壳/路由/主题；O3 只碰 `apps/bead` 的转换模块，避开 O2 的导航骨架文件除非必要。

## 已完成任务

- [x] 确认 Soul 锁不改写为拼豆（BD1）
- [x] 专属分支 `cursor/beadflow-integration-c441` @ `541d0dc`+
- [x] PR #16 draft
- [x] ROUND 1 先派 3 个 Fable 云端（上限 3）
- [x] F2 前端 IA 合入专属线
- [ ] O1 bead-core；O2 壳已在跑（须遵守 D-UI id 前缀与 /assemble 双壳）
- [x] F3 算法契约合入（#18）
- [ ] F1 完成后合入；再派 F4/F5/O3/S1/S2
- [ ] O1 空闲后 resume，令其按 round1-algorithms.md 补测
- [ ] 第一份可运行转图 + 壳

## 已知问题 / 禁令

- 不要合 PR #4 / #7 / #10 / `main` 进 unique trunk。
- 不要做 AC-27。
- 不要把 bead 加进根 Cargo workspace。
- hosted Soul CI 空 runner（Billing）与本线无关，不要 empty-commit 去「修」。

## 下一轮重点（ROUND 2 预告）

Review O1–O3；补 B04 沉浸指引或 B05 库存（看哪块先可接）；未覆盖面优先。
