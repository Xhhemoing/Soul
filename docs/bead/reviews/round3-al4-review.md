`claude-fable-5-thinking-xhigh`

# ROUND 3 · AL-4 rust oracle 边界 fixture · 实现审查

Reviewer：`claude-fable-5-thinking-xhigh`（实际运行 Claude Fable 5 thinking，无静默降级；只审查，不实现）。
被审对象：吸收面 `d34a150`（absorb AL-4，PR #56），内容提交 `9c21a95`（dither-rounding）+
`417e9be`（transparency-box-average），基线 `origin/cursor/beadflow-integration-c441` @ `83818fc`
（B08 复审 #55 合入后）。
审查依据：`docs/bead/reviews/round2-align-al.md` §5 AL-4（原始清单：把 §2 的 125/234 灰对写进
`parity.rs::cases()`，并补 round1 §2 遗留的 box-average × 半透明组合）与 §2（AL-2 留下的缺口：
oracle 四份旧 fixture 全部不踩取整决策边界）、BD18（rust 是 oracle，不得向 TS 收敛）、BD15。
本机复核：`cargo test --manifest-path crates/bead-core/Cargo.toml` = **155 全绿**（12 个 target
分项 30+19+6+13+19+11+12+8+11+7+18+1，与父代理门禁 155 命中；对基线 153 恰好 +2，即两条新测试）。
另做两次破坏性反证（改完即还原，未入任何提交，细节见 §2/§3）。hosted Bead CI 仍是计费空
runner，非产品失败，不计入。日期：2026-08-26。

范围对账：`git diff 83818fc..d34a150` 全部落在 5 个文件——两份新 fixture JSON、
`crates/bead-core/src/parity.rs`（+134，仅新增两个 case 构造器、两个 `pub` 常量与文档注释）、
`crates/bead-core/tests/parity.rs`（+189，两条新测试 + 既有断言扩到六案）、父代理记账
`docs/agent-progress.md`（吸收提交 `d34a150` 独享）。`apps/**`、`crates/bead-core/src/` 其余
七个模块、Soul 树、`.github/**` 全部零字节差。

---

## 0. 结论先行

**ACCEPT。0 条 HIGH，0 条 MED，3 条 LOW（全部一行级打磨，无一需要行动）。** 任务钉出的三条
硬红线全部核清（§1）；两个新 case 都被破坏性反证证明是**真锁**而非摆拍：把 `to_channel` 退成
纯夹取重生成，`dither-rounding.json` 第二格 G01→G02 且分歧沿误差扩散重排大半张网格，
`transparency-box-average.json` 的 (0,2) 格 G32→G33（§2）；把 box 滤波的 alpha 均值改成先取整
再比阈值重生成，**只有** `transparency-box-average.json` 变化——2/4 覆盖格从空翻成珠子（§3）。
round2-align-al §5 AL-4 开出的两张单子（125/234 灰对 + box-average × 半透明）逐字兑现，且
125/234 正是 TS 侧 `dither.test.ts` AL-2 哨兵用的同一对灰值——跨语言的同一道岔现在两侧各有
一把锁。不重开架构。

## 1. 任务钉出的硬红线（逐条对树验）

1. **rust 算法未向 TS 重写（BD18）✅。** `git diff 83818fc..d34a150 -- crates/bead-core/src/`
   只命中 `parity.rs`——它是 fixture 案例定义与 JSON 发射器，不是算法；diff 内容只有两个新
   case 构造器（`dither_rounding_case` / `transparency_box_average_case`）、两个灰值常量和
   `cases()` 尾部追加两项。`color.rs` / `quantize.rs` / `fit.rs` / `detect.rs` / `steps.rs` /
   `bom.rs` / `palette.rs` 零字节差，`parity.rs` 既有的 `build` / `json_string` /
   `NEAR_TIE_MARGIN` 原样未动。算法语义没有一处被触碰。
2. **`apps/bead` 未改 ✅。** 全 diff 文件清单无任何 `apps/**` 路径。TS 侧也不会被新文件牵连：
   `oracle-parity.test.ts` 用写死的四元 `CASES` 数组按名读文件（48 行），不扫描目录，新增的两份
   JSON 对 TS 套件是不可见的（接线是后续刀，见 §6）。
3. **新 case 真踩取整边界 ✅（反证实测，非仅读注释）。** 在隔离工作树把
   `color::to_channel` 的 `value.round().clamp(0.0, 255.0) as u8` 改成
   `value.clamp(0.0, 255.0) as u8`（纯夹取，截断代替取整）后跑 emit example 重生成六份
   fixture：`dither-rounding.json` 与 `transparency-box-average.json` **双双改写**（各 8 行），
   `pixel-art` / `photo-flat` / `with-transparency` 逐字节不变——这同时复核了 round2 §2 的前提
   「旧 fixture 不锁查表取整」。`photo-dithered.json` 在这个 rust 侧实验里也变了 44 行，但那是
   经 `box_average` 颜色压缩里的 `to_channel` 传进来的；**查表取整这条道岔被单独隔离的只有
   `dither-rounding`**：它 `Sampling::Nearest` + 全不透明，管线里除了查表前的 `to_channel`
   再无第二处取整，注释「Nothing sits between the input and the rounding」经源码核实为真，
   `the_fixture_set_covers_the_paths_the_review_names` 还把这一点断言进了测试
   （sampling 必须是 Nearest，否则钉住的是滤波器的取整而不是查表的）。

## 2. dither-rounding —— 125/234 灰对，算术逐位复核

- **道岔算术独立重算无误。** 种子 125 灰最近色 G08 Slate `#6B7A85` = (107, 122, 133)，残差
  (18, 3, −8)；Floyd–Steinberg 右邻权重 7/16 落到 234 灰上：234 + (7.875, 1.3125, −3.5) =
  **(241.875, 235.3125, 230.5)**。取整（round half away from zero）→ (242, 235, 231) → 最近色
  **G01 White**；纯夹取截断 → (241, 235, 230) → **G02 Cream**。230.5 恰好悬在半码值上——这正是
  round2 §2 记录的那格，也与 TS `dither.test.ts:147-161` 的 AL-2 哨兵逐数吻合（同一对
  125/234、同一组 carried 期望值）。§5 AL-4 的「把 §2 的 125/234 灰对写进 cases()」逐字兑现。
- **提交的网格站在取整一侧。** `dither-rounding.json` 的 codes[1]（cell (1,0)）= "G01"；
  新测试 `the_dither_ramp_turns_on_how_the_lookup_rounds` 不止对拍——它在测试体内**徒手重走**
  整条分岔（断言 seed→G08、kernel[0] = (1,0,7/16)、carried = [241.875, 235.3125, 230.5]、
  取整侧 G01、截断侧 G02），最后才断言提交网格取了取整分支。任何一环漂移都单独可见。
- **反证：分歧真的扩散。** 纯夹取补丁下重生成，第二格 G01→G02 之后两条路径放置不同残差，
  36 格里绝大多数重排（G08 种子外几乎整张网格换位），BOM 与四份 steps 全部跟着变——注释里
  「the disagreement runs through half the grid」只算保守陈述。锁不是恰好在一格上成立的巧合。
- **构造干净。** 6×6 全不透明、一格一源像素（Nearest）、双轴 ramp（列 +2 / 行 −3）数值域
  217..242 无越界；fixture 源字节与构造器逐位对得上（行 0：125, 234, 236, 238, 240, 242；
  行 1 起于 232−3y）。

## 3. transparency-box-average —— 洞过 box 滤波，两半规则都吃重

- **补的正是缺的组合。** 旧四案里 `with-transparency` 走 Nearest（每格继承单像素 alpha，
  「部分覆盖的格子」从未出现），两个 photo 案走 box 但全不透明。新案 12×12 八角环 + BoxAverage
  + 开抖动，`RING_MASK` 逐块核验：每格盖 2×2 源块，覆盖数只出现 **0 / 2 / 3 / 4**（无 1，
  与注释声明一致；顺手抽查 (1,0)=3→G37、(2,0)=4→G38、(0,2)=4→G32、四角 0→空，均与提交
  codes 吻合）。
- **alpha 半码值悬空是真的（反证实测）。** `fit.rs::box_average` 用**未取整的 f64 均值**比
  128 阈：2/4 覆盖 = 127.5 < 128 → 空。把这一行改成先 `to_channel` 再比（127.5 会进位到
  128）重生成：**六份 fixture 只有本案变化**——2/4 格全部从 "" 翻成珠子（codes 第 9、10、11、
  27、28、33、35 等位补进 G37/G13/G04/G15/G26/G25/G41/G28/G24），25 行改写。也就是说这条
  阈值语义此前**没有任何 fixture 锁着**，现在有且仅有这一份锁。
- **颜色只取不透明样本、且过取整。** 3/4 格的颜色 = 三个不透明样本线性光均值再
  `to_channel`；§1 的纯夹取反证里本案 (0,2) 格 G32 Light Blue→G33 Blue，证明 box 颜色路径的
  取整在本案里同样吃重（透明角一旦掺进颜色也会立刻翻码）。新测试
  `the_box_filtered_hole_straddles_the_alpha_threshold` 自己重跑 `fit::plan` + `render`，
  逐格断言「不透明 ⇔ 覆盖 ≥ 3/4」、断言 2/4 与 3/4 两侧都有格子存在（防 mask 改画后退化成
  只有空和满），再对 (0,1) 这个 3/4 格逐通道复算线性光均值。断言面覆盖了注释声称的每一条。
- **抖动 × 空格纪律顺带上锁。** 本案开 Floyd–Steinberg，`map_dithered` 的空格「不放珠、不发
  残差、来者即止」路径第一次在 box 滤波产生的洞上走 parity（`with-transparency` 只在 Nearest
  洞上走过）。

## 4. 测试与 fixture 质量

- **既有断言全部扩到六案**：`CASE_NAMES` 常量统一 `cases()` 顺序断言、`committed()` 编译期
  include、无浮点格式检查（两份新 JSON 纯整数、`}\n` 收尾）循环全改用它；
  `the_fixture_set_covers_the_paths_the_review_names` 对两个新案各钉 sampling + dither +
  「必须有洞」。emit example 自动带出六份文件（实测输出六行 wrote）。
- **155 = 153 + 2** 对账成立：新增恰好两条测试，无删无改他人断言。基线与两次反证还原后
  各跑一遍全绿，工作树 `git status` 干净。
- 三条 LOW（记录即可，不要求行动）：
  1. `the_box_filtered_hole_straddles_the_alpha_threshold` 末尾的
     `assert!(palette.nearest_rgba(cell).is_some())` 近乎恒真（前一行已断言 a = 255），是
     装饰性断言。
  2. `seen` 直方图只断言 `seen[2] > 0 && seen[3] > 0`，没顺手断 `seen[1] == 0`——mask 注释
     声称块覆盖只有 0/2/3/4，这一声明本身没进断言面（本审查已人工核验为真）。
  3. `dither_rounding_case` 注释「runs through half the grid」偏保守（实测几乎全网格重排）；
     方向是低估不是夸大，无害。

## 5. 纪律项

- **BD18**：见 §1.1。两个内容提交对 `crates/bead-core/src/` 只碰 `parity.rs` 案例定义；
  测试新 import（`srgb_compress` / `srgb_expand` / `to_channel` / `Sampling` /
  `FLOYD_STEINBERG_KERNEL`）全是既有 `pub` 项，无为测试开洞的可见性改动。
- **BD15**：`git diff 83818fc..417e9be` grep `://` 零命中（两个内容提交无任何 URL 字面量）。
  吸收提交 `d34a150` 在 `docs/agent-progress.md` 加了一行 `cursor.com/agents/...` 链接——与该
  账本既有每行同款，属父代理记账惯例，非产品代码。
- **禁区**：全 diff 不出 `crates/bead-core/**` + `docs/agent-progress.md`。`apps/desktop`、
  `crates/soul-*`、根 `Cargo.toml`、`deny.toml`、`.github/**`、`docs/PRODUCT_LOCK.md` 等
  零触碰。`bead-core` 自身仍是独立 workspace root，未挂进 Soul 的 members。

## 6. 跟进项

### 新增 HIGH / MED

无。

### 值得排期的非 HIGH（下一刀）

| # | 优先级 | 内容 |
|---|--------|------|
| AL-5 | MED | 把 TS `oracle-parity.test.ts` 的 `CASES` 从四案扩到六案。oracle 侧的锁已落地，但跨语言对拍要等 TS 消费这两份 JSON 才闭环（`run()` 已支持 nearest/box-average 与 fixed-boards，预期是纯接线）。父代理进度账里「TS 侧跟 oracle 新 case 后置」与此同项 |

### NO_HIGH_VALUE_CHANGE_FOUND（不动）

- §4 的三条 LOW：装饰性断言、`seen[1]` 未断零、注释保守措辞——单开提交都是噪音，下次路过顺手。
- `ROUNDING_SEED_GREY` / `ROUNDING_RAMP_GREY` 作为 `pub` 常量暴露：测试要引用，fixture crate
  的 API 面扩两个 u8 无碍。

## 7. 禁区自查

本审查只新增 `docs/bead/reviews/round3-al4-review.md` 一个文件。§2/§3 的两次破坏性反证
（`color.rs` 纯夹取、`fit.rs` alpha 先取整）均在隔离工作树临时改动、重生成 fixture 观测 diff
后即 `git checkout` 还原，未进任何提交；还原后复跑 `cargo test` 155 全绿、`git status` 干净
才收尾。子代理 `gh` 只读，未创建 PR（BLOCKED_PR）。本文无 schema 声明键；判定语一律称
confidence/evidence。
