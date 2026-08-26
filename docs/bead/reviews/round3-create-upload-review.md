`claude-fable-5-thinking-xhigh`

# ROUND 3 · WP-B03 `/create` 上传接线 + bead-v1 + BD20 · 实现审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审查，不实现）。
被审对象：合并提交 `bb4ce71`（absorb WP-B03 upload + bead-v1 IDB + BD20，即 PR #45 的吸收面），
基线 `origin/cursor/beadflow-integration-c441`。
审查依据：`docs/bead/reviews/round3-create-upload.md`（D-UP-1…16 / T-UP-1…21，本 WP 的法律）、
`docs/agent-decisions.md`（BD14 / BD15 / BD18 / BD19 / BD20）、`round2-data.md` §5、`round2-inventory.md` §2.3。
本轮只新增本文件；未改任何产品文件、测试文件、rust、Soul 树。

范围对账：吸收提交比实现分支多出两个父代理记账文件（`docs/agent-progress.md` 更新与
`round3-import-export.md` 的 B07 IA 落库），不属于实现者的交付面；实现分支本身的 diff 恰为
`apps/bead/**` 33 文件 + 根 `pnpm-lock.yaml`（BD16 同提交重生成，增量仅 fake-indexeddb）。
实现分支的提交结构满足 IA §5「测试改写独立 commit」的要求（`6cf7e63` 单独承载 CreatePage.test.tsx 改写）。

---

## 0. 结论先行

**接受（ACCEPT-WITH-NITS）。无 HIGH、无 MED 缺陷。** BD19 的整套 `bead-v1` 契约、BD20 的
`(paletteId, code)` 命名空间、DATA-3 的解码上限、doc-先-项目-后的保存时序全部成立并有测试钉死；
IA 点名的四个实现者旗标逐一核实为真。`pnpm --filter @bead/app test` 实测 **29 个文件 / 426 条全绿**
（父代理门禁 29/426，命中），`tsc --noEmit && eslint .` 零输出。下面列 4 条 LOW 与若干观察，
均不构成回退理由。不重开架构。

## 1. 任务钉出的检查点

### 1.1 BD19：第一滴网格落盘 = 整套 bead-v1，Grid 不进 localStorage ✅

- `stores/bead-v1.ts`：库名 `bead-v1`、version 1、四仓 `state`/`patterns`/`progress`/`meta` 一次建齐
  （bead-v1.ts:45-104）；打开是惰性单例 promise，失败 → `persistenceFailed` + 内存视图收敛
  （bead-v1.ts:171-198），DATA-1 语义原样继承。不引任何包装库。
- 迁移照 §2.1 对账后的四类：三个整数组 put 进 `state` 仓、`progress` 数组逐条拆进 `progress` 仓
  （keyPath `projectId`）、事务提交后删 `bead.state` 键，二次打开读不到键即幂等（bead-v1.ts:125-143）。
  迁移中途失败时不删旧键、内存视图直接改喂旧 blob——比契约多想了一步「空库像丢数据」的观感问题。
- **Grid 永不进 `bead.state`**：非测试源码里 `STORAGE_KEY` 的写者只有降级仓的元数据 read-modify-write
  与迁移的删除；降级仓的 `savePatternDoc` 拒绝写入而不是把网格序列化到别处（repository.ts:174-183）。
  repository.test.ts:471-472 直接断言旧 blob 里没有 `cells`；T-UP-15（repository.test.ts:543-581）
  枚举四仓键集，`Step[]`/BOM/第二份网格一个都没落盘。**红线锁死。**
- 主题照旧留 localStorage（`bead.theme`，theme.tsx 未被触碰），与 §5 的裁决一致。

### 1.2 BD20：`(paletteId, code)` 身份、迁移盖章、转换 BOM、替代不跨空间 ✅

- `stores/types.ts:13-28`：`PALETTE_NAMESPACES` 封闭 union（gallery / generic-5mm）+ 标签表；
  `InventoryEntry.paletteId` 必填（types.ts:85-92）。
- 身份键升级全链路：reducer 三个库存 action 以 `paletteCodeKey` 判重/匹配（store.tsx:90-116）、
  `stockByCode`/`requiredByCode` 聚合键（inventory.ts:198-209）、`selectStock` 先命名空间后码
  （inventory.ts:84-88）、三个面板的 li key 与回调全带双半键。
- 迁移盖章：`toInventoryEntry` 对**没写** `paletteId` 的存量条目盖 `gallery`，对**写错**的整条丢弃
  （persisted.ts:71-79）；bead-v1 迁移经由 `parsePersistedState` 走同一条路（bead-v1.ts:134-136）。
  实现比 IA 字面更严：localStorage 降级仓的每次读取也走同一盖章，语义是 IA 的超集，无害。
- 转换 BOM = `buildBom(grid, GENERIC_5MM)`，名/hex 从色板条目现算、现算不落盘
  （inventory.ts:171-189）；`/inventory` 双源合并 + 显式「正在读取转换项目豆图……」加载态
  （InventoryPage.tsx + ShortagePanel.tsx，R-UP-4 的缓解落实）。画廊路径一行未改
  （`selectRequirements` 仅加 `paletteId: GALLERY_PALETTE` 标记）。
- 替代池 `entry.paletteId !== wanted.paletteId` 即跳过（inventory.ts:266），跨色板替代维持推迟。
- T-UP-18 的 G07 案例五连测（inventory.test.ts）：两行独立、互不抵扣、颜色再近也不入对方池、
  `selectStock` 排序、`conversionRequirements` 的整行断言——round2-inventory §2.3 的撞码彻底解除。

### 1.3 DATA-3：SourceTooLarge 在 drawImage 之前 ✅

- `algo/decode.ts:47-63`：`MAX_SOURCE_SIDE = 4096`，按**位图头**判，位于画布分配与 `drawImage` 之前；
  `AlgoErrorCode` 增 `"SourceTooLarge"`（image.ts）。闸门放在解码函数内部，B07 导入天然继承。
- T-UP-8 超额完成：边界 4096 放行（双轴向）、4096×4096 头过闸走到 drawImage、4097×10 与 10×4097
  抛 `SourceTooLarge` 且**不画**、`bitmap.close()` 在 finally 里照样被调（decode.test.ts:73-105）。
- 工作台把该错误翻译成人话「图片太大：最长边不能超过 4096 像素」（UploadWorkbench.tsx:76-81）。

### 1.4 保存时序：savePatternDoc 成功后才 addProject ✅

- UploadWorkbench.tsx:218-250：`mintProjectId` → `createPatternDoc` → `await savePatternDoc`，
  catch → 内联「转换结果没有保存」并 return，**不 mint 项目**；成功后才 `addProject(createProjectFromConversion(...))`。
  `saving` ref 挡双击。todo 留在原地出成功提示，active 跳 `/workspace`——双 CTA 语义与详情页镜像。
- T-UP-14 四连测（upload.test.tsx:278-353）：doc 与项目都在仓、状态正确、导航正确、注入 reject 后
  项目数为零、确认前零字节落盘（改参数后 `loadProjects()` 仍空）。

### 1.5 仓库表面：数组形签名一字不改，新面恰为三个方法 ✅

- `ProjectRepository` 六方法（含 `loadProgress(): Promise<ProgressCursor[]>` / `saveProgress(cursors)`）
  形状与 B04 落地时一致（repository.ts:22-29）；`store.tsx` 四个 save effect 逐字未动
  （store.tsx:192-210）。bead-v1 内部以 `progress` 仓承载：load = `getAll` + 逐条消毒；
  save = 单事务「删被剪 + put 保留」（bead-v1.ts:287-325），T-UP-6 双测钉住剪枝与 last-writer-wins。
- 新增面恰为 `loadPatternDoc` / `savePatternDoc` / `deleteProject` 三个（repository.ts:37-43），
  逐项目粒度的 progress 读写没有预埋。`deleteProject` 单事务级联删 patterns + progress
  （bead-v1.ts:370-388），无 UI 调用方但随契约落地并被 T-UP-3 测试。
- 写序列化：bead-v1 把写任务串成 promise 链（bead-v1.ts:204-209），`saveProgress` 的剪枝读到的
  一定是先入队的 `saveProjects` 写完后的 projects——store.tsx 不 await 的四个 effect 因此不会互相踩。
  这是契约没点名但确实堵住竞态的一手，质量加分。

### 1.6 拼装接线：AssembleCanvas 零改动，同形状适配器 ✅

- `AssembleCanvas.tsx` 不在 diff 里——API 与实现原样。`AssembleSession` 的 prop 从
  `fixture: FixtureGrid` 换名为 `board: BoardSource`（结构同形，注释点名 D-UP-14），会话逻辑零变化。
- AssemblePage 网格来路二分支（AssemblePage.tsx:40-92）：`sourcePatternId !== null` → `fixtureGridFor`
  同步照旧；`null` → `usePatternDoc` async hook，loading → `LOADING_GRID_NOTE`（「正在读取豆图……」
  + `aria-busy`），命中 → `boardSourceOf(doc)`（patterns.ts:162-166，swatch 从 `GENERIC_5MM` 现算，
  rgb→hex 纯函数），未命中 → 新版 `NO_GRID_NOTE`。
- T-UP-16 三测：转换项目渲染 `assemble-grid`、aria-label 报 generic-5mm 色号（G06 Black / G15 Red）、
  步进/模式切换/游标落仓/种游标恢复/四模式在场，与画廊项目同一套行为断言。
  T-UP-17 双测：无 doc → 断言 `NO_GRID_NOTE` 全文且不提 WP-B03；永不 settle 的 `loadPatternDoc`
  钉住加载态。既有 T-ASM-17 只把同步断言改成 `await findByText`（分支变 async 的必然），语义未损。

### 1.7 CreatePage.test.tsx 改写：§5 映射逐条成立 ✅

- 用例1 替换：upload 标「现在就能用」+ 显式断言「尚未实现，归 WP-B03」不在文档里；
  **B07 ×2、B06 ×1 的占位断言原样保留**（CreatePage.test.tsx:27-28），B06 占位面板的
  「不是缺陷」话术另有一测（:51-58）。
- 用例2 替换：`?entry=upload` 面板出文件输入、无「不是缺陷」/「WP-B03」话术；另补直链一测（D-UI-1）。
- 用例3（gallery）语义不动。`NO_GRID_NOTE` 文案改写与 IA §5 的建议句逐字一致（AssemblePage.tsx:18-19），
  `assemble.test.tsx` 的正则 `/还没有豆图网格/` 机械保绿。upload 入口 `detail` 改为描述实际能力。

### 1.8 卫生项 ✅

- `fake-indexeddb` 仅在 devDependencies 与 `repository.test.ts` 出现；运行时依赖面零增（D-UI-7）。
- 全树无 `Worker`/`fetch`/`XMLHttpRequest`/`WebSocket`/`sendBeacon`；非测试源码唯一的「URL」是
  `URL.createObjectURL` 的运行时 blob:（用完 revoke，UploadWorkbench.tsx:121-128），无远程 URL 字面量；
  `isolation.test.ts` 递归扫描自动覆盖全部新文件并通过（T-UP-21）。
- rust、Soul 树、`.github/**`、根配置零触碰；`crates/**` 一字节未动（BD18）。

## 2. D-UP-1…16 与 T-UP-1…21 落点速查

| ID | 判定 | 落点 |
|---|---|---|
| D-UP-1 | ✅ | CreatePage.tsx:27-35（owner null + detail 改写）、:91（面板位渲染工作台，无新路由无浮层） |
| D-UP-2 | ✅ | UploadWorkbench.tsx:39-41、63-69、136-143（MIME 白名单 + 空 type 扩展名后备 + 内联拒绝） |
| D-UP-3 | ✅ | decode.ts:47-63（上限在 drawImage 前）；解码只走 `decodeImage`，`colorSpaceConversion:"none"` 原样 |
| D-UP-4 | ✅ | 三态段选 UploadWorkbench.tsx:319-338；auto 不传 kind、手动覆盖走 `options.kind`（:176-183）；徽标展示 kind + 置信度百分比（ConversionPreview.tsx:44-45，字段名就是置信度，BD15） |
| D-UP-5 | ✅ | 复选框 PixelArt 路径禁用 + 注明（:340-351）；徽标以 `result.ditherApplied` 为准（ConversionPreview.tsx:46） |
| D-UP-6 | ✅ | 三预设映射 Framing union（:165-174），默认固定板 28；`fixed-boards` 不在 UI |
| D-UP-7 | ✅ | blob: 预览 + revoke（:83-87、121-128）；拖拽（pointer 事件 + 捕获，:276-296）+ 方向键（Shift ×10，:258-270）；输出格数滑杆内部换算 scale 且夹取 ≤56（:446-462）；framing.ts:463-471 使手动档输出恰等于 outputCells |
| D-UP-8 | ✅ | 参数变更即重算 + 60ms debounce（:189-204）、`aria-busy` + 「转换中…」；无 Worker；确认前零字节（T-UP-14 末测钉死） |
| D-UP-9 | ✅ | 页面私有 `<ConversionPreview>`（借 `.assemble__bead` 技术、不借 AssembleCanvas）；徽标四项、BOM 经 ColorSwatch 必附码文本；空网格禁存 + 内联说明（:214-216、486-490） |
| D-UP-10 | ✅ | §1.4；双 CTA 语义、await-成功-才-mint、失败内联不静默 |
| D-UP-11 | ✅ | projects.ts:35-50（`createProjectFromConversion`，Project 形状零新字段）；标题预填/trim/≤64/空回落（:71-74、241）；backdrop 与 `createProjectFromPattern` 一致 |
| D-UP-12 | ✅ | types.ts:132-139 照 §5.2；编解码 + 读回校验为纯函数（patterns.ts:61-155），违规整篇弃 |
| D-UP-13 | ✅ | 游标五键不变（types.ts:100-107）；`ProgressDoc` 仅声明（:114-116），全树无 doneBits 写者 |
| D-UP-14 | ✅ | §1.6；AssembleCanvas 零改动，适配函数在 stores/patterns.ts |
| D-UP-15 | ✅ | §1.2 全链路 |
| D-UP-16 | ✅ | bead-v1 全部 catch → `markFailed`（横幅），唯一例外 `savePatternDoc` 双通道（markFailed **且** rethrow，bead-v1.ts:344-362）；无 catch-后-假装-成功 |

| T | 判定 | 落点 |
|---|---|---|
| T-UP-1 | ✅ | repository.test.ts:306-341（Int16Array 原样往返 + 六个老方法回各仓） |
| T-UP-2 | ✅ | repository.test.ts:343-404（四键搬三数组 + 逐条游标、原键删、二开不重迁、空库不造数据） |
| T-UP-3 | ✅ | repository.test.ts:406-426（单事务级联、旁的项目不受影响，直读仓核对） |
| T-UP-4 | ✅ | repository.test.ts:428-477（失败可观察 + 内存视图 + savePatternDoc 拒绝 + 无 IDB 降级三测） |
| T-UP-5 | ✅ | patterns.test.ts:58-103（长度/越界/未知与不可索引 paletteId/非 Int16Array/坏出处只丢出处）+ repository.test.ts:479-506（手改仓里的越界文档读回 null） |
| T-UP-6 | ✅ | repository.test.ts:508-541（剪枝连行删 + updatedAt 收敛，直读仓核对） |
| T-UP-7 | ✅ | repository.test.ts:107-127（纯函数半）+ :374-383（迁移盖章半）；`isInventoryEntry` 拒未知空间 |
| T-UP-8 | ✅ | decode.test.ts:73-105（含边界放行与「头过闸走到 drawImage」的防呆测） |
| T-UP-9 | ✅ | upload.test.tsx:93-126（png/jpg/大写扩展名收；gif/webp/无类型拒 + 面板不出现） |
| T-UP-10 | ✅ | upload.test.tsx:128-148（确定性 fixture；判定 + 置信度徽标；手动覆盖改判） |
| T-UP-11 | ✅ | upload.test.tsx:150-167（徽标以结果为准，开关禁用 + 注明） |
| T-UP-12 | ✅ | upload.test.tsx:169-263(前三测)（28/56、aspect 28×14、手动 12×12、999→56 夹取） |
| T-UP-13 | ✅ | upload.test.tsx:266-276（全透明图：说明 + 双 CTA 禁用） |
| T-UP-14 | ✅ | upload.test.tsx:278-353（含空标题回落与注入 reject） |
| T-UP-15 | ✅ | repository.test.ts:543-581（枚举四仓键集 + 违禁词扫描） |
| T-UP-16 | ✅ | assemble.test.tsx「转换项目走同一条会话」前三测（同一套步进/模式/游标断言在转换板上重演） |
| T-UP-17 | ✅ | 同 describe 后两测（新版 NO_GRID_NOTE 全文 + 永不 settle 的加载态） |
| T-UP-18 | ✅ | inventory.test.ts「T-UP-18 G07 命名空间对撞」五测（§1.2） |
| T-UP-19 | ✅ | inventory.test.tsx「T-UP-19」三测（页面合并、画廊库存不抵扣、显式加载态） |
| T-UP-20 | ✅ | inventory.test.ts T-INV-5 整串升级（【画廊】标签）+ `not.toMatch(/http|:\/\//)` 保留（:290-292） |
| T-UP-21 | ✅ | isolation.test.ts 递归 src 未改动，429 行新源码自动入扫并绿 |

Do-not-regress：navigation / persistence-banner / empty-states / theme-backdrop / oracle-parity /
全部 `algo/` 既有测试在 29 文件里原样全绿；inventory 既有 T-INV-* 在原文件内改写未删（helper 缺省
`GALLERY_PALETTE`，逐条语义保持）；`repository.test.ts` 既有用例只增不改。

## 3. 四个实现者旗标的核实（不当福音，逐个对着树验）

1. **realm-safe Int16Array brand check —— 属实。** patterns.ts:95-98 用
   `Object.prototype.toString.call(value) !== "[object Int16Array]"` 判品牌（structured clone 跨 realm
   时 `instanceof` 会误拒），再 `Int16Array.from` 复制回本 realm 构造器；普通数组仍被拒
   （patterns.test.ts:93 钉住），契约的「必须 typed array」没有被顺手放松。
2. **无 IDB 降级下 pattern 写拒绝不翻 `persistenceFailed` —— 属实且有测试。** repository.ts:174-183：
   拒绝即全部答案，注释写明翻信号会把还活着的元数据仓拖去空内存副本（丢真数据去报别仓的错）。
   repository.test.ts:460-476 断言拒绝后 `isPersistenceFailed() === false` 且项目仍在 localStorage。
   注意与 bead-v1 打开失败的场景区分：那里横幅**应该**亮（open 失败已经翻过），T-UP-4 第二测正是断言
   拒绝 + 横幅同现——两个场景各自正确。
3. **D-UP-7 拖拽 + 方向键 —— 属实。** 键盘：四方向 + Shift ×10 步长（UploadWorkbench.tsx:258-270）；
   拖拽：pointer 事件（触屏同路）+ `setPointerCapture` + 实测视口矩形换算源像素（:276-296）。
   两条交互各有集成测试（upload.test.tsx:204-263），拖拽测里对夹取到 50% 的几何断言算得对
   （16px 源、8px 框、160px 视口、80px 拖动 → 右移 8 源像素到头）。
4. **NO_GRID_NOTE 文案 —— 属实。** AssemblePage.tsx:18-19 与 IA §5 建议句逐字一致；测试断言全文
   且不再提 WP-B03/上传（assemble.test.tsx T-UP-17）。

## 4. 发现（按严重度）

### HIGH — 无

### MED — 无

### LOW

- **LOW-1 · localStorage 降级仓的 `deleteProject` 不级联 progress**（repository.ts:184-187）。
  §5.3 的唯一跨仓不变量在 bead-v1（bead-v1.ts:370-388）与内存桩（repository.ts:266-272）都成立，
  唯独降级仓只删了会话内的 pattern Map，游标留在 blob 里。今天零后果：`deleteProject` 无 UI 调用方，
  且下一次 `saveProgress` 的 sanitize 会把孤儿剪掉（自愈）。但三个实现对同一契约面的行为分叉是
  未来删除 UI 只在两个后端上测试时的暗坑。修复约 3 行（read-modify-write 掉 progress），留给后续小 PR。
- **LOW-2 · 保存被拒后文档留在内存视图里**。两个降级路径都先 `patterns.set` 再 throw
  （bead-v1.ts:348-351、repository.ts:177-182）：一篇被告知「没有保存」的文档在会话内仍可被
  `loadPatternDoc` 读到。今天不可见（项目未 mint，无人会拿这个 id 来问），但「报告失败却留货」
  与 DATA-1 的诚实哲学有一丝出入；把 set 挪到写成功之后即齐。
- **LOW-3 · 重算窗口整块结果区卸载**。参数一变 `options` 身份即变，`result` 立刻为 null，
  预览、标题输入框、双 CTA 全部消失再回来（UploadWorkbench.tsx:206-216、475）。标题的 state
  不丢、busy 态有「转换中…」，纯属视觉抖动；保留上一帧预览 + 蒙层是更平滑的做法，非必改。
- **LOW-4 · `readStoreEntries` 是测试用直连却出在生产模块**（bead-v1.ts:392-408）。它绕开写序列化
  链自开连接，放在产品源码里等于邀请未来的生产调用。会被 tree-shaking 剪掉、今天无危害；
  更合适的家是 `src/test/`。

### 观察（不计数）

- 迁移遇到**损坏**的旧 blob 时照样删键（解析回空态后事务提交）——与 `parsePersistedState`
  「坏数据丢弃」哲学一致，但意味着手改坏的 blob 无法靠再次打开抢救；边界极窄，记录即可。
- 盖章发生在 `toInventoryEntry`（每次解析）而非仅迁移时刻——语义超集，见 §1.2，判定为优于字面。
- 「加入待拼」成功后再点会再 mint 一个新项目（新 id 新 doc）——与详情页可重复实例化的语义一致，
  不算缺陷；若要挡，禁用已保存态的 CTA 即可。
- `onblocked` 直接拒绝打开（bead-v1.ts:102）——多标签页升级窗口退化为内存会话 + 横幅，
  DATA-5 已接受的边界，v1 无升级路径时几乎不可达。
- debounce 覆盖所有参数变更而不只滑杆（60ms），一条代码路径，无害。

## 5. 残余风险对账（R-UP-1…5）

- **R-UP-1 · Worker 顺延**：按 §7 落地为主线程 + `aria-busy`，父代理未否决。全树无 Worker；
  最坏面被 DATA-3 的 4096² 上限封顶，管线仍是纯函数，日后换线程零 API 破坏。风险维持记录，无恶化。
- **R-UP-2 · 孤儿 PatternDoc**：真实存在（doc-先-项目-后的设计代价），`deleteProject` 已随契约落地
  作为回收通道；另见 LOW-2 的会话内存孤儿变体。维持「记录不修」。
- **R-UP-3 · 迁移盖章误伤**：实现即 IA 预告的行为（无 paletteId 一律 gallery），测试钉住；
  UI 从未提供过录 generic 码的路径，边界维持已知。
- **R-UP-4 · /inventory 双源时序**：显式加载态已做进 ShortagePanel 且被 T-UP-19 第三测
  （永不 settle 的 loadPatternDoc）钉住；`useConversionDocs` 以 id 串为 key，避免无关重渲染重读网格。缓解到位。
- **R-UP-5 · 两套存储并存**：theme 留 localStorage、档案进 IDB，读回校验（T-UP-5 + 迁移消毒）
  延续 `parsePersistedState` 哲学；devtools 手改面翻倍的事实不变，维持记录。

## 6. 验收记录

```
pnpm --filter @bead/app test
Test Files  29 passed (29)
     Tests  426 passed (426)      ← 父代理门禁 29/426，命中

pnpm --filter @bead/app lint     ← tsc --noEmit && eslint . 零输出
```

父代理本地门禁照录：独占吸收面上 29 文件 / 426 条全绿、lint 干净。托管 `bead.yml` 两个 job
因 billing 落在空 runner 上失败——与 `a5a69c4` 记录的同一基础设施况，非产品失败，不影响本判定。

审查环境：`cursor/bead-r3-b03-review-c441` @ 基于 `bb4ce71`（即最新 `cursor/beadflow-integration-c441`），
独立 worktree，冻结 lockfile 安装。

## 7. 禁区自查

本轮只新增 `docs/bead/reviews/round3-create-upload-review.md`。未改 `apps/bead/**`、
`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、`apps/desktop/**`、
`crates/soul-*/**`、根 Cargo members、`deny.toml`、`.github/**`。本文无外网 URL（BD15）。
