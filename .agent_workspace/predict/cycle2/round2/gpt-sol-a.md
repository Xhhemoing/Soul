MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# C2R2：本机 magic 的 PK/ZIP 家族与「只撤回、不提升」测试规格

## 0. 结论

本轮只收紧 S1，不实现它。结论是：

1. `PK\x03\x04` 不能映射成单一的 `Archive`。对 Soul 当前八桶，它至多提供兼容集合
   `{Archive, Document, Sheet, Slides}`：普通 ZIP、DOCX/ODT/EPUB、XLSX/ODS、
   PPTX/ODP 都可能有相同开头。
2. magic 应当是扩展名计划之后的**减法器**，不是第二个分类器。设扩展名基线已经提出
   kind `E`，magic 的有证据兼容集合为 `C`；仅当 `E ∉ C` 时撤回该 move。`E ∈ C`
   只表示「没有发现桶级矛盾」，不表示格式已经验证。
3. `Unrecognised` 即使命中 PNG/PDF/ZIP magic 也仍不移动。最强的机械性质应是
   `moves_with_magic ⊆ moves_extension_only`；保留下来的 move 的来源、目的地和 kind
   均逐字不变。
4. `Unknown`、`TooShort` 和预算未覆盖不是反证；本轮的 **dispute-only** 规则继续采用
   扩展名基线，并明确标为未验证。若将来要「验不出就不动」，那是另一个
   **verify-or-withhold** 策略，必须另立配置、文案和测试，不能悄悄混入本规则。
5. 只看 PK 前缀不能区分 OOXML；搜索前 512 字节里的 `word/`、`xl/`、`ppt/` 也不是可靠
   证明，普通 ZIP 可以伪造这些 entry name，而合法 OOXML 可以把它们排在窗口之外。
   不解析 ZIP 中央目录、不解压 `[Content_Types].xml` 时，必须保留 `zip_family`。
6. ODF 与 EPUB 是有限的例外：规范特意要求首个 `mimetype` entry 为未压缩、无 extra
   field，因而可以在固定前缀里进一步缩小兼容集合；这仍只是可选的有界规则，不为 OOXML
   打开通用 ZIP 解析。

本槽独立读取约束、当前 `soul-fileplan` 源码与现有测试；未读取 Cycle 2 Round 2 的其他
槽位。未改产品 crate，未进行 git 操作。

## 1. 当前基线与这项实验的准确边界

当前代码已经把「扩展名猜测」与「计划」分开：

- `kind.rs:83-115` 只取最后一段小写扩展名并查固定表；
- 与 ZIP 容器有关的现有扩展名是：

  | 当前 kind | ZIP 家族扩展名 |
  |---|---|
  | `Archive` | `zip` |
  | `Document` | `docx odt epub` |
  | `Sheet` | `xlsx ods` |
  | `Slides` | `pptx odp` |

- `scan.rs:411-430` 只用名称和 metadata 产生 `ScannedEntry.kind`，文档明确承诺不打开文件；
- `plan.rs:211-267` 先排除目录、嵌套、未知和目标占用，最后才产生 move；
- `plan.rs:164-191` 的 canonical JSON 目前没有读取策略、magic 规则版本或内容裁决；
- `tests/no_write_api.rs:33-68` 的 source needle 只防写，不防 `File::open`/`Read`；
  `DirectorySnapshot` 也不含 atime。现有 `disk_unchanged()` 因而不能证明未来 magic 没有打开
  文件。

本机复核 `cargo test -p soul-fileplan`：50 项通过、0 失败。现有公共夹具的正文是
`"jpeg-ish bytes"`、`"pdf-ish bytes"` 等占位字符串，不是有效 magic。dispute-only 下它们
应走 `Unknown -> 保留基线`，但不能拿来证明任何签名命中。

S1 的输出单位必须仍是 **Soul 的八个目标桶**，不是 MIME 精确格式。比如 `.docx` 内容实为
PDF 时，两者都属于 `Document`，目的地仍是 `文档/`；桶级 checker 不应谎称扩展名格式正确，
但也没有理由撤回同一个目标桶。

## 2. 为什么 PK 不是 `Archive`

### 2.1 ZIP 自身就不只有一个开头

PKWARE APPNOTE 规定 local file header signature 为 `0x04034b50`，落盘字节即
`50 4b 03 04`；EOCD 为 `50 4b 05 06`。可观察边界包括：

- 非空 ZIP 通常从 `PK\x03\x04` local header 开始；
- 空 ZIP 可以直接从 `PK\x05\x06` EOCD 开始；
- split/spanned ZIP 可能先有 `PK\x07\x08` marker，再跟 local header；同一字节序列也被
  一些实现用作 data descriptor signature；
- self-extracting ZIP 可以先有可执行 stub，PK 不在 offset 0；
- 只有四个 PK 字节、后续 header 截断或字段不可能的文件，不因此成为有效 ZIP。

所以前缀规则既有假阴性（SFX、规则没覆盖空 ZIP），也有可伪造的假阳性。UI 最多说
「开头符合某条签名」，不能说「已经验证是有效 ZIP」。

### 2.2 同一个 local header 覆盖四个 Soul 桶

| 容器实例 | Soul 桶 | 仅 `PK\x03\x04` 能否区分 |
|---|---|---|
| 普通 `.zip` | `Archive` | 否 |
| `.docx` / `.odt` / `.epub` | `Document` | 否 |
| `.xlsx` / `.ods` | `Sheet` | 否 |
| `.pptx` / `.odp` | `Slides` | 否 |

OOXML 使用 ZIP 物理包；精确区分 Word/Excel/PowerPoint 一般要查看
`[Content_Types].xml`、relationships 或内部 `word/`、`xl/`、`ppt/` parts。它们不保证在
任意小前缀里以可直接搜索的明文出现。把 `word/` 字节搜索当判据还有两个反例：

1. 普通 ZIP 放一个名为 `word/` 的 entry，就能伪装；
2. 合法生成器调整 entry 顺序或 extra field 后，判别 entry 会落到 512 字节之外。

因此本轮不采用「搜索前几个 local header」作为精确 OOXML oracle，也不引入 ZIP
central-directory parser。后者会把固定偏移比较升级成对攻击者可控容器结构的解析，超出
这条有界 magic 实验。

### 2.3 ODF/EPUB 可以有界缩小，但仍兼容 `Archive`

ODF 1.4 要求存在的 `mimetype` 是 ZIP 第一个 entry、STORED、无 extra field；规范说明
`mimetype` 从 offset 30 开始，媒体类型从 offset 38 开始。EPUB OCF 对
`application/epub+zip` 有同型的强制要求。这允许固定前缀规则：

| 固定前缀 media type | 更具体的格式桶 | 与结构桶合并后的兼容集合 |
|---|---|---|
| `application/vnd.oasis.opendocument.text` | `Document` | `{Document, Archive}` |
| `application/vnd.oasis.opendocument.spreadsheet` | `Sheet` | `{Sheet, Archive}` |
| `application/vnd.oasis.opendocument.presentation` | `Slides` | `{Slides, Archive}` |
| `application/epub+zip` | `Document` | `{Document, Archive}` |

集合保留 `Archive` 是刻意的：一个叫 `book.zip` 的 EPUB 确实也是 ZIP 容器；本规则只撤回
明确错误的桶，不替用户决定应按内层语义还是外层容器整理。相反，名为 `book.jpg` 的同一
内容与 `{Document, Archive}` 完全不相交，可以撤回图片 move。

固定前缀缩小必须同时验证 local header 的 compression method、filename length、
extra-field length、entry 名和完整 media type；只在任意位置搜到 `mimetype` 或媒体类型
字符串不算命中。条件不完整时回退 `zip_family`，不能凭半条证据生成精确类型。

## 3. 建议的证据类型与组合规则

不要让 detector 直接返回一个 `FileKind`。需要保留「可能是多个桶」和「没有证据」的差别：

```text
MagicEvidence
  Match {
    rule_id,                 # 稳定、带版本的规则名
    format_id,               # 例如 zip_family / odf_text / pdf
    compatible_kinds,        # 非空集合，不含 Unrecognised
  }
  RecognisedOutsideTaxonomy {
    rule_id, format_id,      # 例如 pem_private_key；是正证据，但没有可移动桶
  }
  Unknown
  TooShort
  NotSniffed { reason }      # mode_off / budget / open_failed / changed
```

`Unknown` 绝不能编码成空集合。空集合的语义是「已识别为当前 taxonomy 之外的格式」，
例如 PEM 私钥；`Unknown` 则是「没有足够证据」。混淆两者会把所有纯文本、短文件和未收录
格式都错误撤回。

令扩展名基线计划为 `B`，其中某条 move 的 kind 为 `E`，规则为：

```text
若 B 没有 move：
    不打开文件；原 leave reason 不变
若 evidence 是 Unknown / TooShort / NotSniffed：
    原 move 不变，显示为 unverified
若 evidence 是 Match(C) 且 E ∈ C：
    原 move 不变，显示为 bucket-compatible
若 evidence 是 Match(C) 且 E ∉ C：
    撤回 move，改为 KindDisputed
若 evidence 是 RecognisedOutsideTaxonomy：
    撤回 move，改为 KindDisputed
```

其中 `zip_family` 的 `C` 必须恰为：

```text
{Archive, Document, Sheet, Slides}
```

不是 `Archive`，也不含 `Image/Audio/Video/Code`。若 detector 同时命中多个可能格式，
`C` 取候选兼容集合的**并集**；只要仍有一种解释与 `E` 相容就不撤回。用交集会把
「尚不能区分」误写成「已经排除」。

`KindDisputed` 的用户文案应是：「扩展名建议放进 X，但有界文件签名与这一类不相容；
本次不建议移动。」不能写「文件其实是 Y」，除非具体规则真的给出了 Y。PK 命中尤其只能
显示 `zip_family`。

## 4. 纯组合器测试：先钉规则，再碰文件 I/O

以下测试只向纯函数传 `extension_kind + MagicEvidence`，不建临时目录：

| ID | 扩展名基线 `E` | evidence | 精确 oracle |
|---|---|---|---|
| W01 | `Image` | `{Image}` | 保留原 move |
| W02 | `Image` | `{Document}` | 撤回，`KindDisputed` |
| W03 | `Document` | `{Archive,Document,Sheet,Slides}` | 保留；只能标 compatible |
| W04 | `Sheet` | 同上 | 保留 |
| W05 | `Slides` | 同上 | 保留 |
| W06 | `Archive` | 同上 | 保留 |
| W07 | `Image/Audio/Video/Code`（参数化） | 同上 | 全部撤回 |
| W08 | `Unrecognised` | `{Image}` | 仍无 move，不得提升 |
| W09 | `Unrecognised` | ZIP family | 仍无 move，不得提升 |
| W10 | 任意可移动 kind | `Unknown` | 原 move 不变，unverified |
| W11 | 任意可移动 kind | `TooShort` | 原 move不变，unverified |
| W12 | 任意可移动 kind | `NotSniffed(budget/open_failed)` | dispute-only 下原 move不变 |
| W13 | `Slides`（`.key`） | `RecognisedOutsideTaxonomy(pem_private_key)` | 撤回 |
| W14 | `Document` | `{Document,Archive}` | 保留，不得改成 `Archive` |
| W15 | `Sheet` | 两个模糊候选 `{Sheet}` 与 `{Document}` | 按并集保留 |

必须再做穷举性质测试，遍历 `FileKind::ALL` 与所有规则集合：

```text
P1  moves_after ⊆ moves_before
P2  retained.from == baseline.from
P3  retained.to   == baseline.to
P4  retained.kind == baseline.kind
P5  baseline 无 move时，任何 evidence 都不能创建 move
P6  相同输入重复运行，decision/rule_id/排序逐字相同
P7  detector 的 evidence 只依赖字节与固定规则版本，不依赖文件扩展名
```

P7 很重要：若 detector 先看 `.docx` 再把 PK 叫作 DOCX，它就不再是独立交叉检查，只是把
扩展名原样回显。

## 5. PK/ZIP 字节夹具矩阵

夹具应是合成、固定字节，并在 manifest 记录长度、SHA-256、生成器版本和预期首字节。
单测直接喂 `&[u8]`；端到端测试才写入临时授权根。不要在测试运行时调用 Office、
LibreOffice、系统 `file` 或联网下载样本。

### 5.1 同一普通非空 ZIP，换名字

用同一份合法、含一个普通 entry 的 ZIP 字节，参数化文件名：

| ID | 文件名 | 扩展名 kind | magic evidence | 最终 oracle |
|---|---|---|---|---|
| PK01 | `plain.zip` | `Archive` | `zip_family` | 保留到 `压缩包/` |
| PK02 | `plain.docx` | `Document` | `zip_family` | 保留到 `文档/`，不得称 DOCX 已验证 |
| PK03 | `plain.xlsx` | `Sheet` | `zip_family` | 保留到 `表格/` |
| PK04 | `plain.pptx` | `Slides` | `zip_family` | 保留到 `演示/` |
| PK05 | `plain.odt/ods/odp/epub` | 各自当前 kind | `zip_family` | 全部保留 |
| PK06 | `plain.jpg` | `Image` | `zip_family` | 撤回，`KindDisputed` |
| PK07 | `plain.mp3` | `Audio` | `zip_family` | 撤回 |
| PK08 | `plain.mp4` | `Video` | `zip_family` | 撤回 |
| PK09 | `plain.rs` | `Code` | `zip_family` | 撤回 |
| PK10 | `plain.dat` | `Unrecognised` | 生产管线不应打开 | 原 `UnrecognisedKind`，不得新增 move |
| PK11 | `plain.key` | `Slides` | `zip_family` | 保留但只标 compatible；PK 单独不能证明或否定 Keynote |

PK02–PK05 是本轮最重要的防回归：任何实现把 PK 硬映射为 `Archive`，都会错误撤回全部
ZIP-based office 文件。

### 5.2 真包、伪线索与顺序变化

| ID | 合成夹具 | 必须得到 |
|---|---|---|
| PK12 | 最小 DOCX、XLSX、PPTX，各自正常扩展名 | 有界非解析 detector 均为 `zip_family`，各自 move 保留 |
| PK13 | 同一最小 DOCX 命名为 `photo.jpg` | `zip_family` 与 `Image` 不相容，撤回 |
| PK14 | DOCX 命名为 `bundle.zip` | 保留 `Archive`；不把目的地改成 `Document` |
| PK15 | 普通 ZIP 只有一个名为 `word/` 的 entry | 仍是 `zip_family`，不得升级为 DOCX |
| PK16 | 普通 ZIP 的 payload 明文含 `xl/`、`ppt/`、`[Content_Types].xml` | 仍是 `zip_family` |
| PK17 | 合法 OOXML 把判别 entry 放到第 5 个或前 512 字节之后 | 仍是 `zip_family`，不得因顺序变 `Unknown` 后声称格式冲突 |
| PK18 | local header 带较长 extra field | 仍只能按边界检查得出的证据处理；不得用固定“第二 entry offset”越界 |

PK15–PK18 共同杀死「在固定小窗口里搜目录名就精确判 OOXML」的方案。

### 5.3 ZIP 边界

| ID | 字节形态 | oracle |
|---|---|---|
| PK19 | 22 字节合法空 ZIP：从 `PK\x05\x06` 开始、零 entry、零 comment | 可命中专门的 `zip_empty -> {Archive}`；`.zip` 保留，`.docx` 撤回 |
| PK20 | `PK\x07\x08` spanning marker 后跟完整 local header | 至多 `zip_family`；`.zip` 保留，`.jpg` 撤回 |
| PK21 | 只有 `PK\x03\x04` 四字节，文件也只有四字节 | `TooShort`，不得声称 ZIP；dispute-only 下原扩展名计划不变 |
| PK22 | 30 字节 local header 声明的 filename/extra 长度超出整个文件 | `TooShort/Unknown`，不得命中 |
| PK23 | `MZ...PK\x03\x04...` self-extracting ZIP | prefix-only 为 `Unknown`；这是显式 coverage gap，不向后扫描任意长度 |
| PK24 | 随机数据在 offset 100 偶遇 `PK\x03\x04` | `Unknown`；不能全缓冲区无界搜索 |
| PK25 | 同时满足两条冲突 magic 的合成 polyglot | 取候选兼容集合并集或 `Unknown`，不得靠规则登记顺序强行选一个 |

PK19 的窄化来自完整 EOCD 明确表示零 entry；若第一版不实现该专门规则，则预注册 oracle
应改为 `Unknown` 并单列 coverage，不能把 `PK\x05\x06` 冒充 `PK\x03\x04`。两种版本都不能
把空 ZIP 误报成已验证 OOXML。

## 6. ODF/EPUB 固定前缀测试

这组只在选择实现 fixed-prefix refinement 时启用；否则四者都合法回落
`zip_family`。一旦启用，就必须同时满足全部结构条件。

| ID | 夹具与文件名 | oracle |
|---|---|---|
| FX01 | 合规 ODT 前缀，`paper.odt` | `{Document,Archive}`，保留 |
| FX02 | 同一 ODT，`paper.docx` | 同属 `Document` 桶，保留但不称 DOCX 正确 |
| FX03 | 同一 ODT，`paper.xlsx` | 与 `Sheet` 不相容，撤回 |
| FX04 | 同一 ODT，`paper.zip` | 因含 `Archive`，保留 |
| FX05 | 同一 ODT，`paper.jpg` | 撤回 |
| FX06 | 合规 ODS，`sheet.ods` / `sheet.odt` | 前者保留，后者撤回 |
| FX07 | 合规 ODP，`deck.odp` / `deck.xlsx` | 前者保留，后者撤回 |
| FX08 | 合规 EPUB，`book.epub` / `book.zip` / `book.jpg` | 前两者保留，图片撤回 |
| FX09 | `mimetype` 被 DEFLATE 压缩 | 不得命中精确 ODF/EPUB；回落 `zip_family` |
| FX10 | local header 有 extra field | 同上 |
| FX11 | `mimetype` 不是第一个 entry | 同上 |
| FX12 | media type 大小写错误、截断、尾随换行或前导空白 | 同上 |
| FX13 | 普通 ZIP payload 偶然含完整 media type 字符串 | 仍是 `zip_family` |
| FX14 | `book.dat` 含合规 EPUB | 仍 `UnrecognisedKind`，不得提升到 Document |

## 7. 端到端计划与 I/O 测试

### 7.1 只打开基线本来会移动的文件

以 injectable opener 记录路径和读取字节数。夹具同时放：

- 顶层、可识别、目标空闲的 4 个 baseline move；
- 未识别 `.dat`；
- 嵌套、already-sorted、destination-taken 各 1；
- 目录和 symlink 各 1。

精确断言：opener 只收到那 4 个 baseline move，其他路径一次也不打开。这样「不提升」不只靠
组合函数，而是由数据流保证：未知文件的字节根本没有机会创建目的地。

### 7.2 move 只能减少

四个 baseline move 中放入：

1. 扩展名与单值 magic 同桶；
2. 扩展名与单值 magic 冲突；
3. 扩展名与 `zip_family` 相容；
4. `Unknown`。

最终恰保留 1、3、4；2 变为 `KindDisputed`。原有目录、嵌套、未知、占用 reason 均不变，
总排序仍按 relative 稳定。再对目录 entry 顺序做全排列，canonical JSON 必须相同。

### 7.3 开关、预算与失败必须可观察

- `magic=off`：open 次数严格为 0，现有 Tree 的 `plan.to_json()` 和 `plan_hash` 与当前基线
  逐字相同。
- `magic=dispute_only,prefix=N,rule_version=V`：这些策略字段进入被哈希值；即使本次没有
  move 被撤回，也不能把「读过正文的计划」冒充成 `off` 计划。
- 到达 open/byte budget 后，其余 candidate 为 `NotSniffed(budget)`，计数进入计划；
  dispute-only 下保留基线，但不得显示为 checked。
- permission error、短读和文件在读前被替换分别产生稳定的 `NotSniffed` reason，不 panic，
  不把旧 buffer 复用于下一文件。
- 同一个 candidate 不得因目录遍历顺序不同而有时被预算覆盖、有时不被覆盖；预算选择应先按
  baseline move 的 canonical path 排序。

`verify-or-withhold` 若未来立项，则预算/open failure/Unknown 都可撤回，但它必须使用另一
`mode` token、另一套 expected moves 和「无法验证，所以不建议移动」文案。不能让同一个
`dispute_only` 名字在两版中改变语义。

### 7.4 授权、竞态与审计

- magic 读取是正文读取，不受当前「目录 metadata 扫描」同意自动覆盖。打开首个 candidate
  之前必须有独立、可见的 opt-in；当前 v0.1 默认仍为 off。
- 读取只能发生在 canonical authorized root 内；扫描后把 candidate 换成 symlink 时，
  opener 必须 no-follow/身份复核，绝不能读到 root 外。
- 读取阶段之后要再取 snapshot；现有 scan 的 `after` snapshot 发生在计划之前，若 magic
  被插在其后，它无法覆盖读取期竞态。
- 审计只记 candidate/opened/matched/disputed/not-sniffed 的整数计数，不记文件名、
  media type 对应文件、原字节、字节摘要或错误正文。
- buffer 有固定上限，退出单文件后清零/释放；网络连接数恒 0，规则库随构建固定，不运行时下载。
- 不再声称 `disk_unchanged()` 证明「没打开正文」；至少报告 `opened_files` 与
  `bytes_read`。atime 是否变化是文件系统策略，不能靠当前 snapshot 看见。

## 8. plan hash 与内容变化测试

magic 一旦影响计划，至少把下列非正文值放进 canonical JSON：

```text
mode + detector_id + detector_version + prefix_limit + total_budget
+ per-candidate decision token/rule_id
+ opened/matched/disputed/not_sniffed counts
```

不放原字节，不把内容 digest 写进审计。测试：

1. 同文件、同字节、同策略重复预览，hash 相等；
2. `off` 与 `dispute_only` 即使 moves 相同，hash 不相等；
3. 把同长度 `.jpg` 内容从有效 JPEG 换成 PDF，并恢复 mtime，move 变
   `KindDisputed`，hash 必须变化；
4. 把 PDF 换成另一份同为 PDF 的同长度内容，桶级 decision 可相同；预览 hash 是否绑定
   exact bytes 要由产品另定，但 v0.1.1 执行前必须在同一身份上重做 magic，不能把旧 verdict
   当成当前文件的证明；
5. rule version 改变时，即使输出偶然相同，hash 也改变；
6. 任一 `KindDisputed` 回到 move 时，旧批准必定因 plan hash 不同而拒绝。

第 4 条把两个目标分开：计划 hash 绑定的是用户批准的动作和观察策略；TOCTOU 安全依靠执行前
对当前文件身份与 verdict 的复核。若选择让 hash 绑定 exact content digest，也只能放在短生命
计划里，不能持久化进无正文审计。

## 9. 直接杀线

出现任一项就否决相应实现：

1. `PK\x03\x04 -> Archive`，导致正常 DOCX/XLSX/PPTX/ODF/EPUB 被撤回；
2. magic 命中让 `.dat`、无扩展名或其他 `Unrecognised` 新增 move；
3. magic 改写已保留 move 的 kind、目标文件夹或文件名；
4. 普通 ZIP 只因 entry 名叫 `word/` 就被判为精确 DOCX；
5. UI 把 `zip_family` 说成「确认是 ZIP/Office 文件」；
6. `Unknown` 与 `RecognisedOutsideTaxonomy` 共用同一空集合；
7. detector 根据扩展名选择 evidence，形成自证循环；
8. `magic=off` 仍打开文件，或使当前 plan JSON/hash 改变；
9. 读取任何非 baseline move、symlink 目标或授权根外路径；
10. 原字节、文件名、逐文件 MIME、摘要进入审计或日志；
11. 同字节同版本结果不稳定，或预算覆盖依赖非确定目录遍历顺序；
12. 为区分 OOXML 偷开通用 ZIP 解压/central-directory 解析而没有新的攻击面、资源上限和授权
    评审。

## 10. 可数性、锁冲突与证伪结论

- **输入：** 已有扩展名 baseline move，以及 opt-in 后每个 baseline candidate 的固定上限
  前缀；不读未识别、嵌套、占用或已分类文件。
- **输出：** 原 move 或 `KindDisputed`；绝不产生新 move。evidence 是稳定 rule ID 与兼容
  kind 集合，不是概率。
- **可数性：** baseline candidates、opened、bytes read、exact matches、family matches、
  disputes、unknown、too-short、budget/open failures，均为整数。
- **与锁冲突：** 打开文件即突破 v0.1 的零正文字节契约，所以默认关闭；本报告不给 v0.1 或
  v0.1.1 自动授权。窗口标题、LLM、网络、执行与持久化均不参与。
- **收益证伪：** 在获准的合成/离线语料上，若 `KindDisputed` 对人工确认错误扩展的 precision
  不足，或 family/unknown 使召回几乎为零，S1 没有足够收益；保持扩展名基线。
- **规则证伪：** 任一测试违反 `moves_after ⊆ moves_before`，或 PK02–PK05 中任一被当作
  `Archive` 冲突，即推翻本规格。

最终建议不是「用 magic 识别真实类型」，而是更窄的：

> 固定、离线、有界的字节证据只可否定与扩展名目标桶明确不相容的 move；PK 是四桶家族，
> 不确定就不声称确定，未知文件永不因内容获得新动作。

## 参考锚点

- [PKWARE ZIP APPNOTE 6.3.9](https://pkware.cachefly.net/webdocs/APPNOTE/APPNOTE-6.3.9.TXT)：
  local header、EOCD、data descriptor 与 spanning marker。
- [ISO/IEC 29500-2:2021（OOXML Open Packaging Conventions）](https://cdn.standards.iteh.ai/samples/77818/11a9edfd37cc4bcc9f0c4128edde32bc/ISO-IEC-29500-2-2021.pdf)：
  OOXML 的 ZIP 物理映射与 media types stream。
- [OASIS OpenDocument 1.4 Part 2: Packages](https://docs.oasis-open.org/office/OpenDocument/os/part2-packages/OpenDocument-v1.4-os-part2-packages.html)：
  首个、未压缩、无 extra field 的 `mimetype` 规则。
- [W3C EPUB OCF 3.2](https://www.w3.org/publishing/epub32/epub-ocf.html)：
  EPUB ZIP container 与固定 `application/epub+zip` 首 entry。
