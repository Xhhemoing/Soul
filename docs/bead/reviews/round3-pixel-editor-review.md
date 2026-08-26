`claude-fable-5-thinking-xhigh`

# ROUND 3 · WP-B06 像素编辑器 `/edit/:id` · 实现审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审查，不实现）。
被审对象：吸收提交 `00cc9c7`（absorb WP-B06 pixel editor at /edit/:id，即 PR #48 的吸收面），
基线 `origin/cursor/beadflow-integration-c441` @ `c9f2828`（B03 复审合入后的 tip）。
审查依据：`docs/bead/reviews/round3-pixel-editor.md`（D-ED-1…21 / T-ED-1…20，本 WP 的法律）、
`round3-create-upload.md` §2（契约面）、`docs/agent-decisions.md`（BD14 / BD15 / BD19 / BD20）、
`round2-assemble.md` D-ASM-4/6/13、`round1-frontend.md` D-UI-1/4/7。
本轮只新增本文件；未改任何产品文件、测试文件、rust、Soul 树。

范围对账：吸收提交对 `apps/bead/**` 与实现分支 tip `a11358e` **逐字节一致**（`git diff` 空），
额外只有父代理记账 `docs/agent-progress.md`。实现分支三个 commit 满足 D-ED-19 的纪律：
`379b15d`（产品面）、`768d50b`（新测试 T-ED-1…20）、`a11358e`（**单独承载** CreatePage.test.tsx 改写，
与 B03 的 `6cf7e63` 同款）。零新依赖、零锁文件改动（BD16 不触发）。

---

## 0. 结论先行

**退回（REJECT）。1 条 HIGH，2 条 LOW。** 退回面极窄：HIGH-1 是 `EditorCanvas.tsx` 一个函数里
`setPointerCapture` 与 `event.target` 读格的组合错误——**真实浏览器里拖笔只能画下第一格**，
画笔的拖动笔画（D-ED-8 的核心手势）整个失效；jsdom 没有 `setPointerCapture`，488 条测试全部
走的是未捕获路径，绿灯是真的、但对这条缺陷是盲的。本审查在真实 Chrome 上双重复现：
最小复现（捕获开 = 0 个 extend 事件）与**真实构建产物**（六格拖动只落 1 颗）。修复约 5 行
（extend 里改用命中测试），修后本实现即 ACCEPT-WITH-NITS 成色——除此之外，D-ED-1…21 逐条成立、
六条硬红线全部锁死、`pnpm --filter @bead/app test` 实测 **32 文件 / 488 条全绿**（父代理门禁
32/488，命中）、`tsc --noEmit && eslint .` 零输出，浏览器上 mint → 编辑 → 自动保存 → 刷新回读
IDB 全通、localStorage 只有 `bead.theme`。实现者旗标（「不是缺陷」断言换家）**核实为正当，确认**。
不重开架构。

## 1. 任务钉出的硬红线（逐条对树验）

1. **Grid 不进 localStorage ✅。** 新源码全树无 `localStorage` 写者（仅 `editor-storage.test.ts`
   断言其为空）；T-ED-7（editor-storage.test.ts:85-121）编辑一轮后枚举 `patterns`/`state`/`progress`
   三仓 + 旧键：网格只在 `patterns` 仓一份，`state` 仓字符串里 `cells`/`grid`/`bom`/`past`/`future`
   等违禁词全部缺席。真实 Chrome 复核：编辑 + 刷新后 localStorage 键恰为 `["bead.theme"]`。
2. **仓库表面零增量、`loadProgress(id)` 未复活 ✅。** `stores/repository.ts`、`stores/types.ts`、
   `stores/store.tsx` 都不在 diff 里（对基线 `c9f2828` 零字节差）；全树 `loadProgress` 只有无参数组形。
   游标归零走既有 `upsertProgress` action（EditPage.tsx:160-165），写盘经既有 store effect。
3. **AssembleCanvas API 零改动、三个禁区目录零触碰 ✅。** `pages/assemble/**`、`pages/inventory/**`、
   `stores/inventory.ts` 对基线零字节差；编辑器自有 `<EditorCanvas>`（D-ED-7），借 DOM 网格技术
   不借组件。
4. **B07 占位断言未删 ✅。** `getAllByText("尚未实现，归 WP-B07")).toHaveLength(2)` 原样保留
   （CreatePage.test.tsx:35）；面板级「不是缺陷」断言换家到 import-pattern（见 §3，确认正当），
   B07 的占位覆盖是**加强**而不是删除。
5. **零外网 URL 字面量 ✅。** 新源码 grep 无 `http(s)://`/`fetch(`/XHR/WebSocket/sendBeacon；
   唯一颜色字面量 `#808080` 是兜底灰。isolation.test.ts 递归扫描自动覆盖 `pages/edit/**` 并新增
   扫描面自检（T-ED-20，:66-72）。
6. **编辑操作不进 `algo/` ✅。** 镜像/洪泛/替换/撤销栈全部落 `pages/edit/editor.ts`（页面私有，
   session.ts 先例）；`algo/**` 对基线零字节差。import 方向 pages→algo（buildBom / grid / palette），
   D-INV-10 层向合法，parity 边界未被玷污（BD14/BD18 不受扰）。

## 2. D-ED-1…21 与 T-ED-1…20 落点速查

| ID | 判定 | 落点 |
|---|---|---|
| D-ED-1 | ✅ | 守卫链只放行 `sourcePatternId === null` ∧ doc 命中（EditPage.tsx:88-96）；画廊项目守卫屏点名 WP-B08（:42-43），画布不渲染 |
| D-ED-2 | ✅ | 转换文档可编辑，`provenance` 读入后原样写回（EditPage.tsx:134、:147）；editor-storage.test.ts:72-82 钉住 |
| D-ED-3 | ✅ | 新路由 `edit/:id` 在 AppShell children（routes.tsx:29）；非 `proj-*` id 在同一守卫屏拦下（T-ED-9 第二测） |
| D-ED-4 | ✅ | BlankBoardForm.tsx:41-63：trim ≤64、空回落「未命名豆图」（:47）、`savePatternDoc` 成功后才 `addProject`（:52-60）、失败内联不 mint（:54-57）；`createProjectFromBlank` 落 `stores/projects.ts:61`（status draft、backdrop black/`#101014`、零新字段） |
| D-ED-5 | ✅ | 两预设 `BOARD_28`（默认）/`BOARD_56`（BlankBoardForm.tsx:29、:37）；编辑器无 resize 面 |
| D-ED-6 | ✅ | `paletteId` 恒 generic-5mm（经 `createPatternDoc`）；选色器 = 48 条 ColorSwatch + 码文本（EditorTools.tsx:128-138），`EDITOR_SWATCHES = paletteSwatches(GENERIC_5MM)`（editor.ts:26） |
| D-ED-7 | ✅ | 自有 `<EditorCanvas>`：DOM 网格、非聚焦 span + `data-x`/`data-y`、容器级指针委托、`data-testid="editor-grid"`（EditorCanvas.tsx:77-99）；AssembleCanvas 零改动。委托本身的浏览器缺陷见 HIGH-1 |
| D-ED-8 | ⚠ | 工具三件 + 「空」即橡皮（EditorTools.tsx:19-26、editor.ts:28-29）；笔画 = down→up 格集合、批量 dispatch（editor.ts:305-315、EditorCanvas.tsx:66-68）——reducer 模型完全正确，但真实浏览器里 pointermove 被捕获重定向，拖动笔画失效（**HIGH-1**） |
| D-ED-9 | ✅ | 四档、`width-1-x`、去重、只作用画笔（editor.ts:74-90、:281；油漆桶/替换绕过对称有 T-ED-2 末测钉住） |
| D-ED-10 | ✅ | 4-连通、空区可灌、种子==活动色 no-op 无历史、板边即边界（editor.ts:147-176） |
| D-ED-11 | ✅ | 单发取色（含「空」）、自动回画笔（editor.ts:322-326） |
| D-ED-12 | ✅ | 全文档替换；源下拉 = 在用码带颗数、对当前 BOM 逐帧重解析防陈旧（EditorTools.tsx:73）、「将改写 N 颗」（:176）、源==目标禁用（:75）；纯函数 `replaceChanges`（editor.ts:179-187） |
| D-ED-13 | ✅ | `editorBom = buildBom(grid, GENERIC_5MM)` memo 现算（editor.ts:204-206、EditPage.tsx:218）；行可点换活动色（EditorTools.tsx:200-214）；派生数据零持久化（T-ED-7 钉住） |
| D-ED-14 | ✅ | 会话内存栈、整操作粒度、100 条上限双落点（editor.ts:250、:313）、新操作清 redo、快捷键三式 + `typesInto` 忽略表单（EditPage.tsx:62-66、:187-202）、按钮栈空禁用 |
| D-ED-15 | ✅ | 800ms 防抖整篇 put（EditPage.tsx:141-174）、`pagehide` + 卸载双 flush（:178-185）、`role="status"` 微文案三态（:52-59、:237-239）、失败续编 + 不新增错误 UI（T-ED-17） |
| D-ED-16 | ✅ | 保存成功且游标在场才 `upsertProgress` 归零，mode/elapsedMs 原样（EditPage.tsx:159-166）；无游标不创建；提示句常显（:250）；`stepIndex` 已 0 时不空写 |
| D-ED-17 | ✅ | ① blank mint 直落 `/edit/:id`（BlankBoardForm.tsx:62）；② Workspace 三段卡片 `EditLink` 均以 `sourcePatternId === null` 把关（WorkspacePage.tsx:17-22、:34、:79、:100——§7 无状态锁，三段都出链接符合规约）；③ 「去拼装」`beads===0` 禁用、draft 先提 active（EditPage.tsx:224-228、:243） |
| D-ED-18 | ✅ | 守卫序恰为 hydration → isProjectId → 项目在 → 非画廊 → doc 命中（EditPage.tsx:92-96），五屏全部带「回拼装台」、零自动跳转（T-ED-9 断言 currentPath 不动） |
| D-ED-19 | ✅ | 改写与实现同 PR、独立 commit `a11358e`；B07 断言见 §1.4 与 §3 |
| D-ED-20 | ✅ | `pages/edit/{EditPage,EditorCanvas,EditorTools}.tsx + editor.ts` 四件套；样式进 app.css（+137 行，无 URL）；编解码直接复用 `stores/patterns.ts` 的 `createPatternDoc`/`decodeCells`，零第二套 codec |
| D-ED-21 | ✅ | 固定 12px 格径 + 双轴滚动兜底（app.css `.editor__grid`）；56×56 ≈ 745px，1280 视口内；无缩放/平移控件 |

| T | 判定 | 落点 |
|---|---|---|
| T-ED-1 | ✅ | editor.test.ts:44-107（四档落点/去重/关=单格/偶数宽无自映射/奇数宽折叠/Bresenham 补线/四向一笔四格） |
| T-ED-2 | ✅ | editor.test.ts:109-149（对角不渗漏/空区灌/「空」清区/no-op 无历史/板边/对称正交） |
| T-ED-3 | ✅ | editor.test.ts:151-174（颗数/他码不动/目标空=擦除/from==to 拒/缺席码 0 颗无历史） |
| T-ED-4 | ✅ | editor.test.ts:176-242（整笔一条/往返逐格一致/同格两笔回落笔前/零变更无条目/清 redo/101 丢最旧/栈空 no-op/桶与替换各一条） |
| T-ED-5 | ✅ | editor.test.ts:244-263（取码/取空 → 活动色 + 回画笔，reducer 级） |
| T-ED-6 | ✅ | editor-storage.test.ts:39-83（空白文档往返 + 读回校验 + 编辑后往返 + provenance 保留），每例自带 IDBFactory |
| T-ED-7 | ✅ | editor-storage.test.ts:85-121（§1.1 详述，T-UP-15 口径延伸成立） |
| T-ED-8 | ✅ | edit.test.tsx:79-152（doc 先项目后/56 档/空标题回落/注入 reject → 零 mint + 内联 alert + 不导航） |
| T-ED-9 | ✅ | edit.test.tsx:154-196（未知 id/画廊 id/画廊项目点名 B08/doc 未命中/永不 settle 的加载态，均有出路无跳转） |
| T-ED-10 | ⚠ | edit.test.tsx:198-234 在 jsdom 全绿；但拖动一测（:223-233）走的是无 `setPointerCapture` 的路径——真实浏览器上该行为失效（HIGH-1），绿灯不覆盖生产 |
| T-ED-11 | ✅ | edit.test.tsx:236-260（灌空区 16 格/取色回画笔且不画格） |
| T-ED-12 | ✅ | edit.test.tsx:262-293（源列表只列在用码并带颗数/应用后画布与统计同步/目标空=擦除/源==目标禁用） |
| T-ED-13 | ✅ | edit.test.tsx:295-317（count 降序 + 下标升序整串断言/点行换活动色/合计） |
| T-ED-14 | ✅ | edit.test.tsx:319-348（按钮≡快捷键三式/焦点在下拉时不触发/栈空禁用） |
| T-ED-15 | ✅ | edit.test.tsx:350-392（防抖后落仓且不抢跑/pagehide 立即 flush/卸载 flush + 重进回读）；真实 timer + `vi.waitFor` 轮询替代 IA §5.3 字面的 fake timers，文件头注明理由（user-event session 先于换 timer 会死锁）——手段偏离、语义等价，判可 |
| T-ED-16 | ✅ | edit.test.tsx:394-435（stepIndex 5→0、mode/elapsedMs 原样/无游标不造/提示句可见性双向） |
| T-ED-17 | ✅ | edit.test.tsx:437-453（「保存失败」/续编可用/不冒充 PersistenceBanner） |
| T-ED-18 | ✅ | edit.test.tsx:455-475（草稿卡出链指 `/edit/:id`；画廊项目卡无此链接） |
| T-ED-19 | ✅ | edit.test.tsx:477-502（空板禁用/去拼装 flush 后经 D-UP-14 null 分支渲染 `assemble-grid`/draft→active/todo 不改状态） |
| T-ED-20 | ✅ | isolation.test.ts 递归扫描未收窄，另加扫描面自检（:66-72）确保 `pages/edit` 永在网内——比「无需改动」多钉一颗钉子，判为加分而非 drive-by |

## 3. 实现者旗标的核实：「不是缺陷」断言换家 —— **确认（正当且必要）**

旧测试 `B06 / B07 的占位面板照旧写明不是缺陷`（B03 落地后的 CreatePage.test.tsx:51-58）名字带
B07，**身体只开过 blank 面板**——面板级「这是 X 的占位说明页，不是缺陷」断言在改写前只有
B06 这一个家。D-ED-4 把 blank 面板变成真表单后，这个家必然消失；若断言随之蒸发，
CreatePage.tsx:97-101 的 `selected.owner !== null` 分支（占位面板的自我声明机制，R6 的本体）
将**零面板级覆盖**——下一个把这段话术改坏的 PR 不会被任何测试拦下。换家到 `导入已有豆图`
（import-pattern，仍是 WP-B07 占位）是唯一能让该不变量继续有测试的落点（CreatePage.test.tsx:58-65）。

与 D-ED-19「B07 两条占位断言原样保留，不许顺手动」不冲突：那两条指用例 1 的
`getAllByText("尚未实现，归 WP-B07")).toHaveLength(2)`（:35），逐字保留；换家没有触碰 B07 的
产品面与既有断言，是纯测试面的**收养**而非改动。§5.4 映射表的其余三行（B06 标签 → 「现在就能用」、
blank 说明页 → 新建表单测试、画廊用例不动）逐条兑现。判定：实现者的裁量正确，且优于字面执行。

## 4. 发现（按严重度）

### HIGH

- **HIGH-1 · `setPointerCapture` 把拖动笔画整个杀死（真实浏览器）。**
  EditorCanvas.tsx:53-55 在 pointerdown 时对**容器**取指针捕获，而 extend（:60-69）用
  `pointOf(event.target)`（:63 → :30-35）从 `data-x`/`data-y` 读格。Pointer Events 规范下，
  捕获生效后**所有后续指针事件都重定向到捕获元素**：`event.target` 恒为 `.editor__grid` 容器，
  dataset 无坐标，`pointOf` 返回 null，`onExtend` 一次都不会发——拖动只画下 pointerdown 那一格。
  本审查两重实证（真实 Chrome headless）：
  1. 与 EditorCanvas 逐行同构的最小复现：捕获开 → 六格拖动 **0 个 extend 事件**、全部
     pointermove 目标 = 容器；捕获关 → 3 个 extend、目标 = SPAN。
  2. **真实构建产物**（`vite build` + preview）：mint 空白板 → 单击落 1 颗（画笔本身好的）→
     六格拖动后合计 **2 颗**（预期 7）——拖动只贡献了 pointerdown 一格。
  jsdom 的 `Element.prototype.setPointerCapture` 是 `undefined`，:53 的存在性守卫让 488 条测试
  全部走未捕获路径，T-ED-10 的拖动测试因此对生产行为免疫。触屏更糟：即使去掉显式捕获，
  pointer events 的隐式触摸捕获也会把 target 钉死在起笔 span 上——所以**修复不是删捕获**，
  而是 extend 里改命中测试：`document.elementFromPoint(event.clientX, event.clientY)` 取格
  （B03 工作台的拖拽早就按捕获语义用 clientX 换算，UploadWorkbench.tsx:276-296 是现成先例）。
  约 5 行 + 一条钉住回退路径的测试（stub `elementFromPoint` 即可在 jsdom 断言）；父代理浏览器核
  必须包含一次拖笔。这是 D-ED-8 核心手势在生产的整体失效，构成退回理由。

### MED — 无

### LOW

- **LOW-1 · `goAssemble` 无视 flush 失败照样提状态、照样导航**（EditPage.tsx:224-228）。
  `await flush()` 拒绝时（保存失败态）仍 `setProjectStatus(…,"active")` 并进 `/assemble/:id`，
  拼装页随即渲染**上一次成功落盘**的网格——用户刚画的、编辑器刚说「保存失败」的那部分不在板上，
  且项目已被提为 active。今天窗口极窄（要求写失败 + 立刻去拼装），但「报告失败却放行推进」与
  D-ED-17 ③「门在推进」的语义有出入；flush 失败时留在编辑器并让 `role="status"` 的「保存失败」
  说话即齐。
- **LOW-2 · 笔画进行中键盘 undo/redo 会撕裂历史一致性**。快捷键监听不看 `state.stroke`
  （EditPage.tsx:187-202），reducer 的 undo/redo 也不守（editor.ts:331-352）：指针按住拖笔时
  另一只手 Ctrl+Z/Y，历史条目会在未收笔的 stroke 增量之间插队，收笔后 past 里两条目的
  before/after 可能互相踩格，往返 undo 能落到一个从未存在过的板面。一行守卫
  （`stroke !== null` 时 undo/redo no-op）即齐。边界极窄，记 LOW。

### 观察（不计数）

- `role="img"` 挂在一个交互画布上（EditorCanvas.tsx:80）：D-ED-7 点名该语义属拼装会话，编辑器
  照用了它 + `aria-hidden` 格子。读屏用户听到的是一张静态图。R-ED-1 已把键盘作画记为 v0 边界，
  aria-label 又兼作测试的颗数通道，判观察不判缺陷；日后换 `role="application"` 更诚实。
- T-ED-15 用真实 timer + 轮询替代 IA 字面的 fake timers（见 §2 表内注）；整套 suite ~18s，可接受。
- `pagehide` 的 flush 是 async fire-and-forget（EditPage.tsx:179）：卸载竞态下 IDB 事务多半来得及
  提交但无保证——R-ED-5 已接受单笔损失，维持记录。
- `latest` ref 模式让三个指针回调恒等稳定、memo 画布在工具面板重渲染时跳过 3136 节点
  （EditPage.tsx:126-131）——质量加分；画布 label 含颗数，网格变更帧本来就要重画，memo 语义自洽。
- 失败后 `savedRevision.current = -1`（EditPage.tsx:151）让下一次任何 flush 必重试——
  与 D-UP-16「不 catch-后-假装-成功」同源，正确。
- isolation.test.ts 的扫描面自检（见 T-ED-20）与 `strokeEnd` 不清空未变更笔画的 redo 栈
  （editor.ts:312）都是契约没要求但做对了的细节。

## 5. 偏离与残余风险对账

- **DEV-ED-1（新路由 `/edit/:id`）**：按 D-ED-3 落地（routes.tsx:29，AppShell 内、无高亮项）；
  吸收进 #48 即父代理未否决。**DEV-ED-2（转换文档可编辑）**：守卫链不拦 provenance 文档，
  编辑后 provenance 原样（§2 D-ED-2）。**DEV-ED-3（游标归零）**：T-ED-16 双测钉住，mode/elapsedMs
  确未被动。**DEV-ED-4（Workspace 链接）**：对 WorkspacePage.tsx 的改动恰为一个 `EditLink` 组件
  与三处使用，全部以 `sourcePatternId === null` 把关——批准面之内，无 drive-by。
- **R-ED-1**：画布确为指针驱动、格子非聚焦；选色/替换/统计/撤销全程键盘可用（radio/select/button）。
  维持记录。**R-ED-2**：B03 交付物（`createPatternDoc`/`decodeCells`/`addProject`/内存仓库双身）
  全部在场且被直接复用，无漂移。**R-ED-3**：画廊守卫文案点名 B08（EditPage.tsx:42-43）。
  **R-ED-4**：接受不修，实测保留。**R-ED-5**：`pagehide` + 卸载双 flush 落地；见观察第三条。
  **R-ED-6**：批量合帧做了（strokeTo 批点 + Bresenham 补线），但其浏览器管道被 HIGH-1 切断——
  修复 HIGH-1 后该缓解才真正生效。**R-ED-7**：未另设防，维持 DATA-5 口径。

## 6. 验收记录

```
pnpm install --frozen-lockfile   ← 独立 worktree，锁文件未动
pnpm --filter @bead/app test
Test Files  32 passed (32)
     Tests  488 passed (488)     ← 父代理门禁 32/488，命中
pnpm --filter @bead/app lint     ← tsc --noEmit && eslint . 零输出
```

浏览器核（真实 Chrome headless，`vite build` + preview，本机回环）：
blank mint → 落 `/edit/proj-*` → 单击画 2 颗 → `role="status"` 到「已保存」→ 刷新回读 2 颗
（IDB patterns 仓）→ localStorage 键恰为 `["bead.theme"]`。**拖笔六格只落 1 颗**（HIGH-1 的实证）。

审查环境：`cursor/bead-r3-b06-review-c441` @ 基于 `00cc9c7`（即最新 `cursor/beadflow-integration-c441`），
独立 worktree。托管 `bead.yml` 的空 runner 状况沿 `a5a69c4` 记录，不影响本判定。

## 7. 退回的最小修复面（不重开架构）

1. `EditorCanvas.tsx` extend：捕获生效时用 `document.elementFromPoint` 命中测试取格（~5 行），
   pointerdown 路径不动；加一条 stub `elementFromPoint` 的 jsdom 回退测试。
2. （随手可修的 LOW，不阻塞）goAssemble 在 flush 失败时不提状态不导航；undo/redo 在
   `stroke !== null` 时 no-op。
3. 父代理浏览器核补一次**拖动**笔画 + 一次触屏模拟（隐式捕获路径）。

## 8. 禁区自查

本轮只新增 `docs/bead/reviews/round3-pixel-editor-review.md`。未改 `apps/bead/**`、
`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、`apps/desktop/**`、
`crates/soul-*/**`、根 Cargo members、`deny.toml`、`.github/**`、`pnpm-lock.yaml`。
浏览器验证只访问本机回环地址，零外网流量；本文无外网 URL（BD15/BD17）。
