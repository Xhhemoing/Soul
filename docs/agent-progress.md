# 编排进度（BeadFlow / LOOP20）

父代理：https://cursor.com/agents/bc-4efce4bb-1286-4d06-badf-5c61280bc441  
专属分支：`cursor/beadflow-integration-c441`  
产品原则：`docs/bead/PLAN.md`  
工作包：`docs/bead/WORK_PACKAGES.md`  
拍板：`docs/agent-decisions.md`

## 总览

| 项 | 值 |
|---|---|
| 当前轮次 | ROUND 2 |
| 目标轮次 | ≥20，之后继续，除非用户停止 |
| 每轮编制 | 5× Fable-xhigh + 3× Opus-fast + 2× gpt-5.6-sol-xhigh-fast |
| 已检查模块 | 壳 + bead-core + TS 管线 + DATA-1 + AL + B04/B05 IA |
| 当前任务 | B04/B05 IA 已合；B04 实现改走本机隔离 worktree（云端 VM 仍满）；S1/S2 探针已派 |
| PR | #16 专属线；#20–#34（#34 B05 IA） |
| Merge | AL `331b513`；复审 `3fac2fd`；B04 IA `47b9ac0`；B05 IA `fbe0921`；不合 unique trunk |
| Blocked | 子代理 `gh` 只读（BLOCKED_PR）；云端异步 VM ≈3 |

## 已检查模块（11.5）

| 面 | 现状 | 下一动作 |
|---|---|---|
| 前端 | 壳已合；B04 IA D-ASM-1..13；B05 IA D-INV | Opus 实现 B04（等 VM），随后 B05 |
| 后端 | 无 bead 服务 | v0 本机，不造第二灵魂核 |
| API | 无 bead API | 本地 store 契约 |
| 数据库 | Soul 加密 SQLite，与 bead 无关 | WP-B09 IndexedDB（Grid 落盘才上，BD19） |
| 登录与权限 | Soul HITL/E1；bead 无账号 | 单机；图纸默认不出网 |
| Storage | DATA-1 已合：写失败后读写同切内存并提示 | B04 只存游标（mode/stepIndex/elapsedMs） |
| Cache | 无 | 后置 |
| 第三方 | 无品牌色板授权 | fixture `generic-5mm` |
| 核心业务 | 色号 + BOM + 四模式 steps 已对 rust fixture；AL 复审 PASS | AL-4（MED）rust 边界 fixture 后置；B04 |
| 测试 | `@bead/app` 22 文件 **258** 绿；bead-core 153 | B10 独立 workflow 仍缺 |
| 构建 | Soul just/pnpm | 隔离，勿改根脚本 |
| CI/CD | Soul ci.yml | B10 独立 job |
| 性能 | 无转图基准 | 后置大图 |
| 安全 | Soul 锁不适用于 bead UI | 本地 XSS/文件导入审查 |
| 可靠性 | DATA-1 已合 | AL 复审 |

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
| O1 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B01 `crates/bead-core` | **完成** tip `2a47329`。合同补丁已只收 `crates/bead-core`（避免整支 merge 回滚 TS 管线）。本机 153 测试绿 |
| O2 | claude-opus-5-thinking-high-fast | **唯一实现** WP-B02 `apps/bead` | **完成** `dc35ef0` → 已 merge。43 tests（子代理）。#21 |
| O3 | claude-opus-5-thinking-high-fast | **唯一实现** `apps/bead/src/algo/`（BD14） | **完成并合入** `5ca90c2` #24。本机 `@bead/app` 20 文件 217 绿。`/create` 上传仍故意不接线 |
| S1 | gpt-5.6-sol-xhigh-fast | 覆盖缺口探针 | **BLOCKED** 等 VM |
| S2 | gpt-5.6-sol-xhigh-fast | 构建/CI/性能探针 | **BLOCKED** 等 VM |

壳 Review：**完成**。SH-1..4 **完成并合入** `35e1047` #25。本机 `@bead/app` 220 绿。core Review：[bc-40caa864](https://cursor.com/agents/bc-40caa864-ee53-5058-a593-b3c27fbe396a)。TS Review 仍在跑。

实现互不覆盖：O1 只碰 `crates/bead-core`；O2 已完成勿再改壳骨架；O3 只碰 `apps/bead/src/algo/`。

## 已完成任务

- [x] 确认 Soul 锁不改写为拼豆（BD1）
- [x] 专属分支 `cursor/beadflow-integration-c441` @ `541d0dc`+
- [x] PR #16 draft
- [x] ROUND 1 先派 3 个 Fable 云端（上限 3）
- [x] F2 前端 IA 合入专属线
- [x] O2 壳合入专属线（#21）
- [x] O1 bead-core 合入（#23）+ 合同补丁 `1880487`，153 绿
- [x] F3 算法契约合入（#18）
- [x] F1 地图收进专属线；BD14–BD16 已拍
- [x] F5 CI 清单合入（#20）
- [x] F4 数据审查合入（#29）；DATA-1 合入（#31）
- [x] 壳 Fable Review 合入（#22）；SH-1..4 合入（#25）
- [x] O3 转图管线合入（#24）
- [x] AT-2 + rust parity 对齐合入（#28）
- [x] TS / core / align Review 合入（#26 #27 #30）
- [x] AL-1/2/3 合入专属线 `331b513`（抖动取整 + steps slug/断言 + 分叉表）
- [x] AL 复审合入 `3fac2fd`（#32，三项 PASS，无新增 HIGH）
- [x] B04 IA 合入 `47b9ac0`（#33，`docs/bead/reviews/round2-assemble.md`）
- [x] B05 IA 合入 `fbe0921`（#34，`docs/bead/reviews/round2-inventory.md`）；BD20 码空间
- [ ] B04 实现（Opus 本机 worktree 在途；云端 VM 仍满）
- [ ] S1/S2 覆盖与构建探针（本机 worktree 在途）
- [ ] AL-4（MED）rust 侧补跨取整边界的 oracle fixture（不改 TS 语义）
- [ ] B05 库存/BOM UI（等 B04 实现席）
- [ ] `/create` 上传接线（独立变更；若持久化 Grid 必须同时上 IDB，BD19）
- [ ] B10 独立 bead workflow
- [x] 本机复核：`@bead/app` 22 文件 258 绿（2026-08-25，AL 吸收后）

## 已知问题 / 禁令

- 不要合 PR #4 / #7 / #10 / `main` 进 unique trunk。
- 不要做 AC-27。
- 不要把 bead 加进根 Cargo workspace。
- hosted Soul CI 空 runner（Billing）与本线无关，不要 empty-commit 去「修」。

## 下一轮重点（ROUND 2）

B04/B05 IA 均已合。下一刀：**Opus 实现 B04**（等 VM）。B05 实现、B10、S1/S2、AL-4 后置。判定器/产品框定/检测退化按 align-review §4 NO_HIGH_VALUE，不动。

```text
ROUND 2（进行中）
子代理任务    F4；DATA-1；AL；AL 复审 #32；B04 IA #33；B05 IA #34
发现问题      assemble 仍占位；画廊与 generic-5mm 的 G07 实撞
修复问题      DATA-1；AL；BD20 钉码空间与 BD19 同触发点
测试结果      @bead/app 258 绿；bead-core 153 绿（未改 rust）
Commit        fbe0921 merge B05 IA；本提交记进度 + BD20
PR            #16 #32 #33 #34
Merge状态     已进专属线；不合 unique trunk
下一轮重点    Opus 实现 B04（只存游标）；随后 B05
在途          [R2 Opus B04 implement](https://cursor.com/agents/bc-6a5bc9c6-7bdc-528b-bb4a-b9611abce044)（本机 worktree）· [R2 gpt-sol coverage](https://cursor.com/agents/bc-661a636b-0864-57bc-ab00-e375887a24ff) · [R2 gpt-sol build CI](https://cursor.com/agents/bc-687124cc-a345-5d96-beff-a9440eaa6755)
已收          [R2 Fable review AL](https://cursor.com/agents/bc-7b365534-e6fd-5e45-bc50-769cc93d4454) → #32；[R2 Fable B04 IA](https://cursor.com/agents/bc-d265c746-7f1f-5358-9493-9e49856a4b82) → #33；[R2 Fable B05 IA](https://cursor.com/agents/bc-e1451aad-5003-5ffb-9364-0a3bb1f0e619) → #34
Blocked       云端异步 VM 仍满；实现改本机隔离 worktree
```

### ROUND 2 · AL-1/2/3 吸收

来源：[R2 Opus AL-2 AL-3](https://cursor.com/agents/bc-2dbfdfac-21a3-5f48-8505-07cf5535e917)  
分支 `cursor/bead-r2-al23-c441`（基于当时专属线 `6e9f3c1`，不含 DATA-1；与 store 无路径冲突）。

- AL-1 `81184be`：contract.md 分叉表按 rust 现状。
- AL-2 `c0d346f`：`dither.ts` 查表前 `toChannel`；G7；`fixtures/parity.json` 两条开抖动用例重生成。
- AL-3 `f0e6a21`：`Phase::slug` `inner-edge`；四份 oracle steps 逐组断言。

禁区守住：`crates/bead-core` 零触碰。

### ROUND 1 · F1 交付

`docs/bead/reviews/round1-map.md` @ `0c59fb9`。后端 / 登录 / 缓存三面 NO_HIGH_VALUE_CHANGE_FOUND。共存三约束已升为 BD15/BD16；O3 落点为 BD14。
