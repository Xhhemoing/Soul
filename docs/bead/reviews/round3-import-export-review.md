`claude-fable-5-thinking-xhigh`

# ROUND 3 · WP-B07 导入导出 · 实现审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审查，不实现）。
被审对象：吸收提交 `e6ca0ae`（absorb WP-B07 import/export，即 PR #51 的吸收面），
基线 `origin/cursor/beadflow-integration-c441` @ `7757f4f`（B06 拖拽修复浏览器核通过后的 tip）。
审查依据：`docs/bead/reviews/round3-import-export.md`（D-IE-1…20 / T-IE-1…19，本 WP 的法律）、
`round3-create-upload.md` §2（bead-v1 契约面）、`round3-pixel-editor.md`（D-ED-1 判别式、§2.6 空文档合法性）、
`docs/agent-decisions.md` BD15 / BD17 / BD19 / BD20、B05 复审 MED-1（blob URL 铸造时机）。
本轮只新增本文件；未改任何产品文件、测试文件、rust、Soul 树。

范围对账：吸收提交对 `apps/bead/**` 与实现分支 tip `3dbf43f` **逐字节一致**（`git diff` 空），
额外只有父代理记账 `docs/agent-progress.md`。实现分支五个 commit 满足 §10 的纪律：
`477637e`（schema + 校验器）、`b422a42`（两面板 + CreatePage 改面）、`0ea94a4`（**单独承载**
CreatePage.test.tsx 改写，B03 `6cf7e63` / B06 `a11358e` 同款）、`d584e25`（Workspace 单项目导出）、
`3dbf43f`（T-IE 验收矩阵）。零新依赖、零锁文件改动（`package.json` 不在 diff 里，BD16 不触发）；
R-IE-1 的硬时序满足：切点在 B03（#45）与 B06（#48/#50）全部合入之后。

---

## 0. 结论先行

**ACCEPT-WITH-NITS。0 条 HIGH，1 条 MED，5 条 LOW。** 七条硬红线全部锁死（§1），D-IE-1…20
逐条成立（§2），T-IE-1…19 除 16 的一个子句外全部有真测试落点（§3）。本审查独立工作树实测
`pnpm --filter @bead/app test` = **36 文件 / 566 条全绿**（父代理门禁 36/566，命中）、
`tsc --noEmit && eslint .` 零输出。实现者两条旗标（`upsertProgress` 盖章 `updatedAt`、
读不到文档的 null-source 项目不进归档）**逐行核实为真且正当，确认**（§5）。唯一 MED 是
测试覆盖缺口而非行为缺陷：T-IE-16 的「导入全空网格 → 拼装侧空网格态」子句实际没被任何
测试走到（用例标题这么说、身体却开的是编辑器），拼装侧 `EMPTY_GRID_NOTE` 全套件零消费者——
行为本身经逐行核读是安全的（守卫在 `AssemblePage.tsx:90-91`，空网格渲染提示句、不渲染
session、无崩溃路径），补一个 ~10 行用例即可。不重开架构。

## 1. 任务钉出的硬红线（逐条对树验）

1. **Grid 不进 localStorage ✅。** 新源码全树无 `localStorage` 写者；T-IE-12
   （import-storage.test.ts:287-310）导入一轮后枚举 `patterns` / `state` 两仓 + 旧键
   `bead.state`：网格只在 `patterns` 仓一份，`state` 仓序列化串里 `cells` 缺席，
   legacy storage 长度为 0。
2. **仓库表面零增量、`loadProgress(id)` 未复活 ✅。** `stores/**` 一个文件都不在 diff 里
   （对基线 `7757f4f` 零字节差）；全树 `loadProgress` 只有无参数组形（repository.ts:27、
   bead-v1.ts:287）。导出枚举由 projects 数组驱动（export.ts:52-66），**没有**
   「getAll patterns」仓方法；导入写盘只经 `savePatternDoc` / `addProject` /
   `upsertProgress` / `addInventoryEntry` 四个既有通路（import.ts:61-70）。
3. **零 `$schema` / `$id` / http(s) 字面量 ✅。** 不落 JSON Schema 文件、不引 ajv/zod；
   `$schema` / `$id` 两串只作为**被禁止之物的名字**出现在注释与散文里（beadproj.ts:6、
   beadproj.md:8、schema/README.md），格式本体（`BeadprojFile`）无任何 URL 形字段。
   新源码 grep 零远程 URL；isolation.test.ts:70 新增扫描面自检，`schema/` 目录被 NE-1
   递归扫描这件事本身成为断言（T-IE-19 落地）。
4. **未臆造 `.pat` / `.gamedev` 解析器 ✅。** 嗅探只读前 ≤16 字节（import.ts:24、:52-55，
   `file.slice(0, 16)`，T-IE-7 用 spy 断言 slice 实参），比对 PNG/JPEG 魔数与首字节 `{`，
   其余一律 `UNSUPPORTED_FORMAT`——**没有任何成功解析路径**。文案带文件名 +
   「暂不支持解析该格式」（ImportPatternPanel.tsx:73），测试锁死不许诺（import-export.test.tsx:241
   断言无「正在解析 / 即将支持」）。
5. **blob URL 不在 useMemo / 渲染体铸造 ✅。** 全树唯一 `createObjectURL` 调用点是
   `downloadBeadproj`（export.ts:90-103），仅被两个点击处理器调用（ImportProjectPanel.tsx:96、
   WorkspacePage.tsx:49）；T-IE-17（import-export.test.tsx:388-404）断言渲染期 mint 计数为 0、
   点击后恰 1、用后 revoke（`setTimeout(0)` 延迟 revoke 防止同 tick 取消下载——比 B05 MED-1
   的教训又多想了一步）。
6. **零静默截断 ✅。** title >256 → 整条 entry 拒绝而非截断（beadproj.ts:199-201，
   T-IE-6 明文测「拒绝而不是截断」）；513 轴 → `SCHEMA_INVALID`、57 轴 →
   `GRID_TOO_LARGE_FOR_V0` 且文案点名两个数字（beadproj.ts:236-250）；三层门无一提议裁剪。
   `beadprojFileName` 把文件名 stem 切到 64 字（beadproj.ts:518）是 D-IE-16 授权的
   **文件名消毒**，不触碰数据本体。
7. **导入库存绝无盖章路径 ✅。** 文件层校验用 `isInventoryEntry`，注释明写**故意不用**
   会盖 `gallery` 章的 `toInventoryEntry`（beadproj.ts:364-372）；缺失 / 未知 `paletteId`
   丢弃并计数（T-IE-6 末测：三条坏行全丢、`droppedInventory === 3`；T-IE-11 存储级复核
   两个命名空间的 G07 并存互不抵扣）。

## 2. D-IE-1…20 落点速查

| ID | 判定 | 落点 |
|---|---|---|
| D-IE-1 | ✅ | 两入口留在 `?entry=` 面板位、零新路由（CreatePage.tsx:104-105；routes 无改动）；两条 `owner: null`（:59、:67），detail 改写为实际能力且不许诺解析（:60-72） |
| D-IE-2 | ✅ | `import-pattern` = 纯识别面（嗅探 → 改道或拒绝，零解码零写盘，import-export.test.tsx:271-279 spy 断言）；`import-project` = `.beadproj` 导入 + 「导出全部」双向通道 |
| D-IE-3 | ✅ | 单一形状 `{format, version, projects, inventory?}`，单项目导出 = 长度 1 归档（export.ts:72-78 走同一 `buildBeadprojFile`）；文件零时间戳，序列化是 state 的纯函数（beadproj.test.ts:410-414 逐字节断言） |
| D-IE-4 | ✅ | `schema/beadproj.ts` TS 即 schema + 重建式校验器；`beadproj.md` 散文（.md 不进 e0 扫描面）；零 JSON Schema 文件、零 ajv/zod |
| D-IE-5 | ✅ | cells = 明文整数数组，`-1` 空，∈ {-1} ∪ [0,48)（`PALETTE_SIZE` 从 `GENERIC_5MM.entries.length` 现算，beadproj.ts:53）；JSON↔Int16Array 全部经 `createGrid`/`createPatternDoc`/`decodeCells` 既有 codec（import.ts:88-92、beadproj.ts:445），零第二套编码器 |
| D-IE-6 | ✅ | ①20MB 门在 `File.size` 上、读字节前（checkImportFileSize，两面板都先过，T-IE-14 断言 `text()` 未被调）；②≤512 效度门；③≤56 v0 接收门独立错误码 + 双数字文案（beadproj.ts:236-250） |
| D-IE-7 | ✅ | `format` ≠ → `SCHEMA_INVALID`、`version` ≠1 → `UNSUPPORTED_VERSION` 且不解析 projects（beadproj.ts:381-390）；未知键重建式天然丢弃（T-IE-1 种键断言绝迹） |
| D-IE-8 | ✅ | Entry 无 id（导出侧 beadproj.test.ts:392-394 断言序列化串不含 owner id）；逐 entry `mintProjectId()`（import.ts:125-131）；`sourcePatternId` 只收 null 或过 `isPatternId` 的串，catalog 查不到不拒（import.ts:100-103 注释点名两个降级消费端） |
| D-IE-9 | ✅ | 双射在 pattern 自检**之前**判（beadproj.ts:329-337），违者整条拒绝带序号；导出侧同一双射再执行一遍（buildBeadprojEntry:444-445，画廊项目手里有 doc 也不写 pattern，beadproj.test.ts:400-408 钉住） |
| D-IE-10 | ✅ | 逐 entry 串行 `await savePatternDoc` → `addProject` → `upsertProgress`（import.ts:129-153，调用序被 T-IE-10 逐字符串断言）；写失败断路、已成保留、汇总报「已导入 n / N，第 k 条失败」；汇总进 `role="status"`（ImportProjectPanel.tsx:135）。库存合并在 entry 循环之后（见 §6 记录 2） |
| D-IE-11 | ✅ | 无双 CTA，`status` 从文件读且 ∈ 四值（beadproj.ts:203-206，导入后 done/draft 原样，import.test.ts:151-169）；title ≤256 拒超、trim 空 → 「未命名导入」；createdAt 有限正数；backdrop/backdropColor 同 `isProject` 口径 |
| D-IE-12 | ✅ | `inventory` 归档层可选；每条必带已知 `paletteId`，缺/未知丢弃计数绝不盖章（§1.7）；合并 = 已存在 `(paletteId, code)` 跳过（`normalizeCode` + `paletteCodeKey` 与 reducer 同键，import.ts:155-171），幂等有测（import.test.ts:263-279）；汇总报「新增 x / 跳过 y / 丢弃 z」 |
| D-IE-13 | ✅ | progress 校验借 `toProgressCursor`（占位 id `proj-beadproj-check` 过 `isProjectId`，beadproj.ts:306-318）；导入盖新铸 projectId；`stepIndex` 不夹取（消费端既有语义）；`doneBits` 不在 v1，种进去被丢（T-IE-1） |
| D-IE-14 | ✅ | 导出全部 = 全部项目（画廊项目纯元数据 entry 含 status/backdrop/游标）+ 文档 + 游标 + 全量库存；枚举由 projects 数组驱动，孤儿 doc 故意不出现（import-storage.test.ts:217-225 断言归档串不含孤儿 id）；导出全程零写盘（import-export.test.tsx:484-498 spy 断言） |
| D-IE-15 | ✅ | ①「导出全部」在 `import-project` 面板，项目与库存皆空不渲染（ImportProjectPanel.tsx:111、:151）；②单项目导出挂 Workspace 卡片，判别式 = `sourcePatternId === null` ∧ doc 命中（WorkspacePage.tsx:41-42，`usePatternDoc` ready 才渲染）；画廊卡与丢文档卡都无导出控件（T-IE-17 两测）。控件是 `<button>` 而非链接——语义更对（动作非导航），不算偏离 |
| D-IE-16 | ✅ | 铸造点在点击处理器（§1.5）；文件名确定：消毒标题（空 → `bead-project`）/ `bead-projects.beadproj`；MIME `application/json`（blob type 有测）；路径分隔符进不了文件名（beadproj.test.ts:428-433） |
| D-IE-17 | ✅ | PNG/JPEG 魔数 → 非错误改道提示 + `?entry=upload` 链接（ImportPatternPanel.tsx:52-59）；面板零解码（decodeSpy 全程零调用）、不传 File 跨面板；`{` 首字节 + `.beadproj` 扩展名 → 指路 `import-project`（:61-68） |
| D-IE-18 | ✅ | 只读前 ≤16 字节；无公开规范的两格式全拒 `UNSUPPORTED_FORMAT` 带文件名；detail 与错误文案都不许诺解析（R-IE-3 缓解落实）。BOM / 前导空白跳过后再判 `{`（import.ts:46-48）是对「首字节 `{`」的宽化：编辑器存出的 BOM 文件仍能被认成 JSON，且 `File.text()` 解码时剥 BOM，下游 `JSON.parse` 不受扰——宽进严出，判定合规 |
| D-IE-19 | ✅ | 封闭 union 五码进 `schema/beadproj.ts`（:60-65）+ entry 级错误（序号 + code + message）；五码各有面板内联呈现（alert / 拒绝清单 / 改道 status）；无 catch-后-假装-成功路径（唯一缝隙见 LOW-2，是**未捕获**而非假装成功） |
| D-IE-20 | ✅ | 校验器 + 错误码 + `beadproj.md` + `beadproj.test.ts` 进 `schema/`；嗅探纯函数进 `pages/create/import.ts`；导出纯函数进 `pages/create/export.ts`（IA §11 明文许可的家）；两面板组件；`algo/**` 零字节差；全部主线程（D-IE-6 ③ 解除 Worker 触发条件成立） |

## 3. T-IE-1…19 落点速查

| ID | 判定 | 落点 |
|---|---|---|
| T-IE-1 | ✅ | beadproj.test.ts:76-148（白名单键排序断言 + 种键绝迹 + provenance 逐字段 + 空 projects 合法） |
| T-IE-2 | ✅ | :150-183（format / version / 非数组 / 坏 JSON 四路） |
| T-IE-3 | ✅ | :185-229（长度不符、-2/48/1.5/"0"、gallery paletteId、序号 = 文件位置三条混排） |
| T-IE-4 | ✅ | :232-268（20MB 边界、513→SCHEMA_INVALID、57→GRID_TOO_LARGE_FOR_V0 双数字、56×56 过、0/负轴拒） |
| T-IE-5 | ✅ | :271-298（双射四路，含纯元数据画廊 entry 通过、`proj-` 串冒充 sourcePatternId 拒） |
| T-IE-6 | ✅ | :301-368（六路字段违规、256 边界、trim 空 → 未命名导入、progress 复用 toProgressCursor、库存三坏行丢弃计数） |
| T-IE-7 | ✅ | import.test.ts:30-70（PNG/JPEG/json/unknown 分类、BOM + 空白、15 字节不越界、slice(0,16) spy、JSON 数组 = unknown） |
| T-IE-8 | ✅ | import-storage.test.ts:142-225（真 `fake-indexeddb` + `createRepository` 旗舰往返：项目/文档含 provenance/游标/双命名空间库存逐字段回来，id 与游标 `updatedAt` 两个设计内例外；导出逐字节确定；再导出等价；孤儿 doc 不进归档） |
| T-IE-9 | ✅ | :228-253（两次导入 → 4 项目 4 id 互异、2 文档、2 游标，零覆盖） |
| T-IE-10 | ✅ | import.test.ts:171-187（第 2 条注入 reject → entry 1 完整、entry 2 无项目、entry 3 未执行、汇总「已导入 1 / 3」+「第 2 条」） |
| T-IE-11 | ✅ | import-storage.test.ts:255-284 + import.test.ts:221-297（并存互不抵扣、跳过计数、丢弃计数、归一化同键、幂等） |
| T-IE-12 | ✅ | import-storage.test.ts:287-310（`patterns` 仓恰一份、`state` 仓无 cells、legacy localStorage 空） |
| T-IE-13 | ✅ | import-export.test.tsx:166-192（两入口「现在就能用」、面板文件输入、无占位话术、直链可达） |
| T-IE-14 | ✅ | :194-229（stub `File.size` 21MB → 读前拒 + 内联 FILE_TOO_LARGE、坏 JSON、version 2、图片面同过文件门） |
| T-IE-15 | ✅ | :232-279（.pat/.gamedev → UNSUPPORTED_FORMAT 不许诺；PNG → 改道 + `?entry=upload` 链接；decodeSpy 全程零调用；识别面零写盘） |
| T-IE-16 | ⚠ | :282-342 覆盖汇总 `role="status"`、Workspace 现身、`assemble-grid` 渲染、拒绝清单带序号；但**全空网格子句**（:344-375）标题写「拼装侧空网格态」、身体开的是编辑器（draft 卡无「继续拼豆」），`/assemble/:id` 的 `EMPTY_GRID_NOTE` 分支全套件零测试消费者 → **MED-1** |
| T-IE-17 | ✅ | :378-440（渲染期零 mint、点击恰 1、revoke、文件名、恰一条 entry 过自校验、带游标、画廊卡 / 丢文档卡无控件） |
| T-IE-18 | ✅ | :443-533（归档含画廊纯元数据 entry + 库存 + 游标；零写盘；皆空不渲染；纯库存备份；面板到面板闭环回导） |
| T-IE-19 | ✅ | isolation.test.ts 递归扫描自动覆盖全部新文件 + 新增 `schema` / `pages/create` 扫描面自检（:70）；本审查另 grep：零 eval / `new Function`、零第三方解析依赖（package.json 不在 diff） |

Do-not-regress：36 文件全绿即含 `navigation` / `persistence-banner` / `empty-states` /
inventory 全部 / `algo/` 全部 / B03 create-upload / B06 edit / assemble 既有用例，无一改动无一红。

## 4. 门禁实测（本审查独立工作树）

```text
pnpm --filter @bead/app test   → Test Files 36 passed (36) / Tests 566 passed (566)   ← 父代理门禁 36/566 命中
pnpm --filter @bead/app lint   → tsc --noEmit && eslint . 零输出，exit 0
git diff 3dbf43f e6ca0ae -- apps/bead → 空（吸收逐字节忠实）
```

## 5. 实现者旗标核验（不当福音，逐行验过）

1. **「upsertProgress 盖章 updatedAt」→ 真，正当。** `store.tsx:256` 的 action 无条件
   `{ ...cursor, updatedAt: Date.now() }`（BD19：盖章归 action）；导入端传
   `ProgressCursorInput` 四字段、不传 `updatedAt`（import.ts:145-150）。后果：文件里的
   `updatedAt` 被校验、被导出、但**不被恢复**——T-IE-8 的「逐字段相等（id 除外）」字面上
   多出第二个例外。判定：IA 自己钉死「导入经 `upsertProgress` action 走既有 save effect」
   （§3.1），绕开 action 直写 `saveProgress` 才是真偏离；`beadproj.md` §2 明文披露、
   测试标题明文点名（import.test.ts:214）。确认为设计内例外，不是缺陷。
2. **「null-source 项目读不到文档 → 不进归档」→ 真，正当。** export.ts:60-64 逐项目
   `loadPatternDoc(id).catch(() => null)`，null 则 `skippedProjects += 1` 跳过；面板导出
   提示报「另有 n 个项目读不到豆图文档，未纳入归档」（ImportProjectPanel.tsx:100-105）、
   `beadproj.md` §7 成文。判定：D-IE-9 双射之下这是唯一自洽解——给 null-source 写一条
   无 pattern 的 entry，本格式自己的校验器都会拒收；静默丢会违「fail closed 要出声」，
   这里**计数出声**了。确认。

## 6. 缺陷清单

### MED

- **MED-1 · T-IE-16 空网格子句无测试落点** — `apps/bead/src/pages/create/import-export.test.tsx:344`。
  用例标题「全空网格的项目合法：拼装侧空网格态，不崩」，身体导入 draft 项目后从工作台点的是
  「编辑豆图」，`/assemble/:id` 从未被访问；`AssemblePage.tsx:20` 的 `EMPTY_GRID_NOTE` 在
  全套件 grep 只有定义与使用两处、零测试消费者。行为本身核读为安全：`occupiedCount === 0`
  时渲染提示句、`AssembleSession` 不渲染（AssemblePage.tsx:86-92、:113-116），无崩溃路径；
  但矩阵明文要的断言不存在，且用例标题对读者撒了谎。修法 ~10 行：导入 entry 用
  `status: "active"` 或直接 `/assemble/:id` 直链，断言 `EMPTY_GRID_NOTE` 文本在、
  `assemble-grid` 不在。不阻塞吸收，随任一后续 PR 补。

### LOW

- **LOW-1 · 文件输入不复位** — `ImportProjectPanel.tsx:50` / `ImportPatternPanel.tsx:37`。
  处理完不清 `event.target.value`；Chromium 系再选**同一个文件**不触发 change，于是
  T-IE-9 引以为傲的「同一文件导入两次 → 两套项目」在真实浏览器里要先换个文件或离开页面。
  一行 `event.target.value = ""` 的事。
- **LOW-2 · `file.text()` 拒绝未接住** — `ImportProjectPanel.tsx:64`。读取本身失败（选后
  文件被移走等罕见路径）时异常逃出 `void handleFile`，`finally` 清了忙态但无内联错误——
  不是 catch-假装成功，是漏了具名呈现。包一层 try 落 `SCHEMA_INVALID` 级内联即可。
- **LOW-3 · Workspace 导出忽略不可用返回值** — `WorkspacePage.tsx:49`。`downloadBeadproj`
  在无 `URL.createObjectURL` 环境返回 false，面板路径有 `EXPORT_UNAVAILABLE` 内联
  （ImportProjectPanel.tsx:96-99），卡片路径静默无为。目标浏览器不缺该 API，体感极窄。
- **LOW-4 · 坏 provenance 静默丢弃而非拒 entry** — `beadproj.ts:252-259`、:291。
  `kind: "Sketch"` 之类整个 provenance 被丢、entry 照收（beadproj.test.ts:134-140 钉的
  就是这个行为）。与 `toPatternDoc` 读回门的既有先例一致（可选出处元数据，丢了不改变
  网格语义），但它是 D-IE-11「任一字段违规 = entry 拒绝」全表中唯一的宽处理，应记录在案。
- **LOW-5 · 导入库存绕开表单配额门** — 文件行只过 `isInventoryEntry`（IA §3.3 明文指定），
  不受表单的 code ≤16 / name ≤64 约束；`beads` 超 99,999 被 reducer `clampBeads`
  （store.tsx:94）静默夹取——夹取是全体调用方共享的既有 reducer 行为，非 B07 新造。
  账记在 IA 头上，不是实现偏离；20MB 文件门为字符串长度封了顶。记录不改。

### 记录（非缺陷）

1. `UNSUPPORTED_FORMAT` 文案「认得出这是个豆图文件」（ImportPatternPanel.tsx:73）对任意
   未知二进制（比如一个 PDF）轻微高抬了「认得出」——但不许诺解析，R-IE-3 的红线守住了。
2. entry 中途写失败后库存合并**仍执行**（import.ts:129-171：断路只断 entry 循环）。
   D-IE-10 原文只说「停止后续 entry」且「inventory 合并在全部 entry 之后执行」；库存归
   归档层不归任何 entry，且走的是 `state` 仓（与失败的 `patterns` 仓不同故障域），汇总
   同时报两件事。判定合规，语义择优。
3. 单项目导出控件是 `<button>` 不是 DEV-IE-2 字面的「链接」——触发动作用 button 是更对的
   语义；`ExportLink` 每卡一次 `usePatternDoc` 全文档读取（≤6.3KB/卡）换判别式精确性，
   与 D-ED-1 同判别式的要求一致，代价可忽略。

## 7. 裁决

**ACCEPT-WITH-NITS。** 七条硬红线零违例；仓库表面零增量；schema 无 URL、无第二套 codec、
无第二套存储；fail closed 贯穿三层门与 entry 级拒绝；两条实现者旗标核实为真且披露充分；
父代理门禁（36/566 + lint 清洁）独立复现命中。MED-1 是测试矩阵的覆盖缺口（行为经核读安全），
5 条 LOW 全部是一行到十行级的打磨，无一阻塞吸收——e6ca0ae 可以留在 exclusive tip 上。
不重开架构。

## 8. 禁区自查

本轮只新增 `docs/bead/reviews/round3-import-export-review.md`。未改 `apps/bead` 任何源码
与测试（全部留给后续修缮席），未触碰 `docs/PRODUCT_LOCK.md`、`docs/STATUS.md`、
`apps/desktop/**`、`crates/soul-*/**`、根配置、`deny.toml`、`.github/**`、`pnpm-lock.yaml`。
子代理 `gh` 只读未破例；**BLOCKED_PR**：不建 PR，推分支即止。本文无外网 URL（BD15/BD17）。
