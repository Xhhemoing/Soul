# 工程质量复查与修复记录 — 2026-09-24

核查对象：`E:\Project\Soul`，起始 HEAD `9700949` 加当前工作树。仓库已有未提交的 Windows 门禁修复；复查期间还存在其他构建及文档更新。本记录只归属下列文件与验证，不将其他会话的改动、测试或审查算作本轮成果。续验中只读核对了独立收尾工作树 `E:\Project\Soul-closeout-20260924` 的 `2973ad1` 日志；未改写该工作树。

结论等级：`[V]` 已用代码/命令验证；`[T]` 未完成验证。没有新功能、依赖、schema、IPC 或文件执行能力变更。本轮没有提交、推送、安装或卸载。

## BF-20260924-01：目录扫描资源上限可被跳过项绕过

- **问题 [V]**：`max_entries = 5` 时，80 个不适合生成计划的文件名全部进入 `skipped`。`read_in_name_order` 还会先收集并排序整个目录，之后才应用扫描上限。
- **根因**：预算使用成功项 `entries.len()`；底层目录枚举没有预算参数。
- **修复**：统一计算已检查项，非法名称、链接与读取错误消耗预算；枚举先取剩余预算，再用额外一个条目判断截断，然后仅排序已取得的样本。
- **文件**：`crates/soul-fileplan/src/scan.rs`、`tests/scan_is_bounded.rs`、`tests/scan_support/listing_budget.rs`。
- **验证 [V]**：修复前 80 对 5 的断言失败；修复后通过。10 万项惰性输入只拉取 6 项；删除枚举上限的独立变异副本会拉取 10 万项，两个预算断言均失败。
- **边界**：截断后的样本来自文件系统枚举前缀，再做局部排序，不保证得到全目录字典序最小的项目。文件系统若改变样本，快照保守地报告差异；不再承诺不受枚举顺序影响。
- **证据**：`target/quality-fileplan-red-20260924.log`、`target/quality-fileplan-final-20260924.log`、`target/quality-listing-mutation-20260924.log`。

## BF-20260924-02：快照多读取一层目录

- **问题 [V]**：`max_depth = 1` 时扫描仅列出一个子目录，快照却还读入其下一层，共记录 3 项。
- **根因**：快照使用 `depth < max_depth`，扫描使用 `depth + 1 < max_depth`，两处边界不同；快照也未标明深度截断。
- **修复**：统一深度条件；深度被截断时标记部分视图。深度 0 不枚举子项。`disk_unchanged` 注释明确仅比较采样元数据。
- **文件**：`crates/soul-fileplan/src/scan.rs`、`tests/scan_is_bounded.rs`。
- **验证 [V]**：修复前快照 3 对 1 的断言失败；修复后快照、扫描均只含 1 项。新增深度 0 用例通过。
- **证据**：同上目录扫描 red/final 日志。

## BF-20260924-03：Telegram 损坏正文被静默丢弃

- **问题 [V]**：正文缺失、类型错误或富文本数组夹有损坏片段时，解析仍成功，正文变成空串或残缺文本。
- **根因**：`flatten_text` 对未知根值返回空串，数组使用 `filter_map` 丢弃无效片段。
- **修复**：正文解析返回 `Option<String>`；任一不合法片段产生 `text` 字段缺陷并拒绝整个解析结果。错误说明只含固定词汇和消息定位，不引用正文。
- **文件**：`crates/soul-import/src/telegram.rs`、`tests/telegram.rs`。
- **验证 [V]**：旧产品库的独立复现出现“损坏正文仍被接受”；新产品库上两项正式回归测试的独立 harness 通过，覆盖缺失正文、null、数字、对象、损坏富文本、空标题和未知但结构合法的实体类型。
- **兼容性**：合法空字符串、空数组和带字符串 `text` 的未知实体类型仍可导入。此前被容错吞掉的损坏导出现在会明确拒绝。
- **证据**：`target/quality-telegram-red-20260924.log`、`target/quality-telegram-probe-green-20260924.log`。完整 Cargo 测试结果以验证表为准。

## BF-20260924-04：Windows 严格 lint 被 Unix 专用代码阻断

- **问题 [V]**：`cargo clippy -p soul-fileplan --all-targets -- -D warnings` 在 Windows 因 `unused_mut` 和 `unused_imports` 失败。
- **根因**：仅 Unix 测试使用的可变绑定与 `SkipReason` 导入没有对应条件编译。
- **修复**：将导入及可变绑定限定在 Unix 分支，保持测试语义；没有关闭 lint。
- **文件**：`crates/soul-fileplan/tests/unauthorized_paths.rs`、`tests/file_names_are_data.rs`。
- **验证 [V]**：严格 lint 修复前报错、修复后成功；Windows 模块完整测试通过。Linux 实机分支未运行。
- **证据**：`target/quality-fileplan-clippy-red-20260924.log`、`target/quality-fileplan-clippy-green-20260924.log`。
- **续验补充**：根工作区第一次续验通过 1041 项测试，但编译报告采集测试里的 `NO_FOREGROUND_SOURCE` 与 `Duration` 两处未使用导入。已让导入与其使用者一样受 `#[cfg(not(windows))]` 限定（`crates/soulcore/tests/session_collect.rs`、`tests/collect_probe.rs`），没有屏蔽告警。最终全量复验结果见下表；该补充不宣称全工作区严格 clippy 已通过。

## BF-20260924-05：TCP 零采样被记录为通过

- **问题 [V]**：隔离收尾版本 `2973ad1` 的 G-W 日志中，`0 sample(s) of the TCP table` 仍被判定为 AC-21 的 Windows 观察通过。该版本不含本记录的目录扫描和 Telegram 修复；此处只引用它暴露出的脚本行为。
- **根因**：监测命令在子进程启动之后才发现/自动加载，短进程可能已退出；最终判断只检查外网连接列表为空，没有要求采样次数大于零。
- **修复**：提前发现/加载监测命令；Windows TCP 观察必须同时满足 `Samples > 0` 与外网连接列表为空，否则门禁失败。未修改安装、卸载或用户数据操作。
- **文件**：`scripts/install-smoke.ps1`、`scripts/test-install-smoke-watch.ps1`、`crates/soulcore/tests/install_smoke_script.rs`。
- **验证 [V]**：新回归执行从生产脚本 AST 提取的真实判定与 finding 函数。修复前零采样用例显示 `Passed=True`，回归退出 1；修复后零采样和已观察到外网连接均拒绝，一次/多次空采样均通过，共 4 项。原有真实子进程的 5 项退出码/JSON/UTF-8 回归继续通过。
- **本机集成 [V]**：新脚本配合现有 debug 二进制运行 `-SkipInstall`，15 项检查通过，TCP 表记录 1 次采样。该次只验证脚本集成，不代表新打包产物或整套 G-W。
- **证据**：`target/quality-tcp-watch-red-20260924.log`、`target/quality-tcp-watch-green-20260924.log`、`target/quality-continue-process-watch-20260924.log`、`target/quality-tcp-smoke-20260924.log`。
- **边界**：这是轮询 TCP 表；不证明短连接、UDP 或整个 WebView2 的所有出网均被捕获。旧版本零采样的成功行不能充当 Windows 观察证据。命令不可用时的既有 skipped 分支没有在本次改为强制失败。

## 验证状态

| 检查 | 本轮结果 |
|---|---|
| `cargo fmt --all -- --check` | [V] 通过 |
| `git diff --check` | [V] 通过 |
| `soul-fileplan --all-targets` | [V] 66 项通过；独立 `target/quality-review` 构建目录 |
| `soul-fileplan --all-targets` 严格 clippy | [V] 通过，`-D warnings` |
| Telegram 新增解析回归独立 harness | [V] 2 项通过，链接本轮重新编译的产品库 |
| 前端 lint | [V] 通过并已续验，tsc + ESLint；`target/quality-continue-ui-lint-20260924.log` |
| 前端 Vitest | [V] 续验 14 个文件、191 项通过；`target/quality-continue-ui-tests-20260924.log` |
| 前端生产构建 | [V] 通过 |
| Schema 冻结校验 | [V] 通过；使用当前树构建的 `target/debug/xtask.exe` |
| E0 静态审计 | [V] 通过；增加脚本回归后的续验为 16 个产品 crate、215 个源文件 |
| E1 出网回归 | [V] 11 项通过：跨 origin/端口重定向、第三人正文占位、响应上限及无限流；`target/quality-egress-tests-20260924.log` |
| 禁词静态审计 | [V] 通过，94 个词、119 个文件 |
| 工程门禁自身回归（xtask） | [V] 33 项通过，含伪装回环地址、越权 HTTP 依赖、schema 漂移与 SBOM；`target/quality-guardrails-tests-20260924.log` |
| `soul-import --lib` 严格 clippy | [V] 通过，`-D warnings`；`target/quality-import-clippy-20260924.log` |
| `cargo test --locked --workspace --all-targets` | [V] 最终复验通过：134 个测试程序、1041 项通过、0 失败/忽略，编译告警 0；`target/quality-workspace-final-20260924.log` 与同名 JSON 退出码记录 |
| Cargo Telegram 集成测试 | [V] 最终根工作区全量复验内 15 项通过，含新增正文解析回归；旧单独运行曾因磁盘不足（LNK1180）失败，`target/quality-telegram-green-20260924.log` 保留实际失败内容，文件名不作为结果 |
| 桌面壳 `--all-targets --locked` | [V] 续验通过：7 个测试程序、88 项通过、0 失败/忽略；`target/quality-desktop-rerun-20260924.log` 与同名 JSON 退出码记录 |
| TCP 证据判定回归 | [V] 4 项通过；修复前零采样误判可复现，修复后明确拒绝 |
| 真实子进程退出码/JSON/UTF-8 回归 | [V] 5 项通过；`target/quality-continue-process-watch-20260924.log` |
| debug `install-smoke -SkipInstall` | [V] 15 项通过，TCP 采样 1 次；现有 debug 产物，仅证明新脚本集成 |
| 本次修复的 release 打包安装 / 真机出网观察 | [T] 本轮未执行；不据此宣称 G-W、G-M 或 Goal 1 通过 |
| 依赖漏洞数据库检查 | [T] 本轮未执行；本机未找到 `cargo-deny`，静态 E0 审计不替代依赖漏洞检查 |

续验执行于本机 Windows 11 build 26200、Rust 1.83.0、PowerShell 7.6.5、MSVC x64 环境。最终根工作区命令退出 0，确认 Telegram 15 项与安装脚本 13 项（含新增 TCP 判定）实际执行；12 个本轮修改源码/脚本文件的 SHA-256 在验证后复核一致，清单位于 `target/quality-continue-sources-20260924.json`。这是 `9700949` 加现有工作树改动的证据，不是固定提交的完整 G-W 记录。

## 审查、回退与遗留

- 审查状态：已做本轮差异自审、正常/异常输入和边界回归；未声称独立审查人或用户验收。
- Git 状态：本轮未创建提交、标签或快照，未暂存他人修改。原有门禁/桌面壳/文档改动继续保留。
- 回退：在当前差异中仅反向应用本记录对应的代码和测试块，删除新增 `tests/scan_support/listing_budget.rs`，并撤回本记录及 STATUS 对应条目。共享工作树中不要用全树 reset/restore 回退。
- 完整测试仍有平台密钥库、原生构建与桌面环境依赖。局部通过不能扩大成整套安全审计、所有平台通过或发布门禁通过。
- 本轮未给并发目录替换、重解析点竞争或依赖最新漏洞状态做完整验证，未据源码注释将其记为安全保证。
