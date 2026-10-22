# 算法执行与架构成本优化

> 实施方式：主会话统一调度，按 subagent-driven-development 分配独立写路径；两项候选完成必要语义测试、同条件前后测量和独立只读评审。遵守项目既有目录和授权，不自动提交。

日期：2026-10-21。用户要求下一阶段侧重算法和架构优化，继续由主会话统筹、子 Agent 使用 `gpt-6.1-sol/high`、记录尤其是结果分析。开工实读版本 `codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`，保留全部既有修改。当前事实入口为 [STATUS](../STATUS.md)，此前覆盖依据为[综合测试](2026-10-21-comprehensive-validation.md)。

本文首批实现与结果保留原执行范围；后续用户授权的查询、准备与人物展示优化追加在[第二批](#第二批查询准备与人物展示)，不覆盖首批测量或把历史通过转给新候选。

**目标：** 在冻结规则、用户值、证据和授权/事务结果相同的前提下，减少图重建的重复查找，以及研究保存发布复验的无用派生计算。收益由本轮改前/改后实际数据判断，不能从复杂度直接承诺端到端倍数。

**架构：** 第一项在单次 rebuild 生命周期内以局部有序索引复用既有边/推断，不持久缓存。第二项复用同一候选扫描与来源快照选择逻辑，prepare 完整分类构桶，发布仅执行真正消费的快照复验。两个工作面不改变共享锁顺序、事务协议或公开 API，可独立认领。

**技术栈：** Rust 1.83；既有 BTreeMap、soul-store-api FakeStore、SQLCipher、Tauri；不新增依赖、schema、hash、冻结 contract、baseline 或 gate。

## 边界与取舍

- T4D/A0/A1/A2/A3 常量和规则来自[算法决定](../algorithms/DECISION.md)，D108 的坏时间整体拒绝、全库 as_of、用户纠正与机器值分离继续成立；本批不改评分、语义冲突识别或 A1 的既定能力。
- 研究保存仍是一次明确批准、精确来源复验、原 BEGIN IMMEDIATE 与文件发布/COMMIT 边界。政策、绑定、CK 存活与遗忘失效都必须进入同样的来源比较；不解封正文，不移动文件写入或取消审计。
- 只使用合成 fixture/临时库，普通测试仍排除 AGENTS 指定的真实采集源；无真实模型、真实用户库、安装、提交或平台门禁启动。G-L 暂缓。
- 在当前 dirty 开发分支按路径认领操作，保持原两个 workspace 与 target；不复制或提交既有工作来制造干净候选。仅改认领路径，修改前内容快照与运行日志放仓库外。
- 选择局部计算复用，是因为已有明确重复工作且能保留接口；持久缓存/增量重建需要额外失效规则，重排锁和文件发布会涉及授权时序，均不纳入首批。时间戳单次解析和 JSON 分配削减属于其他可选项，本批不混入以免模糊收益来源。

## 调查证据

算法调查定位 `soul-graph/src/build.rs` 对每个 peer 线性查找既有 relationships/inferences：旧查找部分为 O(P×(E+I))，推断谓词还反复格式化关系 UUID。既有 SQLCipher 列表有确定排序，当前 `.find()` 选择第一个匹配项。T4D 已按 peer 分组评分，不能把该查找成本归因于冻结评分规则。现有 growth fixture 固定 10 peers，无法展示 P 随规模增长时的二次方触发形状。

架构调查定位 `research_retention::publish_research_with_store` 在事务内再次调用完整 prepare，随后只比较 `current.sources`。新 dataset 的 rows/counts 没有被消费。完整扫描不解封正文，`opened_store()` 只是 Arc clone；不能将其误写成反复解密正文或重开连接。来源集合只排除 doc/index 时间一致、canonical 且确证范围外的行，坏 JSON/政策拒绝/不可用来源仍必须保留。

## 任务 1：图重建局部查找索引

负责人 `graph_lookup_optimization`，独立评审由非实现者完成。允许改动：

- `crates/soul-graph/src/build.rs`。
- 新增 `crates/soul-graph/tests/rebuild_lookup.rs`：first-match 与拒绝范围反例。
- 新增 `crates/soul-graph/tests/rebuild_scale_probe.rs`：默认 ignored 的有限规模观察。

实现约束：按原 existing 列表顺序建立 `BTreeMap<Uuid, &SoulRelationship>`，只为连接 owner 的边选择另一端为键，使用 `entry(...).or_insert(...)` 保留首项。推断仅索引现有 tie 前缀及字符串 target，以原始 relationship_id 字符串为键，同样保留首项；禁止先解析 UUID 再归一。peer 遍历、`read_strength` 调用位置、写入顺序与有序 evidence 均不变；不预解析无关边。

- [x] 先补可在旧实现通过的等价保护：重复双向边首项锁值/ID；重复推断首项 ID/verdict；target 字符串精确匹配；无关坏 tie_strength 不引入新拒绝。它们是重构保护，不伪称已发现功能 bug 或 RED。
- [x] 完成内存 FakeStore 的 100/1000/10000 peers、三次独立 store 诊断。每个 peer 固定合法观测；先 rebuild 形成旧 E/I，再计时无新数据的 rebuild；构造/预备/断言/输出在计时外，不调用 people。比较前后完整边/推断结果，而非仅返回成功。
- [x] 主会话在旧生产代码上运行普通用例及显式诊断并保留原始值。
- [x] 实施局部索引，运行相同用例/诊断与既有冻结 fixture、纠正、时间和跨层回归。
- [x] 独立核对 first-match 反例、字符串匹配、错误范围、O(E+I) 额外局部索引空间及实际收益。FakeStore 含 schema/内存读写，不能将其称为纯评分耗时或真实 SQLCipher 性能。

## 任务 2：研究发布只计算所需的来源复验

负责人 `retention_revalidation_optimization`。允许改动：

- `crates/soul-store/src/research_retention/query.rs`、`research_retention.rs`。
- 对应既有 `research_retention/tests.rs` 与 `tests/research_retention.rs`，只补具体语义差集。

内部数据流：统一扫描产生精确 `SourceSnapshot` 集合、候选总数与范围外计数；prepare 在该遍历上执行原分类、schema/绑定检查和桶计数；发布只取得同样快照并与 held 来源比较。私有 helper 可使用 visitor，公开 `prepare_research_retention`/`publish_research_retention` 不变。

`CANDIDATES` SQL、排序、100000 候选/16 MiB 元数据限制与执行顺序保持。坏 JSON 也要 push snapshot；唯一来源排除条件仍为时间一致+canonical+范围外。不能复制两套来源规则、跳过政策/坏类型的来源快照、删除 doc.source 防篡改分支或以旧批准替代当前读取。跳过分类前先证明分类中可能返回的错误语义：固定枚举序列化、schema 失败仅排除、计数由已有上限约束；若存在真实错误差异则先停止该候选。

- [x] 先复核现有同 ID 绑定替换、遗忘同桶替换、范围外变化、上限、不解封正文、第二 writer 和发布后 COMMIT 失败测试。
- [x] 补必要反例：准备时已排除但仍相关的来源变化使批准失效；范围外变化允许但全局上限仍生效。先在旧实现运行，不写耗时断言。
- [x] 抽取统一扫描，发布仅保留精确快照比较；不混入额外分配优化。
- [x] 运行原研究保存/事务/取消反例和真实 handler 合成回归；同一增长实验三次新临时库复测。
- [x] 独立检查来源集合等价、所有拒绝与副作用、原事务和取消/发布时序，再结合测量决定是否保留改动。

## 测量与验证安排

只有主会话执行 Cargo/完整测试；两名实现者先写各自测试并等待主会话授权第二阶段。所有子 Agent 均为 `gpt-6.1-sol/high`，只读调查/评审不自启重编译。主会话负责 STATUS、PLAN_INDEX、方案和结果记录。

本轮原始目录：`C:/Users/86080/AppData/Local/Temp/soul-optimization-8a6a5c6bd1904d39b7691bae7e9d7df0/`。Windows 11 10.0.26200、Ryzen 7 H 260、16 逻辑处理器、可见内存 16004104 KiB；Rust/Cargo 1.83.0、PowerShell 7.6.5、Python 3.13.14。沿用既有 target/OpenSSL，子进程 JOBS=4、DEV/TEST DEBUG=0、INCREMENTAL=0、SOUL_RESEARCH_TRIALS=3。根测试使用仓库既有 `[profile.test] opt-level = 1`，原始日志为 `[optimized]`，不是 release 测量；DEBUG=0 只去调试信息。没有改全局配置或控制系统缓存/后台负载。

| 验证面 | 主会话执行方式 | 解释边界 |
| --- | --- | --- |
| 改前/改后 SQLCipher 端到端 | `cargo test -p soulcore --test research_growth_probe --locked --offline -- --ignored --exact cumulative_growth_and_zip_costs --nocapture` | 三场景×三轮×100/1000/10000；独立 Python 读取真实 ZIP；一次 12000 稀疏行拒绝；固定 10 peers，不承担高 P² 证明 |
| graph 等价性 | `cargo test -p soul-graph --test rebuild_lookup --locked --offline` | 重复首项、UUID 字符串、坏旧边范围；不改变冻结算法 |
| graph 成本 | `cargo test -p soul-graph --test rebuild_scale_probe --locked --offline -- --ignored --nocapture` | 明确内存路径，先建立旧 E/I，再测同库重复重建；无性能阈值 |
| 研究复验 | `cargo test -p soul-store --lib research_retention --locked --offline`；`cargo test -p soul-store --test research_retention --locked --offline` | 精确来源与事务反例，以真实临时 SQLCipher 执行为主 |
| 集成回归 | 根 workspace all-targets 排除 `windows::runs_on_windows`；独立桌面按 AGENTS 排除两项真实采集 IPC | 只在最终生产候选稳定后统一运行，不因每次文字修改重复全量；前端源码不在本批写范围 |
| 编译质量 | 相关 workspace 严格 Clippy 与格式检查；普通 core 库编译 | 不降 lint，不更新依赖/锁文件；独立桌面结果单列 |

先确认输出完整相同、拒绝和副作用一致，再分析时间。报告每个形状/阶段全部样本及 min/median/max、相对变化；不过滤慢样本，不把首次编译计入操作耗时，不将 3 次样本称为 p95。若某条端到端路径收益不明显，明确记载，不用局部倍数掩盖。是否优化查询/取消压力由本次实际改动影响判断；本批保留原 prepare 扫描和原锁协议，不预先承诺停止响应改善。

## 执行记录

- 开工阶段完成 Git/状态核对、两线只读定位及方案取舍；先执行旧实现测量，再授权各生产文件改动。
- 旧实现三轮增长实验 `before-growth.log` exit 0，1 项显式诊断 passed / fixture 1 filtered，27 个成功检查点、18 次真实 ZIP 保存/独立读取和 1 个 12000 行拒绝。此轮万事件保存阶段之和中位数：稀疏 1167.20 ms、密集 1114.23 ms；exec 阶段分别 613.20/570.92 ms；不借用第五批历史样本作为改前数据。
- 旧实现 `before-retention.log`：10 passed / 60 filtered，包含 3 项新语义保护；`before-graph-regression.log`：4 passed。`before-graph-scale.log`：1 个显式诊断 passed，9 条记录全部完整模型断言通过。100/1000/10000 peers 的中位重建时间分别 7.46/211.27/25044.83 ms，明确是 FakeStore 路径。
- 两名实现者随后仅改三个生产文件。`graph-build-before.rs`、`retention-query-before.rs`、`retention-module-before.rs` 保存本轮修改前内容；保留原 D108 与其他 dirty 工作。独立评审先看实际差分，确认 first-match、精确字符串、错误范围、SQL/限额/来源集合及事务时序无具体发现；最终运行证据仍需单独核验。
- 改后定向 `after-graph-regression.log` 4 passed，`after-retention-unit.log` 10 passed，`after-retention-integration.log` 7 passed，均 exit 0。
- 首次 `run-after.ps1` 通过 PowerShell 函数转发参数，函数消费了 `--`；`after-graph-scale.log` 实际命令因 Cargo 不认识直接传入的 `--ignored` 而 exit 1，没有进入测试。此前 fmt 的实际命令为 `cargo fmt --all --check`，exit 0。保留原脚本/失败日志，改用数组直接转发给原生 pwsh，并以 `native-arguments-probe.ps1` 验证分隔符保留。后续用独立日志 `after-graph-scale-retry.log` 重跑，最终根 fmt 另执行规范完整命令；不修改产品或测试来规避调度失败。
- 改后 `after-graph-scale-retry.log` exit 0，9 条记录全部完成完整模型断言；`after-growth.log` exit 0，27 个成功检查点、18 次 ZIP 保存/独立读取与 1 次稀疏 12000 行拒绝。独立分析纠正任务卡曾将测试配置误写为 unoptimized 的描述：以 Cargo.toml 与实际日志为准，本页采用 opt-level=1；不影响原始样本和前后比率。

## 前后测量与分析

下表单位均为毫秒；每格为 **min / median / max**，每档三次，百分比为未舍入中位数的 `(after / before - 1) × 100%`。不丢弃慢样本，不按运行次序硬配为同一系统负载，不计算 p95。原始全部样本在同目录的前后日志，主会话另保存 `growth-all-samples.json`、`growth-statistics.json` 便于复算；这些临时测量材料不构成新 baseline 或门禁。

### 图重建：高 peer 数的重复查找

| peers | 改前 min / median / max | 改后 min / median / max | 中位变化 | 改前/改后中位比 |
| ---: | ---: | ---: | ---: | ---: |
| 100 | 6.3521 / 7.4556 / 8.9333 | 5.7372 / 6.2465 / 7.8947 | −16.22% | 1.19× |
| 1000 | 193.0861 / 211.2711 / 246.3371 | 83.7543 / 90.2707 / 228.2032 | −57.27% | 2.34× |
| 10000 | 24770.9605 / 25044.8281 / 25946.7447 | 892.0488 / 911.6791 / 943.7332 | −96.36% | 27.47× |

每次独立 FakeStore 有 P 个 peer、P 条既有关系、P 条既有推断、2P 次观测及 P+1 条 evidence；每个 peer 的一条 evidence 合并双方向 source_ref，另有一条真实用户纠正证据。构造、初次 rebuild、纠正、settle rebuild 都在计时外；只计第三次真实 rebuild。计时后完整 relationship/inference 模型与 settle 后相比保持相同，返回 ID 列表、2P 读取数和零未解析 peer 也有断言。随机初建 ID 使两次独立进程的模型不同，因此这不是跨版本逐字节相等证明；等价反例由新旧均通过的四项定向测试与既有冻结用例共同覆盖。

10k 档前后区间不重叠，支持消除重复查找带来显著改善。100 档区间重叠；1000 档改后 228.2032 ms 慢样本完整保留，不能说每次都变快。局部索引增加 O(E+I) 的键/引用空间，旧向量仍持有对象；未量峰值内存。该实验计入 FakeStore 列表克隆、serde/schema 与写回，排除 SQLCipher、Session 事务、审计、辅助来源同步和 people 查询，**27.47× 仅是这档内存路径的中位比，不能当作实际导入倍数**。

### SQLCipher：同库增长与保存

前后每轮均为三种形状各三次新库，库内顺序增长 100→1000→10000。导入固定 10 peers；稀疏每事件占一小时，密集按分钟进入小时桶。18 个保存点全部实际产生 ZIP 并经独立 Python 读取，每次都断言事务关闭、事件计数、行数及重开 dataset 一致；该诊断没有审计链断言。另一个稀疏 12000 行点拒绝保存，不并入成功耗时分布。

`commit` 包括 rebuild；`people` 为展示查询；`query` 是留存查询；`prepare` 包含查询、编码与预检；`execute` 包含编码、来源复验、写盘、同步与发布；`save sum` 为 prepare、begin、execute、complete 的和，排除用户等待、断言与独立读取器进程。各阶段中位数之和一般不等于每轮阶段和的中位数；不能将 execute 称为纯 I/O 或内部持锁时间。

| 形状 | 累计事件 | 阶段 | 改前 min / median / max | 改后 min / median / max | 中位变化 |
| --- | ---: | --- | ---: | ---: | ---: |
| import | 100 | commit | 33.10 / 37.12 / 52.19 | 38.18 / 39.41 / 50.13 | +6.16% |
| import | 100 | people | 1.92 / 2.38 / 2.94 | 1.82 / 2.36 / 3.15 | −0.74% |
| import | 1000 | commit | 296.75 / 358.85 / 399.01 | 277.26 / 317.52 / 655.25 | −11.51% |
| import | 1000 | people | 33.31 / 35.90 / 43.47 | 33.50 / 42.91 / 43.62 | +19.53% |
| import | 10000 | commit | 4067.95 / 4294.64 / 4361.62 | 3750.07 / 3895.11 / 4096.89 | −9.30% |
| import | 10000 | people | 773.69 / 801.35 / 859.12 | 714.67 / 719.33 / 805.16 | −10.24% |
| sparse_hour | 100 | query | 3.95 / 4.22 / 4.27 | 3.14 / 4.87 / 5.81 | +15.35% |
| sparse_hour | 100 | prepare | 4.79 / 6.49 / 11.68 | 4.72 / 6.15 / 9.17 | −5.29% |
| sparse_hour | 100 | execute | 10.01 / 10.23 / 17.64 | 5.88 / 6.38 / 13.43 | −37.60% |
| sparse_hour | 100 | save sum | 19.00 / 23.96 / 24.94 | 12.67 / 13.61 / 24.88 | −43.20% |
| sparse_hour | 1000 | query | 36.67 / 47.01 / 51.32 | 34.09 / 35.44 / 35.57 | −24.61% |
| sparse_hour | 1000 | prepare | 33.43 / 35.43 / 41.25 | 41.89 / 43.02 / 53.03 | +21.44% |
| sparse_hour | 1000 | execute | 43.63 / 44.29 / 48.04 | 24.07 / 24.83 / 26.28 | −43.94% |
| sparse_hour | 1000 | save sum | 79.47 / 81.36 / 90.69 | 68.45 / 71.05 / 79.22 | −12.66% |
| sparse_hour | 10000 | query | 530.73 / 541.12 / 561.25 | 480.58 / 523.27 / 539.75 | −3.30% |
| sparse_hour | 10000 | prepare | 522.16 / 548.93 / 652.17 | 485.41 / 525.96 / 550.11 | −4.18% |
| sparse_hour | 10000 | execute | 572.02 / 613.20 / 675.51 | 336.39 / 376.30 / 392.40 | −38.63% |
| sparse_hour | 10000 | save sum | 1096.26 / 1167.20 / 1330.41 | 866.68 / 879.66 / 928.13 | −24.64% |
| dense_minute | 100 | query | 3.34 / 3.82 / 6.68 | 4.34 / 4.48 / 5.23 | +17.30% |
| dense_minute | 100 | prepare | 3.91 / 5.10 / 6.24 | 4.88 / 5.92 / 5.97 | +15.94% |
| dense_minute | 100 | execute | 6.70 / 7.79 / 7.97 | 5.22 / 6.24 / 8.83 | −19.86% |
| dense_minute | 100 | save sum | 12.70 / 14.32 / 16.21 | 11.90 / 16.16 / 16.34 | +12.83% |
| dense_minute | 1000 | query | 34.29 / 35.66 / 40.59 | 35.23 / 37.34 / 37.82 | +4.72% |
| dense_minute | 1000 | prepare | 36.27 / 38.84 / 43.03 | 36.29 / 40.35 / 46.05 | +3.89% |
| dense_minute | 1000 | execute | 37.92 / 43.58 / 59.80 | 26.35 / 26.84 / 28.45 | −38.42% |
| dense_minute | 1000 | save sum | 76.39 / 84.10 / 104.75 | 65.39 / 70.92 / 74.22 | −15.68% |
| dense_minute | 10000 | query | 430.66 / 528.04 / 529.41 | 498.92 / 507.08 / 521.13 | −3.97% |
| dense_minute | 10000 | prepare | 479.99 / 538.69 / 552.08 | 468.51 / 473.35 / 503.85 | −12.13% |
| dense_minute | 10000 | execute | 539.83 / 570.92 / 576.20 | 335.24 / 348.73 / 394.67 | −38.92% |
| dense_minute | 10000 | save sum | 1023.86 / 1114.23 / 1130.61 | 820.05 / 840.90 / 870.41 | −24.53% |

独立分析还逐项比对 shape/trial/checkpoint、delta、counts、contacts/ties、桶数、ZIP 字节、事务关闭及拒绝标志，前后全部一致。稀疏 100/1000/10000 桶分别生成 10731/94434/931437 字节 ZIP；密集 2/17/167 桶分别为 1617/3030/17133 字节。结构与规模一致不意味着随机 UUID、密钥或文件逐字相同。12000 行拒绝仅前后各一次，目标文件均不存在，不能用于三次统计或反推精确最小拒绝行数。

万事件时 execute 两种形状中位数均下降约 39%，前后区间不重叠；save sum 约下降 25%。与只移除第二次派生构桶/验证的实现一致，支持保留本次来源复验优化。两项改动在同一候选生效，运行顺序也不是随机交错，不能仅凭本批数据严格隔离每项改动的全部因果贡献。固定 10 peers 的导入没有表现出 FakeStore 高 P 档的倍数收益；小规模 commit、people、query/prepare 以及密集 100 的 save sum 存在变慢，不隐藏也不从三次运行推断新的回归根因。query 与 prepare 的核心工作保留，变化不应归因于发布索引优化。

本次未测 release、真实用户分布、长期墙钟、峰值内存、自然尾延迟或完整写盘与停止的并发时序；不能宣称停止响应已变快。来源复验仍需完整扫描与 JSON 解析，prepare 仍生成 dataset，既有全候选资源上限必须保留。若继续优化，优先细分 query/prepare 的扫描、解析与分类成本，以及 people 对证据的重复扫描；先测具体路径，再决定是否减少分配或共享单次请求内的读取结果，不直接引入持久缓存或改变授权锁序。

## 最终回归与交付

两项优化保留，新增 7 项普通语义保护和 1 项默认 ignored 的图规模诊断。只有 `soul-graph/src/build.rs`、`soul-store/src/research_retention.rs`、`soul-store/src/research_retention/query.rs` 三个生产文件属于本批改动；未改变冻结算法、来源/授权/事务语义，未更换依赖或锁文件。新测试、方案、评审和 STATUS 同步记录；保留开工时其他 dirty 工作，未提交。

主会话在稳定生产候选上串行执行以下命令，每条最终均 exit 0。日志名加 `.log` 后位于上述原始目录，完整起止 UTC 与退出码见 `execution.jsonl`。该文件有 19 条阶段回执：18 条成功，1 条参数转发失败；失败及重试均保留，不把所有尝试写成全绿。

| 日志名 | 实际命令 | 结果 |
| --- | --- | --- |
| after-root-tests | `cargo test --workspace --all-targets --locked --offline -- --skip windows::runs_on_windows` | 181 个结果块，1497 passed / 0 failed / 13 ignored / 3 filtered |
| after-root-docs | `cargo test --workspace --doc --locked --offline` | 19 个结果块，1 passed |
| after-desktop-tests | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again --skip windows::runs_on_windows` | 独立 workspace 7 个结果块，130 passed / 0 failed / 2 filtered |
| after-root-clippy | `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | 通过，不降低 lint |
| after-desktop-clippy | `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | 通过 |
| after-core-check | `cargo check -p soulcore --lib --locked --offline` | 普通非测试 core 库通过 |
| after-root-format-final | `cargo fmt --all -- --check` | 规范命令通过 |
| after-desktop-format | `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check` | 通过 |

普通回归、doc 与独立桌面共 1628 项通过，另显式执行 graph/growth 诊断 2 项，共 1630 个已通过的不同测试入口。改前与定向复测不重复累计，27 个增长检查点和 18 次 ZIP 不是另加的测试数量。根默认 ignored 的 13 项包含这两个被单独执行的诊断；真实采集过滤总计 5 项，不把这些过滤项记为通过。

代码及测量独立核验见[评审回执](../reviews/2026-10-21-algorithm-architecture-optimization.md)。评审发现增长段多写审计断言，已修正并保留记录；没有因该措辞问题扩充生产范围。`git diff --check` exit 0，既有 PRODUCT_LOCK 的 CRLF 提示未据此重写文件。前端源码未变，本批不重复前端/打包；前轮通过不转写为本轮执行。

本批完成已选优化、当前合成运行、结果分析和独立评审；不标 GATED/INTEGRATED，不签发完整 G-W/G-L/G-M、安装、真人效果或 Goal 关闭。下一有意义的验证点是对仍占时的 query/prepare 与 people 分段测量；本批到此收口，不自动展开缓存、锁协议或无关重构。

## 第二批：查询、准备与人物展示

用户在首批收口后明确授权继续，重点是查询/准备中的全候选扫描与解析，以及人物展示重复证据读取。开工再次实读 Git/STATUS/PLAN_INDEX：仍为 `6a56d852d23a051c7c4e9845642791295c541105 + dirty`，首批实现保留。本批主会话统一执行，调查、实施和独立评审继续使用 `gpt-6.1-sol/high`。

**近期成果及边界：** 选择能够在单次请求内减少重复处理、且输出/拒绝/副作用保持的改动；先在当前生产实现取得三轮改前测量，新增必要等价保护后再实施。保留全候选资源限制、快照规则、冻结算法、现有授权/事务和时间语义，不引入持久缓存、SQL 范围下推、schema 迁移或新 gate。只用合成库，不运行真实采集和完整平台门禁。

初步只读发现纠正了两项假设：当前 `scan_sources` 每候选在 Rust 侧只做一次 JSON 文本解析，范围外分类已被跳过；剩余成本包含 retained Value 深拷贝、typed 反序列化和丢弃详细错误的 schema 检查。SQL 中既有 `json_valid/json_extract` 不属于这一次 Rust 解析，本批未改变或单独计时其内部处理。`people()` 展示持久图，不是按联系人反复解析全库 interaction；实际重复点是 contact/edge 匹配和按边逐项 get_evidence，不能擅自加入其原本没有的全局坏时间拒绝。

本批日志与前后快照目录：`C:/Users/86080/AppData/Local/Temp/soul-query-optimization-45840f357cb543a8969a20cd59c12538/`。沿用首批环境与根 `opt-level=1` 测试配置；所有新数据单独记录，不直接拿首批 after 当本批 before。相同 `research_growth_probe` 的三轮改前执行已 exit 0：27 个成功检查点、18 次保存/独立读取和 1 次拒绝。万事件 query 中位稀疏 593.30 ms、密集 566.93 ms；prepare 分别 568.50/572.95 ms。随后旧生产上的留存单元 13、集成 7、图 incidence 4、people reading 4 项及 1 项/12 记录人物诊断均通过；主会话逐文本核对三个生产文件仍与改前快照相同后，授权了生产改动。

### 候选与验证分工

| 负责人 | 允许生产写路径 | 选定候选与必须保持的行为 |
| --- | --- | --- |
| query_prepare_optimization | `soul-store/src/research_retention/query.rs` | 由借用 Value 反序列化 typed event，省去先深拷贝整个 Value；在原位置使用已有 schema 布尔检查，省去最终丢弃的错误详情。保留 typed/schema 两层检查、分支顺序、snapshot/SQL/限额与发布协议 |
| people_read_optimization | `soul-graph/src/view.rs`、`soulcore/src/commands/graph.rs` | 所有旧边成功读取后建 contact→incident edges 局部索引，保持原聚合及字典序时间语义；单次 people_view 按原顺序懒缓存成功 EvidenceRowView，重复引用仍原样输出，首个缺失/坏 doc 仍在原位置拒绝，不预读未引用 evidence |

两名实现者先只写本路径测试，主会话运行旧代码保护与新诊断后才授权生产替换；双方不改共享接口、Session 热点、schema、依赖、锁文件或文档。主会话保存三生产文件改前快照和原 10 项留存单元测试。新增测试路径为 graph `view_incidence.rs`、core `people_view_reading.rs` 与 `people_view_scale_probe.rs`（共享合法合成 fixture 放 `people_view_support/mod.rs`），留存新增必要差集放原 tests.rs。

人物展示的 evidence 读取是带主键索引的点查，不是每次全表扫描。设 K 为全部边的引用槽位、U 为唯一 evidence ID，旧实现读取 K 次，局部元数据复用候选读取 U 次；结构计数不冒充 SQL trace。映射额外保存 O(U) 项元数据，K=U 没有数据库读取节省，需要正面报告开销。诊断使用 100/1000 peers × unique/shared × 三次独立临时 SQLCipher 库，C/E/K 对齐、U 按形状不同；计时为真实 people_view，不包含 Session 锁，另测 graph load，构造、参考读取、完整模型断言与输出在计时外。现有 growth 的 Session::people 则保留为另一条真实应用层参照。

独立设计核对确认产品 Session 的 people/correct/release 展示读取处在同一 store mutex guard 内；请求局部映射只保存 ID/kind/method/strength 四项元数据，不带正文、来源指针或授权状态。此方案仅支持既有单 Session 互斥路径的一次展示内复用，**不承诺任意外部 writer 下的逐点读取等价**：受信代码仍能绕开 Session 直接开 SQLCipherStore，或以原始 SQL 在两次同 ID 读取间改写/删除；旧逻辑可能第二次读新值或报错，局部映射会复用首次元数据。现有 people_view 本就没有数据库快照或原子活性契约，OS lease 也不能阻止原始 SQL writer；不据此新增事务或宣称更强一致性，跨请求变更可见性仍必须测试。

留存保护重点是借用/拥有 Value 反序列化的合法与坏类型等价、typed 可读但 schema 必须拒绝的多错误文档，以及 policy/unavailable/time 排除顺序；既有范围外、全局上限和来源变化拒绝继续复用。图保护覆盖 self-loop 仅计一次、反向边、孤立/缺端点/forgotten 联系人、原顺序与未关联坏边仍拒绝；展示保护覆盖重复槽位、缺失/坏 doc 顺序、未引用坏 doc 不读取和跨请求来源变化可见。新用例先在旧代码通过，属于等价保护，不伪称 RED。

### 第二批执行记录

- 人物实现 Agent 曾遇到一次模型容量中断，使用同一 `gpt-6.1-sol/high` 恢复既有工作，未更换模型或扩大写路径。
- 两实现者完成三处生产文件的限定改动；独立评审已核对新增 11 项普通测试与 1 项 ignored 诊断的实际代码，未发现具体测试缺陷。测试包含请求 UUID 与返回 doc ID 错绑反例，防止缓存键错误。
- 首次 `after-root-format.log` exit 1，仅为新增留存测试两处 rustfmt 换行差异；作者用 apply_patch 修正，局部格式检查通过，生产与测试逻辑不变。失败日志保留，主会话改用 `after-root-format-retry.log` 重跑并继续原序列；不以静态格式通过替代后续执行。

### 首候选 A 的测量与调整依据

这里的 A 指借用 Value/is_valid、incident 索引及“所有成功 ID 都保存元数据”的首版请求局部复用。改后定向 13+7+4+4 项保护与两个诊断均 exit 0；随后根 1508 项、doc 1 项、独立桌面 130 项通过，双 workspace 严格 Clippy/格式及普通 core 编译通过。根 14 ignored / 3 filtered、桌面 2 filtered。A 串行流程在 14:37:18 UTC 完整退出后才授权 B 改动，不将 A 结果转签 B。`after-people-scale.log` 和 `after-growth.log` 是 A 的测量，不覆盖为下一候选的数据。全部原始记录已解析为 `all-measurement-samples.json`，统计为 `measurement-statistics.json`；另保存 `initial-a-samples.json` / `initial-a-statistics.json`，独立分析者从原始日志重算。

| A 的关键对照（中位 ms） | 改前 | A | 变化 |
| --- | ---: | ---: | ---: |
| 10000 稀疏事件 query / prepare | 593.30 / 568.50 | 391.83 / 390.04 | −33.96% / −31.39% |
| 10000 密集事件 query / prepare | 566.93 / 572.95 | 368.39 / 373.41 | −35.02% / −34.83% |
| 100 shared peers 的 people_view | 7.1041 | 2.4606 | −65.36% |
| 1000 shared peers 的 people_view | 82.1786 | 15.5957 | −81.02% |
| 100 unique peers 的 people_view | 5.9801 | 7.7266 | **+29.21%，+1.7465 ms** |
| 1000 unique peers 的 people_view | 93.8160 | 93.3278 | −0.52%，基本持平 |
| 固定 10 peers、100 独有事件的 Session::people | 1.8449 | 3.5332 | **+91.51%，+1.6883 ms** |
| 固定 10 peers、1000 独有事件的 Session::people | 21.5760 | 24.3976 | **+13.08%，+2.8216 ms** |
| 固定 10 peers、10000 独有事件的 Session::people | 853.0680 | 709.8268 | −16.79% |

人物 before/A 各 12 条记录，growth 各 28 条；输出/计数/ZIP/事务/拒绝结构一致，所有慢样本保留。A 的 unique/100 graph_load 自身也由 0.9936→1.5314 ms，该函数不做 evidence cache，故不能把所有增耗归因缓存。另一方面，不能用“可能是环境波动”抹去独有路径的负例。A 中密集 100 保存阶段之和中位 +2.95%，一次 begin 为 12.6418 ms；12000 行单次 prepare 拒绝为 423.6930→490.7383 ms（+15.82%），仅前后各一次，均不得隐藏或当作三次分布。execute 本批实现未改，测量下降不是纯 I/O 优化证据。

主会话因此决定收窄为候选 B：**只缓存重复引用 ID 的成功元数据**，唯一引用直接读取、转换并放入输出，避免 A 在 K=U 路径额外保存/复制四个字符串。这是根据实际负例减少可确认的多余工作，不宣称已经定位了全部耗时根因。请求 ID 的预扫描可排序其独立副本，只用于识别重复项；实际数据读取仍严格按原 edge/slot 顺序。无新阈值、持久缓存、公开接口或事务。A 执行期间作者只准备仓库外草稿，等待当前执行通道退出后再改生产；B 必须重新跑相同诊断及相关最终回归，不能继承 A 的通过。

### 候选 B 的落实与成本

B 仅在 `soulcore/src/commands/graph.rs` 上收窄 A：完整图读取成功后，复制 K 个请求 UUID 并排序；相邻相同且不同于已收集末项时才保存重复 ID，避免先收集 K 项再 dedup 的容量保留。设 D 为引用至少两次的不同 ID 数，实际读证据仍按原槽位顺序：唯一项直接转换并移动输出，重复项懒缓存成功元数据。读取次数仍由 K 降为 U，缓存元数据由 A 的 O(U) 缩为 O(D)；预处理增加 O(K log K) 排序、O(K) UUID 临时空间和每槽 O(log D) 判断。这是减少无收益的元数据分配/复制，不代表消除了所有 K=U 开销，峰值内存尚未测量。

独立评审先看 A 快照到 B 的实际差分，核对空/单槽、同边及跨边重复、混合 unique、三次以上重复、请求 UUID/doc ID 错绑与首错顺序，没有具体发现。重复集合保持严格升序，UUID 预扫描不读取或转换证据；旧完整 load 和跨请求生命周期不变。作者局部 rustfmt 与 diff 检查通过后停写；主会话重新启动 `selective-*` 串行回归及相同诊断，最终证据另列，不以静态评审代替运行。

全候选 SQL、10 万候选与 16 MiB 元数据上限继续保留：直接按索引时间下推 WHERE 会跳过 doc/index 不一致或坏 JSON 的候选，也可能使范围外来源不再触发原有全局限额与批准失效规则。该语义不能仅凭性能要求改写。本次降低每候选分类成本，**没有把扫描复杂度从 O(N) 降为范围内行数**；单次 Rust JSON 文本解析仍存在，借用 Value 只省深拷贝，typed event 本身仍会分配拥有的字段。`is_valid` 复用既有编译 schema，不改变规则；查询与 prepare 同时受益，不能从这组联合候选隔离两处改动各自的耗时贡献。

### 第二批最终人物测量

以下为独立分析者从 `before-people-scale.log` 与 `selective-people-scale.log` 重算的完整八组结果，单位 ms，每格 min / median / max，每档三次。A 原始日志与统计保留。三个候选均采用相同的独立临时库构造和预热参考读取，不随机交错；不能据 n=3 推断 p95 或完全隔离环境与实现的因果贡献。

| 形状 | peers | 测量面 | 改前 min / median / max | B min / median / max | 中位变化 |
| --- | ---: | --- | ---: | ---: | ---: |
| unique | 100 | graph load | 0.9641 / 0.9936 / 1.1045 | 0.9968 / 1.4976 / 1.9290 | **+50.72%** |
| unique | 100 | people_view | 5.7111 / 5.9801 / 8.3946 | 7.6707 / 8.4622 / 10.8891 | **+41.51%** |
| unique | 1000 | graph load | 28.0036 / 29.5221 / 34.1380 | 20.4060 / 23.5739 / 25.4616 | −20.15% |
| unique | 1000 | people_view | 86.3093 / 93.8160 / 107.4606 | 78.5250 / 82.9069 / 98.7898 | −11.63% |
| shared | 100 | graph load | 0.9714 / 1.0870 / 1.2265 | 1.1191 / 1.4491 / 1.7828 | **+33.31%** |
| shared | 100 | people_view | 5.2175 / 7.1041 / 7.2366 | 1.3533 / 1.3727 / 1.8712 | −80.68% |
| shared | 1000 | graph load | 19.3291 / 21.6787 / 23.6099 | 11.5254 / 14.9204 / 16.1399 | −31.17% |
| shared | 1000 | people_view | 80.2408 / 82.1786 / 82.3310 | 15.0009 / 16.9381 / 19.8594 | −79.39% |

人物 before/A/B 各 12 条记录，三日志 9–20 行为样本、23 行测试通过、25 行 EXIT=0。每档 C=P+1、E=P、K=4P、stored evidence=4P；unique 的 U=4P，shared 的 U=3，后者为控制重复程度的合成形状，不能当作真实分布。完整 graph/view 与独立手写 expected 的断言在打印前成功；重复槽、顺序、用户/机器值分离与完整四字段元数据一致。graph load 与包含 load 的 people_view 分别计时，不相加或相减解释为纯证据读取成本。

| people_view 中位 ms | 改前 | A | B | B 相对 A |
| --- | ---: | ---: | ---: | ---: |
| unique / 100 | 5.9801 | 7.7266 | 8.4622 | **+9.52%** |
| unique / 1000 | 93.8160 | 93.3278 | 82.9069 | −11.17% |
| shared / 100 | 7.1041 | 2.4606 | 1.3727 | −44.21% |
| shared / 1000 | 82.1786 | 15.5957 | 16.9381 | **+8.61%** |

**B 没有实测修复 unique/100 的退化，也没有全面胜过 A。** 该档相对改前中位 +2.4821 ms，B 的 10.8891 ms 慢样本保留；unique/1000 的 98.7898 ms 与 shared/1000 的 19.8594 ms 也不剔除。B 的 shared 两档相对改前中位分别约 5.18× / 4.85×，前后区间不重叠，支持重复证据场景的显著收益；unique/1000 区间仍重叠，只记本组三次观察改善。小规模 load 自身也变慢，而它不调用证据缓存，故不能将全部增耗归因缓存；同样不能借此抹去退化。

选择 B 的依据是共享形状仍有明确收益，并消除了 unique 路径可确认的额外元数据缓存/复制；这个选择接受当前小规模观察代价，**不声称所有工作负载无回归**。本批停止继续试探阈值或反复重跑挑选更快样本。若后续优先解决该差集，应独立拆分 UUID 识别、graph load 与点查/转换成本，并采用更有控制的交错测量；当前未完成这项归因。

### 第二批最终增长测量

最终候选 B 的 `selective-growth.log` exit 0。下表单位 ms、每格 min / median / max、每档三次，与本批 before 对比；原始 80 条 before/B 记录和 76 项统计保存在 `selective-all-samples.json` / `selective-statistics.json`，A 的文件不覆盖。固定 10 peers 的同库导入与稀疏/密集采集形状、100→1000→10000 累积次序和独立 Python ZIP 读取沿用原诊断。prepare 包含查询/编码/预检，execute 包含编码/来源复验/写盘/同步/发布；save sum 是每轮四阶段相加后再取统计，不是阶段中位数之和。

| 形状 | 累计事件 | 阶段 | 改前 min / median / max | B min / median / max | 中位变化 |
| --- | ---: | --- | ---: | ---: | ---: |
| dense_minute | 100 | query | 4.02 / 4.49 / 9.23 | 3.08 / 3.29 / 3.69 | −26.75% |
| dense_minute | 100 | prepare | 4.77 / 5.51 / 7.34 | 3.63 / 5.01 / 5.32 | −9.10% |
| dense_minute | 100 | execute | 5.41 / 6.16 / 6.92 | 5.37 / 7.30 / 8.06 | +18.58% |
| dense_minute | 100 | save sum | 12.92 / 15.85 / 15.88 | 12.61 / 13.49 / 13.94 | −14.89% |
| dense_minute | 1000 | query | 31.96 / 32.05 / 49.16 | 26.04 / 45.68 / 48.26 | +42.52% |
| dense_minute | 1000 | prepare | 33.88 / 36.88 / 44.61 | 33.80 / 36.36 / 37.16 | −1.41% |
| dense_minute | 1000 | execute | 28.51 / 43.09 / 51.14 | 21.91 / 27.85 / 30.46 | −35.37% |
| dense_minute | 1000 | save sum | 64.60 / 90.40 / 93.82 | 60.01 / 66.22 / 66.96 | −26.75% |
| dense_minute | 10000 | query | 560.97 / 566.93 / 568.70 | 356.95 / 381.43 / 386.64 | −32.72% |
| dense_minute | 10000 | prepare | 519.13 / 572.95 / 586.10 | 378.06 / 384.35 / 401.83 | −32.92% |
| dense_minute | 10000 | execute | 329.13 / 386.16 / 398.18 | 308.56 / 331.66 / 339.70 | −14.11% |
| dense_minute | 10000 | save sum | 850.79 / 961.84 / 989.65 | 688.67 / 734.24 / 735.47 | −23.66% |
| import | 100 | commit | 38.46 / 46.70 / 64.10 | 32.84 / 35.54 / 47.99 | −23.90% |
| import | 100 | Session::people | 1.81 / 1.84 / 2.07 | 1.77 / 1.86 / 2.12 | +0.72% |
| import | 1000 | commit | 297.32 / 314.17 / 343.35 | 270.29 / 303.61 / 311.93 | −3.36% |
| import | 1000 | Session::people | 20.90 / 21.58 / 32.49 | 20.00 / 25.47 / 28.48 | +18.06% |
| import | 10000 | commit | 3975.63 / 4041.79 / 4546.31 | 3405.69 / 3439.51 / 3668.99 | −14.90% |
| import | 10000 | Session::people | 799.12 / 853.07 / 1203.00 | 691.16 / 711.48 / 718.44 | −16.60% |
| sparse_hour | 100 | query | 3.20 / 5.48 / 6.77 | 3.56 / 3.58 / 3.92 | −34.56% |
| sparse_hour | 100 | prepare | 4.24 / 6.66 / 8.74 | 4.36 / 4.89 / 5.05 | −26.57% |
| sparse_hour | 100 | execute | 5.26 / 6.62 / 8.41 | 5.42 / 6.56 / 6.80 | −0.99% |
| sparse_hour | 100 | save sum | 11.23 / 16.84 / 16.94 | 11.05 / 13.32 / 13.77 | −20.92% |
| sparse_hour | 1000 | query | 35.12 / 43.82 / 44.51 | 29.74 / 31.97 / 46.30 | −27.05% |
| sparse_hour | 1000 | prepare | 39.31 / 44.43 / 49.67 | 30.53 / 36.95 / 42.12 | −16.85% |
| sparse_hour | 1000 | execute | 30.70 / 33.23 / 38.63 | 23.45 / 25.71 / 26.43 | −22.63% |
| sparse_hour | 1000 | save sum | 76.77 / 79.78 / 85.40 | 58.01 / 61.74 / 70.60 | −22.62% |
| sparse_hour | 10000 | query | 499.43 / 593.30 / 620.27 | 352.74 / 397.13 / 470.22 | −33.06% |
| sparse_hour | 10000 | prepare | 483.10 / 568.50 / 614.67 | 412.46 / 459.48 / 471.16 | −19.18% |
| sparse_hour | 10000 | execute | 369.49 / 411.91 / 437.32 | 334.14 / 377.61 / 379.33 | −8.33% |
| sparse_hour | 10000 | save sum | 854.58 / 1007.91 / 1028.44 | 748.70 / 839.03 / 852.27 | −16.76% |

before/A/B 各有 27 个成功检查点及 1 个拒绝点，结构字段逐项一致；各 18 次保存均经独立 Python 读取后打印，四阶段和及采集写入 delta 核对一致。稀疏 100/1000/10000 行对应 ZIP 10731/94434/931437 字节；密集 2/17/167 行为 1617/3030/17133 字节。测试确认事务关闭、计数、行数和重开 dataset 一致，不增加其没有的审计链断言，也不把随机 ID/密钥或 ZIP 字节数一致描述为文件逐字相同。

万事件 query 中位稀疏/密集分别下降 33.06% / 32.72%，prepare 下降 19.18% / 32.92%；这四项三样本的前后范围不重叠，结合等价保护支持保留分类处理优化。保存阶段之和下降 16.76% / 23.66%。但本批没有修改 execute 或 import commit 实现，不能把其观测下降归因于新增 I/O 或导入优化；A 到 B 的 query 实现也相同，耗时变化只代表不同次运行观察，不能宣称由证据缓存间接加速。

最终增长负例同样保留：密集 1000 的 query 中位 32.05→45.68 ms（+42.52%），密集 100 的 execute +18.58%；固定 10 peers 的 Session::people 在 100/1000 事件分别 +0.0133 ms（+0.72%）和 +3.8974 ms（+18.06%）。1000 事件展示并未因 B 收窄缓存而恢复改前水平；100 事件接近改前也不能代替独立人物诊断 unique/100 的退化。三样本范围多有重叠，未控制 CPU/后台负载/缓存，因此不据这些正负观察断言具体系统根因。

| 12000 稀疏行拒绝，单次 ms | 改前 | A | B |
| --- | ---: | ---: | ---: |
| query | 432.9922 | 429.0959 | 452.6011 |
| prepare 拒绝 | 423.6930 | 490.7383 | 475.3817 |

三个版本均在 prepare 拒绝，没有目标文件；拒绝后关闭并重开库，仍有 12000 个事件。该拒绝点没有事务关闭的单独断言，不能移用成功保存的 `transaction_closed` 记录。B 的拒绝耗时没有改善，不并入成功三次分布。不能从这一个超限点反推最小拒绝行数。所有失败/慢样本保留，未运行真实导出、真实采集、release、峰值内存、自然尾延迟或完整 ZIP 与停止并发；局部收益不续签正式平台或真人效用结论。

### 第二批最终回归与交付

最终保留 B，即 query 的借用 Value/布尔 schema 检查、graph load 的 incident 索引及 people_view 的重复 ID 元数据复用。本批改动的生产文件仍为 `crates/soul-store/src/research_retention/query.rs`、`crates/soul-graph/src/view.rs`、`crates/soulcore/src/commands/graph.rs`，新增 11 项普通语义保护和 1 项默认 ignored 的人物诊断；旧 growth 诊断复用。没有更换依赖/锁文件、授权/事务协议或冻结算法，已有其他 dirty 工作保留，未提交。

B 生产候选在下列完整串行执行期间没有再次修改。每条命令均 exit 0；日志名加 `.log` 后位于本批原始目录。阶段在 2026-10-21 14:38:40–14:48:31 UTC 执行，其中计时样本排除编译与测试 fixture 构造。`execution.jsonl` 共 32 条回执：31 成功、1 次 A 格式失败，所有日志末尾退出码一致、阶段 UTC 无重叠。B 的 11 阶段全部成功，未以 A 通过替代。

| B 日志名 | 实际命令 | 结果 |
| --- | --- | --- |
| selective-root-format | `cargo fmt --all -- --check` | 通过 |
| selective-people-reading | `cargo test -p soulcore --test people_view_reading --locked --offline` | 4 passed，包含于下方根回归，不重复累计 |
| selective-people-scale | `cargo test -p soulcore --test people_view_scale_probe --locked --offline -- --ignored --nocapture` | 1 passed，12 条完整断言后记录 |
| selective-growth | `cargo test -p soulcore --test research_growth_probe --locked --offline -- --ignored --exact cumulative_growth_and_zip_costs --nocapture` | 1 passed / 1 filtered；27 成功点、18 次 ZIP/独立读取、1 次拒绝 |
| selective-root-tests | `cargo test --workspace --all-targets --locked --offline -- --skip windows::runs_on_windows` | 184 个结果块，1508 passed / 0 failed / 14 ignored / 3 filtered |
| selective-root-docs | `cargo test --workspace --doc --locked --offline` | 19 个结果块，1 passed |
| selective-desktop-tests | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again --skip windows::runs_on_windows` | 独立 workspace 7 个结果块，130 passed / 0 failed / 2 filtered |
| selective-root-clippy | `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | 严格检查通过，未降低 lint |
| selective-desktop-clippy | `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked --offline -- -D warnings` | 严格检查通过 |
| selective-core-check | `cargo check -p soulcore --lib --locked --offline` | 普通非测试库通过 |
| selective-desktop-format | `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml --all -- --check` | 通过 |

根、doc、独立桌面合计 `1508 + 1 + 130 = 1639` 项，再加本次显式人物/growth 两项诊断，**共 1641 个通过的不同测试入口**。根 14 默认 ignored 中仅这两项本轮另行运行；5 个真实采集 filtered 不算通过。before、A、定向复测、27 检查点和 18 次 ZIP 不重复计入测试总数。首批 graph 诊断没有在 B 上再次显式运行，不借用首批 PASS 扩大当前分母。

实现差分、三个版本的测量和最终证据见[第二批独立评审](../reviews/2026-10-21-algorithm-architecture-optimization.md#第二批查询准备与人物展示)。前端源码没有改，本批未重跑前端/生产包；完整 G-W/G-L/G-M、安装、真实用户效用与 release 均不在此次执行范围。本批完成选定实现、合成执行、正负结果分析和独立核验，不标正式 GATED/INTEGRATED 或 Goal 关闭。
