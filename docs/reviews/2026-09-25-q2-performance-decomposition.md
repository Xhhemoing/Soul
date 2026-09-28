# Q2 性能分解测量记录 — 2026-09-25

## 结论

本轮在测试层为 Q2-03 增加了第二条真实 SQLCipher 诊断路径，将合成导入拆分为解析、加密导入写入、图谱重建、事务总时间和事务残差。现有 `Session` 路径继续提供端到端上下文。此记录是单机描述性测量，不是性能门禁，也不证明产品已经优化。

四组正式样本中，`encrypted_import_write` 的中位数都是分解事务内最大的阶段。peer 数从 10 增加到 100 时，`graph_rebuild` 中位数在 1,000 条消息下从 58.0396 ms 增至 88.2075 ms，在 10,000 条消息下从 715.0674 ms 增至 1,228.4537 ms。该结果支持后续分别细分加密写入和按 peer 数控制变量测量图谱重建；本轮不据此修改产品实现。

## 范围与源码身份

| 项目 | 值 |
|---|---|
| 基线提交 | `b40a5770fe18c3366771db8ee83b53ef306d13e5` |
| 设计与计划提交 | `0ba34aa38d1e7e24a40c3fae0d079abc580cfec6` |
| 真实事务分解提交 | `dd2b2f302d992d30277ba0a4656c6ecfc615e6e3` |
| JSONL v2 与测量源码提交 | `2d9661c80681ebb32acf82232cead2bda05c75f4` |
| 分支 | `codex/q2-perf-decompose-20260925` |
| 产品源码身份 | `ebff0c96325e0f297a1c397de6a3a1421fdd369d`，本轮未修改 |
| 实现文件 | `crates/soulcore/tests/q2_scale.rs` |
| 新增生产依赖、schema、IPC、算法、索引、线程池、UI 或 AC | 无 |

直接分解路径在事务外调用现有解析器，在同一个真实 `SqlCipherStore::transact` 内分别计时 `import_commands::commit` 和 `graph_commands::rebuild`。它使用独立的临时数据库，并有意排除私有且在事务后执行的 `Session::sync_identifiers`。因此，直接路径与 `Session` 路径只能并列观察，不能相减或视作同一调用链。

`transaction_overhead_residual` 的定义是：

```text
transaction_total
- encrypted_import_write
- graph_rebuild
```

该残差包含 `BEGIN IMMEDIATE`、`COMMIT`、计时器开销和闭包间隙，不能标记为纯 commit 或 fsync 时间。

## TDD 与执行记录

| 检查 | 结果 |
|---|---|
| 变更前定向 Q2-03 测试 | 1 项通过，0 失败；首次独立构建约 29m04s，测试本身约 0.48s |
| 新契约 RED | 编译按预期以 `E0425` 失败；唯一缺失符号为 `observe_decomposed`，退出码 101 |
| 新分解测试 GREEN | 1 项通过，0 失败；使用真实解析器、SQLCipher 写入、图谱重建和事务 |
| 原普通 Q2-03 测试 | 1 项通过，0 失败 |
| Task 2 普通测试 | 2 项通过，0 失败；显式测量测试被过滤 |
| 格式检查 | `cargo fmt --all -- --check` 通过 |
| 显式测量 | 退出码 0，用时 137.51s；4 组案例、4 次预热、20 次正式试验、1 个完成记录 |

显式测量使用已提交且干净的源码 SHA：

```powershell
$env:CARGO_TARGET_DIR = 'E:\Project\Soul-target-q2-perf-20260925'
$env:SOUL_Q2_SOURCE_SHA = '2d9661c80681ebb32acf82232cead2bda05c75f4'
cargo test -p soulcore --test q2_scale measure_four_synthetic_scales_with_five_independent_trials --locked -- --ignored --exact --nocapture
```

## 证据与结构验证

| 证据 | 值 |
|---|---|
| JSONL | `D:\Soul-q2-performance-evidence-20260925\measurement\q2-scale-1790318280597512000.jsonl` |
| JSONL 大小 | 41,660 bytes |
| JSONL SHA-256 | `255BD8163A1E17014BD75A1BFE17F761ABBB0D66BD940D1C1BB091D5C15EC75B` |
| 验证回执 | `D:\Soul-q2-performance-evidence-20260925\validation.json` |
| 验证回执大小 | 9,245 bytes |
| 验证回执 SHA-256 | `DAFF3120A6FDEA029890ACCF3599D87F4830916B048A2E089E727CCCE2F0D2F9` |
| 验证结果 | PASS |

验证器逐行执行 `ConvertFrom-Json`，确认共 30 条记录：1 个 `run`、24 个 `sample`、4 个 `summary` 和 1 个 `complete`。它还确认：

- schema 为 `soul-q2-scale-v2`，全部记录的源码 SHA 一致；
- 每组案例含 trial 0 的一次预热和 trial 1–5 的五次正式试验；
- 每个样本完整包含 9 个 `Session` 阶段和 5 个分解阶段；
- 每个样本的消息、证据、联系人、边、研究排除和审计计数符合预期；
- 每个样本的事务残差算术成立；
- 每组汇总的 min、median、max 可从五个正式样本逐项重算；
- 完成记录为 4 组案例、4 次预热和 20 次正式试验。

## 正式样本中位数

单位为毫秒。每个值来自对应案例的五次正式试验；预热样本不参与汇总。
每个阶段单独取中位数，因此各阶段中位数不要求与事务总时间中位数相加相等。

| 消息 / peers | Session commit including rebuild | parse | encrypted import write | graph rebuild | transaction residual | transaction total |
|---|---:|---:|---:|---:|---:|---:|
| 1,000 / 10 | 324.3374 | 18.6011 | 236.2695 | 58.0396 | 27.9912 | 326.7212 |
| 1,000 / 100 | 359.1684 | 16.0268 | 224.1595 | 88.2075 | 30.7744 | 347.3583 |
| 10,000 / 10 | 3,940.5447 | 97.8976 | 2,936.6848 | 715.0674 | 511.7222 | 4,130.1267 |
| 10,000 / 100 | 4,465.9088 | 92.1168 | 2,600.0556 | 1,228.4537 | 503.7576 | 4,349.6299 |

## 测量环境与限制

- 主机为 Windows 11 `10.0.26200`，AMD Ryzen 7 H 260，8 核 16 线程，约 16 GB 可见内存。
- 工具链为 `rustc 1.83.0`、`cargo 1.83.0` 和 `pwsh 7.6.5`。
- 使用工作区 test profile（`opt-level=1`），debug assertions 开启；构建时间不计入阶段时长。
- 每个观察使用新的临时 SQLCipher 数据库；OS page cache、后台调度、文件系统状态和温度状态未控制。
- 每个 trial 固定先运行 `Session` 观察、再运行分解观察，顺序和页面缓存影响未控制；两条路径不能作为配对等价样本比较。
- 数据是固定的合成双人对话；profile 和 memories 为空。未覆盖真实用户数据、重复导入、IPC、UI、WebView、安装器或 Linux。
- 每组只有五个正式样本。本记录不提供 p95、硬件门槛、跨平台结论或真实用户性能结论。

## 审查

三名只读 agent 分别被分配范围、测试质量和测量有效性审查。经过有界等待、打断和压缩任务后仍未生成回执；按协调指令记录并跳过该阻塞点，未将其计为独立审查通过。

主线程随后完成三项显式 fallback 审计。回执均标记 `independent=false`，结论为 PASS，Critical/Important finding 均为空：

| 审计 | 回执 | SHA-256 |
|---|---|---|
| 范围 | `D:\Soul-planning-evidence-20260925\q2-perf-scope-review.json` | `CA623B4B365454A3A1D05F778D12CC27BD9D400D423FF93DD23AE83FA5D0C73A` |
| 测试与代码质量 | `D:\Soul-planning-evidence-20260925\q2-perf-test-review.json` | `50B00D0569C964BF0EB2BDD1ED8AAF82B572FFE34A0B00DF947CCA6F643EE5B9` |
| 测量有效性 | `D:\Soul-planning-evidence-20260925\q2-perf-measurement-review.json` | `71DF2257BE7931A9F668E50669811AFF8C5D9E6AA166C7EEF84BFAC077772668` |

记录的非阻塞边界包括：汇总函数固定为五个正式样本；直接路径依赖命令返回计数，而图谱回读、证据、隐私和审计完整性由原 `Session` 测试覆盖；固定执行顺序及缓存影响未控制；阶段中位数分别计算，不能相加解释。

## 最终校验

在与显式测量相同的 Cargo 构建环境下完成：

| 检查 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | 通过，退出码 0 |
| `cargo test -p soulcore --test q2_scale --locked -- --skip measure_four_synthetic_scales_with_five_independent_trials` | 2 项通过，0 失败，1 项显式测量被过滤；0.35s |
| `git diff --check` | 通过，退出码 0 |
| JSONL 验证 | PASS |
| 文件范围 | 没有计划外的已提交或未提交文件 |

机器可读回执：`D:\Soul-q2-performance-evidence-20260925\final-verification.json`。

## 后续决策

若继续 Q2 性能工作，先在同样的测试边界内把 `encrypted_import_write` 分解为联系人、事件、加密正文、interaction evidence 和审计写入，并为图谱重建增加固定消息数、递增 peer 数的控制变量样本。任何产品优化提案都应建立在新的分解证据上，并单独验证正确性与收益。

本记录不关闭 Q2-00、正式 Q2-05、Goal 1 或 Goal 2。
