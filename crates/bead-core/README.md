# bead-core

BeadFlow 的算法源（WP-B01）。一张图片进来，一份色号清单和四套拼装指引出去。

这个包**不在**仓库根 `Cargo.toml` 的 `[workspace].members` 里，它自己就是一个
workspace 根（`Cargo.toml` 里那张空的 `[workspace]` 表就是干这个的）。这样 Soul 的
`deny.toml`、`e0-audit` 和 `just ci` 不必去理解一张它们并不发布的依赖图。

零依赖：全部是 std 集合上的算术。不开 socket，不碰文件。

## 怎么跑

从仓库根目录：

```bash
cargo test  --manifest-path crates/bead-core/Cargo.toml
cargo clippy --manifest-path crates/bead-core/Cargo.toml --all-targets -- -D warnings
cargo fmt   --manifest-path crates/bead-core/Cargo.toml --all -- --check
```

或者 `cd crates/bead-core` 之后直接 `cargo test`。根目录的
`cargo test --workspace` 跑不到这里——那是刻意的。

## 里面有什么

| 模块 | 负责 |
|------|------|
| `color` | sRGB→Lab（D65, 2°）、CIEDE2000 |
| `palette` | 色板、最近色查找、`generic-5mm` 夹具 |
| `grid` | 矩形格子与 `Cell` 坐标 |
| `image` | 已解码的 8-bit sRGB 栅格 |
| `detect` | 像素图 / 照片判定 |
| `fit` | 板型框定与重采样 |
| `quantize` | 像素→色号，可选 Floyd–Steinberg |
| `steps` | 四种拼装顺序 |
| `bom` | 备料清单、库存比对、ΔE 替代色 |
| `pipeline` | 把上面接起来，给默认值 |

## 几个需要知道的决定

**CIEDE2000 只吃 Lab。** `ciede2000` 的参数是 `Lab`，不是 `Rgb`。
`docs/bead/fixtures-ciede2000.md` 里那 20 组来自 Sharma、Wu、Dalal (2005) Table I 的
数据本身就是 Lab，容差 `1e-4`。把它们先过一遍色彩空间转换再比对，等于把两份误差预算
叠在一个断言里，坏了也分不清是谁坏的。所以 sRGB→Lab 是另一组测试
（`tests/srgb_lab.rs`），不与那张表混。

**`generic-5mm` 是夹具，不是品牌色卡。** 48 色，厂商中立。没有哪家厂商公布可机读的
Lab 值，照抄别人的色卡只会把别人的误差记在我们头上。这份夹具里任意两色的 CIEDE2000
距离都大于 3——测试会断言这一点，否则后面「ΔE<3 替代色」这条规则就是空的。真正的品牌
色板以后从 `Palette::new` 进来。

**顺序都是全序。** 这个包是 WP-B03 浏览器实现的对照标准，同一张图必须得到同一串色号。
所以每个排序都有决胜键，同分时按色号（`code`）比，不靠哈希遍历顺序。

**替代色只从余量里挑。** 如果一个颜色本图自己还要用，拿它去顶替缺色只是把缺口搬了个
地方。阈值写在 `bom::SUBSTITUTE_MAX_DELTA_E` 这一个常量里，候选按 ΔE 升序并带上距离，
最后由看着豆子的人拍板。

**四种拼装模式都是对格子的严格划分。** 每个可放置的格子恰好出现一次。进度条依赖这条
性质，所以四种模式各测一遍，而不是只测一次。

## 用起来

```rust
use bead_core::bom::{check_stock, Inventory};
use bead_core::fit::{BoardSpec, FitMode};
use bead_core::image::Image;
use bead_core::palette::Palette;
use bead_core::pipeline::{to_pattern, PatternOptions};
use bead_core::steps::{plan_steps, ColorOrder, StepMode, StepOptions};

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
    &StepOptions::default(),
);
let check = check_stock(&pattern.bom, &palette, &inventory);
```

`Image` 要求像素已经是 8-bit sRGB。解 PNG / JPEG 不归这个包管——这样它就不用背图像
编解码依赖，WP-B03 也能把浏览器的 `ImageData` 原样递进来。

## 范围

不进 `apps/desktop`，不碰 `crates/soul-*`，不改根 workspace members、`deny.toml`
或任何 Soul 锁文件。规划见 `docs/bead/PLAN.md`，工作包见
`docs/bead/WORK_PACKAGES.md`。
