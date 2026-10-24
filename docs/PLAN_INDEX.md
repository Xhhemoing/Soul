# 计划索引（按任务读取）

本页只负责导航；当前进度只在 [STATUS](STATUS.md) 维护。AI 开发入口为 [AGENTS](../AGENTS.md)，实际版本以本次 Git 检查为准。

## 开始一项任务

1. 读取 Git 状态、分支与 HEAD，保留已有修改。
2. 读 [STATUS](STATUS.md) 的当前摘要，确认已实现、仅设计、暂缓与未验证的边界。
3. 按下表读取任务相关规范和设计；历史回执按需进入，不默认通读全部文档。

## 规范权威

| 要确认什么 | 唯一权威 |
| --- | --- |
| 产品方向、v0.1 十三条切片与功能边界 | [PRODUCT_LOCK](PRODUCT_LOCK.md) |
| 已拍板的选择与语义变更 | [DECISIONS](DECISIONS.md) 为决定索引；当前治理见 D110 / [ADR-001](decisions/ADR-001-repository-governance.md)，API 见 D111 / [ADR-002](decisions/ADR-002-local-api-runtime.md)。旧 D61–D64 的 CI/合并部分按 D110 更新，其余产品、算法、授权与平台决定保留 |
| T4D / A0 / A1 / A2 / A3、常量、as_of 与回退链 | [算法决定](algorithms/DECISION.md)；中文话术见 [COPY_ZH](algorithms/COPY_ZH.md)，落选方案见 [REJECTED](algorithms/REJECTED.md) |
| Goal 1 验收矩阵、红线与关闭条件 | [ACCEPTANCE](ACCEPTANCE.md)；矩阵与产品切片同时满足 |
| 加密、密钥、遗忘、审计与安全边界 | [SECURITY](SECURITY.md) |
| 数据契约与既有 lock | [schemas/](schemas/) |

## 按开发问题定位

| 问题 | 具体入口 |
| --- | --- |
| 2026-10-24 临时分支包含哪些近期开发、已知问题及待作者分析的事项 | [近期开发交接](plans/2026-10-24-development-handoff.md)；随源码提交的日期快照，区分已有实现、回执范围、原生/效用缺口与待讨论问题，不替代 STATUS 或既有产品决定 |
| 长期目标、近期 S1 验证、下一切片与投入收敛 | [长期战略与近期验证方案](plans/2026-10-23-long-term-strategy.md)、[D112 / ADR-003](decisions/ADR-003-long-term-strategy.md)；承接作者两份 v1.2 文件及减少冒烟/哈希/单点空转的要求。S1 合成准备见 §13，反馈修正见 §13.1，HTTP/前端主路径与导航修复见 §13.2；起草焦点与复制状态修复见 §13.6.3–13.7，[复制独立评审](reviews/2026-10-24-s1-draft-copy.md)与[统计定向复审](reviews/2026-10-23-s1-feedback-statistics.md)分别记录。普通验证优先 HTTP，真实使用与作者验收仍按授权 |
| 规范化按什么顺序落实、哪些实际运行过 | [规范化计划与回执](plans/2026-10-23-repository-normalization.md)；分别记录 N1 规则/文档/CI、N2 runtime/API、N3 写入/事件及同步回执，远端结果不借用本地检查 |
| 日常协作、失败处理与 CI 如何执行 | [协作与记录](operations/repository-workflow.md)、[CI 覆盖与远端证据](operations/ci.md)、[ADR-001](decisions/ADR-001-repository-governance.md) |
| 本地 HTTP API 如何与 Tauri 共用操作及事件 | [本地 API 架构与合成启动](architecture/local-api.md)、[ADR-002](decisions/ADR-002-local-api-runtime.md)、[N2 独立复审](reviews/2026-10-23-local-api-phase1.md)、[N3 独立复审](reviews/2026-10-23-local-api-phase2.md)；共享读写 runtime、主体归属与事件/UI 刷新已实现，默认关闭且仅合成验证 |
| 核心框架与当前实现是否一致、下一闭环如何执行 | [v1.3 修订与执行分析及 D109 实施回执](design/2026-10-22-core-framework-v1.3-execution.md)、[产品切片独立评审](reviews/2026-10-22-scoped-register-implementation.md)；精确人物／场合本地语域偏好已接存储／遗忘、Session/IPC/UI，范围与实际验证分别记录。原[文档核验](reviews/2026-10-22-core-framework-v1.3-revision.md)、[v1.2 增量审核](reviews/2026-10-22-core-framework-v1.2-review.md)、[v1.1 五层审核](reviews/2026-10-22-core-framework-v1.1-review.md)保留各自历史范围；全套 Claim/自动召回未实施 |
| 调研、测试与实验结果如何保存、何时整理 | [AGENTS 记录要求](../AGENTS.md#调研测试与实验记录)；每次结果落盘，任务收尾/交接整理，活跃开发每 7 天检查；最近周整理范围及剩余项见 [STATUS](STATUS.md#记录维护) |
| 不预设信息选项，从合成原文提取需求/来源并判断缺口与选材 | [开放任务完整流程](design/2026-10-22-open-task-flow.md)及[独立评审](reviews/2026-10-22-open-task-flow.md)；Sol 24/Luna 12 原文主批完成，预整理因 HTTP502/未知结算停止；系列实验已按作者要求暂停，见[检查点](design/2026-10-22-open-task-flow.md#暂停检查点2026-10-22) |
| 来源元数据、缺口/冲突边界与全新任务三模型复测 | [元数据与新任务复测](design/2026-10-22-task-metadata-retest.md)及[独立评审](reviews/2026-10-22-task-metadata-retest.md)；12 新场景/72 case、相关与无关不可读、明确更正、完整替代证据、累计费用 |
| 新合成任务的必要性、缺口、支持组选材与真实模型比较 | [任务与模型合成比较](design/2026-10-22-task-model-comparison.md)与[独立评审](reviews/2026-10-22-task-model-comparison.md)；24 family/96 case、Jev 留存及 GPT-6 Luna/Sol、6.1 Sol 配对实测、本地算法阶段计时、场景/费用/标注边界 |
| 数据处理流程、具体选材算法与 Jev/本地模型怎样取舍 | [数据处理与选材方案及合成重放](design/2026-10-22-data-selection-pipeline.md)；分类型处理、CK 派生边界、支持组选择、BGE/Qwen/Jev 候选及完整分母；[独立评审](reviews/2026-10-22-data-selection-pipeline.md)；不代表模型或生产默认已接入 |
| 任务信息核对、缺口与一次补证，以及后续适用条件方案 | [任务上下文方案与实施回执](design/2026-10-22-task-context-plan.md)；第 13、14 节为显式字段、手选候选和必要澄清的轻量实施；前文保留报告修订及后续语义/持久化候选，不能全部视为已实现 |
| 应取多少材料、何时可以停止 | [固定数量与任务需求停止对照及结果](design/2026-10-21-evidence-stopping.md)与[独立评审](reviews/2026-10-21-evidence-stopping.md)；同候选双排序、机器需求／充分支持组、真实请求字节、过细需求与不可用结果 |
| 更强词面、重复整理与语义在新任务中怎样比较 | [六任务 × 三候选条件对照及结果](design/2026-10-21-retrieval-controls.md)与[独立评审](reviews/2026-10-21-retrieval-controls.md)；事实替代来源、同条件预算、完整来源映射与五种固定方法 |
| 本次目标如何与统计证据共同准备 | [沟通意图与证据共同准备](design/2026-10-21-communication-joint-preparation.md)；已保存目标的独立选择、实际预览、脱敏及来源复验 |
| 放宽关键词会否用噪声挤掉必要材料 | [有界沟通检索对照](design/2026-10-21-bounded-communication-retrieval.md)；同一候选池内比较软排序和交错补选，区别材料覆盖、预算和未测语义效果 |
| 明确任务范围与意图后，词面和语义排序如何比较 | [任务感知检索实验与结果](design/2026-10-21-task-aware-retrieval.md)及[独立评审](reviews/2026-10-21-task-aware-retrieval.md)；同候选预算、十二份受限语义预测、真实 Session 重放及失败保留 |
| 人际沟通任务的材料是否完整、瓶颈在哪里 | [2026-10-21 第一批任务与材料诊断](design/2026-10-21-communication-material-diagnosis.md)；协调、婉拒、关系跟进的合成选择与实际请求观察，区分材料传递、模板边界和未测语义效果 |
| 算法与架构怎样按测量优化 | [2026-10-21 优化方案与回执](design/2026-10-21-algorithm-architecture-optimization.md)及[独立核验](reviews/2026-10-21-algorithm-architecture-optimization.md)；首批图重建/发布复验，[第二批查询、准备与人物展示](design/2026-10-21-algorithm-architecture-optimization.md#第二批查询准备与人物展示)，各自前后数据和回归 |
| 本轮全面测试如何分工、运行与分析 | [2026-10-21 综合测试](design/2026-10-21-comprehensive-validation.md)及[独立核验](reviews/2026-10-21-comprehensive-validation.md)；程序/算法/架构三线与实际日志、失败、结论限制；当前状态只看 STATUS |
| 实验版先做什么、怎样保留研究数据 | [2026-10-20 实验版计划](design/2026-10-20-research-experiment.md)；D107 留存见第二批、算法/规模见第三批、D108 缺失/非法时间规则见第四批、增长/ZIP/停止压力及[保存取消补充](design/2026-10-20-research-experiment.md#研究保存取消补充实测2026-10-21)见第五批；[独立读取器](../scripts/read-research-retention.py)、[留存评审](reviews/2026-10-20-research-retention.md)；不要求整个应用恢复，正式状态只看 STATUS |
| 此前 N1/N2、VAL 与 MODEL 预备怎样完成 | [2026-10-20 前阶段计划与回执](design/2026-10-20-next-stage-plan.md)；D102–D106 的已执行范围保留，不由旧下一动作自动展开生产恢复 |
| 当前实现分层、操作流程与未来架构 | [ARCHITECTURE](ARCHITECTURE.md)；图源和 SVG 在 [architecture/](architecture/)，已实现与设计中分别标记 |
| W1–W5：owned 扫描、候选记忆、持久导入/恢复、自然键与本地反馈 | [剩余模块计划及验证](design/2026-10-16-remaining-implementation.md)；W2 为明确候选内选择，W3/W4 含不可变预览、OS lease、迁移和遗忘重导边界 |
| U1–U7：事项、目标、待办、提醒通知与本地准备 | [原计划](design/2026-10-16-unified-assistance.md)第 10–11 节 / [重建实施验证](design/2026-10-19-unified-assistance-implementation.md)；当前状态看 STATUS，旧丢失工作区验证不移用 |
| 导入交付与人物摘要的执行框架 | [逐模块计划与回执](design/2026-10-16-import-summary-continuation.md)；后续持久恢复与幂等以 W3/W4 为准 |
| 导入成功后的来源列表刷新 | [来源刷新修复](design/2026-10-16-import-source-refresh.md)；不自动选来源或重放导入 |
| 洞察定位、Session 组合链与现代导入审计 | [工作流连续性](design/2026-10-16-workflow-continuity.md) |
| 可信导入统计参与本次起草 | [洞察起草材料](design/2026-10-16-insight-draft-context.md)；D76，默认 local，另选派生计数才进入最终 E1 |
| 明确查看一条导入消息来源 | [本地来源核对](design/2026-10-16-local-evidence-view.md)；D77，两步精确锚读取，不授予原文出网或起草用途 |
| Goal 1 的工作包和实现依赖 | [GOAL1_PLAN](GOAL1_PLAN.md) |
| 长期数字分身、商业与研究三条路线 | [ROADMAP](ROADMAP.md)保留长期依赖与正式阶段；近期按[战略指引](plans/2026-10-23-long-term-strategy.md)，不扩大 v0.1 产品或出网边界 |
| Goal 2 任务、文件归属与前置 | [GOAL2_PLAN](GOAL2_PLAN.md)；D68 独立质量预备不代表正式启动 |
| VAL-01 的失败、缺测、指标分母与操作入口 | [记录适配与离线重算](design/2026-10-19-val-01-evaluation-design.md#计时缺测与人工元数据入口) / [合成受控生成与盲呈现协议](design/2026-10-19-val-01-evaluation-design.md#d104-合成受控生成与盲呈现入口) / [本机合成操作工具](design/2026-10-19-val-01-evaluation-design.md#d105-本机合成操作入口)；D101–D105，执行范围和命令见原设计，不代表真人效用或 Goal 关闭 |
| DATA-01 的局部恢复、旧备份与遗忘 | [恢复边界设计](design/2026-10-19-data-01-recovery-boundaries.md)；N1 存储见第 10 节，D106 测试专用 N2 Session 见第 11 节；区分有限实验、设计选择和实际验证，不是已生效生产契约 |
| MODEL-01 实现前的现有能力与有限实验 | [差集与合成实验](design/2026-10-20-model-01-boundary-probe.md)；关键词反证选择、用户语气与记忆材料的实际请求观察，只输出元数据，不启动生产模型或真实效用试验 |
| 哪个源码版本实际运行过门禁 | [gates/README](gates/README.md) 与对应提交的记录；根与独立 desktop 是两个 Cargo workspace |
| Windows 真机必须观察什么 | [作者手动清单](../scripts/author-manual-checklist.md)；不以源码或 mock 代填 |
| 实现/只读评审角色、单写者热点和构建调度 | [AGENTS](../AGENTS.md) / [.codex/](../.codex/)；任务卡与短交接留在会话或仓库外 |
| 审查发现与局部修复证据 | [reviews/](reviews/)；局部修复不等于完整门禁 |
| 使用闭环与候选算法研究 | [2026-10-15 研究](design/2026-10-15-usage-framework.md) / [参考模型验证](design/2026-10-15-framework-validation.md)；不扩大生产算法或验收范围 |

## 目录与材料归属

| 路径 | 放什么 |
| --- | --- |
| 根目录 | AGENTS、README、workspace / 工具链 / 锁文件；不放运行日志和临时任务卡 |
| `crates/` | 根 Rust workspace；模块边界见 ARCHITECTURE |
| `apps/desktop/` | React UI 与独立的 `src-tauri` Cargo workspace |
| `fixtures/`、`scripts/` | 合成测试材料、既有执行与人工验收入口 |
| `docs/` 顶层 | 产品、决定、当前状态、验收、路线和架构入口 |
| `docs/product/` | 产品规范导航；既有 PRODUCT_LOCK、ACCEPTANCE、SECURITY 保持原路径和权威 |
| `docs/decisions/` | 新 ADR 的决定、理由与边界；DECISIONS 保留编号索引和旧决定 |
| `docs/operations/` | 日常命令、协作、失败处理、记录和 CI 运行方式 |
| `docs/plans/` | 新跨模块实施顺序、阶段验收与相应回执；当前状态仍只放 STATUS |
| `docs/design/` | 既有具体方案、调研与各自实验回执，优先续写所属主题；参考实现留在 `design/reference/`，不为改目录而重复拆散活跃记录 |
| `docs/algorithms/`、`docs/schemas/`、`docs/architecture/` | 算法权威、数据契约、架构设计及图源与导出图 |
| `docs/gates/`、`docs/reviews/` | 对应版本的平台证据、审查与修复记录 |
| `docs/archive/`、`docs/history/` | 本次起归档与既有历史记录；保留原日期、失败和未完成项，不覆盖当前 STATUS |
| `target/`、`node_modules/`、`.pnpm-store/` | 本地构建或依赖缓存；保持既有路径，不纳入提交 |
| `.learnings/` | 本地工具的草稿记录；稳定结论整理进 AGENTS 或既有设计，不作为另一份项目记忆权威 |

静态 [架构 HTML](architecture.html)、[框架 HTML](framework-design.html) 和 [历史总览](project-overview.html) 保留各自标注的版本范围；当前状态从 STATUS 查询。历史阶段的 [Wave 1 计划](SOUL_WAVE1_IMPLEMENTATION_PLAN_2026-09-27.md)继续保留为原范围依据；旧状态见 [archive](archive/STATUS_20261023.md)及 [history/](history/)。D64 已删除的工作提示词和扫描流程不恢复；CI 配置恢复按 D110，远端启用与实际运行分别记录。

## 变更时维护哪一处

| 改动 | 应维护的文件与边界 |
| --- | --- |
| 产品方向 | PRODUCT_LOCK，DECISIONS 追加决定 |
| 算法判档 | 按 algorithms/DECISION 回退链，DECISIONS 留痕；不另设阈值 |
| 验收矩阵 | 不删已通过项；新增项写清 Given / When / Then 与执行者 |
| 工作包 | Goal 1 冻结包只减不增，WP12 保持删除；后续工作独立编号 |
| schemas | 同批维护既有 schemas.lock.json，说明已落库数据影响 |
| 当前进展 | 更新 STATUS 摘要；命令、失败与详细回执留在相关设计或门禁，旧状态归档时修复相对链接 |
| 调研、测试、实验与周期整理 | 按 [AGENTS 记录要求](../AGENTS.md#调研测试与实验记录)保存逐次结果，详细证据归所属文档；本页只维护导航，STATUS 只保留当前结论与最近周整理信息 |
| 合并 | 遵守 D110 / ADR-001 的目标 SHA 远端证据要求；当前暂缓或后置某平台不等于该平台通过，正式验收仍按 gates/README |
