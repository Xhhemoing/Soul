# ROUND 2 · WP-B05 库存与清单 — 前端 IA

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只做 IA / 拆分，不实现）。
基线：`origin/cursor/beadflow-integration-c441` @ `4ae4827`。
输入：`docs/bead/PLAN.md`（备料 / 资产）、`docs/bead/WORK_PACKAGES.md`（WP-B05）、`round1-frontend.md`（D-UI-5 / 空态清单 / §5 状态所有权）、
`round1-map.md`（WP-B05 精化条）、`round2-data.md` 与 `docs/agent-decisions.md` **BD19**、`round1-core-review.md`（rust `check_stock` 语义）、
`apps/bead/src/pages/inventory/InventoryPage.tsx`（stub）、`stores/inventory.ts` / `types.ts` / `repository.ts` / `store.tsx` / `projects.ts` / `catalog.ts`、
`algo/bom.ts` / `substitutes.ts` / `palette.ts` / `color.ts`、`fixtures/catalog.ts`、`app/empty-states.test.tsx`、`components/`（ColorSwatch / EmptyState / PersistenceBanner）。
硬禁区自查：本轮只新增本文件；未触碰 `apps/bead` 产品文件、Soul 锁、`apps/desktop`、`crates/soul-*`、根配置、`deny.toml`、`.github/**`。

---

## 0. 结论先行

1. **v0 BOM 真源 = 来源图纸的 `pattern.palette`**（D-INV-1）。五张 fixture 的 `Σ palette[].beads === beadCount` 逐张核实成立，
   palette 本身就是精确 BOM；`buildBom` 在 B05 里**一行都不调**——没有可用的 Grid（BD19 禁止为此持久化），
   且它输出 `generic-5mm` 的 G 码，与画廊码空间冲突（`G07` 实撞，§2.3）。
2. **空库存也算缺口**（D-INV-8）。零库存想要采购清单是「备料」的第一用例，缺口 = 全部需求是正确输出；
   round1 空态表的「录入库存后才能做缺口预警」文案由本轮**取代**，同 PR 更新既有测试，不许删测。
3. **替代语义对齐 rust oracle 的余量制**（D-INV-9）：候选池 = 扣掉自身需求后余量 > 0 的库存条目，排除同码自身，
   严格 ΔE00 < 3，ΔE 升序 + 库存下标升序；展示永远 码 + 名 + hex + ΔE，绝不只给颜色。

## 1. 决策表（D-INV-*，实现者照抄，不自行发明产品法）

| # | 决策 | 内容 |
|---|---|---|
| D-INV-1 | BOM 真源 | v0 = 来源图纸 `pattern.palette`（码/名/hex/颗数俱全且总和已验证）。`sourcePatternId === null`（空白项目）无 BOM，排除出缺口计算。`buildBom` 保留给 BD19 触发点之后的转换项目，B05 不调用 |
| D-INV-2 | 缺口范围 | `selectInProgress`（status ∈ {todo, active}）的全部项目，按**码字符串精确相等**聚合需求；同一图纸实例化两次需求翻倍（拼两份就是要双份豆）。draft / done 不计 |
| D-INV-3 | 码空间 | v0 单一扁平码空间 = fixture 画廊码。`generic-5mm` 的 G 码 v0 不进库存流（转换项目尚无持久 Grid）。已知冲突：画廊 `G07 苔绿 #4c7a44` vs generic-5mm `G07 Silver #B7BFC6`——**第一个把 Grid 持久化的 PR（BD19 触发点）必须同时给库存与 BOM 引入色板命名空间字段**，在那之前不加字段 |
| D-INV-4 | `InventoryEntry` 形状 | 不变：`{code, name, hex, beads}` 四字段，不加字段不迁移。码 = 身份键，录入边界 trim + 大写归一，≤16 字符；name ≤64 字符（可空串）；hex 必填且必须过 `parseHexColor`（严格 `#rrggbb`，存储归一为小写带 `#`）；beads 为 0–99999 整数（配额 DoS 上限，沿 round2-data §6.3 先例） |
| D-INV-5 | 存储与动作 | 留在 localStorage 整包（BD19：不引 IndexedDB）。`store.tsx` 用三个细粒度 action **替换**现有无调用方的 `setInventory`：`addInventoryEntry(entry)` / `setInventoryBeads(code, beads)` / `removeInventoryEntry(code)`。持久化走既有 save effect，零新管线；页面仍不直碰存储 |
| D-INV-6 | 编辑范围 | 行内只可改 `beads`；改码/名/hex = 删除后重录（码是身份，原地改码等于带合并语义的重命名，v0 不做）。新增遇重复码：表单内联报错「色号已存在，请直接修改颗数」，不静默合并；reducer 对重复码 no-op 兜底 |
| D-INV-7 | 删除 | 立即删除，无确认弹层（重录成本一行表单；round2-data §7 的零弹窗面照顾的是浏览器权限，不禁 in-app confirm，但 v0 不值得加） |
| D-INV-8 | 空库存缺口 | 缺口面板不因库存为空而停摆：库存视为 0，缺口 = 全部需求。库存区自身零态文案改为「还没有库存记录：先在上方录入色号和颗数」（表单常驻，不再链去 `/create`）。`empty-states.test.tsx` 的 Inventory 断言同 PR 更新 |
| D-INV-9 | 替代语义 | 逐缺口色计算。候选池按库存原序构建：`remaining = beads − 该码自身精确需求`（下限 0），只收 `remaining > 0` 且码 ≠ 缺口码的条目；调 `findSubstitutes`（严格 < 3，G8），排序 ΔE 升序 → 池下标升序（池保持库存原序，等价 G6 tie-break）。展示：ColorSwatch（码+名+色块）+ hex 文本 + `ΔE00 x.xx`（两位小数）+ `余 N 颗` |
| D-INV-10 | hex→Rgb 桥 | 新纯函数 `parseHexColor(hex): Rgb \| null` 放 `stores/inventory.ts`（表单校验与替代池共用）。层向：stores 可 import `algo/`（纯函数），`algo/` 永不 import stores；不复用 `pages/assemble/backdrop.ts`（页面私有），不改 `algo/` 任何文件 |
| D-INV-11 | 导出采购文本 | 纯文本。交付双通道：只读 `<textarea>`（手动全选复制）+ `<a download>`（`Blob` + `URL.createObjectURL`，运行时 blob: 不是出网 URL 字面量，BD15 安全）。**不用** Clipboard API（round2-data §7 零权限弹窗）。无缺口时不渲染导出控件 |
| D-INV-12 | 缺口排序 | 缺颗数降序，同数按码字典序升序（镜像 rust BOM 的 count 降序 + code 升序惯例） |
| D-INV-13 | 派生数据 | BOM、需求聚合、缺口、替代、采购文本一律 selector 现算，**禁止持久化**（IA §5 / round2-data §5.2 原样延伸） |
| D-INV-14 | 写失败 | 不新增任何错误 UI：AppShell 里的 `PersistenceBanner`（DATA-1）已全局覆盖 `/inventory`；写失败后 CRUD 继续在内存态可用，横幅说明不落盘。禁止 per-action toast，禁止动 repository 的失败语义 |

## 2. BOM 真源论证（D-INV-1 的证据）

### 2.1 palette 即精确 BOM（逐张核实）

| 图纸 | `Σ palette[].beads` | `beadCount` |
|---|---|---|
| gal-slime-01 | 168+96+84+64 = 412 | 412 ✓ |
| gal-torii-02 | 520+380+340+300 = 1540 | 1540 ✓ |
| gal-cakebox-03 | 900+760+360+240 = 2260 | 2260 ✓ |
| gal-lantern-04 | 320+250+190 = 760 | 760 ✓ |
| gal-arcade-05 | 420+330+230 = 980 | 980 ✓ |

这个不变量要用测试锁死（T-INV-15），防 WP-B08 扩画廊时把它拆散。

### 2.2 为什么不是 `buildBom(内存 fixture grid)`

- fixture 图纸**没有**逐格网格，为 BOM 平权而发明五张网格 = 与 `beadCount` 双写同一事实，必漂移；
- `buildBom` 的行是 `generic-5mm` 索引 → G 码，而详情页、库存录入、用户手里的豆都在画廊码空间——同一颗豆两套码；
- BD19 明令 Grid 持久化之前不上 IDB，内存 grid 还得每次重建，纯支出。

### 2.3 触发点（与 BD19 同一时刻）

第一个持久化 `Grid` 的 PR（B03 `/create` 接线或 B07 导入）让转换项目有了真网格，那个 PR 必须：
① 转换项目 BOM 改走 `buildBom(grid, GENERIC_5MM)`；② 给 `InventoryEntry` 与需求行引入色板命名空间字段并迁移；
③ 解除 `G07` 冲突。在那之前 B05 不为此预留任何字段（D-INV-3、D-INV-4）。

## 3. 缺口计算（纯函数，全部进 `stores/inventory.ts`）

```ts
interface RequirementRow { code: string; name: string; hex: string; required: number }
interface ShortageRow   { code: string; name: string; hex: string; required: number; inStock: number; shortage: number }
interface SubstituteGroup {
  wanted: ShortageRow;
  candidates: { code: string; name: string; hex: string; deltaE: number; remaining: number }[];
}

parseHexColor(hex: string): Rgb | null
normalizeCode(code: string): string           // trim + toUpperCase
selectRequirements(projects: readonly Project[], lookup?: (id: string) => Pattern | undefined): RequirementRow[]
selectShortages(requirements: readonly RequirementRow[], inventory: readonly InventoryEntry[]): ShortageRow[]
selectSubstituteGroups(shortages, requirements, inventory): SubstituteGroup[]
buildPurchaseText(shortages: readonly ShortageRow[], groups: readonly SubstituteGroup[]): string
```

- `selectRequirements`：过滤 `selectInProgress` ∩ `sourcePatternId !== null`，`lookup` 默认 `catalog.findPattern`（注入点只为测试）；
  按码聚合求和，名/hex 取首次出现（画廊内部一致，T-INV-15 锁）。
- `selectShortages`：`shortage = max(0, required − inStock)`，`inStock` 按码精确匹配；只输出 `shortage > 0` 的行，排序按 D-INV-12。
- 替代池 `remaining` 里的「自身需求」也来自 `requirements`（rust `check_stock` 同义：先扣自用再当候选，round1-core-review §2 已核）。
- hex 解析失败的库存条目（手改 localStorage 的残留）不进替代池，但仍在库存列表展示（码文本仍在，不致盲）。
- 这些类型是派生形状，**留在 `inventory.ts`**，不进 `types.ts`（types.ts 只放持久化形状，防有人顺手落库）。

## 4. 导出采购文本（D-INV-11 的格式锁）

行文法（每字段必须在场，措辞可微调但测试要锁四要素 码/名/hex/颗数）：

```
拼豆采购清单（正在拼 / 待拼项目 vs 当前库存）

R04 朱红 #d8412f 缺 230 颗
  可替代：G12 Rose #f0559a（ΔE00 1.24，库存余 96 颗）
Y01 明黄 #f5d13b 缺 250 颗

合计缺 480 颗，共 2 个色号
```

- 内容对给定 state 完全确定（无时间戳），测试可做整串断言；文件名 `bead-purchase-list.txt`。
- 文本里**不得**出现任何 URL / 商购链接（BD15）；T-INV-5 直接断言不含 `http` 与 `://`。

## 5. 组件树与文件落点

```
InventoryPage                                  pages/inventory/InventoryPage.tsx（重写 stub）
├─ <section aria-label="色号库存">
│   ├─ <StockForm>                             pages/inventory/StockForm.tsx
│   │     码 / 名称 / hex / 颗数 + 内联错误（aria-describedby）
│   └─ <StockList> → <StockRow>*               pages/inventory/StockList.tsx
│         ColorSwatch + 颗数编辑（number input）+ 删除按钮（可及名含码：「删除 R04」）
│         零态：EmptyState「还没有库存记录：先在上方录入色号和颗数」（无 CTA，表单就在上方）
├─ <section aria-label="缺口预警">
│   └─ <ShortagePanel> → <ShortageRow>*        pages/inventory/ShortagePanel.tsx
│         需求 / 库存 / 缺口 三数并示 + 导出控件（textarea + 下载）
└─ <section aria-label="近似色替代">
    └─ <SubstitutePanel>                       pages/inventory/SubstitutePanel.tsx
          按缺口色分组：wanted ColorSwatch → 候选行*（码+名+hex+ΔE00+余量）
```

- 三个 `<section aria-label>` 与 stub 完全同名，round1 组件树的三面板结构（StockList / ShortagePanel / SubstitutePanel）不动摇；
  子组件按 explore 页先例放页面目录，不进 `components/`（无第二消费者）。
- 复用 `ColorSwatch`（码文本内建，谁都渲染不出裸色块）、`EmptyState`、`Card`；样式加在 `app.css`，不引任何 UI 依赖（D-UI-7）。
- `store.tsx` 增三 action（D-INV-5）；`repository.ts` 把 `isInventoryEntry` 收紧为
  `code: string ∧ name: string ∧ hex: string ∧ Number.isInteger(beads) ∧ beads ≥ 0`（丢弃不崩溃，沿 SH-4 先例）。

## 6. 状态所有权增量（round1 §5 表的 delta）

| 状态 | 拥有者 | 持久化 | 写者 | 读者 |
|---|---|---|---|---|
| 库存条目 | InventoryStore（store.tsx reducer） | Repository（localStorage 整包，不变） | 三个细粒度 action（仅 `/inventory` 表单触发） | StockList / 缺口 / 替代 selector |
| 需求 / 缺口 / 替代 / 采购文本 | 无（selector 现算） | **禁止** | — | ShortagePanel / SubstitutePanel / 导出 |
| 表单草稿与内联错误 | StockForm 页内 state | 不持久化 | 表单 | 表单 |

## 7. 空态 / 错误 / 写失败矩阵

| 位置 | 条件 | 呈现 | 出路 |
|---|---|---|---|
| 库存区 | 无条目 | 「还没有库存记录：先在上方录入色号和颗数」 | 表单常驻页顶 |
| 表单 | 码重复 / hex 非法 / 颗数越界 | 字段内联错误，条目不入库 | 改后重提 |
| 缺口区 | 无 todo/active 项目 | 「没有正在拼或待拼的项目，先去挑一张图纸」 | CTA →`/explore` |
| 缺口区 | 在拼项目全是空白项目 | 「当前项目没有配色清单来源，无法计算缺口」 | 无 CTA |
| 缺口区 | 有 BOM 且库存全覆盖 | 「库存足够，当前没有缺口」；不渲染导出控件 | — |
| 缺口区 | 库存为空 | 正常出全量缺口行（D-INV-8） | 导出可用 |
| 替代区 | 无缺口 | 「没有缺口时不需要替代建议」 | — |
| 替代区 | 某缺口色无 ΔE00<3 候选 | 该组显式一行「库存内没有 ΔE00 < 3 的替代」 | 只能采购 |
| 全页 | 写失败（DATA-1） | 既有 `PersistenceBanner` 常显，CRUD 继续内存态可用 | 不新增 UI，不得回退 |

## 8. 验收测试矩阵（T-INV-*）

纯函数（`stores/inventory.test.ts`，新建）：

| # | 断言 |
|---|---|
| T-INV-1 | `normalizeCode` trim+大写；`parseHexColor` 收严格 6 位（可带 `#`），拒 3 位 / 非法字符，输出归一 |
| T-INV-2 | 需求聚合：两项目共享色码求和（如 slime-01 + torii-02 的 B05 = 64+300）；空白项目、draft/done 不计；同图纸双实例翻倍 |
| T-INV-3 | 缺口算术：全覆盖无行；部分覆盖差值；空库存 = 全量；排序缺颗降序 + 码升序 |
| T-INV-4 | 替代池：ΔE00 = 3 整拒收（严格 <）；同码自身排除；`remaining = beads − 自身需求`，0 不进池；排序 ΔE 升序 + 池序 |
| T-INV-5 | 采购文本：整串断言（码/名/hex/颗数四要素 + 合计行）；`expect(text).not.toMatch(/http\|:\/\//)` |
| T-INV-15 | fixture 不变量：全部 PATTERNS `Σ palette.beads === beadCount`；同码跨图纸的 name/hex 一致 |

组件（testing-library，`pages/inventory/inventory.test.tsx` 新建；种子用画廊 palette 切片喂 `renderApp({ seed })`）：

| # | 断言 |
|---|---|
| T-INV-6 | 新增：填表 → 行出现（含码文本）；重复码 → 内联错误、无第二行 |
| T-INV-7 | 改颗数 → 缺口行实时重算 |
| T-INV-8 | 删除（按「删除 <码>」可及名）→ 行消失、缺口相应增大 |
| T-INV-9 | 非法 hex / 负数颗数 → 错误呈现、库存不变 |
| T-INV-10 | §7 全部零态逐条（无项目 CTA 指 `/explore`；全空白项目文案；库存足够文案；空库存出全量缺口） |
| T-INV-11 | 替代行含码+名+hex+`ΔE00` 文本（对色弱可读，非仅色块） |
| T-INV-12 | 有缺口时 textarea 内容 = `buildPurchaseText` 输出；无缺口时无导出控件 |
| T-INV-13 | `test/flaky-storage.ts` 注入写失败 → 横幅在、CRUD 仍改 UI |
| T-INV-14 | `repository.test.ts` 扩：畸形库存条目（缺 hex / beads 为 NaN）被丢弃不崩溃 |

**Do-not-regress**：`app/persistence-banner.test.tsx`、`app/navigation.test.tsx`、`test/isolation.test.ts`（NE-1 自动覆盖新文件）原样保绿；
`app/empty-states.test.tsx` 的 Inventory 块**改写**为新文案（删 stub-note 断言），其余块不动。

## 9. 非目标（实现者不得顺手做）

- `/create` 上传转图管线（WP-B03）；`/assemble` 四模式与进度（WP-B04）；像素编辑（B06）；导入导出（B07）；画廊扩容（B08）。
- IndexedDB / `bead-v1`（BD19：只随第一个持久化 Grid 的 PR 出现）；`InventoryEntry` 加命名空间字段（同一触发点，D-INV-3）。
- rust / `crates/**` 任何改动；Soul 树；品牌授权色板（BD9）；账号体系（BD7）。
- Clipboard API、CSV/PDF 富格式导出、商购链接（BD15）。
- 跨色板替代（round1-map WP-B05 精化条已推迟）；替代量跨缺口预留（R-INV-3）；完工自动扣减库存；原地改码/名/hex（D-INV-6）；缺口按项目分列明细。

## 10. Ready-for-Opus 清单（照此下刀，逐项可勾）

- [ ] `stores/inventory.ts`：新增 §3 全部纯函数与派生类型；保留 `selectStock` / `totalBeads`。
- [ ] `stores/store.tsx`：`setInventory` 换成 `addInventoryEntry` / `setInventoryBeads` / `removeInventoryEntry`（reducer 各一 case；重复码 no-op；beads 夹取 0–99999）。
- [ ] `stores/repository.ts`：收紧 `isInventoryEntry`（§5 末条）。仅此一处，失败语义一个字不动。
- [ ] `pages/inventory/`：重写 `InventoryPage.tsx`，新增 `StockForm` / `StockList` / `ShortagePanel` / `SubstitutePanel`，三个 `aria-label` 区名不变。
- [ ] 样式进 `app.css`；复用 ColorSwatch / EmptyState / Card；**零新依赖**（D-UI-7）。
- [ ] 测试：新建 `stores/inventory.test.ts`、`pages/inventory/inventory.test.tsx`；扩 `repository.test.ts`；改写 `empty-states.test.tsx` Inventory 块。
- [ ] 不碰：`algo/**`（只 import）、`fixtures/catalog.ts`、`schema/**`、路由表、`components/ColorSwatch.tsx` 签名。
- [ ] BD15 自查：新源码零外网 URL 字面量（NE-1 会抓）；文案无出站链接。
- [ ] 全绿口径：`pnpm --filter @bead/app test`。

## 11. 残余风险（R-INV-*）

- **R-INV-1 · `G07` 码冲突**：画廊与 `generic-5mm` 各有一个 `G07`，色完全不同。v0 两空间不见面（D-INV-3），
  BD19 触发点若忘记引命名空间，缺口对比会把 Silver 当苔绿。缓解：本文 §2.3 与 BD19 双处钉死同一触发点。
- **R-INV-2 · fixture 不变量靠约定**：BOM 真源建立在 `Σ palette.beads === beadCount` 上，B08 扩画廊可能破。缓解：T-INV-15 变成门禁。
- **R-INV-3 · 替代量不预留**：两个缺口色可能同时看中同一候选的余量，v0 只做展示性建议，不做分配。记录为已知边界，不修。
- **R-INV-4 · 文案锁进测试**：§7 的零态句被测试整串断言，后续调文案要连测试一起改；这是 round1 空态表的既有代价，维持。

## 12. NO_HIGH_VALUE_CHANGE_FOUND 子树

- 「一键把图纸配色入库」按钮：录入表单 + 空库存全量缺口已覆盖同一需求路径，按钮引入库存被隐式重复填充的歧义。NO_HIGH_VALUE_CHANGE_FOUND。
- 库存消耗跟踪（拼完自动扣豆）：需要 B04 的完工事件与逐格计数，v0 缺口语义按「在拼即占用」已自洽。NO_HIGH_VALUE_CHANGE_FOUND。
- 库存搜索 / 分页：72 色以内的列表，码升序即可扫读。NO_HIGH_VALUE_CHANGE_FOUND。
- `InventoryEntry` 预埋命名空间字段：D-INV-3 已论证推迟到 BD19 触发点，提前加字段 = 无消费者的迁移面。NO_HIGH_VALUE_CHANGE_FOUND。
