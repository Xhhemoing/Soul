`claude-fable-5-thinking-xhigh`

# ROUND 3 · WP-B08 Explore fixture 画廊 + Fork 改色 · 实现审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审查，不实现）。
被审对象：吸收提交 `0c1f54d`（absorb WP-B08 gallery Fork，即 PR #54 的吸收面），
基线 `origin/cursor/beadflow-integration-c441` @ `2dcd1bb`（B08 画廊 IA #53 合入后）。
审查依据：`docs/bead/reviews/round3-gallery.md`（D-GAL-1…14 / T-GAL-1…11，本 WP 的法律）、
`round3-pixel-editor.md`（D-ED-1 判别式）、`round3-create-upload.md`（D-UP-10/14/16）、
`round3-import-export.md`（D-IE-9 双射、R-IE-4）、`docs/agent-decisions.md` BD7 / BD15 / BD19 / BD20。
本轮只新增本文件；未改任何产品文件、测试文件、rust、Soul 树。

范围对账：吸收提交对 `apps/bead/**` 与实现分支 tip `51dd0cf` **逐字节一致**（`git diff` 空），
额外只有父代理记账 `docs/agent-progress.md`。实现分支四个 commit 满足纪律：
`aaf7e3b`（fixtures 扩容 + torii 网格 + catalog.test.ts）、`aa96c7a`（fork 纯函数 + 详情页 +
预览 + T-GAL 矩阵）、`93d6a3e`（**单独承载** D-GAL-14 ① 与 edit.test.tsx 改写，§5.4 纪律，
B03 / B06 / B07 同款）、`51dd0cf`（gift ribbon 去重构，图形零变化）。零新依赖、零锁文件改动、
零 `.github/**` 改动；`store.tsx` / `stores/{repository,types,patterns,inventory}.ts` /
`schema/**` / `algo/**` / `pages/{assemble,inventory,create,workspace}/**` 全部零字节差——
IA §7 允许改面之外只多了 `stores/inventory.test.ts` 两个字面量，见 §5.2 的账。

---

## 0. 结论先行

**ACCEPT-WITH-NITS。0 条 HIGH，0 条 MED，2 条 LOW。** 任务钉出的七条硬红线全部锁死（§1），
D-GAL-1…14 逐条成立（§2，其中 D-GAL-6 的 hex 解析一处字面偏离经核判优），T-GAL-1…11 每行
都有真测试落点（§3）。本审查独立工作树实测 `pnpm --filter @bead/app test` =
**39 文件 / 608 条全绿**（父代理门禁 39/608，命中）、`tsc --noEmit && eslint .` 零输出、
根 `e0-audit` / `denylist-audit` 双清洁。实现者三条旗标（parseHexColor 复用、inventory.test.ts
torii 字面量、预览不动 app.css）**逐行核实为真且正当，确认**（§5）。两条 LOW 都是一行级
打磨（一句过时的测试散文、一个不可达的灰色回退），无一阻塞。不重开架构。

## 1. 任务钉出的硬红线（逐条对树验）

1. **fork 产物 `sourcePatternId` 恒 null ✅。** `createProjectFromFork` 硬编码 null 并把
   三条契约写进注释（projects.ts:91-104）；fork-page.test.tsx:39-44 断言落库值；T-GAL-3
   实走 D-ED-1 守卫（`editor-grid` 渲染即放行），T-GAL-6 锁死拼装绝不读回 fixture 原色。
2. **Grid 不进 localStorage ✅。** fork 写盘单点 `await savePatternDoc`（PatternDetailPage.tsx:85）
   → `patterns` 仓；B08 四个新文件全树无 `localStorage` token；fixture 网格本体是构建期代码 +
   内存 cache（grids.ts:427-449），「nothing here is ever persisted (BD19)」头注原样成立。
3. **fork 操作不进 `algo/` ✅。** `algo/**` 对基线零字节差；fork.ts:22-23 只 import
   `createGrid` / `GENERIC_5MM` / `nearestIndex` / `preparePalette`，方向 pages→algo 合法
   （D-ED 结论 4 同款）；并列取最小下标未另立实现，直接吃既有 `nearestEntry` 的严格 `<`
   （palette.ts:134，fork.test.ts:101-118 双向对拍钉死）。
4. **cakebox 未获网格 ✅。** `SPECS` 无 `gal-cakebox-03` 且缺席理由成文（grids.ts:409-412）；
   catalog.test.ts:40-46 与 grids.test.ts:69-71 双处钉 null；D-GAL-10 禁用门就压着它测
   （fork-page.test.tsx:94-98）。
5. **既有 gal-* id 未改名未删 ✅。** catalog.ts 的 diff 里五张旧图 id / title / 色板
   code / name / hex 零改动——torii 仅 beads / beadCount / estimatedMinutes 改为网格派生值
   （DEV-GAL-3 授权）；grids.ts 对旧图形只删了 3 行注释（cakebox 缺席理由改写），slime /
   lantern / arcade 的画法逐字节未动；catalog.test.ts:27-34 把 append-only（含数组位置）
   钉成断言，R-IE-4 的旧归档命中面无新增风险。
6. **零 URL 字面量 ✅。** isolation.test.ts 递归扫描（fetch / XHR / WebSocket / sendBeacon /
   远程 scheme 五模式）自动收编全部新文件；catalog.test.ts:99-117 加码两层：URL 形**字段名**
   扫描（avatar / homepage / handle 等 15 个键形）+ 序列化串四模式扫描（scheme、协议相对
   `//`、`www.`、裸域名后缀），Creator 键集闭合断言 `["bio","checkIns","id","name"]`。
   本审查另 grep 新文件零命中。
7. **AssembleCanvas 未被预览复用 ✅。** `PatternPreview`（PatternPreview.tsx:22-37）是页面
   私有组件，第三次做出 D-UP-9 / D-ED-7 的同款裁决并把理由写进头注；借的是
   `.upload__grid` / `.assemble__bead` 两个既有 class（app.css 零 diff），无 `cellStates`、
   无交互、无聚焦目标、容器 `aria-hidden`（T-GAL-8 逐项断言）。

## 2. D-GAL-1…14 落点速查

| ID | 判定 | 落点 |
|---|---|---|
| D-GAL-1 | ✅ | 发现流 / `?tag=` / `?fav=1` / 详情五字段 / 创作者页骨架全部不在 diff（PatternFeed 仅头注、CreatorPage 仅删 stub 一行）；CatalogStore 保持构建期只读、无 provider、无运行时写者（fork-page.test.tsx:170-180 断言 fork 后 catalog 串逐字节不变） |
| D-GAL-2 | ✅ | +4 图纸（gift-06 / mochi-07 / mush-08 / quilt-09，全部在 `SPECS` 有真网格）+1 创作者（cr-nagi，恰四字段）；append-only 连数组位置一起钉（catalog.test.ts:27-38）；四 TAG 各 ≥2（:48-59，反向断言无板外标签）；色板 4–5 条 ≤10、板型只 square-28/56、hex 全 `#RRGGBB` 小写（:61-76） |
| D-GAL-3 | ✅ | torii 56×56 程序化 paint（grids.ts:189-218，六矩形鸟居 + halo 外描边——四格宽柱子容不下内 rim 的理由成文）；beadCount 1776 = `occupiedCount` 派生且明断 ≠1540（catalog.test.ts:79-89）；cakebox 保持无网格（§1.4） |
| D-GAL-4 | ✅ | 推荐 = catalog 声明序，Explore 渲染零改动；diff 里无任何加权 / 随机 / 计数代码 |
| D-GAL-5 | ✅ | fork.ts 整篇量化进 `GENERIC_5MM`，gallery 命名空间不进 `PatternDoc`（fork-page.test.tsx:49 断言 `paletteId: "generic-5mm"`）；头注复述「null 是契约不是缺口」 |
| D-GAL-6 | ✅* | per-entry 量化（fork.ts:54-56，O(色板) 次 ΔE）；cells 经映射表重写、空格 `-1` 原位、越界 throw（forkGrid:63-71）；并列最小下标吃既有 `nearestEntry`（§1.3）。*hex 解析未做「fork 模块本地纯函数」而是复用 `stores/inventory.ts` 的 `parseHexColor`——字面偏离，经核判优，见 §5.1 |
| D-GAL-7 | ✅ | `await savePatternDoc(createPatternDoc(projectId, 量化网格))` 成功后才 `addProject` → `navigate(/edit/:id)`（PatternDetailPage.tsx:74-95，D-UP-10 次序第三次复用）；`createProjectFromFork` 进 projects.ts 第四 mint（draft / null / black / `#101014`）；标题 `原题（Fork）` trim 含后缀 ≤64（fork.ts:95-98，fork.test.ts:187-191 钉 64 边界）；失败内联、不 mint、不导航（T-GAL-4）；多次 fork 各自独立（T-GAL-7） |
| D-GAL-8 | ✅ | null 硬编码 + 三契约注释（projects.ts:93-99）；T-GAL-3 走通编辑器守卫、T-GAL-6 锁死「绝不拼出原色」、B07 双射由 null∧doc 自动成立；出处只进标题，无归因字段 |
| D-GAL-9 | ✅ | fork 文档不带 `provenance`（createPatternDoc 本就不加，PatternDetailPage.tsx:80-82 注释点名判死理由）；fork-page.test.tsx:55 断言 `undefined` |
| D-GAL-10 | ✅ | 门 = `fixtureGridFor(pattern.id) !== null`，同步读、无加载态（PatternDetailPage.tsx:51-52）；禁用而非隐藏 + 常显理由「这张图纸还没有网格，暂不能 Fork」（:26、:161-171）；T-GAL-5 正反两测 |
| D-GAL-11 | ✅ | Fork 是 DetailActions 第四键（:143-169；fork-page.test.tsx:106-119 断言四键序 + explore 卡内零按钮）；CreatorPage stub note 已删；DEV-GAL-1 照 IA 落详情页 |
| D-GAL-12 | ✅ | 有网格换真预览、无网格保持占位（PatternDetailPage.tsx:101-105）；页面私有组件、非 AssembleCanvas（§1.7）；非交互、`aria-hidden`、色号清单文字替代面在场（T-GAL-8）；56×56 单实例 3136 节点实测（fork-page.test.tsx:195-198） |
| D-GAL-13 | ✅ | `checkIns` 静态、fork 后 catalog 与 checkIns 逐字节不变（T-GAL-7 第二测）；diff 里无评论 / 点赞 / 分享 / 被 fork 计数的任何面 |
| D-GAL-14 | ✅ | ① `GALLERY_NOTE` 逐字命中 IA 新句（EditPage.tsx:43-44），守卫逻辑零改动（:95 判别式原样）；② PatternFeed 头注指向详情页（:5-9）；③ catalog.ts 头注随扩容改写（:4-8）；§5.4 改写走独立 commit `93d6a3e` |

## 3. T-GAL-1…11 落点速查

| ID | 判定 | 落点 |
|---|---|---|
| T-GAL-1 | ✅ | fork.test.ts:34-127 — **全部 9 张**图纸的字面映射表（EXPECTED_MAP）+ 16 条 hex→G 码人读对照表（EXPECTED_CODES）双份钉死；`#RRGGBB` 全量画廊值逐位对拍 + 大小写等价 + 坏值（`#abc` / 中文 / 空串 / `#12345g`）抛错；并列最小下标 twins 构造 + 与 `nearestIndex` 逐图全量对拍；值域 [0, entries.length) 显式断言 |
| T-GAL-2 | ✅ | :130-178 — 保形（宽高 / 空格原位）+ 映射重写；越界下标 throw；两绿（#2f7d55/#4c7a44）并 G27 的构造案例经 `buildBom` 验合并颗数；无合并时 mergedTargets 空；`forkFixture` 对 slime/torii/quilt 三网格颗数守恒 + 空格位掩码全等 |
| T-GAL-3 | ✅ | fork-page.test.tsx:28-70 — 真路由真仓库：doc 落 `patterns` 仓（generic-5mm / 28×28 / 138 颗 / `toPatternDoc` 过 / 无 provenance）→ 项目 draft / null / 「团子三兄弟（Fork）」→ 落 `/edit/:id` 且 `editor-grid` 渲染；第二测断言搬的是原图形状不是空板 |
| T-GAL-4 | ✅ | :73-90 — `savePatternDoc` 注入 reject → `role="alert"` 内联 FORK_SAVE_ERROR、projects 空、path 不变、`editor-grid` 缺席、按钮复活可重试 |
| T-GAL-5 | ✅ | :93-119 — cakebox 禁用 + 理由常显；mochi 可用且理由缺席；DetailActions 四键序 + explore 卡整卡单链接 |
| T-GAL-6 | ✅ | :122-146 — fork → 编辑器替换 G09→G22 → 去拼装：`assemble-grid` 恰 70 颗 rgb(255,212,0)，画廊墨黑 / 樱粉 / 抹茶绿三色绝迹（D-GAL-8 ② 锁死），aria-label 报 G 码 |
| T-GAL-7 | ✅ | :149-180 — 同图两次 fork = 两项目两 doc、id 互异；fork 后 `checkIns` 数组与 `PATTERNS` JSON 串逐字节不变 |
| T-GAL-8 | ✅ | :183-204 — testid 预览、`aria-hidden`、无 button/a/input/tabindex、28² = 784 节点；quilt 56² = 3136 节点单实例；cakebox 无预览保持占位 |
| T-GAL-9 | ✅ | grids.test.ts **零字节改动**，`it.each(GRIDDED_PATTERN_IDS)` 自动收编 8 张网格（逐色颗数 / 总数 / 尺寸 / 下标域 / 零颗色违规）；catalog.test.ts:26-76 补集合层：5 旧 id 原位、4 新 id 全带网格、TAG ≥2、标签集闭合、id 唯一 |
| T-GAL-10 | ✅ | catalog.test.ts:79-89（torii 56×56 / occupiedCount = beadCount / ≠1540）+ grids.test.ts 派生 it.each 自动抓 |
| T-GAL-11 | ✅ | isolation.test.ts 递归扫描自动覆盖 4 个新文件与扩容后 fixture（NE-1 五模式含注释）；catalog.test.ts:99-117 补「无 scheme 的 URL 形字段」盲区（键形扫描 + 序列化串四模式 + Creator 键集闭合） |

§5.4 既有测试改写对账：edit.test.tsx:177-188 **改写不删**（守卫行为断言原样：`editor-grid`
缺席、出路链接在场、不重定向；文案断言换成 `/只读/`+`/Fork/`+`/详情页/` 且 `/WP-B08/` 绝迹），
独立 commit `93d6a3e` ✓；navigation.test.tsx 不在 diff ✓；empty-states / instantiate /
assemble / `algo/` 测试零改动 ✓；inventory.test.ts 见 §5.2。

## 4. 门禁实测（本审查独立工作树）

```text
pnpm --filter @bead/app test              → Test Files 39 passed (39) / Tests 608 passed (608)   ← 父代理门禁 39/608 命中
pnpm --filter @bead/app lint              → tsc --noEmit && eslint . 零输出，exit 0
cargo run -q -p xtask -- e0-audit         → 14 crates / 325 files，clean
cargo run -q -p xtask -- denylist-audit   → 94 terms / 129 files，clean
git diff 51dd0cf 0c1f54d -- apps/bead     → 空（吸收逐字节忠实）
```

hosted Bead CI 照旧是 billing 空 runner（作业 ~3s、`steps: []`），与 B03–B07 吸收同症，
**非产品失败**；本机命令仍是门，上表即门。

## 5. 实现者旗标核验（不当福音，逐行验过）

1. **「hex 解析复用 stores/inventory.ts 的 parseHexColor」→ 真，正当，确认。**
   基线 `2dcd1bb` 该函数已导出（inventory.ts:107，D-INV-10 自称「the one hex→Rgb bridge」）、
   严格六位、拒 `#abc`（fork.test.ts:96-98 钉死）；`stores/inventory.ts` 本体零字节差
   （§7 零改动清单守住）；import 方向 pages→stores 全库既有。这与 D-GAL-6 字面
   「hex→Rgb 解析是 fork 模块本地纯函数」**偏离**——但 IA 给「本地函数」的两条理由
   （不动 `algo/`、不碰 palette.ts 的私有 helper）在复用下同样成立且更强：仓里
   `#RRGGBB` 读法保持单数，fork.ts:44-45 把理由写在了脸上。判优，不要求改。
2. **「inventory.test.ts torii 字面量更新」→ 真，必要，账记 IA。** DEV-GAL-3（IA 获准的
   显式偏离）把 torii 颗数改为网格派生值，而 T-INV-2 恰用 torii 字面量做聚合断言
   （inventory.test.ts:115-125：`108+300→108+266`、`520→790`）——IA §5.4 的「inventory
   原样保绿」漏算了这一格。改动带注释点名 DEV-GAL-3 与「聚合规则一字未动」、与扩容同
   commit（`aaf7e3b`，即引发它的那一刀）。这是 IA 映射表的漏项，非实现偏离。确认。
3. **「预览复用 .upload__grid / .assemble__bead，不动 app.css」→ 真，确认。** app.css
   零字节差；两 class 皆既有（`.upload__grid` @ app.css:476，吃 `--assemble-columns`
   变量，PatternPreview.tsx:24 内联供值；`.assemble__bead` @ :760）；预览不设
   `data-cell-state`，拼装三态选择器不点火。app.css 本就不在 IA §7 允许改面清单——
   不改它同时满足 §7 与 D-GAL-12 的「复用渲染技术」。容器选 `.upload__grid` 而非
   `.assemble__grid` 是 IA 未指定的自由度（8px 细胞 + overflow 更合预览），合规。

## 6. 缺陷清单

### LOW

- **LOW-1 · grids.test.ts:68 用例标题散文过时** — 「没有网格的图纸返回 null，**立体拼豆
  图纸永不上网格**」的后半句被本轮的 `gal-gift-06`（立体拼豆 tag + 有网格）证伪；断言
  本体只钉 cakebox 与未知 id，仍然成立、仍然该在。一行改名（如「gal-cakebox-03 永不上
  网格」）即可，随任一后续 PR 捎带。
- **LOW-2 · PatternPreview.tsx:33 不可达的灰色回退** — `palette[cell]?.hex ?? "#808080"`：
  越界下标在 `decodeFixtureRows`（grids.ts:59-61）已是 throw，永远到不了这里；而「静默
  灰豆」恰是解码器明文要防的「悄悄留白」的近亲——真出 bug 时它会把 fixture 病灶画成
  一颗无人追问的灰豆。收紧为直接索引（让它炸）即可。一行的事。

### 记录（非缺陷）

1. `forkTitle` 按 UTF-16 code unit 切（fork.ts:97），理论上可切断增补平面字符——与
   UploadWorkbench.tsx:241 / BlankBoardForm.tsx:47 同款技术，一致性优先；画廊标题全是
   BMP 短串，触发面为零。记录不改。
2. catalog.ts:5-6 新头注宣称 B08 是「最后一个扩容的工作包」——IA 没说这话，是实现自加
   的承诺散文；append-only 已被 catalog.test.ts 钉成机器可查，散文无害。
3. T-GAL-1 的 EXPECTED_MAP 按 IA 授权「实现算一次、人工过目后写成字面量」产出；其对
   `nearestIndex` 的忠实由同文件第四测全量对拍保证，drift 检测目的达成。G 码对照表
   （G01 白 / G09 墨黑 / G15 朱红 / G27 双绿合并）经人读抽查合理。

## 7. 裁决

**ACCEPT-WITH-NITS。0 HIGH / 0 MED / 2 LOW。** 七条硬红线零违例；fork 是纯消费者，
仓库 / 编辑器 / 拼装 / 库存 / B07 五个契约面零增量兑现（`store.tsx`、`stores/*.ts` 源码、
`schema/**`、`algo/**` 全部零字节差）；量化、序列、可用性门、预览、文案四处收尾逐条照
D-GAL 落地；T-GAL 矩阵每行有真测试落点且既有测试改写照 §5.4 走独立 commit；父门禁
39/608 + lint 独立复现命中，e0 / denylist 双审计清洁，吸收对实现 tip 逐字节忠实。
唯一的字面偏离（parseHexColor 复用）方向是更少的解析器，判优；两条 LOW 是一行级打磨，
无一阻塞——`0c1f54d` 可以留在 exclusive tip 上。不重开架构。

## 8. 禁区自查

本轮只新增 `docs/bead/reviews/round3-gallery-review.md`。未改 `apps/bead` 任何源码与测试
（LOW 两条留给后续修缮席捎带），未触碰 `docs/PRODUCT_LOCK.md`、`docs/STATUS.md`、
`apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml`、`deny.toml`、`.github/**`、
`pnpm-lock.yaml`。子代理 `gh` 只读未破例；**BLOCKED_PR**：不建 PR，推分支即止。
本文无外网 URL，仅引用仓内路径（BD15/BD17）。
