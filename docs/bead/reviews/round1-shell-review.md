# ROUND 1 · WP-B02 壳交付审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审已合入的壳，不重写产品）。
基线：`origin/cursor/beadflow-integration-c441` @ `7da17d0`（含 WP-B02 合并 `d9b4a5c` 与 CI 清单合并 `7da17d0`）。
输入：`docs/bead/reviews/round1-frontend.md`（IA，D-UI-1…8 / R1…R6）、`round1-cicd.md`（O2 验收清单 LK/E0/NE/V-O2）、`round1-map.md`（§2 共存约束 / §4 B02 精化）、`docs/agent-decisions.md` BD14–BD16、`apps/bead` 全部 55 个文件的源码。

**总评：这是一份实的交付，不是空壳。** 导航不是死链——图纸能实例化成项目、项目能进 `/assemble/proj-*`，且这条链路有端到端测试钉住（IA 验收第 7 条的"防自欺"要求落实了）。43 项测试 / 9 个文件全绿，隔离四条验证本审查全部本机重跑过。发现 4 个需要 Opus 跟进的实缺口（1 中 3 低）与两处记录性偏差，均不构成回滚理由。

---

## 1. 验证记录（本审查全部亲手重跑，非转述 PR 描述）

| # | 命令 | 结果 |
|---|------|------|
| 1 | `pnpm install --frozen-lockfile`（根，pnpm 10.15.0） | 绿（3 workspace 项目，锁文件 up to date） |
| 2 | `pnpm --filter @bead/app lint`（tsc --noEmit && eslint） | 绿 |
| 3 | `pnpm --filter @bead/app test` | 绿：**9 文件 / 43 测试全过** |
| 4 | `cargo run -p xtask -- e0-audit` | **clean**（240 文件扫描，含 apps/bead 全树） |
| 5 | `cargo run -p xtask -- denylist-audit` | **clean**（116 文件） |
| 6 | `pnpm --filter @soul/desktop lint && test`（LK-3 回归） | 绿：14 文件 / **172 测试全过** |
| 7 | `just ci` 其余 cargo 侧（fmt/clippy/schema/fixtures/test/smoke-lint/sbom） | 见 §1.1 |

### 1.1 cargo 侧共存回归

bead 合并对 cargo 的输入**零字节**（diff 只含 `apps/bead/**` + `pnpm-lock.yaml`，`crates/**`、根 `Cargo.toml`、`justfile`、`deny.toml`、`.github/**` 全未动，已用 `git diff 0c3871e..d9b4a5c --stat` 核实），cargo 各门在理论上不可能被此 diff 打红。本机仍完整重跑了 `lint schema fixtures-verify test smoke-lint sbom` 全链，结果：全绿（EXIT=0）。`just` 二进制本环境未装，按 justfile 注释逐条用 cargo 等价命令执行。

## 2. 逐项审查

### 2.1 实修 vs 空导航 —— 通过

`instantiate.test.tsx` 驱动完整闭环：`/pattern/gal-slime-01` →「转入工作台」→ `/workspace` 出现「继续拼豆」→ 断言 href 以 `/assemble/proj-` 开头 → 点进沉浸页 → 断言主导航消失。「加入待拼」路径同样测到待办计数变 1。Workspace 不会永远空、`/assemble/:id` 可达，导航测试不是自欺（IA §7 第 7 条原话落实）。进度逻辑正确地没做（ProgressBar 恒 0 并注明归 WP-B04）。

### 2.2 双 chrome（D-UI-2）—— 通过

`routes.tsx` 里结构分叉一眼可见：7 条常规路由 + `/` redirect + `*` 兜底挂 `<AppShell>` 布局路由；`/assemble/:id` 是并列的顶层路由。测试断言沉浸页内无「主导航」、退出链接与 `Escape` 双通道都回 `/workspace`。守卫屏不自动跳转（保后退键），`empty-states.test.tsx` 断言 pathname 停在原地。

### 2.3 id 前缀（D-UI-4 / R1）—— 通过

`stores/ids.ts` 用 branded type + 运行时守卫把 `gal-/proj-/cr-` 钉进类型系统；`ids.test.ts` 钉前缀常量本身、fixture 全量合规、错前缀互斥（`isPatternId("proj-abc") === false`）、裸前缀非法。`/pattern/:id` 与 `/assemble/:id` 都先过 `isPatternId`/`isProjectId` 再查库。R1 的缓解措施完整落地。

### 2.4 主题 vs 拼装背景（D-UI-6）—— 通过

两套系统物理分居：`stores/theme.tsx`（`data-theme` + `bead.theme` 持久化）与 `pages/assemble/backdrop.ts`（读项目记录，文件头注释明说禁止 token 泄漏）。`theme-backdrop.test.tsx` 三个用例：backdrop 不读 `data-theme`、改 backdrop 不动主题、自定义色写进项目记录（经 repository 断言持久化）。

### 2.5 `/create` 占位入口（R6）—— 通过

五入口全部到位；未实现的四个在列表上直接标「尚未实现，归 WP-B03/B06/B07」，说明页再写一遍「占位不是缺陷，请勿提 bug」。`CreatePage.test.tsx` 把归属钉死（B03×1、B07×2、B06×1），后续工作包想改归属必先改测试。选中项走 `?entry=`（D-UI-1），无新路由。

### 2.6 不引 Soul —— 通过

三重防线：eslint `no-restricted-imports` 禁 `@soul/*`、`@tauri-apps/*`、`**/apps/desktop/**`；`isolation.test.ts` 全源码 grep；本审查独立 grep 复核零命中。`package.json` 依赖只有 react / react-dom / react-router（D-UI-7），无 UI kit、无状态库。

### 2.7 无 URL 字面量 / `$schema` —— 通过

独立 grep `https?://|fetch\(|XMLHttpRequest|WebSocket|sendBeacon` 全树零命中；`package.json`/`tsconfig.json` 无 `$schema`（E0-4）；`index.html` 无 CDN/字体外链（E0-6）；`src/schema/` 只有 README，无 `$id`（E0-7）；e0-audit clean 是机器复核（E0-5）。

### 2.8 锁文件（BD16 / LK-1…LK-4）—— 通过

- **LK-1**：`pnpm-lock.yaml` 与 `package.json` 在**同一个提交** `ba9dc38`（脚手架首提交）落地，bead 分支不存在中间红提交；干净环境 `--frozen-lockfile` 绿。
- **LK-3**：锁文件 diff +90/−0 纯新增（`apps/bead` importer + `react-router`/`cookie`/`set-cookie-parser` 等新包），`apps/desktop` importer 零改动；Soul 桌面 172 测试仍绿。
- **LK-4**：根 `package.json`、`pnpm-workspace.yaml` 零字节改动。
- **LK-2**：见 §3 SH-5（等价满足，形式偏差）。

### 2.9 隔离测试 —— 部分通过，见 SH-1

`isolation.test.ts` 只覆盖了 Soul 引用禁令，**NE-1 要求的 no-egress 源码断言（`fetch(`/`XMLHttpRequest`/`WebSocket`/`sendBeacon`/外网 URL）没有落地**。当前树干净（grep + e0-audit 双证），但可执行的守门测试缺位——这是 WP-B09「图纸默认不出网」红线的执行形式，也是 map §4 B02.4 的明文要求。

### 2.10 回归 —— 通过

合并 diff 只含 `apps/bead/**` + `pnpm-lock.yaml`。`ci.yml`、`justfile`、根 `Cargo.toml`、`deny.toml`、根 `package.json`、`pnpm-workspace.yaml`、`apps/desktop/**`、`crates/**` 全部零字节。Soul 侧 lint + 172 测试本机复跑绿。

### 2.11 缺失测试 —— 两处边缘分支未测，判定不值得补

Explore 的"fixture 集为空"防御分支（需 mock 掉 catalog 模块才能触发）与 AssemblePage 的未 hydration「正在读取本地项目……」分支没有测试。两者都是防御性死分支，mock 成本高于价值。NO_HIGH_VALUE_CHANGE_FOUND。

### 2.12 过度设计 —— 无实质问题

依赖面、组件面、状态面都贴着 IA 的减脂线走。branded id type 是 R1 的直接回应，不算发明。两处记录性注意：`src/algo/` 占位（SH-6）与 `BoardKind` 含 hex/round（SH-3）。

---

## 3. 发现清单（SH-*，供 Opus 跟进席引用）

### 需要动代码的（Opus follow-ups，按优先级）

- **SH-1（中）· NE-1 no-egress 断言缺失。** 在 `apps/bead/src/test/isolation.test.ts` 现有扫描器上加第二个用例：断言全源码无 `fetch(`、`XMLHttpRequest`、`WebSocket`、`sendBeacon`、`http(s)://` 字面量（该文件已自排除，模式字符串不会自伤；`fetch(` 等非 URL 也不会触发 e0）。约 10 行。参照 `apps/desktop/src/contract.test.ts` 的思路。
- **SH-2（低）· 端口 1430 ≠ E0-8 钉的 1520。** 壳（`ba9dc38`，17:24）与 CI 清单（`814250f`，同日更晚合入）并行产出，壳无从遵守。共存实质（≠1420 + `strictPort`）已满足，但 ROUND 2 按 E0-8 逐条打钩会红。二选一，建议前者：① `vite.config.ts` 改 `port: 1520` + README 同步（2 行）；② ROUND 2 把 E0-8 改写为「≠1420 且 strictPort」。
- **SH-3（低）· fixture 板型越过 v0 范围。** `gal-lantern-04`（hex）、`gal-arcade-05`（round）与 map §3「v0 只做方板、六角/圆板推迟」冲突。B02 里只是展示字段无害，但这两张图可实例化成项目，B04（只实现方板）落地后会拼错。跟进：fixture 两处改 `square-28`（2 行，归 SH-1 同一批或 WP-B08 扩容时顺手），或 B04 对非方板出守卫屏。
- **SH-4（低）· 持久化形状校验缺 backdrop 字段。** `repository.ts::isProject` 只验 `id/title/status`；localStorage 是用户可手改的边界，缺 `backdrop` 的记录会通过过滤，`readableTextColor(undefined)` 在 `/assemble` 抛异常。跟进：`isProject` 补两行校验或 parse 时填默认值。可与 SH-1 同一 PR。

### 记录性（无需动代码）

- **SH-5 · LK-2 以等价形式满足。** 清单写的是根 `.gitignore` 加 `apps/bead/dist/`，实际落的是 `apps/bead/.gitignore`（`dist/` + `coverage/`），git 语义等价且同提交。ROUND 2 打钩时按此注记，不要求返工。
- **SH-6 · `src/algo/` 占位落在 BD14 划给 O3 的目录里。** 只有 `NOT_IMPLEMENTED` 抛错 + 归属常量，应用零引用，测试钉边界——判定为合理的边界桩，非越权实现。**O3 开工时应原地替换该模块**（含删掉占位测试），不要绕开另起目录。

## 4. NO_HIGH_VALUE_CHANGE_FOUND 子树

以下子树本轮判定无高价值改动，列出免后续重复论证：

- 路由表与双 chrome 结构（`app/`）：与 IA §2/§4 逐条对齐。NO_HIGH_VALUE_CHANGE_FOUND。
- store 骨架（`stores/`，SH-4 除外）：async repository（D-UI-5）、selector 现算派生数据、页面不碰 localStorage，全部合规。NO_HIGH_VALUE_CHANGE_FOUND。
- 空状态覆盖（IA §3 清单）：十行清单里可测的行全有测试。NO_HIGH_VALUE_CHANGE_FOUND。
- a11y（IA §6）：底栏 `aria-label`/`aria-current`/文字常显、色块必带色号文本（`ColorSwatch` 把文本做进组件让调用方无法省略）、`<html lang="zh-CN">`、每路由 `<title>` 有断言。NO_HIGH_VALUE_CHANGE_FOUND。
- 主题 token（`tokens.css`）：IA §6 列的 8 个 token 全在，双主题成对。NO_HIGH_VALUE_CHANGE_FOUND。
- 依赖面：运行时仅 react/react-dom/react-router，caret 锁 major（R5）。NO_HIGH_VALUE_CHANGE_FOUND。

## 5. 禁区自查

本审查只新增本文件（`docs/bead/reviews/round1-shell-review.md`），未改 `apps/bead` 任何源码（SH-1…SH-4 均留给 Opus 实现席），未触碰 Soul 锁、`apps/desktop`、`crates/**`、根配置、`.github/**`、`STATUS.md`。
