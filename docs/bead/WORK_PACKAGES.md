# BeadFlow 工作包

实现只允许落在隔离树：`apps/bead/**`、`crates/bead-core/**`。  
`crates/bead-core` **不得**加入根 `Cargo.toml` `[workspace].members`（以免 Soul `deny.toml` / e0-audit / `just ci` 被拖进无关依赖）。  
`apps/bead` 会被 `pnpm-workspace.yaml` 的 `apps/*` 扫到；根脚本仍 `--filter @soul/desktop`，不要改根 `package.json` 的 lint/test/build 指向。

实现模型：`claude-opus-5-thinking-high-fast`。  
分析 / Review：`claude-fable-5-thinking-xhigh`。  
禁止静默降级；回复第一行自报实际 slug。

## WP-B01 — bead-core（算法源）

独立 Cargo 包：`crates/bead-core/Cargo.toml`，`[workspace]` 只有自己。  
共存硬约束（BD15）：Cargo.toml 不要写 `repository`/`homepage` URL；Rust 源里置信度叫 `confidence`，禁止出现 `score`/`得分`/`分数`/`评分`。

必须实现并单测：

1. **CIEDE2000**（Sharma 2005 / CIE 142）：RGB sRGB→Lab D65，再算 ΔE00。用公开样例钉至少 5 组已知值。
2. **色板映射**：输入像素 RGB，输出最近色号；品牌色板以 fixture 提供（先做一份文档化的 5mm 熔豆通用板，命名空间 `generic-5mm`）。
3. **Floyd–Steinberg**：可开关；误差按 7/16、3/16、5/16、1/16 扩散；超出色板时仍量化到最近色。
4. **像素图判定**：规则启发式即可（网格周期性 / 低独特色数），返回 `PixelArt | Photo` 与置信度；不要接外部视觉 API。
5. **切分**：固定板（28×28、56×56）、按图比例适配、固定视口手动缩放（输入 scale + crop）。
6. **四模式步骤拆分**（输入：`Grid<ColorId>`）：
   - Color-by-Color：默认点缀色先行（计数升序），可切计数降序。
   - Tile：切 28×28，左到右、上到下。
   - Outline→Infill：连通分量，外轮廓 → 内边界 → 填充。
   - Row-by-Row：自上而下、自左而右。
7. **BOM**：色号、显示名、颗数。
8. **替代色**：库存缺色时，在库存内找 ΔE00&lt;3 的候选，按 ΔE 升序。

完成标准：`cargo test` 在该包内绿；有 README 说明如何运行。  
验收钉死：`docs/bead/reviews/round1-algorithms.md`（G1–G8 与 T-* 测试号）。Sharma 数值以 `docs/bead/fixtures-ciede2000.md` 为准；文中 #34 若夹具未列，标 TODO，禁止编造。

## WP-B02 — 应用壳

`apps/bead`：Vite + React + TypeScript + Vitest。包名 `@bead/app`。

- 底栏或等价导航：灵感 / 拼装台 / + / 资产。
- 路由：`/explore` `/workspace` `/create` `/inventory` `/pattern/:id` `/assemble/:id` `/creator/:id`。
- 个人条（Explore 顶）：进度摘要、待办数、收藏数（先读本地 store）。
- 黑/白主题 token；后续组装页可全屏沉浸。
- 不要引用 `@soul/desktop` 或任何 `soul-*` crate。

完成标准：`pnpm --filter @bead/app test` 绿；导航与空状态有测试。  
共存硬约束（BD15/BD16）：清单与源码不要写外网 URL（含 JSON `$schema`）；落地提交必须重生成根 `pnpm-lock.yaml`，并给 `.gitignore` 加 `apps/bead/dist/`。

## WP-B03 — 图像转豆图（浏览器管线）

落点已拍板（BD14）：只写 `apps/bead/src/algo/`。不要建 `packages/bead-algo`。等 WP-B02 壳合入后再开工。  
在该目录用 TypeScript 复刻 B01 的公开契约（同一 fixture 必须得到同一色号序列，比对色号而非裸 ΔE 浮点）。  
v0 不强制 wasm；Rust 包是算法 oracle。  
支持上传 png/jpg、像素图/正常图切换、三种框定、抖动开关、板型预设。

## WP-B04 — 沉浸式拼装台

四种模式可视化：非当前步 15% 透明度或灰度；当前色/格高亮。  
悬浮控制球：下一步、撤销、锁定当前行。  
计时、BPM、25% 里程碑与「当前色号完成」。  
透光 1:1 模式：按 CSS mm 近似（注明显示器未校准）。  
语音/手势不做。

## WP-B05 — 库存与清单

本地库存 CRUD；BOM 对比；缺色预警；ΔE&lt;3 替代建议；导出采购文本。

## WP-B06 — 像素编辑器

对称、油漆桶、拾色器、色号替换、颜色统计。可后于 B03。

## WP-B07 — 导入导出

`.beadproj` JSON（schema 进 `apps/bead/src/schema`）；png/jpg 栅格；`.pat` / `.gamedev` 能识别则解析，否则明确 `UNSUPPORTED_FORMAT`。

## WP-B08 — Explore fixture 画廊

本地推荐流、标签、详情页（难度、颗粒、耗时、板型、配色）、Fork 改色。无真实社交后端。

## WP-B09 — 本地存储

IndexedDB（或等价）保存项目、库存、进度。无账号体系则单机档案。禁止把图纸默认传到外网。

## WP-B10 — 测试 / 构建 / 隔离 CI

为 bead 增加**独立** workflow 或 job，**不要**改现有 Soul `ci.yml` 的 push 过滤，使其变成 bead 门禁。  
根 `just ci` / Soul e0-audit 必须保持原行为。

## 禁止

- 改 `docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md` 的 Soul Goal 1 事实。
- 改 `apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml` members、`deny.toml`。
- 把本线合并进 `cursor/soul-goal1-7b1c`（除非用户另令）。
- 为凑 Commit / PR 做无意义重命名。
