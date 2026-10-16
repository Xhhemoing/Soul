# 单条导入消息的本地来源核对

起点 `1b41f1dcc0df8ea4929ede94daa89fbc77352148`，PR #66 草稿。按作者持续开发、自动审查和阶段同步的指示，本模块在编码前完成独立只读规划复核，状态 **PLAN REVIEWED**。本节是具体实现计划，实际实现和检查结果另记，不增加 Goal 1 关闭条件。

## 缺口与用途

图谱已显示证据元数据，洞察能定位关系，D76 也能选择可信统计；用户仍不能明确打开其中一条导入消息核对来源。本模块只服务当前 Graph 用途：先取得一条证据的事件元数据，再明确选择一条敏感消息在本机显示。只支持有精确现代导入索引的消息，不猜旧数据的归属，不从触发洞察的 namespace 推断所有事件都来自那里。

来源查看不认证旧 A2 快照仍有效，也不为生成草稿逐句背书。证据的默认 E1 策略为 Deny，导入事件为 Placeholder，二者都不授予原文出网；已有 Graph / SoulProfile 用途不变。本次不新增 Draft、Memory、研究、复制或导出入口，不把导入文本当作指令。

## 核心、存储与 IPC（PLAN REVIEWED）

两个独立纯读命令 `evidence_sources(request)` / `read_evidence_event(request)`，包装为 `evidenceSources` / `readEvidenceEvent`。嵌套请求使用 snake_case、deny_unknown_fields，不接受正文、blob、密钥、AAD、来源 namespace 或预算。外层沿用 Tauri 参数习惯，读取本身不改变 draft source revision。

| 类型 | 字段与意义 |
| --- | --- |
| EvidenceSourcesRequest | contact_id、relationship_id、evidence_id |
| EvidenceEventRequest | 同上，另有 event_id；每次仅一条 |
| EvidenceEventAvailability | available / no_body / unavailable / too_large |
| EvidenceEventSource | event_id、可选 namespace_id、既有 source、occurred_at、direction、venue、可选 body_chars、availability、notice |
| EvidenceSourcesView | 回显三个锚、既有 kind、events、notice；不含消息正文和外部自然键 |
| EvidenceEventView | 回显四个锚及真实 namespace_id、source、occurred_at、direction、venue、body、notice；只序列化输出，Debug 隐藏正文 |

每次都在同一个 store guard 内重验当前有效 self↔contact 关系、两端存活、关系包含指定 evidence、subject 和 Graph 用途。消息证据必须有完整规范 GraphInteraction 引用；坏引用、混入不透明引用、重复引用拒绝，不将它伪装成无来源。v1 最多读取 64 条引用的元数据；这是本地查看容量，不能冒充算法常量或 Graph URL 预算。已知纠正等不支持类型明确返回无聊天原文说明。

事件须是两种现有导入 source 的 ImportItem、Raw，时间、actor、privacy、direction 与 venue 一致。按 event_id 的既有 UNIQUE 索引反查真正的 ImportMessageIdentity 和 namespace，核对 SQL 列与 doc、namespace active、source、owner==self、sender、group、is_owner、内容密钥归属及存活。既有导入器不把 owner 群消息归因到任何 peer，查看器同样拒绝该组合，不能让伪造的当前关系引用为它补出归属。复用已有密封 MessageKey 解码，使用已有 conversation_ref 算法核对会话；不新增 hash、表或迁移，不返回 conversation_id / external_id。缺索引、旧数据、撤销或遗忘后只拒绝当前项，不尝试另一来源。

实现初审另发现：outgoing direct 的身份索引只证明 sender=owner，没有独立 recipient 字段，关系引用可能把 A 会话的本人发言伪挂到 B。补强方案在改码前再次独立复核（PLAN REVIEWED）：必须在当前同一关系中找到同真实 namespace / conversation 的规范 incoming peer 发言，以其真实 sender、用途、身份、CK、AAD 复验既有 actual-speaker 归属。witness 不要求 direct，不新增算法规则；只用 incoming 分支，不递归 outgoing 查找，也不打开 witness 正文。沿同一本地容量限制实际检查的 evidence 行、source_refs 和唯一候选事件，最多各 64；超限或找不到时当前 outgoing 不可查看，不把局部扫描冒称全关系证明。该核验不证明真实收件人或阅读者。

存储只增加两个固有窄方法，不扩通用读取引擎或 trait：

- `import_source_for_event(event_id, max_identity_plaintext_bytes)`：单事件反查，返回精确 message identity 与 namespace。message identity 和可选 account seal 的首次解封前，先用 SQL scalar 核对真实 row / field / CK / subject、AAD 和 ciphertext 长度，按实际 AEAD tag 扣出明文字节长度。revoked 先结构性拒绝，不为拒绝再解封。
- `event_body_plaintext_bytes(event_id, sealed)`：只读取所选 body 的真实归属和 ciphertext 长度，不加载 ciphertext；由 store 按实际 AEAD tag 得到明文字节数。

核心给 identity 的受信上限为既有 `MAX_MESSAGE_CHARS`（65536）的四倍字节，即 256 KiB；每次身份解封均先检查。允许既有 validator 和规范解码各进行一次已限额的身份解封。外部标识没有天然的小长度限制，合法大 identity 仍可导入和去重，只在本地查看时明确不可用；本模块不改变导入语义。正文也在 open 前检查既有字符数与四倍 UTF-8 字节上限，open 后严格 UTF-8、实际字符数一致，完整交付或拒绝，不截断。不宣称对任意物理损坏数据库的所有分配都建立了全局上界。

元数据路径不能打开任何 event.body_ref；可解封已限额身份以核对归属。第二次点击只打开选中的 body。短同步 Session / store guard，沿用 import busy 的拒绝行为，不新增 worker 槽、一次许可、token 或持久批准。核心保证当前锚和用途，不能宣称它能证明物理用户手势。

读取沿用 people_graph / memory_detail 的纯读语义。现有 AuditAction 没有 source.read，不伪造 inference.write 或成功 capability.reject，不新造 schema/action。成功和普通拒绝不改审计链；错误、Debug、日志不含正文。

## 界面（PLAN REVIEWED）

每条证据先提供“核对本地来源”，只取元数据；面板列出准确事件，再由用户选择“在本机显示这一条敏感内容”。同一时刻只显示一条正文，明确本地敏感来源和当前核对范围，显示既有标识提示与完整 ID。React 纯文本呈现，不解析 HTML、Markdown、链接或资源，不新增复制、存储、端点、记忆或起草按钮。

关闭、切换 evidence/event/relationship、来源通知、hash 改变及卸载都立即使票据失效，清除正文与错误；IPC 前和交付前核对 mount、render-time source/hash、选择代次和精确锚。旧 DOM 回调不得在地址/来源已变但 React 尚未提交时发起读取。关闭后迟到成功/错误均丢弃，重开需要新点击。面板标题可聚焦，关闭仅在原按钮仍连接且归属不变时恢复焦点。

正文只持于当前组件状态，不放 URL、持久存储、日志或共享缓存。清空页面状态不是安全擦除已经返回的 JavaScript 内存，密钥销毁也不能追溯抹除已交付文本；应用内来源变化负责清除显示，后续读取重新验证。

## 验证与分工

核心实现者负责 store 的窄方法、既有身份解码复用、Session / IPC / core wrapper 与核心测试；界面实现者负责 Graph、查看面板、fakeCore 与 UI 回归。集成人单独维护本设计与当前状态，持有唯一构建通道；独立只读评审者先核实际 dirty diff 再签复审。

- 合成现代 JSONL / Telegram 的直接 incoming/outgoing 和群 incoming 精确来源；owner 群消息即使被伪造关系引用也拒绝。真实 owner→A 事件伪挂 B、witness 缺失/撤销/超容量都拒绝；跨 namespace 引用不得套用 bundle namespace。
- 元数据不 open body，选一条不 open 另一条；损坏另一条 body 的反例仍可核对选中项。legacy、忘记、撤销、未知及跨锚拒绝，不回退另一人。
- SQL/doc/row/field/CK/AAD、sender/owner、会话、venue、ts、subject、用途变化均拒绝；active CK 配错误 pointer 不通过。
- 合法 65536 字符和四字节 UTF-8 边界、谎报 char_count、真实大 ciphertext、非法 UTF-8、超大 identity/account 的首次解封前预检；不以截断冒充成功。
- 纯读前后领域数据与审计链不变，端点已配置仍零 HTTP，Debug/错误无正文。
- 真实 wrapper 与 React 联合覆盖两次明确点击、A/B 迟到、close→late success/error、source/hash/unmount/旧 DOM、显式重开及 hostile literal 无导航。
- 相关 Rust、完整前端、type/lint/production build、两 workspace 格式、实际命令表、schema/E0/禁词与文档静态检查。原生桌面/IPC、完整 G-L、G-W/G-M 单独记实际范围，不用静态检查替代。

## 实际结果

四份已审方案先同步为 `cc32eb29e6a76c32109c5e4b33d531344b9f2389`，当时明确 CLAIMED、不把规划当实现。核心/存储/IPC、界面及评审修复均完成独立实际 dirty diff 复核（REVIEWED）；实现与结果由包含本文的提交标识，PR #66 保持草稿。

- Linux / pwsh 7.4.6 / Rust 1.83.0，最终实际命令 `cargo test -p soulcore -p soul-import -p soul-store --all-targets --locked --offline`：**67 套件 / 473 通过、0 失败、1 忽略**，退出 0。忽略项仍是既有 Q2-03、要求干净 SHA 与 Windows pwsh 的专项测量，没有新增忽略。三包范围不是整个根 workspace，更不是独立 desktop workspace。
- 新增 `session_evidence_sources.rs` **15 项**真实 Session / 临时 SQLCipher / loopback MockLlm 回归通过，覆盖两种导入、incoming/outgoing 与群 incoming、真实 namespace、准确原文、零 HTTP/审计不变、无正文、遗忘/撤销/旧索引、坏另一 body 不影响选中项、跨锚/会话/用途/指针、UTF-8/65536 字符边界、谎报字符数与大实际 ciphertext、巨大 identity 仍可去重，以及 owner 群和 owner→A 伪挂 B。共享预算反例先证明两个单项 raw 各自成功，再要求同次 metadata 拒绝，防止每条重置。
- 存储新增 **5 项**真实 SQL 反例通过，核对 SQL/doc 绑定、密封行重放、CK 销毁、namespace 撤销先于身份解封、body row/field/key/subject/nonce，以及 message/account 的真实长度先于 open。定向 `cargo test -p soul-store --lib local_source_tests --locked --offline` 5/5 与此前 core lib 24/24 都是最终覆盖的子集，不另加到总数。
- 完整前端直接 CLI `vitest run --maxWorkers=2`：**43 文件 / 517 测试**，TypeScript、ESLint、临时 `NODE_ENV=production` 的 Vite build 均退出 0（51 modules）。包括 20 项 UI 和 8 项真实 wrapper 回归，明确两步、A/B 迟到、已显示正文/错误后的清除、旧 DOM、微任务与 hostile literal。原 Graph 档位测试只收口为精确三个档位按钮，原值与可用性断言保留。
- 初审发现的 owner 群与 outgoing peer 证明缺口均先补审方案，再实际补码和反例。首轮新测试编译修正 Telegram parser 的 Value 输入与 EventFilter 借用；首轮执行 14 过/1 失败来自 Telegram #13 空附件 caption 的错误假设。修正为有真实空消息对照的 NoBody/raw 拒绝，非空仍强制 Available/原文匹配，incoming/outgoing/group 正向保证保留，未改生产绕过。
- 首次 core lib 遇到旧 `soul_draft` 依赖 E0463，定向重建该包后通过。全部目标曾两次在既有 `crash_recovery` 缓存 ELF 缺执行位处以 os13 中断，均为 never executed、非断言失败；恢复构建缓存可执行权限后，同命令完整重跑退出 0。最终 Cargo 输出头部仍包含旧 E0463/旧测试类型诊断及增量清理 warning，所示旧源码与当前文件不符；原始输出保留，不称完全干净的新构建。结果依据是退出 0 及实际执行的 67 套件，其中当前 15 项来源测试逐项通过。未修改源码或测试来回避执行。
- E0 首轮发现 hostile fixture 的两条外部合成 URL，未增加 ignore 或放宽规则；仅换为 loopback 合成地址，保留 no links/assets/network 断言，经独立窄审后重新跑完整前端和审计。最终 schema-freeze、E0（16 shipped crates / **277** 源码文件）、禁词（94 项 / **133** 文件）均通过。依赖锁、持久 schema、T4D/A0–A3 未改。
- 根/独立桌面 workspace `cargo fmt --all -- --check` 通过。实际命令 inventory **57**，从原 `command_surface.rs` 提取原 6 条断言、独立 rustc 编译执行 **6/6**；这只验证源码命令面。新增 native mock handler 场景源码真实配置并调用两命令、核 nested snake 参数/foreign origin/审计/零 POST，未编译或运行原生测试。HTML 历史 script/SVG 字节未变，四处入口与相对链接静态核对，未称新浏览器交互验收。

所有检查仅使用合成材料、临时库和 loopback，不读取真实用户库或聊天导出。当前缺原生 GTK/WebKit 开发依赖，原生 desktop/IPC 未运行；完整 G-L 继续暂缓，G-W/G-M 未执行。不安装、发布或合并 main，不标 GATED/INTEGRATED，不关闭 Goal 1、正式 Goal 2 或 Wave 1。真实模型效用仍是独立未测事项；本模块的本地查看不能作为原文出网或生成草稿事实保证。
