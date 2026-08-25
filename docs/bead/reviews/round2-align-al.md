# Round 2 对齐审查 — AL-1/2/3（contract 表修正 + 抖动取整 + 步骤划分 parity）

- 审查模型：`claude-fable-5-thinking-xhigh`（无降级；只审不改，本 PR 仅新增本文档）
- 被审对象：合并提交 `331b513`（内容提交 `81184be` AL-1 + `c0d346f` AL-2 + `f0e6a21` AL-3），基线 `cursor/beadflow-integration-c441` @ `e0ff662`
- 对照物：`docs/bead/reviews/round1-align-review.md` §4（AL-1/2/3 原始清单）、`crates/bead-core`（`detect.rs` / `steps.rs` / `bom.rs` / `color.rs` / `quantize.rs` / `fit.rs` / `parity.rs`，BD8/BD18 oracle）、`apps/bead/src/algo/contract.md`、BD15
- 本机复核：`pnpm --filter @bead/app test` **22 文件 258 全绿**（含新增的 AL-2 用例 2 条与 AL-3 的 `it.each` 4 条）；与 `6174981` 进度记录的 258 一致
- 另做两次破坏性反证（改完即还原，未入库）：回退 AL-2 取整 → 7 红；回退 AL-3 slug → 3 红。细节见 §2/§3
- 日期：2026-08-25

## 0. 结论

**三项全部通过（PASS），无残留 HIGH，无新增 HIGH。** round1-align-review §4 开出的三张单子逐条兑现：过期分叉表按 rust 现状重写且句句核对得上源码；抖动查表在 parity 路径上仅剩的那条语义分叉已按 BD18 由 TS 收敛到 `color::to_channel`，且新测试真的踩在旧路径会翻盘的那一格上；`inner-edge` 改名彻底（两棵树 grep 零残留），四份 oracle plan 的 steps 断言把区域顺序、洞判定、8 邻接优先和 slug 一起锁死。BD18 被遵守（`git diff e0ff662 331b513 -- crates/` 为空），BD15 无新增 URL 字面量。

一个如实记录的边界（非 HIGH）：oracle 的 `photo-dithered` fixture 本身仍没踩到取整决策边界——把 `toChannel` 回退成纯夹取，`oracle-parity.test.ts` 依然全绿，红的是 TS 侧哨兵（`dither.test.ts` AL-2 用例 + `fixtures/parity.json` 两条开抖动用例，共 7 红）。也就是说 G7 取整这条收敛的**跨语言**锁还欠一条真踩边界的 oracle fixture，见 §5 的 AL-4。

## 1. AL-1 —— contract.md 分叉表：修对了

原单要求：把「网格提取 / Outline→Infill / tie-break」三行过期情报改成对着 `1880487` 之后的 rust 写。逐句对源码核验：

- **网格提取行**。rust 列的每个断言都对得上 `detect.rs`：主干 gcd + 相位（`period_of` 对 `position − first` 连续取 gcd）、`MAX_CELL_PROBE = 64`、不要求整除 ✔；「变化位置 < 2 ⇒ 该轴 1 像素一格」= `changes.len() < 2 → (1, 0)` ✔；「周期 > 64 ⇒ 取 ≤ 64 的最大因子而不是放弃」= `largest_divisor_at_most(spacing, MAX_CELL_PROBE.min(length))` ✔；「透明像素一律等价、只比不透明 RGB」= `column_equal`/`row_equal` 比 `Rgba::rgb()`（`Option<Rgb>`，alpha < 128 一律 `None`，None == None）✔。TS 列同样属实：`periodAndPhase` 在 `edges.length < 2` 或 `period > MAX_DETECTED_CELL` 时返回整轴一格，`samePixel` 逐字节比 RGBA 四通道。
- **Outline→Infill 行被正确地移出表**（AL-3 落地后并入「已收敛」段），tie-break 行删除并改写成「已经收敛、不要再收敛一次」：`bom.rs` 的 `a.id.cmp(&b.id)`、`steps.rs::color_by_color` 的 `a_id.cmp(b_id)` 都是 `ColorId`（色板索引）升序，确非 `code` 字符串序 ✔。
- **剩下的行恰好是 align-review 点名不动的东西**：判定器权重（TS 0.5/0.3/0.2 vs rust 0.45/0.35/0.20，均核对过 `classify.ts` 与 `detect.rs::analyze`）、产品框定（表里补注了「parity 只走已共享的 fixed-boards」，与 `parity.rs` 的 `ScaleCrop` unreachable、`AspectFit` 无用例一致）、检测退化语义。没有新的 parity 路径分叉被这张表藏起来。
- 一处措辞小瑕（不扣分）：rust 列「变化位置 < 2 ⇒ 该轴 1 像素一格（`GridGeometry::NONE`）」——`NONE` 是两轴常量，单轴的准确说法是 `period_of` 返回 `(1, 0)`。语义无误，只是括号引用偏松。

判定：通过。ROUND 2 及以后照这张表干活不会再对着不存在的 rust 收敛。

## 2. AL-2 —— 抖动查表取整：收敛真实，锁咬得动

- **修法与 oracle 同式**。`dither.ts` 新增 `toChannel(v) = clamp(roundHalfUp(v), 0, 255)`，仅用于查最近色前；误差仍按未取整 f64 累计并以未取整值算残差（`dr = rawR − chosen.r`）——与 `quantize.rs::map_dithered` 完全同构：rust 也是 `to_channel(current)` 只喂查表、残差用 `current − placed`。近平局余量两侧同基：TS 用取整后的 `lookup` 进 `rgbToLab`，rust 用 `clamped.to_lab()` 进 `decision_margin`。
- **「round 半值远离零 = roundHalfUp（夹取后）」的注释属实**：两者仅在负半整数（−0.5、−1.5…）上不同，而这些值两侧都夹到 0。可达域内唯一的真差异是 NaN（rust `to_channel` 显式回 0，TS `clamp(NaN)` 会漏出 NaN）——抖动路径输入是有限 u8 加有限权重，NaN 不可达，记观察不记账。
- **新测试踩在真边界上**。第一条用例直接双查证明 (241.875, 235.3125, 230.5) 小数查表得 G02、取整成 (242, 235, 231) 得 G01；第二条走完整 `quantize` 断言 `["G08", "G01"]`。**反证**：本机把 `toChannel` 回退为纯 `clamp` 后重跑——恰好 7 红：AL-2 管线用例（第二格回到 G02）+ `parity.test.ts` 的 photo-dither / transparent-hole 各 3 条（色号、步骤、余量）。旧浮点路径确实会立刻被抓。
- **fixture 重生成是诚实的**。`c0d346f` 对 `fixtures/parity.json` 的全部 diff hunk 落在 707–2585 行之间——`photo-dither`（678 起）与 `transparent-hole`（1131 起）两个真正开抖动的 case；`pixel-art-upscaled-4x`（PixelArt 路径强制关抖动，`ditherApplied: false`）与 `photo-no-dither` 逐字节未动。余量从 0.0093/0.0020 升到 0.0466/0.0065，且 `parity.test.ts` 用 `toBeCloseTo(…, 10)` 把记录值钉在活管线输出上，摆拍不可能过。
- **G7 文档同步改口**，contract.md 分叉表的「抖动查表」行移入「已收敛」段。

判定：通过。唯一的余项：oracle 的 `photo-dithered` fixture 未踩取整边界，旧路径下 `oracle-parity.test.ts` 依然全绿——本条收敛的跨语言证据要等一条真踩边界的 oracle fixture（§5 AL-4），当前由 TS 侧哨兵持锁，方向（TS 已与 rust 同式）没有疑问。

## 3. AL-3 —— slug 改名 + steps 逐组断言：锁住了

- **改名彻底**。两棵树 grep `inner-border` 零命中；`inner-edge` 出现在 `steps.ts`（类型 + emit）、`steps.test.ts`、contract.md G4、TS `fixtures/parity.json`，与 `steps.rs::Phase::slug` 同名。产品侧确无消费点：`splitSteps`/`step.part` 在 `apps/bead/src` 的 algo 之外只有 `index.ts` 的 barrel 导出，components/pages/stores 零引用——「改动只落在 algo」的说法核实。
- **标签重建逐个对上 rust 的 `StepGroup::label` 格式**：
  - color-by-color：`${id} ${displayName} ×${count}` = rust `"{code} {name} ×{len}"`（`describe` + `format!`，含同一个 `×` 字符）；TS 默认 ascending = rust `AccentFirst`，平局同为色板索引升序。
  - tile：`Board 28x28 r{row}c{col}`，板名与 `fit.rs::BoardSpec::square_28()`（`"28x28"`）核对一致；**row/col 从组首格坐标反推**（`floor(first.x / 28) + 1`）而不是用组序号——rust 空板被 `plan_steps` 过滤后组序号会错位，坐标反推是唯一正确做法，这条要求做对了。板名写死 square_28 有注释自认，且未来若 oracle 换板会当场红，属自警而非隐患。
  - outline-infill：`Region ${group + 1} ${part}`——TS 组号是行优先扫描的分量序号（0 起），rust `region_number` 同一扫描序（1 起），同构；slug 经 `step.part` 进标签，一漂移即红。
  - row-by-row：`Row ${group + 1}`——TS `rowByRow` 的 `group` 存**行号 y** 而非过滤后序号，与 rust「按 y+1 命名、空行丢弃」的语义严格一致（当前四条 fixture 虽无空行，语义仍是对的）。
- **断言是全序全等**：先钉 `expected.steps` 的 mode 顺序 = `parity_step_modes` 顺序 = `SPLIT_MODES`，再对每份 plan 逐组比 `{label, cells}`，cells 是 `[x, y]` 有序数组的 `toEqual`——组序、格序、坐标一个都跑不掉。`with-transparency` 确有 `Region 1 inner-edge` 组（12 格），洞语义被真实覆盖。
- **反证**：本机把 `emit("inner-edge", …)` 改回 `inner-border` 重跑——3 红：`oracle-parity.test.ts` 的 with-transparency steps、`parity.test.ts` 的 transparent-hole、`steps.test.ts` T-OUT-2。提交信息「slug 一旦漂回去立刻红」属实，且是三道独立的锁。
- 本提交对 `fixtures/parity.json` 只动了 2 行（slug 字符串本身），诚实。

判定：通过。round1 §2 指出的「`expected.steps` TS 侧尚未比对」缺口关闭。

## 4. 纪律项

- **BD18**：`git diff e0ff662 331b513 -- crates/` 为空——三个提交零触碰 rust，全部由 TS 向 oracle 收敛。oracle fixture（`crates/bead-core/fixtures/parity/*.json`）只读未改。
- **BD15**：三个 AL diff grep `://` 零命中。全树扫描 `apps/bead` + `crates/bead-core` 的命中仅 `isolation.test.ts` 里故意拆开的扫描器测试样本（`${"http"}${"s://"}` 之类，拼不成完整字面量），docs/bead 命中全是既有审查文档转述允许名单——无新增外网地址。
- **禁区**：三个提交只动 `apps/bead/src/algo/**` 与其 fixture/contract.md，未碰 `apps/desktop`、`crates/soul-*`、`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、根 `Cargo.toml`、`deny.toml`。`6174981`/`e24b812` 仅动 `docs/agent-progress.md`。

## 5. 跟进项

### 新增 HIGH

无。

### 值得排期的非 HIGH

| # | 优先级 | 内容 |
|---|--------|------|
| AL-4 | MED（rust 侧，属 core 工作包不属 TS PR） | 给 oracle 补真踩边界的 parity case：一条抖动 ramp 刻意穿过取整决策边界（把 §2 的 125/234 灰对写进 `parity.rs::cases()` 即可），顺手补 round1 §2 遗留的 box-average × 半透明组合。落地后 G7 取整与 box-average 洞语义才有跨语言锁，当前只有 TS 侧哨兵 |

### NO_HIGH_VALUE_CHANGE_FOUND（不动）

- **判定器公式**（0.5/0.3/0.2 vs 0.45/0.35/0.20，退化 confidence 0.5 vs 1.0）：仍双双绕开 parity，理由同 round1 §4，未变。
- **产品框定模式**（`board`/`aspect`/`manual` vs `AspectFit`/`ScaleCrop`）：parity 所需 `fixed-boards` 已共享，`ScaleCrop` 被生成器有意排除，未变。
- **检测退化语义**（整幅一格 vs 1×1、>64 放弃 vs 因子回退、RGBA 四字节 vs 不透明 RGB）：全在 parity 路径外，AL-1 已如实记录，行为不必动。
- **`toChannel` 的 NaN 分支**（rust 回 0，TS 漏 NaN）：抖动路径不可达，加防御分支属给不可能发生的场景写代码。
- **tile/row 组的 `color` 元数据**（rust 算 `single_color`，TS 恒 null）：不在 label/cells 断言面上，TS 产品侧无消费点，收敛无收益。
- **contract.md 的 `GridGeometry::NONE` 括号措辞**（§1 末的瑕）：下次顺手改，单开提交是噪音。

## 6. 禁区自查

本审查只新增 `docs/bead/reviews/round2-align-al.md`。§2/§3 的两次破坏性反证均在工作区临时改动、跑完即 `git checkout` 还原，未进任何提交（`git status` 复核干净后才重跑 258 全绿收尾）。本文不含外网地址与 schema 声明键；判定器数值一律称 confidence/evidence。
