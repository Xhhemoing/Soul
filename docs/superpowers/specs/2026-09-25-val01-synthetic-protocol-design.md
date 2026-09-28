# VAL-01 合成个性化评测协议设计

## 目的

VAL-01 要把路线图中的首轮个性化评测约束变成一套可以机械检查的协议。当前交付仅使用合成任务和合成记录，证明任务定义、A/B/C 条件、顺序平衡、反馈字段、分母、缺失处理、截止规则与隐私停止规则彼此一致。

这项工作不开展真实用户试验，不生成效用结论，也不把合成干跑解释成个性化有效。真实作者试用仍依赖 M0 关闭、可用构建、明确开始窗口和逐次授权。

## 权威输入与源码身份

- 实现基线：436ea1642cdcf1106028d59d9859c1550e1e5e69。
- 分支：codex/val01-protocol-20260925。
- 路线图输入：E:/Project/Soul/docs/ROADMAP.md，SHA-256 3639FC0F1FBB2E3E1201D299D21B62C6A884D6017D1703128FF345414CAC07AA，重点为第 94–105 行及第 137–161 行。
- readiness audit：D:/Soul-planning-evidence-20260925/val01-readiness-audit.json，SHA-256 D6D0F33A1ADF5DBD03A5E4809B07072EFF25DEAD053DFFCA29AB94CEFB19CD7E。
- RES-01 source audit：D:/Soul-planning-evidence-20260925/res01-source-audit.json，SHA-256 BA787228E3173973A4039097D4C66A4A00CB0F879884E1F3F35B5DA541EFD7B9。

路线图快照位于主工作树的未提交规划层，不作为本分支的产品源码。本规格复制并冻结完成 VAL-01 所需的具体约束，不依赖执行者从该工作树推断缺失语义。

## 已选方案

用户于 2026-09-25 批准方案 A：固定三类起草任务，配套协议文档、合成 fixture、反馈记录 JSON Schema 和确定性 PowerShell 干跑验证器。

未选方案：

1. 把起草、关系纠正和记忆遗忘混在一个评测中。三者没有统一的 A/B/C 候选和编辑耗时口径，无法形成同质分母。
2. 只写说明文档。没有 fixture、结构约束和失败样本时，顺序、分母和缺失处理仍不可机械证明。
3. 在 Rust 产品 crate 或 IPC 中实现评测。VAL-01 当前不需要产品能力，且路线图明确禁止将评测入口偷偷发布。

## 范围

### 包含

- 三类合成起草任务及固定字段。
- A、B、C 三种 brief 来源的语义。
- A 对 B 的主要比较及 B 对 C 的诊断比较。
- 固定种子的任务顺序和左右候选平衡算法。
- 手工反馈记录结构、指标分母和缺失处理。
- 作者试用窗口的预注册规则。
- 本地记录、汇总、脱敏、保留与删除规则。
- 隐私或安全问题的暂停条件。
- 只读取合成文件的干跑验证器及其自测。

### 不包含

- 产品 UI、IPC、schema lock、数据库或算法修改。
- 模型、端点或网络调用。
- 自动生成候选文本。
- 自动收集反馈、遥测或研究导出。
- 联系、招募或提醒参与者。
- 真实聊天、姓名、账号、密钥、端点或体验记录。
- VAL-02、RES-02、VAL-03、RES-03 的结果。

## 交付物边界

后续实现计划仅创建以下七个聚焦文件：

1. docs/evaluations/val01/personalization-protocol-v1.md
   人可读协议，定义任务、比较、指标、窗口、停止和数据处理。
2. docs/evaluations/val01/manifest.json
   固定协议版本、顺序种子，并记录协议、fixture、schema 和干跑记录的 SHA-256；manifest 不对自身做哈希。
3. docs/evaluations/val01/task-fixtures.json
   六个合成任务，每类两个，用于干跑，不代表真实试用样本。
4. docs/evaluations/val01/feedback-records.schema.json
   评测专用 JSON Schema，验证记录 envelope 及其中每一条 comparison record；不进入 docs/schemas，不受产品 schema freeze 管理。
5. docs/evaluations/val01/dry-run-records.json
   只含 synthetic participant 的合成记录，覆盖完成、平局、缺失、生成失败和隐私暂停。
6. scripts/validate-val01-protocol.ps1
   fail-closed 验证器，读取上述文件并输出机器可读回执。
7. scripts/test-val01-protocol.ps1
   使用临时副本执行有效和变异失败测试，不改产品文件。

不新增 crate、npm 依赖或第三方 PowerShell 模块。

## 三类任务

所有 fixture 都必须使用明确标记的合成内容，不能影射真实个人。每项固定 task_id、task_class、source_material、requested_outcome、constraints 和 synthetic=true。

### 1. reply_to_inbound

用户回复一条已有消息。source_material 是合成的短消息，requested_outcome 描述回复目的。任务必须标记 conversation_shape=reply，并为所有 arm 使用完全相同的输入材料、上下文预算和约束。

这类任务检验资料是否改变回复的表达方式，不检验事实检索。候选不得补充输入中没有的承诺、日期、关系或经历。

### 2. initiate_coordination_request

用户主动发起协调、请求或安排。任务标记 conversation_shape=opening，没有需要回复的原消息。source_material 只含合成的目标和必要事实。

候选可以改变语气、直接程度和组织方式，但不能新增时间、地点、权限或对方意图。

### 3. rewrite_existing_draft

用户提供一段合成草稿并要求改写。source_material 包含原草稿，requested_outcome 说明希望改善的表达属性。

候选必须保留草稿中的事实和行动边界。不能把改写任务变成代替用户决定、扩写经历或推断人格。

每类提供两个 canonical fixture，仅验证协议覆盖。未来作者试用的至少 30 个配对任务不得通过重复这六个 fixture 冒充真实任务量。

## A、B、C 条件

- A：无个性化基线，对应 ProfileBrief::neutral()。
- B：现有自动档案视图，对应 ProfileBrief::from_view()。
- C：用户在任务前手工提供的简短偏好参考。

同一任务的三个条件必须固定模型标识和版本（端点提供时）、参数、输入、脱敏策略、上下文预算和候选生成时点。C 不能临时获得 A/B 没有的事实材料。协议记录 brief 来源和条件标识，不在仓库保存真实 brief 或候选正文。

每个任务形成两个独立盲比较：

1. primary：A_vs_B，用于主要个性化偏好指标。
2. diagnostic：B_vs_C，用于判断自动整理档案相对手工偏好参考是否仍有额外价值或明显损失。

不增加 A_vs_C。评估者只看到 left 和 right，不看到 A、B、C 标签。

## 固定顺序与平衡

协议种子固定为 soul-val01-v1-order。

对每个 participant_key：

1. 以 SHA-256(seed | participant_key | task_id) 作为任务排序键，按十六进制升序形成稳定任务顺序。
2. 以 SHA-256(seed | participant_key) 的首字节最低位作为 participant_offset。
3. A_vs_B 的左右顺序由 (task_rank + participant_offset) mod 2 决定。
4. B_vs_C 使用相反 parity，由 (task_rank + participant_offset + 1) mod 2 决定。

偶数任务数时，每种比较必须精确左右各半；奇数任务数时差值不得超过一。验证器重新计算顺序，不信任记录中的自述。

participant_key 在合成干跑中只能使用 synthetic-pNNN。真实试用的 pseudonymous key 由用户在本地生成，不进入仓库明细。

## 反馈记录契约

dry-run-records.json 的根对象包含 schema_version、protocol_version 和 records。feedback-records.schema.json 验证整个 envelope，并对 records 中每一条 comparison record 拒绝未知字段。comparison record 至少包含：

- schema_version、protocol_version、record_id。
- phase：synthetic_dry_run、author_pilot 或 volunteer_pilot。
- participant_key、participant_group。
- task_id、task_class、task_rank、comparison。task_rank 从 0 开始。
- left_arm、right_arm、order_seed。
- outcome：completed、abandoned、refused、generation_failed 或 safety_stopped。
- blinded_choice：left、right、tie 或 null。
- edit_seconds：非负整数或 null；只有 A_vs_B 的 completed 记录可填写。
- failure_type：none、missing_candidate、model_error、task_ambiguous、user_declined、privacy_issue、safety_invariant、other。
- privacy_issue：布尔值。
- model_conditions：只记录模型标识/版本（可得时）、参数摘要、上下文预算和条件哈希；不得包含密钥、端点或正文。
- started_at、completed_at：真实 phase 的 started_at 必须是带时区时间；completed 记录的 completed_at 也必须是带时区时间，其他 outcome 必须为 null。synthetic_dry_run 使用固定合成时间。

completed 记录必须有 blinded_choice。B_vs_C 是 preference-only，edit_seconds 必须为 null。未完成、拒绝、生成失败或安全暂停的记录必须令 blinded_choice、edit_seconds 和 completed_at 为 null，并给出非 none failure_type。privacy_issue=true 必须同时 outcome=safety_stopped 且 failure_type=privacy_issue 或 safety_invariant。

## 指标与分母

协议同时报告 offered denominator 和 valid denominator，禁止只留下成功样本。

### A 对 B 偏好

- offered_AB：所有已排定 A_vs_B 的任务。
- valid_AB：两侧候选均存在、完成盲选且未触发安全停止的任务。
- B 胜记 1，平局记 0.5，A 胜记 0。
- 先按 participant 计算 valid_AB 平均分，再对参与者分数取中位数。
- abandoned、refused、generation_failed、safety_stopped 不进入偏好均值，但必须分别占 offered_AB 的数量和比例。
- valid_AB=0 的 participant 记为 insufficient，不填 0。

### B 对 C 诊断

使用同样的 offered_BC 和 valid_BC 规则，单独报告，不与主要 A_vs_B 指标合并。

### 编辑耗时

- 只对 completed 的 A_vs_B 记录使用 edit_seconds；时间从显示两侧候选后开始，到用户认可基于盲选候选形成的最终文本为止。
- 每个 participant 分别报告 A 或 B 被选中后的原始中位秒数和样本数；C 只参与 preference-only 的 B_vs_C 诊断，不计算编辑耗时。
- 比较比例时，基线为 0 秒的任务不进入比例，但原始值仍报告。
- 未完成任务不得以 0 秒填充。

### 完成与失败

完成率分母是所有 offered 比较。refused、abandoned、generation_failed 和 safety_stopped 分别报告。隐私或安全失败不能被偏好或耗时收益抵消。

干跑验证器只证明计算规则和缺失处理，不产生真实指标结论。

## 作者试用窗口

author_pilot 在 M0 关闭且用户明确开始后才可创建 pilot-window.json。该记录必须在第一次候选生成前冻结：

- start_at：第一项获准任务之前的带时区时间。
- cutoff_at：start_at 加 14 个自然日。
- target_pairs：至少 30 个 A_vs_B 任务，并为相同任务生成对应 B_vs_C 诊断比较。
- protocol_version 和 fixture/条件哈希。

开始后不得为追逐结果延长 cutoff 或增加目标样本。到期不足 30 项、条件变化或有效分母不足时，结论为 uncertain。当前实现不创建 pilot-window.json，也不启动倒计时。

## 数据最小化、保留与删除

- 仓库只保存合成 fixture、合成干跑记录和经人工检查的聚合收据。
- 真实输入、候选正文、手工 brief、姓名、账号、端点和密钥永不提交仓库，也不交给 agent。
- 真实逐任务结构化记录只保存在用户选择的本地目录；仓库汇总不得包含 participant_key。
- 原始本地结构化记录的默认删除期限为 cutoff_at 后 30 天。聚合验收完成后可提前删除。
- 隐私事件如需排查，只记录固定类别和计数；原始内容在完成必要的本地排查后立即删除。
- 删除期限、实际删除时间和未删除原因必须进入本地完成回执；没有回执不能声称数据处理完成。

合成文件可随仓库保留，因为内容必须明确为虚构且不含真实标识。

## 暂停条件

出现以下任一情况，相关 phase 立即停止，不能继续补样本：

- 未授权原始内容进入模型请求或评测记录。
- 第三方、mixed 或其他无用途许可材料被当作可研究反馈。
- 用户锁定的纠正被静默覆盖。
- 已遗忘内容仍可在候选、缓存或记录中读取。
- 密钥、端点、账号或可识别个人信息进入 fixture、记录或日志。
- 候选新增输入中不存在的高风险事实、承诺或权限。
- 模型/任务/参数变化使配对条件不再可比。

安全暂停记录保留 failure category，不保留触发正文。恢复必须经过现有缺陷修复和独立验证，不能由 VAL-01 验证器自行解锁。

## 验证器行为

scripts/validate-val01-protocol.ps1 使用 PowerShell 7 内置能力：

1. 首行设置 ErrorActionPreference=Stop，所有文本按 UTF-8 读取。
2. 解析 protocol、manifest、fixture、schema 和 dry-run records；重算 manifest 中除 manifest 自身外的每个 SHA-256。
3. 校验文件哈希、版本、唯一 ID、三类任务及每类两个 fixture。
4. 校验 fixture 全部 synthetic=true，禁止出现真实 participant phase。
5. 按固定 SHA-256 算法重算 task_rank 与左右顺序。
6. 校验 A_vs_B 和 B_vs_C 平衡。
7. 校验 outcome、choice、edit_seconds、failure_type 和 privacy_issue 的交叉约束。
8. 重算 offered/valid 分母、偏好分数、失败计数和缺失处理。
9. 拒绝 pilot-window、真实 participant key、原始候选正文和禁止字段。
10. 输出 JSON 回执，包含输入哈希、计数、检查结果和 errors；任一错误退出非零。

验证器不调用产品二进制、模型、网络或数据库。

## 自测策略

scripts/test-val01-protocol.ps1 在临时目录复制有效 fixture，然后执行：

1. 有效完整干跑：通过。
2. 缺少一种 task_class：失败。
3. 重复 task_id：失败。
4. 左右顺序与固定算法不符：失败。
5. A_vs_B 或 B_vs_C 不平衡：失败。
6. completed 但 choice 缺失：失败。
7. 未完成却填 edit_seconds：失败。
8. privacy_issue 未触发 safety_stopped：失败。
9. synthetic 文件出现 author_pilot 或非 synthetic participant：失败。
10. 增加 raw_text、endpoint、api_key 或未知字段：失败。
11. 把缺失样本填成 0 分或 0 秒：失败。
12. 修改输入而不更新 manifest 哈希：失败。

每个失败用例必须验证非零退出和稳定 reason_code，避免只匹配任意错误。

## 错误处理

- JSON 无法解析、schema 不匹配、文件缺失、哈希漂移或统计不可重算时 fail closed。
- 验证器先写包含 errors 的回执，再以非零退出。
- 不自动修复 fixture，不猜测缺失值，不把 null 转成 0。
- dry-run 失败不影响产品文件，也不触发网络或模型重试。

## 验收

设计落地后的实现必须证明：

- 所有七个交付文件位于限定路径，工作树没有计划外变更。
- 有效干跑通过，十二类变异均按预期失败。
- 输出分母与手工计算一致，valid=0 时为 insufficient 而非 0。
- 固定种子在重复运行中产生相同顺序和哈希。
- 仓库示例只含 synthetic phase 和 participant。
- 静态扫描未发现密钥、端点、真实标识、产品 IPC 或 schema lock 变更。
- pwsh 自测、验证器正式干跑和 git diff --check 均退出 0。
- 独立规格审查和质量审查没有未解决的 Critical/Important finding。

## 可声明与不可声明

完成后可以声明：VAL-01 协议结构、合成任务、顺序、分母和隐私停止规则已经机械验证。

不能声明：

- 个性化优于基线。
- 用户更喜欢 B 或 C。
- 编辑时间改善、留存、首次任务成功或商业价值。
- 已获得研究同意。
- 模型、人设或人格表示科学有效。
- VAL-02、RES-02、VAL-03、RES-03、Q2-00、正式 Q2-05、Goal 1 或 Goal 2 已关闭。
