# 数据处理与选材研究独立评审回执

日期：2026-10-22。候选：`codex/soul-rebuild-20261019` @ `6a56d852d23a051c7c4e9845642791295c541105` 加本轮 dirty。协调者整理本回执；独立只读评审者为 `pipeline_review`（`gpt-6.1-sol/high`），实现者为 `selection_probe`，不是实现者自评或协调者自签业务评审。

范围为[设计与实际回执](../design/2026-10-22-data-selection-pipeline.md)、新增 `context_selection_probe.rs` 及三个子模块、共享 `task_retrieval_probe/adapter.rs` 的有界扩展。评审者先读任务和实际文件，再读运行日志与说明；直接读取 untracked 文件，不把空 diff 当作没有改动。只读相关 fixtures、parser、ranking、adapter，未扫描全库、执行 Cargo、安装、读取真实库或改文件。

## 结论

修复后的最终候选没有剩余阻塞项，符合只用合成数据、复用现有路径、保留失败、只改研究测试与文档的任务范围。支持研究代码及列明的实际运行结果，不代表生产模型接入、正式平台门禁、真人效用或合并批准。

## 首轮发现与修复复审

首轮新 replay 对适配器返回值使用 `expect`；旧适配器先断言实际 retained 等于 selected，因而正常准备拒绝或最终 HTTP 裁剪无法作为不可用条件写入并继续。该问题是静态发现；首轮 180 条短 fixture 最大请求仅 3323 bytes，未触发它，不影响首轮已记录的事实覆盖和字节结果。普通 context 预算裁剪与最终 HTTP 裁剪不同，未将所有预算路径统称不可达。

实现者新增 `observe_prefix_allow_loss`，旧 `observe/observe_prefix` 仍传 `require_full_retention=true`。新观察入口要求 selected 属于 submitted、retained 属于 selected，端点 owner 集合精确等于实际 retained 与原 owner 许可的交集；引用、来源复验、无 alias 越权和零请求仍断言。只对实际 retained 检查正文；没有捕获全部 panic 来隐去不变量缺陷。

新 `observe_row` 对返回的已建模错误记录具体 reason、`attempted_prepare=true`、null 实际观测，外层仍记录当前条件并继续。正常裁剪记录实际 ID 和两段损失，标记 unavailable；没有所选组时 `group_retained_complete=null`，不以空集合的真值冒充需求满足。

复审核对两个真实普通回归均在最终日志通过：三条各约 7000 bytes 的摘要触发 16 KiB context 预算，AND 组缺成员记不可用且同一 Harness 下一正常条件成功；`prefix_count` 错误记 null 指标并继续。后者只是观察入口参数拒绝，未冒充 Session 准备拒绝的复现；本轮没有真实触发最终 64 KiB HTTP 裁剪。

## 原始结果独立重算

原始预测只读路径：`C:\Users\86080\AppData\Local\Temp\soul-evidence-stopping-20261021-cb16ed72`。本轮路径：`C:\Users\86080\AppData\Local\Temp\soul-context-selection-20261022`。评审检查 `unit-final.log`、`clippy.log`、`format.log`、`command-exits.txt`、`final/replay.log`、`final/results.jsonl` 和 `final/summary.json`；其 PowerShell 只读重算退出 0。

- 最终四目标普通测试 33 passed、9 ignored，其中新目标 11；显式重放 1 passed。限定四目标 test profile 严格 Clippy、根格式检查均退出 0。旧首轮 31 passed 不迁移为最终数量。
- 最终 180 个唯一条件、154 次实际准备/可用、26 不可用、740 个 retained 引用。所有可用条件 requested 与 retained 一致、支持组完整、两种损失为 0，零出站请求。
- 逐字段核对机器 needs/support 和原文件、完整语义排序和原响应；q13 截断 JSON 保留。用 PowerShell 独立实现相同贪心选择核对 48 个可用支持组条件，无选择或组差异。
- 按 fixture 的任务、opaque ID 公式和可接受来源独立重算事实覆盖，无不一致。首轮与最终的身份、可用性、原因、预测、排名、选择、引用、覆盖、字节、不适用和重复指标逐项比较，差异 0；计时不要求相同。
- 有效性分母独立核准：两种支持组策略各 12/18 case 可用；共同配对子集 12 case/27 facts，来自四任务。q13 格式失败形成 6 个不可用条件；另五 case 的空支持组形成 20 个不可用条件，不解释为事实不存在。

共同 12 case 中，MMR 前缀停止为 26/27 facts、20306 bytes，支持组为 27/27、15309 bytes；语义前缀为 27/27、15558 bytes，支持组为 27/27、15363 bytes。语义固定四条在同子集为 19350 bytes。24.61%、1.25% 和 20.60% 的字节比较使用一致分母；全批结果及所有策略见设计正文，不将子集结果扩成全 18 case 成功。

评审中一次用对象 JSON 字符串比较发现字段顺序伪差异，随后按字段与数组逐项核对为零。这是核验方法修正，不是产品缺陷或预测被改写。

## 资料与范围核验

评审另核对 TypeSafe 官方模型、并行问题文档以及 BGE/Qwen 模型卡。未发现 Jev 输入计价、共享 state、别名和托管边界的具体表述错误；两个本地重排器列为待测独立候选，没有承诺当前 Windows 可用或默认堆叠。本文具体方案是结合项目约束的推断，不把发布方指标当 Soul 结果。

选择器参数不含人工真值，gold 只作事后评分；固定四/八不依赖机器映射成功。完整排列、任务/预测输入对应和整个来源池复验继续保留。没有增加 hash、冻结 contract、额外 gate、生产向量库或授权路径。

未验证：新留出任务、模型推理/校准、旧模型费用和在线耗时、冷/热延迟、峰值资源、Windows 模型部署、生产接入、最终 HTTP 裁剪的实际反例、实际 Session 准备拒绝后的逐行恢复、独立桌面/UI和正式 G-L/G-W/G-M。后验重放的 27/27 是材料覆盖，不是通用语义准确率；新模型调用为 0 不代表原始判断免费。
