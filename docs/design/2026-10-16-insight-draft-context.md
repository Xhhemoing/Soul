# 可信洞察作为本次起草材料

起点 `f6972e8efa96aee65452c5b7e05a82c8c4c01f5a`，PR #66 草稿。用户要求持续按项目计划开发、自动审查并阶段同步。方案在编码前经独立只读复核，状态为 **PLAN REVIEWED**；实现与实际测试结果分开记录。用途决定见 D76，不增加 Goal 1 验收项。

## 要补的实际缺口

导入已经产生有效 A2 洞察，但起草只能选择手写记忆。用户不能明确选择一份仍有效的关系统计、核对来源，再决定是否让模型使用它。本模块使用一个指定 bundle 中一个联系人的全部可信 points，不把统计写成记忆，不混选记忆和统计，不把原始聊天作为起草材料，也不向页面或端点交付原文。

统计描述当前整库有效关系记录，不限于触发 bundle 的那一次导入。引用仅说明统计材料的出处；通用起草端点没有逐句事实验证，不能因此把生成草稿标成逐句有据。本机确定性模板继续只按已有语气规则生成，统计可供人核对；未勾选 includeCounts 时端点没有计数。

## 核心与 IPC 计划（PLAN REVIEWED）

核心按 exact bundle ID 读取，不依赖 recent 列表窗口；要求 active、未 ignored、操作 committed、namespace active、contact 属于原 bundle，并复验 W5 完整依赖。依赖不止是 Graph URL 的引用子集。在同一 store guard 下调用原统计生成器取得纯 Counts / narrative=None 的 PersonSummary，不从 UI 字符串、序列化 View 或模型叙事反构造材料。

| 类型 | 字段与含义 |
| --- | --- |
| InsightContextRequest | bundle_id、contact_id；嵌套 snake_case，deny_unknown_fields，不收 text 或 points |
| InsightEvidenceSource | evidence_id、既有 EvidenceKind、event_ids；只含对应引用的本地元数据 |
| InsightReference | bundle_id、operation_id、namespace_id、contact_id、relationship_id、summary、sources、notice；sources 恰为该 summary 全部引用的去重集合 |
| InsightContextView | context_id、reference；context 是本会话一次用途，不持久化批准 |
| InsightDraftValue | draft、reference；本机模板行为不改变 |
| InsightDraftPlan | prepared{plan、encoded_bytes、body_preview、includes_counts}、reference；以最终实际请求为准 |

新增三个薄命令：prepare_insight_context(request)、local_draft_with_insight(pasted, context_id)、prepare_draft_with_insight(pasted, context_id, include_counts, include_original)。Tauri 外层参数沿用 camelCase，generate_draft(approval) 与 discard_draft 不变；实际命令表预期由 52 增为 55，最终以静态核对结果为准。

独立 Held/PendingInsight 与 Memory 互斥。普通起草、手写记忆准备、统计准备、整体 discard、编辑和来源修改均不能留下另一类旧用途。local 成功或实际派发消费本次 context；prepare 不自动 HTTP。begin 与 complete 再核完整来源，已发生请求先审计再判断可否接收；重启不能恢复上下文或批准。Session ignore/recompute 保守使当前用途失效；直接 store 重算仅创建新 bundle，若旧 bundle 及其依赖仍有效，不宣称它自动撤销旧行。

替换清理针对尚未派发的 Held/Pending 用途。已派发 worker 仍沿用 D69 的许可余量；本轮没有为直接核心调用的普通 local 与已发 HTTP 并行另加状态。页面仍通过同一物理 lane、输入票据和显式 discard 拒绝旧意图交付，不能把清理未派发上下文说成撤回了已发字节。

includeCounts 默认 false，只提供固定本地参考标记；另选 true 才调用既有可信 summary_body 的匿名 A2 派生计数适配。该内部聚合适配不改变原 evidence/event 的 privacy，不调用 Memory/OwnerExcerpt，也不接受 paste 豁免。base paste 的原文豁免只交给 base 一次脱敏，不能命中统计 turn。通过现有 final JSON serializer 核对 64 KiB；整项可容纳则保留全部 points 与引用，否则拒绝，不能静默裁剪一部分统计。

W5 已有来源复验可能打开密封内容以验证存活，立即丢弃；本模块不新增原文 resolver，不向页面或端点交付原文。沿用现有 store、schema、算法、锁和唯一 HTTP 出口。

## 交互计划（PLAN REVIEWED）

LocalInsights 对有效未忽略的指定联系人提供显式选择，只在当前窗口保留 bundle/contact UUID 意图，不保存摘要副本，不自动准备或出网。与记忆候选明确互斥，切换时可调用既有 discard_draft 并等待真实 shared discard drain，以撤销旧核心用途；不能把清除 UI 候选当成核心批准已销毁。不自动准备、查询摘要或生成材料不等于禁止安全撤销 IPC。

Draft 提供独立统计面板，用户点击后取得本地预览和全部来源。独立 unchecked 勾选允许本次将 A2 派生计数给端点；最终 quote 冻结材料与勾选，paste 二次确认只影响 paste。显示最终 reference、includes_counts、body_preview 和实际 JSON 字节数；失效后须重新预览或明确清除，不能静默回退为普通有依据起草。

使用共享物理槽及 mount / source / 选择 / paste 代次。在 IPC 前和交付前核对，旧回调只收尾，不展示旧预览、批准或引用；unknown 不自动重试。来源引用只标注统计材料，不给端点整篇草稿背书。

## 验证计划

- 合成导入后选择一个 contact，引用集合准确、不混同操作另一人或 owner 群消息；未知、ignored、invalidated 或 missing bundle 拒绝，任意 UI 正文参数拒绝。
- 相同 UUID 引用下更改计数、band、用户锁、会话或内容密钥仍须拒绝旧上下文；遗忘后残留标识不能复活用途。Session ignore/recompute 和直接 store 新 bundle 的边界分别测试。
- 本地模板保持原结果且零 HTTP；includeCounts=false 实际 wire 没计数，true 只有可信统计，均无 contact/evidence UUID、账号、源正文和端点旧 narrative；paste 豁免不能扩大。
- 预算使用“base 自己可容纳、加完整统计才超限”的反例，整项拒绝且没有旧 pending 回退；实际 JSON 字节数与最终预览、引用一致。
- 一次派发、遗忘/重启/新普通或 memory 准备使旧批准失效；HTTP 已发后失效保留出网审计并拒收草稿。
- 前端真实包装与 React 联合回归覆盖 memory↔insight、同 operation 的 A/B 快速选择、discard 未完成、来源变化、旧预览/quote 迟到、默认勾选和最终材料一致；渲染/选择/本机预览不触发 generate/person_summary。
- 按实际变更运行定向核心与完整前端检查，原命令静态断言及 schema/E0/禁词/格式；原生桌面/IPC、完整 G-L、G-W/G-M 继续单独记录，不用静态替代。

## 实际结果

计划在编码前独立复核；核心/IPC/wrappers、UI 与评审修复均完成实际源码复审（REVIEWED）。阶段一的四份契约文档已同步为 `7d4421e253e7493519b5762de12007e6898845bd`，当时明确标 CLAIMED，不把规划当实现。阶段二源码与结果由包含本文的提交标识，PR #66 保持草稿。

- Linux / pwsh 7.4.6 / Rust 1.83.0：最终 `cargo test -p soulcore -p soul-import --all-targets --locked --offline` **52 套件 / 370 通过、0 失败、1 忽略**，退出 0。忽略项仍是既有 Q2-03、要求干净冻结 SHA 与 Windows pwsh 的专项测量，没有新增忽略。首轮在 39 套件后遇到缓存 `session_import_work` ELF 缺执行位、未执行该套件而中断；仅恢复一个构建缓存文件的执行权限后，同命令完整重跑，不把首轮 partial 当通过。
- 新增 `session_insight_context.rs` **10 项**真实 Session / 临时 SQLCipher / loopback MockLlm 测试均通过。覆盖 exact 引用、recent 窗口外读取、纯匿名 wire、默认未提供计数与明确提供、精确 serializer 字节、同 ID 改计数及同 bundle 另一人变化、实际 HTTP 后遗忘的审计拒收、普通/memory 替换、重算边界、合法 base 可容纳而完整统计超限的整项拒绝、重启。相关六套定向 **42 项**与此前单元 **24 项**也通过，它们是最终测试的子集，不另外累计。
- 完整前端直接 CLI `vitest run --maxWorkers=2` **41 文件 / 489 测试**通过，TypeScript、ESLint 与临时 `NODE_ENV=production` 的 Vite build 通过。包括 14 项新 UI/真实 wrapper 联合用例、8 项 core wrapper 用例及 4 项本地反馈读取反例。首轮补正新测试 fixture 缺失的既有 band 字段；关联回归还复现 ignore 自身来源变更导致列表永久丢弃，已修复为 mutation 成功后绑定一次新读取，保留旧断言并验证读中变更/卸载/拒绝的因果边界。
- 首个 Rust 编译节点发现新 revalidate 尾表达式 MutexGuard 生命周期不满足 Rust 1.83，改为显式局部 guard 后编译与单元通过；没有改变存储或授权语义。最终对认领 Rust 文件做定向格式化，根与独立桌面 workspace `cargo fmt --all -- --check` 均退出 0。
- 三个新增命令接到实际 Tauri 注册与 core wrapper，inventory 实测 **55 项**；原 6 条命令源码断言提取实际 inventory、独立编译执行 **6/6**。新增 native mock handler 测试源码调用三命令并核参数，仅源码复核/格式通过，未编译或执行原生测试。
- schema-freeze、E0、禁词检查退出 0。E0 实测 **16 shipped crates / 273 源码文件**，禁词实测 94 项 / 132 文件；冻结 JSON schema、持久化 schema、T4D/A0–A3 与依赖锁未改。HTML 历史 script/SVG 字节保持，新增链接和当前说明静态核对通过，未称为新的浏览器交互验收。

全部验证仅用合成材料、临时测试库和 loopback 模型，不读取真实用户库或联系外部人员。当前缺 pkg-config / GTK / WebKit 开发依赖，原生 desktop workspace/IPC 未运行；完整 G-L 继续暂缓，G-W/G-M 未执行。没有安装、发布或合并 main，不标 GATED/INTEGRATED，不关闭 Goal 1、正式 Goal 2 或 Wave 1。后续原文受控本地解引用及真实模型效用须单独规划验证，不能继承这次统计用途或 E1 批准。
