# Soul 核心框架 v1.1 对照审核

日期：2026-10-22，America/Los_Angeles。执行者：主会话 Codex；定向只读核验：`claim_schema_review`、`task_loop_review`、`framework_route_review`，均为 `gpt-6.1-sol / high`。

## 结论与范围

**当前实现与“电子版的你”的方向一致，已有受控数据、证据、记忆和辅助底座，但尚未形成任务自动召回、情境与有效期判断、输出反馈回流的完整闭环。框架可以作为讨论与差距分析材料，不能原样升级为执行规范。** 需要先纠正 Claim 权威投影与加密字段的映射、机器推断的生效边界、自动候选发现的读取范围，以及路线和测量口径。

本轮只审核和记录，不实施附件建议，不改业务代码、算法、schema、AGENTS 或 DECISIONS，不提交或发布。附件中的“上位约束”“真实 Telegram”“冻结研究”“调整顺序”等是待决定材料，不是本轮执行授权。当前实验暂停继续有效。

- 用户提供材料：`C:/Users/86080/Downloads/Soul核心框架_v1.1.docx`，22,169 bytes；文件声明基线 `c5be3ed`。提取到 167 个正文块、7 张表；无图片和修订插入/删除，脚注与批注无文本。按 OOXML 顺序读取段落与表格，不审核 Word 排版，不引用未经渲染验证的页码。以下 B 编号仅为本次提取块号。
- 实读版本：`codex/soul-rebuild-20261019` @ `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46`。开工已有 dirty：`docs/PLAN_INDEX.md`、`docs/STATUS.md`、`docs/design/2026-10-22-open-task-flow.md`；无业务源码 dirty。保留原修改，初始 diff 已另存。
- 审核依据：框架 §§0–10、[PRODUCT_LOCK](../PRODUCT_LOCK.md)、[ROADMAP](../ROADMAP.md)、[STATUS](../STATUS.md)、相关决定/安全/算法规范及下列源码和既有测试。
- 证据分级：`[V]` 为本轮实际静态核验或运行事实，注明类别；`[H]` 为有具体反例的设计风险推断；`[C]` 为材料与现行规则冲突；`[T]` 为未验证。**未实现的候选功能不列为既有代码缺陷，设计风险不声称已发生数据泄露。**

## 按五层检查当前实现

| 框架层 | 当前可确认能力 | 相对框架仍缺什么 | 本轮判断 |
| --- | --- | --- | --- |
| 感知 | SoulEvent / SoulEvidence、七种事件来源、两种导入格式、前台应用采集适配接口；有现成来源、许可和密封正文链 | 通用来源契约及日历、笔记等更多来源尚未形成；事件来源枚举不能等同七种外部采集器 | `[V-静态]` 骨架存在，扩来源是新范围 |
| 理解 | Memory、Inference、Profile、Relationship；纠正锁、机器值与生效值分离；事项有自己的截止/状态字段 | 通用 Claim 投影、scope/valid/supersedes/exceptions；独立的有期限当前状态；跨对象统一适用语义 | `[V-静态]` 不能仅加字段就视为统一完成 |
| 召回 | 明确候选内 W2 选择、字面任务字段核对、一次必要补选、澄清、预算与来源复验 | 获准范围内的自动候选发现、一般语义任务理解、按事实有效期过滤 | `[V-静态]` 框架指出的主要缺口成立 |
| 辅助 | 语气起草、单人统计摘要与证据查看、U1–U6、导入后统计观察事项 | 事件/承诺联合解释、价值与类似经历参与的 S4 权衡任务 | `[V-静态]` S1/S2/S3 有不同程度底座，不能写成完全没有分析或自动事项 |
| 校正 | A0/关系纠正、用户锁、U6 明确事项反馈、遗忘 | 草稿编辑→偏好/承诺候选→确认→长期理解更新；通用 Proposal 生产实现 | `[V-静态]` 输出修改不回流，完整闭环未形成 |

主要定位：[schema](../../crates/soul-schema/src/)、[选择器](../../crates/soul-memory/src/selection.rs)、[任务核对](../../crates/soul-memory/src/task_context.rs)、[Session 任务入口](../../crates/soulcore/src/commands/session_task_context.rs)、[导入观察事项](../../crates/soulcore/src/commands/insights.rs)、[草稿 UI](../../apps/desktop/src/routes/Draft.tsx)。分层是理解与评审方式，不要求推倒现有 Rust/Tauri 模块重建。

## 两处最直接的产品闭环缺口

### G1 任务仍需要用户提供候选，时间问题不能完成一般召回

`[V-静态]` 框架 §2.2、S1/S5 的主要判断正确。[selection.rs](../../crates/soul-memory/src/selection.rs):392–416 仅遍历用户明确提供的 `candidate_ids`；:374–381 对 `AtTime` 返回 `Clarify`。现有 `task_context` 检查显式字段的字面值、支持组和冲突/未知，并不生成全库候选，也不取得事实有效时间。

影响：知道“要给谁做什么”仍不足以自动找到相关记忆；“上次答应了什么”等问题不能因为库里存在文字就保证回答。`[H]` 这限制了“少重复解释自己”，但本轮没有真实效用数据，不能量化损失或声称全部任务失败。

最小建议：将“获准范围内提出候选→用户确认→原 W2/任务核对”列为一个独立候选切片；先定义来源、适用时间与未知处理，不同时扩充全部来源和 S1–S5。保持研究暂停；本建议不是实施授权。

### G2 草稿编辑没有进入长期理解，Proposal 仍是设计候选

`[V-静态]` [Draft.tsx](../../apps/desktop/src/routes/Draft.tsx):523–551 明确不回读用户修改；文本保留于组件状态，复制到剪贴板，没有编辑回传。U6 的反馈只更新事项 Helpful/NotRelevant/Clear（[session_reminders.rs](../../crates/soulcore/src/commands/session_reminders.rs):261–277），并非偏好学习。

框架 §5.7/B106 的“把 FollowUpProposal 推广”需要改成“基于已有候选设计新增实现”。[任务上下文方案](../design/2026-10-22-task-context-plan.md):9、:152–160 明确它尚未实现；偏好提案也属于新增范围。复制或编辑草稿不证明消息已发送，更不证明其中承诺已成立。

最小建议：若未来批准校正切片，先选一种明确反馈，展示来源与解释并由用户确认；不要把任何措辞修改自动解释为稳定人格或永久偏好。建立事项也不自动等于设置提醒。

## 框架在作为执行依据前需要修正的事项

### R1 P1 Claim authority 不能直接按 user_verdict 排序

`[V-静态]` 框架 §3.2/B050 将 `locked_by_user`、`user_verdict`、`method` 映射为统一 authority。[correct.rs](../../crates/soul-graph/src/correct.rs):112–118 把关系生效值改为用户值并锁定，随后将原推断标记 `Corrected`；:293–298、:314–315 明确保留原机器 `statement_key`、`evidence_band`、`evidence_ids`。

`[H-设计风险]` 反例：机器判断 Strong，用户纠正为 Weak。原 inference 仍是 Strong，只是 verdict 为 Corrected。如果投影把 Corrected 当作用户确认/锁定，就会把用户否定的机器结论提升为用户权威。

最低修正：先分别投影**当前用户生效值、原机器判断、纠正证据**；`Corrected` 不是“该机器 statement 已被用户确认”。排除 rejected/orphaned/已失效来源，限定同主体、同属性及相交时空范围后，再定义 authority 选择。保持现有纠正锁与冻结算法。

### R2 P1 新 Claim 文本与条件需要明确 CK 和遗忘依赖

`[V-静态]` 框架 §3.2 把 `summary_ref` 与 `statement_key` 一起对应“内容（加密）”，这不准确。[memory.rs](../../crates/soul-schema/src/memory.rs):35–52 的正文引用是 `SealedText`，而 [inference.rs](../../crates/soul-schema/src/inference.rs):39–56 的 `statement_key` 是普通 String，当前生产者写的是固定模板键，没有独立正文 CK。[store.rs](../../crates/soul-store/src/store.rs):978–985 仅注册现有记忆标题/摘要的密封引用；[forget.rs](../../crates/soul-store/src/forget.rs):752–756 将推断置 orphaned 而保留其 JSON。

`[H-设计风险]` 将自由文本论断或敏感 scope/exception 直接追加到 JSON，可能在销毁原证据 CK 后留下复制内容。主库仍由 SQLCipher 加密，这里讨论的是**逐内容遗忘**，不是已证实的现有明文数据库漏洞。

最低修正：区分固定 statement key 与密封正文；逐字段确定 CK 归属、行/字段 AAD、来源依赖失效及遗忘后重开断言。方案 B 虽不建新表，仍需处理现有 JSON schema 的 `additionalProperties: false`、版本和旧记录缺字段语义（[memory schema](../schemas/memory.schema.json)、[inference schema](../schemas/inference.schema.json)）。旧字段缺失不能默认“全局适用”或“现在有效”。本轮不新增 hash、contract 冻结或 gate。

### R3 P1 自动候选发现的本地范围与出网许可必须分清

`[V-静态]` 框架 §5.3 的“不出网＋用户确认”不足以支持“现有授权语义完全不变”。当前 [DraftMemory.tsx](../../apps/desktop/src/routes/DraftMemory.tsx):43–84 与选择器仅处理明确候选，逐条本人节选另行选择。自动发现候选会在确认之前扩大本地读取范围；本地匹配不等于已经取得本次用途和正文读取许可。

同样，框架将 Observation 描述为“不可变、带用途许可”，不能仅由现有 Event+Evidence 类型推出：Event append 拒绝重复 ID，但 Evidence 是同 ID upsert（[store.rs](../../crates/soul-store/src/store.rs):640–655、:799–810）；Event consent_id、Evidence privacy 可缺省。现有调用者可能提供更强检查，这些事实**不证明已有授权绕过**。

最低修正：明确候选发现的获准本地范围、主体和用途；确认候选不等于 OwnerExcerpt，也不等于最终 E1 批准。保留当前来源/遗忘/许可复验、隐私分类、预算和单次消费。Observation 的不可变与用途约束应写为待定义投影要求，不能依靠“有 evidence UUID”推断成立。

### R4 P2 所有机器理解必须确认才生效会改变现有冻结语义

`[C]` 框架 §3.3/§6/B055、B133–135 将任何机器理解限定为确认后生效；现有 [build.rs](../../crates/soul-graph/src/build.rs):165–170 未锁时直接采用 T4D machine_band，[profile service](../../crates/soul-profile/src/service.rs):476–502 记录 Unreviewed 推断后，未锁轴会 Applied、已锁轴会拒绝改动。

最低修正：若本意是约束新 LLM 生成的理解，应写“新 LLM 理解只生成待确认提案”，并明确冻结确定性推断仍保留机器值、证据及用户锁优先。若希望全部推断改成确认后生效，必须把它标为产品/算法语义变更。§5.10 的“Strong 只来自纠正”仅适用于当前 A0 特质轴路径，不能推广为所有 Claim/T4D 的通则。

### R5 P2 VAL-lite 当前字段不能支持全部五项周指标和 T2 金标

`[V-静态]` 框架 §7/B140 的记录是场景、Claim 数/kind、E1、使用结果、耗时、评分与失败分类；没有明确拒绝/取消/失败/未评分分母、缺失耗时口径、事实错误条数或“同类错误”的关联。只有 Claim 数与 kind 也不能重建 T2 所需的具体候选、适用状态及必要/禁用材料。

`[H]` 仅记录完成并评分的任务会高估采纳率；缺测耗时按零或混入中断时间会扭曲编辑耗时；将“理解错”勾选直接当事实错误数不成立；无任务族/纠正关联无法回答是否复发。

最低修正：定义每项指标的分母、缺测、取消及计时边界；用最小必要的枚举/本地关联记录实际可获得状态。若北极星保留“少重复解释”，需有对应观察口径，或明确本阶段尚未测量。用于 T2 的材料应另行经过来源、范围和保存许可确认；脱敏摘要不是自动授权保存真实正文的测试集。复用 [VAL-01 现有缺测与人工元数据设计](../design/2026-10-19-val-01-evaluation-design.md)，不照搬全部盲评流程，也不删除失败记录。

### R6 P2 A/B 路线不能从干净分支与 CI 绿直接跳到真实 Telegram

`[C]` 框架 §8 的 A/B 阶段与当前 [ROADMAP](../ROADMAP.md):5、:82–102、[PRODUCT_LOCK](../PRODUCT_LOCK.md) 的 D107 实验定位及 [STATUS](../STATUS.md) 暂停状态不一致。D107 已替代旧近期顺序，但没有提前 MODEL 的生产实现，也没有自动授权真实使用；M0、正式 VAL-02 的前置仍保留。没有 hosted CI，局部测试、完整 G-W/G-L/G-M 与作者实际观察是不同证据。

最低修正：把“切换唯一主线为真实 S1、MODEL-01 提前、VAL-lite 先行”继续放在待拍板选择中，并标明各自改变哪项现有决定。实际使用需明确同意和拟用路径的运行依据；既不把框架中的每日任务量当许可，也不把任何局部绿色文字当正式验收。本轮不要求恢复 G-L、安装、采集或真实数据实验。

### R7 P2 当前状态与已有能力的几处表述需要更新

`[V-静态]` §2.3/B036“源码大量 dirty，只是推送同名分支”错误。开工源码与 HEAD 一致，dirty 仅三份文档；未提交内容也不会由普通 Git push 推送。框架 41 个提交/23 个触及代码的统计可按约定范围复算，但“代码”扩展名和日期口径需写明；文档篇数不能混用文件数与 Markdown 篇数，也不能据此直接断言开发无效。

此外：

- S2 不是全靠手工：导入事务已自动创建本地统计观察事项（[insights.rs](../../crates/soulcore/src/commands/insights.rs):212–233）；尚缺的是聊天语义承诺提案。
- 单人人事统计摘要已经是 UI→IPC→Session→policy→A2/可选 E1 的生产链（[Graph.tsx](../../apps/desktop/src/routes/Graph.tsx):292、[draft.rs](../../crates/soulcore/src/commands/draft.rs):1005–1038）；缺的是开放任务驱动的事件/承诺联合解释。
- values/boundaries 的 schema 形状是 `Value`，但已有 question/event/evidence 指针和密封来源展示（[profile.rs](../../crates/soulcore/src/commands/profile.rs):553–569）；“未形成任务使用”不等于“完全无结构”。
- AGENTS 允许常规短检查直接记录在所属文档，并不要求每次小改动新建一篇设计和一篇评审。可以压缩 STATUS 摘要，但不能用“只更新一份文档”抹去独立评审、失败与原始证据。

## 建议的决策顺序

1. 先修订上述事实和契约语义，使框架与现行权威关系清楚；不把“统一 Claim”解释为新建全系统模型或重新实现安全边界。
2. 由作者决定是否把已暂停的研究主线切换为 S1 产品闭环。若切换，区分正式 VAL-02 与另行限定的自用观察，不沿用附件当作真实数据授权。
3. 只选择一个可验收缺口：例如获准候选发现，或一种明确反馈→确认后的偏好更新。它们分别解决“反复挑材料”和“改完仍不记得”，不把 C–G 全部阶段同时启动。
4. 真实效果未知就保留未知；合成测试继续用于安全不变量和可构造的算法/场景反例，但不宣称证明真人效用。“所有新研究必须等真实失败”不应禁止发现遗忘、授权、预算等无需真实数据即可暴露的缺陷。

以上是审核建议，未形成新决议、实现计划或实施授权；本轮到审核交付停止。

## 本轮实际验证和留痕

本机持久证据目录：`E:/SoulSyntheticLab/framework-review-20261022/`，未随仓库发布。源文档副本、提取文本和脚本用于重读；无真实聊天、真实库、密钥或凭据。

| 活动 | 命令或方法 | 结果与证据 |
| --- | --- | --- |
| 候选状态 | `git status --short`、`git branch --show-current`、`git rev-parse HEAD`；`git diff -- docs/PLAN_INDEX.md docs/STATUS.md docs/design/2026-10-22-open-task-flow.md` | 均退出 0；初始状态与既有 diff 保存为 `initial-status.txt`、`initial-docs.diff` |
| 框架读取 | bundled Python 运行 `extract_framework.py`；OOXML 按正文顺序提取 | 退出 0；`extraction.log`、`extraction-metadata.json`、`framework-extracted.txt`；无版式验收 |
| 数据/Claim 核验 | 定向读取 schema、store、forget、correct、算法与安全依据 | 已完成静态核验；`claim-schema-notes.md`、`claim-schema-commands.log`；子 Agent 未运行测试 |
| 任务闭环核验 | 定向追踪 UI→IPC→Session→selection/draft/assistance | 已完成静态核验；`task-loop-notes.md`、`task-loop-commands.log`；未运行 UI/IPC 测试 |
| 路线独立核验 | Git/文档统计、当前决定、VAL 测量与授权对照 | `framework-route-notes.md`、`framework-route-commands.log`；未进行远端请求或重新查询费用 |
| 当前局部回归 | `cargo test -p soul-memory -p soul-algo-trait -p soul-draft --locked --offline`，`CARGO_BUILD_JOBS=1` | **退出 0，233 passed，0 failed / ignored / filtered；doc-tests 0。** `targeted-tests.log`、`targeted-tests-metadata.txt`、`run-targeted-tests.ps1`；Windows / PowerShell 7.6.5 / Rust-Cargo 1.83.0；2026-10-22 19:24–19:26 PDT |
| 主线程定向复核 | authority 的相反机器值、手选/时间澄清、草稿本地状态；检查实际测试汇总 | `authority-counterexample.txt`、`main-static-checks.log`、`main-receipt.md`；静态反例没有当作新生产故障 |
| 主报告定向独立复核 | Claim/schema 核验者复核 R1–R4/五层矩阵；路线评审者复核现行决定、暂停、VAL 与测试日志/分母 | 两者在各自覆盖范围无新增具体问题，已追加各自 notes/log；未重复运行产品测试，不代签平台或作者验收 |
| 记录与链接检查 | 见 `final-document-checks.log`，仅检查本轮文档、证据可定位性及既有修改保留 | 不替代产品运行，不签 G-W/G-L/G-M |

主线程路径定位有三次失败，均非产品测试失败：误猜 `session_tests/mod.rs`、`soulcore/src/commands/correct.rs`、`soul-graph/tests/corrections.rs` 不存在；分别保留原失败/摘录与退出码，随后通过实际文件列表定位。子任务的失败与早期未完整留存输出也在各自日志中注明，未用后一次成功冒充首次成功。

首次文档检查在新增文件的 `git diff --no-index --check` 返回 1 时中止；复核没有空白诊断输出。已保存 `document-check-attempt1.log`，改为直接核验新增文件尾空白与结尾换行，并保留 tracked 文档的原 Git 检查；这不是产品回归失败，也没有因此重跑产品测试。

`[T]` 未执行根完整测试、独立桌面 workspace、前端、完整构建/安装、完整 G-W/G-L/G-M、真实数据/真人效用、模型请求、schema 迁移或新 Claim 遗忘演练：本轮没有业务实现，既有研究暂停且本轮仅需定向审核。233 项仅覆盖所列三个 crate，不能代表全部 T1、五场景达标或 Goal 关闭。

记录维护：已查看 STATUS，规则启用日为 2026-10-22，首次完整周整理到期 2026-10-29，本轮尚未到期；只整理本次审核材料，未声称完成历史存量盘点。
