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
| 当前任务 | 文档落地、开 PR、派 ROUND 1 十子代理 |
| PR | 待本提交后创建 |
| Merge | 未合；不合 unique trunk |
| Blocked | 无（云端子代理并发上限可能 BLOCKED，见下） |

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

## ROUND 1 派单（启动）

| # | 模型 | 方向 | 状态 |
|---|---|---|---|
| F1 | claude-fable-5-thinking-xhigh | 全库地图 + WP 精化（11.5） | 待派 |
| F2 | claude-fable-5-thinking-xhigh | 前端 IA / 路由 / 空状态 | 待派 |
| F3 | claude-fable-5-thinking-xhigh | 算法契约审查（CIEDE2000 / 四模式） | 待派 |
| F4 | claude-fable-5-thinking-xhigh | 数据 / 存储 / 权限 / 不出网 | 待派 |
| F5 | claude-fable-5-thinking-xhigh | 测试 / CI / 可靠性隔离 | 待派 |
| O1 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B01 bead-core | 待派 |
| O2 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B02 应用壳 | 待派 |
| O3 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B03 转图管线（可与 O1 契约对齐后开工） | 待派 |
| S1 | gpt-5.6-sol-xhigh-fast | 覆盖缺口探针 | 待派 |
| S2 | gpt-5.6-sol-xhigh-fast | 构建/CI/性能探针 | 待派 |

实现互不覆盖：O1 只碰 `crates/bead-core`；O2 只碰壳/路由/主题；O3 只碰 `apps/bead` 的转换模块，避开 O2 的导航骨架文件除非必要。

## 已完成任务

- [x] 确认 Soul 锁不改写为拼豆（BD1）
- [x] 专属分支
- [ ] ROUND 1 十子代理在途
- [ ] 第一份可运行转图 + 壳

## 已知问题 / 禁令

- 不要合 PR #4 / #7 / #10 / `main` 进 unique trunk。
- 不要做 AC-27。
- 不要把 bead 加进根 Cargo workspace。
- hosted Soul CI 空 runner（Billing）与本线无关，不要 empty-commit 去「修」。

## 下一轮重点（ROUND 2 预告）

Review O1–O3；补 B04 沉浸指引或 B05 库存（看哪块先可接）；未覆盖面优先。
