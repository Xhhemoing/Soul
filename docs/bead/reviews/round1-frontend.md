# BeadFlow 前端 IA — Round 1

Review 模型：`claude-fable-5-thinking-xhigh`（实际运行时 Claude Fable 5 thinking，无静默降级）。
角色：前端 IA / Review，不写业务实现。
输入：`docs/bead/PLAN.md`、`docs/bead/WORK_PACKAGES.md`、`apps/desktop/package.json`（只读，取技术栈惯例）。
现状：`apps/bead` 与 `crates/bead-core` 尚不存在，本文是**前置 IA**——约束 WP-B02 的落地形状，而非评审已有代码。
硬禁区已守：本轮只新增本文件，不触碰 Soul 锁、`apps/desktop`、`crates/soul-*`、根 Cargo workspace、`deny.toml`、`STATUS.md`。

---

## 1. 决策摘要（编号供后续 Review 引用）

- **D-UI-1 路由即状态。** 可分享、可后退的状态（当前 tab、图纸 id、标签筛选）一律进 URL，不进 React state。
- **D-UI-2 双 chrome 模型。** `AppShell`（头部槽 + 内容 + 底栏）承载 6 条常规路由；`/assemble/:id` 在 shell **之外**渲染，无底栏，自带退出。这个结构分叉现在做很便宜，B04 再改很贵。
- **D-UI-3 「+」是完整路由 `/create`，不是浮层。** 完整路由可测、可后退、可直达；若后续产品想要 sheet 形态，只改 chrome 不改路由。
- **D-UI-4 id 命名空间前缀。** 画廊图纸 `gal-*`、本地项目 `proj-*`、创作者 `cr-*`。`/pattern/:id` 只收图纸 id，`/assemble/:id` 只收项目 id。图纸必须先实例化为项目（「加入待拼」/「转入工作台」）才能拼装。
- **D-UI-5 Repository 接口从第一天就是 async。** B02 用 localStorage 实现，但接口签名按 IndexedDB（B09）设计，避免 B02→B09 全站翻改。
- **D-UI-6 拼装背景 ≠ 应用主题。** 黑/白/自定义拼装背景是逐项目的画布底色，存进项目记录；应用主题 token（黑/白）是全局 `data-theme`。两套系统禁止互相泄漏。
- **D-UI-7 依赖最小化。** 新增运行时依赖只有 `react-router`（v7 库模式，测试用 memory router）。不引 UI kit、不引 CSS 框架、不引状态库——context + useReducer + repository 足够 v0 体量。与 `@soul/desktop` 的零冗余依赖哲学一致，但**不 import 它任何代码**。
- **D-UI-8 空状态承担新手引导。** 不做独立 onboarding 流程；每个零态给一句解释 + 一个主 CTA。

## 2. 路由表

| 路由 | 页面 | 底栏高亮 | Chrome | `:id` 语义 | 备注 |
|---|---|---|---|---|---|
| `/` | redirect | — | — | — | 302 → `/explore` |
| `/explore` | ExplorePage | 灵感 | AppShell | — | 个人条 + 发现流；`?tag=` 与 `?fav=1` 筛选 |
| `/workspace` | WorkspacePage | 拼装台 | AppShell | — | 正在拼置顶 / 草稿 / 历史完工 三段 |
| `/create` | CreatePage | + | AppShell | — | 五入口：上传转图 / 从画廊选 / 空白项目 / 导入豆图 / 导入项目库 |
| `/inventory` | InventoryPage | 资产 | AppShell | — | 库存 / 缺口 / 替代 三面板 |
| `/pattern/:id` | PatternDetailPage | 无（返回头） | AppShell | `gal-*` | 难度、颗数、耗时、板型、配色清单 + 双 CTA |
| `/creator/:id` | CreatorPage | 无（返回头） | AppShell | `cr-*` | 作品集网格 + 被拼打卡数 |
| `/assemble/:id` | AssemblePage | — | **沉浸 chrome** | `proj-*` | 全屏；shell 外渲染 |
| `*` | NotFoundPage | 无 | AppShell | — | 一句话 + 回 `/explore` |

路由集与 WP-B02 清单逐条对齐；`/` redirect 与 `*` 兜底是 IA 必需的补充，不是新业务面。

## 3. 空状态清单（B02 验收要求"导航与空状态有测试"，逐条列出即测试用例清单）

| 位置 | 零态条件 | 呈现 | 主 CTA |
|---|---|---|---|
| 个人条·进度卡 | 无 active 项目 | 「还没有正在拼的项目」 | → `/create` |
| 个人条·待办/收藏 | 计数为 0 | 显示 `0`，**不隐藏**（布局稳定） | 待办 → `/workspace`；收藏 → `?fav=1` |
| Explore 发现流 | 标签/收藏筛选后为空 | 「没有匹配的图纸」 | 清除筛选 |
| Explore 发现流 | fixture 集为空（防御） | 同上文案 | → `/create` |
| Workspace 三段 | 各自为空 | 每段一句解释 | 正在拼段 → `/explore` 与 `/create` 双入口 |
| Inventory | 无库存记录 | 「录入库存后才能做缺口预警」 | 录入库存（B05 前为占位表单入口） |
| `/pattern/:id` 未知 id | catalog 查不到 | 「图纸不存在或已下架」 | ← 返回 `/explore` |
| `/creator/:id` 未知 id | 同上 | 「创作者不存在」 | ← 返回 |
| `/assemble/:id` 未知/非项目 id | store 查不到 | 守卫屏（不自动跳转，保住后退键） | 退出 → `/workspace` |
| `/create` | 永不为空（静态入口） | — | 导入失败态（`UNSUPPORTED_FORMAT`）归 B07 |

## 4. 组件树

```
main.tsx
└─ <App>  (ThemeProvider → StoreProvider → RouterProvider)
   ├─ <AppShell>                      布局路由：<header slot> + <Outlet/> + <BottomNav/>
   │   ├─ ExplorePage
   │   │    ├─ <PersonalStrip>
   │   │    │    ├─ <ProgressSummaryCard>   点按 → /assemble/:activeId
   │   │    │    ├─ <TodoCountChip>         点按 → /workspace
   │   │    │    └─ <FavoritesChip>         点按 → /explore?fav=1
   │   │    ├─ <TagFilterRow>               二次元 / 像素游戏 / 立体拼豆 / 节日限定
   │   │    └─ <PatternFeed>
   │   │         └─ <PatternCard>*          整卡为单一链接 → /pattern/:id
   │   ├─ WorkspacePage
   │   │    ├─ <ActiveProjectHero>          进度条 + 用时 + 「继续拼豆」→ /assemble/:id
   │   │    ├─ <ProjectSection kind="draft"> → <ProjectCard>*
   │   │    └─ <ProjectSection kind="done">  → <ProjectCard>*（用时、成品预览）
   │   ├─ CreatePage
   │   │    └─ <CreateEntryList> → <CreateEntry>*   （B03/B07 之前，转图与导入入口指向 stub 说明页）
   │   ├─ InventoryPage
   │   │    ├─ <StockList> → <StockRow>*   （色号 + 颗数）
   │   │    ├─ <ShortagePanel>             （B05 填充：BOM 对比缺口）
   │   │    └─ <SubstitutePanel>           （B05 填充：ΔE00<3 替代）
   │   ├─ PatternDetailPage
   │   │    ├─ <BackHeader>
   │   │    ├─ <PatternPreview>
   │   │    ├─ <PatternMeta>               难度 / 颗粒总数 / 预估耗时 / 板型
   │   │    ├─ <PaletteList> → <ColorSwatch code=…>*
   │   │    └─ <DetailActions>             「加入待拼」（建项目留在原地）｜「转入工作台」（建项目并跳 /workspace）
   │   ├─ CreatorPage
   │   │    ├─ <BackHeader>
   │   │    ├─ <CreatorHeader>             名称 + 被拼打卡数
   │   │    └─ <PatternFeed variant="grid">（Fork 按钮随 B08 上，B02 不出死按钮）
   │   └─ NotFoundPage
   └─ AssemblePage                          shell 外；层级从底到顶：
        ├─ <AssembleBackdrop>               黑/白/自定义，读项目记录（D-UI-6）
        ├─ <AssembleCanvas>                 B02 仅静态网格占位；四模式渲染归 B04
        ├─ <SessionHUD>                     计时/进度/里程碑（B04）
        ├─ <FloatingControlBall>            下一步/撤销/锁行（B04）
        └─ <ExitButton>                     常显、可 Esc 触发 → 返回 /workspace
```

共享原语：`<EmptyState>`、`<ColorSwatch>`（色块 + 必显色号文本）、`<ProgressBar>`、`<Card>`。

建议目录（IA 级约定，实现可微调）：

```
apps/bead/src/
  app/        App.tsx  router.tsx  AppShell.tsx  BottomNav.tsx  tokens.css
  pages/      explore/ workspace/ create/ inventory/ pattern/ creator/ assemble/ not-found/
  components/ EmptyState/ ColorSwatch/ ProgressBar/ Card/
  stores/     repository.ts  catalog.ts  projects.ts  inventory.ts  theme.ts
  fixtures/   catalog.json（3–5 张图纸、2 位创作者，够撑 detail/creator 路由）
  schema/     （B07 预留，B02 建空目录即可）
```

## 5. 状态所有权

| 状态 | 拥有者 | 持久化 | 写者 | 读者 |
|---|---|---|---|---|
| tab / 详情 id / `?tag` / `?fav` | URL（router） | URL 本身 | Link / navigate | 各页面 |
| 主题 light/dark | ThemeStore | localStorage `bead.theme`；`data-theme` 挂 `<html>` | 主题开关（shell 头部） | 全部 token 消费者 |
| 画廊图纸/创作者/标签 | CatalogStore | 构建期 fixture，**只读** | 无（B08 只扩 fixture） | Explore / Pattern / Creator |
| 项目（草稿/进行/完工）、进度、用时、收藏、待办 | ProjectStore | Repository（B02: localStorage；B09: IndexedDB，同接口） | Detail 双 CTA、Assemble 会话提交、Workspace 操作 | PersonalStrip / Workspace / Assemble |
| 库存 | InventoryStore | Repository 同上 | Inventory CRUD（B05） | Inventory / ShortagePanel |
| 拼装会话（当前步、计时器运行态） | AssemblePage 页内 state | 不直接持久化；里程碑与退出时写回 ProjectStore（节奏由 B04 定） | B04 | 页内 |
| 拼装背景 | 项目记录字段 | 随 ProjectStore | 背景选择器 | AssembleBackdrop |

规则：页面不直接碰 localStorage，只调 store action；个人条计数、缺口等派生数据一律 selector 现算，禁止落库；`ProjectRepository` / `InventoryRepository` 接口全 async（D-UI-5）。

## 6. a11y / 主题

- 底栏：`<nav aria-label="主导航">`，四链接文字常显（灵感/拼装台/+/资产），激活项 `aria-current="page"`，触达面 ≥ 44×44px。「+」也是链接，不是无名图标按钮。
- **颜色永不作为唯一信号**：所有色块（配色清单、库存、替代建议）必须并排显示色号文本。拼豆用户里色弱比例不可忽略，ΔE 替代色对比尤其如此。
- 沉浸页：退出按钮常显 + `Escape` 快捷键双通道；它是页面不是 modal，不做焦点陷阱，但保留标题层级；里程碑动效尊重 `prefers-reduced-motion`（B04）。
- 主题 token 挂 `[data-theme]` 的 CSS custom properties：`--bg-canvas` `--bg-surface` `--text-primary` `--text-secondary` `--accent` `--border` `--focus-ring` `--swatch-outline`。浅色色块在白底需要 `--swatch-outline` 描边。文本对比 ≥ 4.5:1（AA），两套主题都要过。
- 焦点：全站 `:focus-visible` 用 `--focus-ring`；键盘顺序 = 视觉顺序；PatternCard 整卡单链接，卡内不嵌第二个可点目标（Fork 到 B08 时以卡外 action 排布解决）。
- `<html lang="zh-CN">`；每路由独立 `<title>`（测试可断言）。
- 底栏形态在桌面宽度照用（v0 减脂）；>900px 换侧栏属后续打磨，见风险 R4。

## 7. WP-B02 必须落地 vs 后置

**必须落地（B02 验收线）：**

1. 全部 7 条路由 + `/` redirect + `*` 兜底；底栏激活态；memory router 下的导航测试。
2. AppShell / 沉浸 chrome 的结构分叉（D-UI-2）——即使 Assemble 只有占位网格 + 背景切换 + 退出。
3. 第 3 节空状态清单全量实现并测试（这就是验收里"空状态有测试"的具体化）。
4. 个人条三格读本地 store，零态正确、布局稳定。
5. 主题 token 两套 + 切换 + 持久化。
6. Store 骨架：async repository 接口 + localStorage 实现 + catalog fixture（3–5 图纸、2 创作者），让 `/pattern/:id`、`/creator/:id` 渲染真实内容而非 lorem。
7. **最小项目实例化**：Detail 双 CTA 能创建项目记录（id、名称、来源图纸、状态），否则 Workspace 永远空、`/assemble/:id` 永远不可达，导航测试成了自欺。进度逻辑不在此列。
8. `?tag=` / `?fav=1` 客户端筛选——量小、成本低，且是"筛选后空态"测试的前置。

**后置（明确不属于 B02，实现者不得顺手做）：**

| 内容 | 归属 |
|---|---|
| 四种步骤模式渲染、悬浮控制球、计时/BPM/里程碑 | WP-B04 |
| 上传转图管线（`/create` 的上传入口先指向 stub 说明） | WP-B03 |
| 库存 CRUD、缺口预警、替代建议、采购导出 | WP-B05 |
| 像素编辑器、Fork 改色 | WP-B06 |
| `.beadproj` / `.pat` / `.gamedev` 导入导出与失败态 | WP-B07 |
| Fork 按钮与画廊扩容 | WP-B08 |
| IndexedDB 迁移（接口已预留） | WP-B09 |

## 8. NO_HIGH_VALUE_CHANGE_FOUND

以下子树本轮判定**不值得发明**，列出以免后续轮次重复论证：

- 设置/个人资料页：无账号体系，主题开关放 shell 头部即可。NO_HIGH_VALUE_CHANGE_FOUND。
- 通知 / 消息 IA：无社交后端。NO_HIGH_VALUE_CHANGE_FOUND。
- 搜索路由：fixture 规模 < 50，标签筛选已覆盖发现需求。NO_HIGH_VALUE_CHANGE_FOUND。
- 独立 onboarding / 教程流：空状态承担引导（D-UI-8）。NO_HIGH_VALUE_CHANGE_FOUND。
- i18n 抽取层：v0 中文字面量直写，抽 i18n 属投机性架构。NO_HIGH_VALUE_CHANGE_FOUND。
- 分享 / deep-link 出站：本机档案、禁默认外传（B09 约束），无可分享目标。NO_HIGH_VALUE_CHANGE_FOUND。

## 9. 残余 UI 风险

- **R1 · id 命名空间漂移**：若 B03/B08 未按 `gal-*`/`proj-*` 前缀落库，`/pattern/:id` 与 `/assemble/:id` 深链会互串。缓解：前缀写进 B02 的 store 类型与测试。
- **R2 · 1:1 物理尺寸**：B04 的透光 1:1 依赖 CSS mm 近似且显示器未校准（PLAN 已注明）；真实尺寸模式可能需要 Assemble 的第二布局态，本 IA 只预留 backdrop/canvas 分层，未解。
- **R3 · localStorage 容量**：项目含逐格进度时 5MB 上限可能吃紧；D-UI-5 的 async 接口让 B09 换 IndexedDB 无 API 破坏，但 B02 期间大图纸可能先撞墙——fixture 控制在小图即可。
- **R4 · 桌面宽度下的底栏**：v0 全宽度沿用底栏是刻意减脂，宽屏体验欠佳属已知债。
- **R5 · router 依赖**：react-router 大版本频动，锁 major、测试走 memory router，降低升级面。
- **R6 · `/create` 的 stub 入口**：B03/B07 落地前，上传与导入入口是"说明页"而非功能；需在 stub 上写明归属工作包，避免测试者当 bug 报。
