`claude-fable-5-thinking-xhigh`

# ROUND 2 · WP-B04 沉浸式拼装台 · 实现审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审查，不实现）。
被审对象：合并提交 `15ad3b3`（merge: absorb WP-B04 immersive assemble），基线 `origin/cursor/beadflow-integration-c441`。
审查依据：`docs/bead/reviews/round2-assemble.md`（D-ASM-1…13 / T-ASM-1…19）、`docs/agent-decisions.md`（BD15 / BD19 / BD20）。
本轮只新增本文件；未改任何产品文件、测试文件、rust、Soul 树。

---

## 0. 结论先行

**接受。无 HIGH 缺陷。** 合并面 18 个文件全部落在 `apps/bead/**`，与 IA 的 Ready-for-Opus 清单逐条对得上；
BD19 五键红线、BD20 码空间、D-ASM-12 框架不变量、D-UI-2 双 chrome 全部成立并有测试钉死。
`pnpm --filter @bead/app test` 实测 **25 个文件 / 316 条全绿**（预期 316，命中）。
下面列 1 条 MED 与 4 条 LOW，均不构成回退理由，留给后续小 PR 或 B09/B08 顺带处理。

## 1. 逐项核对（任务钉出的六个检查点）

### 1.1 ProgressCursor 恰为五键，Grid/Step[] 不落盘 ✅

- `stores/types.ts`：`ProgressCursor` 字段恰为 `projectId / mode / stepIndex / elapsedMs / updatedAt`，
  注释原文引用 BD19 与 round2-data §5 的 IDB 触发条件。`PersistedState` / `EMPTY_STATE` 增第四数组。
- `stores/repository.ts` 的 `toProgressCursor` 是**重建**而不是透传：手改 blob 里夹带的多余键
  （`cells`/`steps`/`doneBits`/`grid`…）在消毒时就被剥掉，永远进不了下一次写盘。
  `sanitizeProgress` 做了 §3.2 要求的两步收敛：同 projectId 取 `updatedAt` 最大者、孤儿剪除；
  `saveProgress`（localStorage 实现）在写盘前再过一次 sanitize，双保险。
- T-ASM-7 落在 `repository.test.ts`：往 `saveProgress` 塞带 `cells/steps/doneBits/grid` 的走私对象，
  断言落盘条目 `Object.keys(...).sort()` 恰为五键，且原始 blob 字符串不含四个违禁词。**红线已锁死。**
- 全树无任何代码持久化 `Grid` 或 `Step[]`；`Step[]` 一律 `stepsOf(grid, mode)` 现算
  （带 WeakMap 缓存，每板每模式至多算一次——R-ASM-2 的实现要求也顺带满足）。

### 1.2 fixture 网格用 pattern.palette 下标，非 generic-5mm；cakebox 无网格 ✅

- `fixtures/grids.ts`：字符 `0`–`9` 就是 `pattern.palette` 下标，头注释点名 BD20；
  `fixtureGridFor` 的展示色板从 `pattern.palette` 逐条映射 `{code,name,hex}`。
  `pages/assemble/**` 与 `fixtures/grids.ts` 全文无 `generic` 引用（grep 证实）。
- `grids.test.ts` 专门有一条 BD20 断言：展示色板逐项 `toEqual` `pattern.palette` 的画廊码，
  并在注释里点名 G07 撞码这个已知实例。
- `gal-cakebox-03` 不在 `SPECS` 里，测试断言 `fixtureGridFor("gal-cakebox-03") === null` 且
  `GRIDDED_PATTERN_IDS` 不含它；T-ASM-17 用它做「暂无网格」的在库用例。§4.3 必做两张
  （slime 28×28、arcade 56×56）都在，应做档的 lantern 也画了（torii 未画，见 LOW-4）。
- 一致性钉（§4.2）：逐 fixture 断言尺寸符合 board、逐色下标计数 == `palette[i].beads`、
  总非空格数 == `beadCount`，另加「每个声明的颜色至少画了一颗」的防呆。
  `catalog.ts` 的 diff 恰在批准面内：新增派生说明注释 + 三张有网格图纸的
  `beads`/`beadCount` 数字（slime 412→300、lantern 760→340、arcade 980→1238）；
  码号、名称、hex、标题、无网格的 torii/cakebox 一律未动。`navigation.test.tsx` 仅 412→300 一处。

### 1.3 框架不变量 D-ASM-12；双 chrome 不变 ✅

- `AssemblePage.tsx`：守卫链三态文案照旧；找得到的项目**无条件**渲染
  `<AssembleBackdrop>`（testid / `data-backdrop` 原样）、`<PersistenceBanner>`、`<h1>`、
  「退出拼装」Link、Escape 监听、背景 fieldset（radio 可及名原样）。只有
  `fixture !== null && occupiedCount > 0` 才挂 `<AssembleSession>`。
- T-ASM-17 双参数化（`sourcePatternId: null` 与 cakebox 项目）断言框架在、会话区
  （画布 / 悬浮球 / 计时）不在。
- `app/routes.tsx` 未被合并触碰：`/assemble/:id` 仍在 AppShell 之外，无 mode 段（D-ASM-3 ✅）。
  既有 navigation（含 Escape）、theme-backdrop 三条、empty-states、persistence-banner、isolation
  套件除 412 一处外零改动且全绿——它们用 `sourcePatternId: null` 的项目直进拼装页，
  正是框架不变量在养活（T-ASM-19 ✅）。

### 1.4 模式切换重置 stepIndex、保留 elapsedMs；mode 不进 URL ✅

- `session.ts` reducer `switchMode`：`stepIndex: 0`、`rowSlice: 0`，`elapsedMs` 原样带过；
  同模式切换是恒等空操作。纯逻辑测试直接断言 `elapsedMs === 90_000` 不动。
- 集成面 T-ASM-10：点 radio 后 `beads("done")` 归零，且 `repository.loadProgress()` 里
  mode 已是 `row-by-row`、stepIndex 0——写进仓库这半也验了。
- 提示句 `MODE_SWITCH_HINT`（「切换模式将从第 1 步开始（用时不清零）」）常显，测试钉住。
- URL 全程只有 `/assemble/:id`，mode 只活在游标里（D-ASM-3 ✅）。

### 1.5 计时器不逐 tick 写盘 ✅

- 写盘 upsert effect 的依赖是 `[upsertProgress, projectId, mode, stepIndex, milestoneCount]`——
  `elapsedMs` 走 `latest` ref 在写盘时刻顺带取值，tick 只改 `elapsedMs`，不触发 effect。
- 卸载 flush + `pagehide` flush 都在；`document.hidden` 暂停 / 复显续走有 fake-timers 集成测试。
- 实现者加了一条 IA 没点名要的「写盘时刻」集成测试（`秒针不落盘，卸载与 pagehide 才把用时刷进游标`）：
  走秒 3s 后断言仓库里 `elapsedMs` 仍为 0，pagehide 后变 3000，导航卸载后变 5000。
  这是本次交付里质量最高的一条测试，直接把 D-ASM-8 的反例锁死。
- 秒针 tick 的重渲染半径：`computeCellStates` 用 `useMemo`（依赖不含 elapsedMs）、
  `<AssembleCanvas>` 包 `memo` 且所有 props 在 tick 间值相等、计时显示隔离成 `<SessionClock>`
  ——§5 的性能要求（R-ASM-2）满足。

### 1.6 既有套件仍然成立 ✅

navigation / theme-backdrop / persistence-banner / empty-states / isolation 语义均未被架空：
背景 radio、testid、`data-backdrop`、横幅文案全部原样，抽组件（`AssembleBackdrop.tsx`）只是搬家。
persistence-banner 的「两套外壳都挂横幅」在新页面结构下依旧成立（banner 在框架层，不在会话区）。

## 2. D-ASM-1…13 与 T-ASM-1…19 落点速查

| ID | 判定 | 落点 |
|---|---|---|
| D-ASM-1 | ✅ | `types.ts` 五键 + `session.ts` 页面 reducer；派生数据零持久化 |
| D-ASM-2 | ✅ | `grids.ts` + `FixtureSwatch` 适配器；`AssembleCanvas` 不 import catalog |
| D-ASM-3 | ✅ | 路由零变化，mode 只在游标 |
| D-ASM-4 | ✅ | reducer + T-ASM-10 + 常显提示句 |
| D-ASM-5 | ✅ | CSS：pending 15% 透明、current inset 描边 = `readableTextColor(backdrop)`、reduce-motion 停动画 |
| D-ASM-6 | ✅ | DOM CSS grid、`data-cell-state`、canvas 边界留在 `<AssembleCanvas>` |
| D-ASM-7 | ✅ | 三键 orb（min-height 44px、aria-pressed、逐行 disabled + title）；行窗口纯会话态 |
| D-ASM-8 | ✅ | 可见走秒 / hidden 暂停 / 无手动暂停；写盘时刻见 MED-1 的一处偏差 |
| D-ASM-9 | ✅ | `beadsPerMinute` null 语义、里程碑穿越去重、恢复预标、色号播报仅单色模式 |
| D-ASM-10 | ✅ | `data-scale="physical"`、5mm + gap 0 + inset 描边、常显免责声明含「未校准」「140mm」 |
| D-ASM-11 | ✅ | 完成面板（总用时/总颗数）、「标记完工」→ done + `/workspace`、面板上撤销可用 |
| D-ASM-12 | ✅ | 框架无条件、会话区条件化；T-ASM-17 + 既有套件双重钉 |
| D-ASM-13 | ✅ | `→`/`←` + `ownsArrowKeys` 表单守卫（INPUT/SELECT/TEXTAREA/contentEditable）；无空格键 |

| T | 判定 | 落点 |
|---|---|---|
| T-ASM-1 | ✅ | `fixtures/grids.test.ts`（含解码器四种抛错路径与缓存恒等） |
| T-ASM-2…5 | ✅ | `pages/assemble/session.test.ts`（stats/BPM/reducer 边界/行窗口/里程碑，全部纯函数无 DOM） |
| T-ASM-6/7 | ✅ | `stores/repository.test.ts`（畸形丢弃、孤儿剪除、updatedAt 收敛、五键红线 + 违禁词扫描） |
| T-ASM-8…18 | ✅ | `pages/assemble/assemble.test.tsx`（11 个 describe 逐条对号，另加写盘时刻一条） |
| T-ASM-19 | ✅ | 既有套件零改动（仅批准的 412→300），316 条全绿 |

## 3. BD 红线

- **BD15**：`apps/bead/src` 全树 grep 无 `http(s)://` 字面量；新增源码零外网 URL。
- **BD19**：见 §1.1。补一句：`upsertProgress` 的 `updatedAt` 由 action 现打 `Date.now()`，
  调用方类型 `ProgressCursorInput` 在编译期就拿不到第六个键的入口。
- **BD20**：见 §1.2。库存流未被本 WP 触碰，`buildBom` 无新调用。
- **零新运行时依赖**（D-UI-7）：合并未触碰任何 `package.json` / lockfile。
- **禁区**：合并面 18 文件全在 `apps/bead/src/**`；rust、Soul、`apps/desktop`、根配置、`.github` 零触碰。

## 4. 发现（按严重度）

### HIGH — 无

### MED

- **MED-1 · 行窗口推进不是写盘时刻**。IA §3.2 写明「下一步/撤销（**含行窗口推进时顺带刷新
  `elapsedMs`**）」是写盘时刻之一；实现的 upsert effect 依赖只有 `mode / stepIndex / milestoneCount`，
  纯 `rowSlice` 移动（未穿越里程碑时）不落盘。后果限定在崩溃场景：锁行拼一个大单色步的期间
  `elapsedMs` 的落盘新鲜度退化到「上一次跨步/里程碑/pagehide」，比 IA 设计的粒度粗一档；
  `stepIndex` 语义无损（rowSlice 本就不持久化，恢复回步首是已接受的精度损失）。
  修复是把 `state.rowSlice` 加进 effect 依赖再补一条断言（几行），但按本轮授权（非 HIGH 不动手）留给后续。

### LOW

- **LOW-1 · 进场即写一次游标**。upsert effect 挂载时就跑一次，进过拼装页的项目立刻多一条
  stepIndex 0 / elapsedMs 0 的游标。§3.2 没把「进场」列为写盘时刻，但此写幂等、五键、
  随项目删除被孤儿剪除回收，无实际危害；顺带让「进场即恢复点」成立，倾向保留现状。
- **LOW-2 · 模式切换清空已播报里程碑**。`switchMode` 重置 `announcedMilestones: []`，新模式下
  重新穿越 25% 会再播报一次。D-ASM-9 的「会话内去重」按字面可以读成跨模式去重，但游标归零后
  进度确实从 0 重算，重播是自洽的语义；两读都通，不改。
- **LOW-3 ·「空图纸」态（`EMPTY_GRID_NOTE`）无测试也无在库触发用例**。分支照 §4.3 保留是对的
  （防未来全空 fixture），但当前没有任何 fixture 能走到它。B08 扩 catalog 时若引入新 fixture，
  顺带补一条全空网格的单测即可。
- **LOW-4 · `gal-torii-02` 网格未画**。§4.3 把它归「应做（工期允许就补）」档，缺席合规；
  torii 的 `beadCount: 1540` 仍是手设估值，注释已声明。留给 B08 或独立小 PR。

### 观察（不计数）

- 计时按 `setInterval` 固定 +1000ms 累加，前台节流下有漂移；隐藏页已暂停，v0 精度足够。
- 悬浮球用主题 token（`--bg-surface` 等）画表面：D-UI-6 管的是 backdrop 不读主题，
  当前步描边确实走 `--assemble-outline`（backdrop 亮度派生），不冲突。
- `AssemblePage` 导出的 `NO_GRID_NOTE`/`EMPTY_GRID_NOTE` 目前无外部消费者，仅文件内用——无害。

## 5. 验收记录

```
pnpm --filter @bead/app test
Test Files  25 passed (25)
     Tests  316 passed (316)      ← 预期 316，命中
```

审查环境：`cursor/bead-r2-b04-review-c441` @ 基于 `15ad3b3`（即最新 `cursor/beadflow-integration-c441`）。

## 6. 禁区自查

本轮只新增 `docs/bead/reviews/round2-assemble-review.md`。未改 `apps/bead/**`、
`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、`apps/desktop/**`、
`crates/soul-*/**`、根 Cargo members、`deny.toml`、`.github/**`。本文无外网 URL（BD15）。
