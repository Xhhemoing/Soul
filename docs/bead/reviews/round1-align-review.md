# Round 1 对齐审查 — 刚合入的 TS 对齐（AT-2 + oracle parity）

- 审查模型：`claude-fable-5-thinking-xhigh`（无降级；只审不改，本 PR 仅新增本文档）
- 被审对象：合并提交 `3779384`（内容提交 `a67afcc` AT-2 修复 + `f9422cf` oracle 对齐），基线 `cursor/beadflow-integration-c441` @ `dc4b91d`
- 对照物：`docs/bead/reviews/round1-algo-ts-review.md`（AT-1/AT-2 原始清单）、`apps/bead/src/algo/contract.md`、`crates/bead-core`（含 `1880487` 的契约吸收）、BD18/BD8/BD14/BD15
- 本机复核：`pnpm --filter @bead/app test` 21 文件 246 全绿（含 `oracle-parity.test.ts` 4×4 组断言）；`cargo test --manifest-path crates/bead-core/Cargo.toml` 全绿（含 `tests/parity.rs` 10 项、fixture 逐字节锁）
- 日期：2026-08-25

## 0. 结论

三问三答：**AT-2 真锁住了；四份 rust fixture 是真对照，不是摆拍；BD18 被遵守（本次对齐零触碰 rust）。** 原审查（round1-algo-ts-review.md §1）说验收核心「结构性不可能成立」——那个结构性障碍已经拆掉：色板真的是同一份 48 色（TS 测试在运行时正则解析 `palette.rs` 逐条比对），fixture 真的是 oracle 写出、rust 侧逐字节锁定、TS 侧原样读入逐格断言。

但**不是 NO_HIGH_VALUE_CHANGE_FOUND**：contract.md 文末的「尚未收敛」表有 3/7 行是过期情报（描述的是 `1880487` 之前的 rust），且 parity 路径上还剩一条真分叉（抖动查表取整）没有任何哨兵护着。清单见 §4。

## 1. AT-2 —— 真锁住了

原 AT-2 要求：按几何直取采样点；钉死前导残格语义；补 3 个 fixture（末格截断、offset≠0、offset≠0+截断）走完整管线断言色号。逐项核对：

- **修法正确**。`framing.ts` 的 `collapseLattice` 直接走检测几何：格边界在 `offset + k·cell`，每个逻辑像素取自己那段 `[start, min(end, span))` 的中点，完全绕开了出错的 `源尺寸/目标尺寸` 比值路径。`pipeline.ts` 的 `collapseUpscale` 已改为调它，`resampleNearest` 保留但只服务显式重采样，不再承担撤销放大。
- **前导残格钉死了**。`latticeCells = (offset > 0 ? 1 : 0) + ⌈(span − offset)/cell⌉`（下限 1）——原审查指出的「x < offsetX 的像素被静默丢弃」已改为残格算一个逻辑像素，contract.md G3 用同一公式写明。
- **三个用例齐了且是全管线**。`pipeline.test.ts` 的「AT-2 截断与相位偏移的放大图」describe 恰好三例：9×4 末格截 1px、11×4 offset=3 末格完整、12×4 offset=3 且末格截断；全部经 `imageToPattern`（aspect 框定 + PixelArt 路径），既断言 `detectedGrid` 几何又断言逐格色号（`[G15, G26, G33(, G22)]`），末格截断例还额外断言三色互异——正是原 bug 的「第二格错取、末格整格丢失」两个症状各有一条断言压着。
- **锁经受住了后续变更**。AT-2 修复（`a67afcc`）先落、色板换 48 色（`f9422cf`）后落，期望色号被一致地更新（红 G08→G15 等），说明这组测试对着行为而不是对着旧数字。

判定：回归锁成立。用比值法回退 `collapseLattice` 或删掉残格分支，这三条测试会立刻红。

## 2. Oracle parity —— 四份 fixture 是真对照

对照链完整，每一环都验过：

1. **fixture 是 oracle 亲手写的**：`bead_core::parity::build` 生成、`examples/emit-parity-fixtures.rs` 落盘，JSON 无浮点（`the_fixture_format_has_no_floating_point_in_it` 断言到逐行）。
2. **rust 侧逐字节锁**：`tests/parity.rs::every_case_matches_its_committed_fixture` 用 `include_str!` 比对生成物与提交物全等——rust 算法一动，fixture diff 必须有人解释。
3. **TS 侧原样消费**：`oracle-parity.test.ts` 直接读 `crates/bead-core/fixtures/parity/*.json`（不复制、不翻译），四个用例（pixel-art / photo-flat / photo-dithered / with-transparency）各跑两遍——一遍按 oracle 顺序直调原语（`planFixedBoards → renderFit → quantize → buildBom`），一遍走完整 `imageToPattern` 门面——逐格比色号、逐行比 BOM（码、名、颗数）。空格用空串对齐 oracle 的拼法。
4. **色板不是「像」而是「是」**：测试在运行时正则解析 `palette.rs` 的 48 条 `("G??", "name", "#HEX")`，断言条数与逐条 id/名/hex 全等。抽查 G01/G08/G15/G25/G48 两侧一致。rust 侧改一条色，TS 这条测试就红——两份源码无法再静默漂移。
5. **近平局防线在位**：生成器拒绝 margin ≤ 1e-6 的 fixture（`no_committed_fixture_rests_on_a_near_tie`），且 `the_margins_are_not_marginal` 把实际下限压到 > 1e-3，libm/Math 的 ulp 级差异翻不了盘。
6. **绕开判定器是对的**：oracle 生成器本来就不经过判定器；TS 测试同样绕开（全管线那遍用 `kind: "Photo"` 强制 oracle 语义路径），注释把理由写明了。比的是同一条管线。

覆盖面上的两个诚实缺口（不推翻结论，记为 §4 的跟进）：fixture 的 `expected.steps`（四种步骤划分，rust 侧文档明说是契约一部分）TS 侧**尚未比对**；box-average × 半透明的组合没有任何 oracle fixture 覆盖（透明例用 nearest，photo 例全不透明），TS 的 `boxAverageCell` 与 `fit.rs::box_average` 目前是逐行目检一致 + 本侧测试锁，不是 parity 锁。

## 3. BD18 —— 遵守

- `git diff 5859e..f9422cf -- crates/` 为空：本次对齐两个提交对 rust 零触碰，「分歧由 TS 向 rust 收敛」的方向被执行——generic-5mm 是 TS 抄 rust 的 48 色，`planFixedBoards`/`renderFit` 是 TS 照 `fit::fixed_boards`/`fit::render` 重写的。
- rust 侧确实变过，但那是 `1880487`（WP-B01 bead-core 吸收，rust 在其职责范围内），发生在本次对齐**之前**且已由 round1-core-review 审过。本次对齐是消费那个新状态，不是伪装成 TS PR 去改 oracle。
- `fixtures/parity.json`（TS 自产回归锁）重新生成并重新定位为「锁 oracle 表达不了的本侧路径」，注释与 contract.md 均如实改口，不再冒充 parity 证据。

## 4. 剩余分叉的处置建议

先说一个必须纠正的事实错误：**contract.md 文末「尚未与 oracle 收敛」表有 3/7 行描述的是 `1880487` 之前的 rust，已过期**——

| 表中行 | 表的说法 | rust 现状 |
|---|---|---|
| 网格提取 | `detect_block_size`：k 须整除两轴、≤ 32、无相位 | `detect::detect_grid`：gcd + 相位，`MAX_CELL_PROBE = 64`，不要求整除 |
| Outline→Infill | 分量按同色区域做 | 分量在非空掩码上 4 邻接，洞 = 边界洪泛不可达，外轮廓 8 邻接判定、优先归类——与 TS 同式，只剩 slug 不同（`inner-edge` vs `inner-border`） |
| tie-break 次键 | `code` 字符串升序 | `ColorId`（色板索引）升序（`bom.rs` 的 `a.id.cmp(&b.id)`），与 TS 同键 |

放着不改，ROUND 2 会照着这张表去「收敛」早已收敛的东西，或者把 TS 对齐到一个不存在的 rust。

### 值得 ROUND 2 动的

| # | 优先级 | 内容 |
|---|--------|------|
| AL-1 | HIGH（文档，便宜） | 修 contract.md 的过期分叉表（上面三行），改成对着 rust 现状写：检测侧真正剩的分叉是退化语义（TS「无周期⇒整幅一格」vs rust「⇒1×1」；TS「周期>64⇒放弃」vs rust「取 ≤64 的最大因子」；透明像素等价约定不同），outline 剩的是 slug，tie-break 已收敛 |
| AL-2 | MED | 抖动查表取整：TS 用未取整小数码值查最近色，rust `to_channel` 先四舍五入到 u8——这是 **parity 路径上仅剩的语义分叉**。photo-dithered 今天全绿靠的是这张 ramp 没踩到决策边界：1e-3 的 margin 下限护得住 ulp，护不住半个码值的扰动，任何新的抖动 fixture 都可能静默翻盘。按 BD18 由 TS 收敛（查表前 `roundHalfUp`，正域上与 rust 的 round 同义），同步改 contract.md G7 并重生成 `fixtures/parity.json` |
| AL-3 | MED | 步骤划分 parity：oracle fixture 已带四种 `expected.steps`，`tests/parity.rs` 的文档把「same four step sequences」写进了契约，TS 侧却只比了 codes + BOM。先按 BD18 把 TS 的 `inner-border` slug 收敛为 `inner-edge`（查清 UI 消费点），再在 `oracle-parity.test.ts` 补 steps 逐组逐格断言。区域/洞/8 邻接语义两侧已同式，剩的就是命名和缺的断言 |

### NO_HIGH_VALUE_CHANGE_FOUND（ROUND 2 不动）

- **判定器公式**（TS 0.5·flat + 0.3·grid + 0.2·color vs rust 0.45·flat + 0.35·√面积色数 + 0.20·block，退化特判与 confidence 归一也不同）：两侧的 parity 都**设计上绕开**判定器，收敛它没有任何 fixture 收益，却会改动产品可见的分类行为并重调一串 TS 测试。留到判定器真要进 parity（或 rust 核要换进产品）那天。
- **产品框定模式**（TS `board`/`aspect`/`manual` vs rust `AspectFit`/`ScaleCrop`）：parity 需要的 `fixed-boards` 已共享。`ScaleCrop` 被 rust 生成器**有意**排除（浮点参数进 fixture 就要两侧约定解析舍入，`parity.rs` 里写着 unreachable）；`AspectFit` 的整板搜索 emitter 虽能表达，但产品側没有消费方。为收敛而收敛是负价值。
- **检测退化语义**（无周期时整幅一格 vs 1×1、>64 的放弃 vs 因子回退、透明像素等价约定）：全在 parity 路径之外（oracle 生成器不撤放大），只影响 TS 独有的像素图前处理边角。在 AL-1 里如实记录成「已知分叉」即可，行为不必动。

## 5. 禁区自查

本审查只新增 `docs/bead/reviews/round1-align-review.md`。未触碰：产品代码（`apps/**` 源码、`crates/**`）、`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、根 `Cargo.toml`、`deny.toml`、lockfile。本文不含外网地址与 schema 声明键；判定器数值一律称 confidence/evidence。
