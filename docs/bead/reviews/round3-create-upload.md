# ROUND 3 · WP-B03 `/create` 上传接线 — 前端 / 数据 IA

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只做 IA / 拆分，不实现）。
基线：`origin/cursor/beadflow-integration-c441` @ `696d0a8`。
输入：`docs/bead/PLAN.md`（获取豆图 / 图像转换）、`docs/bead/WORK_PACKAGES.md`（WP-B03）、`docs/bead/reviews/round2-data.md` §5（bead-v1 契约）、
`docs/agent-decisions.md` BD14 / BD15 / BD19 / BD20、`round2-inventory.md`（D-INV-1/3、§2.3 触发点）、`round2-assemble.md`（D-ASM-2/6/8）、
`round1-frontend.md`（D-UI-1/5/7、§5 状态所有权）、`round1-map.md` §4 B03/B09、
`apps/bead/src/pages/create/CreatePage.tsx` + `CreatePage.test.tsx`、`algo/`（pipeline / image / decode / framing / classify / grid / palette / steps / bom）、
`stores/`（repository / types / store / projects / inventory / ids）、`pages/assemble/`（AssemblePage / AssembleSession / AssembleCanvas / assemble.test）、
`fixtures/grids.ts`、`test/isolation.test.ts`、`test/render.tsx`。
硬禁区自查：本轮只新增本文件；不触碰 Soul 锁、`apps/desktop`、`crates/soul-*`、根配置、`deny.toml`、`.github/**`。

---

## 0. 结论先行（本 WP 的法律）

1. **`/create` 上传是「第一个持久化 `Grid` 的 PR」，BD19 在此兑现：同一 PR 必须整套落地
   `bead-v1` IndexedDB 契约（round2-data §5）。** 把转换网格 JSON 化塞进 localStorage `bead.state`
   的任何设计在本文里直接判死——round2-data §2.3 的算术（512² 两次导入即耗尽 origin 配额、
   整包重写把 MB 级同步写砸进主循环）不因 v0 上传输出 ≤56×56 而失效：契约的意义就是**在第一滴
   网格落盘时钉住形状**，不给 B07 留第二套存储发明的机会。
2. **BD20 与 BD19 同一触发点：本 PR 同时给库存与 BOM 引入色板命名空间**，否则画廊
   `G07 苔绿 #4c7a44` 与 `generic-5mm` `G07 Silver #B7BFC6` 在缺口对比里互撞（round2-inventory
   §2.3 的三件套 ①②③ 全部在此兑现）。
3. 算法侧**零发明**：`decodeImage → imageToPattern` 管线已在 `src/algo/` 完工并有 oracle parity
   测试。本 WP 是**接线 + 持久化 + 命名空间**，唯一要动 `algo/` 的是 DATA-3 的解码源尺寸上限
   （round2-data §6.5 明文归本 PR）。

## 1. 决策表（D-UP-*，实现者照抄，不自行发明产品法）

| # | 决策 | 内容 |
|---|---|---|
| **D-UP-1** | 入口形态 | 上传工作台渲染在 `/create?entry=upload` 面板位（D-UI-1：选中入口已在 URL，不开新路由、不做浮层）。`ENTRIES` 里 upload 的 `owner` 改为 `null`（「现在就能用」），detail 文案同步改写 |
| **D-UP-2** | 文件门 | `<input type="file" accept="image/png,image/jpeg">`，零权限弹窗（round2-data §7）。收单文件；MIME 白名单 `{image/png, image/jpeg}`，type 为空时按扩展名 `.png/.jpg/.jpeg` 后备；其余（gif/webp/svg/…）内联错误「只支持 png / jpg」，不静默忽略 |
| **D-UP-3** | 解码路径 | 只走现有 `decodeImage`（`createImageBitmap` + `colorSpaceConversion:"none"`，T-PAR-2 已钉）。**同 PR 修 DATA-3**：在 `decode.ts` 内、`drawImage` 之前按位图头尺寸设上限 ≤4096×4096（RGBA 64MB），超限抛新 `AlgoErrorCode` `"SourceTooLarge"`——放解码函数内部使 B07 导入天然继承同一门 |
| **D-UP-4** | 路径切换 | 三态段选：自动（默认）/ 像素图 / 照片。自动 = 启发式结果，并展示 `classification.kind` 与 `confidence`（百分比文案，字段名就叫置信度，BD15）；手动覆盖走 `PipelineOptions.kind`。像素图路径抖动被管线强制关闭是既有契约，UI 只如实呈现 |
| **D-UP-5** | 抖动开关 | 复选框，仅在生效路径（Photo）可交互；PixelArt 路径下禁用并注明「像素图路径不做抖动」。展示以结果的 `ditherApplied` 为准，不以开关状态为准 |
| **D-UP-6** | 框定预设 | 三预设映射 `Framing` union：①固定板（28×28 / 56×56 二选一 → `{mode:"board"}`）；②按比例适配（最长边 28 或 56 → `{mode:"aspect", maxSide}`）；③手动视口（→ `{mode:"manual", scale, crop}`）。默认 = 固定板 28。`fixed-boards` 模式**不进 v0 UI**（oracle parity 专用；多板拼接 post-v0，round1-map §3） |
| **D-UP-7** | 手动视口 UI | 源图预览（`URL.createObjectURL` 的运行时 blob:，非 URL 字面量——D-INV-11 先例；用完 revoke）+ 正方形取景框：拖拽/方向键平移，「输出格数」滑杆（内部换算 `scale`），确定后得 `{scale, crop}`。**v0 输出网格钉死 ≤56×56/轴**：三预设天然满足，手动档在 UI 层夹取——超过 56 的网格 DOM 画布（D-ASM-6）与 v0 方板范围都不接 |
| **D-UP-8** | 转换执行 | 参数变更即重算（滑杆连续输入 debounce），主线程跑管线 + `aria-busy` 忙态「转换中…」。**Web Worker 顺延**（对 round1-map §4 B03.3 的显式偏离，理由见 §7）。全过程纯预览：**确认保存前零字节落盘** |
| **D-UP-9** | 预览呈现 | 页面私有 `<ConversionPreview>`：DOM 网格（复用 `.assemble__bead` 的渲染技术但不复用 `AssembleCanvas`——它的 `cellStates` 语义是拼装会话的，undefined 即空格）+ BOM 摘要（`PipelineResult.bom`：码/名/颗数，色块必附码文本）+ 判定徽标（kind、confidence、ditherApplied、网格尺寸）。空网格（`occupiedCount === 0`）禁止保存并内联说明 |
| **D-UP-10** | 保存触发点 | 显式双 CTA，镜像详情页语义：「加入待拼」（status `todo`，留在原地出成功提示）/「转入工作台」（status `active`，跳 `/workspace`）。序列：`await savePatternDoc(doc)` **成功后**才 `addProject`——项目永不指向不存在的文档；保存失败 = 内联错误「转换结果没有保存」，不 mint 项目，禁止静默（§5.5） |
| **D-UP-11** | 项目记录 | `mintProjectId()`；`sourcePatternId: null`；title 输入框预填文件名（去扩展名、trim、≤64 字符，空 → 「未命名转换」）；backdrop 默认与 `createProjectFromPattern` 一致（black / `#101014`）。新纯函数 `createProjectFromConversion` 进 `stores/projects.ts`。Project 形状**不加字段**：转换项目的判别 = `sourcePatternId === null` 且 `loadPatternDoc` 命中 |
| **D-UP-12** | PatternDoc | 照 round2-data §5.2 原样：`{projectId, paletteId:"generic-5mm", width, height, cells: Int16Array(-1=空), provenance:{kind, ditherApplied}}`。`Grid ↔ Int16Array` 编解码 + 读回校验（长度 = width×height、每格 ∈ {-1} ∪ [0, entries.length)，违规整篇弃）为纯函数进新 `stores/patterns.ts`。56×56 = 3136 格 ×2B ≈ 6.3KB/篇 |
| **D-UP-13** | 进度形状 | **进度保持现有 `ProgressCursor` 五字段游标**（BD19 原文），迁入 `progress` 仓后形状不变；`doneBits` 本 PR 不写（B04 未偏离步序，无消费者）。`Step[]`、BOM、缺口、替代照旧禁止持久化 |
| **D-UP-14** | assemble 接线 | AssemblePage 网格来路二分支：`sourcePatternId !== null` → `fixtureGridFor`（同步，现状）；`null` → `loadPatternDoc(projectId)`（async hook，加载态「正在读取豆图……」）。命中 → 经 `stores/patterns.ts` 的适配函数转成与 `FixtureGrid` **同形状**的 `{grid, palette:{code,name,hex}[]}`（swatch 从 `GENERIC_5MM` entries 现算，rgb→hex 纯函数）喂 `AssembleSession`。`AssembleCanvas` 零改动（其注释早已预言这扇门）。未命中 → `NO_GRID_NOTE`，文案同 PR 改写（§5） |
| **D-UP-15** | 命名空间 | `PaletteNamespaceId = "gallery" \| "generic-5mm"`（v0 封闭 union，常量进 `stores/types.ts`）。`InventoryEntry` 增 `paletteId` 必填字段；身份键升级为 `(paletteId, code)`；需求/缺口/替代/采购文本全链路带命名空间（§4）。存量库存条目在 bead-v1 迁移时一次性盖 `"gallery"`（D-INV-3：v0 录入全是画廊码） |
| **D-UP-16** | 错误政策 | IDB 打开/写入失败沿 DATA-1 的既有轨道：`persistenceFailed` 信号 + 全局 `PersistenceBanner`，**不新增错误 UI**；唯一例外是保存流程 await 的 `savePatternDoc` 拒绝要额外给内联错误（用户正在等这一下）。任何 catch-后-假装-成功都是缺陷 |

## 2. bead-v1 落地范围（round2-data §5 照办 + 两处对账）

契约条文（库名 `bead-v1`、version 1、四仓 `state`/`patterns`/`progress`/`meta`、迁移一次性单向幂等、
theme 留 localStorage、错误政策、fake-indexeddb 测试口径、不引包装库）**逐条照 §5 执行**，本文不复述。
§5 成文于 B04 落地之前，与现状有两处必须对账：

1. **迁移搬四类不是三类。** §5.4 写「三个数组 put 进 `state` 仓」时 `bead.state` 还没有第四个
   `progress` 数组（B04 后加入）。实际迁移：`parsePersistedState` 读出四键 → `projects` /
   `favorites` / `inventory` 三个整数组 put 进 `state` 仓（保持整数组 last-writer-wins 语义）；
   `progress` 数组**逐条拆开** put 进 `progress` 仓（keyPath `projectId`，`sanitizeProgress` 已保证
   一项目一游标）→ 删除 `bead.state` 键。第二次打开读不到键即幂等。
2. **§5.3 的方法名与现状冲突，裁决如下。** `repository.ts` 现已有
   `loadProgress(): Promise<ProgressCursor[]>` / `saveProgress(cursors)`（B04 落的数组形签名），
   与 §5.3 提议的 `loadProgress(id)` / `saveProgress(doc)` 同名不同形。**保留现有数组形签名一字不改**
   ——这正是 D-UI-5 铺 async 接缝的全部回报，`store.tsx` 与四个 save effect 零改动；内部实现改由
   `progress` 仓承载（load = `getAll`；save = 单事务 put 每条 + 删除被 `sanitizeProgress` 剪掉的
   projectId）。§5.3 的接口增量收敛为三个新方法：

```ts
loadPatternDoc(id: ProjectId): Promise<PatternDoc | null>;
savePatternDoc(doc: PatternDoc): Promise<void>;
deleteProject(id: ProjectId): Promise<void>;   // 单事务级联删 patterns + progress（§5.3 唯一跨仓不变量）
```

   逐项目粒度的 progress 读写等出现消费者再加，不预埋。`deleteProject` 本 PR 无 UI 调用方，
   仍随契约落地并测试：它是唯一跨仓不变量、约 15 行，缺了它每个后续 WP 都会各自发明清理逻辑，
   且 MB 级 pattern 文档没有回收通道。项目删除 UI 明确不在本 WP（§6）。

其余钉死：`state` 仓保持整数组读改写；IDB 打开收敛为惰性单例 promise（打开失败 → `persistenceFailed`
+ 内存 fallback，读写视图收敛，DATA-1 语义原样继承）；多标签页 last-writer-wins 照旧（DATA-5 已知边界）；
`fake-indexeddb` 进 devDependencies（§5.6，不进运行时依赖面，不违 D-UI-7）；`createInMemoryRepository`
测试桩同步扩这三个方法（内存 Map 即可）。

## 3. 存储时序（何时写、写什么）

| 时刻 | 写入 | 仓 | 说明 |
|---|---|---|---|
| 选文件 / 调参 / 预览 | **无** | — | 转换结果是派生数据，确认前不落盘 |
| 双 CTA 确认 | `PatternDoc`（grid + palette id + provenance） | `patterns` | `await` 成功后才走下一行 |
| 同上，紧随 | `Project` 元数据（经 `addProject` → 既有 saveProjects effect） | `state` | doc 先于项目可见：失败最坏留孤儿 doc（无害、可被 `deleteProject` 回收），绝不出现指向空 doc 的项目 |
| 拼装中 | `ProgressCursor`（D-ASM-8 的既有写时刻，节拍 = 步） | `progress` | 形状不变，只换仓 |
| 首次打开 bead-v1 | 迁移（§2.1） | 全部 | 一次性、幂等 |

## 4. BD20 命名空间落地（与 §2 同一 PR，round2-inventory §2.3 ①②③ 兑现）

1. **字段**：`InventoryEntry` 增 `paletteId: PaletteNamespaceId`；`RequirementRow` / `ShortageRow` /
   `SubstituteCandidateRow` 同增。`isInventoryEntry` 校验 `paletteId` ∈ 已知集（丢弃不崩溃，SH-4 先例）。
2. **身份键**：`(paletteId, code)`。`store.tsx` 三个库存 action 的重复判定 / 匹配键、
   `stores/inventory.ts` 的 `stockByCode` / `requiredByCode` 聚合键全部升级；`selectStock` 排序
   先命名空间后码。
3. **转换项目 BOM = `buildBom(grid, GENERIC_5MM)`**（D-INV-1 保留给此刻的那一半）。`/inventory`
   新增 async 派生：加载在拼（todo/active）转换项目的 PatternDoc → `buildBom` → G 码需求行
   （name/hex 从 `GENERIC_5MM` entries 现算），与画廊需求行合并进缺口。**现算不落盘**（D-INV-13
   原样延伸）；加载窗口给显式「正在读取转换项目豆图…」态，防止缺口短暂只显画廊需求被当成完整清单。
   画廊项目路径（`pattern.palette` 为真源）一行不改。
4. **替代池不跨命名空间**：候选过滤加 `entry.paletteId === wanted.paletteId`（跨色板替代
   round1-map §4 B05 已推迟，维持）。
5. **UI**：库存录入表单加命名空间选择（默认「画廊」，选项文案：画廊 / 通用5mm）；库存行、缺口行、
   替代行、采购文本行均带命名空间标签（如 `【通用5mm】G07 Silver #b7bfc6 缺 96 颗`）——色号文本
   不再全局唯一，缺了标签 G07 就是二义的。`buildPurchaseText` 输出与 T-INV-5 整串断言同 PR 更新。
6. **冲突解除验收**：库存里画廊 `G07 苔绿` 与需求里 generic-5mm `G07 Silver` 并存时，缺口两行独立、
   互不抵扣、替代互不入池（T-UP-18 直接钉这个案例）。

## 5. `/create` 既有测试改写（同 PR、独立 commit、逐条映射，禁止静默破坏）

测试改写与功能接线在**同一 PR 内作为独立 commit** 落地，commit 说明引用本表。规则：每条被移除的
断言必须有至少一条新断言接位，`git diff` 里只许出现下表的行。

| 现有断言（`CreatePage.test.tsx`） | 处置 |
|---|---|
| 用例1：`getByText("尚未实现，归 WP-B03")` | **替换**为 upload 入口标注「现在就能用」（`owner: null` 的既有渲染路径） |
| 用例1：`WP-B07 ×2`、`WP-B06 ×1` 断言 | **保留原样**（B06/B07 仍是占位，不许顺手动） |
| 用例2：upload 说明页含 `WP-B03` 与 `不是缺陷` | **整体替换**为上传工作台测试：点开 `?entry=upload` → 出现文件输入与参数面板，不再有占位说明 |
| 用例3：画廊入口无占位说明、CTA 指 `/explore` | **不动** |

同 PR 连带的文案与测试：

- `AssemblePage.NO_GRID_NOTE` 改写为不再把上传归为未建功能（建议：「这个项目还没有豆图网格——
  空白项目的编辑器归 WP-B06，立体拼豆的多板拼接不在 v0。」）。`assemble.test.tsx` 经正则断言
  `/还没有豆图网格/`，机械保绿，但文案变更必须在 PR 描述里列出。
- `CreatePage.tsx` upload 入口 `detail` 文案改为描述实际能力（png/jpg、双路径、抖动、三框定）。
- `inventory` 相关既有测试（T-INV-*）因命名空间字段而更新的部分：**在原文件内改写，不删用例**；
  `repository.test.ts` 的迁移新用例只增不改。

## 6. 非目标（实现者不得顺手做）

- 真实社交后端 / 上传分享 / 任何出网（BD7、BD15、NE-1）。
- `crates/**` 任何改动——rust `bead-core` 是 oracle（BD18），本 PR 不碰一字节。
- Soul 树：`apps/desktop/**`、`crates/soul-*/**`、根配置、`docs/PRODUCT_LOCK.md` 等锁（禁区照旧）。
- 六角板 / 圆板 / 多板拼接（`fixed-boards` UI）——post-v0；v0 只做方板 ≤56×56（D-UP-7）。
- 品牌授权色板（BD9）：色板恒 `generic-5mm`，UI 不出现色板选择器。
- 像素编辑器（B06）、`.pat`/`.gamedev`/`.beadproj` 导入（B07）、画廊扩容（B08）。
- 保存后改参数 / 重新框定：v0 转换文档不可变，重转 = 新项目；项目删除 UI 也不在本 WP。
- `doneBits` 逐格进度、Dexie/idb 包装库、`navigator.storage.persist()`、多标签页一致性
  （round2-data §5.7 全部维持）。
- webp/gif/剪贴板粘贴/拖拽上传：WP 钉 png/jpg 文件选择，其余是后续打磨。

## 7. Worker 顺延的论证（对 round1-map §4 B03.3 的显式偏离）

round1-map 要求「转换跑在 Web Worker 里」。本轮裁决 v0 接线**主线程 + 忙态**，Worker 顺延：

- 预算本身（512×512 照片 → 56×56 板 < 1s 桌面级）主线程即达：管线是对源像素的常数趟数扫描，
  512² ≈ 26 万像素在毫秒到十毫秒量级。真正的最坏面是 D-UP-3 上限内的 4096² 源（≈1680 万像素，
  分类 + 重采样合计约 1–2s）——一次性、有 `aria-busy` 忙态、且发生在用户刚刚主动点了转换参数的
  瞬间，不落在拼装主循环上。
- Worker 引入三个新失败面：Vite worker 打包入口、`RgbaImage` 的结构化克隆/transfer 约定、
  jsdom 无 Worker 导致的测试抽象层。本 PR 已经背着 bead-v1 契约 + BD20 命名空间两件必须品，
  再驮一件可延期的性能工程违反本线「一 PR 一债」的节奏。
- 管线是纯函数（`imageToPattern(image, options)`），换线程是纯宿主问题，日后上 Worker 零 API 破坏。
  触发条件写死：B07 把 512² 网格带进来、或实测超预算，谁先到谁触发。

记录为 R-UP-1，父代理可否决；若否决，Worker 化作为**独立后续 PR**，不并入本 PR。

## 8. 验收测试矩阵（T-UP-*）

存储 / 纯函数（`repository.test.ts` 扩 + 新 `stores/patterns.test.ts`，fake-indexeddb）：

| # | 断言 |
|---|---|
| T-UP-1 | PatternDoc 含 `Int16Array` 经 bead-v1 往返不变（§5.6 必测其一） |
| T-UP-2 | 迁移：种四键 `bead.state` → 打开 → 三数组在 `state` 仓、游标逐条在 `progress` 仓、原键已删、再开不重复迁（§5.6 + §2.1 对账） |
| T-UP-3 | `deleteProject` 单事务级联删 patterns + progress（§5.6） |
| T-UP-4 | 打开失败 ⇒ `persistenceFailed` 可观察、读写走内存视图（§5.6；DATA-1 语义在 IDB 上重申） |
| T-UP-5 | 读回校验：cells 长度不符 / 越界索引 / 未知 paletteId 的 doc 被弃为 null，不崩溃 |
| T-UP-6 | `saveProgress(cursors)` 数组签名在 `progress` 仓上保持剪枝语义（被剪 projectId 的记录被删） |
| T-UP-7 | 迁移给存量库存条目盖 `paletteId:"gallery"`；`isInventoryEntry` 拒未知命名空间 |

管线接缝（`algo/decode.test.ts` 扩）：

| # | 断言 |
|---|---|
| T-UP-8 | 4096×4096 位图头通过；4097×10 抛 `SourceTooLarge` 且 `bitmap.close()` 仍被调（DATA-3） |

上传工作台（新 `pages/create/upload.test.tsx`，经 `renderApp`）：

| # | 断言 |
|---|---|
| T-UP-9 | png/jpg 收；gif/webp/无类型拒并出内联错误，参数面板不出现 |
| T-UP-10 | 自动路径展示 kind + 置信度；手动覆盖后结果随 `options.kind` 变（喂确定性小图 fixture） |
| T-UP-11 | Photo + 抖动开 → 徽标 `ditherApplied`；切 PixelArt → 开关禁用、徽标灭 |
| T-UP-12 | 三预设输出网格尺寸正确（28/56/aspect/manual）；手动档 >56 被夹取或拒绝 |
| T-UP-13 | 空网格（全透明图）禁止保存并说明 |
| T-UP-14 | 「加入待拼」/「转入工作台」→ PatternDoc 在仓、项目 status 正确、导航正确；`savePatternDoc` 注入 reject → 内联错误、项目数不变 |
| T-UP-15 | 保存后存储里**没有** `Step[]`、没有 BOM、没有第二份网格（枚举仓内容断言键集） |

拼装接线（`assemble.test.tsx` 扩）：

| # | 断言 |
|---|---|
| T-UP-16 | 种转换项目 + PatternDoc → `/assemble/:id` 渲染 `assemble-grid`，swatch 来自 generic-5mm；步进/游标/四模式行为与 fixture 项目同一套断言复用 |
| T-UP-17 | 转换项目无 doc → 新版 `NO_GRID_NOTE`；加载中出加载态 |

库存 / BD20（`stores/inventory.test.ts` + `pages/inventory/inventory.test.tsx` 扩）：

| # | 断言 |
|---|---|
| T-UP-18 | G07 案例：画廊 G07 库存 + generic-5mm G07 需求 → 两行独立不抵扣；替代池不跨命名空间 |
| T-UP-19 | 转换项目需求 = `buildBom` 输出并入缺口；加载窗口有显式状态 |
| T-UP-20 | 采购文本带命名空间标签，且 `not.toMatch(/http|:\/\//)`（T-INV-5 升级承接） |

消毒 / 不出网：

| # | 断言 |
|---|---|
| T-UP-21 | NE-1 静态扫描自动覆盖全部新文件（`isolation.test.ts` 递归 src，无需改动）——上传的图片只进 `createImageBitmap`，源码零 `fetch`/XHR/WebSocket/sendBeacon/远程 URL 字面量（BD15）；`URL.createObjectURL` 的 blob: 是运行时值不是字面量，D-INV-11 先例，扫描器天然放过 |

**Do-not-regress**：`navigation.test.tsx`、`persistence-banner.test.tsx`、`empty-states.test.tsx`、
`oracle-parity.test.ts`、全部 `algo/` 既有测试原样保绿；`/create` 与 inventory 的既有用例按 §5 映射
改写不删。全绿口径 `pnpm --filter @bead/app test`；提交前根 `just ci` + e0-audit / denylist-audit
照 B10 惯例（fake-indexeddb 落锁文件，BD16：同一提交重生成根 `pnpm-lock.yaml`）。

## 9. 状态所有权增量（round1 §5 表的 delta）

| 状态 | 拥有者 | 持久化 | 写者 | 读者 |
|---|---|---|---|---|
| 上传文件 / 参数 / 预览结果 | UploadWorkbench 页内 state | **禁止** | 页内 | 页内 |
| PatternDoc（网格 + 色板 id + 出处） | Repository | IDB `patterns` 仓 | 保存双 CTA（唯一写点） | AssemblePage 适配、/inventory 转换需求 |
| 项目元数据 / 收藏 / 库存 | ProjectStore / InventoryStore | IDB `state` 仓（整数组语义不变） | 既有 action | 既有 selector |
| 步游标 | ProjectStore `progress` | IDB `progress` 仓（形状不变） | D-ASM-8 既有写时刻 | AssemblePage 恢复 |
| BOM / 缺口 / 替代 / `Step[]` | 无（selector / hook 现算） | **禁止** | — | /inventory、AssembleSession |

## 10. 残余风险（R-UP-*）

- **R-UP-1 · Worker 顺延**：4096² 最坏 1–2s 主线程忙。缓解：忙态 + 上限 + §7 的触发条件；父代理可否决。
- **R-UP-2 · 孤儿 PatternDoc**：doc-先-项目-后的两步保存在第二步失败时留孤儿 doc。无害（不可见、
  `deleteProject` 可回收、IDB 配额 GB 级），记录不修。
- **R-UP-3 · 迁移盖章误伤**：手改 localStorage 录入过 G 码通用豆的条目会被盖成 `gallery`。
  UI 从未提供该路径，码文本仍在可手动重录；记录为已知边界。
- **R-UP-4 · /inventory 双源需求**：画廊（同步）+ 转换（async）合并有时序窗口，忘做加载态会让缺口
  短暂少列。缓解：§4.3 的显式加载状态 + T-UP-19。
- **R-UP-5 · 两套存储并存**：theme 留 localStorage、档案在 IDB，devtools 手改面翻倍。缓解：
  读回校验（D-UP-12 / T-UP-5）延续 `parsePersistedState` 哲学。

## 11. NO_HIGH_VALUE_CHANGE_FOUND 子树

- 转换参数草稿持久化（关页恢复参数）：预览是派生数据，重选文件成本一次点击。NO_HIGH_VALUE_CHANGE_FOUND。
- 上传历史 / 最近转换列表：Workspace 项目列表已是归宿。NO_HIGH_VALUE_CHANGE_FOUND。
- 拖拽上传区 / 剪贴板粘贴：`<input type="file">` 已覆盖 v0 需求路径，新增的是事件面不是能力。NO_HIGH_VALUE_CHANGE_FOUND。
- `AssembleCanvas` 泛化改造：`{code,hex}` 形状的 palette prop 本来就为这扇门设计，零改动即接。NO_HIGH_VALUE_CHANGE_FOUND。
- Project 加 `kind`/`hasPatternDoc` 判别字段：`sourcePatternId === null` + doc 命中已判别，加字段 = 双权威。NO_HIGH_VALUE_CHANGE_FOUND。
- IDB 包装库 / 逐记录 CRUD 化 `state` 仓 / `navigator.storage.persist()`：round2-data §5.7 逐条维持。NO_HIGH_VALUE_CHANGE_FOUND。

## 12. Ready-for-Opus 清单（照此下刀，逐项可勾）

- [ ] `algo/decode.ts` + `image.ts`：`SourceTooLarge` 上限（DATA-3），`decode.test.ts` 扩 T-UP-8。
- [ ] `stores/types.ts`：`PaletteNamespaceId`、`PatternDoc`、`ProgressDoc`（= 游标 + 可选 doneBits，本 PR 不写 doneBits）、`InventoryEntry.paletteId`。
- [ ] `stores/patterns.ts`（新）：Grid↔Int16Array 编解码、doc 校验、generic-5mm swatch 适配（rgb→hex）。
- [ ] `stores/repository.ts`：bead-v1 打开/迁移/四仓；三个新方法；数组形签名与 DATA-1 语义一字不动；`createInMemoryRepository` 同步扩。
- [ ] `stores/projects.ts`：`createProjectFromConversion`。
- [ ] `stores/inventory.ts` + `store.tsx`：`(paletteId, code)` 身份键、替代池同空间过滤、转换需求合并 hook。
- [ ] `pages/create/`：`UploadWorkbench.tsx`、`ConversionPreview.tsx`；`CreatePage.tsx` 入口改 `owner: null`。
- [ ] `pages/assemble/AssemblePage.tsx`：网格来路二分支 + `NO_GRID_NOTE` 新文案。
- [ ] `pages/inventory/`：命名空间标签与表单选择。
- [ ] 测试：§8 全表；§5 映射改写为独立 commit。
- [ ] `fake-indexeddb` devDependency + 根 `pnpm-lock.yaml` 同提交（BD16）；`.gitignore` 已含 `apps/bead/dist/`。
- [ ] BD15 自查：零外网 URL 字面量（NE-1 会抓）；不碰 `crates/**`、Soul 树、`.github/**`。

## 13. 禁区自查

本轮只新增 `docs/bead/reviews/round3-create-upload.md`。未改 `apps/bead` 任何源码（全部留给实现席），
未触碰 `docs/PRODUCT_LOCK.md`、`docs/STATUS.md`、`apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml`、
`deny.toml`、`.github/**`。
