# 编排进度（BeadFlow / LOOP20）

父代理：https://cursor.com/agents/bc-4efce4bb-1286-4d06-badf-5c61280bc441  
专属分支：`cursor/beadflow-integration-c441`  
产品原则：`docs/bead/PLAN.md`  
工作包：`docs/bead/WORK_PACKAGES.md`  
拍板：`docs/agent-decisions.md`

## 总览

| 项 | 值 |
|---|---|
| 当前轮次 | ROUND 3 |
| 目标轮次 | ≥20，之后继续，除非用户停止 |
| 每轮编制 | 5× Fable-xhigh + 3× Opus-fast + 2× gpt-5.6-sol-xhigh-fast |
| 已检查模块 | 壳 + bead-core + TS + DATA-1 + AL + B04 + B05 + B10 workflow |
| 当前任务 | ROUND 3：B06 像素编辑已吸收（#48）；测试/浏览器核/复审在途 |
| PR | #16 专属线；#20–#46（#45 B03 实现；#46 B07 IA） |
| Merge | B04 `15ad3b3`；B05 `8526c5c`；B10 本提交；不合 unique trunk |
| Blocked | 子代理 `gh` 只读；hosted Bead CI 空 runner（非产品失败） |

## 已检查模块（11.5）

| 面 | 现状 | 下一动作 |
|---|---|---|
| 前端 | B03 上传 + B06 `/edit/:id` 已吸收 | B06 本机测试/浏览器核；B07 实现后置 |
| 后端 | 无 bead 服务 | v0 本机，不造第二灵魂核 |
| API | 无 bead API | 本地 store 契约 |
| 数据库 | Soul 加密 SQLite，与 bead 无关 | `bead-v1` IDB 已随 B03 吸收（BD19） |
| 登录与权限 | Soul HITL/E1；bead 无账号 | 单机；图纸默认不出网 |
| Storage | DATA-1 + bead-v1：网格进 IDB，游标仍可迁自 localStorage | B07 导入导出后置 |
| Cache | 无 | 后置 |
| 第三方 | 无品牌色板授权 | fixture `generic-5mm` |
| 核心业务 | 色号 + BOM + 四模式 steps 已对 rust fixture；AL 复审 PASS | AL-4（MED）rust 边界 fixture 后置；B04 |
| 测试 | `@bead/app` **29 文件 / 426**；bead-core 153 | B06/B07 实现锁后置 |
| 构建 | Soul just/pnpm | 隔离，勿改根脚本 |
| CI/CD | 独立 `bead.yml` 已合；hosted 两作业 3s 空 runner（无 step） | 本机命令为门；勿 empty-commit |
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
- [x] B04 实现合入 `15ad3b3`（#37）；本机 `@bead/app` **25 文件 / 316** 绿
- [x] S2 构建探针合入（#35，`docs/bead/reviews/round2-build.md`）
- [x] S1 覆盖探针合入（#36，`docs/bead/reviews/round2-coverage.md`）
- [ ] AL-4（MED）rust 侧补跨取整边界的 oracle fixture（不改 TS 语义）
- [x] B05 实现合入 `8526c5c`（#39）；本机 `@bead/app` **27 文件 / 358** 绿
- [x] B04 复审合入 `76ae0ef`（#38，ACCEPT；MED-1 行窗口写盘后置）
- [x] B03 上传 IA 合入（#43，`round3-create-upload.md` D-UP-1..16）
- [x] B03 实现吸收 #45（`bead-v1` + BD20 + CreatePage 测试改写）
- [x] B03 复审合入（#47，ACCEPT-WITH-NITS，0 HIGH/MED / 4 LOW）
- [x] B03 浏览器核：上传红图 → 28×28 预览 → 转入拼装 → 库存 G15 缺口无 URL → 画廊史莱姆不回归；刷新后网格仍在
- [x] B06 像素编辑 IA 合入（#44，`round3-pixel-editor.md` D-ED-1..21）
- [x] B06 实现吸收 #48（`/edit/:id` + blank mint；实现席报 32/488）
- [x] B07 导入导出 IA 合入（#46，`round3-import-export.md` D-IE-1..20）
- [x] B05 复审合入（#40，ACCEPT；MED-1 blob URL 后置）
- [x] B10 独立 `bead.yml` 合入（#41）；Soul `ci.yml` 零 diff
- [x] B10 复审合入（#42，ACCEPT，0 HIGH/MED）
- [x] 本机复核：`@bead/app` 22 文件 258 绿（2026-08-25，AL 吸收后）

## 已知问题 / 禁令

- 不要合 PR #4 / #7 / #10 / `main` 进 unique trunk。
- 不要做 AC-27。
- 不要把 bead 加进根 Cargo workspace。
- hosted Actions 空 runner（Billing）：Soul 与 Bead `9b2d65a` 同症（作业 3s、无 step、无 runner_name）。不要 empty-commit 去「修」。本机命令仍是门。

## 下一轮重点（ROUND 3）

B06 已吸收（#48）。下一刀：本机测试 + 浏览器核 blank/`/edit` + Fable 复审。B07 实现、AL-4、MED-1 后置。

```text
ROUND 2（进行中）
子代理任务    F4；DATA-1；AL；AL 复审 #32；B04 IA #33；B05 IA #34
发现问题      assemble 曾是占位；画廊与 generic-5mm 的 G07 实撞
修复问题      DATA-1；AL；BD20；B04 沉浸拼装
测试结果      @bead/app 316 绿；bead-core 153 绿
Commit        15ad3b3 absorb B04；本提交记进度
PR            #16 #32–#37
Merge状态     已进专属线；不合 unique trunk
下一轮重点    B05 实现；B10 workflow；MED-1 后置
已收          [R3 Opus B06 implement](https://cursor.com/agents/bc-192bf45d-a57f-5d88-ad19-97fbe830e072) → #48
已收          [R3 Fable review B03](https://cursor.com/agents/bc-dda3ba42-5efd-5794-893b-ccd3dcd9eace) → #47
已核          B03 上传流：预览/保存/库存/画廊回归/刷新 IDB 均通过
已收          [R3 Opus B03 implement](https://cursor.com/agents/bc-2919c00f-1dff-5f7f-bb0a-9107a34f58a1) → #45
已收          [R3 Fable B07 import IA](https://cursor.com/agents/bc-e46705b1-e91b-55ec-8c47-404ef086f3c6) → #46
已收          [R3 Fable B06 editor IA](https://cursor.com/agents/bc-c104f901-b479-501d-9a47-6b90381b06b2) → #44
已收          [R3 Fable B03 upload IA](https://cursor.com/agents/bc-f50eea16-cc7b-5263-bcc1-e07033175f52) → #43
已收          [R2 Fable review B10](https://cursor.com/agents/bc-f2043eb1-5f82-5964-99a9-ff585747319c) → #42
已收          [R2 Opus B10 workflow](https://cursor.com/agents/bc-08c28463-3949-5d72-b729-c048eaf21aa2) → #41
已收          [R2 Fable review B05](https://cursor.com/agents/bc-f8cb512c-e819-5100-8d2c-7fc3d03bb1b0) → #40
已收          [R2 Opus B05 implement](https://cursor.com/agents/bc-07588339-9d46-582e-9230-b027218aa67b) → #39 `8526c5c`
已核          B04 拼装流通过；B05 资产：空库存全量缺口、加 C01 后缺口下降、采购文本无 URL
已收          [R2 Opus B04 implement](https://cursor.com/agents/bc-6a5bc9c6-7bdc-528b-bb4a-b9611abce044) → #37；[R2 Fable review B04](https://cursor.com/agents/bc-9087419b-67fc-5fd9-aa37-9f8bd95154cd) → #38
Blocked       无（B04 已由本机 worktree 落地）
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
