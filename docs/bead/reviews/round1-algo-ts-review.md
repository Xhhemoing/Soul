# Round 1 TS 转图管线审查 — 已合入的 WP-B03（apps/bead/src/algo）

- 审查模型：`claude-fable-5-thinking-xhigh`（无降级；只审不改，本 PR 仅新增本文档）
- 实现模型（被审方）：`claude-opus-5-thinking-high-fast`（O3，#24 已合入）
- 基线：`cursor/beadflow-integration-c441` @ `384bc3d`（含 WP-B03 合入 `e94e6c5`，内容提交 `de716a7` + `3e01025`）
- 对照物：`docs/bead/reviews/round1-algorithms.md`（G1–G8 / T-\*）、`apps/bead/src/algo/contract.md`、`crates/bead-core`（WP-B01，`5d62896` #23）、BD14/BD15/BD17
- 本机复核：`pnpm --filter @bead/app test` 20 文件 217 全绿；`cargo test --manifest-path crates/bead-core/Cargo.toml` 全绿；`cargo run -q -p xtask -- e0-audit` 与 `denylist-audit` 均 clean
- 日期：2026-08-25

## 0. 结论

**不是 NO_HIGH_VALUE_CHANGE_FOUND。** TS 侧本身质量很高：G1–G8 逐条落实、T-\* 测试矩阵基本齐、契约文档（contract.md）比原审查更细。但有两个必须跟进的问题：

1. **验收核心目前是空转的**（AT-1）：「同一 fixture 在 Rust 与 TS 得到同一色号序列」在当前代码下**结构性不可能成立**——两侧连 `generic-5mm` 色板本身都不是同一份（40 色 vs 48 色，RGB 完全不同），`crates/bead-core` 对 `fixtures/parity.json` 零引用。contract.md 是一份「Rust 侧照抄即可」的单向宣言，Rust 侧至今一条都没抄。
2. **一个已复现的色号序列 bug**（AT-2）：像素图路径在放大图末格被截断（典型来源：裁剪过的截图，正是 `detectGrid` 支持相位偏移要服务的输入）时，`collapseUpscale` 采样漂移跨格，输出错误色号且会整格丢色。

## 1. 与 Rust oracle 的色号序列对照 —— 目前对不上，且不是浮点级差异

契约的验收标准（contract.md 第一段、WP-B03 原文）是色号序列全等。逐模块对照后：**两侧是两套各自绿灯、彼此无关的实现**。分歧不在 ulp，在语义：

| # | 维度 | TS（src/algo） | Rust（bead-core） | 后果 |
|---|------|----------------|--------------------|------|
| 1 | `generic-5mm` 色板 | 40 条（G01 纯白…G40 炭灰），中文名，如 G08=(230,40,50) | 48 条（G01 White…G48 Skin Deep），如 G08=#6B7A85 Slate | 同一 id 指向两套色域；任何「色号全等」比对无意义 |
| 2 | 透明（G1） | alpha≥128，空格不进 BOM/步骤/误差流 | `Image` 是纯 RGB，**没有 alpha 通道**；透明靠 `StepOptions::skip` 按色跳过 | parity fixture 的 RGBA hex（4 字节/像素）Rust 连解析都做不到；`transparent-hole` 用例无对侧 |
| 3 | 重采样（G5） | 手写 box，**线性光** + alpha 加权，`roundHalfUp`；判据 2×1 黑白→**188** | `box_average` 在 **sRGB 码值**空间取整像素平均，`f64::round` | 契约原话「哪天看到 128 就是有人改成了码值空间平均」——Rust 现在就输出 128 |
| 4 | 框定三模式 | board=整幅重采样到 28/56；aspect=最长边 maxSide；manual=像素 crop 矩形×scale | FixedBoards=cover **居中裁剪**；AspectFit=在整板拼数里选长宽比最近的 cols×rows；ScaleCrop=相对 cover 的视口缩放平移 | 参数形状和语义都不同，连同一请求都表达不了 |
| 5 | 网格提取（G3） | `detectGrid`：变化位置差值 gcd + 相位，上限 64，不要求整除 | `detect_block_size`：k 须整除两个维度，k≤32，**无相位** | 3px 偏移变体（T-GRID-1）在 Rust 侧无法表达 |
| 6 | 判定器 | 0.5·flat + 0.3·gridEvidence + 0.2·colorEvidence（256 上限）；全透明特判 PixelArt@0.5 | 0.45·flat + 0.35·palette(√面积) + 0.20·block(0/1)；confidence=到边界距离归一 | 同图可判成不同 kind → 走不同路径 → 序列必不同 |
| 7 | 抖动（G7） | 查表输入是**未取整**的小数码值（仅夹取）；空格不收不传误差；照片路径默认关 | 查表前 `round().clamp()` 到 u8；无空格概念；照片路径默认**开** FS；还带契约明令不做的 serpentine 变体 | 决策边界附近序列分叉；默认行为相反 |
| 8 | Outline→Infill（G4） | 分量在**非空掩码**上做（跨色连通）；洞=外部洪泛不可达；slug `inner-border` | 分量按**同色区域**做；outline=「挨着任何非本区域者」；slug `inner-edge` | 步骤划分和命名双重不兼容 |
| 9 | tie-break（G6） | CBC/BOM 次键=**色板索引**升序 | 次键=**code 字符串**升序 | generic-5mm 两侧各自巧合一致，任意色板下分叉 |
| 10 | 0×0 网格（G8） | 合法，空 BOM、零步骤 | `Grid::from_vec` 直接报 `Empty` | 边界语义相反 |
| 11 | 近平局哨兵（T-PAR-3） | `nearestEntry` 返回 runnerUpDeltaE，管线交出 minRunnerUpMargin | `PaletteMatch` 无次优信息 | oracle 侧按契约本应生成/校验哨兵，现在没有 API 可做 |
| 12 | parity fixture（T-PAR-1） | `fixtures/parity.json` 4 用例，期望值由 TS 自产自销 | **零引用** | 「同一份文件同时被 vitest 与 cargo test 消费」未发生；现状只是 TS 回归锁 |

一致的部分（真正照抄成功的）：CIEDE2000 全部分支、7 位小数 sRGB→XYZ 矩阵、D65 白点、ε/κ 有理数、h′=0 约定、±180° 回卷、最近色严格 `<` 取最小索引、FS 权重 7/3/5/1、替代色严格 `<3`。也就是说**颜色数学已对齐，管线语义全未对齐**。

判断：这不是 TS 侧的错——TS 是 G1–G8 审查契约的忠实实现，Rust（先落地的 WP-B01）几乎每条都与契约相左。但「Rust 是 oracle」的说法目前名不副实：真正的行为规范在 TS + contract.md 里。**两侧必须选一个方向收敛，不能继续各自绿灯。**

## 2. 已复现 bug：截断放大图的 collapseUpscale 采样漂移

`pipeline.ts` 的 `collapseUpscale` 把撤销放大交给 `resampleNearest`，而后者用 `scaleX = image.width / targetWidth` 推采样点。当放大图宽度不是 cellWidth 的整倍数（末格被截断——即被裁剪过的像素图截图，`detectGrid` 支持 offset 的意义所在），`scaleX ≠ cellWidth`，采样点逐格漂移并越过格边界。

复现（本机已跑）：9×4 图，3 个逻辑像素 [红,绿,蓝] 4× 放大、末格截为 1px。`detectGrid` 正确给出 cellWidth=4、offset=0，但收缩输出为 `[G08, G08, G15]`——第二格错取了红，蓝格整格丢失：

```ts
const img = imageFromPixels(9, 4, (x) => {
  const cell = Math.min(Math.floor(x / 4), 2); // [红,绿,蓝]，末格 1px
  ...
});
imageToPattern(img, { framing: { mode: "aspect", maxSide: 3 }, kind: "PixelArt" });
// grid 3×1，codes = [G08, G08, G15]，期望 [红,绿,蓝] 三个不同色号
```

修法方向：采样点按检测几何直接算（`sx = offsetX + tx * cellWidth`，或取格中心并对末格夹取），不要经过 width/target 的比值。同时注意：前导残格（x < offsetX 的像素）目前被 `logicalWidth = ceil((width−offsetX)/cellWidth)` 静默丢弃——该行为契约没钉、测试没锁，修复时一并写进 contract.md 并加 fixture。

现有测试为何没抓到：`framing.test.ts` 的最近邻用例与 `pipeline.test.ts` 的像素图用例全部是整倍数、offset=0；parity 的 `pixel-art-upscaled-4x` 也是 28=4×7 整除。

## 3. 缺测

- （并入 AT-2）截断末格与 offset≠0 的放大图走完整管线的用例——上述 bug 的逃逸原因。
- T-PAR-2 的「单测一张已知 PNG 的前 N 像素字节」没有做：jsdom 无真 `createImageBitmap`，`decode.test.ts` 退而锁调用形状（colorSpaceConversion/premultiplyAlpha=none、1:1 绘制、平滑关闭）。作为单元测试这是合理的取舍且已在注释说明，但真字节断言需要浏览器模式（Playwright/vitest browser）跑一次，round 2 补。
- Sharma 表只嵌了 1–20 + 34 共 21 组。原审查的 8 组必选全在，合格；全 34 组是 SHOULD，不算缺口。

## 4. create 上传未接线 —— 可接受

`decodeImage` 已实现并导出，但 `src/algo` 之外无任何 import（全树检索确认）。判定可接受，理由：

1. BD14 把 O3 落点钉死为「只写 `apps/bead/src/algo/`」，接线必然要动 `pages/create`，越界；
2. 不是遗忘：合入提交 `384bc3d` 与 `docs/agent-progress.md` 都写明「`/create` 上传仍故意不接线」，并已列为独立跟进项（「独立变更，勿打坏 B02 文案测试」）；
3. WP-B02 的 CreatePage 桩测试因此保持有效，无回归。

条件：该跟进项必须真的排上（AT-3），否则 WP-B03 验收里「支持上传 png/jpg」始终只完成了一半。

## 5. 过度设计 —— 未见

逐文件看过：`preparePalette` 的 WeakMap 缓存（避免每次量化重算 40 个 Lab）、`decode.ts` 的 `Canvas2d` 最小接口（OffscreenCanvas 与 DOM canvas 双轨）、`imageFromPixels` 测试构造器，都有直接用途。没有为六角板/多板拼接等 v0 外需求预留的死代码。4 个 parity 用例、40 色色板的规模也克制。`resampleNearest` 的 offset 参数当前只被 `collapseUpscale` 用到，属于正常内聚。

反而是 **Rust 侧**有超契约面：serpentine 抖动变体（G7 明令不做）、AspectFit 的整板搜索。这属于 WP-B01 对齐时要处理的（并入 AT-1），不是本次 TS 审查对象的问题。

## 6. URL 与 BD15 denylist —— clean

- `constraints.test.ts` 把 BD15/BD16 做成了子树 linter：禁词（英文那个词及其三个中文对应）、`http(s)://`、`$schema` 全部零匹配才过，且扫描含 fixtures 与 md。本机 217 绿包含它。
- 手工全树 grep 复核 `src/algo`：clean。判定器数值输出叫 `confidence`、中间量叫 `evidence`，与 BD15 一致。
- `cargo run -q -p xtask -- e0-audit`（14 crate、276 文件）与 `denylist-audit`（94 词、127 文件）均 clean。
- BD17：本树无 workflow_dispatch 触碰，注释里也无 URL 字面量。

## 7. 回归 —— 无

- 合入前壳是 43 测试，现 20 文件 217 全绿（含 B02 的导航/空状态/CreatePage 桩测试），placeholder 换真实现没有打坏任何既有测试。
- `e94e6c5` 未动 package.json/pnpm-lock（无新依赖），BD16 无涉。
- `cargo test` bead-core 全绿，Rust 侧未被触碰。

## 8. Opus 跟进清单

| # | 优先级 | 内容 |
|---|--------|------|
| AT-1 | HIGH | **两侧收敛决策 + 实施**。推荐方向：以 TS + contract.md 为规范改写 `crates/bead-core`（色板换成 TS 的 40 色 generic-5mm、加 alpha/空格、box 改线性光、框定改三模式同参数、detectGrid 带相位、判定器同公式、抖动查表不取整、outline-infill 改非空掩码、tie-break 改索引、Grid 允许 0×0、nearest 加 runner-up、删 serpentine），然后让 `cargo test` 直接消费 `fixtures/parity.json` 的 4 个用例逐色号断言。若改判 Rust 为主，则 TS 与 contract.md 同规模重写且 round1-algorithms.md 的 G1–G8 需重议——代价更大，不推荐。做完前，README/进度文档里「rust oracle + TS 管线都已合入」的表述应加注「色号 parity 尚未建立」 |
| AT-2 | HIGH | 修 `collapseUpscale`/`resampleNearest` 截断漂移（见 §2 复现），按几何直取采样点；钉死前导残格语义；补 3 个 fixture：末格截断、offset≠0、offset≠0+截断，走 `imageToPattern` 全管线断言色号 |
| AT-3 | MED | `/create` 上传接线（decodeImage→imageToPattern→store），独立 PR，保住 B02 桩测试；这是 WP-B03 验收「支持上传 png/jpg」的收尾 |
| AT-4 | LOW | `color.test.ts` 顶部注释称 Sharma #34 的值「fixture 文档里也引了」——`docs/bead/fixtures-ciede2000.md` 实际只有 1–20 组。值本身与论文一致（0.9082，非编造），把 #34 补进该文档或改注释，二选一（WP-B01 验收语言原本要求「夹具未列标 TODO」） |
| AT-5 | LOW | round 2 用浏览器模式补一张真 PNG 的解码字节断言（T-PAR-2 的另一半，见 §3） |

## 9. 禁区自查

本审查只新增 `docs/bead/reviews/round1-algo-ts-review.md`。未触碰：产品代码（`apps/**` 源码、`crates/**`）、`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、根 `Cargo.toml`、`deny.toml`、lockfile。复现脚本只写在 `/tmp`，未入库。
