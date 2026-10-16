# 最新框架剩余能力的实施计划

基线 `b66f093a1407887eb873fe33defe984906252d29`，PR #66。作者要求继续按计划完成未完成部分；仍逐板块规划、独立复核、编码、实际验证，不将局部检查写成完整平台验收。

## 范围与顺序

| 模块 | 要交付的完整用例 | 验收重点 |
| --- | --- | --- |
| W1 扫描 | 现授权目录 → 锁外只读扫描 → 审计/版本校验 → 有归属的计划预览 | 单真实槽、控制可达、未授权零扫描、旧结果不展示、无写文件命令 |
| W2 记忆上下文 | 本次明确候选 → 手选/受限检索 → 完整出处预览 → 受控起草 → 修改/遗忘失效 | 用途不继承、默认不外发、真实序列化体预算、来源可解、旧结果拒收 |
| W3 导入/恢复 | 不可变准备 → 操作 ID → 锁外解析/事务 → 持久计数回执 → 重挂/重启查询 | 不在 Session 等长 store、领域/回执原子、不能自动重放、并发所有权 |
| W4 去重/迁移 | 明确来源身份 → 封存自然键 → 去重/冲突 → 遗忘传播 | 不存正文 hash、旧数据缺 ID 不猜、编辑冲突拒绝、墓碑不复活、升级失败回滚 |
| W5 本地闭环 | 导入提交 → 现有算法的本地结果 → 证据 → 纠正/忽略 → 失效/再算 | 不自行出网、不是新关系评分、反馈不冒充自动训练、重启状态可核验 |

Android、多端同步、新云端、自动代发、文件写执行与冻结人格算法修改仍属于独立未来版本。真实资料/用户实验不由此自动授权。缺失事实时间或不可规范化的语义必须澄清，不能为“完成”伪造模型能力。

## W1：先规划并验证

输入保持 `preview_plan(path)`，输出保持 `PlanPreview`。Session 锁内登记唯一工作、复制已有目录授权，并把唯一 file-plan `TokenIssuer` 移入 owned work；不复制 token ledger。锁外执行原文件计划服务。完成时匹配本会话工作身份、归还 issuer、记录实际扫描及注入审计，再核对来源/授权代次，最后交付。审计失败和来源失效都归还资源。物理 worker 未结束不释放槽。

文件动作本来不发能力令牌。仅当被放弃的 Scan lease 已终结，才可重建空 file-plan issuer；这只恢复容量，不能声称直接丢弃 completion 仍完成审计。生产 Tauri 持同一 Session 的闭包总负责 complete。

原有同步 Session 调用保留兼容，原生壳改 async，HTTP/扫描/未来导入共用慢任务准入；本机模板和只读短操作不因此被全面禁用。旧 import 入口也必须尊重在途扫描。页面在调用前同步占槽，编辑/卸载失效展示但不冒称核心取消；成功授权只有确认仍属于原输入才清空，core wrapper 补交付前来源校验。

独立规划复核通过后开始 W1 编码。反例：两工作不能同时准入；外来 completion 不偷槽；源变后扫描审计仍在但内容不返；编辑后成功不清新输入；卸载回包不显示；未授权路径拒绝且无 walk；原 39 命令形状与只读保证保留。

## W2：本次记忆与受限检索（PLAN REVIEWED）

核心新用例先接收明确候选 UUID（最多 32）、本候选内关键词、类型与是否询问特定事实时间，调用 `soul-memory::select_for_draft`。结果只包含实际可读的手写记忆、两条密封 blob 出处与排除原因。当前 schema 没有事实有效时间，特定时间查询返回 Clarify；关联 opaque 来源无法证明可用时排除。Clarify/Abstain 不成为空集合的可用记忆许可，普通无记忆模板仍是用户可选路径。

`prepare_memory_context` 持有唯一有代次的 selection；返回完整本地预览与临时 context UUID。本机起草只伴随这些可核对参考，不把文本拼成已推断事实。本次记忆用途在实际本机起草或端点 dispatch 时消费。端点准备只接受本 snapshot 明确 Owner 的勾选子集，第三人/混合/未知始终占位；原 paste 豁免仅指向 paste，不传给 memory。新预览默认不勾出网内容。

档案、paste 与每条 memory 分别脱敏；单次 paste 豁免只消费一次。policy 的组合器仅接收已经脱敏的 `RedactedBody`，checked 合计并拒绝多份豁免。预算循环仅组合已经脱敏的材料，使用与实际请求相同的 JSON serializer 精确计算完整字节数（上限 64 KiB），按整条 memory 移出，再重算最终预览、引用、Owner IDs 与实际 body 预览；不把 UTF-8 字节数称为 token 数。

Held → Pending → owned Work/Completion 都携带实际保留的 snapshot 与准备身份；不能用完成时的新 HeldMemory 替代旧依赖。发送前及 attempt audit 后分别复验实际 snapshot 与代次，失效结果不写 `draft.create`。候选、查询、Owner 勾选变化与 paste 编辑一样走现有 discard drain；来源变化也清掉已展示参考和结果。Memory 页明确选条目，当前 WebView 只保留 UUID；不自动全库解密、持久授予用途或继承研究许可。

反例包括候选外零读取、遗忘/编辑/重启、无效 AAD/CK、linked 来源、unknown 隐私、中文与 JSON 转义预算、paste 豁免不扩大、旧准备与新 context 交错、最终正文/出处/勾选一致、交付到页面前的来源变化。库层 10 项测试已实际通过；跨层实现和验证继续进行。

## 其他模块的语义决定

后续各模块先把具体接口、迁移与反例补入本文或对应设计，再评审和编码。用户本轮“全部完成”作为继续实现当前计划的授权；安全缺省是本轮工程选择，不伪装成作者已逐项指定。每次迁移仅对合成库执行验证，不接触用户真实资料。

## W3：持久导入与恢复

本节方案先在会话经独立规划复核，再实现底层。生产入口采用 `prepare_import(text, source, namespace, legacy_choice)` 产生操作/预览 UUID 并持有唯一不可变 `StagedImport`；准备成功交付前写入无正文 reserved 记录。`confirm_import` 只接两个 UUID，不重新接收正文。discard/replace 必须指名原操作；迟到确认不得使用新材料或新批准。

底层接口位于 `soul-store::import_ops` 与 `soul-store-api::import`。`ImportRuntimeLease::claim(database_path)` 持 OS 非阻塞文件锁，路径规范化，文件不删除、不按是否存在判断占用；Session 与真实 worker 保留同一持有期。生产 Session 未取得 lease 时拒开整个可写 store。lease 只成功恢复一次，恢复后才能 reserve/start；不能重新调用恢复把本持有期的活工作标中断。

操作状态为 reserved → running → committed/rejected；未确认准备可 discarded/invalidated。真正独占接管的新持有期把遗留 reserved/running 改为 interrupted，不恢复正文或批准。运行中的导入以 CAS 消费预览。领域、图谱、原 import 审计及 committed 计数回执必须在同一个事务中；`commit_import_operation` 拒绝事务外调用。操作列表只返回固定枚举、UUID 和计数，来源 revoked 后展示历史回执而不声称仍有可用事实。COMMIT 或查询结果无法确认时，核心返回 unknown，不把它改写为已回滚。

## W4：自然键、迁移与遗忘

`soul-import::indexed::{prepare_indexed, validate_indexed, commit_indexed}` 共用原事件/证据构造。预检保存普通匹配主键与联系人状态；提交在同一领域事务重新分类，namespace/联系人/密钥/命中条目/重复分区改变则要求重新准备。Telegram 按稳定本人账号选择既有 namespace，拒绝同账号另造 UUID；JSONL 必须明确选择来源，不猜文件名或导出时间。

自然消息键为 namespace + conversation id + external message id。任意外部 ID 封在发送者已有内容密钥下；历史索引按 namespace 每批最多 256 项流式解封，仅对本次命中项打开事件正文，辅助内存随输入和单批大小增长。相同键且内容/元数据相同则跳过，同键变正文、时间、发送者或会话属性整次冲突；缺失消息不代表撤回。坏索引或不可解封密钥拒绝，不冒充未命中。没有新增正文 hash。

store schema 由 2 升为 3：新增操作、namespace、参与者映射、消息键表，以及只保留本地 event UUID 的 `import_indexed_events` 覆盖集合；DDL、旧历史计数与版本 stamp 原子提交。覆盖 UUID 在遗忘后保留，防止删掉密钥索引后把已遗忘事件误认未迁移历史。旧库没有外部消息 ID，不能自动回填；默认拒绝旧来源重导。明确选择 forward_only 后，预览与最终回执持续说明只为本次和后续建立索引、旧历史可能重叠，不改旧行、不宣称全历史去重。

遗忘预览新增将被撤销的 namespace 数量；精确 scope 包含 namespace 主键/状态与待删 identity 主键。销毁密钥、删除对应封存 ID 索引、撤销 namespace 在同一销毁事务完成。墓碑及 revoked namespace 阻止重导，新 namespace 也不能绕过可识别墓碑新造密钥。无正文操作计数与审计可保留，不能据此恢复密钥或重新批准。

新增回归覆盖 lease 持有、重复恢复/CAS、回执事务、重启中断、v2 升级及 DDL 回滚、重复/重叠/冲突、预览依赖变化、legacy 显式选择、坏索引，以及遗忘后解封/重导与覆盖分类。实际执行结果由主会话统一记录；本段不预报测试通过。

## 实际结果

W1 已完成独立 dirty diff 复审（REVIEWED）。实际核心 lib 21/21 通过，包含新增两个扫描工作测试；前端定向 4 文件/49 测试、TypeScript 与 ESLint 通过。初始根目录读取晚于成功授权、同步 transport throw、编辑/卸载和交付间隙来源变化均有回归。不是原生 IPC 验证。

W2 库层 selection 已独立源码复审且 10/10 测试通过，含真实临时 SQLCipher 重启、编辑与遗忘轨迹。其余层仍在实现，不把库层结果扩大到完整用例。
