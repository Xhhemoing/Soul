# ROUND 3 · WP-B08 Explore fixture 画廊 — 前端 / 数据 IA

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只做 IA / 拆分，不实现）。
基线：`origin/cursor/beadflow-integration-c441` @ `10619fd`（B07 导入导出 + 其评审吸收之后）。
输入：`docs/bead/PLAN.md`（Explore / 创作者主页 / v0 范围「fixture 画廊，不做真实社交后端」）、`docs/bead/WORK_PACKAGES.md`（WP-B08）、
`docs/agent-decisions.md` BD7 / BD15 / BD17 / BD19 / BD20、`round1-frontend.md`（D-UI-1/4/5/7、§5 状态所有权、§6 单链接卡）、
`round1-map.md` §4 B08（fixture 无外链字段、数据放目录名字面为 `fixtures` 的路径）、
`round3-create-upload.md`（D-UP-10/11/12/14/16、T-UP-5/14）、`round3-pixel-editor.md`（D-ED-1/4/17/18、R-ED-3 的 Fork 缺口）、
`round3-import-export.md`（D-IE-9 双射、D-IE-14、R-IE-4）、
`apps/bead/src/fixtures/`（catalog.ts / grids.ts）、`stores/`（catalog / types / ids / projects / patterns / usePatternDoc / store）、
`pages/explore/`（ExplorePage / PatternFeed / PersonalStrip）、`pages/pattern/PatternDetailPage.tsx`、`pages/creator/CreatorPage.tsx`、
`pages/edit/EditPage.tsx`（守卫文案）、`algo/palette.ts`（GENERIC_5MM、nearestIndex）、`app/navigation.test.tsx`。
**未读**：任何 `/tmp/wt-*` 实现工作树的未提交文件——本文只对 exclusive tip 与已合入的 IA/评审成文。
硬禁区自查：本轮只新增本文件；不触碰 Soul 锁、`apps/desktop`、`crates/soul-*`、根配置、`deny.toml`、`.github/**`、锁文件。

---

## 0. 结论先行（本 WP 的法律）

1. **B08 的两块实肉是「fixture 扩容」与「Fork 改色」。** 发现流、标签筛选、详情页五字段、创作者页
   在 B02 已交付并有测试，B08 不重做、不重构——`catalog.ts` 头注「WP-B08 grows this set;
   nothing else here should change」就是边界。CatalogStore 保持构建期只读、无 provider
   （round1 §5 写者一栏「无（B08 只扩 fixture）」在本 WP 原样兑现）。
2. **Fork 色板拍板（B06 R-ED-3 留下的缺口，本文定案）：fork 时把画廊色板量化到
   `generic-5mm`，不给 `PatternDoc` 保留 gallery 命名空间。** 理由见 D-GAL-5——
   `paletteForNamespace("gallery")` 返回 null 是契约本身，不是缺口。
3. **Fork 写盘只走 B03 的 `savePatternDoc`，次序照 D-UP-10：doc 成功落盘后才 mint 项目。**
   fork 产物是一个普通的 null-source + doc 项目，编辑器（D-ED-1）、拼装（D-UP-14）、
   库存需求（useConversionDocs）、B07 导出双射（D-IE-9）**全部零改动自动收编**。
4. **无真实社交后端（BD7）在每个子决策里重申**：checkIns 静态、fork 不增计数、无发布回画廊、
   无评论/点赞/OAuth；fixture 数据零外链字段（BD15 + round1-map §4 B08）。

## 1. 决策表（D-GAL-*，实现者照抄，不自行发明产品法）

| # | 决策 | 内容 |
|---|---|---|
| **D-GAL-1** | 范围裁定 | B08 = fixture 扩容 + Fork 改色 + 两处文案收尾。发现流 / `?tag=` / `?fav=1` / 详情五字段 / 创作者页骨架不动。画廊数据恒为构建期只读代码：无运行时画廊写入、无「把 fork 发布回画廊」（那需要后端，BD7 判死） |
| **D-GAL-2** | 扩容形状 | **append-only**：既有 5 张图纸与 2 位创作者的 id、色板、网格一律不删不改名（R-IE-4：旧归档的 `sourcePatternId` 要能继续命中 catalog）。新增 **4 张图纸（每张必须带 `grids.ts` 网格）+ 1 位创作者**（字段仅 `{id,name,bio,checkIns}`，禁止头像/主页/社交句柄/任何 URL 字段）。扩容后每个 TAG（二次元/像素游戏/立体拼豆/节日限定，封闭四集不扩）至少 2 张图纸。每张色板 ≤10 条（`grids.ts` 的 `0`–`9` 编码硬上限），建议 4–6 条；板型只许 square-28 / square-56（六角/圆板 post-v0 照旧）。颗数/色板 beads 由网格派生，`grids.test.ts` 既有机制自动抓 drift（D-ASM-2） |
| **D-GAL-3** | 存量网格补课 | `gal-torii-02` 本轮**补上网格**（56×56，程序化 paint 先例），其 `beadCount`/palette beads 随之改为派生值——展示数字变化是 D-ASM-2 的正常代价（DEV-GAL-3 可否决）。`gal-cakebox-03` **必须保持无网格**：它是「暂无网格」态（D-ASM-12）与 fork 禁用门（D-GAL-10）的活例，测试压着它 |
| **D-GAL-4** | 推荐流 | v0 推荐 = **fixture 声明序**（catalog 数组顺序即编辑推荐位），Explore 渲染现状零改动。不做加权、不做随机、不做热度计数——任何「推荐算法」都需要行为数据，BD7 下没有行为数据的合法来源 |
| **D-GAL-5** | Fork 色板拍板 | **fork 时 gallery → generic-5mm 量化**，弃「保留 gallery 命名空间」方案。三条硬理由：① `gallery` 是「逐图纸局部色表的码命名空间」，没有全局稳定索引的色表——`paletteForNamespace` 返回 null、`toPatternDoc` 整篇拒收（T-UP-5 已钉）是**设计**而非缺口；收编它 = 发明全局画廊色表注册表 + 翻改 codec/读回校验/`boardSourceOf`/库存 hook/`beadproj` schema 五个面。② 量化后 fork 立即免费获得整个下游：编辑器选色器绑死 `GENERIC_5MM`（D-ED-6）、拼装 null 分支（D-UP-14）、库存需求（BD20 机器）、B07 导出导入。③ fork 的产品语义就是「二创改色」（PLAN）——把图带进可编辑色板是目的本身，不是妥协 |
| **D-GAL-6** | 量化机制 | **按色板条目（per-entry）而非逐格**：对 `pattern.palette` 每条 hex → Rgb → `nearestIndex(rgb, preparePalette(GENERIC_5MM))`（ΔE00 最近邻，**并列取最小下标**——G2/G6 血统的既有裁决，`nearestEntry` 已实现）；`cells` 经这张 ≤10 条的映射表重写，空格保持 `-1`。per-entry 保证同色恒同映射、合并可计数（O(色板) 次 ΔE 而非 O(3136)）。hex→Rgb 解析是 fork 模块本地纯函数（`palette.ts` 的同款解析是私有 helper；**不动 `algo/`**——它是 oracle parity 域，fork 是产品操作，import 方向 pages→algo 合法，D-ED 结论 4 同款） |
| **D-GAL-7** | Fork 序列 | 详情页一键：`await savePatternDoc(createPatternDoc(projectId, 量化网格))` **成功后** → `addProject(createProjectFromFork(id, title))` → `navigate(/edit/:id)`。`createProjectFromFork(id, title)` 进 `stores/projects.ts` 第四个 mint（status **draft**、`sourcePatternId: null`、backdrop black/`#101014`），与 blank/conversion 并排——不复用 `createProjectFromBlank` 是为了名字不说谎（blank 的注释钉着「全 -1」）。标题 = `原题（Fork）`，trim ≤64（D-UP-11/D-ED-4 同限）。失败 = 内联错误、不 mint、不导航（D-UP-10/D-UP-16 镜像）。同图可多次 fork，各自独立项目 |
| **D-GAL-8** | source 判别 | fork 产物 **`sourcePatternId` 必须为 null**，这是被三条既有契约钉死的结论而非偏好：① D-ED-1 守卫只放行 null-source ∧ doc 命中，非 null 直接进只读屏；② D-UP-14 的拼装分支对非 null 项目读 fixture 网格——改色后会拼出**原色**；③ D-IE-9 双射「null ⇔ 有 doc」，非 null 带 doc 的 entry 在 B07 导入端整条被拒。出处只进标题（人读），不加归因字段（BD7 下无归因面，加字段 = 双权威） |
| **D-GAL-9** | provenance | fork 文档**不带 `provenance`**（字段可选）。fork 既非 `PixelArt` 也非 `Photo`；扩 kind union = schema 增量跨 `toProvenance` 校验、`beadproj` schema、B07 测试三处，服务一个展示性字段——判死（D-ED-14 同款裁决哲学） |
| **D-GAL-10** | 可用性门 | 只有 `fixtureGridFor(pattern.id) !== null` 的图纸可 fork。无网格图纸（当前 gal-cakebox-03）：按钮**禁用 + 常显理由**「这张图纸还没有网格，暂不能 Fork」（禁用而非隐藏：可发现性 + 与拼装「暂无网格」态同一语言）。fork 读 fixture 网格是同步的，无加载态 |
| **D-GAL-11** | 按钮落点 | Fork 是 `PatternDetailPage` 的 `DetailActions` 第四键（加入待拼 / 转入工作台 / 收藏 / **Fork 改色**）。PatternCard 保持整卡单链接（round1 §6 不变量），创作者页与发现流均经卡→详情一跳到达 fork。`CreatorPage` 删除 stub note「Fork 二创改色归 WP-B08…」。对 PLAN「创作者主页一键 Fork」的字面偏离记 DEV-GAL-1 |
| **D-GAL-12** | 详情页预览 | 有网格的图纸把空占位 `pattern-card__preview` 换成**真网格预览**：页面私有组件，复用 `.assemble__bead` 渲染技术但**不复用 `AssembleCanvas`**（其 `cellStates` 语义是拼装会话的——D-UP-9 / D-ED-7 的第三次同款裁决）；非交互、格子非聚焦、容器 `aria-hidden` + 相邻文字描述照旧（色号清单就是文字替代面）。56×56 单实例 3136 节点，D-ASM-6 论证成立。无网格图纸保持现状占位。范围加法，记 DEV-GAL-2 可砍 |
| **D-GAL-13** | 无社交重申 | `checkIns` 是静态 fixture 数字，fork / 收藏 / 实例化都不增它；无评论、点赞、关注、分享出站；无账号/OAuth；收藏照旧本地 store。「被 fork 数」之类的计数面一并不做（没有后端就没有真数，假数比没有更糟） |
| **D-GAL-14** | 文案收尾 | ① `pages/edit/EditPage.tsx` 的 `GALLERY_NOTE` 从「Fork 改色归 WP-B08」改为指路：「画廊图纸的网格是只读的——回图纸详情页 Fork 一份副本即可改色。」（字符串级改动，守卫逻辑一字不动）；② `PatternFeed.tsx` 头注「a Fork button arrives with WP-B08…」改为指向详情页；③ `catalog.ts` 头注的「WP-B08 grows this set」随扩容更新。映射见 §5.4 |

## 2. 契约对账（fork 是纯消费者，五个面零增量）

1. **Repository 面零增量**：fork 只调 `savePatternDoc`（B03 三方法之一），成为 `patterns` 仓第四个
   写点（上传双 CTA、编辑器自动保存、B07 导入之后），全部写入仍经同一扇门。`store.tsx` 的
   `addProject` action 原样够用，reducer 零改动。
2. **编辑器零改动收编**（除 D-GAL-14 ① 的一条字符串）：fork 产物过 D-ED-18 守卫链的每一关；
   画廊实例化项目（「加入待拼」/「转入工作台」mint 的非 null source）照旧进只读屏——fork 与
   实例化是两个动词，前者给你可改的副本，后者给你可拼的原件。
3. **拼装零改动**：fork 产物走 D-UP-14 的 null 分支（`usePatternDoc` → `boardSourceOf` →
   generic-5mm swatch）；画廊项目照旧走 `fixtureGridFor`。
4. **库存零改动**：fork 产物经编辑器「去拼装」提为 active 后（D-ED-17 ③），`useConversionDocs`
   自动把它的 `buildBom` 需求并进缺口，BD20 命名空间机器照转。
5. **B07 零改动**：fork 产物 = null-source ∧ doc，D-IE-9 双射自动成立；单项目导出链接
   （D-IE-15 ②）在 Workspace 卡上自动出现；`beadproj` schema 不动（doc 无 provenance，字段本可选）。
6. **读回校验对称**：量化映射的值域是 `[0, 48)`，`toPatternDoc` 的越界检查天然通过；
   编码复用 `stores/patterns.ts` 的 `createPatternDoc`/`encodeCells`，零第二套 codec。

## 3. 状态所有权增量（round1 §5 表的 delta）

| 状态 | 拥有者 | 持久化 | 写者 | 读者 |
|---|---|---|---|---|
| 画廊图纸/创作者/标签（扩容后） | CatalogStore | 构建期 fixture，**只读** | 无（本 WP 只扩 fixture 文件本身） | Explore / Pattern / Creator / fork 映射 |
| fork 量化映射表 / 合并计数 | 无（fork 瞬时现算） | **禁止** | — | fork 序列内部 |
| fork 产物 `PatternDoc` | Repository | IDB `patterns` 仓 | `savePatternDoc`（第四写点） | 编辑器 / 拼装适配 / 库存需求 / B07 导出 |
| fork 产物项目元数据 | ProjectStore | IDB `state` 仓 | 既有 `addProject` | Workspace / 守卫链 |

## 4. 显式偏离（父代理可否决，逐条给回退方案）

| # | 偏离 | 内容与理由 | 被否决时的回退 |
|---|---|---|---|
| **DEV-GAL-1** | Fork 落详情页 | PLAN 写「创作者主页……一键 Fork」；本文放 `DetailActions`。理由：PatternCard 单链接不变量（round1 §6）禁止卡内第二点击目标；fork 需要「无网格禁用 + 理由」的解释面，卡上没有位置；创作者页到详情一跳之遥 | 创作者页网格视图给每卡**卡外** fork action（round1 §6 预留的排布方案）；代价：卡外按钮与卡的视觉配对、CreatorPage 测试面翻倍 |
| **DEV-GAL-2** | 详情页真网格预览 | WP 文本没点名预览；但「从画廊选」与「fork 前看一眼」都压在这块空灰 div 上，fixture 网格现成 | 保持占位 div；删 T-GAL-8；其余决策不受影响 |
| **DEV-GAL-3** | torii 补网格改动既有展示数字 | `gal-torii-02` 的 1540 手写估值变为网格派生值（D-ASM-2 机制）。动了既有 fixture 的可见数字 | torii 不补网格；fork 面少一张 56 板大图，「暂无网格」双例并存，其余不动 |

## 5. 验收测试矩阵（T-GAL-*）

### 5.1 纯函数（新 `pages/pattern/fork.test.ts`，无 DOM）

| # | 断言 |
|---|---|
| T-GAL-1 | 量化映射：对**全部**现有画廊色板逐条钉死字面映射表（实现算一次、人工过目后写成字面量，drift 即红）；并列取最小下标；hex 解析对 `#RRGGBB` 全量画廊值正确 |
| T-GAL-2 | `forkGrid`：保形（width/height/空格 `-1` 原位）；cell 经映射表重写；构造两条画廊色映到同一 G 码的案例 → 合并计数正确 |

### 5.2 集成（`renderApp` harness + 内存仓库双身）

| # | 断言 |
|---|---|
| T-GAL-3 | 详情页 fork → doc 在 `patterns` 仓（`generic-5mm`、尺寸 = fixture 网格、cells 过 `toPatternDoc`）→ 项目 draft / null source / 标题「…（Fork）」→ 落 `/edit/:id` 且 `editor-grid` 渲染（守卫放行） |
| T-GAL-4 | `savePatternDoc` 注入 reject → 内联错误、项目数不变、不导航（T-UP-14 / T-ED-8 同款） |
| T-GAL-5 | 无网格图纸（gal-cakebox-03）：fork 按钮禁用 + 理由文案在场；有网格图纸按钮可用 |
| T-GAL-6 | 闭环：fork → 编辑器全局替换一个色号 → 「去拼装」→ `assemble-grid` 的 swatch 来自 generic-5mm 且反映替换后颜色（T-ED-19 形状复用——这一测同时锁死 D-GAL-8 ②：绝不读回 fixture 原色） |
| T-GAL-7 | fork 两次 = 两个独立项目；fork 后 `checkIns` 与 catalog 数据不变（D-GAL-13） |
| T-GAL-8 | 详情页：有网格图纸渲染预览（testid，非聚焦、无点击目标）；无网格保持占位（DEV-GAL-2 获准时） |

### 5.3 扩容完整性

| # | 断言 |
|---|---|
| T-GAL-9 | `grids.test.ts` 既有派生断言零改动收编全部新图（迭代 `GRIDDED_PATTERN_IDS`）；每个 TAG ≥2 张图纸；全部新增图纸都在 `GRIDDED_PATTERN_IDS`；既有 5 个 `gal-*` id 原样在场（append-only） |
| T-GAL-10 | torii 补网格后 beadCount / palette beads = 派生值（grids.test 自动抓；DEV-GAL-3 获准时） |
| T-GAL-11 | NE-1 静态扫描自动覆盖全部新文件与 fixture 数据：零 `fetch`/XHR/WebSocket/sendBeacon/外网 URL 字面量（含注释，BD15/BD17）；新 Creator/Pattern 对象无 URL 形字段 |

### 5.4 既有测试改写映射（与实现同 PR、独立 commit，禁止静默破坏）

| 现有断言 | 处置 |
|---|---|
| `edit.test.tsx`「画廊项目：只读，文案点名 WP-B08」（断言 `/WP-B08/` 与 `/只读/`） | **改写不删**：守卫行为断言原样（不渲染 `editor-grid`、有出路），文案断言改为新 `GALLERY_NOTE`（`/只读/` 与 `/Fork/` + `/详情页/`） |
| `navigation.test.tsx` 创作者页断言（`被拼打卡数：128`、史莱姆链接） | **不动**（append-only 保证保绿）；CreatorPage stub note 若被任何测试断言，同 commit 改写 |
| `empty-states.test.tsx` / `instantiate.test.tsx` / inventory / assemble / `algo/` 全部测试 | 原样保绿。全绿口径 `pnpm --filter @bead/app test`；提交前根 `just ci` + e0-audit / denylist-audit 照 B10 惯例（零新依赖，锁文件不动） |

## 6. 消毒 / 不出网

- 零新运行时依赖（D-UI-7）、零锁文件改动、零 `.github/**` 改动。
- fixture 数据全部留在目录名字面为 `fixtures` 的路径（`apps/bead/src/fixtures/`，round1-map §4 B08）；
  即便该树被 audit 豁免，仍执行零外网 URL（NE-1 递归 src 会扫到，BD15 不分树）。
- 无图片/二进制资产：预览由网格派生，创作者无头像（D-GAL-13）。
- fork 写盘单点 `savePatternDoc`；无 `eval`、无 `dangerouslySetInnerHTML`、无新权限面。

## 7. 允许改面（越此即 drive-by）

1. `fixtures/catalog.ts`、`fixtures/grids.ts`（+ 其测试）——扩容与 torii 网格。
2. `pages/pattern/`：`PatternDetailPage.tsx`（fork 键 + 预览挂载）、新 `fork.ts`（量化纯函数）、
   新预览组件、各自测试。
3. `stores/projects.ts`：`createProjectFromFork`（唯一 stores 改动）。
4. `pages/creator/CreatorPage.tsx`：删 stub note 一行。
5. `pages/explore/PatternFeed.tsx`：头注一行（D-GAL-14 ②）。
6. `pages/edit/EditPage.tsx`：`GALLERY_NOTE` 字符串一处。

**零改动**：`algo/**`、`crates/**`、`stores/{repository,types,patterns,inventory}.ts`、`store.tsx`、
`schema/**`、`pages/{assemble,inventory,create,workspace}/**`、Soul 树、根配置。

## 8. 残余风险（R-GAL-*）

- **R-GAL-1 · 量化保真**：画廊 hex 与最近 G 码有 ΔE 距离，fork 后颜色微移、偶发两色并一
  （T-GAL-2 的构造案例证明可能）。编辑器就是补救面（fork 的下一站），接受不阻塞；
  不做 fork 前确认对话框（「一键」是 PLAN 原文，统计面板会如实展示结果码）。
- **R-GAL-2 · 出处只在标题**：改标题即断链。BD7 下没有归因后端，加字段 = 无消费者的双权威；接受。
- **R-GAL-3 · 网格手工成本**：4 张新图 + torii 都要程序化 paint（grids.ts 先例），这是 B08 实现的
  主要工时；测试只能抓机械 drift，图好不好看是产品判断，评审席过目。
- **R-GAL-4 · 归档引用**（R-IE-4 承接）：append-only + 禁改名把「旧归档 `sourcePatternId` 查不到」
  压到零新增风险；后续任何「下架」需求必须走软下架而非删条目。
- **R-GAL-5 · 同图多 fork 同名**：两个「…（Fork）」标题并存；v0 无重命名 UI（B06 已记录），接受。
- **R-GAL-6 · 预览节点数**：56×56 详情页预览 3136 节点单实例可行（D-ASM-6）；正因如此
  feed 缩略图判 NO_HIGH_VALUE（×N 卡不可行），两个结论是同一枚硬币。

## 9. NO_HIGH_VALUE_CHANGE_FOUND 子树

- 推荐引擎 / 热度 / 随机轮换：无行为数据合法来源（BD7）。NO_HIGH_VALUE_CHANGE_FOUND。
- feed 卡缩略图：R-GAL-6 的算术。NO_HIGH_VALUE_CHANGE_FOUND。
- fork 前配色预览/重映射 UI：编辑器统计与替换面板就是该面，前置它 = 造第二个编辑器。NO_HIGH_VALUE_CHANGE_FOUND。
- fork 谱系 / 「被 fork 数」/ 归因字段：无后端无真数（D-GAL-13）。NO_HIGH_VALUE_CHANGE_FOUND。
- 用户作品发布回画廊：catalog 是构建期代码，运行时写入 = 造社交后端。NO_HIGH_VALUE_CHANGE_FOUND。
- 搜索 / 分页 / 虚拟化：fixture <50，round1 判定维持。NO_HIGH_VALUE_CHANGE_FOUND。
- 标签集扩容：四标签是 PLAN 原文且筛选行测试压着；新图用旧标签即可。NO_HIGH_VALUE_CHANGE_FOUND。

## 10. Ready-for-Opus 清单（照此下刀，逐项可勾）

- [ ] `fixtures/catalog.ts` + `grids.ts`：+4 图纸（带网格）+1 创作者；torii 网格（DEV-GAL-3）；append-only 自查。
- [ ] `pages/pattern/fork.ts`（新）：hex 解析、per-entry 量化映射、`forkGrid`、合并计数 + `fork.test.ts`（T-GAL-1/2）。
- [ ] `stores/projects.ts`：`createProjectFromFork`。
- [ ] `PatternDetailPage.tsx`：fork 键（可用性门 D-GAL-10、序列 D-GAL-7、失败内联）+ 预览挂载（DEV-GAL-2）。
- [ ] 新预览组件（页面私有，非交互）。
- [ ] 文案三处（D-GAL-14）+ §5.4 映射改写为独立 commit。
- [ ] 测试：§5 全表；`pnpm --filter @bead/app test` 全绿 + 根 `just ci` 绿 + e0/denylist 绿。
- [ ] 红线自查：零新依赖、零锁文件、零外网 URL、§7 零改动清单逐条核对。

## 11. 禁区自查

本轮只新增 `docs/bead/reviews/round3-gallery.md`。未改 `apps/bead` 任何源码（全部留给实现席），
未读取任何 `/tmp/wt-*` 实现工作树，未触碰 `docs/PRODUCT_LOCK.md`、`docs/STATUS.md`、
`docs/FORMAL_WORK_PROMPT.md`、`apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml`、`deny.toml`、
`.github/**`、`pnpm-lock.yaml`。本文无外网 URL，仅引用仓内路径（BD15/BD17）。
