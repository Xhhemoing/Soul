# v1.3 执行记录快照（含未完成项）

> 2026-10-23 America/Los_Angeles 保存；原分支 codex/soul-rebuild-20261019，HEAD c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46 加 dirty。以下保留原日期、结果、失败和未完成项，仅调整相对链接；不是当前状态或新的验收。当前入口见 [STATUS](../STATUS.md)；API 当前设计见 [local-api](../architecture/local-api.md)。

# Soul 核心框架 v1.3 修订与执行分析

日期：2026-10-22，America/Los_Angeles。状态：**D109 有限产品切片已实现，所列合成验证通过，独立源码评审无剩余阻塞；dirty、未提交，待作者验收**。最初文档任务为“按审核意见把框架修订为 v1.3，保留现有结构，然后分析如何具体执行”；作者随后要求“接下来按照计划推进落实，建议使用subagent分工协作”，确认下文三个实质决定。下文原建议与文档回执保留历史语境，实际实现以末尾实施回执为准；没有扩大为全套框架、真实效用或平台门禁完成。

## 本轮交付与依据

- Word：`C:/Users/86080/Downloads/Soul核心框架_v1.3.docx`；以 v1.2 原件为基础，保留 0–10 节、8.1、附录和 11 张表。v1.1、v1.2 原件保留。
- 审核依据：[v1.1 五层审核](../reviews/2026-10-22-core-framework-v1.1-review.md)、[v1.2 增量审核](../reviews/2026-10-22-core-framework-v1.2-review.md)。既有权威仍为 PRODUCT_LOCK、DECISIONS、ROADMAP、SECURITY 与 ACCEPTANCE。
- 本轮起点：`codex/soul-rebuild-20261019`，HEAD `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46`。业务源码无 dirty；STATUS、PLAN_INDEX、开放任务暂停记录和两轮审核记录已有未提交变更，均保留。
- 证据目录：`E:/SoulSyntheticLab/framework-v1.3-20261022/`。源副本、逐段修订映射、提取文本、代码取证、渲染及失败记录只在本机保存，不暗示已随仓库发布。

核心修正：旧记录缺 scope／valid 保留未知；生效值按对象真实来源投影，机器值和纠正证据可缺省；Corrected 不代表机器原结论被接受；偏好必须明确本人归属、适用范围及下一任务应用；UUID 日志不等于 T2 fixture；D107 批准研究、作者随后暂停；窄范围观察不等于正式 VAL-02。全套 Claim 和自动召回均不是首切片的强制前置。

## 建议先交付的一个结果

**让用户明确保存的一项语域偏好，在下一次相同联系人、相同场合的本地模板起草中真正生效，同时不影响其他任务。**

首切片建议支持已有 `VoiceRegister::Casual / Plain / Formal` 三个选项，只做语域这一维；不新增语域常量或从措辞变化推断语域。场合建议为闭集 `personal / work / study`，这是拟新增产品字段，尚未落地。联系人限定用户显式选择的 Active 人物 UUID；群组、服务和本人不当作本次人物收件对象。任务类型固定为普通本地回复模板。

一个可以直接验收的合成例子：

1. 全局语域为 Formal；用户选择合成联系人 X、personal 场合，生成“您好，…”的本地草稿。
2. 用户编辑后主动选择“保存我的表达偏好”，选 Casual。确认页明确“我对 X 在私人场合使用随意语域；用于以后的同范围本地回复，持续到更改或遗忘”。这是显式长期选择，不从“您→你”自动推断。
3. 保存后，全局 Profile 的值和锁不变。下一次重新显式选择 X/personal，生成“嗨，…”并显示应用了哪条确认偏好。
4. Y/personal、X/work、未选联系人或场合的普通起草均不应用这条偏好。未知或失活的关联不能回退为全局规则。
5. 用户遗忘这条偏好后，既有 CK 被销毁，typed 生效值被清除，旧提案／准备失效；重新打开测试库后仍不应用。

这个例子证明的是明确范围内的确定性行为闭环，不能证明“回复更像本人”或减少真实编辑。真人效用仍未测。第一版只接普通本地模板；端点、手选记忆 W2、Insight、Assistance 和自动候选后续再决定。界面切换到这些路径时不得继续显示该偏好已被应用。

## 为什么这样拆分

| 已有基础（静态核对） | 能复用什么 | 仍需补什么 |
| --- | --- | --- |
| [voice.rs](../../crates/soul-profile/src/voice.rs) 的现有语域枚举；[template.rs](../../crates/soul-draft/src/template.rs) 的三种实际招呼 | 用现有规则验证输出变化，不调用模型 | 精确联系人／场合绑定和限定读取 |
| [brief.rs](../../crates/soul-draft/src/brief.rs) 的 `ProfileBrief::set_by_user`；[session.rs](../../crates/soulcore/src/commands/session.rs) 的 `owner_brief` 与 `draft_pasted` | 每次构造 brief，在副本应用用户确认值 | 新 scoped 入口及结果、偏好 revision 的关联；不能调用持久化全局 `set_voice` |
| [Draft.tsx](../../apps/desktop/src/routes/Draft.tsx)、[draftRequests.ts](../../apps/desktop/src/routes/draftRequests.ts) 和 [core.ts](../../apps/desktop/src/core.ts) 的请求／编辑生命周期 | 复用 revision、lifetime、取消与 source mutation 失效 | 主动反馈入口，后端验证当前结果；单纯编辑仍不回传保存 |
| [memory service](../../crates/soul-memory/src/service.rs) 与 [store](../../crates/soul-store/src/store.rs) 的 Memory/CK/事务 | 密封可读确认说明；复用 Memory 遗忘单位 | typed scope 关联、普通编辑失活及联系人遗忘传播 |
| [forget.rs](../../crates/soul-store/src/forget.rs) 的预览／当前性／销毁流程 | 扩展现有影响面和回执 | 新关联必须显式进入同一流程；现有联系人遗忘不会自动找到新 scope 关联 |

不要只保存一条普通 Preference 后宣称闭环完成：当前 brief 不会自动把它转换成语域规则，W2 也不承担这种执行语义。不要直接改全局 voice：那会把“对 X”扩成所有联系人。不要先建总 Claim 表：本切片只需要精确范围的已确认设置，来源投影与通用条件系统可以以后按需要推进。

## 最小数据方案及必须承认的成本

建议采用 **Memory(Preference)＋一个有限 typed scope 关联**。Memory 保存短而可读的本人确认说明，沿用现有密封正文、CK 和遗忘入口；关联保存 owner、person UUID、occasion、register、Memory ID、active 状态及 revision。字段与表名是候选，不是现有数据库能力。

不把 JSON 塞进 summary：当前普通 Memory 页面和 W2 会直接把 summary 当正文；通用编辑还可能使它与索引失配。也不把 applicability 偷放进 `source_event_ids`、`evidence_ids`、隐私标记或 Relationship，它们已有不同职责。

| 具体失败场景 | 最小处理 | 实施边界 |
| --- | --- | --- |
| 同 scope 双击确认、旧提案覆盖新值 | 唯一 scope 约束、一次确认、expected revision CAS；明确替换当前值 | 使用既有事务、UUID／版本语义，不加 hash、冻结 contract 或新 gate |
| Memory 写成功但关联或审计失败 | 同一事务核对、密封／写 Memory、写关联及无正文审计；任何失败回滚 | 复用 `SqlCipherStore::transact`，不新增偏好锁 |
| 普通 Memory 编辑后，旧 typed 语域仍生效 | 在 store 写边界使关联失活并更新 revision；专用确认才原子同步说明和值 | 只禁 UI 不足；通用编辑的失活原因须可见 |
| 忘记偏好正文却仍按枚举应用 | CK 销毁与关联生效值清除必须同属遗忘处理 | 枚举／UUID 不必被当自由文本逐字段密封，但仍需删除生效语义 |
| 忘记联系人后，关联到他的偏好仍生效 | 将关联 Memory CK 加入联系人遗忘影响面，含无姓名联系人；旧准备失效 | 新遗忘传播是需明确决定的语义改动；不能宣称现有实现已覆盖 |
| 遗忘预览后，同数量替换了偏好 | 预览当前性包含关联 ID/revision，旧预览要求刷新 | 沿用现有单 writer 边界；不扩大成任意跨进程 writer 支持 |
| 老版本编辑／忘记 Memory，不知道维护新关联 | 实施时核对并升级既有 DB schema version，沿用旧程序拒绝更高版本 | 当前静态值为 7；不预先占用下一号，不新增版本门禁体系 |
| 普通旧 Preference 被解析成永久规则 | 新表初始为空，不从旧正文补造 scope | 原 Memory JSON 可保持不变；兼容读取和规则生效分开 |

被替换的活动偏好可沿用同 Memory/CK 重封确认说明并更新 revision；遗忘后重新明确创建必须使用新 Memory/CK，不能复活旧密钥。未确认提案只存在 Session，取消、替换当前任务、离页或重启后不可再确认。持久化仅保存已确认规则，不保留编辑前后正文或影子历史。

有一个依赖细节需在实施前解决：`soul-profile` 已依赖 `soul-store-api`，后者不能反向引入 `VoiceRegister`。可以把同一个枚举定义下移并由原路径 re-export，保持 token 和调用路径；或让底层使用不对外开放的受验证 token。不得复制另一套语域枚举权威。此处属于局部实现选择，不要求增加 crate 或更新依赖锁文件。

## 接口与任务生命周期

下面是**拟新增接口**，命名可在实现时调整。现有普通起草入口继续保持原有语义。

| 动作 | 最小输入与输出 | 关键校验 |
| --- | --- | --- |
| `draft_scoped_reply` | pasted、显式 person/occasion → draft、result_id、scope、applied_preference_id/revision（可无） | person 有效；仅查精确 scope；当前性校验后在 brief 副本应用，再生成本地模板 |
| `propose_scoped_register` | 当前 result_id、edit_revision、用户选择的 register → proposal_id、确认预览 | 不从文字自动分类；结果属于当前 Session/任务/scope；生成或编辑切换后旧提案不可确认 |
| `confirm_scoped_register` | proposal_id、expected_revision → 保存回执 | 一次确认；值／范围和未来使用已明示；事务内复核联系人、Memory、CK 与现行 revision |
| `discard_scoped_register_proposal` | proposal_id → 取消回执 | 不留下有效偏好；之后同 ID 不可确认 |
| `lookup_confirmed_register`（内部） | exact scope → 可无的 preference_id/revision/memory_id/register | 只读当前有效关联，不遍历或解密其他 scope 正文，不用 W2 文本推断规则 |

前端 edit_revision 不是授权凭据。后端持有当前结果与提案的 scope、Session/lifetime、期望偏好版本，确认时复验。后端只能知道已收到的 IPC／生命周期变化，不能察觉仍留在页面内的打字。前端编辑或切换 scope 时先同步撤销提案 ownership，禁止再排入旧 confirm；对已建立的确认预览，按顺序发送 discard／版本失效并收到回执后再允许继续编辑。提案请求晚到时，前端拒绝展示并清理对应提案。确认已提交期间暂锁编辑与 scope，收到保存／失败回执后解锁；已进入事务并提交成功的确认不因随后编辑而被追溯撤销，用户要撤回已保存规则需走明确更改／遗忘。取消和离页的失效消息也沿既有有序 IPC 处理，未收到后端回执前不宣称服务端状态已清除。

普通本地生成可在既有 store guard 下完成读取、应用和生成，避免造一个新的异步模型协议；UI仍需通过现有请求生命周期拒绝旧结果晚到。上述顺序只传 ID／revision／动作，不逐字回传编辑正文，也不新增 hash 或 gate。

锁方向保持 Session → store Mutex → SQLite。确认使用现有事务；遗忘本来就自行开启事务并清理 WAL，不能再包一层 `transact`。未确认状态失效和已确认偏好遗忘是不同动作，但都必须让旧结果／准备不能继续应用。

## 实施顺序与文件认领

这是有限实施工作包，现已由 D109 承接授权。从存储语义起步；每一步产生可验收结果，不并行修改共享事务或 Session 协议。

| 顺序 | 工作与候选路径 | 可交付检查点 |
| --- | --- | --- |
| 0 | 在既有 DECISIONS／所属计划承接下文三项决定；核对实施时 HEAD/dirty | 明确范围、未来使用、持久化与遗忘；不变更正式 Goal 状态 |
| 1 | 数据负责人：拟新增 `crates/soul-store-api/src/scoped_preference.rs`、`crates/soul-store/src/scoped_preferences.rs`；现有 `lib.rs`、`sql.rs`、`store.rs`、`forget.rs`，必要时 fake/forget receipt | 精确查询、原子确认、普通编辑失活、三个遗忘入口、重开及旧库行为均有合成反例 |
| 2 | 同一协议串行接线：拟新增 `crates/soulcore/src/commands/session_scoped_preferences.rs`，现有 `session.rs`／`memory.rs`；必要的枚举归属调整 | Session 一次提案／确认／取消、当前性和 brief 副本应用；无需完整 Claim 或 E1 改动 |
| 3 | 桌面负责人接已确定接口：Tauri `commands.rs`、`lib.rs`，前端 `core.ts`、`Draft.tsx`、拟新增 `DraftScopedVoice.tsx`，必要的 `draftRequests.ts` 与 fakeCore | 用户能显式选 scope、看确认范围、保存、下一任务看到已应用；其他路径不冒充支持 |
| 4 | 对应现有 tests 及必要新用例；独立只读评审重点看 store、遗忘、Session/IPC 与异步 UI | 修复具体问题并定向复验，保留失败；不以方案评审签产品验收 |
| 5 | 更新本记录实际命令／结果，STATUS 留短摘要 | 交付工程行为与未测效用边界；不自动 commit、push 或勾人工清单 |

推荐一个实现者贯穿存储和 Session；UI在接口稳定后接入，可由第二实现者认领不冲突路径。Session、IPC、schema／事务和 STATUS 由协调者安排单写者；只有一份完整构建通道。独立评审可并行调查，但不擅自编译、安装或改锁文件。

## 原拟定验收与验证安排（实际结果见实施回执）

| 类别 | 至少需要的合成断言 |
| --- | --- |
| 正向应用 | 三个现有语域值可明确选择；同 person/occasion 的下一任务应用正确模板值，展示来源，重启后保留 |
| 范围与全局 | 不同人、不同场合、缺标识、未知值、失活联系人不应用；全局 voice/锁和无关任务完全不变 |
| 提案生命周期 | 仅编辑／复制无写入；编辑／scope 变化同步阻止前端排入旧 confirm，已收到 discard／版本失效后后端拒绝旧提案；确认提交中暂锁编辑；覆盖晚到、离页、重新生成、重启和确认先提交的顺序反例 |
| 原子性与重复 | 重复确认仅一条生效值；旧 revision 不覆盖；失败注入无半条规则、孤立成功审计或多余活动 CK |
| 普通编辑 | 直接 store 调用与 UI 编辑都使旧关联失活；不靠解析普通 Memory 正文恢复执行规则 |
| 遗忘 | Memory、CK、联系人（含无姓名）遗忘均销毁说明、清生效值、失效旧准备；同数量替换使遗忘旧预览失效；重开不复活 |
| 兼容 | 旧库无自动 scoped 规则，未知仍未知；更高版本拒绝按既有协议；破损关联不能回退应用 |
| 读取与审计 | 精确查询不解密其他 scope；审计无草稿／编辑正文；本地模板路径零模型／端点调用 |

先用现有 synthetic fixtures 和临时测试库写必要失败用例，再实现并跑定向检查。新的 `scoped_` 测试命名仅是建议；运行后必须核对实际执行数量，零匹配不能当通过。拟用命令示例：

```pwsh
$ErrorActionPreference = 'Stop'
cargo test -p soul-store -p soulcore --locked --offline scoped_
if ($LASTEXITCODE -ne 0) { throw 'Scoped preference tests failed' }
cargo test -p soul-memory -p soul-profile -p soul-draft --locked --offline
if ($LASTEXITCODE -ne 0) { throw 'Adjacent memory/profile/draft tests failed' }
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline --test command_surface
if ($LASTEXITCODE -ne 0) { throw 'Desktop command surface failed' }
pnpm --filter @soul/desktop test -- src/routes/Draft.test.tsx src/routes/DraftScopedVoice.test.tsx
if ($LASTEXITCODE -ne 0) { throw 'Draft UI tests failed' }
```

拟新增桌面 `scoped_` IPC 用例与前端新文件在实施时才存在；`command_surface` 只查命令面，不能代替新 IPC 行为测试。此处不复制未经核对的完整门禁 recipe。最终按实际改动补齐根／桌面格式、相关静态检查与 schema 检查；UI若改生产路径则运行适当生产构建。限定合成时继续排除 AGENTS 指明的三项真实采集用例；不为本切片恢复暂缓 G-L，也不把局部测试签为完整 G-W/G-M。

## 开工前应明确的三个实质决定

1. **范围与优先级**：接受普通本地模板、精确 Active 人物 UUID＋有限场合、现有三种语域这一维；将其作为一个提前的最小产品切片，不自动启动其余 MODEL/Claim/Proposal 工作。
2. **用户语义**：确认同时允许后续同 scope 的有限读取／应用，持续到更改或遗忘；这是用户明确指定的全局语域局部例外，全局值与锁不变。普通 Memory 编辑会停用该执行规则，须重新明确确认才启用。
3. **数据与遗忘**：采用 Memory＋typed 关联，进行必要的数据库版本变化；Memory/CK/联系人遗忘全部清除对应生效设置并销毁关联密封说明。遗忘后新确认使用新 Memory/CK。

这三项可作为一次明确的有限实施授权，不必逐文件索取许可。具体模块名、内部 token 放置、测试文件拆分由实现者在范围内决定。真实数据试用是以后的独立事项：目的、来源范围、路径验证和停止条件尚未确定；当前合成工程验收不以导入真实 Telegram、20 条真人样本或正式 VAL-02 为前置。

## 文档修订回执（业务实施之前）

| 活动 | 实际结果 | 本机证据 |
| --- | --- | --- |
| 当前状态与依据 | Git 起点、STATUS、PLAN_INDEX、两轮审核及两条实现路径定向读取；业务无 dirty；周整理首次到期 2026-10-29，本轮未到期 | `initial-status.txt`、`initial-*.md`、`main-static-checks.log` |
| 并行取证 | 数据与任务路径分别只读核对；未构建或运行未来验收。数据调查的一次无匹配搜索退出 1，随后定位成功，均留痕 | `execution-data-*`、`execution-loop-*` |
| Word 内容与结构 | bundled Python OOXML 定点修改；最终 71 段、71 单元格修订，11 张表结构及段落样式保留，仅 document.xml 改变。首次 namespace 比较失败在写入前停止，修正比较方法后退出 0 | `revise_framework.py`、`revision-map.json`、`authoring-attempt*.log`、`structural-validation.json` |
| 渲染与视觉 | bundled renderer 缺 LibreOffice 退出 1；隐藏 Microsoft Word 16 只读导出成功，Poppler PNG 成功，最终 15 页全部目视检查。修复行对应和交叉引用，调整列宽与分页 | `reference-render-attempt*.log`、`final-render-attempt*.log`、`png-render*.log`、`visual-qa.md` |
| 独立核验 | F1–F5 与结构／范围定向复核；初始编辑失效缺口修正后无剩余具体阻塞。142 项修订逐项匹配；映射辅助脚本一次索引错误退出 1，修正后退出 0；初始发现与失败均保留 | [文档核验记录](../reviews/2026-10-22-core-framework-v1.3-revision.md)、`v1.3-independent-review*` |
| 文件与范围交付 | 原 v1.2 和开放任务暂停记录逐字节比较，章节／表格／语义锚点和本地链接核对；最终复制到 Downloads 后再次核对字节 | `validate-delivery.py`、`final-validation.json`、`delivery-checks.log` |

全部证据路径相对于本节开头的外部持久目录。没有运行业务测试、正式门禁、模型或真实数据，也未提交发布。上轮 233 项局部回归不计入本轮新能力验证。实施方案仍待上述三项决定；文档修订没有解除已有研究暂停。

## D109 实施回执（2026-10-22）

作者随后授权以上有限切片；由主线程接 Session/IPC 和权威记录，`scoped_data` 负责存储/遗忘，`scoped_ui` 负责 React/core 包装，两名实现者写路径分离，完成后安排独立只读评审。起点仍为 `codex/soul-rebuild-20261019` / `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46`，业务源码初始干净、前轮文档 dirty 保留。本轮候选未提交，不以 HEAD 代表全部改动。

持久证据目录 `E:/SoulSyntheticLab/scoped-register-implementation-20261022/`，仅本机可访问。`initial-*.md`、`initial-status.txt` 保存起点；首次代码读取误用不存在的 fakeCore 路径退出 1，实际文件为 `src/test/fakeCore.ts`，已定位，不影响代码。随后两次猜测测试/trait 文件路径读取失败，已改按 `rg --files` 和定义定位，不当作实现证据。

测试优先：`data-red.log` 初轮 7 失败因 fixture 的空 trait_axes 不满足原 schema（退出 101）；修为合法 unknown 轴后 `data-red2.log` 为 1 通过、6 失败（退出 101），五项明确命中未实现 stub、一项版本仍 7。保留两轮，不以首次错误冒充预期行为失败。全部输入为合成与临时库，未恢复暂停的研究实验、真实采集或外部模型调用；邻层普通回归包含既有研究留存存储单元测试，不是启动暂停的研究入口或对照实验。

### 实际落地与操作

1. 起草页新增“按联系人和场合本地起草”可选入口；现有档案可用时选择有效第三方人物与私人/工作/学习场合，调用本地模板。无材料选择时才可进入，端点、手选记忆、Insight、Assistance 继续各自原路径。
2. 草稿编辑留在页面内存；用户主动选择随意/平实/正式并点击“保存我的表达偏好”，审阅本人归属、精确范围、未来使用和失效语义后一次确认。仅固定确认说明入密封 Preference，编辑正文不上传、不存储。
3. 下一次同 scope 起草在新 brief 副本应用确认语域并显示 Memory/revision 来源，其他 scope 和全局值/锁不变。缺少档案明确提示先设置，不把未知补零或默认创建授权。
4. Memory 普通编辑会停用规则，编辑处有说明；Memory/CK/联系人遗忘销毁关联确认说明并清 typed 值，联系人无姓名也覆盖。重新确认遗忘过的 scope 必须新 Memory/CK。现有 Memory 只有 `#/memory` 路由，链接进入记忆页并展示 ID，不伪造单条深链。

实际源文件：`soul-store-api/scoped_preference.rs` 下移同一 VoiceRegister（profile 原路径 re-export），`soul-store/scoped_preferences.rs` 与 sql/store/forget 接 schema 8、CAS/原子确认、普通编辑失活和遗忘；`session_scoped_preferences.rs` 接一次结果/提案/确认及本地生成，普通/Memory/Insight 入口清旧状态；Tauri 五命令及注册/清单与 `core.ts` 对齐；`DraftScopedVoice.tsx` 处理生命周期。没有改冻结 Memory JSON、两个 Cargo.lock 或依赖清单。

### 实际命令、结果与失败

以下命令全部由主线程在 `E:/Project/Soul` 以 pwsh 执行，首行 Stop，原生命令退出码即时记录。日期/工具详见 `environment-and-counts.txt`：2026-10-22 Pacific，PowerShell 7.6.5、Rust/Cargo 1.83.0、Node v24.18.0、pnpm 10.15.0。表中日志相对本节外部持久目录；测试数不跨重跑累计，初始 RED/GREEN 与最后回归分开。

| 实际命令/活动 | 结果/退出码 | 原始证据与范围 |
| --- | --- | --- |
| `cargo test -p soul-store --lib scoped_register --locked` | RED1 0/7，RED2 1/7，均 101；GREEN 10/10，0 | `data-red.log`、`data-red2.log`、`data-green.log`；精确/CAS/重开、普通编辑、三个遗忘单位、quote、wrapper/故障回滚、旧库/非法人物 |
| `cargo test -p soulcore --test session_scoped_preferences --locked` | 首次编译失败 101（测试误多传遗忘时间参数）；修后 RED 0/3，101；GREEN 3/3，0 | `session-red.log`、`session-red2.log`、`session-green.log`；后续扩充见 core-regression |
| `pnpm --filter @soul/desktop test -- src/routes/DraftScopedVoice.test.tsx` | 初始 0/7，1；race RED 7/8，1；source RED 8/9，1 | `ui-red.log`、`ui-race-red.log`、`ui-source-red.log`；确有缺入口、误称当前有效、成功取消仍锁定反例 |
| `pnpm --filter @soul/desktop test -- src/routes/DraftScopedVoice.test.tsx src/routes/Draft.test.tsx` | 22/22，0 | `ui-green.log`；当时新 7 与原 Draft 15，不替代后续变更复验 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip scoped_register --locked` | RED 1/2，101（命令未注册）；GREEN 2/2，0 | `ipc-red.log`、`ipc-green.log`；真实 handler/参数序列化、保存/取消/遗忘/重启 |
| `cargo test -p soul-store --lib --test schema_version --test import_migration --test conformance_real --locked` | 94/94，0 | `store-regression.log`；83 lib＋4 conformance＋2 migration＋5 schema-version |
| `cargo test -p soulcore --test session_scoped_preferences --test session_atomic_edits --test draft_commands --test profile_memory_commands --locked` | 33/33，0 | `core-regression.log`；5 scoped＋5 atomic＋20 draft＋3 profile-memory。真实审计触发器失败后 Memory/CK/blob/audit/binding 全回滚，提案不可重放；配置 loopback 端点下 scoped 零请求 |
| `cargo test -p soul-memory -p soul-profile -p soul-draft --locked` | 145/145，0 | `adjacent-regression.log`；普通邻层回归，零用例 doc-test 不充数 |
| `cargo test -p soulcore --test session_draft_work --test session_memory_context --test session_insight_context --locked` | 25/25，0 | `session-context-regression.log`；11＋4＋10，含已有旧结果和上下文失效路径 |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again` | 136/136，0；过滤 2 | `desktop-regression.log`、`desktop-counts.txt`；含 command_surface 6、IPC 73。仅合成；不是未过滤全量/正式 G-W |
| `pnpm --filter @soul/desktop test` | 59 文件、711/711，0 | `ui-regression.log`；在最后已知 source 异常分类修复之前，后者按下一行定向复验，不将 711 转签为最终源码全量 |
| 最终 `pnpm --filter @soul/desktop test -- src/routes/DraftScopedVoice.test.tsx` | 初次 9/10，1；只修第十项断言后 10/10，0 | `ui-final-focused.log`、`ui-final-focused2.log`；初次精确文字遗漏 `Error:` 前缀，并非锁定实现失败。最终核验取消结构化拒绝→unavailable/未知提示、所有编辑锁定、无确认/成功。不是屏蔽所有 Error 的新承诺 |
| `pnpm --filter @soul/desktop lint` | 初次 1；render ref 改 phase 后 0；最终 0 | `ui-lint.log`、`ui-lint2.log`、`ui-final-lint.log`，TypeScript＋ESLint |
| `pnpm --filter @soul/desktop build`（临时 `NODE_ENV=production`，finally 恢复） | 两次生产构建均 0 | `ui-production-build.log`、`ui-final-build.log`；产物 `apps/desktop/dist`，非桌面安装包 |
| `cargo clippy -p soul-store -p soul-store-api -p soul-profile -p soul-memory -p soul-draft -p soulcore --lib --locked -- -D warnings` | 0 | `core-clippy.log`；所列生产 lib，不称全 targets Clippy |
| `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --lib --locked -- -D warnings` | 0 | `desktop-clippy.log`；独立桌面生产 lib |
| 定向 rustfmt；两 workspace `cargo fmt … --check`；`cargo run -q -p xtask --locked -- schema-freeze --check` / `e0-audit` / `denylist-audit`；`git diff --check` | 全部 0 | `format-targeted.log`、`final-static.log`；沿既有检查，不增门禁 |
| 独立源码评审及定向复审 | 完成；非命令活动无虚构退出码 | [独立评审](../reviews/2026-10-22-scoped-register-implementation.md)。清单漏项与两项 UI 异步问题修复，无剩余静态阻塞 |

主线程文件读取中还出现一次 PowerShell rg 通配路径不成立，已改为目录 `-g` 定位；文档 patch 一次因只匹配了段落前半句失败且未写入，后续用完整行修正。UI worker 的取证与一次保存后误入 token 失败见 `ui-inspection.log`；评审两次定位失败见评审记录。均保留事实，不冒充业务失败或伪造缺失的原始终端副本。最终候选 tracked diff/新增文件副本与状态保存到 `final-candidate.patch`、`final-source/`、`final-status.txt`，未提交源码由这些材料和工作区共同定位。

### 交付状态与剩余边界

上述 297 项根相关回归、136 项限定合成桌面回归、711 项当时全前端回归及最终 10 项定向验证分别成立，不把重跑数叠加成独立样本。工程实现与独立静态评审已完成；作者使用验收、真实 Windows 页面/剪贴板/窗口生命周期、安装、真人效用以及 G-L/G-W/G-M 未执行。没有 commit/push/merge、安装/卸载或正式关闭 Goal；暂停检查点与起点副本逐字节相同，既有周整理首到期日 2026-10-29 未到期。

近期可验收结果已达到本次有限切片范围，停止增加无关流程或测试。后续先由作者在合成资料中检查完整交互，再按明确授权决定真实使用与更大框架切片；全套 Claim、自动召回、其他生成路径应用和真实效果测量仍不在已完成范围。

## 后续一：核对当前偏好与定位记忆（2026-10-22）

作者继续要求“接下来按照最新计划持续推进，自行完善各个审阅编写流程”。承接 D109 已完成的保存/应用闭环，本轮交付其日常核对路径：用户明确选择人物和场合后，可以主动查询当前偏好状态，并通过来源链接直接打开对应 Memory，继续使用已有编辑与两步遗忘。此项属于同一有限产品切片的可用性补齐，不改变规则的保存、应用或遗忘语义，也不代表作者已完成上一轮人工验收。

起点仍为 `codex/soul-rebuild-20261019` / `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46`，上一轮所有业务/文档 dirty 保留。证据目录 `E:/SoulSyntheticLab/scoped-register-inspection-20261022/`；起点 diff、状态、计划和暂停记录已保存。周整理首次到期仍为 2026-10-29，未到期。

### 方案与边界

采用精确范围只读状态＋现有 Memory 路由定位。仅继续显示历史保存回执无法回答“现在是否还会用”，另建全局偏好管理系统则引入不必要的列表、筛选和写入口；本方案直接复用当前表、内容密钥、Memory 编辑和遗忘流程。

- `scoped_register_status(person_id, occasion)` 固定真实 owner，只查用户选中的范围，返回 `person_id/occasion/state/preference/memory_id/notice`。`state` 区分 `absent/active/inactive/forgotten/unavailable`；只有已验证 active 才带 register 和当前 preference receipt。缺少关联的精确查询才是 absent；读取失败不能伪装 absent，遗忘必须有明确墓碑/销毁依据，损坏或不可读不能猜成用户遗忘。
- 状态查询使用 `&self` 与同一 store guard，不调用生成、确认、失效或写审计，不消费当前结果或提案；不向界面返回确认正文或其他范围材料。
- 起草页提供主动“查看此范围当前偏好”按钮；切换人物/场合、来源变更、离页或新查询使旧响应不再显示。状态明确为本次读取快照，生成时仍按原路径重新核对。保存回执不升级为永久有效证明。
- Memory 深链只携带合法 UUID，用户点击后读取那一条已有 Memory。错误、缺失、重复或不合法参数不回退到另一条记忆；A→B→A、同组件导航、StrictMode 与晚到响应必须保持精确对象。导航不触发编辑、确认或遗忘。
- 不新增数据库迁移、依赖、hash、冻结 contract、gate、出网能力、自动召回或后台采集；研究暂停和平台后置保持。

### 分工与验证点

| owner | 路径/接口 | 必须验证的结果 |
| --- | --- | --- |
| `scoped_data` | `session_scoped_preferences.rs`、`session_scoped_preferences.rs` 集成测试；根协调后认领 | absent/active/普通编辑后inactive/遗忘/不可用准确投影；正常与拒绝读取不写库、不消费提案；不同scope不泄漏 |
| `scoped_ui` | `core.ts`、`DraftScopedVoice.tsx`、Memory/路由相关最小改动及fake/tests | 主动查询、scope/source/lifetime晚到拒收、精确Memory定位、非法导航不调用写操作、原普通页面路径保持 |
| 主线程 | Session出口、Tauri command/handler/清单及真实IPC测试，计划/状态/日志 | 三处命令声明一致；合法读取及非法参数经真实handler；只运行一份重构建；记录命令、退出码、原始输出与适用候选 |
| 独立评审 | 实际本轮diff、新接口、导航和失效边界 | 只报告具体缺陷/反例，修复后仅定向复审，不把范围外功能当阻塞 |

先准备最小编译入口和行为测试，主线程运行 RED，再实现 GREEN。相关 Session 与真实 IPC 定向检查、Memory/Scoped/路由前端回归、类型/lint/生产构建及必要格式检查由主线程统一运行。只使用合成临时库，桌面不运行两项真实采集用例；不重复执行未改动的全仓实验。

本轮流程改进落实在任务内：外部 `run-check.ps1` 每次保存命令、时间、原生退出码和输出，失败/重试分别留档；实现前固定接口与单写者；独立评审先看本轮差异再看说明；实际结果仍回写本计划，STATUS 仅留摘要。不改全局配置，不另建常驻流程系统。

### 执行回执

核对起点与必要源文件后完成方案和并行只读调查。一次猜测 `router.ts` 读取失败（实际是 `router.tsx`），未写代码，见外部 `inspection-notes.txt`。本节日志均位于上述本机持久目录；只代表本轮 dirty 候选，运行环境见 `environment.txt`。

| 实际命令/活动 | 结果与适用范围 | 日志 |
| --- | --- | --- |
| `cargo test -p soulcore --test session_scoped_preferences --locked scoped_status_` | RED：6 失败、5 filtered，退出 101；stub 将所有状态误作 absent，未正确拒绝非法来源 | `data-status-red.log` |
| `cargo test -p soulcore --test session_scoped_preferences --locked` | GREEN：11 通过、0 filtered，退出 0；新增 6 项与原有 5 项，涵盖准确状态、损坏/SQL 失败、遗忘重启及提案/审计不变 | `data-status-green.log` |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip scoped_register_status_wire --locked` | RED：1 失败、75 filtered，退出 101；实际 handler 未注册新命令 | `ipc-status-red.log` |
| 前端首次 RED（`pnpm --filter @soul/desktop test --` 后跟三个文件） | 参数转发使 Vitest 收到 literal `--`，范围扩大；观察到 Memory target 9 项、status 8 项及命令面 1 项失败后由主线程中断，exec 返回 1，无完整测试汇总/原生退出码。未在运行中修改候选，不把中断当完整回归 | `ui-status-red.log` |
| 定向 rustfmt | 退出 0，仅格式化本轮三个 Rust 文件 | `format-targeted.log` |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test ipc_roundtrip scoped_register_status_wire --locked` | GREEN：1 通过、75 filtered，退出 0；合法读/范围隔离、审计不变和提案保留、普通编辑/遗忘后状态、非法场合拒绝 | `ipc-status-green.log` |
| `pnpm --filter @soul/desktop exec vitest run`，指定新增三文件及 Memory / Memory.write / Memory.outcome / DraftScopedVoice | 首次 68 通过、2 失败，退出 1；in-flight 两测试读数期望不符。诊断确认两次均在操作前重复读 A，非提前读 B；稳定初始 hash 事件后保留严格 A→B 次序断言 | `ui-status-green.log`、`ui-navigation-diagnostic.log` |
| 同批旧按钮和派发间隙反例 | 诊断 9 通过、3 失败，退出 1；除上述两项外，旧 save 点击到 IPC microtask 间发生 source 变化仍写 A，实际得到 B、A 两次写入。增加派发前 source 复核；并核验旧地址按钮不能选入起草材料 | `ui-navigation-diagnostic.log` |
| 上述七文件复验 | 71/71 通过，退出 0；属于后续晚到写回执修正前候选 | `ui-status-green2.log` |
| `cargo clippy -p soulcore --lib --locked -- -D warnings`；独立桌面 workspace 同参数 | 两者退出 0；只覆盖生产 lib，不称所有 targets | `core-clippy.log`、`desktop-clippy.log` |
| `cargo test -p soulcore --test session_scoped_preferences --test session_atomic_edits --test draft_commands --test profile_memory_commands --locked` | 39/39 通过，退出 0；11 scoped＋5 atomic＋20 draft＋3 profile-memory | `core-regression.log` |
| `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets --locked -- --skip collection_can_be_granted_and_taken_back_over_the_ipc --skip a_restart_finds_collection_off_again` | 137 通过、2 filtered，退出 0；含 command surface 6 与 IPC 74，保留真实采集排除，不称未过滤全量/正式 G-W | `desktop-regression.log` |
| 两 workspace `cargo fmt … --check`；`cargo run -q -p xtask --locked -- schema-freeze --check` / `e0-audit` / `denylist-audit` | 全部退出 0；复用既有检查 | `root-format.log`、`desktop-format.log`、`schema-freeze.log`、`e0-audit.log`、`denylist-audit.log` |
| 晚到更新与真实 App 链接恢复 RED | 12 通过、2 失败，退出 1；晚到更新确实重新显示旧正文。链接用例首次误期望已调用 detail，实际在此前就因 source 变化拒绝，修正该时序假设后重跑 | `ui-delivery-red.log` |
| 修正后的真实 App 链接 RED | 1 失败、13 skipped，退出 1；先拒读、detail 0 次且不自动重试的断言通过，准确失败于缺少主动“重新读取此地址”按钮 | `ui-link-recovery-red.log` |
| 七文件交互复验 | 73/73 通过，退出 0；写入回执只作历史事实、额外来源变化后不重新展示旧正文，真实 App 链接失效后可主动恢复 | `ui-status-green3.log` |
| 最终只读投递间隙 RED | 1 失败、14 skipped，退出 1；控制 Promise 顺序与两次 source 通知后，旧只读正文确实复活。区分只读和写入，前者在最后投递再次比较原 source，后者保留自身通知与成功回执语义 | `ui-read-delivery-red.log` |
| `pnpm --filter @soul/desktop lint` | 首次退出 1，仅 Memory effect 内同步 setState 违反现有规则；将更新移至仍拥有请求的微任务、保留同步地址遮挡与物理 slot 后退出 0，没有关闭 lint 规则 | `ui-lint.log`、`ui-lint2.log` |
| 首次最终 `pnpm --filter @soul/desktop test` | 62 文件、743 通过/1 失败，退出 1；所有新增交互通过，唯一旧 Graph 测试仍要求 `#/memory?contact=x` 回首页，与新深链路由冲突 | `ui-regression.log` |
| `pnpm --filter @soul/desktop exec vitest run src/routes/graphTarget.test.ts src/routes/memoryTarget.test.ts` | 21/21 通过，退出 0；只更新旧测试为进入 Memory 页并由其 parser 判定 invalid，保留其余未扩展路由回首页和非法参数拒绝断言，生产源码未改 | `ui-routing-regression.log` |
| 最终 `pnpm --filter @soul/desktop test`；`pnpm --filter @soul/desktop lint` | 62 文件、744/744 通过，退出 0；最后测试预期调整后的 TypeScript/ESLint 亦退出 0。包含本轮全部最终交付修复，不将先前 71/73 项结果转签 | `ui-regression2.log`、`ui-final-lint.log` |
| `pnpm --filter @soul/desktop build`（临时 `NODE_ENV=production`，finally 恢复） | 退出 0；实际产物 `apps/desktop/dist`，不是桌面安装包 | `ui-production-build.log` |

后续前端定向命令使用 `pnpm --filter @soul/desktop exec vitest run <files>`，避免经 test 脚本转发 literal `--`。失败时 pnpm 尾部附加了 `Command "vitest" not found`，但前方实际 Vitest 测试与断言失败已完整记录；不把它误诊为未运行或依赖缺失。首次范围错误和中断原样保留。原有 Memory 页面也存在旧写入晚到覆盖当前显示的缺口，本轮因新增 source 归属和同页导航而定向补齐，不伪称全部问题由新功能首次引入。

### 实际交付与使用路径

在“按联系人和场合本地起草”中选择人物/场合，点击“查看此范围当前偏好”；页面显示尚未设置、当前可用、已停用、已遗忘或当前不可用的读取快照。只有验证为 active 才显示对应语域与 revision，起草仍走既有重新核对。查询不改变草稿、提案、全局设置或审计。

状态和已应用/已保存偏好中的来源链接使用 `#/memory?memory_id=<UUID>`，用户点击后精确打开对应 Memory；普通 `#/memory` 列表继续可用。非法参数不自动读另一条；返回 ID 不符不展示正文。已有编辑/两步遗忘入口继续使用，读链接本身不触发写入。若离页清理或来源变化使读取失效，用户可点击“重新读取此地址”，不会后台自动重试。

本轮实现位于 Session scoped 模块及真实 Tauri 命令注册，前端 `core.ts`、`DraftScopedVoice.tsx`、`Memory.tsx`、`memoryTarget.ts`、`router.tsx` 和对应测试；没有改 `App.tsx`、`memoryRequests.ts`、schema 或依赖。两个实现者按路径分工，构建由主线程统一执行，[独立评审的发现与修复](../reviews/2026-10-22-scoped-register-implementation.md#后续一当前偏好核对与精确记忆导航2026-10-22)已归档。

本轮工程实现、独立静态评审和上述运行验证已完成：最终根相关 39、限定合成桌面 137（2 项真实采集 filtered）、全前端 744 项通过，所列 lint/Clippy/格式/既有审计及生产构建均通过。阶段状态为已实现、已评审、限定范围已验证；仍为 dirty 源码，不标正式 GATED/INTEGRATED。先前失败、中断、错误假设和不同候选的通过记录全部保留，不叠加重复运行的数量。

候选证据：`final-source/` 与 `final-source-paths.txt` 保存 32 个本次相关源码/测试文件，`source-snapshot-check2.log` 逐字节核对当前候选；`final-candidate.patch`、`final-status.txt` 保存 tracked diff 和未提交状态。文档本地链接、依赖/暂停边界与最后 diff 检查另有日志。UI 调查与交接在 `ui-investigation.log`、`ui-handoff.md`，数据交接在 `data-task.md`。这些原始材料保存在本机外部目录，不暗示已随 Git 发布。

当前交付仅覆盖本机源码与合成工程验证。真实 Windows 手工页面、剪贴板、窗口生命周期、安装、真人效用、G-L/G-W/G-M 和正式 Goal 关闭不在本次已验证范围；未自动 commit/push/merge。研究暂停文件与起点逐字节一致，依赖/锁文件未变，见 `boundary-check.log`。周整理首次到期仍为 2026-10-29，没有提前声称完成整轮整理。

## 后续二：Windows 原生窗口合成交互（2026-10-22，进行中）

作者继续明确要求补齐 Windows 端合成数据交互验收，发现问题自行修复并安排独立复审。承接上一片源码，本轮运行真实 Windows/Tauri/WebView 窗口，验证向导与合成导入、全局/精确范围偏好、确认与取消、后续应用、Memory 定位/编辑停用/遗忘、复制及关闭/恢复/重启。只使用本轮新建合成数据目录和独立 WebView 缓存，不触发真实采集，不配置外部模型端点，不安装/卸载。

起点为 `codex/soul-rebuild-20261019` / `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46` 加既有 dirty 候选。开工时 `open-task-flow.md` 已有额外 dirty 修改，以本轮起点副本为准保留，不还原。外部持久证据目录 `E:/SoulSyntheticLab/scoped-register-windows-20261023/`；目录名使用本批次标识，实际命令日期/时区以日志为准。起点状态/diff/计划/暂停材料均已保存。

主线程负责唯一构建与真实窗口操作；`windows_launch` 只读核对启动隔离和单实例边界，`windows_scenarios` 只读梳理具体交互和合成材料；必要修复分配唯一写者，完成后安排独立只读复审。现有 Computer Use 能力已初始化，首次观察没有 Soul 窗口或进程，桌面未锁。实际启动前再次检查，不结束未知用户实例。已有 release 产物日期早于候选，必须重新构建并使用 custom-protocol 嵌入当前生产前端。

使用既有合成 `fixtures/import/soul-import-v1/valid_basic.jsonl` 或由其明确生成的合成材料，任何复制观察仅涉及本轮合成文本。截图/可访问性树只抓取指定测试窗口；不抓取其他应用正文。UI 原生行为、源码/单元测试、未观察项分别记录，不勾选作者人工清单、不把本轮有限交互签为完整 G-M/G-W。

### 执行记录

开工取证和工具结果见 `inspection.log`、`prelaunch-processes.log`。自动化技能索引文件缺失的一次读取退出 1；已使用可用的 Computer Use 文档和 `@oai/sky`，未安装新工具或修改全局配置。构建、操作、失败与复验结果在本节按实际追加。

| 实际命令/活动 | 结果与边界 | 本轮外部证据 |
| --- | --- | --- |
| `pnpm --filter @soul/desktop build`，临时 `NODE_ENV=production` | 2026-10-22 22:29（America/Los_Angeles）退出 0，63 modules；产物为当前 `apps/desktop/dist` | `production-frontend.log` |
| `cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml --release --features custom-protocol --bin soul --locked`，临时 `CARGO_BUILD_JOBS=2` | 22:29:25–23:00:56，约 31 分 30 秒，退出 0；包含首次 OpenSSL 编译和 soulcore 优化，当前 release/soul.exe 嵌入本轮生产前端 | `native-release-build.log`、`build-progress.jsonl` |

启动配方保存在外部 `launch-synthetic.ps1`，每次重新传入同一合成库和 WebView 目录；`inspect-owned-process.ps1` 只检查该次 PID/启动时间及后代 WebView 的实际缓存路径。路径声明不替代启动后的核验。原生操作、截图及每步时间在 `ui/` 留存。两项调查的路径假设失败、续读技能路径失败和进程查询的非零结果，均在 `investigation-and-scope.txt` 保留；没有据这些检查签发产品失败或通过。

首次启动 PID 31316 的 JSON 和隔离核对成功，但重定向 GUI 输出的外层脚本没有结束回执，随后 UI helper 超时且进程消失，退出原因/退出码未知；此实例没有执行 UI 输入，不诊断为产品崩溃。第二次去掉输出重定向后直接启动成功，PID 948，6 个已观察 WebView 后代的路径均为精确的合成 WebView 根目录下 `EBWebView`。检查器原先误要求根目录本身，纠正为仅接受根目录或精确 `EBWebView` 子目录，独立评审确认仍保留 PID/可执行路径/启动时间及后代边界。原失败和脚本副本保留。

已观察初始向导采集关闭、无模型端点、授权目录为 0；期间工具报告用户输入介入，重新观察时向导已进入未回答的问题页，因此不将此前确认动作算作代理成功执行。随后从 0 回答跳过 11 题进入概览，页面仍显示未采集。全局语域明确设为 Formal，显示用户锁。经原生文件选择器仅选入本轮 `input/valid_basic.jsonl`，预览 5 条新消息，确认后显示 committed / recorded、5 事件 / 5 证据、2 关系重建。人物 X 私人范围初读 absent，草稿以“您好”开头。保存/取消、范围隔离、编辑/遗忘及重启验证继续进行，尚未签发整个矩阵完成。

### 停止时的实际交互回执

上述“继续进行”为该时点进展；随后作者询问后台控制方式并通过实体 Escape 停止 Computer Use。停止后没有继续任何窗口输入，没有观察到应用退出，不把停止自动化当成测试实例已关闭。实际完成范围如下，非命令 UI 活动不虚构进程退出码。

| 场景 | 实际结果 | `ui/` 证据 |
| --- | --- | --- |
| 全局 Formal 与用户锁 | 页面显示正式且“你自己设过，推断不再改这项”；022 时点仍平实，023 才是已生效快照 | `023-import-open.json`（标签虽为 import，快照仍在 profile） |
| 取消偏好保存 | 明确提案后取消，再读仍 absent | `054-preference-proposal-cancel-test.json`、`056-preference-cancelled.json`、`058-after-cancel-status-absent.json` |
| 保存后读与下一次应用 | Memory `01a16776-9e70-7981-b305-fba0449a441b`，revision `01a16776-9e72-7b43-af7c-fe549c10b841`；保存、active 读取与下一草稿 applied 匹配，草稿以“嗨”开头 | `062-preference-saved-first.json`、`063-saved-status-active.json`、`064-next-draft-applies-casual.json` |
| 同人物不同场合 | X / work 显式确认切换后清除旧显示，读 absent，草稿“您好”且无 applied | `067-occasion-work-committed.json`、`068-x-work-absent.json`、`069-x-work-formal-draft.json`；066 只是弹层突出选择，不算已切换 |
| 不同人物同场合 | Y / personal 读 absent，草稿“您好”且无 applied | `075-person-y-committed.json`、`076-y-personal-absent.json`、`077-y-personal-formal-draft.json` |
| 原生复制 | 已先用合成原文覆盖剪贴板；编辑为唯一合成标记并点击复制，出现“已复制到剪贴板” | `046/047` 合成种子、`079-clipboard-edit-current-draft.json`、`080-clipboard-copy-button.json`；粘贴核对未执行，不能签完整剪贴板通过 |

Memory 精确链接的原生点击、普通编辑停用、遗忘确认/取消、托盘恢复、真正退出与重启持久化均未完成。此前 Session/真实 IPC/jsdom 通过记录保留，但不能替代这些原生观察。当前这批没有确认业务源码缺陷或实施业务修复。启动脚本/路径检查器改动属于验收工具修正，首次未知退出保留。

`scoped_review` 已独立核对上述初段至 X/work；没有最终签发整个矩阵。其指出 `ui/actions.jsonl` 有 166 个前导 NUL，不能当合法 JSONL 整体解析，原件保留，不静默覆盖；逐步 JSON 和 PNG 独立保存。评审者一次误读根目录 actions 文件退出 1，随后定位正确 `ui/actions.jsonl`，详见外部 `investigation-and-scope.txt` 与[独立记录](../reviews/2026-10-22-scoped-register-implementation.md#后续二windows-合成交互初段证据核对)。

## 后续三：本地 API 化可行性与建议

作者在询问后台控制后提出“能否支持将后端 api 化”。本节为当前 dirty 候选的静态可行性核对与建议，尚未实现、开放端口或新增执行授权；以减少重复 UI 自动化为近期目标，继续只用合成数据。不开启已暂停的研究/模型实验，不将未完成的 Windows 原生观察改记通过。

本轮分支/HEAD 同上。调查日期以 `E:/SoulSyntheticLab/local-api-feasibility-20261023/` 的带时区日志为准；主线程检查当前 Git 状态、STATUS/PLAN_INDEX/PRODUCT_LOCK/SECURITY，两个 gpt-6.1-sol/high 只读子 Agent 分别检查核心适配和权限/进程边界。原始安全摘录及失败保存在 `api-core-investigation.log`、`api-boundary-investigation.log`，主线程范围与文档核对在 `investigation.txt`。没有读真实库/密钥、运行测试/构建或调用 Computer Use。

### 当前事实与关键缺口

| 当前源码事实 | 对 API 化的影响 |
| --- | --- |
| `soulcore` 是无 Tauri 依赖的 Rust library；桌面 `commands.rs:61` 的 SessionState 持有 `Arc<Mutex<Session>>`，assistance/notification runtime 已共享它 | 不需要重写算法与存储；可给同一运行中核心增加传输适配器 |
| 导入/E1 等已有锁内 begin、锁外 execute、锁内 complete 的 owned worker 路径 | HTTP 不能持锁等待长任务，也不能把请求断连当作业务取消；仍须完成结算/审计 |
| Session::open 取得规范化路径对应 OS lease 后再打开 SQLCipher；原始 SqlCipherStore::open 不强制此 lease | 首批优先复用桌面的同一个 Session。后续 headless 要遵循同一所有权入口，不另开第二个 Session 争用运行中的库；lease 不代表抵御任意恶意进程 |
| `core.ts` 的 source invalidation 是单 WebView 内本地通知，并明确不覆盖 direct native callers；MemoryRequests 是前端页面请求跟踪 | 后台 API 写入目前不会自动通知 UI。新增入口必须接入跨入口变更/忙碌通知，失效旧草稿、旧 Memory 读结果与提案，不能只完成 HTTP 返回 |
| 仓库已有 `tests/val01_evaluation/operator/http.rs` 的有限 HTTP 测试操作器 | 可参考其反例与测试结构；它明确不是产品 IPC，不能把它当作已完成的通用 API 服务 |

### 方案取舍

1. **推荐首片：同进程本地 HTTP/JSON API。** 桌面显式开启受限自动化模式，HTTP 与 Tauri 共享运行时和同一 Session。脚本可直接驱动正在运行的合成实例；现有前端继续用 Tauri，不强制先改为 HTTP。传输层只负责鉴权、typed 参数、限额和结果映射，业务仍在 core。
2. **后续：独立 headless host。** 复用相同业务入口和 API 适配器，无须 WebView 即可做后台场景；显式配置合成 data-dir，沿用开库/lease/退出结算。对同一库，只由一个 host 持有 Session；不采用桌面与独立服务同时各自开库的默认部署。
3. **远程或多用户 API。** 会引入新的身份、传输与数据共享范围，不属于当前本机自动化切片；本节不授予公网/局域网监听或真实数据访问。

首片接口按现有业务动作组织，不直接暴露 SQL、任意文件读取或通用 `invoke(command,args)`。以下为候选 HTTP 形状，不是已存在接口：

| 第一片能力 | 建议接口形状 | 必须复用的语义 |
| --- | --- | --- |
| 状态、人物和 Memory 查询 | `GET /v1/status`、`GET /v1/people`、`GET /v1/memories/{id}` | 固定 Session owner、有限结果、精确 ID，不在 URL 中放正文或凭据 |
| 范围状态及本地草稿 | `POST /v1/scoped-register/status`、`POST /v1/drafts/local` | 精确人物/场合、五态、生成时重验与实际来源回执 |
| 合成导入 | `POST /v1/imports/prepare`、`POST /v1/imports/confirm`，及已有 operation 结果读取 | 文件仅限批准的合成输入根，预览/修订/提交结果复用原核心；断连不盲目重提 |
| 范围偏好写入 | `POST /v1/scoped-register/proposals`、`…/confirm`、`…/cancel` | 同一次核心提案、显式确认、一次消费；不从草稿编辑推断选择 |
| Memory 编辑和合成遗忘 | `PATCH /v1/memories/{id}`、`POST /v1/forget/previews`、`…/confirm` | 普通编辑停用、影响面重验、逻辑提交/清理/审计分开报告；仅授权的合成自动化模式开放写入 |

### 必须落到实现里的边界

- 入口默认关闭，显式启动时先校验本轮绝对合成数据目录和合成输入根；缺省不得回落真实平台目录，请求不能切换数据库。初片不提供真实采集、目录授权、外部模型配置/调用或研究明文保存接口。
- `Session::open` 会以不可用 store 状态返回对象，不能把拿到 Session 或端口监听成功当作开库成功。启动及健康状态必须检查 `status().store_opened`，对外给出不含底层敏感细节的不可用原因。
- 监听仅数字 `127.0.0.1`，不使用通配地址；每次运行生成密码学随机鉴权凭据，由受控启动通道交给已授权驱动，不放 URL/日志。严格 Host/Origin 检查、不开放通配 CORS；loopback 本身不等于调用授权。同账户可读文件/DPAPI 不宣传成能隔离同账户恶意进程。
- API 凭据只授予列出的能力，不等于用户已经阅读每个提案。当前合成场景可按用户明确批准的脚本执行两阶段动作；将来真实敏感操作仍须保留独立的人在回路确认。调用者不能直接铸造核心 capability。
- 所有入口共用业务执行/写入协调。当前 held preview 是单槽，UI 与 API 不能各自认为独占；首片由服务端明确区分桌面 UI 与一个已认证自动化驱动主体，准备、确认、取消都核对同一主体归属，不接受请求自报 caller_id 作为身份。若将来支持多个独立驱动，必须由不同凭据或已认证独立会话区分。适配层拒绝消费另一主体提案，同时保留核心对过期/替换提案的拒绝；既有单槽业务语义不擅自改为多槽。
- API 写入与 Tauri 写入均经过共同变更协调，前端在请求派发和结果投递时复核；重连先失效本地快照再主动读当前状态。现有 JS 自身写入已有两次通知并影响回执处理，不能简单叠加 Rust 事件造成双重失效；需区分本次操作与其他来源变更。Memory 列表、全局 config/status 也须明确刷新路径，单独把事件接到 invalidateDraftSource 不够。API 成功后 UI 更新属于验收内容，不能仅凭共享 mutex 推断成立。
- 请求大小、结果分页、队列/并发和超时有界；沿用现有导入限制和业务类型，不增加第二套算法常量。HTTP 断连、超时、主动取消、已提交但清理失败分别报告；日志仅安全元数据，不含正文、凭据或底层敏感错误细节。

### 近期实施与验收安排（尚未执行）

先接显式合成启动、鉴权/拒绝路径和状态/本地草稿只读入口，再接已有两阶段写入及跨入口通知，最后用同一套外部脚本跑导入→偏好确认→后续应用→编辑停用→遗忘→真正进程退出/重启。脚本通过真实 HTTP 操作真实合成 Session，禁止直接改库制造结果。

验收重点：无凭据/错 Host/错 Origin/根外文件拒绝且无业务副作用；人物/场合/owner 不串用；跨调用者不能消费别人的提案；断连后的真实操作结果可确认、不误报回滚；API 写入使在途 UI 旧结果失效；同库第二 host 被现有所有权机制拒绝；退出重启后持久状态正确。涉及授权和数据正确性的改动由独立只读评审检查实际 diff，主线程统一运行受影响测试与真实 API 脚本。剪贴板、文件选择器与托盘仍保留极少量原生交互观察，不拿 API 代替这些平台证据。

本节结论等级：现有分层与缺口为已读源码事实；API 实施可行性为基于这些事实的设计判断；接口路径、运行时通知与 headless 形态为建议，未经实现或运行验证。不添加新 hash、冻结 contract、baseline 或正式 gate，不更改既有算法、迁移、遗忘或授权协议。

方案独立核对：`api_boundary_review` 只读审查本节，未发现事实冲突或误称实现，指出调用者身份必须由服务端认证绑定、不能接受请求自报 caller_id；已按上文明确首片两个服务端主体及准备/确认/取消归属。源码定位调查与方案核对的命令、退出码及两次修正路径查找均在其外部日志。主线程 `git diff --check` 退出 0（保留已有 PRODUCT_LOCK 换行提示），32 个前轮相关源码/测试与 Windows 开始副本逐字节一致，见 `source-unchanged.json`。仅文档变更，不重复业务测试或冒称新的 API 运行结果。
