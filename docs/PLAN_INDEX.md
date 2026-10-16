# 计划索引（30 秒定位权威文件）

核于 2026-10-16。本页只导航到当前仓库实际存在的权威文件；历史分支说明不作为当前进度。AI 开发先读项目级 [AGENTS.md](../AGENTS.md)，其职责是协作和执行导航，不是第二份产品规范。

## 一、先读四份

| 文件 | 唯一权威范围 |
|---|---|
| [PRODUCT_LOCK.md](PRODUCT_LOCK.md) | 产品边界、v0.1 十三条垂直切片、出网分级、不可协商约束 |
| [DECISIONS.md](DECISIONS.md) | 已拍板的选择；本地门禁为 D61–D63，文档清理为 D64，平台定位为 D65，长期目标与并行规划为 D66–D67，预备支线为 D68，近期框架与记忆/持久导入/自然键/遗忘语义见 D69 起的逐项决定 |
| [algorithms/DECISION.md](algorithms/DECISION.md) | T4D、A0/A1/A2/A3、常量、as_of 纪律与回退链 |
| [ACCEPTANCE.md](ACCEPTANCE.md) | Goal 1 验收矩阵、工作包、红线与关闭条件 |

## 二、按问题查

| 问题 | 去哪 |
|---|---|
| 现在到哪一步、还有哪些缺口 | [STATUS.md](STATUS.md) 顶部当前状态；历史段按当时日期阅读 |
| 当前分层、操作流程与后续执行架构图 | [ARCHITECTURE.md](ARCHITECTURE.md)；明确区分已实现与设计中，图源在 [architecture/](architecture/) |
| 最新框架 W1–W5 的已接线能力与边界 | [ARCHITECTURE 当前表](ARCHITECTURE.md) / [HTML 顶部当前框架](framework-design.html) / [剩余模块计划](design/2026-10-16-remaining-implementation.md)：owned 扫描、本次候选记忆、持久导入/恢复、封存自然键去重/迁移、本地 A2 反馈；实际验证看 STATUS |
| 本次记忆用途、过滤/澄清与实际请求预算 | [剩余模块计划的 W2 节](design/2026-10-16-remaining-implementation.md)；明确候选内的关键词/类型选择，缺事实时间先 Clarify，默认 local，真实 JSON serializer 64 KiB 与整条裁剪/引用复验 |
| 持久回执、不可变预览确认、OS lease 与去重迁移 | [W3/W4 计划](design/2026-10-16-remaining-implementation.md)；确认只带 IDs，schema 3，同事务回执/原审计；forward-only 不代表全历史去重，遗忘撤销 namespace |
| 洞察定位、实际 Session 组合链与现代导入审计 | [工作流连续性计划与结果](design/2026-10-16-workflow-continuity.md)；精确关系及指定证据元数据、地址/来源/挂载归属；原生 IPC 测试源码与实际可执行检查分开记录 |
| 可信导入统计如何参与本次起草 | [洞察起草材料契约](design/2026-10-16-insight-draft-context.md)，D76；已独立复核方案，当前实现状态及实际结果看 STATUS 与设计结果节，不把契约存在当成代码完成 |
| 从实际使用出发的候选算法与反例实验 | [10 月 15 日使用框架研究](design/2026-10-15-usage-framework.md) / [交互 HTML 的历史研究区](framework-design.html)；参考算法、SVG 与实验保持固定历史范围，不覆盖顶部生产状态 |
| 导入交付与人物摘要的此前生产切片 | [逐模块计划与验证](design/2026-10-16-import-summary-continuation.md)；D70，先规划复核再编码；后续持久幂等/恢复以 W3/W4 为准，平台验证单独记录 |
| v0.1 是否做某个功能 | [PRODUCT_LOCK.md](PRODUCT_LOCK.md) 的最小垂直切片与砍/留 |
| 一件事怎样算完成 | [ACCEPTANCE.md](ACCEPTANCE.md)；矩阵与产品十三条切片同时满足 |
| Goal 1 的实现拆分与依赖 | [GOAL1_PLAN.md](GOAL1_PLAN.md) |
| 门禁怎么跑、哪个版本有证据 | [gates/README.md](gates/README.md) 与对应 SHA 的记录 |
| Windows 真机要看到什么 | [作者手动清单](../scripts/author-manual-checklist.md) |
| 数据契约 | [schemas/](schemas/) 正文与 schemas.lock.json |
| 加密、密钥、遗忘、审计 | [SECURITY.md](SECURITY.md) |
| 算法判档与中文话术 | [algorithms/DECISION.md](algorithms/DECISION.md)、[COPY_ZH.md](algorithms/COPY_ZH.md)、[REJECTED.md](algorithms/REJECTED.md) |
| 长期目标、版本路线、个人/商业/研究如何并进 | [ROADMAP.md](ROADMAP.md)；含能力工作包、验证方法及多 agent 资源分配 |
| Goal 2 具体派工、文件归属、依赖、测试与交付回执 | [GOAL2_PLAN.md](GOAL2_PLAN.md)；正式启动须满足前置条件，D68 独立质量预备支线不等于正式启动 |
| 项目专用实现与只读评审角色、单写者热点、重构建调度 | [AGENTS.md](../AGENTS.md) 与 [.codex/](../.codex/)；任务卡和短交接留在会话或仓库外，不新增流程系统 |
| 审查发现和修复证据 | [reviews/](reviews/)；局部修复回执不等于完整门禁通过 |

## 三、仓库与验收状态

- 当前续写基于 `b66f093a1407887eb873fe33defe984906252d29`，W1–W5 已接入并逐模块独立源码复审；源码完成不等于已合入 main。最终测试数量由 [STATUS](STATUS.md) 记录。
- W5 依赖并非“只有计数”：还保存 UUID、固定状态与既有 conversation_ref 的规范字节编码，完整来源复验失败后展示仅保留历史操作计数。忽略不训练，重算创建同操作的新 bundle。
- 本地主干记录 `8aae8f3` 已合入 M0-2 收敛，仓库包含核心与桌面实现；不再是只有计划和算法 crate。
- M0-3 安装修复基线为 `95ff7d6`：继承 `1eb471a` / `e2fdf16`，新增注册路径解码、临时副本卸载及完整目录消失判定；隔离 G-W 与受控回归通过。实际安装首次失败、部分恢复仍有残留，见 [安装修复记录](gates/20260925-95ff7d6-win.md)。
- D68 独立集成源码 `ebff0c9` 的完整 G-W 与补充检查已通过，见 [四包集成门禁记录](gates/20260925-ebff0c9-win.md)。此前记录的同源码 NSIS 外部证据目录当前缺失，不能代勾 G-M 0；真实安装/卸载按用户要求 `NOT RUN`，G-M / G-L 未齐，Goal 1、Q2-00、正式 Q2-05 与 Goal 2 均未关闭。
- 根 Cargo workspace 与 `apps/desktop/src-tauri` 是两个独立 workspace。根目录测试不覆盖桌面壳。
- 计划冻结、代码实现、平台门禁通过、Goal 1 关闭分别记录。旧版本的绿不能替代当前版本的证据。
- 本轮按用户指示暂时跳过 Linux 门禁；这不是豁免 Goal 1 的 G-L 关闭条件。
- 当前环境缺原生 GTK/WebKit 开发依赖，静态 Tauri 命令断言不等于原生构建/IPC。完整 G-L 继续暂缓，本轮 G-W/G-M 未运行，Goal 1、正式 Goal 2 与 Wave 1 均未关闭。

## 四、历史材料的边界

D64 已移除 FORMAL_WORK_PROMPT、GOAL2_POLISH_PROMPT、PLAN_VERIFY_PROMPT 与扫描流程材料。当前验收和规划使用本页所列文件，不恢复这些旧入口。
历史分支、临时探针、过程草稿可以解释由来，不能替代当前源码与门禁结果。

## 五、改动规则

| 改动 | 前置 |
|---|---|
| 产品方向 | 先改 PRODUCT_LOCK，在 DECISIONS 追记 |
| 算法判档 | 按 algorithms/DECISION 的回退链，DECISIONS 留痕；不另设阈值 |
| 验收矩阵 | 不删已通过项；新增项写清 Given / When / Then 与执行者 |
| 工作包 | Goal 1 冻结包只减不增，WP12 保持删除；后续 Q2/能力包独立编号，不追加到 Goal 1 验收 |
| schemas | 同批更新 schemas.lock.json，并说明对已落库数据的影响 |
| 合并 | 遵守 D63；本次跳过某平台不等于该平台通过 |
