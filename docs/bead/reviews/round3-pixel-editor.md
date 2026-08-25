# ROUND 3 · WP-B06 像素编辑器 — 前端 / 数据 IA

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只做 IA / 拆分，不实现）。
基线：`origin/cursor/beadflow-integration-c441` @ `b9e8866`。
输入：`docs/bead/PLAN.md`（像素编辑器一句：对称、油漆桶、拾色器、色号替换、颜色统计）、`docs/bead/WORK_PACKAGES.md`（WP-B06）、
`docs/agent-decisions.md` BD14 / BD15 / BD19 / BD20、`round1-frontend.md`（D-UI-1/2/3/4/5/7、§5 状态所有权）、
`round1-map.md` §4 B06（NO_HIGH_VALUE until B03 网格模型定型——该前提已被 `round3-create-upload.md` 解除）、
`round2-data.md` §5（bead-v1 四仓）、`round3-create-upload.md`（D-UP-1…16、§2 契约对账、T-UP-*）、
`round2-assemble.md`（D-ASM-4/6/9/12/13、session.ts 先例）、`round2-inventory.md`（D-INV-2/13、BD20 触发点）、
`apps/bead/src/pages/create/CreatePage.tsx` + `CreatePage.test.tsx`、`algo/`（grid / palette / bom / steps / framing）、
`stores/`（types / repository / store / projects / ids）、`pages/assemble/`（AssemblePage / AssembleCanvas / session）、
`pages/workspace/WorkspacePage.tsx`、`fixtures/grids.ts`、`app/routes.tsx`。
**未读**：B03 实现工作树（`/tmp/wt-b03`）的任何未提交文件——本文只对 exclusive tip + B03 IA 成文。
硬禁区自查：本轮只新增本文件；不触碰 Soul 锁、`apps/desktop`、`crates/soul-*`、根配置、`deny.toml`、`.github/**`、锁文件。

---

## 0. 结论先行（本 WP 的法律）

1. **B06 编辑的是已持久化的 `PatternDoc.cells`，仓就是 B03 落的 bead-v1 `patterns` 仓。**
   编辑器全部读写只走 `loadPatternDoc(id)` / `savePatternDoc(doc)`（round3-create-upload §2.2 钉死的
   三方法之二），**零新仓、零新 Repository 方法、零 localStorage 网格**。任何把 Grid JSON 化塞进
   `bead.state` 或另开第二套存储的设计在本文直接判死——BD19 的触发点已由 B03 兑现，B06 是纯消费者。
2. **空白项目 = 先 mint 文档、再开编辑器。** `/create?entry=blank` 面板收集标题与板型，确认时
   `await savePatternDoc(空文档)` **成功后**才 `addProject`（D-UP-10 的 doc-先-项目-后次序原样复用），
   然后导航进编辑器。项目永不指向不存在的文档；保存失败 = 内联错误、不 mint 项目。
3. **硬时序**：B06 实现必须切在 **B03 实现合入之后的 tip** 上。本文的每个契约引用
   （`PatternDoc`、`stores/patterns.ts` 编解码、`addProject`、`fake-indexeddb`）都是 B03 实现的交付物；
   B03 实现评审是 B06 开工的闸门（§6 R-ED-2）。
4. **算法侧零发明**：镜像 / 洪泛 / 全局替换是编辑操作，不是转换管线——纯函数落**页面私有**
   `pages/edit/editor.ts`（`pages/assemble/session.ts` 先例），**不进 `algo/`**。`algo/` 是 oracle parity
   域（BD14/BD18：contract.md + parity 测试对齐 rust），编辑器在 rust 侧无对应物，塞进去会玷污
   parity 边界。BD14 不受影响：不建新包、不动 `pnpm-workspace.yaml`。颜色统计**复用** `buildBom`
   （import 方向 pages→algo 合法，D-INV-10 层向）。

## 1. 决策表（D-ED-*，实现者照抄，不自行发明产品法）

| # | 决策 | 内容 |
|---|---|---|
| **D-ED-1** | 编辑对象 | 可编辑集合 = `sourcePatternId === null` 且 `loadPatternDoc` 命中的项目（空白 + 转换修补）。画廊项目（`sourcePatternId !== null`）**不可编辑**：fixture 网格是构建期只读代码、还是 catalog 颗数的权威（T-ASM-1），且其色板是逐图纸的画廊码——PatternDoc 只认 `generic-5mm`（D-UP-12），装不下。「Fork 改色」归 WP-B08（fork 时的画廊→G 码量化是 B08 的决策，本文只记录该缺口，§9 R-ED-3） |
| **D-ED-2** | 转换项目修补 | 转换文档**允许像素级编辑**（转完手工修杂点是编辑器的头号用例）。这与 round3-create-upload §6「v0 转换文档不可变」一句表面冲突——该句管的是 B03 自己不做「保存后改参数 / 重新框定」，不是禁掉编辑器 WP；仍列为显式偏离供父代理否决（§4 DEV-ED-2）。`provenance` 字段编辑后原样保留（出处近似化，展示用，R-ED-4） |
| **D-ED-3** | 路由 | **新路由 `/edit/:id`**，AppShell 内（有底栏，无高亮项，同 `/pattern/:id` 先例），`:id` 只收 `proj-*`（D-UI-4）。理由压着 D-UI-1：正在编辑的项目是可分享、可后退、可从 Workspace 重进的状态，恰恰**必须**进 URL；塞进 `/create?entry=blank&project=…` 会把静态入口面板改造成双语义路由，破坏 ENTRIES 面板与其测试的不变量。这是 round1 路由表之外的第一条新路由，列为显式偏离（§4 DEV-ED-1） |
| **D-ED-4** | 空白入口 | `/create?entry=blank` 面板位（D-UI-1，不开新路由做表单）从占位说明改为**新建表单**：标题输入（trim ≤64，空 → 「未命名豆图」，先例 D-UP-11）+ 板型段选 + CTA「新建并开始编辑」。blank 的 `owner` 改 `null`（「现在就能用」），detail 文案改写。确认序列：`savePatternDoc({projectId, paletteId:"generic-5mm", width, height, cells: 全 -1, 无 provenance}) → addProject(status:"draft") → navigate(/edit/:id)`。空白项目落 **draft**（Workspace「草稿与设计」段就是为它设计的），backdrop 默认与 `createProjectFromPattern` 一致（black / `#101014`）。新纯函数 `createProjectFromBlank` 进 `stores/projects.ts`，Project 形状不加字段 |
| **D-ED-5** | 板型 | 空白板型两预设：**28×28（默认）/ 56×56**，镜像 D-UP-6 的固定板档；56/轴上限继承 D-UP-7（DOM 画布 D-ASM-6 与 v0 方板范围的同一条天花板）。自由长宽、六角、圆板、多板拼接全部不进 v0（round1-map §3 已推迟；PatternDoc 的 width/height 本身支持任意值，收窄发生在 UI 层）。编辑器内**不提供改尺寸**（重开新画板即可，v0 不做画布 resize） |
| **D-ED-6** | 色板空间 | `paletteId` 恒 `"generic-5mm"`。选色器 = `GENERIC_5MM` 全部 48 条（G01–G48），每格 ColorSwatch + 码文本（色号永不只靠颜色）。画廊命名空间对文档不可用：画廊色板是逐图纸四条的局部表，没有全局注册表可供 `paletteId` 引用。BD20 机器（`(paletteId, code)` 身份、G07 双行独立）由 B03 落地，空白项目的 BOM 自动进 generic-5mm 需求侧，B06 零库存改动 |
| **D-ED-7** | 画布 | 编辑器自有 `<EditorCanvas>`：DOM 网格（D-ASM-6 论证原样成立，≤3136 节点），复用 `.assemble__bead` 的渲染技术但**不复用 `<AssembleCanvas>`**——它的 `cellStates`/`role="img"`/`aria-hidden` 语义是拼装会话的（D-UP-9 的 `<ConversionPreview>` 同一先例）。**`AssembleCanvas` API 零改动**。格子是非聚焦 `span`（`data-x`/`data-y`），指针事件经 React 委托；容器 `data-testid="editor-grid"` |
| **D-ED-8** | 工具模型 | 工具 = 画笔 / 油漆桶 / 拾色器（+ 替换面板、统计面板，非画布工具）。**活动色 = G 码 ∪ 「空」（擦除）**：画笔配「空」即橡皮、油漆桶配「空」即区域清除——不设独立橡皮工具，状态面少一半。笔画 = pointerdown→up 扫过的格集合（pointermove 进行中批量更新，不逐格 setState 抖动） |
| **D-ED-9** | 对称 | 四档：关（默认）/ 左右镜像 / 上下镜像 / 四向。28/56 皆偶数宽：镜像轴在列间，`mirror(x) = width-1-x`，无自映射格。**只作用于画笔笔画**（含擦除）：落点集合 = {原格, x 镜像, y 镜像, 双镜像} 按档位取子集去重。油漆桶 / 替换 / 拾色器**不受对称影响**（镜像种子洪泛的结果不可预期，区域语义与镜像语义正交）。八向（对角）不进 v0（§10） |
| **D-ED-10** | 油漆桶 | **4-连通**洪泛（与 outline-infill 的 4-连通判定同一裁决系，G6 血统），种子格的当前值（**含「空」——空区可灌，这是空白画板的主要用法**）为匹配值，整个连通区改写为活动色。种子值 == 活动色 ⇒ no-op（不产生历史条目）。板边即边界，无对角渗漏 |
| **D-ED-11** | 拾色器 | 单发：点格 → 活动色 = 该格码；点空格 → 活动色 = 「空」（一致性，顺手变橡皮）。取色后自动回到画笔（单发工具，不留驻） |
| **D-ED-12** | 色号替换 | **全文档全局替换**（v0 无选区工具，无选区语义可挂）：源码下拉 = 文档实际用到的码（带当前颗数），目标 = 48 码全表 ∪ 「空」；应用前显示「将改写 N 颗」。源 == 目标 ⇒ 按钮禁用。纯函数 `replaceColor(grid, from, to)` |
| **D-ED-13** | 颜色统计 | 活 BOM = `buildBom(grid, GENERIC_5MM)` 每次编辑后 memo 现算（count 降序 + 下标升序是 `bom.ts` 既有排序），行 = 码 / 名 / 色块 / 颗数 + 合计。**行可点**：点统计行 = 把该码设为活动色（高频取色捷径）。派生数据禁止持久化（D-INV-13 / IA §5 原样延伸） |
| **D-ED-14** | 撤销/重做 | **会话内存，永不持久化**（持久化编辑历史 = 给 PatternDoc 造隐形 schema 增量，判死）。粒度 = 整笔画 / 一次洪泛 / 一次替换（不是逐格）；实现为格增量补丁栈，上限 100 条，溢出丢最旧；新操作清空 redo 栈。快捷键 Ctrl/Cmd+Z、Ctrl/Cmd+Shift+Z 与 Ctrl+Y；事件目标是表单控件时忽略（D-ASM-13 先例）；工具条同时给可点按钮（撤销在栈空时禁用） |
| **D-ED-15** | 写盘 | **防抖自动保存**：任何网格变更后 ~800ms 静默期触发 `savePatternDoc`（整篇 put，≤6.3KB），卸载 / `pagehide` 时 flush。不做显式保存键、不做 D-UP-10 式双 CTA——文档在进编辑器前已存在，编辑不是铸造。保存状态微文案（`role="status"`：保存中… / 已保存 / 保存失败）；`savePatternDoc` 拒绝 ⇒ 状态文案「保存失败」+ 编辑继续内存态，全局 `PersistenceBanner`（DATA-1 轨道）照旧，不新增错误 UI（D-UP-16 同源） |
| **D-ED-16** | 进度游标 | 网格变了步序就变，陈旧 `stepIndex` 指向错误格子比丢进度更糟。规则：**自动保存成功时，若该项目存在 `ProgressCursor`，经既有 `upsertProgress` 把 `stepIndex` 归 0，`mode` 与 `elapsedMs` 不动**（D-ASM-4 模式切换的镜像裁决：elapsedMs 是项目累计用时）；无游标则不创建。编辑器在检测到游标存在时常显一句「编辑会使拼装进度回到第 1 步」。列为显式偏离（§4 DEV-ED-3） |
| **D-ED-17** | 入口链接 | 三处：① `/create?entry=blank` 新建即进；② Workspace 卡片：`sourcePatternId === null` 的项目加「编辑豆图」链接 → `/edit/:id`（草稿卡目前只有标题、是死端，此链接兼救它；画廊项目卡不出该链接）——这是 B06 对 `WorkspacePage.tsx` 的**唯一**批准改面（§4 DEV-ED-4）；③ 编辑器头部 CTA「去拼装」：`occupiedCount > 0` 时可用，status 为 draft 则先 `setProjectStatus(id,"active")` 再导航 `/assemble/:id`（已是 todo/active/done 则只导航不改状态）。空画板不给拼装口（镜像 D-UP-9 空网格禁保存的语义，这里门在推进不在保存） |
| **D-ED-18** | 守卫链 | `/edit/:id` 依序：hydration（「正在读取本地项目……」）→ `isProjectId` → 项目存在 → `sourcePatternId === null`（否则「画廊图纸的网格是只读的——Fork 改色归 WP-B08」）→ `loadPatternDoc` 命中（加载态「正在读取豆图……」；未命中 → 「这个项目还没有豆图文档」）。全部守卫屏不自动跳转（保住后退键，AssemblePage 先例），均带「回拼装台」出路 |
| **D-ED-19** | 测试改写映射 | `CreatePage.test.tsx` 的 B06 占位断言**保留到 B06 实现 PR 才动**，改写与实现同 PR、独立 commit（B03 §5 的同一纪律），映射表见 §5.4。B07 两条占位断言原样保留，不许顺手动 |
| **D-ED-20** | 落点 | 新目录 `pages/edit/`：`EditPage.tsx`（守卫链 + 框架）、`EditorCanvas.tsx`、`EditorTools.tsx`（工具/对称/选色/替换/统计可再拆）、`editor.ts`（纯函数 + reducer，无 DOM）。样式进 `app.css`。`stores/patterns.ts` 的 Grid↔Int16Array 编解码**直接复用**，不写第二套 codec |
| **D-ED-21** | 画布尺寸 | 固定舒适格径（如 12–14px），容器双轴滚动兜底；56×56 ≈ 672–784px，1280 视口无滚动可见（布局约束，不进 jsdom 测试）。不做缩放/平移控件（§10）；不做 1:1 物理模式（那是拼装的透光需求，编辑器无此用例） |

## 2. 与 B03 契约对账（round3-create-upload §2 的消费端）

1. **Repository 面零增量。** `loadPatternDoc(id)` / `savePatternDoc(doc)` / `deleteProject(id)` 三方法
   签名一字不动；数组形 `loadProgress()` / `saveProgress(cursors)` 照旧——**不得**为编辑器复活
   §5.3 旧稿的逐项目 `loadProgress(id)` 撞名签名（round3-create-upload §2.2 的裁决维持）。
   游标归零走既有 `upsertProgress` action，零仓库改动。
2. **`savePatternDoc` 语义 = upsert**（keyPath `projectId` 的 put）。编辑器自动保存整个建立在这上面；
   若 B03 实现把它做成了 insert-only，那是 B03 实现评审要抓的缺陷，不由 B06 打补丁。
3. **读回校验对称**：编辑器写出的文档必须过 T-UP-5 的同一校验（长度 = width×height、每格 ∈ {-1} ∪
   [0, 48)）。选色器绑死 `GENERIC_5MM.entries`，越界索引在 UI 上不可构造；codec 复用保证编码一致。
4. **`createProjectFromBlank` 与 `createProjectFromConversion` 并排**进 `stores/projects.ts`；
   store 复用 B03 落的 `addProject` 通路（reducer case 已在基线树存在）。转换项目判别式
   `sourcePatternId === null ∧ doc 命中` 不变，空白项目天然同型——**不加 kind 字段**（D-UP-11 的
   「不加字段 = 不造双权威」对空白项目同样成立）。
5. **拼装与库存零改动即收编**：D-UP-14 的二分支（`null` → `loadPatternDoc` → 适配 → `AssembleSession`）
   对空白项目自动成立——画出来的板直接可拼；D-UP §4.3 的 /inventory 转换需求 hook 按
   「todo/active ∧ `sourcePatternId === null`」加载文档，空白项目经 D-ED-17 ③ 提为 active 后自动进
   缺口。**B06 不碰 `pages/assemble/**`、`pages/inventory/**`、`stores/inventory.ts` 任何一行。**
6. **空文档合法性对账**：D-UP-9 的「空网格禁止保存」是**上传工作台的规则**（转换出空板 = 转换失败），
   不是 `patterns` 仓的不变量——空白文档生来全空、必须能存。B06 对应的门挪到推进侧：空画板不给
   「去拼装」（D-ED-17）；已提升项目被擦空则 AssemblePage 的空网格态兜底。
7. **`deleteProject` 仍无 UI 调用方**：B06 不做项目删除界面（维持 B03 §2.2 的记录，归后续 WP）。

## 3. 状态所有权增量（round1 §5 表的 delta）

| 状态 | 拥有者 | 持久化 | 写者 | 读者 |
|---|---|---|---|---|
| 编辑会话：工具、活动色、对称档、撤销/重做栈、保存状态 | EditPage 页内 reducer | **禁止** | 页内 | 页内 |
| `PatternDoc.cells`（编辑中网格） | Repository | IDB `patterns` 仓 | **第二个写点**：编辑器防抖自动保存（第一个是 D-UP-10 双 CTA；两者都只经 `savePatternDoc`） | 编辑器加载、AssemblePage 适配、/inventory 需求 hook |
| 步游标 | ProjectStore `progress` | IDB `progress` 仓（形状不变） | 既有 D-ASM-8 时刻 + **D-ED-16 的归零 upsert** | AssemblePage 恢复 |
| 活 BOM / 替换预览计数 | 无（memo 现算） | **禁止** | — | 统计面板 / 替换面板 |
| 项目元数据（新增 draft 空白项目） | ProjectStore | IDB `state` 仓（整数组语义不变） | `addProject` / `setProjectStatus` 既有 action | Workspace / 守卫链 |

## 4. 显式偏离（父代理可否决，逐条给回退方案）

| # | 偏离 | 内容与理由 | 被否决时的回退 |
|---|---|---|---|
| **DEV-ED-1** | 新路由 `/edit/:id` | round1 路由表首次扩一条。理由见 D-ED-3：编辑目标是可深链、可后退、可重进的状态，D-UI-1 的原则本身要求它进 URL；`/pattern/:id` 收 `gal-*`、`/assemble/:id` 是沉浸 chrome，都不能收编 | 编辑器渲染在 `/create?entry=blank&project=<id>`；代价：ENTRIES 面板双语义、Workspace 回跳链接丑化、CreatePage 测试面翻倍 |
| **DEV-ED-2** | 转换文档可编辑 | 对 round3-create-upload §6「转换文档不可变」的显式改写（D-ED-2 论证：该句语境是 B03 不做重新框定，转后修杂点是编辑器的核心价值） | 守卫链加一条「文档带 provenance ⇒ 只读」，编辑器 v0 只服务空白项目；一处守卫 + 两条测试的改动量 |
| **DEV-ED-3** | 网格变更使游标归零 | 编辑器自动保存成功时 `stepIndex → 0`（mode/elapsedMs 不动，D-ASM-4 镜像）。动了 B04 落的进度语义 | 不归零，接受陈旧 stepIndex 被消费端夹取后指向错格；用户自行切模式复位 |
| **DEV-ED-4** | Workspace 卡片加「编辑豆图」 | B06 对 `WorkspacePage.tsx` 的唯一改面：`sourcePatternId === null` 的卡片一条链接（草稿卡目前是无 CTA 死端）。越此即 drive-by | 只保留 `/create` 新建入口，草稿不可重开——功能残缺，不建议 |

## 5. 验收测试矩阵（T-ED-*）

### 5.1 纯函数（新 `pages/edit/editor.test.ts`，无 DOM）

| # | 断言 |
|---|---|
| T-ED-1 | 镜像落点：偶数宽 `mirror(x)=width-1-x`；四档各自的落点集合正确且去重；关 = 单格 |
| T-ED-2 | 洪泛：4-连通不对角渗漏；空区可灌；整板灌注；种子值 == 活动色 ⇒ no-op 且不产生历史条目；「空」作为填充值清区 |
| T-ED-3 | 全局替换：改写颗数正确、其余码不动；目标「空」= 擦除；from == to 拒绝 |
| T-ED-4 | 撤销/重做 reducer：整笔画 = 一条历史；undo/redo 往返回到逐格一致；新操作清 redo；101 条时最旧被丢；栈空时 undo no-op |
| T-ED-5 | 拾色器：取到码 / 空 ⇒ 活动色更新且工具回画笔（reducer 级断言） |

### 5.2 存储（`fake-indexeddb`，扩 `stores/patterns.test.ts` 或新页面级存储用例）

| # | 断言 |
|---|---|
| T-ED-6 | 空白文档（全 -1、无 provenance）经 bead-v1 往返不变，且过读回校验；编辑后文档同样往返（T-UP-1 的编辑器侧对偶） |
| T-ED-7 | 存储卫生：编辑一轮后枚举各仓与 localStorage 键——网格只在 `patterns` 仓一份，无 `Step[]`、无 BOM、无撤销栈、无第二份网格（T-UP-15 口径延伸） |

### 5.3 集成（`renderApp` harness + 内存仓库双身，fake timers 管防抖）

| # | 断言 |
|---|---|
| T-ED-8 | 空白铸造：`?entry=blank` 填表 → 文档在仓（尺寸、全空、generic-5mm）→ 项目 status=draft → 落在 `/edit/:id`；`savePatternDoc` 注入 reject → 内联错误、项目数不变（D-UP-10 次序） |
| T-ED-9 | 守卫链四态：未知 id / 画廊项目（文案点名 WP-B08）/ 文档未命中 / 加载中，各屏有出路、无自动跳转 |
| T-ED-10 | 画笔：点格上色；活动色「空」擦除；四向对称一笔四格 |
| T-ED-11 | 油漆桶灌空区；拾色器取色后回画笔（UI 级） |
| T-ED-12 | 替换面板：源列表只列在用码并带颗数；应用后画布与统计同步更新 |
| T-ED-13 | 统计面板 = `buildBom` 输出（count 降序 + 下标升序）；点行换活动色 |
| T-ED-14 | 撤销/重做按钮与快捷键等价；焦点在标题输入/下拉时按键不触发（D-ASM-13）；栈空按钮禁用 |
| T-ED-15 | 自动保存：改格 → 800ms 后仓内文档已更新；卸载 flush；重进编辑器读回同一网格 |
| T-ED-16 | 游标归零：seed 游标 {mode:"row-by-row", stepIndex:5, elapsedMs:E} → 编辑并保存 → {stepIndex:0, mode 不变, elapsedMs:E}；无游标 seed ⇒ 保存后仍无游标；游标在场时提示句可见 |
| T-ED-17 | 保存失败：注入 reject → `role="status"` 出「保存失败」，画布编辑继续可用，PersistenceBanner 行为不变 |
| T-ED-18 | Workspace：空白 draft 卡出「编辑豆图」→ `/edit/:id`；画廊项目卡无此链接 |
| T-ED-19 | 闭环收编：画几格 → 「去拼装」→ status 变 active、`/assemble/:id` 经 D-UP-14 分支渲染 `assemble-grid`（swatch 来自 generic-5mm）；空画板时「去拼装」不可用 |
| T-ED-20 | NE-1 静态扫描自动覆盖 `pages/edit/**` 全部新文件（`isolation.test.ts` 递归 src，无需改动）：零 `fetch`/XHR/WebSocket/sendBeacon/外网 URL 字面量（BD15/BD17，含注释） |

### 5.4 既有测试改写映射（与实现同 PR、独立 commit，禁止静默破坏）

前提：B03 实现已按其 §5 落地（用例 1 的 B03 断言已换、B06/B07 断言保留）。B06 PR 在那之上：

| 现有断言（B03 落地后的 `CreatePage.test.tsx`） | 处置 |
|---|---|
| 用例1：`getByText("尚未实现，归 WP-B06")` | **替换**为 blank 入口标注「现在就能用」（`owner: null` 既有渲染路径） |
| 用例1：`WP-B07 ×2` 断言 | **保留原样**（B07 仍是占位，不许顺手动） |
| blank 入口说明页断言（若 B03 后仍是占位说明形态） | **整体替换**为新建表单测试（T-ED-8 前半：表单在、无占位说明） |
| 画廊入口用例 | **不动** |

连带：`WorkspacePage` 相关既有测试若断言草稿卡结构，按 D-ED-17 ② 在原文件内改写不删用例；
`assemble.test.tsx`、`navigation.test.tsx`、`empty-states.test.tsx`、inventory 全部测试、`algo/` 全部
测试原样保绿。全绿口径 `pnpm --filter @bead/app test`；提交前根 `just ci` + e0-audit / denylist-audit
照 B10 惯例（B06 零新依赖，锁文件不动）。

## 6. 消毒 / 不出网

- **图像永不进入 B06**：编辑器无文件输入、无拖拽、无粘贴；「从上传起手再修图」的组合路径 =
  先走 B03 上传工作台保存、再从 Workspace 进编辑器——已有 decode 门（DATA-3 上限）之外零新解码入口。
- 零外网 URL 字面量（含注释，BD15/BD17）；本文件与全部新源码可通过 e0-audit 与 NE-1（T-ED-20）。
- 零新权限面：无 Clipboard API、无 pointer lock、无全屏 API（round2-data §7 的零弹窗面维持）。
- 零新运行时依赖（D-UI-7）；`fake-indexeddb` 已由 B03 落为 devDependency，B06 不动依赖面与锁文件。
- 无 `<canvas>`、无 `eval`、无 `dangerouslySetInnerHTML`；写盘只经 `savePatternDoc` 单点。

## 7. 非目标（实现者不得顺手做）

- 六角 / 圆板 / 多板拼接 / 自由长宽 / 画布 resize（D-ED-5）；缩放平移控件（D-ED-21）。
- 选区 / 套索 / 直线 / 矩形 / 图层 / 八向对称——五工具之外的编辑面全部 post-v0。
- 自定义颜色 / 色板编辑：`generic-5mm` 封闭 48 码（BD9 品牌色板照旧禁止）。
- 画廊网格编辑与 Fork 改色（B08）；`.beadproj`/png 从编辑器导出（B07）；项目删除与重命名 UI。
- 撤销历史持久化、编辑历史跨会话恢复（D-ED-14 判死）。
- `pages/assemble/**`、`pages/inventory/**`、`stores/repository.ts`、`stores/types.ts`、`algo/**`、
  `crates/**`、Soul 树、`.github/**`——B06 一个字节不碰。
- 状态锁（done 项目禁编辑之类）：v0 统一规则，不按 status 分叉编辑权。

## 8. 残余风险（R-ED-*）

- **R-ED-1 · 键盘作画不可及**：画布是指针驱动、格子非聚焦，纯键盘用户 v0 画不了图；选色、替换、
  统计、撤销全程键盘可用。记录为已知 a11y 边界，不在 v0 解（逐格焦点 = 3136 个 tab 停靠点，更糟）。
- **R-ED-2 · B03 实现漂移**：本文引用的 `stores/patterns.ts` 函数名、`addProject` 通路、内存仓库双身
  扩展都是 B03 IA 的规定动作；若 B03 实现偏离，B06 实现者以合入后的真实 tip 为准并在 PR 里注明差异。
  B03 实现评审是闸门。
- **R-ED-3 · Fork 色板缺口**：画廊 → generic-5mm 的量化 fork 未定案（B08 的决策），在此之前画廊项目
  在编辑器里是守卫屏。缓解：守卫文案点名 B08，用户有预期。
- **R-ED-4 · provenance 近似化**：转换文档被手工修补后 `{kind, ditherApplied}` 不再精确描述内容。
  展示用字段，接受不修。
- **R-ED-5 · 防抖窗口丢笔**：崩溃 / 强杀落在 800ms 窗口内会丢最后一笔。`pagehide` flush 覆盖正常
  关闭；单笔损失可接受，不上逐笔同步写（把 MB 级同步写砸回主循环正是 bead-v1 要避免的）。
- **R-ED-6 · 笔画渲染节奏**：pointermove 逐格 setState 会在 56×56 上抖；D-ED-8 要求笔画进行中
  批量合帧更新，列为实现要求（D-ASM-6 的 3136 节点结论只对属性级更新成立）。
- **R-ED-7 · 多标签页**：编辑器与拼装页同开时 last-writer-wins（DATA-5 已知边界），不另设防。

## 9. NO_HIGH_VALUE_CHANGE_FOUND 子树

- 编辑器内嵌参考图 / 描图底稿：等于把图像输入面搬进 B06，B03 工作台已覆盖「从图起手」。NO_HIGH_VALUE_CHANGE_FOUND。
- 最近用色 / 收藏色条：48 码单屏可见，统计面板行可点已是捷径。NO_HIGH_VALUE_CHANGE_FOUND。
- 笔刷尺寸（2×2、3×3）：v0 单格笔 + 油漆桶覆盖主路径。NO_HIGH_VALUE_CHANGE_FOUND。
- 编辑器暗持久化草稿（区别于文档本体的 draft 层）：自动保存即草稿，双层存储 = 双权威。NO_HIGH_VALUE_CHANGE_FOUND。
- `Project` 加 `kind`/`editable` 判别字段：`sourcePatternId === null` + doc 命中已判别（D-UP-11 同款裁决）。NO_HIGH_VALUE_CHANGE_FOUND。
- 触屏手势打磨（双指缩放等）：桌面 v0，指针事件天然兼容单指，其余后置。NO_HIGH_VALUE_CHANGE_FOUND。

## 10. Ready-for-Opus 清单（照此下刀，逐项可勾）

- [ ] 前置确认：B03 实现已合入 tip，`stores/patterns.ts` / `loadPatternDoc` / `savePatternDoc` / `addProject` 在场。
- [ ] `pages/edit/editor.ts`（新）：镜像 / 洪泛 / 替换 / 撤销栈 / 编辑 reducer 纯函数 + `editor.test.ts`（T-ED-1…5）。
- [ ] `pages/edit/{EditPage,EditorCanvas,EditorTools}.tsx`（新）：守卫链（D-ED-18）、画布（D-ED-7/21）、工具面（D-ED-8…13）、自动保存（D-ED-15）、游标归零（D-ED-16）、「去拼装」（D-ED-17 ③）。
- [ ] `app/routes.tsx`：AppShell children 增 `edit/:id`（DEV-ED-1 获准后）。
- [ ] `pages/create/CreatePage.tsx`：blank 入口 `owner: null` + 新建表单面板（D-ED-4）。
- [ ] `stores/projects.ts`：`createProjectFromBlank`（唯一 stores 改动）。
- [ ] `pages/workspace/WorkspacePage.tsx`：仅 DEV-ED-4 的一条链接。
- [ ] 测试：§5 全表；§5.4 映射改写为独立 commit；`pnpm --filter @bead/app test` 全绿 + 根 `just ci` 绿。
- [ ] 红线自查：零新依赖、零锁文件改动、零外网 URL（NE-1/e0 会抓）、`AssembleCanvas`/`repository.ts`/`types.ts`/`algo/**`/`crates/**` 零改动。

## 11. 禁区自查

本轮只新增 `docs/bead/reviews/round3-pixel-editor.md`。未改 `apps/bead` 任何源码（全部留给实现席），
未读取 B03 实现工作树的未提交文件，未触碰 `docs/PRODUCT_LOCK.md`、`docs/STATUS.md`、`docs/FORMAL_WORK_PROMPT.md`、
`apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml`、`deny.toml`、`.github/**`、`pnpm-lock.yaml`。
本文无外网 URL，仅引用仓内路径（BD15/BD17）。
