MODEL: claude-fable-5-thinking-xhigh

# Round 1 · Soul SOTA 差距与下一步审计

审计对象：`origin/cursor/soul-product-lock-7b1c`（产品锁）与 `origin/cursor/soul-goal1-7b1c`（Goal 1 实现分支，HEAD `862e858`）。
证据引用一律写远程路径。本报告不含实现代码。

## 一、SOTA 对照表

对照对象：Rewind/Limitless（连续屏幕/音频记录）、Windows Recall（系统级快照回忆）、Pi/Inflection（人格化陪伴）、Memex 类个人历史检索、本地 RAG 助手（Khoj/Reor/AnythingLLM 一类）、computer-use 桌面代理（Copilot/Open Interpreter 一类）。

| 能力 | SOTA 现状 | Soul v0.1 现状 | 判定 |
|---|---|---|---|
| 隐私默认姿态 | Recall 遭反弹后改 opt-in + Hello 门；Rewind 默认全录 | 全部默认关，未同意事件数恒 0，且是 CI 断言不是文案（`origin/cursor/soul-goal1-7b1c:crates/soul-collect/tests/consent_gate.rs`；壳侧向导「不能产出开着的快照」，`crates/soulcore/tests/shell_commands.rs`） | **领先**，且领先在「可证明」 |
| 静态加密 | Recall 用 VBS enclave + Hello；Rewind 历史上近乎明文 SQLite | SQLCipher 整库 + 字段级 XChaCha20-Poly1305（AAD=行id\|字段名）+ 每遗忘单元 CK，盘上字节搜不到明文（`crates/soul-store/tests/no_plaintext_at_rest.rs`）。**但 Windows KEK 保护是骨架**：`DpapiKeyProvider` 两个入口返回 `Unsupported`（`docs/SECURITY.md` 加密落地节） | 设计领先，**Windows 落地未完**——此刻真机姿态弱于 Recall |
| 遗忘语义 | SOTA 最多做「删时间段/删应用」的行删除 | 密码学销毁（销毁 CK）+ 影响面真查询预览 + 派生推断降 orphaned + 中途崩溃不留半毁态（`crates/soul-store/tests/forget.rs`、`tests/crash_recovery.rs`） | **明显领先**，业界没有对标物 |
| 第三人/旁观者保护 | 全体 SOTA 记录一切旁人，无占位概念 | redactor 默认占位 + ≥8 字连续子串泄漏测试 + 研究预览第三人行数恒 0 且构造级强制（`crates/soul-policy/tests/redactor_leakage.rs`、`crates/soul-store/tests/research_preview.rs`） | **领先**，是差异化卖点 |
| 人格建模 | Pi 有暖人格但那是助手自己的人格；roleplay 产品是虚构角色 | 建模的是**用户本人**：五条方向轴、无数字分数、推断必带可解引用证据、用户纠正锁定优先（`crates/soul-profile/tests/correction_lock.rs`、`tests/axes_and_evidence.rs`） | 可检视性领先；**丰度落后**——目前只有问卷 + 导入喂它，没有持续学习回路 |
| 采集密度 | Rewind/Recall 连续截屏 + OCR + 音频，信号量大几个数量级 | 仅前台应用时长，窗口标题被三层测试禁死（`crates/soul-collect/tests/window_titles_are_not_collected.rs`） | **落后（有意为之）**。「复刻电子版的你」目前只靠 Telegram 导出 + 问卷 + 应用时长，灵魂保真度是最大的产品性差距 |
| 记忆检索 | 本地 RAG 助手有向量检索、语义问答 | 记忆是 CRUD + 摘要视图，没有 embedding、没有语义查询（`crates/soul-memory/`） | **落后**。「懂得你的几乎一切」在 v0.1 无法被问出来 |
| 人脉图 | 主流无本地对标（Clay/Dex 是云 CRM） | 自我中心图，边=计数强度/形状/最近接触/证据，第三人 local_only（`crates/soul-graph/tests/ego_graph.rs`） | 隐私与证据链领先；**数据源单薄**（仅 Telegram 一种真实来源） |
| 代理执行能力 | computer-use 代理能点能写能发 | v0.1 只起草不发送、文件只读计划；HITL 有 plan_hash / 一次性令牌 / 未知动作拒绝（`crates/soul-policy/tests/hitl.rs`） | 能力落后、**安全机制领先**。且起草本体（WP10）尚未实现，此刻连「落后的能力」都还没有 |
| 注入防御 | 行业刚开始重视，多数产品无系统性测试 | 导入行/粘贴/文件名三路进 `UntrustedText`，不能授权、不能外连（`crates/soul-policy/tests/injection.rs`、`crates/soul-import/tests/injection_is_data.rs`） | **领先** |
| 审计 | 几乎无人做无正文哈希链审计 | 独立链、无正文、崩溃后可验、不挡遗忘（`crates/soul-policy/tests/audit_chain.rs`、`audit_crash.rs`） | **领先** |
| 本地模型 | LM Studio/Ollama 自带本地推理 | 用户自带 OpenAI 兼容端点；无 key 走确定性语气模板。loopback 属 L 类，指向本机 Ollama 可行 | 持平偏落后，v0.1 可接受 |
| 可安装成品 | 全体 SOTA 有安装器 | `tauri build` 从未在任何 runner 上跑过；MSI/NSIS、WebView2、UAC、托盘全在作者手动清单（`docs/STATUS.md` WP09 Windows 手动缺口 1–7） | **落后**，是 Goal 1 关门项（WP13 + AC-01） |

**一句话结论**：Soul 锁对的是全体 SOTA 都没做的「可证明的隐私/遗忘/证据/审计」四件套，这是真实差异化；缺的是 SOTA 用来堆体验的三件事——采集密度、语义检索、代理能力——其中前两件是产品锁有意推迟的，第三件（起草 WP10）是 Goal 1 内的欠账，必须现在还。

## 二、Goal 1 关门差距

对照 `docs/PRODUCT_LOCK.md` 13 条垂直切片与 `docs/FORMAL_WORK_PROMPT.md` AC 矩阵，已完成 WP01–WP08 + WP09 第一段（STATUS 均有逐条证据）。关门还差：

### 未开工的 WP

| 差距 | 对应切片/AC | 现状证据 |
|---|---|---|
| **WP10 起草 + 人事分析摘要** | 切片 4（语气立即改变）、6、8；AC-07 起草侧、AC-11/12/13/16/17 | `soul-draft` crate 不存在（`git ls-tree origin/cursor/soul-goal1-7b1c crates/` 无此目录）；壳的起草路由是 `Pending` 占位（`apps/desktop/src/components/Pending.tsx`）。redactor/E1/mock LLM 地基全部就绪（WP08 + `soul-testkit/src/mock_llm.rs`），`read_voice` 用户值已可读（WP03） |
| **WP11 只读文件计划** | 切片 9；AC-18 | `soul-fileplan` crate 不存在。D31 明确留在 Goal 1（`docs/DECISIONS.md`）。HITL 侧「写文件令牌不签发不消费」已就绪（WP08 交付 1） |
| **WP09 第二段：功能视图** | 切片 1/3/5 的 UI 面 | 起草/文件计划/导入/记忆/人脉路由全空；壳握的是内存 `Config`，**没接 `SqlCipherStore`**（STATUS「WP09 取舍 8」） |
| **WP13 收尾** | 切片 13；AC-01/21/26 | `tauri build` 从未跑过、无安装 smoke、无 SBOM（STATUS「WP09 Windows 手动缺口 4」） |

### 已知遗留（不是新 WP，但不清掉关不了门）

1. **DPAPI 骨架**（WP02 取舍 3 / SECURITY 加密落地节）：Windows 真机 KEK 无保护。AC-01「干净 Win11 安装」在密钥无保护时通过是名不副实的；SECURITY 明文写「补齐前不得声称 Windows 上 KEK 已受保护」。障碍是 `soul-store` 的 `#![forbid(unsafe_code)]` 与 Win32 绑定冲突——WP07 已有先例（unsafe 圈禁在 `soul-collect/src/windows.rs` 单文件）。
2. **双问卷题号**（WP06 与 WP03 接缝节）：`soul-import` 八题（`voice.directness` 形）与 `soul-profile` 七题（`q.axis.curiosity` 形）并存互不引用，v0.1 收尾前不并，用户会被问两遍。接口已备好：`soul-profile` 实现 `UserStatedSink` 即可，`soul-import` 一行不用改。
3. **壳↔store 单句柄接线**（WP07 遗留 8 + WP09 取舍 8）：整个进程只能有一个 store 句柄，须在 `lib.rs` setup 里开一次、`manage` 起来。这是 WP09 功能视图和 WP10 UI 的共同前置。
4. **预览-遗忘之间无钉住令牌**（WP04 取舍 5）：UI 接遗忘时两次调用之间数字可变。轻微，但 WP09 功能视图落遗忘按钮时要么加短期令牌要么在 UI 如实提示。
5. **导入非事务 + 事件不去重**（WP06 取舍 6/7）：重跑同文件事件翻倍。功能视图暴露导入按钮前要在 UI 承认这一点或加确认。
6. **迁移策略未定**（WP02 取舍 8）：`schema_version=1` 无迁移器。WP10/WP11 若加表列，先按「开发期删库重来」明示成决定。
7. **作者手动清单**（WP09 Windows 手动缺口 1–7）：托盘、UAC、进程名、WebView2、DPI、云开关网络列。CI 补不了，关门前作者必须过一遍。

**AC 完成度粗账**：26 条 AC 中，AC-02/03/04/05/06/08/09/10/14/15/19/20/22/23/24/25 已有 CI 级证据；AC-07 档案侧已证、起草侧待 WP10；AC-11/12/13/16/17 待 WP10；AC-18 待 WP11；AC-01/21/26 待 WP13 + 作者手动。即约 16.5/26 关闭，剩余全部集中在三个未开工 WP 加收尾。

## 三、推荐下一步（P0 / P1 / P2）

### P0 —— 立即做（本批派工）

**P0-1 WP10：起草（不发送）+ 人事分析摘要**
- 为什么：这是产品核心价值主张（作者原意 4/5「起草回复、分析人与事」）在 Goal 1 里唯一的体现；一次关掉 5 条 AC（11/12/13/16/17）加 AC-07 起草侧；也是 SOTA 对照里「代理能力为零」这个最难看的格子的答案。
- 依赖：全部就绪——WP02 store、WP08 permit/redactor、WP03 `read_voice`、testkit mock LLM。按 `docs/GOAL1_PLAN.md` 批 5 本来就轮到它。
- 验收信号：mock LLM 下起草只打精确 origin、跨 origin 302 拒绝；默认请求体过 ≥8 字泄漏检查；豁免仅当次；无 key 时模板降级且非回环连接 0；摘要每条带证据、无诊断词。
- 风险：无 key 模板的质量是否过验收（PRODUCT_LOCK ASSUMPTION 明示不确定）；人事摘要容易滑向诊断语，denylist 测试要先行。

**P0-2 WP11：只读目录扫描与计划预览**
- 为什么：切片 9 是代理层「只读安全路径」的唯一证明，D31 已否决过删除它的提议；AC-18 无它不关。与 WP10 互不依赖，DAG 允许并行（批 5 两人）。
- 依赖：WP08（HITL、UntrustedText 文件名注入）已完成。
- 验收信号：授权 A 扫描出计划预览且 A 磁盘字节不变；B 100% 拒绝；`soul-fileplan` 源码级无写 API（红线明写在 GOAL1_PLAN）；文件名注入串不能变指令。
- 风险：最大风险是实现者顺手写「执行」——工作单必须显式引用红线「`soul-fileplan` 无写 API」，并要求像 `research_preview.rs` 那样加源码自查测试。

**P0-3 壳接真 `SqlCipherStore`（WP09 第二段的第一步）**
- 为什么：它同时是 WP09 功能视图和 WP10 起草 UI 的前置；不接线，批 6 全堵。
- 依赖：WP07 遗留 8 的单句柄纪律（`Arc<Mutex<SqlCipherStore>>`，setup 开一次）。
- 验收信号：壳启动即开库、三条已有 IPC 命令读的是真库快照；`ipc_roundtrip.rs` 扩展后仍绿；进程内只有一个 store 句柄（可加断言）。
- 风险：Windows 上 DPAPI 未落地时开库用什么 KeyProvider——见 P1-1，接线时先走 `TestKeyProvider` 并在 UI/文档如实标注，不要假装受保护。

### P1 —— Goal 1 关门项

**P1-1 补齐 `DpapiKeyProvider`**
- 为什么：SECURITY 明文承诺「补齐前不得声称 Windows KEK 受保护」；不补，AC-01 的「干净 Win11」通过是空的，SOTA 对照里唯一被 Recall 反超的格子就是它。
- 依赖：决定 unsafe 圈禁方式（仿 WP07：单独 windows 文件或单独小 crate，其余仍 forbid）。属 WP02 遗留收口，不算新增 WP，不违反「只减不增」。
- 验收信号：Windows CI 上 KEK 经 DPAPI 包裹往返；Linux 路径不变仍走 `TestKeyProvider`；SECURITY 加密落地节同步更新。
- 风险：windows-latest 上 DPAPI 可测性（CI 机器有用户配置文件，通常可测）；unsafe 审查成本。

**P1-2 WP09 第二段：功能视图（档案/记忆/人脉/导入/遗忘）**
- 为什么：切片 1/3/5 的 UI 面；空路由测试（`App.test.tsx` 钉住占位形状）就是给它撞的。
- 依赖：P0-3 接线完成；WP04 取舍 5（遗忘预览令牌）在此决断。
- 验收信号：向导→问卷→档案→纠正锁定→图→记忆→遗忘全流程在 UI 走通；界面仍无业务逻辑（`command_surface.rs` 的「wrapper 单语句」约束保持绿）。
- 风险：UI 面最容易夹带业务判断，三道锁（eslint / contract.test / command_surface）必须保持。

**P1-3 并掉双问卷题号**
- 为什么：不并，用户被问两遍，切片 2 的体验直接破相。
- 依赖：`UserStatedSink` 接口已就位，`soul-profile` 实现即可；归入 WP09 功能视图或 10 行级小修单。
- 验收信号：一套题号、一次问卷同时产出事件+证据+档案轴；两边测试都绿。
- 风险：低。注意证据不要写两份。

**P1-4 WP13：安装 smoke、SBOM、CI 收口 + 作者手动清单**
- 为什么：AC-01/21/26 与切片 13；`tauri build` 一次没跑过是发布风险的最大未知数。
- 依赖：WP09/10/11 全落地后收口。
- 验收信号：Windows CI 产出安装包并静默安装启动（托盘目视仍归作者）；SBOM 生成；e0-audit / deny 全绿；作者过完 7 条手动清单。
- 风险：打包要下 WiX/NSIS——这是 CI 基建出网不是产品 E0，需在工作单里预先说清，防止实现者为过 e0-audit 而绕路。

### P2 —— 明确推迟（v0.1.1 / Goal 2 / 更晚）

| 项 | 去向 | 理由 |
|---|---|---|
| 文件整理执行与撤销（AC-27） | v0.1.1 | PRODUCT_LOCK 砍/留表钉死；D31 只留只读预览 |
| 目录文件元数据采集 | v0.1.1 | 同表 |
| 语义检索 / embedding 层 | v0.1.1+ 议题 | SOTA 差距真实存在，但产品锁没有它；先在 PRODUCT_LOCK 后期路线里立项再动 |
| 按天分内容密钥（「忘掉昨天」） | 存储边界改动，v0.1.1 | WP07 取舍 1 已写明超出当前边界 |
| 导入事件去重索引 / 导入事务化 | v0.1.1 | WP06 取舍 6/7；需要 `soul-store-api` 边界改动 |
| 研究导出落盘、浅层行为预测、OAuth、更多导入 | v0.2 | 砍/留表 |
| Android 采集 | v0.3 | 锁定 |
| 云端深度分析 | v0.4 | 锁定；v0.1 连代码路径都不许有 |
| Goal 2 二十轮打磨 | Goal 1 关闭后 | FORMAL_WORK_PROMPT 与 STATUS 反复钉死 |

## 四、明确不要做

1. **不启动 Goal 2**。STATUS 当前里程碑与 GOAL1_PLAN 首段都写了「Goal 1 关闭前不要启动」。
2. **不做文件写执行**——即使 WP11 做完发现「顺手能执行」。红线：`soul-fileplan` 无写 API；HITL 不消费写文件令牌（WP08 交付 1 已有测试钉住）。
3. **不加任何云端/E0 路径**，包括「为了以后方便」的 feature flag 或占位 HTTP 代码。E0 必须继续是「无代码路径」而非「有开关但关着」。
4. **不做 OAuth、不做微信/QQ 非官方抓取、不做通用 ZIP 嗅探**（砍/留表 + 不可协商约束 3）。
5. **不采窗口标题、不截屏、不自动发送**——`window_titles_are_not_collected.rs` 的三层测试就是禁令本身，别绕。
6. **不给特质轴加数字分数、不出诊断词**（D22；denylist 已抓到过一次真的，见 WP03 取舍 3）。
7. **不为 SOTA 差距抢跑采集扩面或 RAG**——那是用 Rewind 的路线稀释 Soul 的差异化；先关 Goal 1。
8. **不建第二份产品文档**；改方向只能改 PRODUCT_LOCK。

## 五、给父调度器的 5 条决策建议

1. **立即按 GOAL1_PLAN 批 5 派工**：WP10（opus，数据/E1 面）+ WP11（opus，权限面）并行，两个工作单显式引用红线「不发送」「`soul-fileplan` 无写 API」，并要求仿 `research_preview.rs` 加源码级自查测试。这是当前关键路径，无任何阻塞。
2. **把「壳接 SqlCipherStore」从 WP09 第二段里拆成先行小单**，在批 5 期间由第三人或批 6 首位完成，否则批 6（WP09 功能视图 + WP13）会整体等接线。工作单里写死 WP07 遗留 8 的单句柄纪律。
3. **裁决 DPAPI 补齐的归属与 unsafe 圈禁方式**：建议按「WP02 遗留收口」处理（不违反工作包只减不增），unsafe 仿 WP07 圈禁在单独 windows 文件/子模块，Windows CI 加 DPAPI 往返测试，同一 PR 更新 SECURITY。这一条不做完不要宣布 AC-01 通过。
4. **裁决两个小口子的时机**：双问卷题号合并（归 WP09 功能视图，用 `UserStatedSink`，`soul-import` 零改动）与遗忘预览-执行令牌（WP04 取舍 5，建议 v0.1 用「回执必须等于预览否则重新预览」的 UI 语义，不加新机制）。两条都写进批 6 工作单，防止散失。
5. **WP13 工作单里预先裁决 CI 打包出网**：`tauri build` 下载 WiX/NSIS 属 CI 基建流量，不是产品 E0；e0-audit 继续只管产品 crate。同时把作者 7 条手动清单（STATUS「WP09 的 Windows 手动缺口」）formalize 成 Goal 1 关门 checklist 的一部分——CI 全绿 ≠ Goal 1 关闭，作者真机确认才是最后一道门。
