# MODEL-01 实现前差集与有限合成实验

日期：2026-10-20。起点：`codex/soul-rebuild-20261019` @ `6a56d852d23a051c7c4e9845642791295c541105 + dirty`。

本片承接作者“分析并进行下一阶段的代码编写”的要求，范围选为[阶段计划](2026-10-20-next-stage-plan.md)允许的一次 MODEL 实现前差集与有限合成实验。保留已有 DATA/VAL 和文档修改，不从此请求推定提交、真实试验或生产 MODEL 实现许可。正式 MODEL-01 仍依赖 [ROADMAP 第 8 节](../ROADMAP.md#8-后续工程工作包与依赖)的 M1 用例证据与 DATA-01；本文件不是新生效的产品契约。

## 目标与选择

让下一位设计者能够用实际执行结果回答两个问题：明确候选集合中的反证是否会被关键词过滤移除；用户生效语气与相反记忆材料进入同一请求时，是否仍保持设置与参考材料的边界。只测现有代码能做什么，不给合成结果附加真人效用或模型服从结论。

三个可选方向中，本片选择有限实验：直接扩展事实有效期或语义冲突需要目前尚未满足的生产前置；重复 W2 已有编辑/遗忘测试没有新增问题依据；复用现有 Session 的合成组合可以提供具体场景证据，且不修改选择算法。

## 静态差集

| 能力 | 当前实现与已有验证入口 | 本片如何处理 |
| --- | --- | --- |
| 明确候选内选择 | `soul-memory/src/selection.rs` 的 `select_for_draft` 仅访问指定候选；已有 `soul-memory/tests/selection.rs` | 作为实验依赖，不重新实现检索 |
| 关键词与反证 | 按词面匹配过滤，不识别语义关系 | 用同一候选集合的无查询/关键词查询对照记录差集 |
| 时间有效性 | `MemoryTimeScope::AtTime` 返回 `Clarify`；UUID 时间不是事实时间 | 验证该实验中的时间问题仍拒绝生成可消费上下文，不推断哪条记录最新 |
| 关联来源与引用 | 来源/证据非空的 Memory 暂不支持；引用指向当前 Memory 及其两个密封字段 | 不新增来源解析器，不把能解密等同于事实已核验 |
| 纠正、编辑与遗忘 | profile 保留用户生效值；记忆消费/派发/完成时复验；现有 Session 测试覆盖编辑与遗忘 | 复用既有回归，新增的组合仅观察语气设置和记忆材料同请求共存 |
| 事实/自述/推测、情境和过期 | 尚无完整生命周期与冲突语义；现有 notice 明示限制 | 保留为后续设计问题，不在实验工具中实现第二套模型 |

调查没有发现上述已声明边界的具体违约。以下代码是特征观察，不伪称先失败再修复生产缺陷。

## 实现计划与路径

单个测试入口 `crates/soulcore/tests/model01_boundary_probe.rs`，默认忽略，显式运行才执行。使用 Rust 1.83 和已有依赖；复用真实 `Session`、临时 SQLCipher 库、`NewMemory`、`MemoryContextRequest` 与现有 loopback `MockLlm`，不新增模型、HTTP client、schema 或持久数据类型。

1. 写入三条固定合成记忆。A 表示场所开放，B 表示同一场所关闭，C 是范围外相关记录。只允许 `[A, B]`：无查询时保留两者，查询“开放”时仅保留 A，B 的排除原因为 `KeywordMismatch`。以实际返回 ID 核对引用，避免只核计数。时间问题应返回 `Clarify` 且没有可消费 context。
2. 在独立临时 Session 设置用户正式语气，写入相反随意语气材料。选择该材料，另行明确同意本人材料原文，按精确计划批准发到本机 mock。检查实际请求中的 profile 参考材料与独立记忆数据，检查用户生效设置未被反写。两者都在引用的 user material 中，system 保持既有固定起草指令；mock 的固定回复不作为模型服从证据。
3. 仅输出固定场景名、枚举、计数和布尔值构成的 JSON，并标明 `synthetic=true`、`human_evaluation=false`。不输出正文、查询、标识符、端点或原始请求；不生成文件输入、任意输入或真实模型选项。
4. 主线程统一运行入口、严格定向 Clippy、根格式及受影响既有回归，记录实际命令/退出码。独立评审先看任务和候选代码，再核结果；有具体发现才修复和复验。

仅实现者写上述新测试文件。主线程写本文件、阶段计划、STATUS 与 PLAN_INDEX；这些路径已有修改按开工内容保留。当前 checkout 含未提交依赖，继续使用它，不创建遗漏这些依赖的新 checkout。无生产文件、锁文件、schema、IPC、前端或算法改动。

## 执行入口与验收

```pwsh
$ErrorActionPreference = 'Stop'
cargo test -p soulcore --test model01_boundary_probe --locked --offline report -- --ignored --exact --nocapture --test-threads=1
if ($LASTEXITCODE -ne 0) { throw 'MODEL 有限合成实验失败' }
cargo test -p soulcore --test session_memory_context --test profile_memory_commands --locked --offline
if ($LASTEXITCODE -ne 0) { throw '记忆上下文与用户纠正回归失败' }
cargo clippy -p soulcore --test model01_boundary_probe --locked --offline -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'MODEL 实验静态检查失败' }
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw 'Rust 格式检查失败' }
```

上述命令不包含 Windows MSVC/OpenSSL 的本机环境初始化；使用本机已有工具链，不安装依赖、不更新锁文件、不修改全局环境或 target 路径。日志留仓库外。验收只要求固定场景实际执行、报告忠实反映观察、无正文输出，以及独立评审发现已处理；不新增 gate、hash 或正式关闭条件。

## 结果解释与后续边界

关键词场景即使全部断言通过，也仅表明现有词面选择按其约定运行，不能称为冲突检测通过。语气场景即使全部断言通过，也只证明请求构造和用户设置保存边界，不能证明真实模型服从、材料真实或个性化有效。

生产 MODEL 后续应由真实用例决定是否需要保留反证、如何表达事实有效时间、怎样区分自述和推测，以及如何在用户纠正、用途与遗忘边界内使用材料。这些不是本轮新增的验收标准，也不因此启动 N2/N3、真实端点或真人效用试验。G-L 暂缓，系统 G-M 与安装后置；完整 G-W、独立 desktop workspace 和产品 UI 未由此实验验证。

## 本轮执行记录

实现仅新增 `crates/soulcore/tests/model01_boundary_probe.rs`，一项默认 ignored 的 `report` 测试包含上述两种观察。`owner_brief` 为私有方法，因此通过实际请求与 `Session::profile().voice` 前后值验证其消费结果，没有为实验暴露新 API。输出只有固定元数据，引用、合成正文与 HTTP 请求只在测试内存中用于断言。

本轮 Windows 实际观察：无关键词时两条候选均保留；关键词过滤后只保留一条，反证记录明确被 `KeywordMismatch` 排除；时间问题返回 `Clarify` 且没有可消费 context。另一个临时 Session 中，正式语气与相反记忆材料保持分节，固定 system 指令未被替换，用户锁与生效 voice 未变；精确批准产生 1 次 loopback 请求，重放批准未新增请求。没有依据将这些结果记为真实模型服从或质量增益。

环境为 Rust/Cargo 1.83、Windows MSVC、pwsh 7.6.5；复用已有静态 OpenSSL，进程内 jobs=4、DEV/TEST DEBUG=0、incremental=0。证据目录为仓库外 `E:\Project\Soul-checks-20261020-model-preflight\`，`env.ps1` 与 `environment.json` 保存环境；未设置新的 target 路径或修改全局配置。

| 实际检查 | 本轮结果 | 原始记录 |
| --- | --- | --- |
| `cargo test -p soulcore --test session_memory_context --test profile_memory_commands --locked --offline` | 7 passed / 0 failed，exit 0；相关生产源码本轮未改 | `existing-regression.log` / `existing-regression-result.json` |
| 本文显式 `report` 命令 | 1 passed / 0 failed，exit 0；2 个固定场景 | `model-probe-attempt1.log` / `model-probe-attempt1-result.json` |
| 定向严格 Clippy | exit 0 | `model-probe-clippy.log` / `model-probe-clippy-result.json` |
| 首次根格式检查 | exit 1，仅新文件 3 处换行格式；不是业务断言失败 | `fmt-attempt1.log` / `fmt-attempt1-result.json` |
| 修订后的根格式检查 | exit 0；仅按检查结果调整新文件换行 | `fmt-final.log` / `fmt-final-result.json` |
| 默认模式 `cargo test -p soulcore --test model01_boundary_probe --locked --offline` | 0 passed / 0 failed / 1 ignored，exit 0；未自动运行实验 | `model-probe-default.log` / `model-probe-default-result.json` |
| 格式修订后的显式 `report` 命令 | 1 passed / 0 failed，exit 0；实际执行 0.40 秒，重跑不累计为更多场景或样本 | `model-probe-final.log` / `model-probe-final-result.json` |

最终输出已解析为 JSON，核对合成标识、两个观察项、未实现生产模型/未进行真人评估的标识，并检查没有所用正文、URL 或 UUID；仓库外 `observations.json` 保存该纯元数据。此有限检查不宣称穷尽任意输入的日志泄漏；本入口只接受代码内固定材料。

本片没有生产修复，首次行为运行即通过，不把特征观察写成红绿修复证据。未运行真实端点、真人样本、Linux、完整 G-W、独立 desktop workspace、UI/通知或安装验收；不续签历史平台通过记录。

独立只读评审 `/root/model_review` 已核对本片新源码、必要 Session/材料/批准上下游、原始运行日志及文档，未发现具体功能、隐私或结论真实性阻塞。评审提出的最终格式回执和 STATUS 当前摘要两项收尾已定向复核关闭；没有重复全库扫描。本片状态为 **REVIEWED，开发范围验证完成**；候选仍是 `6a56d85 + dirty`，未提交，未标正式 GATED/INTEGRATED。相关当前状态仅在 [STATUS](../STATUS.md) 维护。
