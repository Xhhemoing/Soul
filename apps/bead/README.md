# @bead/app — BeadFlow 应用壳（WP-B02）

Vite + React + TypeScript + Vitest。这是 BeadFlow 的壳：路由、导航、主题 token、
store 骨架与空状态。转图、拼装指引、库存、导入导出都还没实现，各自归后续工作包。

## 命令

```bash
pnpm --filter @bead/app dev     # 本地开发，127.0.0.1:1430
pnpm --filter @bead/app test    # vitest（导航 + 空状态 + id 前缀 + 实例化）
pnpm --filter @bead/app lint    # tsc --noEmit && eslint
pnpm --filter @bead/app build   # 类型检查 + 产物
```

根 `package.json` 的脚本仍然指向 `@soul/desktop`，本包不改它；bead 的门禁归 WP-B10。

## 结构

```
src/
  app/         App / 路由表 / AppShell / BottomNav / 主题开关 / tokens.css
  pages/       explore workspace create inventory pattern creator assemble not-found
  components/  EmptyState ColorSwatch ProgressBar Card BackHeader
  stores/      ids types repository catalog projects inventory store theme
  fixtures/    catalog.ts（5 张图纸、2 位创作者）
  algo/        NOT_IMPLEMENTED 占位（色彩算法归 WP-B01 / WP-B03）
  schema/      WP-B07 预留
```

## 两处结构约束

**双 chrome（D-UI-2）。** 七条常规路由挂在 `AppShell` 布局路由下（头部 + 底栏）；
`/assemble/:id` 在 shell 之外渲染，没有底栏，自带常显退出按钮与 `Escape` 通道。
路由表在 `src/app/routes.tsx`，这个分叉一眼可见。

**id 前缀（D-UI-4 / R1）。** 画廊图纸 `gal-*`、本地项目 `proj-*`、创作者 `cr-*`。
`/pattern/:id` 只收图纸 id，`/assemble/:id` 只收项目 id；图纸要先经详情页「加入待拼」
或「转入工作台」实例化成项目，才能进拼装页。前缀在 `src/stores/ids.ts` 里有类型与
守卫，并有测试钉住。

主题（`data-theme` 上的 light/dark token）与拼装背景（逐项目字段，黑/白/自定义）
是两套系统，互不读取（D-UI-6），有测试保证。

## 边界

- 不 import `@soul/desktop`、Tauri API 或任何 `soul-*` crate。eslint 与
  `src/test/isolation.test.ts` 双重把关。
- 不实现 CIEDE2000 等色彩算法：`src/algo/index.ts` 只有 `NOT_IMPLEMENTED`。
- `/create` 的未实现入口在页面上写明归属工作包（B03 / B06 / B07），别当 bug 报。
