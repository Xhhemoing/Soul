# 第三批算法与架构实验独立评审

候选：`codex/soul-rebuild-20261019 @ 6a56d852d23a051c7c4e9845642791295c541105 + dirty`。本记录由协调者按独立 `soul_reviewer` 的实际反馈整理；实现者不自签评审，未提交、合并或签发平台门禁。实现与运行回执见[实验计划](../design/2026-10-20-research-experiment.md#第三批代码与运行回执2026-10-21)。

评审者先读取任务、新测试与真实 diff，再核对执行说明和原始日志；只读，不运行构建或接触真实数据。范围为 `soul-graph/src/build.rs` 展示排序、`research_time_boundaries.rs`、`research_selection_boundaries.rs`、`research_path_probe.rs`，不包含先前 N2/VAL/留存的全部 dirty 改动。

测试设计审查未发现具体缺陷。重点确认：实际 SQLCipher/rebuild/证据链；source_refs 本身也反转，避免置换空转；遗忘前重新生成有效上下文，排除重准备导致的混淆；检查真实 CK、正文与引用；测量排除初始化/fixture/断言/清理，空发布闭包未冒称 ZIP 保存。主线程集成时进一步将第二 peer 锚点调到 12:15，使新组合测试能区分全图与逐 peer 的 `as_of`，定向运行已通过。

生产修复审查未发现具体缺陷：顺序键为 adapter 验证所得 UTC 整秒、去尾零的小数字串和原始文本，支持跨 offset 与不同小数长度，并确定性保留原文；只影响 first/last/last_direct 展示。评分输入、统一 `as_of`、用户锁定档位、machine band、算法 crate 与权威文档未变。负 Unix 秒及超长小数仅作静态推断，不伪称已运行专门用例。

最终证据收尾已完成。评审者实际读取初始 offset RED、扩展 3 failed / 3 passed 的 RED、正式导入与 selection 对照，以及最终 `graph-final.log`（195 普通测试 + 1 doc-test）、`time-final.log`（加强锚点后 6 项）、`core-regression-final.log`（26 项）、`path-probe-final.log`（1 项、6 样本）、`clippy-final.log`（严格命令及 EXIT=0）。去重为 222 个测试与 1 个 doc-test，没有累计重复运行。

评审者直接解析最终六条原始 JSON，未使用初测或现成 summary 代算，独立重算 10 项指标的两个规模 min/median/max，20 个表格单元全部一致。另核对撤权六次为 11.7299、6.8118、3.8056、3.4912、4.2552、3.7846 ms；全部 1 次采样/0 个已完成事件，摘要 3.49–11.73 ms 正确，文档没有将另开的空载 runner 误称规模负载下的停止测量。

当前候选没有剩余具体实现、文档或数值问题。没有新增安全流程、hash、门槛或冻结规范；未验证项仍包括真实采集、ZIP 保存耗时、冷缓存、长期增长、10k/100k 容量、p95、UI 延迟、完整根 workspace、desktop/React 重跑及 G-L/G-W/G-M。结论仅支持本批实现和合成运行范围，不是合并批准或正式门禁签发。
