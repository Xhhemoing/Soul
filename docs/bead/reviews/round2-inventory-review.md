`claude-fable-5-thinking-xhigh`

# ROUND 2 · WP-B05 库存与清单 · 实现审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审查，不实现）。
被审对象：合并提交 `8526c5c`（merge: absorb WP-B05 inventory shortages and substitutes），基线 `origin/cursor/beadflow-integration-c441`。
审查依据：`docs/bead/reviews/round2-inventory.md`（D-INV-1…14 / T-INV-1…15）、`docs/agent-decisions.md`（BD15 / BD19 / BD20）。
本轮只新增本文件；未改任何产品文件、测试文件、rust、Soul 树。

---

## 0. 结论先行

**接受。无 HIGH 缺陷。** 合并面 13 个文件全部落在 `apps/bead/src/**`，与 IA 的 Ready-for-Opus 清单（§10）逐项对得上；
BOM 真源 = `pattern.palette`、余量制替代、空库存全量缺口、双通道导出、DATA-1 不动摇——五个钉子全部成立且有测试锁死。
`pnpm --filter @bead/app test` 实测 **27 个文件 / 358 条全绿**（预期 358，命中）。
下面列 1 条 MED 与 3 条 LOW，均不构成回退理由。

## 1. 逐项核对（任务钉出的八个检查点）

### 1.1 BOM 真源是 `pattern.palette`，`buildBom` 一行不调 ✅

- `stores/inventory.ts` 的 `selectRequirements` 只读 `selectInProgress` ∩ `sourcePatternId !== null`
  项目的来源图纸 `palette`，按码字符串精确聚合；同图纸双实例需求翻倍（T-INV-2 钉住 252 = 126×2）。
- 全树 grep：`buildBom` 的调用点只有 `algo/pipeline.ts` 与 `algo/` 自己的测试，库存流零调用。
  头注释原文点名 D-INV-1 / BD19 / BD20，把「为什么不是 buildBom」写在了会被下一个人读到的地方。
- T-INV-15 落地：`Σ palette.beads === beadCount` 对全部 PATTERNS 断言，另加同码跨图纸 name/hex
  一致性检查——B08 扩画廊时这两条就是门禁（R-INV-2 缓解到位）。

### 1.2 无 generic-5mm G 码进库存；无命名空间字段 ✅

- 库存条目全部来自用户表单或 seed，代码里没有任何把 `GENERIC_5MM` 条目灌进库存流的路径；
  `pages/inventory/**` 与 `stores/inventory.ts` 全文无 `generic` 引用（grep 证实，仅注释提及以说明为什么不用）。
- `types.ts` 的 `InventoryEntry` 仍是 `{code, name, hex, beads}` 四字段，无 namespace / palette 字段
  （D-INV-3 / D-INV-4 ✅；BD20 触发点前不预埋，正确克制）。

### 1.3 空库存出全量缺口；empty-states.test 改写而非删除 ✅

- `selectShortages` 对空库存把 `inStock` 落 0、`shortage = required`，不设短路（T-INV-3 纯函数面 +
  T-INV-10 组件面「库存为空仍然出全量缺口」双钉，且断言导出 textarea 在场）。
- `app/empty-states.test.tsx` 的 Inventory describe **保留并改写**为两条：库存零态指回页顶表单
  （断言无 link、有「加入库存」按钮），零库存时缺口区照常出行（lantern-04 的 152/112/76 三行 +
  三个「库存 0 颗」）。stub-note 断言删除正确（stub 已不存在）；其余 describe 块零改动。
- 库存区零态文案「还没有库存记录：先在上方录入色号和颗数」与 §7 逐字一致，无 CTA。

### 1.4 替代语义：余量制 + 严格 ΔE00 < 3 + 绝不只给颜色 ✅

- `selectSubstituteGroups`：池按库存原序构建，`remaining = max(0, beads − 自身精确需求)`，
  `remaining <= 0` 与同码自身都不进池；hex 解析失败的条目落出池外但仍在 StockList 展示。
  调 `algo/substitutes.ts` 的 `findSubstitutes`（严格 `deltaE < 3`，ΔE 升序 → 下标升序），未复制阈值逻辑。
- T-INV-4 用与 `algo/substitutes.test.ts` 同一对冻结色（ΔE00 2.9952 / 3.0012）卡阈值两侧，
  另钉「同码自身不进池但仍抵扣库存」「remaining 扣到 0 出局」「同 ΔE 按库存原序」三条边界。
- 展示面：候选行 = ColorSwatch（码+名内建，`ColorSwatch` 组件本身不允许裸色块）+ hex 文本 +
  `ΔE00 x.xx` + `余 N 颗`；无候选的组显式给「库存内没有 ΔE00 < 3 的替代」（T-INV-11 双条钉住）。

### 1.5 导出文本无 URL；无 Clipboard API ✅

- `buildPurchaseText` 输出对给定 state 完全确定（无时间戳），首行、逐行四要素（码/名/hex/颗数）、
  缩进替代行、合计行与 §4 格式一致；T-INV-5 做整串断言 + `not.toMatch(/http|:\/\//)`。
- 交付双通道 = 只读 textarea + `<a download>`（Blob + `URL.createObjectURL`，运行时 blob:）；
  全树 grep 无 `navigator.clipboard` / Clipboard API 调用（唯一命中是 ShortagePanel 注释里的
  「deliberately absent」）。无缺口时 `ShortagePanel` 在到达导出控件前就返回零态（T-INV-10/12 钉住）。
- NE-1（isolation.test.ts）递归扫全部 `src/**/*.ts(x)`，新文件自动入网，358 条全绿即证无出网字面量。

### 1.6 DATA-1 写失败路径原样 ✅

- `repository.ts` 的合并 diff 恰为一处：`isInventoryEntry` 收紧为
  `code:string ∧ name:string ∧ hex:string ∧ Number.isInteger(beads) ∧ beads ≥ 0`——与 §5 末条逐字一致。
  `markFailed` / `read` / `write` / fallback 语义一个字未动。
- T-INV-13：`FlakyStorage.full = true` 下横幅在（恰一条）、录入与删除仍改 UI、无第二错误面。
  T-INV-14 落在 `repository.test.ts`：缺 hex / NaN / 小数 / 负数 / 缺码 / null / 字符串共 8 种畸形
  条目全部丢弃、好条目留下。既有 `persistence-banner.test.tsx` 零改动保绿。

### 1.7 层向：algo/ 不 import stores ✅

- `algo/**` 全目录 grep 无任何 `stores/` import；合并未触碰 `algo/` 任何文件。
  `parseHexColor` 按 D-INV-10 落在 `stores/inventory.ts`（stores → algo 单向），表单与替代池共用同一实例。

### 1.8 测试口径 ✅

```
pnpm --filter @bead/app test
Test Files  27 passed (27)
     Tests  358 passed (358)      ← 预期 358，命中
```

## 2. D-INV-1…14 落点速查

| ID | 判定 | 落点 |
|---|---|---|
| D-INV-1 | ✅ | `selectRequirements` 只读 palette；`buildBom` 零新调用；空白项目 continue |
| D-INV-2 | ✅ | `selectInProgress`（todo/active）∩ 有来源；码精确聚合；双实例翻倍（T-INV-2） |
| D-INV-3 | ✅ | 无命名空间字段；G 码不进库存流；BD20 注释点名触发点 |
| D-INV-4 | ✅ | 四字段不变；normalizeCode trim+大写；≤16 / ≤64 / 严格 hex / 0–99999 全在表单钉住 |
| D-INV-5 | ✅ | `setInventory` 整个删除，换三个细粒度 action；持久化走既有 save effect，零新管线 |
| D-INV-6 | ✅ | 行内只改 beads；重复码内联报错「色号已存在，请直接修改颗数」+ reducer no-op 兜底 |
| D-INV-7 | ✅ | 删除即时、无确认层；按钮可及名「删除 R04」（T-INV-8） |
| D-INV-8 | ✅ | 空库存全量缺口 + 导出可用；库存零态新文案；empty-states 改写不删 |
| D-INV-9 | ✅ | 余量制、严格 <3、ΔE 升序+池序、码+名+hex+ΔE+余量全要素展示 |
| D-INV-10 | ✅ | `parseHexColor` 在 stores/inventory.ts；层向单向；algo 未改 |
| D-INV-11 | ✅ | textarea + `<a download>`；无 Clipboard；文件名 `bead-purchase-list.txt`；无缺口不渲染 |
| D-INV-12 | ✅ | 缺颗降序 + 码 localeCompare 升序（T-INV-3 第四条） |
| D-INV-13 | ✅ | 页面全部 useMemo 现算；派生类型留在 inventory.ts 未进 types.ts |
| D-INV-14 | ✅ | 零新错误 UI；repository 失败语义原样；T-INV-13 |

T-INV-1…15 全部落地：纯函数面在 `stores/inventory.test.ts`（24 条），组件面在
`pages/inventory/inventory.test.tsx`（12 条），T-INV-14 扩进 `repository.test.ts`，
T-INV-15 在 inventory.test.ts 尾部。Do-not-regress 套件（navigation / persistence-banner /
isolation / empty-states 其余块）零改动全绿。

## 3. BD 红线

- **BD15**：新增源码与采购文本零外网 URL 字面量（NE-1 扫描 + T-INV-5 双保险）；app.css 新增 123 行无 `url()` 引外。
- **BD19**：本 WP 未持久化任何 Grid / 派生数据；库存仍走 localStorage 整包既有管线，未引 IndexedDB。
- **BD20**：两套码空间未见面；`buildBom` 无新调用；无预埋字段。
- **零新依赖**：合并未触碰任何 `package.json` / lockfile。
- **禁区**：13 文件全在 `apps/bead/src/**`；`algo/**`、`fixtures/catalog.ts`、`schema/**`、路由表、
  `ColorSwatch` 签名、rust、Soul、`.github` 零触碰。

## 4. 发现（按严重度）

### HIGH — 无

### MED

- **MED-1 · blob URL 在 `useMemo` 里铸造（渲染期副作用）**。`ShortagePanel.tsx` 的 `PurchaseExport`
  在 `useMemo(() => createDownloadHref(text), [text])` 里调 `URL.createObjectURL`。渲染必须幂等：
  StrictMode（`main.tsx` 已开）下开发态每次挂载双跑渲染，第一份 blob URL 无人回收；React 的
  memo 缓存本身也是「可随时丢弃重算」的契约，丢一次漏一个。已提交的 href 由 effect cleanup 正确
  revoke，泄漏面只有未提交渲染的小文本 blob（页面生命周期内、KB 级），生产态当前无症状——
  所以不是 HIGH。修法几行：在 effect 里铸造并 setState，或点击时现铸。留给后续小 PR。

### LOW

- **LOW-1 · 手改 localStorage 的重复码条目无跨条目去重**。`isInventoryEntry` 按 §5 末条逐字实现
  （逐条形状检查），但 hydration 不做码去重：同码两条会让 `StockList` 出重复 React key、
  `setInventoryBeads` 同时改两行、替代池对每行各扣一次全额自身需求。reducer 侧 `addInventoryEntry`
  的 no-op 挡住了 UI 入口，只有手改 blob 能造出来——而本仓对手改 blob 的既定态度是「边界要设防」
  （repository 头注释）。IA 未要求，属 IA 缝隙带进实现；修法是 load 时按码保首条。
- **LOW-2 · StockList 颗数输入清空即写 0**。`onChange` 直接 `Number(event.target.value)`，
  清空输入框的瞬间 `""` → 0 入库，缺口面板闪一次全量缺口，且受控值回填 `0` 后续键入变成 `"040"`
  这类前导零（`Number` 化后无损，测试可过）。纯 UX 毛刺，无数据损坏；修法是本地草稿态 + blur 提交。
- **LOW-3 · reducer 对 hex / name 无兜底**。`addInventoryEntry` 在 reducer 层归一了 code、夹取了
  beads，但 hex / name 原样透传——防御不对称：绕开表单的程序化调用可存入 `hex: "红色"`。
  下游已免疫（替代池丢弃解析失败的 hex、StockList 只当文本渲染），且表单是唯一调用方，
  所以只是内部一致性瑕疵。

### 观察（不计数）

- IA 文档 §2.1 的 palette 数字表（slime 412 = 168+96+84+64 等）是 B04 合并前的旧 fixture 值；
  B04 已把有网格图纸的数字改为网格派生（slime 300 / lantern 340 / arcade 1238）。实现正确地对
  **现值**断言且 T-INV-15 锁的是不变量本身而非具体数字，无产品缺陷——但 IA 文档如再被引用，
  §2.1 与 T-INV-2 示例数字（"B05 = 64+300"）已过期。
- `createDownloadHref` 对缺失 `createObjectURL` 的守卫看似防御过度，实为 load-bearing：
  `empty-states.test.tsx` 在未打 stub 的 jsdom 里渲染带缺口的 `/inventory`，没有这层守卫会崩。
- 在拼项目的 `sourcePatternId` 指向已下架图纸时静默跳过，与空白项目共用「没有配色清单来源」文案
  ——语义可辩，`selectRequirements` 有对应测试（「来源图纸已下架时跳过而不是崩」）。
- 替代量不做跨缺口预留（两个缺口色同时看中一个候选的余量），与 R-INV-3 的已知边界一致，非缺陷。

## 5. 验收记录

```
pnpm --filter @bead/app test
Test Files  27 passed (27)
     Tests  358 passed (358)
  Duration  9.45s
```

审查环境：`cursor/bead-r2-b05-review-c441` @ 基于 `8526c5c`（即最新 `cursor/beadflow-integration-c441`）。

## 6. 禁区自查

本轮只新增 `docs/bead/reviews/round2-inventory-review.md`。未改 `apps/bead/**`、
`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、`apps/desktop/**`、
`crates/soul-*/**`、根 Cargo members、`deny.toml`、`.github/**`。本文无外网 URL（BD15）。
