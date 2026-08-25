# ROUND 2 · WP-B04 沉浸式拼装台 IA

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只做 IA / 拆分，不实现）。
基线：`origin/cursor/beadflow-integration-c441` @ `e24b812`。
输入：`docs/bead/PLAN.md`（沉浸指引一节）、`docs/bead/WORK_PACKAGES.md`（WP-B04）、
`docs/bead/reviews/round1-frontend.md`（D-UI-2 双 chrome 已落地 / D-UI-6 背景 ≠ 主题）、
`docs/bead/reviews/round2-data.md`（§3 步游标形状 / DATA-1 / DATA-2）、`docs/agent-decisions.md`（**BD19**）、
`apps/bead/src/pages/assemble/AssemblePage.tsx` 与 `backdrop.ts`（现状 stub）、`apps/bead/src/algo/steps.ts`
（四模式已存在，含 `inner-edge`）、`apps/bead/src/algo/grid.ts`、`apps/bead/src/stores/types.ts`（`Project`
无任何进度/网格字段）、`stores/repository.ts`、`stores/store.tsx`、`fixtures/catalog.ts`、
`app/{navigation,theme-backdrop,empty-states,persistence-banner}.test.tsx`、`test/render.tsx`（harness）。
硬禁区自查：本轮只新增本文件；不触碰 `apps/bead` 产品文件、Soul 锁、`apps/desktop`、`crates/soul-*`、根配置。

---

## 0. 结论先行

B04 的全部原料已经就位：四模式拆步是现成纯函数（`splitSteps`），双 chrome、退出通道、backdrop、
DATA-1 横幅、渲染 harness 全部已交付。**B04 缺的只有三样**：进度游标的持久化形状（BD19 已钉，本文落成
类型与仓库增量）、一条把真实网格送上画布的 v0 通路（本文钉为只读 fixture 网格，§4）、以及会话
交互本身（HUD / 悬浮球 / 计时，§6）。实现者照 D-ASM-1…13 执行即可，不需要再发明任何产品法。

## 1. 决策摘要（编号供实现与后续 Review 引用）

| ID | 决定 | 一句话理由 |
|---|---|---|
| **D-ASM-1** | 会话状态归页面 reducer；持久化只有步游标 `{projectId, mode, stepIndex, elapsedMs, updatedAt}`，作为 `bead.state` 的第四个数组走现有 localStorage 仓库 | BD19 原文；`Step[]` 与 `Grid` 是派生数据，禁止落库；IDB 触发点不在本 WP |
| **D-ASM-2** | v0 网格来路 = 只读 fixture：`fixtures/grids.ts` 按 `PatternId` 键入，渲染时经 `project.sourcePatternId` 现取现算；网格是权威，catalog 颗数须与之一致（测试钉死） | 不接 `/create` 上传（B03）、不持久化 `Grid`（BD19）、不建第二个 store；画廊实例化已存在，缺的只是格子 |
| **D-ASM-3** | `mode` 不进 URL，路由保持 `/assemble/:id` | 游标是有存储归宿的持久数据；进 URL 会造双权威——旧链接携带的 mode 会静默重置 stepIndex。D-UI-1 管的是导航态，不管档案 |
| **D-ASM-4** | **模式切换重置游标到第 0 步**；`elapsedMs` 不动（它是项目累计用时，不属于某个模式）；切换器旁常显一句「切换模式将从第 1 步开始」 | BD19 只存一份 `mode/stepIndex`，四模式各存游标违反契约；且 stepIndex 跨模式无语义，保留只会指向错误的格子 |
| **D-ASM-5** | 三态渲染：已完成步 100% 原色（镜像实体板）、当前步 100% + 高亮描边、未来步 **15% 透明度**；灰度方案弃用；描边色用 `readableTextColor(backdropColor)` 现算 | WP 给了「15% 透明度或灰度」二选一：低透明度保留色相利于预判下一步，灰度全丢；描边取自 backdrop 亮度，任何自定义背景都有对比，且不碰主题 token（D-UI-6） |
| **D-ASM-6** | 画布用 DOM（CSS grid + `data-cell-state` 属性），不用 `<canvas>` | v0 网格 ≤56×56（≤3136 节点），jsdom + testing-library 可断言属性；`<canvas>` 需像素级测试，harness 测不了。换 canvas 的边界留在 `<AssembleCanvas>` 组件内（512² 属 B07 之后） |
| **D-ASM-7** | 悬浮球三键语义：下一步/撤销移动游标（持久化粒度=步）；**锁定当前行 = 当前步内的逐行窗口**，行位置仅存会话内存，刷新回到步首；逐行扫描模式下该键禁用 | round2-data §3 已裁定 localStorage 阶段禁止逐格进度数组、「恢复到当前步开头」是可接受精度损失——行窗口给出步内逐行指引而零持久化成本 |
| **D-ASM-8** | 计时器页面可见时走、`document.hidden` 自动暂停；不做手动暂停键；`elapsedMs` 只在游标写盘时刻一并落盘 | 每秒写 localStorage 是无谓整包重写（round2-data §2.2 节拍=步）；崩溃损失=回到当前步首，已被接受 |
| **D-ASM-9** | BPM 与里程碑纯派生、永不持久化。`bpm = round(placedBeads × 60000 / elapsedMs)`，`placedBeads==0 或 elapsedMs<1000` 显示「—」；里程碑 25/50/75/100% 向上穿越各播报一次（会话内去重），恢复进场时把已达里程碑记为已播报；「当前色号完成」**仅**单色模式播报（其余模式 `step.color === null`） | 派生数据禁止落库（IA §5 规则）；恢复时不重放旧里程碑，撤销后二次穿越不再吵 |
| **D-ASM-10** | 1:1 透光模式：格距 5mm 走 CSS `mm` 单位；开关仅会话内存；开启期间**常显**免责声明（含「显示器未校准」与「28 格应约 140mm，可用尺核对」）；三态渲染规则不变，溢出走滚动 | WP 原文「按 CSS mm 近似（注明显示器未校准）」；BD19 不给此开关存储位；tooltip 藏免责声明等于没写 |
| **D-ASM-11** | 完成态：`stepIndex === steps.length` 出完成面板（总用时、总颗数），主 CTA「标记完工」→ `setProjectStatus(id,"done")` 并回 `/workspace`；面板上撤销仍可用 | Workspace 历史完工段已存在，`setProjectStatus` 已存在，缺的只是这一根线 |
| **D-ASM-12** | 框架不变量：**任何找得到的项目**都渲染完整框架（`<h1>` 项目名、退出链接、Escape、背景 fieldset、PersistenceBanner）；网格/会话区（画布、HUD、悬浮球）才以「有无 fixture 网格」为条件 | 现有 navigation / theme-backdrop / persistence-banner 测试全部用 `sourcePatternId: null` 的项目直进 `/assemble/:id`——框架砍了它们就红。空白项目与立体图纸走「暂无网格」设计态 |
| **D-ASM-13** | 键盘：`→` 下一步、`←` 撤销（事件目标是表单控件时忽略）；`Escape` 退出保持现状；不加空格键 | 空格与聚焦按钮的激活语义撞车会双触发；背景色 `<input type="color">` 等控件不能被全局键劫持 |

## 2. 路由与状态所有权

路由零变化：`/assemble/:id` 仍在 AppShell 之外（D-UI-2 保持），守卫屏、退出链接、Escape 原样。
本 WP 对 round1-frontend §5 状态表的**增量**：

| 状态 | 拥有者 | 持久化 | 写者 | 读者 |
|---|---|---|---|---|
| 步游标 `ProgressCursor`（mode / stepIndex / elapsedMs / updatedAt） | ProjectStore（`progress` 数组） | localStorage `bead.state` 第四键，走现有 Repository | AssembleSession 在 D-ASM-8 列出的时刻 upsert | AssemblePage 恢复；（Workspace 显示用时属后续，见 §9） |
| 会话瞬态：行窗口位置、计时运行态、已播报里程碑、1:1 开关、锁行开关 | AssembleSession reducer | **不持久化** | 页内 | 页内 |
| 网格 + 展示色板 | fixture（构建期只读） | 不持久化（BD19） | 无 | AssembleCanvas / 派生统计 |
| `Step[]`、placedBeads、进度 %、BPM、里程碑 | 派生 | **禁止持久化** | — | `useMemo(splitSteps(grid, mode))` 现算 |

依赖方向新增一条并锁死：`pages/assemble` 与 `stores` 可以 import `algo/**`（纯函数），
`algo/**` 永不 import `stores`/`pages`（现状即如此，写进本文防倒灌）。

## 3. 持久化契约（BD19 落成代码形状）

### 3.1 类型（进 `stores/types.ts`，即 round2-data DATA-2 的执行）

```ts
import type { SplitMode } from "../algo/steps.ts";   // 类型单一来源，不复刻联合

export interface ProgressCursor {
  projectId: ProjectId;
  mode: SplitMode;        // 校验用 SPLIT_MODES 运行时常量
  stepIndex: number;      // 0 起，消费端夹取到 [0, steps.length]
  elapsedMs: number;
  updatedAt: number;
}
```

`PersistedState` 增 `progress: ProgressCursor[]`；`EMPTY_STATE` 增 `progress: []`。
**红线**：`ProgressCursor` 的键集合就是以上五个，多一个键（`cells`、`steps`、`doneBits`、`grid`…）
即违反 BD19——第一个持久化 `Grid` 的 PR 必须同时上 `bead-v1` IDB（round2-data §5），那不是本 WP。

### 3.2 仓库增量（`stores/repository.ts`，沿用整数组模式）

- `ProjectRepository` 增 `loadProgress(): Promise<ProgressCursor[]>` / `saveProgress(cursors): Promise<void>`；
  `createRepository` 与 `createInMemoryRepository` 同步实现（read-modify-write 整包，与现有六方法同形）。
- `parsePersistedState` 增第四键消毒，先例照 `isProject`：`projectId` 过 `isProjectId`、`mode ∈ SPLIT_MODES`、
  `stepIndex`/`elapsedMs` 为有限非负数、`updatedAt` 为数字；畸形条目丢弃不崩。再做两步收敛：
  **同 projectId 去重取 `updatedAt` 最大者**；**孤儿剪除**（projectId 在同一 blob 的 projects 里不存在则丢），
  防游标无界增长。
- `store.tsx`：hydration 并入 `progress`；新 action `upsertProgress(cursor)`（reducer 按 projectId 覆盖，
  `updatedAt` 由 action 现打 `Date.now()`）；第四个写回 effect 与现有三个同形（hydrated 后才写）。
- 写盘时刻（全部经 upsert，不加计时器节拍）：下一步/撤销（含行窗口推进时顺带刷新 `elapsedMs`）、
  模式切换、里程碑穿越、组件卸载、`pagehide`。
- **DATA-1 不回归**：写失败路径一个字不改——失败进内存、横幅常显，`PersistenceBanner` 在拼装页已挂载，保持。

### 3.3 给 B09 的迁移修正（记录，不写码）

round2-data §5.4 写的是「三个数组 put 进 `state` 仓」。本 WP 落地后是四个：迁移时 `progress` 数组
**不进 `state` 仓**，逐条 put 进 `progress` 对象仓（keyPath `projectId`，形状即 ProgressDoc 去掉可选
`doneBits`）。B09 实现者以本节为准，其余照 round2-data §5。

## 4. v0 网格来路（不接 B03、不落库）

### 4.1 形式：`apps/bead/src/fixtures/grids.ts`

按 `PatternId` 键入的行字符串画，字符 `.` = 空格，`0`–`9` = `pattern.palette` 的下标（v0 色板 ≤4 条，
`0`–`3` 够用）；配一个解码器 `fixtureGridFor(patternId): { grid: Grid; palette: readonly { code: string; hex: string }[] } | null`，
未知字符抛错（fixture 有病要在测试里死，见 T-ASM-1）。`Grid.cells` 里的 `ColorIndex` 就是
`pattern.palette` 下标——`splitSteps` 只看下标，本来就色板无关。

**展示色板适配器锁死**：`<AssembleCanvas>` 只吃 `grid + palette:{code,hex}[]`，不 import catalog。
B03 接线后 generic-5mm 下标的网格从同一口进来，画布零改动。

### 4.2 权威与一致性

**网格是颗数的权威**。一致性测试逐 fixture 断言：网格尺寸与 `board`（`square-28`→28×28，`square-56`→56×56）
相符；逐色下标计数 == `pattern.palette[i].beads`；总非空格数 == `pattern.beadCount`。
实现顺序是先画格子、后把 catalog 的 `beads`/`beadCount` 改成派生值——这**不是** drive-by：本文批准的
catalog 改动仅限这两个数字字段（以及 `navigation.test.tsx` 里对 `412` 的那一处断言随之更新，
再顺手把 catalog 头注释补一句「颗数由 grids fixture 派生」）。码号、名称、hex、标签、标题一律不动。

### 4.3 覆盖面（钉死，不做菜单）

- **必做**：`gal-slime-01`（28×28）与 `gal-arcade-05`（56×56）——每种板各证一次。
- **应做**（同机制纯手工，工期允许就补）：`gal-torii-02`、`gal-lantern-04`。
- **必不做**：`gal-cakebox-03`（立体拼豆，2260 颗 > 28×28=784 格，单板 `Grid` 根本装不下；多板拼接
  是 post-v0，round1-map §3 已推迟）。它就是「暂无网格」设计态的天然在库用例。
- 空白项目（`sourcePatternId === null`）与无 fixture 的图纸 → 同一个「暂无网格」态：一句话
  （「这个项目还没有豆图网格——上传转图归 WP-B03」）+ 框架保持（D-ASM-12），无画布、无悬浮球、无计时。
- fixture 网格全空（解出 `steps.length === 0`）→「空图纸」态，同样只留框架。

## 5. 组件树（`pages/assemble/` 内的拆分）

```
AssemblePage.tsx                守卫链：hydration → 项目 id → 项目存在（三态文案照现状）
├─ 框架（任何找得到的项目都渲染，D-ASM-12）
│   ├─ <AssembleBackdrop>       现 inline div 抽成组件；data-testid="assemble-backdrop" 与
│   │                           data-backdrop 属性、黑/白/自定义 radio 的可及名一律保持（现有测试红线）
│   ├─ <PersistenceBanner>      已有，保持挂载（DATA-1）
│   ├─ 顶栏：<h1>{project.title}</h1> + 「退出拼装」Link + Escape 监听（原样）
│   └─ 背景 fieldset（原样，可移入 HUD 设置区，但 role/name 不变）
└─ 会话区（仅当 fixtureGridFor 命中且 steps.length > 0）
    └─ <AssembleSession project grid palette cursor?>   持 reducer；卸载/pagehide 落盘
        ├─ <SessionHUD>         计时(mm:ss，≥1h 转 h:mm:ss)、BPM、第 x/N 步 + 进度%、
        │                       模式切换 radiogroup（四模式，legend「步骤模式」+ 重置提示句）、
        │                       1:1 开关 + 常显免责声明、里程碑 aria-live 区（role="status"）
        ├─ <AssembleCanvas>     CSS grid；每格 span data-cell-state="done|current|pending"；
        │                       容器 role="img" aria-label="第 x/N 步…"；1:1 时 data-scale="physical"
        ├─ <ControlOrb>         固定右下；三个 ≥44px 文字按钮：下一步 / 撤销 / 锁定当前行
        │                       （aria-pressed；逐行模式下 disabled）；撤销在 0 步禁用
        └─ 完成面板（stepIndex === steps.length）：总用时、总颗数、「标记完工」→ done + /workspace
```

纯逻辑抽 `pages/assemble/session.ts`（无 DOM，直接单测）：reducer（advance / undo / switchMode /
toggleLockRow / tick / restore 夹取）、`sessionStats(steps, stepIndex, cellsDoneInStep)`、
`beadsPerMinute`、里程碑穿越计算、行窗口切片（当前步 cells 按 y 聚类升序）。
计时显示单独成小组件，秒针 tick 不得触发 `<AssembleCanvas>` 重渲染（3136 节点每秒重排是自找的卡顿）。

### 行窗口（锁定当前行）的精确语义

- 开启时：当前步的行集合 = 步内 cells 的去重 y 升序；`slice` 从 0 起。高亮 = 步内 y == rows[slice] 的格；
  步内 y < rows[slice] 的格按 done 渲染，y > 按 pending。
- 「下一步」：`slice+1`；已是最后一行 → `stepIndex+1, slice=0`（开关保持开启）。
- 「撤销」：`slice>0 → slice-1`；`slice==0 → stepIndex-1, slice=0`（整步回退——持久化粒度就是步）。
- `slice` 永不落盘；刷新回到步首（round2-data §3 已接受的精度损失）。
- `placedBeads` 计入当前步已完成行的格数（喂 BPM 与里程碑）；关闭开关或切模式时 `slice=0`。
- 逐行扫描模式：步即行，按钮 disabled 并给 title「逐行模式本身即按行」。

### 1:1 模式的数字

格距 5mm（generic-5mm 的物理规格）、gap 0、格描边收进格内（inset box-shadow）不得加尺寸；
容器 `overflow: auto` 两轴。免责声明固定字符串，必含「显示器未校准」。非 1:1 的默认格尺寸由实现者
在「1280×800 视口下 56×56 整板无滚动可见」的约束内自选（布局约束，不进 jsdom 测试）。

## 6. a11y 增量（round1-frontend §6 全部沿用）

- 颜色永不做唯一信号：HUD 的当前步行显示「色号 + 名称」文本（单色模式）或「第 y 行 / 第 n 块 ·
  outline/inner-edge/fill 中文名」（其余模式）；`inner-edge` 显示为「内边界」。
- 里程碑与「色号完成」走同一个 `role="status"`（polite）live 区，动效尊重 `prefers-reduced-motion`
  （reduce 时只换文本不放动画）。
- 悬浮球是常规按钮簇不是无名浮层：`aria-label="拼装控制"` 容器 + 三个文字按钮；锁行用 `aria-pressed`。
- 画布格子 `aria-hidden`，语义汇总在容器 aria-label（随步更新）；格子不可点（v0 无点选交互，§9）。

## 7. 测试矩阵（Vitest + 现有 `renderApp` harness；纯逻辑不进 DOM）

| # | 层 | 断言 |
|---|---|---|
| T-ASM-1 | fixture | 每份网格：字符全合法、尺寸符合 board、逐色计数 == catalog `beads`、总数 == `beadCount`（4.2 的一致性钉） |
| T-ASM-2 | 纯 | `sessionStats`：placed/total/fraction；含行窗口部分计数；空步数组返回全零不除零 |
| T-ASM-3 | 纯 | `beadsPerMinute`：正常值四舍五入；placed==0 或 elapsed<1000 → null |
| T-ASM-4 | 纯 | reducer：advance/undo 边界（0 步禁退、末步进完成态）、switchMode 重置 stepIndex 且 elapsedMs 不变、行窗口 slice 推进/跨步/回退语义（§5） |
| T-ASM-5 | 纯 | 里程碑：向上穿越各报一次；恢复进场把 ≤当前分数的里程碑预标已报；撤销后再穿越不重报 |
| T-ASM-6 | 仓库 | `parsePersistedState` 第四键：畸形丢弃、孤儿剪除、同 id 取 updatedAt 最大；load/save 往返 |
| T-ASM-7 | 仓库 | **BD19 红线**：落盘后的游标条目键集合恰为五键；序列化 blob 的 progress 段不含 `cells`/`steps`/`doneBits` |
| T-ASM-8 | 集成 | 默认进场（有网格）：当前步格 `data-cell-state="current"` 数量 == steps[0].cells.length，其余 pending |
| T-ASM-9 | 集成 | 悬浮球下一步/撤销移动高亮；`→`/`←` 等价；焦点在背景色输入框时按键不触发 |
| T-ASM-10 | 集成 | 模式切换 radio → stepIndex 归 0，`repository.loadProgress()` 里 mode 已更新（D-ASM-4） |
| T-ASM-11 | 集成 | 恢复：seed progress（row-by-row, stepIndex=2）→ 对应 radio 选中、第 3 个非空行为 current |
| T-ASM-12 | 集成 | 夹取：seed stepIndex=999 → 完成面板，不崩；「标记完工」→ 项目 status=="done" 且回 /workspace |
| T-ASM-13 | 集成 | 计时（fake timers）：显示走秒；`document.hidden` + visibilitychange 暂停、复显续走 |
| T-ASM-14 | 集成 | 单色模式步进 → status 区含「色号 …」完成播报；tile 模式无此播报（D-ASM-9） |
| T-ASM-15 | 集成 | 1:1 开关：`data-scale="physical"` 出现、免责声明含「未校准」；关闭后移除 |
| T-ASM-16 | 集成 | 锁行：单色模式开启后 current 只剩一行；逐行模式按钮 disabled |
| T-ASM-17 | 集成 | 暂无网格态（`sourcePatternId:null` 与 gal-cakebox-03 项目各一）：框架在（h1/退出/背景 radio），无悬浮球无计时 |
| T-ASM-18 | 集成 | DATA-1 不回归：`flaky-storage` 仓库下拼装页横幅常显，步进仍可用（内存态） |
| T-ASM-19 | 既有 | `navigation`（含 Escape 退出）、`theme-backdrop` 三条、`empty-states` 守卫、`persistence-banner`、`isolation`（NE-1）全绿——除 4.2 批准的那一处 412 断言外**零改动** |

## 8. 明确非目标（实现者不得顺手做）

| 内容 | 归属 |
|---|---|
| 语音 / 手势 | PLAN 明文不进 v0（BD9） |
| `/create` 上传接线、pipeline 进拼装 | WP-B03；接线 PR 同时背 DATA-3 |
| IndexedDB / `doneBits` 逐格进度 / 持久化 `Grid` | BD19：首个持久化 Grid 的 PR 上 `bead-v1`，非本 WP |
| rust `bead-core` 任何改动、Soul 树任何改动 | 禁区 |
| 六角 / 圆板、多板拼接（含 gal-cakebox-03 的网格） | post-v0（catalog 注释与 round1-map §3 已钉方板） |
| Workspace / PersonalStrip 的进度与用时接线（两处 stub 注明归 B04） | 游标落库后是廉价后续，但不进本 WP 验收——避免 B04 摊大；stub 文案由做那件事的 PR 更新 |
| 格子点选 / 点击置豆、悬浮球拖动、单色模式降序切换 UI、手动暂停键、屏幕校准工具 | v0 减脂；降序在 `splitSteps` 已支持，UI 暴露待有真实诉求 |

## 9. 残余风险

- **R-ASM-1 · fixture 作画工作量**：56×56 手画 3136 字符不轻。缓解：4.3 只把两张钉成必做，
  且网格为权威、catalog 数字跟着改，不要求逆向凑 412。
- **R-ASM-2 · DOM 画布性能**：3136 节点逐步换属性没问题，但秒针 tick 若波及画布会卡——§5 已把
  计时显示隔离成组件，列为实现要求。512² 时代换 canvas，边界已留在 `<AssembleCanvas>`。
- **R-ASM-3 · CSS mm 失真**：浏览器按 96dpi 折算，高分屏缩放下偏差可到两位数百分比——免责声明
  与「尺量 28 格 ≈140mm」就是 v0 的全部对策（round1-frontend R2 维持未解）。
- **R-ASM-4 · catalog 数字联动**：4.2 批准面收得很窄（两个数字字段 + 一处测试断言）；越界即 drive-by。
- **R-ASM-5 · 多标签页**：游标同受 last-writer-wins（DATA-5 已知边界），不另设防。

## 10. Ready-for-Opus 清单

1. 新建：`fixtures/grids.ts`（+ `grids.test.ts` ≙ T-ASM-1）、`pages/assemble/session.ts`（+ 单测 T-ASM-2…5）、
   `pages/assemble/{AssembleSession,AssembleCanvas,SessionHUD,ControlOrb}.tsx`。
2. 修改：`stores/types.ts`（§3.1）、`stores/repository.ts`（§3.2 两实现 + parse）、`stores/store.tsx`
   （hydrate + upsertProgress + 第四 effect）、`AssemblePage.tsx`（守卫链 + 框架不变量 D-ASM-12）、
   `app/app.css`（三态、orb、1:1）；`fixtures/catalog.ts` 仅 4.2 批准的数字；`navigation.test.tsx` 仅 412 一处。
3. 顺序建议：类型与仓库（§3）→ fixture 网格（§4）→ session.ts 纯逻辑 → 组件 → 集成测试。
4. 全程红线：新源码零外网 URL（BD15 / NE-1 会扫）、零新运行时依赖（D-UI-7）、`Step[]`/`Grid` 不落盘
   （T-ASM-7 钉死）、backdrop 的 testid / data 属性 / radio 可及名不动（T-ASM-19）。
5. 验收命令：`pnpm --filter @bead/app test` 全绿 + 仓库根 `just ci` 绿（共存回归）；PR 描述引用本文
   D-ASM 编号说明每条落点。

## 11. NO_HIGH_VALUE_CHANGE_FOUND 子树

- 游标进 URL / 可分享拼装深链：双权威 + 无分享目标（round1-frontend §8 已判）。NO_HIGH_VALUE_CHANGE_FOUND。
- 四模式游标各存一份：违 BD19，且跨模式恢复语义不成立（D-ASM-4）。NO_HIGH_VALUE_CHANGE_FOUND。
- 为 56×56 引入虚拟化/canvas 渲染：数量级不需要，测试面变差（D-ASM-6）。NO_HIGH_VALUE_CHANGE_FOUND。
- 屏幕 DPI 校准向导：v0 免责声明 + 尺量足够，校准 UI 是新失败面。NO_HIGH_VALUE_CHANGE_FOUND。
- 会话历史/多段用时统计（每次进出记一段）：`elapsedMs` 单调累计已满足 PLAN 的「用时」。NO_HIGH_VALUE_CHANGE_FOUND。

## 12. 禁区自查

本轮只新增 `docs/bead/reviews/round2-assemble.md`。未改 `apps/bead/**` 任何产品文件（全部实现留给
WP-B04 实现席），未触碰 `docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、
`apps/desktop/**`、`crates/soul-*/**`、根 Cargo members、`deny.toml`、`.github/**`。本文无外网 URL，
仅引用仓内路径（BD15）。
