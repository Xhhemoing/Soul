MODEL_SLUG: gpt-5.6-sol-xhigh-fast

# C2R1：`soul-fileplan` 的 FileKind 与 leave-reason 探针

## 0. 结论

当前基线不是学习算法，而是一个完全确定、可枚举、可解释的有限规则：

```text
文件名最后一段后缀 -> 小写化 -> 69 项精确匹配表 -> 8 个可移动 kind / Unrecognised
```

规划器再按「目录 / 所在深度 / 是否已分类 / 能否识别 / 目标是否占用」的固定优先级，
给每个纳入扫描的普通文件恰好一个 move 或 leave 结果。分类不读正文、不看 MIME、不用
修改时间或大小；扫描只取目录项与 metadata。依据分别在
`crates/soul-fileplan/src/kind.rs:1-7`、`crates/soul-fileplan/src/scan.rs:411-430` 和
`crates/soul-fileplan/src/plan.rs:211-267`。

本槽独立读取源码、现有测试和本循环约束；未读取其他 Cycle 2 Round 1 槽位。

## 1. FileKind 全表

枚举顺序与稳定 token 来自 `crates/soul-fileplan/src/kind.rs:12-49`；中文文件夹来自
`crates/soul-fileplan/src/kind.rs:52-66`；后缀表来自
`crates/soul-fileplan/src/kind.rs:83-103`。

| kind | 稳定 token | 目标文件夹 | 精确后缀（不含点） | 数量 |
|---|---|---|---|---:|
| `Image` | `image` | `图片` | `jpg jpeg png gif bmp webp heic tif tiff svg` | 10 |
| `Document` | `document` | `文档` | `pdf doc docx odt rtf txt md epub` | 8 |
| `Sheet` | `sheet` | `表格` | `xls xlsx ods csv tsv` | 5 |
| `Slides` | `slides` | `演示` | `ppt pptx odp key` | 4 |
| `Archive` | `archive` | `压缩包` | `zip rar 7z gz bz2 xz tar iso` | 8 |
| `Audio` | `audio` | `音频` | `mp3 wav flac aac ogg m4a wma` | 7 |
| `Video` | `video` | `视频` | `mp4 mkv mov avi wmv webm m4v` | 7 |
| `Code` | `code` | `代码` | `rs py ts tsx js jsx java go c h cpp hpp cs sh ps1 sql json toml yaml yml` | 20 |
| `Unrecognised` | `unrecognised` | 无 | 无后缀、空后缀或不在上述 69 项中的任何后缀 | — |

关键语义：

- 只取**最后一个点之后**的部分并调用 Unicode `to_lowercase()`；`photo.JPG` 命中
  `jpg`，`archive.tar.gz` 只看到 `gz`。见
  `crates/soul-fileplan/src/kind.rs:106-115`。
- `.gitignore`、`Makefile`、`notes.` 都没有可用后缀，因而 abstain 为
  `Unrecognised`；现有单测钉住这三类边界。见
  `crates/soul-fileplan/src/kind.rs:122-131`。
- 小写化发生在 `extension_of(name)`，不发生在公开的
  `of_extension(extension)` 内；扫描管线会先调用前者，所以 `photo.JPG` 可命中，
  但外部调用者直接传 `of_extension(Some("JPG"))` 会得到 `Unrecognised`。见
  `crates/soul-fileplan/src/kind.rs:83-115`。
- `svg` 被明确视为图片，`json/toml/yaml/yml` 被明确视为代码，`md/txt` 被视为文档，
  `key` 被视为演示，`iso` 被视为压缩包；表没有内容嗅探来推翻这些选择。
- 常见但未列入的 `avif`、`jfif`、`html`、`css`、`xml`、`zst`、`tgz`、`docm`、
  `xlsm`、`pptm`、`pages`、`numbers`、`mobi`、`opus`、`m4b`、`rb`、`php`、
  `swift`、`kt`、`r` 等都会机械落入 `Unrecognised`。这是精确匹配表的直接结果，
  不是概率低或置信度不足。
- 扫描目录时会强制令 extension 为 `None`，不过规划器先按 `is_dir` 分支，因此不会把
  目录向用户解释成「认不出」。见 `crates/soul-fileplan/src/scan.rs:411-430` 和
  `crates/soul-fileplan/src/plan.rs:214-224`。
- `FileKind::folders()` 与 `for_folder()` 共用同一张枚举表，现有测试验证 8 个文件夹
  都能唯一反查 kind。见 `crates/soul-fileplan/src/kind.rs:69-81,134-143`。

## 2. 五种 leave reason 与真实优先级

token、中文解释定义于 `crates/soul-fileplan/src/plan.rs:54-94`。

| reason | 稳定 token | 中文解释 | 实际触发条件 |
|---|---|---|---|
| `Directory` | `directory` | `这是目录，本版本不动目录` | 深度 1 的目录 |
| `AlreadySorted` | `already_sorted` | `已经在它该在的分类文件夹里` | 普通文件恰为深度 2，第一层目录是 8 个精确分类名之一，且该文件 kind 与目录匹配 |
| `Nested` | `nested` | `在子目录里，本版本只整理散在最外层的文件` | 其余所有深度大于 1 的普通文件 |
| `UnrecognisedKind` | `unrecognised_kind` | `认不出这是哪一类，不猜` | 顶层普通文件的 kind 没有目标文件夹 |
| `DestinationTaken` | `destination_taken` | `目标文件夹里已经有同名的东西` | 顶层、可识别普通文件的完整目标相对路径已在扫描结果中，或已被本计划先前 move claim |

实现不是五个独立标签器，而是以下短路顺序（
`crates/soul-fileplan/src/plan.rs:211-267`）：

```text
1. is_dir
   ├─ depth == 1 -> Directory
   └─ depth > 1  -> 不产生 LeftAlone 行
2. 普通文件且 depth > 1
   ├─ 恰为「匹配分类目录/文件」-> AlreadySorted
   └─ 否则 -> Nested
3. 顶层普通文件且 kind.folder() == None -> UnrecognisedKind
4. 顶层可识别普通文件且目标完整路径 taken/claimed -> DestinationTaken
5. 否则 -> ProposedMove
```

所以 reason 有明确的覆盖关系：

- `图片/report.pdf` 是 `Nested`，不是 `AlreadySorted`。
- `文档/mystery.qqq` 是 `Nested`，不是 `UnrecognisedKind`。
- `图片/2026/photo.jpg` 也是 `Nested`；`AlreadySorted` 特意只承认一层分类。
- 深层目录本身没有 leave 行；顶层父目录的一条 `Directory` 被视为覆盖它。深层普通文件
  仍逐个得到 `Nested`。这与 `crates/soul-fileplan/src/plan.rs:214-239` 的注释和分支一致。
- 完整 move 与 leave 列表最终按相对路径排序；leave 同路径时再按枚举顺序排序。见
  `crates/soul-fileplan/src/plan.rs:269-270`。

现有验收夹具一次覆盖了全部五个 reason：`Makefile`/`mystery.qqq` 未识别，
`sub`/`图片` 是目录，`sub/inner.md` 嵌套，`taken.png` 目标占用，
`图片/already.png`/`图片/taken.png` 已分类。见
`crates/soul-fileplan/tests/authorized_scan.rs:76-118` 和夹具构造
`crates/soul-fileplan/tests/common/mod.rs:87-100`。

## 3. 输出面与 hash

- `OrganizePlan::to_json()` 把 move 的 `kind` token 和 leave 的
  `{path, reason-token}` 纳入 canonical plan JSON，因此路径、分类或 reason 改变会改变
  `plan_hash`。中文 `explanation()` **不在 hash 中**。见
  `crates/soul-fileplan/src/plan.rs:160-196`。
- IPC 视图才把中文 explanation 添回每条 leave 行，并把 kind 的中文标签直接取自目标
  文件夹；`Unrecognised` 的 UI 标签回退为 `未分类`。见
  `crates/soulcore/src/commands/fileplan.rs:110-208`。
- `SkipReason` 是另一套扫描级 taxonomy：`symlink`、`not_a_file_or_directory`、
  `unreadable`、`depth_limit`、`entry_limit`、`unplannable_name`。这些对象不进入
  `left_alone`；IPC 计划只带 `skipped_entries` 数量和 `truncated`，不带逐项 skip
  原因。见 `crates/soul-fileplan/src/scan.rs:113-149`、
  `crates/soul-fileplan/src/plan.rs:272-279` 和
  `crates/soulcore/src/commands/fileplan.rs:120-180`。因此「每个没 move 的对象都有
  leave reason」若把 symlink、不可读项或深层目录也算进去并不成立；准确说法是：
  每个**成功纳入扫描的普通文件**有 move 或 leave。

## 4. 探出的边界与后续可证伪点

1. **分类文件夹名被普通文件占用时，计划仍可能给出不可实现目标。**
   若根目录里有普通文件 `图片` 和另一个 `photo.jpg`，`taken` 包含 `图片`，却不包含
   完整字符串 `图片/photo.jpg`；当前检查仍会提出 `photo.jpg -> 图片/photo.jpg`。
   `crates/soul-fileplan/src/plan.rs:204-205,251-266` 只检查完整目标，没有检查目标父级
   是目录。现有夹具里的 `图片` 是目录，未覆盖此反例。

2. **占用判断是字符串精确相等，不是目标文件系统等价。**
   在大小写折叠文件系统上，顶层 `Photo.JPG` 与既有 `图片/photo.jpg` 可能冲突，但
   `taken.contains("图片/Photo.JPG")` 看不到 `图片/photo.jpg`。代码位置同上。v0.1
   不执行，所以这不会当场覆盖文件；v0.1.1 不能复用此判断作为最终写前安全检查。

3. **截断扫描可漏掉已有目标。**
   `max_depth = 1` 不进入分类目录，`max_entries` 也可在看到目标前截断；计划仍会产生
   move，只同时标 `truncated = true`。扫描分支见
   `crates/soul-fileplan/src/scan.rs:365-371,433-442`，目标判断见
   `crates/soul-fileplan/src/plan.rs:251-267`。后续执行必须以未截断重扫及写前原子
   `no-replace`/等价机制证伪冲突，不能把 preview 的 `DestinationTaken` 当完备证明。

4. **69 项表没有逐项回归测试。**
   当前 kind 单测覆盖后缀提取、文件夹双射和 unknown abstention；验收测试实际命中的
   分类主要是 `csv/jpg/pdf/txt/md/png`。见
   `crates/soul-fileplan/src/kind.rs:118-150` 与
   `crates/soul-fileplan/tests/authorized_scan.rs:59-73,183-190`。最低成本 oracle 应枚举
   全 69 项，断言每项唯一归类，并对每类至少做大小写变体；再加上述父级为普通文件、
   大小写折叠冲突、截断目标三个负例。

## 5. 作为 Cycle 2 基线的可测规格

- **输入：** 文件名、entry 类型、相对深度，以及同一扫描里的相对路径集合；不含正文、
  MIME、窗口标题或外部模型。
- **输出：** 九选一 `FileKind`；顶层可识别文件再输出一个目标路径，否则输出五选一
  leave reason（扫描失败另走六选一 `SkipReason`）。
- **可数性：** 69 个命中后缀、8 个目标目录、5 个 leave token，均可逐项计数；未知项
  明确 abstain，不存在隐藏分数或置信度。
- **锁适配：** 满足 `.agent_workspace/predict/cycle2/CONSTRAINTS.md` 的扩展名分桶、
  不读正文、不执行；计划值中
  `executable_in_this_version` 恒为 false，见
  `crates/soul-fileplan/src/plan.rs:155-170`。
- **证伪：** 任一同名输入得到不稳定输出、任一表外后缀被猜测、任一内容变化（文件名不变）
  改变 kind、任一纳入扫描的普通文件既无 move 又无 leave、或上述三个路径冲突反例被
  当成安全可执行计划，都足以推翻相应性质。

本机探针 `cargo test -p soul-fileplan` 通过：50 个 crate 测试、0 失败；这证明现有
断言保持成立，不覆盖第 4 节新增反例。
