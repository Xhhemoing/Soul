# ROUND 3 · WP-B07 导入导出 — 前端 / 数据 IA

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只做 IA / 拆分，不实现）。
基线：`origin/cursor/beadflow-integration-c441` @ `2a17596`。
输入：`docs/bead/PLAN.md`（格式行：`.png` `.jpg` `.pat` `.gamedev` `.beadproj`）、`docs/bead/WORK_PACKAGES.md`（WP-B07）、
`docs/agent-decisions.md` BD14 / BD15 / BD17 / BD19 / BD20、`round1-map.md`（§2.1 e0 红线、§4 B07 两条精化、§4 B09「导出全部」、§5 `.pat`/`.gamedev` 样本推迟）、
`round1-frontend.md`（D-UI-1/4/5/7、§3「`/create` 永不为空；导入失败态归 B07」）、`round2-data.md`（§5 bead-v1、§6 导入消毒清单 DATA-4）、
`round2-inventory.md`（D-INV-3/11、§2.3 BD20 触发点）、`round3-create-upload.md`（D-UP-1…16、§2 契约对账、§7 Worker 顺延、T-UP-*）、
`round3-pixel-editor.md`（D-ED-1…21、§2.6 空文档合法性、DEV-ED-4 先例）、B05 复审 MED-1（blob URL 铸造时机）、
`apps/bead/src/pages/create/CreatePage.tsx` + `CreatePage.test.tsx`（两条 B07 占位断言）、`stores/`（repository / types / store / projects / ids）、
`algo/`（decode / image / grid / palette）、`schema/README.md`、`fixtures/catalog.ts`。
**未读**：B03 实现工作树（`/tmp/wt-b03`）的任何未提交文件——本文只对 exclusive tip + B03 IA（round3-create-upload）+ B06 IA（round3-pixel-editor）成文。
硬禁区自查：本轮只新增本文件；不触碰 Soul 锁、`apps/desktop`、`crates/soul-*`、根配置、`deny.toml`、`.github/**`、锁文件。

---

## 0. 结论先行（本 WP 的法律）

1. **B07 是 bead-v1 的纯消费者。** 导入写盘只走 B03 落的 `savePatternDoc(doc)` + 既有
   `addProject` / `upsertProgress` / `addInventoryEntry` 通路；导出只读 `loadPatternDoc(id)` +
   数组形 `loadProgress()` + `loadInventory()`。**零新仓、零新 Repository 方法、零 localStorage
   网格**（BD19/BD20 的触发点已由 B03 兑现，第二套存储在本文直接判死）。**硬时序**：B07 实现必须
   切在 B03 实现（PR #45）合入之后的 tip 上——本文引用的每个契约（三方法、`stores/patterns.ts`
   编解码、`InventoryEntry.paletteId`、`fake-indexeddb`）都是 B03 实现的交付物，B03 实现评审是
   B07 开工的闸门（§8 R-IE-1）。
2. **Fail closed 是本 WP 的第一性格**（round1-map §4 B07 / round2-data §6 原文照办）：每道解析门
   给命名错误、禁止静默截断、禁止臆造解析器。`.pat` / `.gamedev` v0 = 识别魔数 → 拒绝并报
   `UNSUPPORTED_FORMAT`，逆向解析等真实样本（round1-map §5 维持）。`.beadproj` schema **不写
   `$id` / `$schema` URL**（BD15/BD17；本文自身也零外网 URL）。
3. **B06 不是闸门，但交互要说清**：导入落成的带文档项目自动满足 D-ED-1 的可编辑判别
   （`sourcePatternId === null` ∧ doc 命中）——B06 合入后零 B07 代码即可进 `/edit/:id`；B06 合入前
   经 D-UP-14 分支直接可拼。导入的**全空网格文档合法**（D-ED §2.6：`patterns` 仓不变量不含非空；
   「空网格禁保存」只是上传工作台的规则），门在消费端：assemble 空网格态兜底、编辑器照常打开。

## 1. 决策表（D-IE-*，实现者照抄，不自行发明产品法）

| # | 决策 | 内容 |
|---|---|---|
| **D-IE-1** | 入口形态 | 两个入口留在 `/create?entry=import-pattern` 与 `?entry=import-project` 面板位（D-UI-1），**零新路由**：导入是一次性文件动作，无可深链、可后退的中途状态（对照 D-ED-3 的论证，结论相反、原则相同）。两入口 `owner` 改 `null`（「现在就能用」），detail 文案改写为实际能力 |
| **D-IE-2** | 面板分工 | `import-pattern`（导入已有豆图）= 豆图文件识别面：收 `.pat` / `.gamedev` / 图片，v0 全部走「识别 → 改道或拒绝」（D-IE-16/17）。`import-project`（导入项目库）= `.beadproj` 导入 + 导出面（单项目导出入口除外，见 D-IE-14/15 与 DEV-IE-1/2） |
| **D-IE-3** | `.beadproj` 单一形状 | 一个 schema 同时覆盖单项目与全量归档：**单项目导出 = 长度 1 的归档**。顶层 `{ format: "beadproj", version: 1, projects: Entry[], inventory? }`；`Entry = { project, pattern?, progress? }`。一个 schema、一个校验器、一条导入代码路径；这就是 round1-map §4 B09「导出全部 = `.beadproj` 数组下载」的落法。**文件内不含时间戳**：内容是 state 的纯函数（D-INV-11 的确定性先例，测试可做 parse-比对） |
| **D-IE-4** | schema 落法 | `apps/bead/src/schema/beadproj.ts`：TypeScript 校验器即 schema（round2-data §5.2「TypeScript 即 schema」同一哲学）+ 同目录 `beadproj.md` 写格式散文（`.md` 不进 e0 扫描面，出处/格式说明都放这）。**不落 JSON Schema 文件、不引 ajv/zod**（零新运行时依赖，D-UI-7；JSON Schema 文件正是 `$schema`/`$id` 红线的高发地）。校验器风格 = `toProgressCursor` 的重建式：白名单字段逐个校验重组，未知键天然丢弃、永不回写 |
| **D-IE-5** | cells 编码 | JSON 明文整数数组，行主序，`-1` = 空格，取值 ∈ `{-1} ∪ [0, 48)`（`GENERIC_5MM` 封闭 48 码）。**不用 base64 / RLE**：56×56 ≈ 3136 个数 ≈ 12KB，明文可人工查验。JSON 数组 ↔ `Int16Array` 的转换**复用** `stores/patterns.ts` 的编解码，不写第二套 codec（D-ED-20 同款裁决）；读回校验与 T-UP-5 同一道门 |
| **D-IE-6** | 尺寸三层门 | ① 文件门：`File.size ≤ 20MB`，超限在**读取字节前**拒绝（`FILE_TOO_LARGE`；D-UP-3 的「分配前设门」同款次序）。② 契约效度门：`width, height` 为正整数且各 ≤ 512（round1-map §4 B07 / round2-data §2.3 的格数上限），违者 `SCHEMA_INVALID`。③ **v0 接收门：每轴 ≤ 56**（D-UP-7 的同一条天花板：DOM 画布 D-ASM-6 与 v0 方板范围），57–512 之间 = 格式合法但 v0 拒收，独立错误码 `GRID_TOO_LARGE_FOR_V0`，文案点名两个数字（「这份豆图是 W×H，v0 支持上限 56×56」）。**任何一层都不静默截断、不提议裁剪**。文件格式本身容 512²：日后只放宽 ③，文件不用迁移（DEV-IE-3 可否决） |
| **D-IE-7** | 版本政策 | `format !== "beadproj"` → `SCHEMA_INVALID`；`version !== 1` → `UNSUPPORTED_VERSION`（fail closed，不尝试解析未来版本）。v1 内的未知多余键被重建式校验器丢弃（宽进字段、严出形状）。六角/圆板不进 v1：schema 无板型字段，方板由 width/height 隐含——几何扩展 = version 升级，不是加可选键 |
| **D-IE-8** | id 全断绝 | `Entry` **不含任何项目 id**：嵌套即关联（pattern/progress 挂在自己的 entry 里，无需 id 相认）。导入逐 entry `mintProjectId()`，文档与游标盖新 id——比 round2-data §6.2 的「重铸」更强：文件里根本没有可被误信的 id，碰撞类缺陷整类消失。`sourcePatternId` 保留：必须为 `null` 或过 `isPatternId` 的 `gal-*` 串；catalog 查不到**不拒**（下游对下架图纸已有优雅降级：需求侧跳过、拼装侧 `NO_GRID_NOTE`） |
| **D-IE-9** | entry 一致性 | 双射钉死：`sourcePatternId === null ⇔ pattern 字段必在`；`sourcePatternId !== null ⇔ pattern 字段必缺`。违者该 entry 整条拒绝（带序号与原因）。理由：画廊项目的网格是构建期 fixture（带文档会被 D-UP-14 分支永远无视）；无文档的 null-source 项目正是 D-UP-10 次序在铸造端防掉的悬空引用，导入端同罪同罚 |
| **D-IE-10** | 导入执行序 | 逐 entry 按文件序串行，每条镜像 D-UP-10：`await savePatternDoc(doc)` **成功后** → `addProject` → 有 progress 再 `upsertProgress`。**中途失败 = 停止后续 entry**，已成 entry 保留（不回滚：孤儿 doc 是 R-UP-2 已记录的无害类），汇总面板报「已导入 n / N，第 k 条失败」。inventory 合并（D-IE-12）在全部 entry 之后执行。汇总进 `role="status"` 区（D-ED-15 先例）；IDB 写失败沿 DATA-1 轨道（`PersistenceBanner`），面板只补一条内联汇总，不新增错误 UI 体系（D-UP-16 同源） |
| **D-IE-11** | 状态与字段校验 | **无双 CTA**：导入是恢复/接收，CTA 就一个「导入」；`status` 从文件读，必须 ∈ {todo, active, draft, done}（DEV-IE-4 可否决）。`title`：string 且 ≤256（round2-data §6.3 配额门），trim 后空 → 「未命名导入」，**不静默截到 64**；`createdAt` 有限正数；`backdrop`/`backdropColor` 过 `isProject` 同款检查。任一字段违规 = 该 entry 拒绝，不做修补性默认 |
| **D-IE-12** | 库存合并（BD20） | `inventory` 只在归档层、可选。每条**必须自带 `paletteId` ∈ 已知命名空间**——缺失或未知一律**拒收该条**（丢弃并计数），**绝不盖章**：给丢字段的 generic-5mm `G07 Silver` 盖 `"gallery"` 章 = 亲手重演 BD20 要防的 `G07 苔绿` 相撞；格式是我们自己发行的，没有旧文件需要宽容。合并政策 = **已存在 `(paletteId, code)` 跳过**（本地为准、重复导入幂等、不求和不覆盖），汇总报「新增 x / 跳过 y / 丢弃 z」 |
| **D-IE-13** | 进度导入 | `progress` 逐 entry 可选，五字段游标（BD19 形状），校验复用 `toProgressCursor` 的规则（projectId 例外：以新铸 id 盖上）。`stepIndex` 越界照旧由消费端夹取，导入端不夹（既有语义，types.ts 注释原文）。`doneBits` 不进 v1（B04 未偏离步序，无消费者——D-UP-13 维持） |
| **D-IE-14** | 导出范围 | 单项目导出 = 该项目一条 entry；「导出全部」= 全部项目（**含画廊来源项目**的纯元数据 entry——status/backdrop/进度也是用户会丢的档案，catalog 是构建期同款，回导即复活）+ 各自文档与游标 + 全量库存。文档枚举**由 projects 数组驱动**：逐个 null-source 项目 `loadPatternDoc(id)`，不加「getAll patterns」仓方法——孤儿 doc（R-UP-2）**故意不出现在导出里**。导出全程零写盘 |
| **D-IE-15** | 导出面 | ①「导出全部」控件放 `import-project` 面板内（该面板即「项目库：导入 / 导出」双向备份通道，DEV-IE-1 可否决）；项目与库存皆空时不渲染（D-INV-11 镜像）。② 单项目导出 = Workspace 卡片一条「导出 .beadproj」链接，**只出现在带文档项目上**（判别式与 D-ED-1 可编辑集合同一条：`sourcePatternId === null` ∧ doc 命中；画廊项目两次点击可从 catalog 重建，不值得第二条卡片规则）——这是 B07 对 `WorkspacePage.tsx` 的唯一改面（DEV-IE-2，先例 DEV-ED-4） |
| **D-IE-16** | 下载卫生 | `Blob` + `URL.createObjectURL` 运行时铸造（非 URL 字面量，D-INV-11 先例）；**铸造点在点击处理器（或 effect + setState），绝不进 `useMemo`/渲染体**——B05 复审 MED-1 的教训升级为本 WP 的硬规则；用后 revoke。文件名确定：单项目 = 消毒后的标题（空 → `bead-project`）+ `.beadproj`；归档 = `bead-projects.beadproj`。MIME `application/json` |
| **D-IE-17** | 图片改道 | `import-pattern` 收到 PNG（`89 50 4E 47`）/ JPEG（`FF D8 FF`）魔数 → **不是错误**：内联提示「图片形式的豆图请走上传入口的像素图路径」+ 链接 `/create?entry=import-pattern` 改 `?entry=upload`。**面板内零解码**：不建第二条转换管线、不传递 File 跨面板，`decode.ts` 仍是全应用唯一的字节→像素门（不偷 B03 的上传）。PLAN 的「实物照片逆向（透视矫正 + 网格提取）」不进 v0（§7） |
| **D-IE-18** | `.pat`/`.gamedev` | 只读文件**前 ≤16 字节**做魔数嗅探（`ArrayBuffer` slice 比对，不解释其余任何字节）。两格式无公开规范、无真实样本（round1-map §5），v0 **没有成功解析路径**：非图片、非 JSON 首字节的一切（含全部真实 `.pat`/`.gamedev`）→ `UNSUPPORTED_FORMAT` 命名错误，文案带文件名与「暂不支持解析该格式」；首字节 `{` 且扩展名 `.beadproj` → 提示去 `import-project` 入口。**禁止臆造解析器**；detail 文案不得许诺解析能力（R-IE-3） |
| **D-IE-19** | 错误码面 | 封闭 union `ImportErrorCode = "FILE_TOO_LARGE" \| "SCHEMA_INVALID" \| "UNSUPPORTED_VERSION" \| "GRID_TOO_LARGE_FOR_V0" \| "UNSUPPORTED_FORMAT"`，与 entry 级错误（序号 + 原因）一起进 `schema/beadproj.ts`。每个错误在面板有内联呈现；catch-后-假装-成功照旧是缺陷 |
| **D-IE-20** | 落点与执行 | 校验器 + 错误码进 `src/schema/beadproj.ts`（WP 原文钉的家）+ `beadproj.test.ts`；面板组件 `pages/create/ImportPatternPanel.tsx` / `ImportProjectPanel.tsx`，魔数嗅探等页面私有纯函数进 `pages/create/import.ts`（`session.ts`/`editor.ts` 先例）；`algo/**` 零改动（BD14/BD18：导入不属 oracle parity 域）。主线程执行：v0 网格 ≤56² ⇒ 序列化/解析在 KB–MB 级，**Worker 触发条件（round3-create-upload §7「B07 把 512² 带进来」）被 D-IE-6 ③ 显式解除**，R-UP-1 维持顺延 |

## 2. `.beadproj` v1 形状（`schema/beadproj.ts` 的 TS 即 schema）

```ts
interface BeadprojFile {
  format: "beadproj";          // 字面量判别符，非 URL（BD15/BD17）
  version: 1;                  // ≠1 ⇒ UNSUPPORTED_VERSION，fail closed
  projects: BeadprojEntry[];   // 单项目导出 = 长度 1；可为空（纯库存备份合法）
  inventory?: {                // 仅归档使用；每条必带 paletteId（D-IE-12）
    paletteId: string; code: string; name: string; hex: string; beads: number;
  }[];
}

interface BeadprojEntry {      // 无 id：嵌套即关联，导入一律重铸（D-IE-8）
  project: {
    title: string;                       // ≤256，trim 后空 → 「未命名导入」
    sourcePatternId: string | null;      // null ⇔ pattern 在场（D-IE-9 双射）
    status: "todo" | "active" | "draft" | "done";
    createdAt: number;
    backdrop: "black" | "white" | "custom";
    backdropColor: string;
  };
  pattern?: {
    paletteId: "generic-5mm";            // v0 唯一命名空间（round2-data §6.4）
    width: number; height: number;       // 各 ≤512（效度）；v0 接收 ≤56（D-IE-6）
    cells: number[];                     // 行主序；-1 空；∈ {-1} ∪ [0, 48)
    provenance?: { kind: string; ditherApplied: boolean };
  };
  progress?: { mode: string; stepIndex: number; elapsedMs: number; updatedAt: number };
}
```

导入端把 `cells` 经 `stores/patterns.ts` 既有 codec 转 `Int16Array` 落 `PatternDoc`；`progress.mode`
必须 ∈ `SPLIT_MODES`。任何违规都落在具名错误上（`SCHEMA_INVALID` 或 entry 级，带定位），
**没有静默修补路径**。

## 3. 与 B03 / B06 契约对账（消费端第三席）

1. **Repository 面零增量。** B07 全部需求被现有面覆盖：`loadPatternDoc` / `savePatternDoc`（导入写、
   导出读）、数组形 `loadProgress()` / `saveProgress(cursors)`（导出读游标；导入经 `upsertProgress`
   action 走既有 save effect）、`loadInventory()`。**不得**为导出复活 round2-data §5.3 旧稿的逐项目
   `loadProgress(id)` 撞名签名（round3-create-upload §2.2 的裁决第三次维持，B06 §2.1 同款）；也**不加**
   「枚举全部 patterns」方法——projects 数组是权威（D-IE-14）。
2. **写点登记**：`patterns` 仓的写点从两个（D-UP-10 双 CTA、D-ED-15 自动保存）变三个（+ 导入逐
   entry），三者都只经 `savePatternDoc` 单点。`deleteProject` 依旧无 UI 调用方（B03 §2.2 的记录
   维持，项目删除界面仍归后续 WP）；导入失败不需要它——`addProject` 是同步 reducer，唯一失败面
   是持久化，归 DATA-1 轨道。
3. **校验复用不重写**：文档读回门 = T-UP-5 同一套；游标规则 = `toProgressCursor`；库存条目 =
   B03 升级后的 `isInventoryEntry`（含 paletteId ∈ 已知集）。B07 只新增文件层校验（§2），
   对象层一律借既有的。
4. **B06 交互（非闸门）**：导入的带文档项目自动进 D-ED-1 可编辑集合与 DEV-ED-4「编辑豆图」
   链接的判别式——B06 合入先后不影响 B07 正确性。**已知合并接触面**：B06（blank 行）与 B07
   （两条 import 行）都改 `CreatePage.tsx` ENTRIES 与 `CreatePage.test.tsx` 用例 1，行级不相交、
   文本级会碰——后落地者负责 rebase（R-IE-2）。
5. **导入时序表**（何时写、写什么）：

| 时刻 | 写入 | 仓 |
|---|---|---|
| 选文件 / 嗅探 / 校验 / 汇总预览 | **无** | — |
| entry k：`await savePatternDoc` | PatternDoc | `patterns` |
| entry k 紧随：`addProject` → 既有 effect | Project | `state` |
| entry k 紧随：`upsertProgress`（若有） | 游标 | `progress` |
| 全部 entry 后：库存合并（新条目逐条 `addInventoryEntry`） | InventoryEntry | `state` |
| 导出（单项目 / 全部） | **零写入** | — |

## 4. 显式偏离（父代理可否决，逐条给回退方案）

| # | 偏离 | 内容与理由 | 被否决时的回退 |
|---|---|---|---|
| **DEV-IE-1** | 「+」页出现导出控件 | 「导出全部」放 `import-project` 面板（D-IE-15 ①）：导入/导出是同一备份通道的两个方向，同面板 = 零新路由、零第二处 Workspace 改面 | 挪 `WorkspacePage` 头部按钮；代价：紧接 DEV-ED-4 的第二次 Workspace 结构改动 |
| **DEV-IE-2** | Workspace 卡片加「导出 .beadproj」 | B07 对 `WorkspacePage.tsx` 的唯一改面（D-IE-15 ②），先例 DEV-ED-4；只挂带文档项目 | 单项目导出改为面板内项目下拉选择器；功能保全、发现性变差 |
| **DEV-IE-3** | v0 导入接收门 56/轴 | round1-map 的 512² 保留为**契约效度上限**（文件格式与 bead-v1 都容得下），v0 接收收窄到 56：57–512 的文档在 v0 没有任何能渲染它的消费端（拼装 DOM 画布、编辑器画布都钉 ≤3136 节点），收进来 = 铸造只能报错的项目 | 接收 ≤512²，给 `/assemble`、`/edit` 各加「过大文档」守卫屏 + Worker 化序列化（R-UP-1 触发条件被拉响）；机器面显著变大 |
| **DEV-IE-4** | 导入保留文件内 status、无双 CTA | 导入是恢复不是铸造：强改 status 会把备份里的 done/draft 全冲成待拼 | 全部导入盖 `todo`（或对单 entry 文件弹双 CTA）；归档恢复语义受损，不建议 |

## 5. 状态所有权增量（round1 §5 表的 delta）

| 状态 | 拥有者 | 持久化 | 写者 | 读者 |
|---|---|---|---|---|
| 导入文件 / 校验结果 / 汇总 | 面板页内 state | **禁止** | 页内 | 页内 |
| PatternDoc | Repository | IDB `patterns` 仓 | 第三个写点：导入逐 entry（皆经 `savePatternDoc`） | 既有消费端不变 |
| 项目 / 库存 / 游标 | ProjectStore / InventoryStore | IDB `state` / `progress` 仓（语义不变） | 既有 action（导入只是新调用方） | 既有 selector |
| 导出文件内容 | 无（读仓现算的纯函数） | **禁止** | — | 下载点击 |

## 6. 验收测试矩阵（T-IE-*）

纯函数（新 `schema/beadproj.test.ts` + `pages/create/import.test.ts`，无 DOM）：

| # | 断言 |
|---|---|
| T-IE-1 | 合法单 entry 文件通过；重建结果只含白名单键（种入未知键 → 输出无此键，绝不回写） |
| T-IE-2 | `format` 非法 → `SCHEMA_INVALID`；`version: 2` → `UNSUPPORTED_VERSION`（不尝试解析） |
| T-IE-3 | cells 长度不符 / 越界码（-2、48）/ `paletteId` 非 `generic-5mm` → entry 拒绝且错误带序号 |
| T-IE-4 | 尺寸门三层：513×10 → `SCHEMA_INVALID`；57×10 → `GRID_TOO_LARGE_FOR_V0`（文案含两个数字）；56×56 通过 |
| T-IE-5 | 双射：null-source 无 pattern / 画廊 source 带 pattern → 各自拒绝（D-IE-9） |
| T-IE-6 | 字段门：非法 status / >256 title / 非法 backdrop / 非有限 createdAt → entry 拒绝；trim 后空 title → 「未命名导入」 |
| T-IE-7 | 魔数嗅探：PNG / JPEG 头 → image 分类；`{` 首字节 → json 分类；其余（含伪 `.pat` 字节）→ unknown → `UNSUPPORTED_FORMAT`；嗅探只读前 16 字节（喂 15 字节文件不越界） |

存储 / 往返（`fake-indexeddb`，扩 `stores/patterns.test.ts` 或新页面级存储用例）：

| # | 断言 |
|---|---|
| T-IE-8 | **旗舰往返**（round2-data §5.6 归 B07 的 round-trip 兑现）：种「带文档项目（含 provenance）+ 画廊项目 + 双命名空间库存 + 游标」→ 导出全部 → 清空 → 导入 → 逐字段相等（id 除外，已重铸）；导出内容对同一 state 逐字节确定（无时间戳） |
| T-IE-9 | 同一文件导入两次 → 两套独立项目/文档/游标，零覆盖零碰撞（D-IE-8 的无 id 设计可观察） |
| T-IE-10 | 次序与断路：3 entry 文件、第 2 条 `savePatternDoc` 注入 reject → entry 1 完整落库、entry 2 无项目被铸、entry 3 未执行、汇总报「已导入 1 / 3」 |
| T-IE-11 | BD20：文件带 generic-5mm `G07` 库存 + 本地已有画廊 `G07` → 导入后两行并存互不抵扣（T-UP-18 延伸）；已存在 `(paletteId, code)` 跳过并计数；缺 / 未知 paletteId 丢弃并计数，**绝无盖章路径** |
| T-IE-12 | 存储卫生：导入一轮后枚举各仓与 localStorage——网格只在 `patterns` 仓一份，无第二套存储痕迹（T-UP-15 口径） |

面板集成（新 `pages/create/import-export.test.tsx`，经 `renderApp`）：

| # | 断言 |
|---|---|
| T-IE-13 | 两入口标注「现在就能用」；`?entry=import-pattern` / `import-project` 各渲染文件输入，无占位说明 |
| T-IE-14 | 文件门：>20MB（stub `File.size`）在读取前拒绝、内联 `FILE_TOO_LARGE`；坏 JSON → `SCHEMA_INVALID` 内联 |
| T-IE-15 | `import-pattern`：`.pat` / `.gamedev` → `UNSUPPORTED_FORMAT` 内联错误（文案不许诺解析）；PNG 字节 → 改道提示 + 指向 `?entry=upload` 的链接，全程零 `decodeImage` 调用（spy 断言） |
| T-IE-16 | 导入成功 → 汇总在 `role="status"`；项目现身 Workspace；带文档项目经 D-UP-14 分支在 `/assemble/:id` 渲染 `assemble-grid`（T-UP-16 复用）；导入的全空网格项目 → 拼装侧空网格态，不崩 |
| T-IE-17 | 单项目导出：blob URL 在点击时铸造（`createObjectURL` stub 记录调用时机，渲染期零调用——MED-1 反例锁死）、用后 revoke、文件名正确、内容过 §2 校验且恰一条 entry；画廊项目卡无导出链接 |
| T-IE-18 | 导出全部：内容含全部项目（画廊 entry 无 pattern 字段）+ 库存 + 游标；项目与库存皆空 → 控件不渲染 |
| T-IE-19 | 消毒：NE-1 静态扫描自动覆盖全部新文件（`isolation.test.ts` 递归 src，无需改动）——文件字节只进魔数比对与 `JSON.parse`，全树零 `fetch`/XHR/WebSocket/sendBeacon/外网 URL 字面量（BD15/BD17，含注释）、零 `eval`、零第三方解析依赖 |

**Do-not-regress**：`navigation.test.tsx`、`persistence-banner.test.tsx`、`empty-states.test.tsx`、
inventory 全部测试、`algo/` 全部测试、B03/B06 落地的 create/assemble 用例原样保绿。全绿口径
`pnpm --filter @bead/app test`；提交前根 `just ci` + e0-audit / denylist-audit 照 B10 惯例
（B07 零新依赖 ⇒ 锁文件不动）。

## 7. 非目标（实现者不得顺手做）

- `.pat` / `.gamedev` 逆向解析（等真实样本，round1-map §5）；`.png` 导出渲染图（导出面 v0 只有 `.beadproj`）。
- 实物照片逆向（透视矫正 + 网格提取）——PLAN 有、v0 无；`import-pattern` 的图片一律改道上传入口。
- 六角 / 圆板 / 多板拼接进 schema（D-IE-7：version 升级的事）；>56 网格的渲染消费端（DEV-IE-3 回退才需要）。
- ajv / zod / JSON Schema 文件、ZIP 容器、压缩、加密——零新依赖、纯 JSON。
- 导入预览 / 选择性导入 / 冲突合并 UI；项目删除与重命名 UI（`deleteProject` 仍无调用方）。
- `crates/**`、Soul 树、`.github/**`、`algo/**`、`stores/repository.ts` 接口面、`AssembleCanvas`——一个字节不碰。
- 剪贴板导入导出（零权限弹窗面维持）；分享出站（BD7/BD15）。

## 8. 残余风险（R-IE-*）

- **R-IE-1 · B03 实现漂移**：本文引用的三方法、codec、paletteId 字段全是 B03 IA 的规定动作；
  若 #45 落地形态有出入，B07 实现者以合入后真实 tip 为准并在 PR 里注明差异（R-ED-2 同款；
  B03 实现评审是闸门）。
- **R-IE-2 · CreatePage 合并接触**：B06 与 B07 都动 ENTRIES 与用例 1，后落地者 rebase；
  行级不相交，纯文本冲突，无语义风险。
- **R-IE-3 · `.pat`/`.gamedev` 全拒的体感**：v0 对这两格式没有一条成功路径，用户拿到的永远是
  `UNSUPPORTED_FORMAT`。缓解：入口文案与错误文案都只说「识别但暂不支持解析」，不许诺能力；
  真实样本收集仍挂 round1-map §5。
- **R-IE-4 · 归档复活下架图纸引用**：B08 若改 catalog，旧归档里的 `sourcePatternId` 可能查不到。
  既有降级（需求侧跳过、拼装侧 `NO_GRID_NOTE`）覆盖，记录不修。
- **R-IE-5 · 库存跳过合并的体感**：本地为准可能让用户以为「导入没生效」。缓解：汇总必报
  「跳过 y 条已存在」；求和/覆盖两种替代语义分别破坏幂等与本地编辑，维持跳过。
- **R-IE-6 · 20MB 主线程读取**：`file.text()` + `JSON.parse` 最坏一次性数百 ms，用户刚点了导入、
  有忙态，与 §7 Worker 顺延同一论证（v0 实际文件 KB 级）。记录不修。

## 9. NO_HIGH_VALUE_CHANGE_FOUND 子树

- JSON Schema 文件 + 运行时校验库：手写重建式校验器已 fail closed，schema 文件是 `$schema` 红线高发地。NO_HIGH_VALUE_CHANGE_FOUND。
- 导出格式版本协商 / 向后写 v0 格式：v1 是第一个版本，无历史读者。NO_HIGH_VALUE_CHANGE_FOUND。
- 拖拽导入 / 剪贴板粘贴：`<input type="file">` 覆盖 v0 需求路径（round3-create-upload §11 同款判定）。NO_HIGH_VALUE_CHANGE_FOUND。
- 导入历史 / 撤销导入：项目列表即归宿，误导入可等项目删除 UI（后续 WP）。NO_HIGH_VALUE_CHANGE_FOUND。
- `patterns` 仓 getAll / 游标索引：projects 数组驱动枚举已够，孤儿 doc 故意不导出。NO_HIGH_VALUE_CHANGE_FOUND。
- cells 的 RLE / base64 压缩：56² 明文 12KB，距 20MB 门三个数量级。NO_HIGH_VALUE_CHANGE_FOUND。

## 10. `CreatePage.test.tsx` 改写映射（与实现同 PR、独立 commit，禁止静默破坏）

前提：B03 / B06 已各自按其 §5 / §5.4 处理了自己的占位断言。**本 IA 拥有的是两条 B07 断言**；
在 B07 实现 PR 之前它们必须原样保绿（本轮不动测试）：

| 断言（B03/B06 落地后的 `CreatePage.test.tsx`） | 处置 |
|---|---|
| 用例 1：`getAllByText("尚未实现，归 WP-B07")` 长度 2 | **替换**为两 import 入口标注「现在就能用」（`owner: null` 既有渲染路径；断言「尚未实现」文本从此在 `/create` 绝迹） |
| 用例 1：B03 / B06 的断言行 | **不动**（各归其 WP，已处理或将处理） |
| import 入口占位说明页断言（若存在） | **整体替换**为 T-IE-13 的面板测试（文件输入在、占位说明无） |
| 画廊入口用例 | **不动** |

连带：两入口 `detail` 文案改写为实际能力（`.beadproj` 导入导出、`.pat`/`.gamedev` 识别即拒、
图片改道上传），`ENTRIES` 里两条 `owner` 改 `null`。`WorkspacePage` 既有测试若断言卡片结构，
按 DEV-IE-2 在原文件内改写不删用例。

## 11. Ready-for-Opus 清单（照此下刀，逐项可勾）

- [ ] 前置确认：B03 实现（#45）已合入 tip，三方法 / `stores/patterns.ts` codec / `InventoryEntry.paletteId` / `fake-indexeddb` 在场。
- [ ] `schema/beadproj.ts`（新）：§2 形状、重建式校验器、`ImportErrorCode`、尺寸三层门；`beadproj.md` 格式散文；`beadproj.test.ts`（T-IE-1…6）。
- [ ] `pages/create/import.ts`（新）：魔数嗅探（≤16 字节）与文件分类纯函数（T-IE-7）。
- [ ] `pages/create/ImportPatternPanel.tsx` / `ImportProjectPanel.tsx`（新）：D-IE-10 执行序、D-IE-12 库存合并、汇总 `role="status"`、导出全部控件。
- [ ] `pages/create/CreatePage.tsx`：两入口 `owner: null` + detail 改写；测试映射按 §10 独立 commit。
- [ ] `pages/workspace/WorkspacePage.tsx`：仅 DEV-IE-2 的一条导出链接（获准后）。
- [ ] 导出纯函数（面板私有或 `pages/create/export.ts`）：state → `.beadproj` 字符串（确定性、无时间戳）；下载铸造点按 D-IE-16。
- [ ] 测试：§6 全表；`pnpm --filter @bead/app test` 全绿 + 根 `just ci` 绿 + e0 / denylist 双绿。
- [ ] 红线自查：零新依赖、零锁文件改动、零外网 URL（含注释与 `beadproj.ts`，NE-1/e0 会抓）、`algo/**` / `stores/repository.ts` 接口 / `crates/**` / Soul 树零改动。

## 12. 禁区自查

本轮只新增 `docs/bead/reviews/round3-import-export.md`。未改 `apps/bead` 任何源码（全部留给实现席），
未读取 B03 实现工作树（`/tmp/wt-b03`）的未提交文件，未触碰 `docs/PRODUCT_LOCK.md`、`docs/STATUS.md`、
`docs/FORMAL_WORK_PROMPT.md`、`apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml`、`deny.toml`、
`.github/**`、`pnpm-lock.yaml`。本文无外网 URL，仅引用仓内路径（BD15/BD17）。
