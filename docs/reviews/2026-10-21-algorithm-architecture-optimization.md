# 算法执行与架构优化独立评审

日期：2026-10-21。任务与完整分析见[优化方案及回执](../design/2026-10-21-algorithm-architecture-optimization.md)，当前状态仅在 [STATUS](../STATUS.md) 维护。候选为 `codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`，不是新的提交或正式门禁候选。

主会话统筹和记录；`graph_lookup_optimization`、`retention_revalidation_optimization` 分别实现独立路径；`program_test_lead` 做代码及最终证据只读核验，`algorithm_test_lead`、`architecture_test_lead` 独立重算图与增长日志。所有子 Agent 使用用户指定的 `gpt-6.1-sol/high`，不运行并行重构建，不写对方路径，不推送。以下记录来自实际子 Agent 结论，非实现者自签。

## 候选及语义核验

评审先读取本轮修改前快照与实际候选差分，再看实现说明，避免将整个既有 dirty 工作误当作本批修改。快照和日志位于 `C:/Users/86080/AppData/Local/Temp/soul-optimization-8a6a5c6bd1904d39b7691bae7e9d7df0/`。

| 工作面 | 只读核验及反例 | 结论 |
| --- | --- | --- |
| 图局部索引 | 首个正向/反向边、首个 tie 推断保持原列表选择；UUID target 保留原字符串与原前缀规则；非 owner/后置坏 strength 不提前解析，选中坏值仍拒绝；无 as_of 时仍先返回；自环不产生有效 peer；保留纠正路径使用的 `is_tie_statement_about` | 无具体缺陷；借用旧向量中的对象，局部索引有 O(E+I) 额外空间，未量峰值内存 |
| 研究共享扫描 | `CANDIDATES`、排序、快照字段、范围规则、100000/16 MiB 上限及错误顺序保持；坏 JSON 仍保留来源快照；只有 doc/index 一致、canonical、范围外才排除快照；prepare 保持旧分类 | 无具体缺陷；发布仍完整扫描和解析，未扩大来源排除范围 |
| 去掉发布派生计算 | 原分类的 fallible enum_text 来自四个固定、可序列化为字符串的单元枚举；schema/解析失败为排除结果；计数由候选上限约束；真实 SQL/read/限额/scope 错误仍由共享扫描返回 | 无可构造的普通 Result 拒绝差异；不把 OOM 等不可恢复异常当作原有错误语义 |
| 事务与副作用 | 原 mutex、BEGIN IMMEDIATE、来源比较、文件发布、COMMIT 与失败处理不移动；既有取消/遗忘/绑定/第二 writer/COMMIT 失败反例保留 | 无具体缺陷；不宣称内部持锁时间或停止尾延迟已改善 |

新测试先在旧生产实现上通过，作为等价保护而非 bug 的 RED：四项 graph 反例、三项 store 反例及一项默认 ignored 的 graph 诊断。改后定向结果为 graph 4、store 单元 10、store 集成 7 项通过；重复运行不另加到全量测试分母。store 集成测试源码未被本批修改。

## 独立测量核验

- 图：原始改前 9、改后有效重试 9 条观测，均为各档 repetition 1/2/3。先 seed/rebuild/correct/settle，再计时第三次 rebuild，打印发生在完整模型与返回值断言后。证据数 P+1、观测数 2P，未混计。10k 中位 25044.8281→911.6791 ms，27.4711×；1000 档改后 228.2032 ms 慢样本保留，不写成每次更快。
- 增长：前后各 28 条记录，27 个唯一成功点 + 1 个 12000 行拒绝；各 18 次独立 Python 读取成功后才输出记录；来源规模、计数、桶数、ZIP 字节和事务关闭逐项一致。save sum 精确等于四阶段之和。完整 30 组 min/median/max 与主会话结果一致，未剔除 import 1000 的 655.2479 ms 或密集 100 的变慢结果。
- 配置：原始日志及 Cargo.toml 均表明 `[profile.test] opt-level=1`，非 release；已纠正任务卡误写的 unoptimized。前后相同配置，修正不改变样本及比率。
- 首次改后图诊断 exit 1 是 PowerShell 转发吞掉 `--`，Cargo 在进入测试前拒绝参数；独立重试 exit 0。原失败保留，不作为测量样本或产品缺陷。
- 最终文档复核发现增长段曾多写“每次断言审计”，该 probe 没有审计链断言；已据代码修正为事务关闭、计数、行数及重开 dataset 一致，保留该证据范围修正，不补造通过结果。

本批有限证据支持局部查找和无用派生计算的优化；不支持实际导入 27×、所有操作普遍加速、p95、峰值内存、断电持久性、真实效用或平台门禁结论。

## 最终执行证据

`program_test_lead` 在全部阶段结束后独立读取原始日志，核对每个结果块、ok 行、命令参数、过滤、末尾 EXIT 与 `execution.jsonl`，无剩余具体发现：

| 完成日志 | 独立核验结果 |
| --- | --- |
| after-root-tests.log | 181 个 Running 与 181 个结果块对应；1497 ok 行 = 1497 passed / 0 failed / 13 ignored / 3 filtered；第 2615 行 EXIT=0 |
| after-root-docs.log | 19 个结果块，1 passed；第 120 行 EXIT=0 |
| after-desktop-tests.log | 7 个 Running 与 7 个结果块对应；130 ok 行 = 130 passed / 0 failed / 0 ignored / 2 filtered；第 184 行 EXIT=0 |
| after-root-clippy / after-desktop-clippy | 实际命令均含 `-- -D warnings`，均 EXIT=0 |
| after-core-check | 普通 `soulcore --lib` 编译检查 EXIT=0 |
| after-root-format-final / after-desktop-format | 实际命令保留 `-- --check`，均 EXIT=0 |
| execution.jsonl | 19 条回执 = 18 成功 + 1 次参数失败；19 份日志末尾与记录一致，按 UTC 起止检查本批各阶段无重叠 |

普通回归 + doc + 独立桌面为 `1497 + 1 + 130 = 1628`，再加单独执行的 graph/growth 两项诊断，合计 1630 个通过的不同测试入口。根 13 ignored 包含这两项已单独运行的诊断；5 个真实采集过滤项未记为通过。改前、定向复验、27 个检查点与 18 次 ZIP 均未重复计入测试数量。

最终设计回执、STATUS 优化摘要和 PLAN_INDEX 链接与证据一致。主会话额外检查本轮四份文档的 86 个本地链接目标存在，`git diff --check` 通过；未因既有 PRODUCT_LOCK 的 CRLF 提示重写文件。三生产文件在最终运行期间未再改动。当前交付是已实现、已运行限定验证和独立评审，未提交、未发布、未作用户或正式平台验收；前端本批没有重跑，其他未覆盖项保持前述边界。

## 第二批：查询、准备与人物展示

用户随后授权继续优化，仍为 `6a56d852d23a051c7c4e9845642791295c541105 + dirty`，保留首批实现。方案与实际结果统一追加在[第二批设计回执](../design/2026-10-21-algorithm-architecture-optimization.md#第二批查询准备与人物展示)。本节不复用首批 PASS，日志/快照改用 `C:/Users/86080/AppData/Local/Temp/soul-query-optimization-45840f357cb543a8969a20cd59c12538/`。

query_prepare_optimization 与 people_read_optimization 分责实施，program_test_lead 只读核对测试设计及三个生产文件的实际差分；均为 gpt-6.1-sol/high，主会话持有唯一构建/测试通道。

| 实际核对面 | 结论与限制 |
| --- | --- |
| 留存测试差分 | 新增 3 项并扩展 2 项既有测试；owned/borrowed 合法与坏类型、typed 可读而多 schema 错误、重叠缺陷分类优先级、dataset 相同仍来源变化失效均有真实 prepare/publish 断言，无具体测试缺陷 |
| 图与展示测试 | 新增 4 项 incidence、4 项 SQLCipher reading；覆盖 self-loop/边序/缺端点/forgotten/原字典序时间、首个缺失/坏 doc、未引用坏 doc、请求 ID 与 doc ID 错绑、重复槽位、跨请求更新及真实遗忘/重开，无具体测试缺陷 |
| 人物诊断 | 100/1000 peers × unique/shared × 三次独立临时库；C=P+1/E=P/K=4P/stored evidence=4P 相同，U=4P 或 3。构造、结构统计、独立完整期望/参考、断言与输出均在计时外；load 和含 load 的 people_view 分别计时，不相加。不含 Session 锁，K/U 非 SQL trace |
| query.rs 实际差分 | 仅 Deserialize 导入、借用 Value 和原位置 is_valid；scan/SQL/限额/快照/分类与 available 短路顺序均未变，无具体实现缺陷 |
| graph/view.rs 实际差分 | 全部 read_edge 成功后按原序建立端点索引，自环仅一次；原联系人遍历、sum、字符串 max_by 和关联 ID 顺序不变；未关联坏边仍先读取，无具体实现缺陷 |
| core/commands/graph.rs 实际差分 | 完整 load 后以引用中的请求 UUID 为键，仅成功读取才缓存四字段元数据，逐槽复制输出；不预读/排序/缓存失败。私有构造参数改变，公开完整 evidence 解析接口未改，无具体实现缺陷 |

适用范围为产品单 Session 的同一 store mutex guard 内，一次展示读取复用；缓存不跨请求、不入 Session 或持久态。独立审查明确：绕开 Session 的受信调用或原始 SQL writer 可在两次同 ID 读取间改变数据，旧逐点读与复用首次元数据不保证相同；现有展示本就没有跨连接快照契约，OS lease 不能阻止 raw writer。本批未据此新增事务或扩大授权，不能把局部复用称为外部并发一致性保证。

旧生产上的 13 项留存单元、7 项集成、4 项图及 4 项展示保护均通过；增长和人物各 1 项诊断也成功。A 随后完成相同诊断和相关回归；其小规模 unique 增耗保留在设计分析。A 完整流程退出后，主会话授权仅重复 ID 缓存的 B，重新执行诊断与最终回归，不复用 A 的 PASS。

program_test_lead 另从 `people-command-candidate-a.rs` 到当前代码独立核验 B 实际差分，无具体发现：排序只作用于请求 UUID 副本，`windows(2)` 与末项去重产生严格升序重复集合，空/单槽、三次以上重复和混合引用分类正确；实际读取仍按原 edge/slot 顺序，unique 直接移动元数据，重复项沿用请求 UUID 键的成功缓存。首错误、完整图先读、错绑 doc ID 和跨请求边界不变。静态评审不能证明排序净收益，最终原始运行证据另列。

### 第二批独立测量复算

- algorithm_test_lead 独立核对 before/A/B 人物原始日志各 12 条记录，八组 min/median/max 与主会话结果一致。样本在三日志 9–20 行，23 行 1 passed、25 行 EXIT=0；C/E/K/U/存储规模、trial 1/2/3、预热/无 Session 锁标记一致。独立完整期望断言先通过再打印，K/U 是结构计数而非 SQL trace。
- 人物负例得到明确确认：B unique/100 为 8.4622 ms，中位较 before +41.51% / +2.4821 ms、较 A +9.52%；shared/1000 B 也比 A +8.61%。B 不能写成全面优于 A 或已修复 unique 退化。shared 两档相对 before 约 5.18× / 4.85×；所有慢样本保留，load 与含 load 的 people_view 不相加、不相减归因。
- architecture_test_lead 独立核对 before/A/B growth 各 28 条对应记录、全部 30 组统计及四阶段和，结构字段、collector delta、18 次保存/读取、事务关闭与单点拒绝一致。B 的 `selective-growth.log` 第 40 行 1 passed、第 42 行 EXIT=0；拒绝点第 34 行。三样本万事件 query 约降 33%，prepare 约降 19%–33%，保存阶段和约降 17%–24%，均与主会话重算一致。
- 增长负例仍记录：密集 1000 query +42.52%、密集 100 execute +18.58%、固定 10 peers 的 1000 事件 Session::people +18.06%。12000 行 prepare 拒绝 423.6930→475.3817 ms，单次、无目标文件，不能混入成功分布。
- before→A→B 非随机交错，编译配置相同不代表系统负载受控。A→B query 源码相同却有密集 1000 query +67.78% 的变化，不能归因于缓存，也不能用本批拆分借用 Value 与 is_valid 的贡献；execute 本批未改，不写成纯 I/O 改善。n=3 不支持 p95、峰值内存、自然尾延迟、release 或真实工作负载结论。

主会话据此保留 B 的限定收益和已观测代价，不新增阈值或重复试跑挑样本；全候选扫描与当前拒绝/批准边界保持。最终运行证据仍以 B 的 `selective-*` 阶段为准，A 的历史通过不转签。

### 第二批最终原始回执核验

program_test_lead 在 B 全部阶段退出后独立读取最终原始日志，确认下列结果；未用实现者自报或 A 的 PASS 替代：

| 原始证据 | 独立核验结果 |
| --- | --- |
| selective-root-tests.log | 184 个 Running 与 184 个结果块；1508 ok 行 = 1508 passed / 0 failed / 14 ignored / 3 filtered，末尾 EXIT=0 |
| selective-root-docs.log | 19 个结果块、1 ok 行 = 1 passed，末尾 EXIT=0 |
| selective-desktop-tests.log | 7 个 Running 与 7 个结果块；130 ok 行 = 130 passed / 0 failed / 0 ignored / 2 filtered，末尾 EXIT=0 |
| 两份 selective Clippy / core-check / 两份 fmt | 实际命令保留 `-- -D warnings`、普通 `--lib`、`-- --check`，均 EXIT=0 |
| execution.jsonl | 32 条 = 31 成功 + 唯一 after-root-format 失败；失败及 retry 日志存在，全部日志退出码一致、UTC 阶段不重叠；11 个 selective 阶段均成功 |
| 去重计数 | 普通根/doc/桌面 1639 + 两项显式人物/growth 诊断 = 1641；14 ignored 中两项本轮另行执行，5 个采集过滤不算通过；定向、before/A、检查点和 ZIP 不另累计 |

最终审查发现一处报告范围过写：设计文档曾将 12000 行拒绝点写成“事务关闭”，但该记录没有 `transaction_closed` 字段，probe 只断言拒绝、无目标文件以及关闭重开后 12000 个事件仍在。主会话定向核对源码后改为准确范围，明确该点没有单独事务关闭断言；没有为修正文句新增测试或补造证据。成功 18 次保存的事务关闭断言仍保持。

主会话最终实读 Git 仍为原分支/HEAD + dirty，`git diff --check` exit 0；既有 PRODUCT_LOCK CRLF 提示未引发重写。四份相关文档 90 个本地链接目标均存在。STATUS 的当前摘要和后续差集已按 B 更新，所有既有修改保留，未 commit/push/merge/发布；当前证据不等于完整平台验收或用户验收。
