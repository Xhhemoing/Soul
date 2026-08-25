# ROUND 1 · F1 全库地图与 WP 精化

Reviewer：`claude-fable-5-thinking-xhigh`（分析/Review，不写产品代码）。  
基线：`origin/cursor/beadflow-integration-c441` @ `75e3c70`。扫描对象：全部 415 个已跟踪文件。  
本文件只属于 BeadFlow 线（`docs/bead/**`），不改写 Soul 任何事实。

## 1. 仓库现状一图

```text
/                       Soul 单仓（bead 目前只有文档，没有一行代码）
├─ Cargo.toml           Soul 根 workspace：16 个成员（15 个 soul-* / soulcore + xtask）。bead-core 禁止加入（BD3）
├─ apps/desktop         Soul 桌面壳：React 19 + Vite 8 + Vitest 4；12 屏；src/core.ts 是唯一 IPC 出口
│  └─ src-tauri         独立 cargo workspace（先例：嵌套 workspace 不进根门禁，bead-core 照抄这个模式）
├─ crates/soul-*        数据面/权限面/业务面 15 crate；soulcore 汇成 36 条 IPC 命令
├─ crates/xtask         Soul 门禁工具：e0-audit / denylist-audit / schema-freeze / sbom（对 bead 有跨界影响，见 §2）
├─ fixtures/            Soul 测试语料；`fixtures` 目录名在两个 audit 里都是豁免词
├─ scripts/             Win11 冒烟 / 首测脚本 / 分支处置
├─ .github/workflows/ci.yml   Soul 五门；push 只触发 main 与 cursor/soul-goal1-7b1c —— bead 分支不会触发它
├─ justfile             `just ci` = lint/schema/e0/denylist/fixtures/test/smoke-lint/sbom/ui-lint/ui-test
├─ pnpm-workspace.yaml  只有 `apps/*` 一个 glob —— apps/bead 一落地就会被 pnpm 扫进来
└─ docs/bead/           本线全部现状：PLAN / WORK_PACKAGES / README / fixtures-ciede2000
```

bead 侧现状：`apps/bead`、`crates/bead-core`、`packages/`、`docs/bead/reviews/` 均不存在。O1–O3 未落地（等 VM 空位）。

## 2. 共存硬约束（WP 里没写、实现前必须知道的）

这是本次全库扫描最有价值的产出：Soul 的三个门禁会**跨界扫到 bead 的树**，WP 文本没有提示。

### 2.1 `xtask e0-audit` 会扫 `apps/bead/**` 与 `crates/bead-core/**` 的 URL 字面量

`crates/xtask/src/egress.rs::audit` 对 `crates`、`apps`、`scripts` 三棵树做 URL 扫描，
扩展名覆盖 `rs/ts/tsx/js/jsx/json/html/css/toml/conf/ps1/sh/cmd/bat`；
允许名单只有回环地址与 `https://soul.local/schemas/`。任何一个外网 URL 字面量都会把 Soul 的 `just ci` 打红。
对 bead 的具体含义：

- `apps/bead` 的 `tsconfig.json` / `package.json` **不要写 `$schema`**（schemastore 的 URL 直接命中）；
- `crates/bead-core/Cargo.toml` **不要写 `repository` / `homepage` / `documentation`**；
- 与 `src` 并置的 `*.test.ts(x)` **会被扫**（豁免的是目录名字面为 `tests`/`fixtures` 的树）；
- `.beadproj` 的 JSON Schema（WP-B07，落 `apps/bead/src/schema`）**不要写 `$id`/`$schema` URL**；
- 引用出处、论文链接一律写进 `.md`（`.md` 不在扫描扩展名里）或 `fixtures/` 目录下；
- `Cargo.lock`、`pnpm-lock.yaml` 不被扫（`.lock`/`.yaml` 不在扩展名里），锁文件里的 registry URL 无碍。

### 2.2 `xtask denylist-audit` 会扫 `crates/bead-core/src/**/*.rs`

`crates/xtask/src/denylist.rs` 扫 `crates/**` 全部非豁免 `.rs` 的字符串字面量与标识符（`tests/` 目录豁免，TS 不扫）。
`fixtures/denylist/diagnostic_terms.txt` 里有 **`score`、`量表`、`得分`、`分数`、`评分`、`计分`**（CJK 按子串匹配）。
对 bead-core 的具体含义：

- 像素图判定的置信度命名用 **`confidence`**，不能叫 `score` / `match_score` / `PixelArtScore`（驼峰会拆词命中）；
- `.rs` 里的中文字符串别含「分数/得分/评分」（「百分数」也会按子串命中「分数」）；
- 想引用这些词的测试放 `crates/bead-core/tests/`（豁免），不要放 `src/` 内联测试。

### 2.3 `apps/bead` 落地那一个提交必须同时重生成 `pnpm-lock.yaml`

`pnpm-workspace.yaml` 的 `apps/*` 会自动把 `apps/bead` 收进 pnpm workspace，而 `just ci` 的
`ui-install` 是 `pnpm install --frozen-lockfile`：新 `package.json` 不进锁文件，Soul 在本线任何分支上的
`just ci` 直接红。这不违反 BD4（根 `package.json` 的 scripts 一个不改），锁文件与 `.gitignore` 都不在禁改名单。
WP-B02 的实现提交必须包含：

1. 重生成后的 `pnpm-lock.yaml`（pnpm 已由 `packageManager` 钉在 10.15.0）；
2. `.gitignore` 增加 `apps/bead/dist/`（现有条目只豁免 `apps/desktop/dist/`；`node_modules/`、`target/` 已通用）。

### 2.4 其余跨界事实（低风险，逐条确认过）

- **根 cargo 门禁看不见 bead-core**：`fmt --all`、`clippy --workspace`、`test --workspace`、`cargo deny`、`sbom`、
  `schema-freeze` 都以根 workspace 为界；`crates/bead-core` 自带 `[workspace]` 即隔离（先例 `apps/desktop/src-tauri`）。
  代价是 bead-core 的 fmt/clippy/test 没有任何现有门禁替它跑 → 必须由 WP-B10 自建（§4）。
- **工具链钉死**：根 `rust-toolchain.toml` 钉 1.83、`rustfmt.toml`（max_width 100）对仓内所有 cargo/rustfmt 调用生效。
  bead-core 必须在 **Rust 1.83 / edition 2021** 上编译，选依赖时先看 MSRV（建议见 §4 B01）。
- **端口**：Soul 桌面 dev server 钉死 `127.0.0.1:1420 strictPort`（`apps/desktop/vite.config.ts`）。
  bead 的 vite 配置应显式钉另一个端口（建议 1520），两个 dev server 才能并存。
- **Soul hosted CI 与 bead 无关**：`ci.yml` push 只触发 `main` 与 `cursor/soul-goal1-7b1c`，且 hosted runner
  因 Billing 全线空跑（STATUS 已记）。bead 分支不会误触发 Soul 五门；反过来 bead 也拿不到 hosted 绿（§3 CI/CD 面）。
- **命名互不干扰**：`@bead/app` 不会被根脚本的 `--filter @soul/desktop` 选中；Soul 侧无任何代码引用 `apps/*` 通配。

### 2.5 一处需要父代理拍板的自相矛盾

`docs/agent-progress.md` ROUND 1 派单表把 O3 写成「唯一实现 `packages/bead-algo`」，但 WP-B03 与同页第 71 行
仍写「O3 只碰 `apps/bead` 的转换模块」。若真落 `packages/bead-algo`，`pnpm-workspace.yaml` 必须加 `packages/*`
glob（目前只有 `apps/*`），锁文件再动一次。**建议 v0 收敛为 `apps/bead/src/algo/`**：少改一处根配置、
一份 vitest 配置管全部、TS 侧不需要被第二个包复用；等 wasm 或复用需求出现再拆包。此裁决应记进
`docs/agent-decisions.md`（BD14），避免 O2/O3 各写各的。

## 3. 十五面模块地图（BeadFlow + Soul 共存视角）

| 面 | Soul 现状（核实过） | bead 现状 | 缺口 / 结论 |
|---|---|---|---|
| 前端 | `apps/desktop`：React 19 + Vite 8 + Vitest 4，12 屏，`core.ts` 唯一 IPC 出口，eslint 强制 | 无代码 | WP-B02；精化见 §4（端口、锁文件、$schema、egress 断言） |
| 后端 | `soulcore` 36 条 IPC 命令 + 15 crate；Tauri 2 壳 | 无，v0 by design 无后端（BD7） | **NO_HIGH_VALUE_CHANGE_FOUND**（v0 不建服务；「后端」的位置由 bead-core oracle + 本地 store 契约占据） |
| API | IPC 契约 + 11 份冻结 JSON Schema（schema-freeze 钉 sha256） | 无 | bead 的「API」= bead-core 公开 Rust API（oracle）+ `.beadproj` schema + TS store 接口。**缺口：BD8 的对照机制没有落法** → §4 B01/B03 的 golden fixtures 方案 |
| 数据库 | SQLCipher 加密 SQLite + Windows DPAPI 链 KEK | 无 | WP-B09 只写了「IndexedDB」一个词：object store 划分、版本迁移、导出全量都没定 → §4 B09 |
| 登录与权限 | 无账号；HITL/consent 门 + E1 仅用户触发 | 无账号，单机档案（BD7） | **NO_HIGH_VALUE_CHANGE_FOUND**（v0 无账号体系是拍板不是缺口；「图纸默认不出网」由安全面的 no-egress 断言承接） |
| Storage | store crate 族 + `config.json` 仅 2 字段 | 无 | 与数据库面同归 B09；补充：`.beadproj` 导出即备份通道，导出/导入往返要有测试 |
| Cache | 无独立缓存层 | 无 | **NO_HIGH_VALUE_CHANGE_FOUND**（v0 转换结果直接存进项目记录，无需独立缓存层；大图性能走 Worker，见性能面） |
| 第三方 | 依赖全钉版本；deny.toml + e0 双网；HTTP 客户端仅 soul-egress | 无依赖 | 品牌色板不可收录（BD9，侵权）→ `generic-5mm` fixture；图像解码用浏览器 canvas，**bead-core 不需要 image crate**（§4 B01）；`cargo deny` 不覆盖 bead-core → 用「近零依赖」抵消 |
| 核心业务 | 灵魂层/代理层（与 bead 无交集） | 仅 PLAN 文本 | B01–B05 全部未落。**PLAN 有、WP 没有的**：六角/圆板、多板拼接 → 建议显式推迟 post-v0 并记录，v0 只做方板（28×28/56×56/比例适配/视口缩放） |
| 测试 | 本机 `just ci-full` 绿：vitest 161 项/14 文件 + cargo 全套 + 桌面壳 5 套 | 无 | bead 测试金字塔缺失 → §4 B10：bead-core cargo 测试（吃 `fixtures-ciede2000.md` 全部 20 对，容差 1e-4）、TS/Rust golden 对照、UI 导航/空态、no-egress 源码断言 |
| 构建 | just + pnpm + cargo，版本全钉 | `docs/bead/README.md` 已写占位命令 | 落地时唯一共享触点是锁文件（§2.3）；bead-core 构建入口 `cargo test --manifest-path crates/bead-core/Cargo.toml` 已在 README，保持 |
| CI/CD | `ci.yml` 五门；hosted 因 Billing 空跑；本机绿 ≠ hosted 绿 | 无 | B10 建独立 `.github/workflows/bead.yml`（push 限 bead 分支 + paths 过滤 + workflow_dispatch），**不碰 ci.yml**；Billing 修好前 bead 也拿不到 hosted 绿 → 本线 ROUND 门禁 = 本机跑 + review 文档记录，禁止 empty-commit 试探 |
| 性能 | 无基准（Soul 侧非目标） | 无 | v0 唯一可预见热点：正常图→色板映射+抖动。§4 B03：转换放 Web Worker；先给软预算（512×512 照片 → 56×56 板 < 1s 桌面级），正式基准推迟 |
| 安全 | E0/E1、注入、脱敏、审计链全套 | 无 | bead 威胁面 = 导入文件解析 + 出网。§4 B07：`.beadproj` 走 schema 校验 + 尺寸上限；`.pat`/`.gamedev` 解析失败必须干净地 `UNSUPPORTED_FORMAT`；禁 `eval`/`dangerouslySetInnerHTML`；no-egress 断言（B02） |
| 可靠性 | crash harness、单实例、崩溃恢复测试 | 无 | v0 最小集：IndexedDB 逐步自动保存拼装进度（步进粒度）、刷新后恢复到当前步、导出/导入往返测试 → 并进 B04/B09 |

## 4. WP 精化（只加实现前必须知道的，不造工作量）

### WP-B01 bead-core（给 O1）

1. **输入契约**：一律吃 `RGBA` 数组 / `Lab` 数组 / `Grid<ColorId>`，**不做 PNG/JPG 解码**，不引入 `image` crate。
   解码属于浏览器（canvas）。这让 bead-core 依赖近零（建议仅 `serde` + `serde_json`，版本随根钉 1.0.217/1.0.135）。
2. **CIEDE2000 验收升级**：`docs/bead/fixtures-ciede2000.md` 已把 WP 的「至少 5 组」升级为 Sharma Table I
   全部 20 对、Lab 直入、容差 1e-4 —— 以该文档为准；sRGB→Lab 与最近邻测试分开断言（同文档最后一段）。
3. **确定性拆步规则**（BD8 对照可测的前提，两侧同一规则）：
   - 最近色并列：取色号索引最小者；
   - Color-by-Color 计数并列：按色号索引升序；
   - Outline→Infill：4-连通判连通分量；「外轮廓」= 与异色或板边 4-邻接的格；分量顺序按 (min_row, min_col)，
     分量内行优先；
   - Tile / Row-by-Row：行优先、左到右（WP 已写，重申为并列裁决的一部分）。
4. **golden 导出**：bead-core 测试里加一个把「fixture 网格 → 四模式色号序列 + BOM」写成 JSON 的用例，
   产物提交到 `crates/bead-core/fixtures/golden/`（目录名 `fixtures` 天然豁免两个 audit），TS 侧直接消费（B03）。
5. **红线复述**：Rust 1.83 / edition 2021；`Cargo.toml` 无 URL 字段；`confidence` 不叫 `score`；
   自带 `[workspace]`；README 写清跑法。

### WP-B02 应用壳（给 O2）

1. 同一提交里重生成 `pnpm-lock.yaml`、`.gitignore` 加 `apps/bead/dist/`（§2.3）。
2. vite 配置钉 `port: 1520, strictPort: true`（不与 Soul 的 1420 抢）。
3. `tsconfig.json` / `package.json` 不写 `$schema`；自带 eslint 配置，不复用 `apps/desktop` 的。
4. 加一个源码断言测试（模式照抄 `apps/desktop/src/contract.test.ts` 的思路）：`apps/bead/src/**` 里不得出现
   `fetch(`、`XMLHttpRequest`、`WebSocket`、`sendBeacon`、外网 URL —— 这是「图纸默认不出网」（B09 红线）
   的可执行形式，也顺手保住 e0-audit 绿。
5. 交付验证除 `pnpm --filter @bead/app test` 外，**必须在仓库根跑一次 `just ci`**：这是共存回归的唯一入口。

### WP-B03 转图管线（给 O3）

1. **落点先拍板**（§2.5）：建议 `apps/bead/src/algo/`，不建 `packages/bead-algo`。
2. 对照走 golden：消费 B01 的 `crates/bead-core/fixtures/golden/*.json`，**比对色号序列与 BOM，不比对 ΔE 浮点值**
   （Rust/JS 的 libm 在 atan2/pow 上可差 ULP，钉浮点会做出脆测试；固定并列裁决后序列是稳定的）。
3. 转换跑在 Web Worker 里，主线程只做进度条 —— 这是性能面在 v0 的全部要求。
4. 解码用 canvas `drawImage` + `getImageData`，不引第三方图像库。

### WP-B04 沉浸式拼装台

精化一条：当前步索引随进度写入 B09 的存储（步进粒度自动保存），刷新/崩溃后从当前步恢复；
其余按 WP 原文即可。悬浮球/计时/里程碑不需要新契约。

### WP-B05 库存与清单

精化一条：库存色号必须与图纸同一色板命名空间（v0 即 `generic-5mm`），替代色搜索只在库存内做 ΔE00<3 升序
（WP 已写）——跨色板替代推迟。导出采购文本用纯文本，别做富格式。

### WP-B06 像素编辑器

**NO_HIGH_VALUE_CHANGE_FOUND**（WP 原文已足够收敛：五个工具、可后于 B03。等 B03 的网格模型定型再动）。

### WP-B07 导入导出

1. `.beadproj` schema 不写 `$id`/`$schema` URL（§2.1）；校验失败与超限（建议：格数 ≤ 512×512、
   文件 ≤ 20 MB）给明确错误，不静默截断。
2. `.pat` / `.gamedev` 是无公开规范的私有格式：先做「识别魔数 → 不认识就 `UNSUPPORTED_FORMAT`」，
   逆向解析推迟到有真实样本再说（避免臆造格式的忙碌工作）。

### WP-B08 Explore fixture 画廊

精化一条：画廊 fixture 里创作者/作品不得带外网链接字段（既是 BD7 无社交后端的一致性，也是 e0 扫描的现实约束）；
fixture 数据放目录名字面为 `fixtures` 的路径下。其余按 WP 原文。

### WP-B09 本地存储

把「IndexedDB（或等价）」落成可实现的契约：

- object store 三个：`projects`（图纸+转换结果+元数据）、`inventory`（色号→颗数）、`progress`（projectId → 模式、当前步、用时）；
- 库名带版本（`bead-v1`），迁移 = onupgradeneeded 单向升级；
- 「导出全部」= 打包成 `.beadproj` 数组下载（Storage 面的备份通道）；
- 「不出网」由 B02 的源码断言承接，此处不再重复造门。

### WP-B10 测试 / 构建 / 隔离 CI

1. 新建 `.github/workflows/bead.yml`：`push` 限 `cursor/bead*` 分支 + `paths: [apps/bead/**, crates/bead-core/**, docs/bead/**]` + `workflow_dispatch`；job 两个：`cargo test --manifest-path crates/bead-core/Cargo.toml`（含 fmt/clippy 该包内跑）与 `pnpm --filter @bead/app lint && test`。**不碰 `ci.yml` 一个字节**。
2. 明知 hosted runner 因 Billing 空跑：workflow 落进去是为了 Billing 修好后即时生效，**本线 ROUND 门禁在此之前 = 子代理本机跑 + 在 review 文档里记录命令与结果**。禁止 empty-commit 试探 runner。
3. 每个实现 PR 的验证清单固定四条：包内测试绿、仓库根 `just ci` 绿（共存回归）、
   `cargo run -p xtask -- e0-audit` 绿、`cargo run -p xtask -- denylist-audit` 绿。后两条已含在 `just ci`，
   单列是因为它们是 bead 最容易踩的两个雷（§2.1/§2.2）。

## 5. ROUND 2 建议焦点

按价值排序：

1. **O1 交付审查（F3 对口）**：CIEDE2000 20 对全过、确定性规则（§4 B01.3）落实、golden 导出存在、
   依赖近零、denylist/e0 双绿。这是 oracle，错了下游全错。
2. **O2 交付审查**：锁文件与 `.gitignore` 是否同提交、根 `just ci` 在 bead 分支上是否仍绿（共存回归第一次实测）、
   no-egress 断言是否落地。
3. **O3 落点裁决执行**（§2.5，父代理拍 BD14）+ golden 对照通路打通（BD8 从口号变成测试）。
4. **B09 存储契约先行**：B04（进度恢复）与 B05（库存）都压在它上面；ROUND 2 至少把 store 接口 + 迁移骨架定下来，
   避免 B04/B05 实现者各自发明存储。
5. **尚未覆盖、留给 ROUND 3+ 的探针面**：大图性能基准（Worker 落地后再测）、`.pat`/`.gamedev` 真实样本收集、
   六角/圆板与多板拼接（PLAN 有、v0 无）、wasm 化 bead-core（等 TS 复刻先证明契约）。
6. **不要做的**：不建 bead 后端、不加账号、不建缓存层、不动 Soul 任何门禁 —— 本文三个
   NO_HIGH_VALUE_CHANGE_FOUND 面在 v0 期间不再往返检查。
