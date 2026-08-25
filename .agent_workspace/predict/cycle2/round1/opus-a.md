MODEL_SLUG: claude-opus-5-thinking-high-fast

# Cycle 2 / Round 1 / opus-a —— fileplan 现行算法的逐条记录，与三条可测后继

角色：**只读调研**。本轮零产品 crate 改动、零 docs 改动、**零 git 操作**，写入面恰为本文件。
读取时点：HEAD `2b0b555`，工作区除 `.agent_workspace/predict/`（未跟踪）外干净。
遵 `predict/CONSTRAINTS.md` 与 `cycle2/CONSTRAINTS.md`：不重开 P0–P8、不实现 Goal 2、
不把候选算法写进产品 crate。

指定读物是 `crates/soul-fileplan/src/{plan.rs,kind.rs}`。为了把「算法吃什么」说准，
也读了同 crate 的 `scan.rs`（`ProposedMove`/`LeftAlone` 的每一个字段都来自
`ScannedEntry`）、`preview.rs`（`plan_hash` 的消费者）、`execute.rs`（`PlanHashMismatch`
的判法）、`screen.rs`（哪些名字根本进不了 `plan::build`）与 `tests/`（今天钉住的行为）。
后四者只用于交叉验证，不作为本文主体。

---

## 0. 一句话结论

现行算法是一张 **69 项扩展名 → 8 类 → 8 个中文文件夹** 的常量表，加上 `build()` 里
**五级短路判定**，全程纯函数、零 I/O、O(F×E) 字符串比较。它的可信度不来自分类准确，
而来自**它拒绝知道的东西**：不开文件、不猜未知扩展名、不动目录、不跨子目录。

但有两处「承诺与检查不对齐」，任何后继算法必须先面对：

1. **`tests/no_write_api.rs` 证明的是「不写」，不是「不读」。** 它的 `WRITES` needle 表里
   有 `File::create`/`OpenOptions`/`io::Write`，**没有 `File::open`、`fs::read`、`io::Read`**。
   `scan.rs` 文档写着「It does not open anything」，这句话今天**没有任何测试兜底**。
2. **`DirectorySnapshot` 哈希 name/type/len/mtime，不含 atime。** 于是一个只读正文的组件
   可以在 `disk_unchanged() == true` 的情况下改遍全目录的 atime——**「盘没动」这个证明恰好
   不覆盖读正文这个动作的唯一足迹**。

这两条决定了 §5 的排序：**S3（近因）零新增 I/O、零新增冲突，S2 的默认档同样零读，
S1（幻数）必须先补这两个洞才谈得上做**。三者都不需要读第三人正文即可交付其默认行为。

---

## 1. 算法在管线里的位置

```
preview()                                    preview.rs
 ├─ HITL check_action(scan.directory)        ← 与目录无关的门
 ├─ scan::scan(authorization, raw, limits)   ← 唯一碰盘的一步，只读目录项
 │   ├─ screen_segment(name)  失败 → SkippedEntry(UnplannableName)，**不进 entries**
 │   ├─ symlink_metadata      symlink → SkippedEntry(Symlink)，**不进 entries**
 │   └─ FileKind::of_extension(extension_of(name))  ← kind.rs 在这里被调用，不在 plan.rs
 ├─ plan::build(&scan)                       ← 本文主体：纯函数，只读 scan
 └─ HITL check_action(plan.files, plan.to_json())  → PlanHash
```

三个位置关系值得先记住，后面反复用到：

- **分类发生在 `scan`，不在 `plan`。** `ScannedEntry.kind` 在走目录时就定了，`plan::build`
  只读 `entry.kind()`。任何改分类的后继算法（S1）改的是 `scan` 的一步，不是 `plan` 的一步。
- **`plan::build` 不接收时钟、不接收配置、不接收随机源。** 签名是
  `fn build(scan: &DirectoryScan) -> OrganizePlan`。它的输出完全由 `scan` 决定，
  这是 `plan_hash` 可复现的**全部**理由。S3 想引入「最近」，第一件事就是打破这条（见 §5.3）。
- **`scan.entries()` 已按 `relative` 升序排好**（`scan.rs:447`），`build` 依赖但不重申这一点；
  它自己在末尾又排了一次（`plan.rs:269-270`），所以即使 scan 的排序被改掉，plan 仍是确定的。

---

## 2. 扩展名表（`kind.rs`）

### 2.1 名字 → 扩展名：`extension_of`

```rust
let (stem, extension) = name.rsplit_once('.')?;   // 最后一个点
if stem.is_empty() || extension.is_empty() { return None; }
Some(extension.to_lowercase())
```

四条可观察后果：

| 输入 | 输出 | 说明 |
|---|---|---|
| `photo.JPG` | `Some("jpg")` | `to_lowercase()` 是 **Unicode** 小写，不是 `to_ascii_lowercase` |
| `archive.tar.gz` | `Some("gz")` | `rsplit_once` 只看最后一段，**复合扩展名不存在**；`.tar.gz` 与 `.gz` 同类（都 Archive），但 `.tar.bz2`→`bz2` 也仍是 Archive，所以今天这条没造成错分 |
| `.gitignore` | `None` | stem 为空。注释明说「A leading dot is not an extension」 |
| `notes.` | `None` | extension 为空。且这个名字**根本到不了这里**——`screen_segment` 的 `TrailingDotOrSpace` 会先把它整条 skip 掉 |

**同 crate 里有两套点号解析规则**：`extension_of` 用 `rsplit_once('.')`（取最后一段），
`screen_segment` 用 `split('.').next()`（取第一段，用于比对保留设备名）。
两者服务不同目的且都正确，但后继算法若要统一「文件名结构」的概念，得知道这是两处。

### 2.2 扩展名 → 类别：`of_extension`（69 项，8 类）

| 类别 | 文件夹 | 项数 | 扩展名 |
|---|---|---|---|
| `Image` | `图片` | 10 | jpg jpeg png gif bmp webp heic tif tiff svg |
| `Document` | `文档` | 8 | pdf doc docx odt rtf txt md epub |
| `Sheet` | `表格` | 5 | xls xlsx ods csv tsv |
| `Slides` | `演示` | 4 | ppt pptx odp key |
| `Archive` | `压缩包` | 8 | zip rar 7z gz bz2 xz tar iso |
| `Audio` | `音频` | 7 | mp3 wav flac aac ogg m4a wma |
| `Video` | `视频` | 7 | mp4 mkv mov avi wmv webm m4v |
| `Code` | `代码` | 20 | rs py ts tsx js jsx java go c h cpp hpp cs sh ps1 sql json toml yaml yml |
| `Unrecognised` | **`None`** | — | 其余一切。`folder()` 返回 `None` ⇒ 永不移动 |

匹配是**精确字符串相等**，无前缀、无通配、无回退。表是 `match` 字面量，编译期常量。

### 2.3 表本身的六个观察（**不是缺陷清单，是后继算法的输入**）

**T1 —— `key` → `Slides` 是一个真实的误分类源。** `.key` 是 Keynote，也是私钥、许可证、
授权文件的常见后缀。`id_rsa.key`、`license.key`、`server.key` 今天会被提议移进 `演示/`。
这是全表里**唯一一个把敏感文件送进错误文件夹**的条目，而且 v0.1.1 一旦可移动就会真的发生。

**T2 —— `ts` 同时是 TypeScript 与 MPEG-TS。** 表选了 `Code`。一个下载的 `.ts` 视频分片
会进 `代码/`。与 T1 同型，但后果轻。

**T3 —— `Code` 一类的取舍不自洽。** 有 `json`/`toml`/`yaml`/`yml`（数据/配置），
**没有 `html`/`css`/`xml`**（同样常见，同样是文本源码）。也没有 `rb php swift kt lua r dart vue`。
这不影响正确性（未知即不动），但意味着「代码」这一类在不同用户目录上的召回率差异极大。

**T4 —— 缺口集中在三处**：相机 raw（`cr2 nef arw dng raw`）、Apple 原生
（`pages numbers heif avif`）、安装包/磁盘映像（`dmg pkg deb rpm msi apk exe`，
`iso` 在但 `dmg` 不在）。还有 `tgz`——`archive.tgz` 因为 `rsplit_once` 取到 `tgz` 而
**落进 `Unrecognised`**，尽管 `archive.tar.gz` 是 `Archive`。同一个压缩包，两种写法两种结局。

**T5 —— `svg` → `Image` 是唯一一个「按用途分类而非按格式分类」的条目。** SVG 是 XML 文本，
可含脚本。放 `图片` 符合用户直觉，但它和 T3 的 `html` 缺席构成一个不一致：一个 XML 方言在图片里，
另一个 XML 方言不在任何类里。

**T6 —— 文件夹名就是类别身份。** `for_folder` 拿中文字面量做精确匹配，是
`AlreadySorted` 判定的**唯一**依据（`plan.rs:230`）。三个后果：
(a) 用户已有的 `Pictures/` 不被认作图片文件夹，会多出一个 `图片/`；
(b) 用户把 `图片` 改名为 `照片`，**所有已归类的文件下次扫描立刻变成 `Nested`**；
(c) 类别身份不可本地化——换语言等于换算法。`FileKind` 另有
`#[serde(rename_all = "snake_case")]` 与手写的 `as_str()` 两套稳定 wire 词
（今天一致，`to_json` 用的是 `as_str()`），而**文件夹名没有对应的稳定 id**。

---

## 3. 留下不动的五个理由（`plan.rs`）

`LeaveReason` 五个变体，各有 `as_str()`（进哈希）与 `explanation()`（给用户看，中文）。

| 变体 | `as_str()` | 用户看到的话 | 触发条件（精确） |
|---|---|---|---|
| `Directory` | `directory` | 这是目录，本版本不动目录 | `is_dir && depth == 1` |
| `AlreadySorted` | `already_sorted` | 已经在它该在的分类文件夹里 | 文件 && `depth == 2` && 首段是某类文件夹名 && 该类 == 本文件的类 |
| `Nested` | `nested` | 在子目录里，本版本只整理散在最外层的文件 | 文件 && `depth > 1` && 不满足上一条 |
| `UnrecognisedKind` | `unrecognised_kind` | 认不出这是哪一类，不猜 | 文件 && `depth == 1` && `kind.folder() == None` |
| `DestinationTaken` | `destination_taken` | 目标文件夹里已经有同名的东西 | 文件 && `depth == 1` && 有类 && 目标路径已被占 |

`LeaveReason` derive 了 `PartialOrd/Ord`，序即声明序，被用作 `left_alone` 排序的次键
（`plan.rs:270`）。**后继若加变体必须追加在末尾**，否则改的是排序语义而不只是加了个值。
（次键实际不可达——`relative` 在同一次扫描里唯一——但它进了排序表达式，改序仍是语义改动。）

---

## 4. `build()` 的五级判定，逐条

判定是**短路的**，顺序即优先级。对 `scan.entries()`（已升序）逐项：

```
1. is_dir?          → depth==1 ⇒ Directory ；depth>1 ⇒ 【什么都不产出】       continue
2. depth > 1?       → AlreadySorted / Nested                                  continue
3. folder()==None?  → UnrecognisedKind                                        continue
4. 目标被占?        → DestinationTaken                                        continue
5. 否则             → ProposedMove{ from, to="{folder}/{relative}", kind, size }
```

末尾：`moves` 按 `from_relative` 升序、`left_alone` 按 `(relative, reason)` 升序。
两处都是 Rust `String` 的字节序（UTF-8 码元序），跨平台一致。

### 4.1 第 1 级：深层目录**从计划里消失**

`depth > 1` 的目录既不产 move 也不产 `LeftAlone`——`continue` 掉了。于是：

> **`moves.len() + left_alone.len()` 不等于 `scanned_entries`。**

差额恰是深层目录的个数。今天的 fixture（`tests/common`）里没有 depth≥2 的目录，
12 = 4 + 8 正好对上，所以 `everything_the_plan_leaves_alone_says_why` 是过的。
在 `Alpha/sub/` 下建一个子目录，等式立刻不成立。

这本身不算错（父目录已经以 `Directory` 的名义被列出，孙目录被它覆盖，注释 `plan.rs:215-216`
就是这么说的），但它意味着**计划里的数字自己不闭合**，UI 若拿这三个数做「N 项已全部说明」
的断言就会说谎。

### 4.2 第 2 级：`AlreadySorted` 只认深度恰好为 2

```rust
relative.split_once('/')
    .and_then(|(head, rest)| (!rest.contains('/')).then_some(head))   // ← rest 不含 '/'
    .and_then(FileKind::for_folder)
    .is_some_and(|kind| kind == entry.kind())
```

`rest.contains('/')` 那一段把 `图片/2024/a.jpg` 排除在外——它是 `Nested`。合理（本版本
不整理子目录），但用户会觉得奇怪：`图片/a.jpg` 说「已经在它该在的地方」，
`图片/2024/a.jpg` 说「在子目录里」。

更重要的是：**`图片/notes.txt`（放错文件夹的文件）也是 `Nested`**，和
`sub/inner.md`（在一个无关文件夹里的文件）用同一句话解释。今天**没有「它在某个分类文件夹里，
但不是那一类」这个理由**。这是后继（尤其 S1）最自然的落点：一个 `Misfiled` 理由。

### 4.3 第 4 级：占位检测的三个盲区

```rust
let taken: Vec<&str> = scan.entries().iter().map(|e| e.relative()).collect();  // 含目录
...
if taken.contains(&destination.as_str()) || claimed.contains(&destination)
```

**盲区 A（关键）—— `taken` 只包含 `entries`，不包含 `skipped`。** 被 `scan` 跳过的东西
（符号链接、socket/设备、不可读项、`UnplannableName`、超出 `max_entries` 的项）
**不在占位表里**。于是：目录里有一个名为 `图片/photo.jpg` 的**符号链接**时，
计划照样提议把散在外层的 `photo.jpg` 移过去。v0.1 无害（不执行），
**v0.1.1 的 AC-27 执行器会撞上它**。占位检测的盲区恰好落在扫描拒绝细看的那些项上。

**盲区 B —— `truncated == true` 时占位表不完整。** 触到 `max_entries`（默认 20 000）
或 `max_depth` 后，`taken` 是残缺的，第 4 级判定不再可靠。`truncated` 已经进了
`to_json` 与哈希，所以信息在——**建议 v0.1.1 的执行器把 `truncated == true` 当作拒绝执行的
充分条件**，而不是当作一个显示用的标志。

**盲区 C —— 精确字节比较。** 在大小写不敏感的卷（macOS/Windows 默认）上，
已有 `图片/Photo.jpg` 而散着 `photo.jpg` 时，`taken.contains` 不命中，计划提议移动，
执行时才撞名。Unicode 规范化（NFC/NFD）同理。中文文件夹名不受分解影响，
**但带重音的文件名会**。这一条今天不产生任何错误输出（因为不执行），是 v0.1.1 的前置件。

### 4.4 `claimed` 今天不可达

两个不同的散文件不可能产生同一个 `destination`：`destination = "{folder}/{relative}"`，
而 `relative` 在一次扫描里唯一。所以 `claimed.contains(&destination)` 永远为假。
它是**对未来的防御**——一旦后继算法让目标名不再等于原名（去重改名、加时间前缀、
S2 的 `重复/` 桶），它立刻变成必要的。保留，但要知道今天它没有测试覆盖，
因为它没有可达路径。

### 4.5 复杂度

`taken.contains` 是 `Vec` 线性扫描，位于对 entries 的循环内：**O(F × E)**，
F = 散在外层且有类的文件数，E = 全部条目数。默认上限 20 000 条时最坏
约 4×10⁸ 次字符串比较。这是一个**预览**，`scan.rs` 的注释明说
「a preview that takes ten minutes on a home directory is a hang」。
换成 `HashSet<&str>` 是 O(F+E) 的一行改动。

记这一笔的原因不是性能洁癖：**S2 的默认档（按 size 分组）本身就需要一张哈希表，
把占位检测顺手改成 HashSet 之后，S2 Tier 0 的额外成本是负的。**

---

## 5. 哈希面：后继算法唯一真正受约束的地方

`to_json()` 是被 `PlanHash::of` 哈希的规范值。`serde_json` 未启用 `preserve_order`
（Cargo.lock 无此 feature），`Value::Object` 是 `BTreeMap`，键序稳定；所有列表都排过序。

进哈希的字段：`action`、`root`（**绝对路径字符串**）、`directory_snapshot`、
`executable_in_this_version`、`scanned_entries`、`skipped_entries`、`truncated`、
`moves[{from,to,kind,size_bytes}]`、`left_alone[{path,reason}]`。

**不**进哈希的：`skipped` 的名字与理由（只有计数）、`modified_unix_seconds`、
`extension`、`injection_signals`、`snapshot_after`。

三条对后继的硬约束：

- **`size_bytes` 在里面。** 文件大小变了，即使移动方案一字不改，`plan_hash` 也变，
  `refuse_execution` 返回 `PlanHashMismatch`。所以 `plan_hash` 不是「方案身份」，
  是「方案 + 被观测到的状态」。**任何加进 `to_json` 的派生量都会扩大「什么算计划变了」。**
- **`executable_in_this_version: false` 在里面**，且有钉测
  (`the_plan_states_in_its_own_hash_that_it_will_not_be_executed`)。这条是范式：
  **一个会改变算法行为的开关，应该出现在被哈希的值里，而不是只在配置里。**
  S1 的「读没读正文」必须照这个样子办。
- **`root` 是绝对路径。** 同一目录换挂载点换哈希。与本文无关，但后继若想跨会话比对计划，
  这是拦路的。

---

## 6. 三条后继（**并列可测，不选赢家**）

合规总表先放在最前面：

| | 默认是否读文件正文 | 默认是否新增 I/O | 需要新的产品锁豁免吗 |
|---|---|---|---|
| **S1 幻数嗅探** | **否（默认关）** | 否 | 开启后需要；默认档不需要 |
| **S2 重复分组** | **否（Tier 0 默认，零读）** | 否 | Tier 1/2 需要，默认不启用 |
| **S3 近因** | **否（永不）** | **否（mtime 已在 `ScannedEntry` 里）** | 否 |

**默认配置下，三者合起来读的第三人正文字节数为 0。** 这是设计前提，不是巧合。

---

### 6.1 S1 —— 本机 MIME 幻数（local magic sniff）

**要解决的问题**：§2.3 的 T1/T2（`license.key` → `演示/`、`.ts` 视频 → `代码/`）
与更一般的「扩展名说谎」。今天的算法**无法区分「扩展名对」和「扩展名被改过」**。

#### 输入 / 输出

- 输入：候选文件的**前 K 字节**（建议 K = 512，覆盖绝大多数容器签名），仅此。
- 输出：`MagicVerdict`，一个封闭枚举，**不含任何字节**：

```
enum MagicVerdict {
    Kind(FileKind),            // 签名唯一确定一类，如 %PDF- → Document
    Family(&'static [FileKind]),// 签名只确定容器，如 PK\x03\x04 → [Archive, Document, Sheet, Slides]
    Unknown,                   // 表里没有
    TooShort,                  // 文件短于签名
    NotSniffed,                // 开关关着，或超出预算
}
```

#### 必须先承认的事：**幻数确定的是容器，不是类别**

| 签名 | 能确定的 | 不能确定的 |
|---|---|---|
| `PK\x03\x04` | 是个 zip | 是 `压缩包` 还是 docx/xlsx/pptx/odt/epub（即 `文档`/`表格`/`演示`） |
| `\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1`（OLE2） | 是个 OLE 复合文档 | doc / xls / ppt 三者不可分 |
| `\x1A\x45\xDF\xA3`（EBML） | mkv 或 webm | 无所谓——两者同为 `视频` |
| `ftyp`（偏移 4） | ISO-BMFF | 需要偏移 8 的 brand 才能分 heic(图) / m4a(音) / mp4(视) |

要把 zip 分成 docx/xlsx 就得**解析 zip 中央目录**——那是读更多正文，并且要在
攻击者可控的结构上做解析，是一个真实的攻击面。**结论：不做容器解析，只出 `Family`。**

#### 组合规则（默认档：幻数只能否决，不能提议）

| 扩展名判定 | 幻数判定 | 结果 |
|---|---|---|
| `Image` | JPEG/PNG/… 同类 | 照旧移动，`kind_source: "extension+magic"` |
| `Image`（`x.jpg`） | `%PDF-` | **不移动**，新理由 `KindDisputed`：「扩展名说是图片，文件开头看起来不是；不猜」 |
| `Document`（`x.docx`） | `Family[Archive,Document,Sheet,Slides]` 含 Document | 视为一致，照旧移动 |
| `Unrecognised`（`x.dat`） | PNG | **仍不移动**（默认档不提升） |
| 任意 | `Unknown` / `TooShort` | 照旧按扩展名走 |

「不提升」这一条是刻意的：一旦幻数能把 `Unrecognised` 变成一次移动，
就等于**文件正文在决定这个文件被放到哪里**——那正是 `kind.rs` 开篇拒绝的事
（「a read-only promise that still opens every file is a smaller promise than it sounds」）。
「提升」作为**单独的变体 S1-b** 留给后面轮次测，默认关，且默认关时行为与今天逐字节相同。

#### 与仓库现状的四个冲突（**这是 S1 真正的代价，不是幻数表**）

1. **`no_write_api.rs` 抓不到读。** needle 表有 `File::create`/`OpenOptions`/`io::Write`，
   没有 `File::open`/`fs::read`/`io::Read`/`BufReader`。做 S1 之前必须**先加一张 `READS` 表**，
   默认对全 crate 生效，只对嗅探模块开例外，并照 `the_search_recognises_a_write_when_it_sees_one`
   的样子补一个「这张表认得读」的对照用例。**不加这张表就做 S1，等于把 crate 最强的那条
   机械保证悄悄降级。**
2. **atime 不在 `DirectorySnapshot` 里。** 开文件会动 atime（`relatime` 下也会，
   当 atime 早于 mtime 或超过一天时）。今天 `disk_unchanged()` 会在嗅探改遍全目录 atime
   的情况下**照样返回 true**。两条出路：把 atime 加进快照（证明变强，嗅探足迹变可见，
   代价是 `a_preview_leaves_the_tree_exactly_as_it_found_it` 在开启嗅探时会红——**这正是它该做的事**），
   或用 `O_NOATIME`（仅 Linux，需属主或 CAP_FOWNER；Windows 无对应物，用 `SetFileTime`
   回写等于写盘，出局）。**推荐前者**，并在预览里报 `opened: N`，让用户看见成本。
3. **字节不得外流。** 缓冲区放栈上定长数组，包外不可见；`Debug` 手写成打码实现
   （crate 有 `#![deny(missing_debug_implementations)]`）；不入日志、不入审计
   （审计无正文是产品锁）、**不入 `to_json`**。只有裁决进计划。
4. **必须进哈希。** 照 `executable_in_this_version` 的先例，`to_json` 加
   `"magic": "off" | "prefix512"`，每条 move 加 `"kind_source"`。用户批准的那个值里
   要记着「这次有没有读正文」。

#### 可数性与预算

只嗅**将要被提议移动的文件**（`depth==1` 且 `folder().is_some()`）——即第 5 级之前的候选。
于是 open 次数 ≤ `moves.len()`，不是 `scanned_entries`。硬上限：≤2 000 次 open、
≤1 MiB 总读入，超出记 `NotSniffed` 并在计划里报计数。

#### 日后怎么证伪

- **先做对照组，再写产品代码。** 一次性本机脚本（不入产品 crate，只输出计数）统计真实目录里
  「扩展名与幻数不一致」的比例。**若 < 1%，S1 的收益不足以抵消上面四条代价，应当否决 S1。**
- **H1（收益）**：不一致率 ≥ 某阈值，且集中在少数可命名的模式（改名截图、下载的 `.jpg` 实为 webp、
  `.txt` 实为 UTF-16、T1 的 `.key`）。若不一致全是随机噪声，`KindDisputed` 只会制造困惑。
- **H2（可用性反噬）**：若 `KindDisputed` 挡下 > 5% 的候选，预览从「四条建议」变成
  「一堆我不确定」，可用性下降。这条要在同一语料上量。
- **H3（更便宜的替代）**：一个只用「大小 + 扩展名 + 名字模式」的分类器能否达到同样的
  不一致检出率？**能，则 S1 不必读正文，直接否决。** 这是必须先跑的对照。
- **回归钉测**：`magic: off` 时，对 `tests/common::Tree` 的 `plan_hash` 与今天完全相等。
- **端到端**：造 `invoice.jpg`（内容 `%PDF-`）、`photo.png`（内容 zip）、`notes.txt`（内容 PNG），
  断言三者都不被移动、理由为 `KindDisputed`。
- **成本钉测**：插一个 open 计数器，断言 `opens == 候选移动数`。
- **atime 钉测（Linux）**：关闭时 atime 不变；开启时计划里报出的 `opened` 与实际变化数相等。

---

### 6.2 S2 —— 重复分组（duplicate grouping）

**要解决的问题**：整理目录时，真正的噪声往往不是「分类错」，是「同一个文件有五份」。
今天的算法会把 `报告.pdf`、`报告 (1).pdf`、`报告 - 副本.pdf` 三个都移进 `文档/`，
一个都不合并，也不提一句。

#### 三档，默认档零读

| 档 | 依据 | 读正文 | 默认 | 能说的话 |
|---|---|---|---|---|
| **Tier 0** | `(size_bytes, extension)` + 名字副本标记 | **否** | **开** | 「可能重复」 |
| **Tier 1** | 首 64 KiB + 末 64 KiB + size 的哈希 | 是（有界） | 关 | 「很可能重复」 |
| **Tier 2** | 全量 SHA-256 | 是（全量） | 关，需逐次明示同意 | 「内容相同」 |

#### Tier 0 的精确定义（这是唯一默认启用的部分）

- 分组键 `(size_bytes, extension)`，只取 `size_bytes ≥ MIN`（MIN 待定，见下）且组内 ≥ 2。
  **必须排除 0 字节**，否则所有空文件互为「重复」。
- 名字证据：只识别**操作系统机器生成**的副本标记，是一张封闭可引用的表，不是启发式猜测：

| 来源 | 形态 |
|---|---|
| Windows 资源管理器（英） | `name (1).ext`、`name (2).ext` |
| Windows 资源管理器（中） | `name - 副本.ext`、`name - 副本 (2).ext` |
| macOS Finder（英） | `name copy.ext`、`name copy 2.ext` |
| macOS Finder（中） | `name 的副本.ext`、`name 的副本 2.ext` |

**不**识别 `v2`、`final`、`最终版`——那些是人写的版本，不是副本，把它们当重复是错的。

- 置信度**三档**（`weak` / `moderate` / `strong`），**绝不给百分比**——
  Cycle 1 的 G10 已经settle：D22 禁量表/百分位，`Band` 故意不实现 `Ord`。
  - `weak`：仅 size 相同
  - `moderate`：size + 扩展名相同
  - `strong`：再加上「其中一个的名字是另一个的副本标记形式」
- 输出形态：`duplicate_groups: [{ kind, size_bytes, members: [relative...], confidence, basis }]`，
  **是对计划的注解，不是移动，更不是删除。**

#### 与锁的冲突

- **Tier 1/2 读正文 ⇒ 默认关。** 哈希值是关于第三人内容的派生数据。
- **绝不提议删除。** 产品锁 v0.1.1 解锁的是「授权根内**可逆**移动」。删除不可逆。
  Tier 2 允许的最强动作是把多余份移进 `重复/`——那是可逆的。
- **哈希不得持久化。** 一个跨会话的文件哈希索引是一份**永远活着、没有遗忘路径**的派生语料
  （Cycle 1 的 G12 讲的就是这个）。本 crate 不依赖 `soul-store`，天然做不到持久化——
  **这条要保持，别在做 S2 时顺手加依赖**（`no_write_api.rs` 的第二层正好会红）。
- 若引入 `重复/` 文件夹：`FileKind::for_folder("重复")` 返回 `None`，
  于是里面的东西下次扫描全变 `Nested`——不算错，但应该引入「保留文件夹」概念，
  与 §4.4 的 `claimed` 一起变成可达代码。

#### 成本

Tier 0 是一次 `HashMap` 分组，O(n)。把 §4.5 的 `taken.contains` 换成 `HashSet`
（O(F×E) → O(F+E)）省下来的时间**远超** Tier 0 的开销。净成本为负。

#### 日后怎么证伪

- **决定性实验（一次性、本机、只出计数）**：在真实目录上跑 Tier 0，
  用 Tier 2 的全量哈希做**金标准**，算 Tier 0 的 precision / recall。
  **precision ≥ 0.9 ⇒ Tier 1/2 不必要，永远不用读正文。** 这个实验做完就能定档。
- **H1（值不值得做）**：`size` 相同且组内 ≥2 的文件占比 < 2% ⇒ 对多数用户一行输出都没有 ⇒ 不做。
- **H2（MIN 阈值）**：precision 若被「同模板导出的一批 512 B 文件」压垮，
  MIN 由同一实验的 precision-vs-MIN 曲线给出，而不是拍脑袋。
- **H3（名字表覆盖率）**：在 Windows/macOS × 中/英 四种环境各造一批真副本，
  断言标记表覆盖 ≥ 95%。这个必须在真机上造，不能靠回忆写表。
- **反面钉测**：`报告_v2.pdf` 与 `报告_final.pdf` 即使 size 相同也**不得**标 `strong`。

---

### 6.3 S3 —— 近因（recency of kind）

**唯一一个零新增 I/O 的后继**：`ScannedEntry::modified_unix_seconds`（`Option<i64>`，秒级）
今天就在结构体里，`plan::build` 从不读它，也不进 `to_json`。整条 S3 不多开一个文件描述符。

三个**必须分开评估**的子算法——它们共用「近因」这个名字，但失败方式完全不同：

#### S3a —— 刚动过的文件不提议移动（安全闸）

`as_of - mtime < T`（建议 T = 1 小时）⇒ 新理由 `RecentlyActive`：「你刚动过它，先不碰」。
理由：一个五分钟前保存过的文件很可能正被某个程序打开着；v0.1.1 移走它会打断编辑器、
打断「最近文件」列表。与 S1 同型——**这是一条只会收缩移动集合的规则**，不引入新的移动。

#### S3b —— 单件类别不建文件夹（支持度阈值）

某一类在外层只有 1 个文件时，为它单独建一个文件夹是噪声。要求同类散文件数 ≥ K（K=2 或 3）
才提议该类，**除非该类文件夹已存在**（那样不会新建任何东西，1 个也可以）。
新理由 `TooFewOfKind`：「这一类只有一个，单独建个文件夹不值当」。

注意这条**会改今天的钉测**：fixture 里 `budget.csv` 是唯一的表格，K=2 时它不再被移动，
`an_authorized_directory_previews_a_plan_and_the_disk_does_not_move` 的四条会变三条。
这不是意外，是这条规则的定义——但它意味着 S3b 必须单独立测，不能和 S3a 混在一次改动里。

（严格说 S3b 是支持度不是近因，放在这里是因为它和 S3a/S3c 共享同一个目标：
让预览更少、更像人会做的整理。**评估时分开。**）

#### S3c —— 近因排序与分档（只改显示）

按「用户最近在这个目录里添/改得最多的类」排序，并给每组标粗档
（`最近 24 小时` / `本周` / `更早`）。

**这条必须留在渲染层，不进 `to_json`。** 今天 `moves` 按 `from_relative` 排序，
那个排序就是哈希可复现的理由（§5）。把展示顺序塞进被哈希的列表等于让哈希依赖展示策略。

#### **本文最关键的一条设计约束：引入「现在」会让 `plan_hash` 漂移**

`plan::build(scan)` 今天是纯函数，不接时钟。近因规则需要一个参照时刻。
**如果它自己去读时钟**，那么同一个没人碰过的目录，一分钟后再扫会得到不同的 `plan_hash`，
`refuse_execution` 于是返回 `PlanHashMismatch` —— 用户看到
「计划在你批准之后变过了，请重新看一遍再决定」，**而计划其实没变，目录也没变**。
这是一次**假拒绝**，它教用户忽略这条拒绝，比不做这个特性更糟。

按优先级的四条修法：

1. **沿用 T4D 的 as_of 纪律**（Cycle 1 的 G13 同一条）：`build(scan, as_of)` 把时刻**作为参数传入，
   绝不在内部读时钟**。`preview()` 已经收着 `now_ms` 了——顺着传下去即可，是一个参数的改动。
2. **量化**：as_of 先粗化（UTC 日）再参与任何判定，于是计划在一天内稳定，漂移可预测、可解释。
3. **把量化后的 as_of 放进 `to_json`**：跨零点时哈希变化是**诚实且可解释**的
   （「这个计划是按某一天算的」），而不是神秘的。
4. **阈值加滞回**，否则卡在 T 上的文件会在连续两次扫描间来回跳。

#### mtime 本身的三个已知不可靠处

- **不是「用户在用它」的可靠代理**：`cp -p`、解压、`git checkout`、下载工具都会保留或重置 mtime。
- **没有 birthtime**：`ScannedEntry` 不收 `created()`，无法区分「新建的」与「刚改的」。
- **没有本地时区**（Cycle 1 的 G2：全仓无 UTC 偏移，`clock.rs` 只产 `Z`）。
  于是「今天」是 UTC 的今天——对 UTC+8 的用户每天错八小时。
  **建议直接用「最近 24 小时」这种无偏移的措辞**，绕开这个缺失的原语，而不是先去补 G2。

#### 日后怎么证伪

- **H1（S3a 值不值）**：统计真实目录里 `as_of - mtime < 1h` 的散文件占比。若 < 2%，
  S3a 几乎不改变任何计划——**但它的价值本来就不在命中率，在于避免那一次灾难**，
  成本又近零，所以低命中率不足以否决它。要否决 S3a 得证明它挡下的都是用户其实想移的。
- **H2（K 怎么定）**：在真实语料上量「被 K 挡下的文件占比」。> 30% 说明 K 太大，
  预览变得什么都不建议。今天 fixture 上 K=2 的效果是 4 条 → 3 条，可直接钉。
- **H3（阈值有没有结构）**：把 T 从 1h 扫到 7d，若计划内容全程不变，
  说明 mtime 在这个目录上没有可用结构 ⇒ 放弃 S3a。这是一个**扫参数就能做的证伪**。
- **H4（mtime 是不是真代理）**：拿 mtime 顺序与 `soul-collect` 的前台会话时间线对照。
  **但 `soul-fileplan` 不依赖 `soul-store`，也不应该依赖**（`no_write_api.rs` 第二层会红）。
  所以这个验证只能是一次性离线研究，**S3 在产品内无法自我校准**——这一点要写进结论，
  不要事后当成缺陷发现。
- **假拒绝钉测（必做）**：同一目录、`as_of` 相差一秒、跨不过量化边界 ⇒ `plan_hash` 必须相等。
  这条测试直接钉住上面那个最关键的约束。

---

## 7. 三者的组合与顺序

- **形状一致**：S1 的 `KindDisputed`、S3a 的 `RecentlyActive`、S3b 的 `TooFewOfKind`
  都是**只收缩移动集合**的规则，都在第 4 级与第 5 级之间插入，都不产生新的移动。
  它们可以任意组合，短路顺序建议：`RecentlyActive` → `KindDisputed` → `TooFewOfKind` → `DestinationTaken`
  （先说最像「你自己的事」的理由）。S2 完全不产生移动，是并行的注解层。
- **新变体一律追加在 `LeaveReason` 末尾**（§3），并各补一句中文 `explanation()`——
  `everything_the_plan_leaves_alone_says_why` 会断言它非空。
- **建议的先后**：
  1. 先做**零风险的地基**：`taken` 换 `HashSet`（§4.5）、深层目录的计数缺口（§4.1）表态、
     `truncated ⇒ 拒绝执行`（§4.3 盲区 B）。这些与三条后继都无关，但 S2 的成本模型依赖第一条。
  2. 再做 **S3**（零新增 I/O，唯一需要的是 as_of 纪律与那条假拒绝钉测）。
  3. 再做 **S2 Tier 0**（零读，且能被一次离线实验决定要不要有 Tier 1/2）。
  4. **S1 最后**，且**先跑 §6.1 的对照组**。在 `READS` needle 表和 atime 快照这两个洞补上之前，
     S1 不应该有产品代码。
- 三条**都不是互斥的**，按 `CONSTRAINTS.md` 的要求并列标为「经确认可测」，不选赢家。
  唯一带前置条件的是 S1（对照组 + 两个洞）。

---

## 8. 后续怎么证伪本文

本文的事实部分全部可查，逐条给出反例的找法：

- 「扩展名表 69 项 / 8 类」→ 数 `kind.rs:88-103` 的 `match` 字面量；`FileKind::ALL` 9 项，
  `folders()` 8 项（`every_folder_name_maps_back_to_exactly_one_kind` 已断言 `ALL.len() - 1`）。
- 「`plan::build` 不接时钟」→ 签名 `fn build(scan: &DirectoryScan) -> OrganizePlan`；
  `rg 'SystemTime::now|Instant::now|clock' crates/soul-fileplan/src/` 今天为空。
- 「`no_write_api.rs` 不查读」→ 读 `WRITES` 常量（`tests/no_write_api.rs:33-68`）：
  没有 `File::open`、`fs::read`、`io::Read`、`BufReader`。出现任一即为反例。
- 「快照不含 atime」→ `DirectorySnapshot::of` 只写入 `relative / tag / metadata.len() / modified`
  （`scan.rs:200-203`）。出现 `accessed()` 即为反例。
- 「`taken` 不含 skipped」→ `plan.rs:205` 只 map 了 `scan.entries()`；
  `scan.rs` 的 `skipped` 是另一个 `Vec`，`build` 从不读它。
- 「`claimed` 不可达」→ 找一对能产生相同 `destination` 的散文件即为反例；
  需要两个 `relative` 相同的条目，文件系统给不出。
- 「深层目录不出现在计划里」→ `plan.rs:214-224`：`depth() == 1` 才 push，其余 `continue`。
  在 fixture 的 `Alpha/sub/` 下加一个子目录，`moves.len() + left_alone.len() < scanned_entries`。
- 「`AlreadySorted` 只认深度 2」→ `plan.rs:229` 的 `!rest.contains('/')`。
  造 `图片/2024/a.jpg`，断言它是 `Nested`。
- 「`图片/notes.txt` 与 `sub/inner.md` 同理由」→ 同上，两者都走 `false` 分支。
- 「`serde_json` 键序稳定」→ `rg preserve_order Cargo.lock` 今天为空；有命中即为反例。
- 「`size_bytes` 进哈希」→ `to_json` 的 `moves` 映射（`plan.rs:180`）。
  改一个文件的内容但不改名，`plan_hash` 变——`the_plan_hash_is_stable_until_the_directory_is_not`
  测的是加文件，加一个「只改大小」的用例即可直接观察。
- 「`.key` → `演示`」→ `FileKind::of_extension(Some("key")) == FileKind::Slides`（`kind.rs:94`）。
- 「`tgz` 未收录而 `tar.gz` 收录」→ `extension_of("a.tgz") == Some("tgz")`，
  `of_extension(Some("tgz")) == Unrecognised`；`extension_of("a.tar.gz") == Some("gz")` → `Archive`。
- 「`con.pdf` 之类根本进不了计划」→ `screen_segment` 的 `RESERVED_DEVICE_NAMES` 比对的是
  `split('.').next()`（`screen.rs:218`），`scan` 据此记 `UnplannableName` 并跳过。
