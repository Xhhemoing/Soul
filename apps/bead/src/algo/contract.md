# `src/algo` — WP-B03 浏览器管线契约

这里是 BeadFlow 的 TypeScript 转图管线。`crates/bead-core`（WP-B01）是算法 oracle
（BD18/BD8）：验收标准是**同一 fixture 在两侧得到同一色号序列**，比对色号而不是裸 ΔE
浮点，**分歧一律由本侧向 Rust 收敛**，不改 `crates/bead-core` 去迁就本文。本文逐条
钉死 `docs/bead/reviews/round1-algorithms.md` 的 G1–G8，同时记录本侧与 oracle 仍未
收敛的部分（见文末）。

色板 `generic-5mm` 逐条抄自 `crates/bead-core/src/palette.rs`：48 色，同码、同名、
同 hex。同一个 `G??` 在两侧必须指向同一颜色，否则「色号全等」无从谈起
（`oracle-parity.test.ts` 直接对着那份 Rust 源码逐条比）。

落点约束（BD14）：只在 `apps/bead/src/algo/`，不建 `packages/bead-algo`。
命名约束（BD15）：判定器的数值输出一律叫 `confidence`，中间量叫 `evidence`；其余同义
叫法（英文 s-c-o-r-e 及其中文对应词）在本树一律禁用，由 `constraints.test.ts` 把关。
共存约束（BD16）：本目录不写外网 URL，JSON fixture 也不写 schema 声明键。

## G1 透明像素

`alpha ≥ 128` 视为不透明，否则是空格（`Grid` 里的 `null`）。空格不进 BOM、不进
任何步骤、不接收也不中转抖动误差。常量 `OPAQUE_ALPHA_THRESHOLD`。

## G2 跨语言浮点漂移

- ΔE00 完全相等时取**色板索引最小者**（`nearestEntry` 用严格 `<` 比较实现）。
- sRGB→XYZ 矩阵钉死为下面 7 位小数形式。行和在 7 位小数上等于 D65 白点，所以中性色
  的 a\*/b\* 落在 1e-4 以内，不需要任何白点特判（不是逐位精确 0，别去断言 0）：

  ```
  0.4124564  0.3575761  0.1804375
  0.2126729  0.7151522  0.0721750
  0.0193339  0.1191920  0.9503041
  ```

  白点 `Xn = 0.95047, Yn = 1.00000, Zn = 1.08883`；ε = 216/24389，κ = 24389/27，
  两侧都按有理数写，不要写十进制近似。**由原色重新推导的矩阵在第 4 位就分叉**，
  会把 `(255,0,0) → L* 53.2408` 打成 53.2222。
- `nearestEntry` 顺带返回 `runnerUpDeltaE`；管线把全图最小的
  `runnerUp − best` 作为 `minRunnerUpMargin` 交出来，parity fixture 用它做近平局
  哨兵（T-PAR-3，阈值 1e-6）。

## G3 网格提取

`detectGrid(image) -> {cellWidth, cellHeight, offsetX, offsetY}` 是公开函数，不藏在
管线里：

- 「变化列」= 与左邻列存在任一像素 RGBA 不同的列下标，行同理；
- 周期 = 相邻变化位置差值的 gcd，相位 = 首个变化位置对周期取模；
- 变化位置少于 2 个（无从推断周期）⇒ 该轴整幅算一格：`size = 跨度, offset = 0`；
- 周期 > `MAX_DETECTED_CELL`（64）同样退化为整幅一格。

撤销放大（`collapseLattice`）按检测到的格点直接取样，**不**经过
`源尺寸 / 目标尺寸` 的比值：末格被截断时两者不等，比值会让采样点逐格漂移并越过格边界
（AT-2 的复现：9×4 图、cell 4、逻辑像素 [红,绿,蓝]，比值法输出 `[G08, G08, G15]`——
第二格错取红、蓝格整格丢失）。格点语义钉死为：

- 格边界在 `offset + k·cell`；
- **前导残格**（`offsetX > 0` 时的 `[0, offsetX)`）与**截断的末格**都各算一个逻辑像素，
  一轴的逻辑像素数 `latticeCells(跨度, cell, offset) = (offset > 0 ? 1 : 0) + ⌈(跨度 − offset) / cell⌉`
  （下限 1）——被裁掉的截图丢的是格子的一部分，不是格子本身；
- 每个逻辑像素取自己那段 `[start, min(end, 跨度))` 的中点 `⌊(start + end) / 2⌋`，
  再夹进图内。整格时它就等于 `start + ⌊cell / 2⌋`，与 Rust `render` 的最近邻取样点同式。

## G4 Outline→Infill

- 连通分量在**非空掩码**上做，**4 邻接**；
- 洞 = 从网格外边界 4 邻接洪泛不可达的背景格；
- 外轮廓 = 与「网格外部或外部背景」8 邻接的分量格；
- 内边界 = 与洞 8 邻接的分量格；
- 一格只归一类，**外轮廓优先**；空类别不产生空步骤；
- 分量顺序按首格「最上、再最左」，步骤内格序行优先。

## G5 缩放重采样

手写 box（面积平均）滤波，**在线性光空间**平均，颜色按 alpha 加权（避免边缘发黑），
alpha 自己线性平均。舍入统一走 `roundHalfUp(x) = floor(x + 0.5)`。

判据用例：2×1 的 [黑, 白] 缩到 1×1 ⇒ 输出 **188**（线性光 0.5 编码回 sRGB）。
如果哪天看到 128，说明有人改成了 sRGB 码值空间平均。

浏览器的 `drawImage` 平滑缩放**不参与**任何一步：它的滤波器没有规范，必然与 Rust
分叉。三种框定：

| 模式 | 语义 |
|------|------|
| `board` | 整幅重采样到 28×28 或 56×56 |
| `aspect` | 最长边 = `maxSide`（默认 28），另一边按比例 `roundHalfUp`，每轴夹到 ≥ 1 |
| `manual` | 先按 `crop` 裁（越界部分补透明，完全在图外报 `CropOutOfBounds`），再乘 `scale` |
| `fixed-boards` | oracle 的 `FitMode::FixedBoards`：格数 = `board × cols/rows`，源图按该长宽比 **cover 居中裁剪**，再按 `sampling` 取样 |

`fixed-boards` 是 `planFixedBoards` + `renderFit` 的门面，逐行对应
`bead_core::fit::{fixed_boards, render}`：

- `sampling: "nearest"` 取格中心 `⌊x0 + cellW/2⌋`，格外一律透明（不夹边、不造边缘色）；
- `sampling: "box-average"` 按覆盖到的**整像素**等权平均（不是面积加权），颜色只取
  不透明样本、在线性光里平均，alpha 取全部样本的平均后再按 128 重判；整格不透明度不足
  就回到透明黑。

前三种模式是产品 UI 的框定，oracle 侧目前没有对应表达；跨语言 parity 只走
`fixed-boards`。

## G6 排序 tie-break

| 场景 | 主键 | 次键 |
|------|------|------|
| Color-by-Color | 颗数（默认升序，点缀色先行；可切降序） | 色板索引升序 |
| BOM | 颗数降序 | 色板索引升序 |
| 替代色 | ΔE00 升序 | 库存下标升序 |
| 连通分量 | 首格行号 | 首格列号 |
| 步骤内格序 | 行 | 列 |

## G7 Floyd–Steinberg

- 经典 raster 扫描（左→右、上→下），**不做 serpentine**；
- 误差在 **sRGB 码值**空间按 f64 累计，**不提前夹取**；只在查最近色前夹到 [0,255]；
- 误差按 7/16 右、3/16 左下、5/16 下、1/16 右下扩散；越界丢弃；
- 空格既不接收也不中转误差；
- 抖动关闭时与逐像素独立最近色映射逐格全等。

像素图路径**强制关闭抖动**（结果里的 `ditherApplied` 报告实际生效值）：像素图本来就
是平块，扩散误差正好毁掉这条路径要保的结构。

## G8 边界

- 替代色阈值是严格 `< 3`（ΔE00 恰好 3 不入选）；
- `Grid` 允许 0×0 / 全透明，产出空 BOM、零步骤，不报错；
- 图像管线遇到 0 尺寸、`scale ≤ 0`、零面积 crop、完全越界 crop 一律抛
  `AlgoError`（`InvalidDimensions` / `InvalidScale` / `InvalidCrop` /
  `CropOutOfBounds` / `EmptyPalette`），不 panic、不返回哨兵值。

## 判定器（PixelArt | Photo）

纯规则启发式，不接外部视觉 API。三个特征，都只看不透明像素：

- `flatNeighbourRatio`：上下/左右相邻像素对中 RGBA 逐字节相同的比例（无相邻对时取 1）；
- `gridEvidence`：按 `detectGrid` 的格子切开后内部完全一致的格子占比；
  `cellWidth ≤ 1 且 cellHeight ≤ 1` 时取 0；
- `colorEvidence = max(0, (256 − 独特色数) / 256)`。

```
evidence   = 0.5·flatNeighbourRatio + 0.3·gridEvidence + 0.2·colorEvidence
kind       = evidence ≥ 0.5 ? PixelArt : Photo
confidence = kind === PixelArt ? evidence : 1 − evidence      // 恒在 [0,1]
```

不透明像素数为 0（全透明图）是唯一特判：判 `PixelArt`，`confidence = 0.5`
（`DEGENERATE_CONFIDENCE`），避开除零。

## 上传解码（T-PAR-2）

`decodeImage` 用 `createImageBitmap(..., { colorSpaceConversion: "none",
premultiplyAlpha: "none" })`，再以自然尺寸 1:1 绘制、`imageSmoothingEnabled = false`
后 `getImageData`。这样嵌入 ICC 配置不会改字节，解码阶段也不会重采样——缩放全部
归 `framing.ts`。**parity fixture 不走这条路**，一律用原始 RGBA 数组。

## parity fixture

跨语言 parity 用 **oracle 自己的** 夹具，本侧只读不改：

- `crates/bead-core/fixtures/parity/*.json`（`pixel-art` / `photo-flat` /
  `photo-dithered` / `with-transparency`）由 `bead_core::parity::build` 写出，
  含原始 RGBA、`fit`/`sampling`/`dither` 与期望色号、BOM；
- `oracle-parity.test.ts` 按同样的 `plan → render → quantize → BOM` 顺序重放这四条，
  逐格比色号、逐行比 BOM（码、名、颗数）。它**绕开判定器**：oracle 的生成器也没有
  判定器，接上去比的就是另一条管线了。它另外走一遍完整 `imageToPattern`
  （`framing: fixed-boards` + `kind: "Photo"`），确认门面与直调同解。

`fixtures/parity.json` 保留，但角色变了：它锁的是 **oracle 表达不了的本侧路径**——
判定器结论、`detectGrid` 撤放大、`aspect` 框定、四种步骤划分。期望值由本侧自产，
是回归锁而不是 parity 证据。某个 case 的 `minRunnerUpMargin ≤ 1e-6` 时换 fixture，
不要改容差。

## 尚未与 oracle 收敛的部分

色号序列已在四条 fixture 上全等；下列语义仍是两套，**待后续按 BD18 由本侧收敛**：

| 维度 | 本侧 | oracle |
|------|------|--------|
| 框定 | `board` / `aspect` / `manual` | `FixedBoards` / `AspectFit`（整板搜索）/ `ScaleCrop` |
| 网格提取 | `detectGrid`：gcd + 相位，上限 64，不要求整除 | `detect_block_size`：k 须整除两轴、≤ 32、无相位 |
| 判定器 | `0.5·flat + 0.3·grid + 0.2·color` | `0.45·flat + 0.35·palette + 0.20·block` |
| 抖动查表 | 输入是未取整的小数码值（仅夹取） | 查表前 `round().clamp()` 到 u8 |
| Outline→Infill | 分量在非空掩码上做，内边界 slug `inner-border` | 分量按同色区域做，slug `inner-edge` |
| tie-break 次键 | 色板索引升序 | `code` 字符串升序（`generic-5mm` 下两者巧合一致） |
| 面积平均 | `resampleBox` 按覆盖面积加权 | 覆盖到的整像素等权（本侧 `renderFit` 已同式） |
