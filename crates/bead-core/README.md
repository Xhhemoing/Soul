# bead-core

BeadFlow 的算法源（WP-B01）。一张图片进来，一份色号清单和四套拼装指引出去。

这个包**不在**仓库根 `Cargo.toml` 的 `[workspace].members` 里，它自己就是一个
workspace 根（`Cargo.toml` 里那张空的 `[workspace]` 表就是干这个的）。这样 Soul 的
`deny.toml`、`e0-audit` 和 `just ci` 不必去理解一张它们并不发布的依赖图。

零依赖：全部是 std 集合上的算术。不开 socket，不碰文件。

验收契约是 `docs/bead/reviews/round1-algorithms.md`。它列的八处缺口（G1–G8）在下面
逐条钉死，编号测试（`T-*`）散落在各测试文件里，用注释标了编号。

## 怎么跑

从仓库根目录：

```bash
cargo test  --manifest-path crates/bead-core/Cargo.toml
cargo clippy --manifest-path crates/bead-core/Cargo.toml --all-targets -- -D warnings
cargo fmt   --manifest-path crates/bead-core/Cargo.toml --all -- --check
```

或者 `cd crates/bead-core` 之后直接 `cargo test`。根目录的
`cargo test --workspace` 跑不到这里——那是刻意的。

改动算法之后重新生成 parity 夹具：

```bash
cargo run --example emit-parity-fixtures --manifest-path crates/bead-core/Cargo.toml
```

## 里面有什么

| 模块 | 负责 |
|------|------|
| `color` | sRGB→Lab（D65, 2°）、CIEDE2000、透明度阈值、通道舍入 |
| `palette` | 色板、最近色查找、次优色与决策余量、`generic-5mm` 夹具 |
| `grid` | 矩形格子与 `Cell` 坐标（允许 0×0） |
| `image` | 已解码的 8-bit sRGB + alpha 栅格 |
| `detect` | 像素图 / 照片判定，以及 `detect_grid` 网格提取 |
| `fit` | 板型框定与 box 重采样 |
| `quantize` | 像素→色号，可选 Floyd–Steinberg |
| `steps` | 四种拼装顺序 |
| `bom` | 备料清单、库存比对、ΔE 替代色 |
| `pipeline` | 把上面接起来，给默认值 |
| `parity` | WP-B03 对照用的共享夹具 |

## G1–G8：钉死的选择

**G1 透明像素。** 输入像素是 `Rgba`，`alpha >= 128`（`color::ALPHA_OPAQUE_THRESHOLD`）
算不透明，低于就是空格。图案格子是 `Option<ColorId>`（`quantize::PatternGrid`），空格
写 `None`。空格不进 BOM、不进任何步骤、抖动时既不接收误差也不中转误差。这是阈值不是
混合：板上要么有一颗豆子要么没有，没有「半颗」。

**G2 跨语言浮点。** ΔE 完全相等时取色板索引最小者。`Palette::nearest_two` 和
`decision_margin` 把「赢了多少」暴露出来，parity 生成器据此断言每格的次优 − 最优
> `parity::NEAR_TIE_MARGIN`（1e-6），有近平局就拒绝出夹具（T-PAR-3）。

**G3 网格提取。** `detect::detect_grid` 是公开的，返回
`GridGeometry { cell_width, cell_height, offset_x, offset_y }`。每根轴单独解：先找出
「整列（整行）与前一列（前一行）不同」的位置，这些位置在格点图上模格宽同余，于是它们
两两之差的最大公约数就是格宽，第一个位置模格宽就是相位。用差值而不是穷举偏移，好处是
相邻两格恰好同色导致的「缺一条边界」不会把答案带偏。一根轴上少于两条边界不足以确定
周期，该轴取 1。格宽上限 `MAX_CELL_PROBE` = 64。

**G4 Outline→Infill。** 分量做在**非空掩码**上，不按颜色分——把熔珠板撑住的是豆子挨着
豆子，不是豆子颜色一样。连通性 4 邻接。洞 = 从网格外边界 4 邻接洪泛到不了的空格。外轮廓
= 与外部空格（或网格边缘本身）8 邻接的格；内边界 = 与洞 8 邻接的格；其余为填充。一格只
归一类，**外轮廓优先**，所以一格宽的环全算外轮廓、每格只出现一次。

**G5 重采样。** 两侧都手写同一个 box（面积平均）滤波，平均在**线性光**里做。舍入统一走
`color::to_channel`：四舍五入（.5 远离零），再饱和到 `0..=255`。判据用例 T-SCL-5：黑白
两像素并成一格，线性光给 188，sRGB 码值平均会给 128；本包给 188。按比例适配时每根轴的
格数同样四舍五入，且不低于 1（所以 3000×1 出来是 28×1 而不是 28×0）。

**G6 决胜键。** 一律色板索引升序。BOM 颗数降序、同数索引升序；Color-by-Color 计数为主
键、同数索引升序（两个方向都是升序，所以 B、C 的相对次序不随主键方向翻转）；替代色 ΔE
升序、同距索引升序；分量按「最上、再最左」的首格排序。

**G7 抖动。** 只有经典 raster（左→右、上→下），没有 serpentine——两种扫描序不可能都是
契约。误差按 sRGB 码值以 `f64` 累计，累计过程中**不夹取**，只在查最近色前夹到
`[0,255]`。越界不扩散。透明格既不接收也不中转。

**G8 边界。** `ΔE < 3` 是严格小于，且只有 `color::within_delta_e` 这一处判断，色板查找
和替代色查找不可能在边界上分家。`Grid` 允许 0×0，产生空 BOM 和零步骤；`Image` 对 0 尺寸
返回 `ImageError::InvalidDimensions`，`fit::plan` 返回 `FitError::InvalidDimensions`。
一律不 panic。

## 判定器的特征与阈值

只看不透明像素：

| 信号 | 定义 | 权重 |
|------|------|------|
| flat neighbours | 相邻（4 邻接中的右、下）不透明像素对里颜色完全相同的比例 | 0.45 |
| palette size | `1 − min(1, 独特色数 ÷ √(w·h))` | 0.35 |
| block period | `detect_grid` 找到大于 1×1 的格子记 1，否则 0 | 0.20 |

加权和 ≥ `DECISION_THRESHOLD`（0.5）判 `PixelArt`，否则 `Photo`。
`confidence` 是加权和到阈值的距离除以阈值，夹到 `0.0..=1.0`。

独特色数除的是像素数的**平方根**而不是像素数，这样 16×16 的十二色 sprite 和 512×512 的
十二色 sprite 读数一样。

两种退化输入是钉死的而不是算出来的（T-CLS-4）：没有不透明像素的图、以及只有一种颜色的
图，都判 `PixelArt { block_size: 1 }`，置信度取 `DEGENERATE_CONFIDENCE`（1.0）。它们不
可能是任何东西的照片，而且在零个像素对上算比例是除零而不是答案。

## 几个需要知道的决定

**CIEDE2000 只吃 Lab。** `ciede2000` 的参数是 `Lab`，不是 `Rgb`。
`docs/bead/fixtures-ciede2000.md` 里那些来自 Sharma、Wu、Dalal (2005) Table I 的数据
本身就是 Lab，容差 `1e-4`。把它们先过一遍色彩空间转换再比对，等于把两份误差预算叠在
一个断言里，坏了也分不清是谁坏的。所以 sRGB→Lab 是另一组测试（`tests/srgb_lab.rs`），
不与那张表混。

**`generic-5mm` 是夹具，不是品牌色卡。** 48 色，厂商中立。没有哪家厂商公布可机读的
Lab 值，照抄别人的色卡只会把别人的误差记在我们头上。这份夹具里最近的一对是 G43 和
G48，相距 ΔE00 5.3575——测试断言这一点，否则后面「ΔE<3 替代色」这条规则在本夹具上就是
空的。真正的品牌色板以后从 `Palette::new` 进来。

**替代色只从余量里挑。** 如果一个颜色本图自己还要用，拿它去顶替缺色只是把缺口搬了个
地方。阈值写在 `bom::SUBSTITUTE_MAX_DELTA_E` 这一个常量里，候选按 ΔE 升序并带上距离，
最后由看着豆子的人拍板。库存里出现与缺色完全相同的另一个色号，就是一条 ΔE=0 的候选排
在最前——「自己其实在库存里」算不算缺，核心函数不替上层判断。

**四种拼装模式都是对非空格的严格划分。** 每个非空格恰好出现一次，空格一个都不出现。
进度条依赖这条性质，所以四种模式各测一遍，还额外跑一百个定种子随机网格（T-SPL-0）。

**parity 夹具是原始 RGBA，不是 PNG。** 浏览器解码时会套 ICC 配置、Rust 的解码器不会，
从压缩图开始的夹具比的其实是两张不同的图（T-PAR-2）。夹具 JSON 里也没有任何浮点数——
一份需要双方对舍入达成一致才能解析的夹具，等于又多了一份没人写下来的契约。

## 用起来

```rust
use bead_core::bom::{check_stock, Inventory};
use bead_core::fit::{BoardSpec, FitMode};
use bead_core::image::Image;
use bead_core::palette::Palette;
use bead_core::pipeline::{to_pattern, PatternOptions};
use bead_core::steps::{plan_steps, ColorOrder, StepMode};

let palette = Palette::generic_5mm();
let options = PatternOptions::new(FitMode::FixedBoards {
    board: BoardSpec::square_28(),
    cols: 2,
    rows: 2,
});

let pattern = to_pattern(&image, &palette, &options)?;
let plan = plan_steps(
    &pattern.grid,
    &palette,
    &StepMode::ColorByColor { order: ColorOrder::AccentFirst },
);
let check = check_stock(&pattern.bom, &palette, &inventory);
```

`Image` 要求像素已经是 8-bit sRGB + alpha（`Image::from_rgba8` 直接吃浏览器
`ImageData` 那种交错字节）。解 PNG / JPEG 不归这个包管——这样它就不用背图像编解码依赖，
WP-B03 也能把 `ImageData` 原样递进来。

## 还没做的（SHOULD，round 2）

- 轻度 JPEG 噪声下的近周期网格仍判 `PixelArt`。现在的 `detect_grid` 要求整列/整行逐像素
  相等，压缩噪声会把边界打散。
- `docs/bead/fixtures-ciede2000.md` 目前是 Sharma Table I 的一个子集，第 34 行（近黑低
  彩度）在 `tests/ciede2000.rs` 里单独内嵌。等文档收全 34 行再合并回去。

## 范围

不进 `apps/desktop`，不碰 `crates/soul-*`，不改根 workspace members、`deny.toml`
或任何 Soul 锁文件。规划见 `docs/bead/PLAN.md`，工作包见
`docs/bead/WORK_PACKAGES.md`，算法契约见 `docs/bead/reviews/round1-algorithms.md`。
