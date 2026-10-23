# N3 统一写操作、主体归属与事件刷新：独立复审

日期：2026-10-23，America/Los_Angeles。任务来自作者明确授权的 N3，范围为合成数据、现有预览/确认/一次消费、HTTP＋IPC 固定回归。候选为 `codex/soul-rebuild-20261019` / `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46` 加既有 D109/N1/N2 和本轮未提交修改；HEAD 本身不是候选版本。

独立只读 reviewer 为 `readonly_api_review`（`soul_reviewer`，`gpt-6.1-sol/high`）；实现由 runtime、API/native 和 UI 写者分片完成，接口与共享文件串行，主线程集成和执行检查。reviewer 先读任务、起点副本及实际 diff，再读实现交接和原始日志，没有修改实现或自行运行构建。原始评审及各轮结果保存在本机 [review.log](E:/SoulSyntheticLab/local-api-phase2-20261023/review.log)，不是随仓库发布的证据。

## 范围与发现

审查覆盖 runtime/core/store 的主体槽、一次消费、导入事务与审计、事件锁序；HTTP 的认证、输入接收、派发及断连关停；生产 IPC 主体注入；native relay 的 AppHandle 生命周期；前端自身回声去重、启动/恢复/事件缺失、旧读取及各页面刷新。

| 发现 | 修复与闭合证据 |
| --- | --- |
| 导入准备/取消持久状态未审计；跨主体提前拒绝绕过既有拒绝审计 | 经作者 N3、“全部写操作进审计”和 ADR-002 第 9 项授权范围核对，增加 `import.prepare` / `import.discard` 白名单并维护既有 schema lock；状态和审计原事务内提交，主体拒绝复用 `capability.reject`。未增加审计 actor 字段或合法遗忘的前置审计条件。store hook 的失败回滚/重试验证见 `store-import-operations.txt`。 |
| B1：执行后 revision 不匹配的导入 Reserved→Invalidated 分支遗漏审计 | 使用同一事务 hook；审计失败回滚到 Reserved，但不恢复 held preparation 或交付批准。`import-completion-audit-red.txt` 两项实际失败，`import-completion-audit.txt` 两项通过、14 过滤；reviewer 已核对源码与原始退出 0。 |
| F1：仅关联 UUID 去回声，其他 caller 可被误认为本窗口 | 同时检查可信 desktop 主体、当前 WebView label 和本地 correlation；覆盖其他 WebView 与 HTTP 复用相同 UUID。 |
| F2：首个快照前或首个已结算快照可能交付旧读取 | 启动同步进入未就绪状态，首个成功订阅后的快照失效旧数据；普通读取在未知/外部在途状态拒绝派发或交付。保留原本地 pending 读取生命周期。 |
| F3：native 监听失败后未恢复 | 保持不可读并有界重试，成功订阅后快照核对；同步失败显示固定安全提示，清理取消重试/迟到订阅。F1–F3 经修复后 `frontend-targeted-6.txt` 12 文件 173 项通过及窄复审闭合。 |
| F4：快照恢复先于本地写入回执时，App 错误页可能永久保留 | App 订阅已有 draft-source，兼收外部变化和本地 finally 唤醒；保留 generation、同步失败/pending 保护。`frontend-app-recovery-red-2.txt` 实际 9 通过/1 失败；`frontend-app-recovery.txt` 2 文件 34 项通过，窄复审闭合。首轮不可配置 ESM spy 的 harness 错误另存，不冒充行为 RED。 |
| 扩大回归暴露的 Graph 导航延迟回调抢走用户焦点 | 受控 rAF 反例实际期望 metadata button、收到 h3；仅在当前焦点等于排队时 initialFocus 才转移，原 hash/source/target/cancel 检查保留。补未接管时仍聚焦标题；`frontend-final-fixes.txt` 5 文件 84 项通过，独立窄复审闭合。真实 WebView 原生焦点时序仍未观察，不以假设扩展追踪机制。 |

HTTP/native 源码复审未发现额外具体阻塞；这不表示没有未知缺陷。已读取 `api-write.txt` 的 4 项通过，覆盖严格 JSON/边界/正文超时、合成导入、显式取消、客户端断开后的已受理操作结算。`desktop-local-api-2.txt` 的 7 项通过覆盖真实回环 HTTP＋生产 IPC、两个方向主体拒绝、主链编辑停用/遗忘/重启和 native 元数据事件。首轮桌面 5 通过/2 失败源于夹具缺少语气档案；仅补已有生产 IPC `set_voice` 前提，业务断言未放松。

扩大检查另有两类测试修正经独立核对：薄层字符串 matcher 只兼容最外层合法尾逗号，其他 token 仍精确匹配；App 等待同一状态元素的读取完成，Memory 只等待初始 A 导航，后续 A→B 的同步旧按钮攻击、旧正文隐藏和零写入断言未改。reviewer 从 `desktop-synthetic-2.txt` 独立汇总 150 通过、2 过滤并核对退出 0。焦点 RED 因参数转发实际跑了完整 64 文件（775 通过/1 失败），明确保留实际范围，不改写为单用例。

## 结论与证据边界

本轮具体源码发现均已修复并由独立 reviewer 闭合。完整检查及最终候选保存以[实施回执](../plans/2026-10-23-repository-normalization.md#n3-统一写操作与事件刷新实施回执)为准，局部测试批次不累加为总覆盖量。

最终证据复核读取当前原始日志及 `attempts.txt`：根 workspace 206 组汇总 1654 通过/29 忽略/3 过滤；桌面 8 组汇总 150 通过/2 过滤；前端 64 文件/777 项通过；生产前端构建、lint、两个 workspace 的最终格式和严格 clippy、schema/E0/禁词检查均退出 0。reviewer 独立重算两份 Rust 日志，核对 218 份源码副本存在及 8 个关键文件与当前字节一致，并实际核对 branch/HEAD 未变、暂存区为空；没有独立重跑构建或沿用旧 N2 绿灯。

最后证据核对发现实施回执对 B1 原始失败的一句表述错误，已据 RED 纠正：当时调用仍返回“来源变化”拒绝，错误在于审计不可用时状态仍持久失效、没有回滚；不是业务调用成功。修正只影响文档，不改变源码或测试结论。当前文件目标核对、STATUS 行数与原生/远端未验证边界见实施回执和外部 `document-targets.json`。

主体只区分当前 WebView label 与单个共享 bearer 驱动，未实现独立多驱动会话或任意同名窗口重建隔离；事件是进程内无正文通知，持久审计没有 actor 字段。旧二进制不能解析新增审计动作，不承诺降级或 CK 恢复。MockRuntime 与生产 IPC handler 运行不等于原生窗口验证；真实采集、外部模型、生产数据、剪贴板/托盘/安装、远端 CI 与正式 G-L/G-W/G-M 不在本轮完成声明内。
