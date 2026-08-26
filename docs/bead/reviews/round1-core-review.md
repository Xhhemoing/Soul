# ROUND 1 · bead-core 实现审查（WP-B01）

- 审查模型：`claude-fable-5-thinking-xhigh`（实际运行 slug，无静默降级；只审不改，未动任何产品代码）
- 审查对象：`crates/bead-core/**`（O1 交付，已合入专属线，merge `4b8702d` / PR #23）
- 基线：`origin/cursor/beadflow-integration-c441` @ `a388cd7`
- 对照物：`docs/bead/reviews/round1-algorithms.md`（G1–G8 / T-*）、`docs/bead/fixtures-ciede2000.md`、`docs/bead/reviews/round1-cicd.md`（WS/E0/DL）、BD15/BD17、`apps/bead/src/algo/contract.md`（O3 已合入的 G1–G8 钉死文）
- 日期：2026-08-25

父代理注：本审查基线是 `a388cd7`。其后 `1880487` 已给 rust 补上 RGBA、线性光 188 与 `fixtures/parity`。CR-1 里「Rust 无 alpha / 锁 128 / cargo 不读 parity」对**当前**专属线不再成立。仍以 BD8/BD18 为准：TS 对齐 rust，不反写 core。

## 0. 结论

**实现是真的，隔离是对的，门禁全绿；但专属线现在同时载着两份互相矛盾的 G1–G8 契约。**

- 八项 WP-B01 功能全部有真实实现，无占位、无桩：110 个测试本机全绿，fmt/clippy `-D warnings` 干净，`e0-audit` / `denylist-audit` 均 `clean`（§4 有命令与输出）。
- 未误进根 workspace：根 `Cargo.toml` 零改动，crate 自带空 `[workspace]` + 已提交 `Cargo.lock`（v4，单包）。依赖数为零，比 WS-3 建议的 serde 两件套还少。
- **主发现（CR-1，ROUND 2 必须先解决）**：O1（bead-core，先合入）与 O3（`apps/bead/src/algo`，后合入）是并行开发的。O3 的 `contract.md` 把 G1–G8 逐条钉死并写明「Rust 侧照抄即可，不要各自发明」，但它合入时 bead-core 已经按**另一组同样被 F3 允许、但不相同的选择**合进来了，无人做对账。结果：两侧在 G1（透明）、G3（网格相位）、G4（Outline 语义）、G5（重采样空间）、G8（空网格）与三种框定模式的**语义全部分叉**，`fixtures/parity.json` 只有 vitest 在消费（它自己注释里写着「WP-B01 has to consume too」），`cargo test` 从未读过它——**B03 的验收标准「同一 fixture 得到同一色号序列」目前不可验证，且按现状对账必炸**。
- 次发现：审查钉的 MUST 测试里有一批 T-* 在 Rust 侧缺失或只算部分覆盖（§3 逐条），其中抖动的冻结 fixture（T-FS-2/3）与替代色边界锁（T-SUB-1）是纯测试债，加测试即可，不用改实现。
- 过度设计只有一处值得点名：serpentine 抖动变体（§5）。

## 1. 真正实现核实（逐模块）

每个模块都读过全文源码，不是转述 PR 描述。

| 模块 | 核实结论 |
|------|----------|
| `color` | **真**。sRGB 解码分段（0.04045/12.92/1.055^2.4）、矩阵 7 位小数与 contract.md 逐位相同、D65 白点 0.95047/1/1.08883、ε=216/24389 与 κ=24389/27 按有理数写。CIEDE2000 覆盖 Sharma 三个不连续分支：C'₁C'₂=0 ⇒ Δh'=0、Δh' 过 ±180° 回卷、h̄' 过 0/360 缝合线（`sum<360` 分支）；`hue_angle` 对 a=b=0 钉 0。G 因子、R_T、S_L/S_C/S_H 齐全。20 组表值 1e-4 全过 |
| `palette` | **真**。`generic-5mm` 48 色厂商中立 fixture；`nearest_lab` 严格 `<` ⇒ 平局取最小索引（G2 一致）；`within` 严格 `< max_delta_e`（G8 的 `<3` 语义在代码层面成立）；重复 code、空色板、>u16 均类型化拒绝 |
| `grid` | **真**。行优先、`neighbours4`、类型化 `GridError`。**注意：0×0 被拒绝**（G8 分叉点，见 §2.5） |
| `image` | **真**。仅 8-bit sRGB，**无 alpha 通道**（G1 分叉点，见 §2.1）；解码归浏览器，符合 F1 §4 B01.1（不引 `image` crate） |
| `detect` | **真**规则启发式：块周期（2..=32 探测取最大）、平邻比、独特色/√面积，加权 0.45/0.35/0.20 过 0.5 判 PixelArt；`confidence` 归一化到 [0,1]。无外部视觉 API，无 socket。**无相位（offset）检测**（G3 分叉点，见 §2.3） |
| `fit` | **真**。三模式齐：FixedBoards（cover 居中裁）、AspectFit（整板 cols×rows 枚举，log 空间比 aspect，平局取更多豆）、ScaleCrop（cover 相对 zoom+pan，pan 夹取不报错）。`render` 手写 Nearest / box 平均，无 `drawImage` 类依赖。**box 平均在 sRGB 码值空间**（G5 分叉点，见 §2.2） |
| `quantize` | **真**。FS 核 7/16、3/16、5/16、1/16，误差 f64 全图缓冲、不提前夹取、查表前夹 [0,255]、越界丢弃——与 contract.md G7 逐条一致。另带 serpentine 变体（§5） |
| `steps` | **真**。四模式都是严格划分（空组丢弃、skip 色不产生步骤、`assert_partitions` 逐模式验证）。**Outline→Infill 是「同色分量 + 同心圈」语义，不是 contract.md 的「非空掩码 + 洞」语义**（G4 分叉点，见 §2.4） |
| `bom` | **真**。计数、颗数降序 + code 升序、`check_stock` 只从余量里挑替代（把「自身在库存」的量先扣掉——合理且已文档化）、严格 `< 3` |
| `pipeline` | **真**。detect → plan → render → map → BOM 串联，PixelArt 默认 Nearest+不抖，Photo 默认 BoxAverage+FS，可覆盖 |

无为凑行数的重命名或空抽象。**「真正实现」这一项通过。**

## 2. 主发现：两份契约在专属线上并存（CR-1）

时间线（`--first-parent`）：`7d5f12e`（壳 review）→ **`4b8702d`（WP-B01 bead-core）** → `4928162` → **`e94e6c5`（WP-B03 TS 管线）** → `384bc3d` → SH-1..4。O1 与 O3 在各自分支并行，两侧对 F3 留了自由度的缺口各自拍板，合入时没有互检。F3 的原话是「实现者可另选，但必须文档化 + fixture 锁定」——**两侧都文档化了，也都用自己的测试锁定了，但锁的不是同一组选择**。逐条分叉：

### 2.1 G1 透明像素 —— 最深的分叉

- TS（contract.md）：`alpha ≥ 128` 不透明，否则 `Grid` 里是 `null`；空格不进 BOM、不进步骤、不接收也不中转抖动误差。parity fixture 是每像素 8 个十六进制字符的 **RGBA**。
- Rust：`Image::from_rgb8` 每像素 **3 字节，没有 alpha**；`Grid<ColorId>` 是稠密的，透明用 `StepOptions::skip`（跳过某些色号）在**步骤层**表达。后果：
  - `Bom::from_grid` 没有 skip 参数，**被跳过的「背景色」照样进 BOM** ——用户会被要求为永远不摆的格子买豆子；「全透明 ⇒ 空 BOM」（T-BOM-2）在 Rust 侧不可表达；
  - 抖动对「透明格」没有概念，背景格照常接收/中转误差，违反 G7 的透明隔离条款（T-FS-7 无法写）；
  - **parity fixture 的 RGBA 格式 Rust 连解析都解析不了**（还没到语义分叉那一步，输入格式就进不去）。

### 2.2 G5 重采样空间 —— 有两个测试各自锁死了相反的答案

- TS：线性光平均，2×1 [黑,白] → 1×1 = **188**；contract.md 原话「如果哪天看到 128，说明有人改成了 sRGB 码值空间平均」。
- Rust：`box_average` 对 u8 码值求和取平均，`the_two_samplers_answer_differently_on_fine_detail` 断言黑白棋盘 → **(128,128,128)**。
- 两个选择 F3 都允许，但 F3 同时要求「必须两侧一致并用 T-SCL-5 锁死」。现在是两侧各自用 T-SCL-5 风格的测试锁死了**相反**的选择。走照片路径的任何 fixture，色号序列从第一格就不同。

### 2.3 G3 网格提取

- TS：`detectGrid` 公开，返回 `{cellWidth, cellHeight, offsetX, offsetY}`，用变化列 gcd 推周期、支持相位。
- Rust：`detect_block_size` 只探 `k` 整除两维且 0 对齐的平块，**无 offset**，带 3px 相位的 8× 放大图会退化为 1。T-GRID-1 的 offset 半边在 Rust 侧缺失，且这一限制未在 README 声明。

### 2.4 G4 Outline→Infill

- TS：非空掩码上 CCL、洞 = 外部洪泛不可达、外轮廓 8 邻接外部、内边界 8 邻接洞、外轮廓优先——即 F3 的推荐语义。
- Rust：**同色**分量（4 邻接）、外轮廓 = 4 邻接触非本分量、内边界 = 紧贴外轮廓的**第二圈同心环**、其余是填充。这是另一种自洽、对 WP 字面（「连通分量，外轮廓 → 内边界 → 填充」）同样说得通的读法，代码文档也写了理由（infill 需要 edge 依靠）。但与 TS 产出的步骤序列**完全不同**，且 Rust 侧没有环形挖洞 fixture（T-OUT-2 缺）。

### 2.5 G8 与框定模式

- TS：`Grid` 允许 0×0 / 全透明（空 BOM、零步骤）；三种框定是 `board`（整幅重采到 28/56，不裁剪）、`aspect`（最长边 = maxSide，另一边比例 roundHalfUp）、`manual`（crop 越界补透明、完全越界报 `CropOutOfBounds`）。
- Rust：`Grid` 拒绝 0×0（类型化错误）；三种框定是 FixedBoards（cover **居中裁剪**）、AspectFit（**整板拼接** cols×rows）、ScaleCrop（pan **夹取、永不报错**）。`parity.json` 里 `{"mode":"aspect","maxSide":7}` 这样的参数在 Rust API 里没有对应物。T-SCL-2 按 F3 字面（100×50→28×14）两侧都没实现——TS 是 maxSide 语义，Rust 是整板语义，**三方各一套**。

### 2.6 排序次键（轻微）

Rust 的 Color-by-Color / BOM / 替代色平局次键是 **code 字符串**（README 已文档化），TS 契约是**色板索引**（替代色是库存下标）。对 `generic-5mm`（G01–G48）两者恰好等价，换任意真实品牌色板（code 非字典序）即分叉。属于埋雷，不是现症。

### CR-1 的处置建议

1. **先拍一份唯一契约**。建议以 `apps/bead/src/algo/contract.md` 为准：它与 F3 的推荐选择几乎逐条重合（alpha≥128、线性光、掩码 CCL + 洞、detectGrid 带相位、0×0 合法），且已有 217 个 TS 测试和带哨兵余量的 parity fixture 锁定。反向（让 TS 照抄 Rust）要重写的面更大，还要放弃已冻结的 fixture。若父代理另拍，以决议为准，但**必须二选一**。
2. bead-core 按契约补齐：RGBA 入口 + `Option<ColorId>`（或等价空格哨兵）、线性光 box 平均、掩码 CCL 语义、`detect_grid` 相位、0×0 合法化；把「Rust 是 oracle」落实为 **cargo test 消费同一份 parity fixture**（放 `crates/bead-core/tests/fixtures/`，路径天然豁免两支审计；零依赖包需要一个手写 JSON 读取器或加 `serde_json` 并按 WS-3 给书面理由——fixture 格式简单，手写更省）。
3. 对账前**冻结双方**对 G1/G3/G4/G5/G8 相关代码的进一步改动，避免对着移动靶修。

## 3. T-* 缺口清单（对照 round1-algorithms.md 的 MUST）

「覆盖」= 有等价断言，不苛求 fixture 逐字相同。

| T-* | 状态 | 说明 |
|------|------|------|
| T-DE-1 | **覆盖** | 20 组全嵌入，1e-4；缝合线组另列。**但 #34（近黑低 C）夹具未列，WP 要求「标 TODO，禁止编造」——没编造（对），也没标 TODO（缺）** |
| T-DE-2 / T-DE-3 | 覆盖 | 全表对称 + 自反（含灰轴） |
| T-SRGB-1 | 覆盖 | 白/黑/三原色/中灰锚点，白点 a\*/b\* 按矩阵舍入放宽并文档化 |
| T-PAL-1 | 覆盖 | 48 色恒等，ΔE < 1e-9 |
| T-PAL-2 | 覆盖 | 同 RGB 双条目取低索引（用 0/1 位而非 3/7，等价） |
| T-PAL-3 | **缺** | 无固定种子 ≥1000 随机 RGB 的 property 测试 |
| T-FS-1 | 部分 | 4 像素 fixture 上验证了「不抖 = 逐像素独立」，无通用等价断言 |
| T-FS-2 | **缺** | 无 2×2 灰 + 黑白板的手推棋盘冻结 fixture |
| T-FS-3 | **缺** | 无 1×8 灰阶梯冻结序列（最右误差丢弃无显式用例） |
| T-FS-4 | 部分 | 越界跳过逻辑在实现里，但 1×1 / 1×N / N×1 无专门抖动测试 |
| T-FS-5 | 覆盖 | 黑白硬边 16×16，输出全在色板内 |
| T-FS-6 | 部分 | 有「抖动均值更近原色」的方向性断言，无 ≤2% 定量界 |
| T-FS-7 | **缺（结构性）** | 无透明概念 ⇒ 无法写；随 CR-1 解决 |
| T-CLS-1 | 部分 | 16×16 sprite ×2/4/8，confidence 断言 >0.5（F3 要求 ≥0.8 的用例是 24×24×12 色 ×8） |
| T-CLS-2 | 部分 | 64×64 双向渐变（4096 独特色）判 Photo；非 256×256/≥10000 |
| T-CLS-3 | 部分 | 1×1 无除零、confidence 有界；多像素单色图与全透明图未测（后者结构性缺） |
| T-CLS-4 | **缺** | 单色图判定结果未钉死（现实现会给 PixelArt 满置信，但没有测试锁它） |
| T-GRID-1 | **缺（半）** | 周期检测有测试（取最大 k），offset 检测不存在 |
| T-SCL-1 | 覆盖 | 块均匀恒等（box 4×4→2×2；nearest 112×112→28×28 逐格） |
| T-SCL-2 | **分叉** | 按 F3 字面两侧都没做；Rust 是整板语义并自锁（§2.5），随 CR-1 拍板 |
| T-SCL-3 | 分叉→部分 | pan 夹取语义自锁；「完全越界报错」被夹取设计消掉了，与 TS 的 `CropOutOfBounds` 不一致 |
| T-SCL-4 | 覆盖 | scale ≤0/NaN、零板、零源全类型化，无 panic |
| T-SCL-5 | **分叉** | 已锁 128（sRGB 码值），TS 锁 188（线性光）——见 §2.2 |
| T-SPL-0 | 部分 | 5 模式 × 固定 fixture（含 skip 变体）有划分断言；无 ≥100 随机网格 property |
| T-SPL-1 | 分叉→部分 | 全 skip → 零步骤已测；0×0 被 Grid 拒绝（§2.5） |
| T-SPL-2 | 部分 | 管线与抖动双跑全等已测；plan_steps 本身无双跑断言（纯函数，风险低） |
| T-CBC-1 | 覆盖 | 升/降序 + 同计数平局双向测试（次键是 code，§2.6） |
| T-CBC-2 | 部分 | 单色一步成立；组内行优先格序无显式断言（实现是行优先） |
| T-TIL-1 / T-TIL-2 | 覆盖 | 56×56 四板顺序 + 5×3/4×4 残板 + skip 板不产步骤（经划分断言间接锁定） |
| T-OUT-1 | 覆盖（等价） | 6×6 全色 → 20/12/4 同心圈 |
| T-OUT-2 | **缺** | 无环形挖洞 fixture；且语义本身待 CR-1 拍板 |
| T-OUT-3 / T-OUT-4 | 覆盖 | 对角不连通（4 邻接）+ 分量按阅读序 |
| T-ROW-1 | 覆盖 | 行序/行内序显式断言；全 skip 行经空组过滤 + 划分断言锁定 |
| T-BOM-1 | 覆盖 | 冻结行集 + Σ颗数 = 784 逐色复核 |
| T-BOM-2 | 部分 | 单色一行成立；空/全透明空 BOM 结构性缺（§2.1） |
| T-BOM-3 | 覆盖 | 同颗数平局 fixture（次键 code，§2.6） |
| T-SUB-1 | **缺** | `<` 严格性在代码里，但无 Sharma #2 入选 / #3 排除的边界锁，无 ≈2.99/3.01 对 |
| T-SUB-2 | 覆盖 | ΔE 升序 windows 断言 + 阈值处全排除断言 |
| T-SUB-3 | 覆盖 | 空列表非错误；余量语义（自身用量先扣）已测并文档化 |
| T-PAR-1/2/3 | **缺（Rust 侧全缺）** | `parity.json` 存在且 TS 侧带哨兵余量消费；bead-core 无 fixtures 目录、无消费测试。CR-1 的核心 |

**纯测试债（不动实现即可补）**：T-PAL-3、T-FS-2、T-FS-3、T-FS-4、T-FS-6、T-CLS-4、T-SUB-1、T-SPL-0 的 property 化、#34 的 TODO 标注。
**随 CR-1 才有意义**：T-FS-7、T-BOM-2 空份、T-GRID-1 offset、T-OUT-2、T-SCL-2/3/5、T-PAR-*。

## 4. denylist / e0 / workspace 隔离（对照 round1-cicd §1）

本机全部实跑（Rust 1.83.0，`rust-toolchain.toml` 生效）：

```text
cargo fmt   --manifest-path crates/bead-core/Cargo.toml --all -- --check   # 干净
cargo clippy --manifest-path crates/bead-core/Cargo.toml --all-targets -- -D warnings   # 0 警告
cargo test  --manifest-path crates/bead-core/Cargo.toml   # 110 passed, 0 failed（11 个二进制目标）
cargo run -p xtask -- e0-audit        # 14 crates walked, 276 files scanned → clean
cargo run -p xtask -- denylist-audit  # 94 terms, 127 files scanned → clean
```

| 项 | 结论 |
|----|------|
| WS-1 | ✅ 空 `[workspace]` 在位，注释写明动机；根 `Cargo.toml` 无 bead-core 字样（grep 验证零命中） |
| WS-2 | ✅ `Cargo.lock` 已提交（v4，仅 bead-core 一个包） |
| WS-3 | ✅ **零依赖**，无 HTTP client、无 `image`。注意：CR-1 的 fixture 消费若选 `serde_json` 需按 WS-3 给书面理由，手写解析则维持零依赖 |
| WS-4 | ✅ 不依赖任何 `soul-*`，无反向依赖；`deny.toml` 零改动 |
| WS-5 | ⚠️ 部分：`docs/agent-progress.md` 只记录了 `cargo test` 绿，fmt/clippy 两条没留记录（本审查已补跑，全绿） |
| WS-6 | ✅ 1.83 / edition 2021 实测编译通过；`rust-version = "1.83"` 已写入 |
| E0-1 | ✅ `Cargo.toml` 无 `repository`/`homepage`/`documentation`，无任何 URL |
| E0-2 | ✅ `src/**` 与 `tests/**` grep `http/https/://` 零命中——Sharma 出处只写论文题名（.rs 注释里无链接），链接类出处全在 `.md` |
| E0-3 | n/a：尚无 golden fixture 目录（CR-1 落地时按此条放 `fixtures/`/`tests/`） |
| DL-1 | ✅ 置信度叫 `confidence`；全 crate 无任何拆词含 s-c-o-r-e 的标识符 |
| DL-2 | ✅ `src/**` 字符串里无 CJK 诊断词（源码为全英文文案） |
| DL-3 | ✅ `src/` 内联 `#[cfg(test)]` 模块存在但不含违禁词（denylist-audit clean 佐证） |

BD17 复核：bead-core 里没有任何 URL 字面量（注释也没有），也没有从 bead ref dispatch Soul CI 的迹象。**「denylist/e0/误进根 workspace」三项全部通过。**

## 5. 过度设计核查

- **serpentine 抖动（`Dither::FloydSteinbergSerpentine`）——唯一实锤**。WP-B01 只要求「可开关」的经典 FS；F3 的 G7 建议经典 raster；O3 的 contract.md 明文「不做 serpentine」且 TS 侧没有实现。Rust 侧多出的这个变体默认不启用、管线自动路径也不会选中它，但它是公开 API：壳一旦把它暴露成选项，TS 无对应实现，parity 当场破。**建议随 CR-1 移除或降级为 `#[doc(hidden)]` + 明确「非契约、禁止跨侧使用」标注。**
- 其余体量克制：`Xyz`/`WHITE_D65` 公开是测试与文档需要；`check_stock_within` 是把阈值参数化（WP-B05 会用）；`Inventory` 是 WP-B01 第 8 条的必需输入；`pipeline` 只做默认值粘合。没有多余抽象层、没有 trait 泛化癖、零依赖。**除 serpentine 外无过度设计。**

## 6. ROUND 2 验收清单（本审查的输出门）

- [ ] **CR-1** 唯一契约拍板并写进一处（建议 contract.md 升为两侧共同契约，或抬进 `docs/bead/`）；bead-core 按 §2 六个分叉点对齐；两侧各自与旧选择绑定的测试（Rust 的 128 断言、Grid 0×0 拒绝等）同步改
- [ ] **CR-2** `cargo test` 消费与 vitest 同一份 parity fixture（同一文件或双份 + 校验和测试），色号序列逐元素全等；oracle 侧带 T-PAR-3 近平局哨兵断言
- [ ] **CR-3** serpentine 移除或标注为非契约 API
- [ ] **CR-4** 纯测试债补齐：T-PAL-3、T-FS-2/3/4/6、T-CLS-4、T-SUB-1、T-SPL-0 property 化；`ciede2000.rs` 给 Sharma #34 标 TODO（引 WP 原话，禁止编造数值）
- [ ] **CR-5** 排序次键统一为色板索引（或契约明文改为 code 并让 TS 跟随），并加一个 code 非字典序的测试色板 fixture 锁死
- [ ] **CR-6** WS-5 三连（fmt/clippy/test）今后每个 bead-core PR 完整留痕，不只留 test

## 7. 禁区自查

本审查只新增 `docs/bead/reviews/round1-core-review.md`。未触碰：`crates/bead-core/**`（审而不改）、`apps/bead/**`、`docs/PRODUCT_LOCK.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/STATUS.md`、`apps/desktop/**`、`crates/soul-*/**`、根 `Cargo.toml`、`deny.toml`、`.github/workflows/**`。无产品代码改动。
