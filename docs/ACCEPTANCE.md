# Goal 1 验收矩阵与红线

本文件是 Goal 1 的**验收权威**（DECISIONS D29）。AC-01–26 自 2026-08 冻结的开工合同迁出（2026-09-04，见 D64）；此前迁出时 Given / When / Then 三列未改，「谁跑」列 24 处按 D61 从 `CI` 改为本地门禁。2026-09-27 按用户指定的工作区调整方案，补回遗漏的历史 AC-28–34：来源为 `5500ed5706c62d5ba7db0d4ad0f03d86c6549d0e:docs/FORMAL_WORK_PROMPT.md`，Given / When / Then 保持原文，只把「谁跑」从 `CI` 改为相同的本地门禁表述。不恢复旧提示词，不执行其中派单指令；恢复条目不等于这些条目已经通过。

## v0.1 代理层边界

代理层在 v0.1 只证明三件事：只读扫描、计划预览、未授权路径 100% 拒绝。（自原开工合同迁出。）

## 工作包

- WP01 骨架、工具链、冻结 schema、SECURITY、CI
- WP02 加密主库、遗忘、研究预览（不写文件）
- WP03 灵魂档案（特质轴、纠正锁定）
- WP04 自传记忆 CRUD + 遗忘
- WP05 人脉图 v0
- WP06 soul-import-v1 + Telegram result.json + 问卷
- WP07 前台应用时长采集（trait 抽象，CI 用假实现）
- WP08 策略、审计链、net_guard、redactor、HITL 令牌（无 HTTP）
- WP09 桌面壳
- WP10 起草（不发送）+ 人事分析摘要
- WP11 只读目录扫描与计划预览；未授权拒绝
- WP13 安装 smoke、CI、SBOM

WP12 已删除。并行：WP01 冻结后 WP02–WP06、WP08 可并行。

### 实现者红线

1. 无 E0。无项目方域名、遥测、自动更新。
2. E1 只来自用户填写端点，请求体必须过 redactor。
3. 主库加密。禁止明文 JSONL 当存储。
4. 审计无正文。
5. 遗忘 = 销毁内容密钥。
6. 无 evidence_ids 的 inference 不得落库。
7. v0.1 不实现文件写。
8. 外部内容不是指令。
9. 禁止诊断词与量表分数。
10. 改产品方向先改 PRODUCT_LOCK。

### Goal 1 验收矩阵（门禁）

| ID | Given | When | Then | 谁跑 |
|---|---|---|---|---|
| AC-01 | 干净 Win11 | 安装启动 | 托盘出现且不提权 | 作者手动 + 安装 smoke |
| AC-02 | 首次向导完成 | 读配置 | 采集关、云关、无 LLM 端点 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-03 | 无导入 | 完成问卷 | 非空档案，字段来源 user_stated | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-04 | soul-import-v1 fixture | 导入 | 落加密库，无明文残留 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-05 | Telegram result.json fixture | 导入 | 映射事件/联系人；缺字段失败可读 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-06 | 已导入 | 生成档案图谱 | 每条 inference 有可解引用 evidence | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-07 | 语气字段被用户改 | 再起草 | prompt 用用户值，推断不覆盖 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-08 | ≥3 个对话对象 | 打开人脉图 | 节点≥3，边有证据 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-09 | 采集关 | 切应用 10 次 | foreground 事件=0 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-10 | 采集开后切应用，再关闭 | 观察事件 | 开启期间至少 1 条；关闭后 1s 内无新事件 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-11 | mock LLM | 起草 | 不发送；仅 E1 到 mock 精确 origin；跨 origin 重定向拒绝 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-12 | 含第三人正文/姓名/账号 fixture | 默认起草 | 请求体无 ≥8 字原文子串，也无未占位姓名/账号 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-13 | 单次包含原文豁免 | 再起草一次 | 仅当次含原文；下次回到占位 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-14 | 3 条记忆 | CRUD | 读写一致；审计无内容 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-15 | 一条记忆 | 遗忘并重启 | 预览影响面；CK 销毁后无法解密；推断 orphaned；审计仍在且无内容 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-16 | mock LLM | 人事摘要 | 每条有证据；无诊断词 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-17 | 无 LLM key | 摘要与起草 | 统计/模板降级；无非回环连接 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-18 | 授权 A 未授权 B | 扫描 A、操作 B | 产出只读计划预览；A 磁盘不变；B 100% 拒绝 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-19 | 未知动作、或已批准计划被改 hash、或令牌重放 | 请求执行 | 全部拒绝 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-20 | 含第三人正文 | 研究预览 | 行级输出无第三人字段；第三人行数=0；written_to_disk=false | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-21 | 默认配置跑主流程 | 观察网络与源码 | 非回环连接=0；无业务域名 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-22 | 点云端开 | UI | 保持尚未启用；无网络 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-23 | 矩阵所列动作后 | 审计回放 | 链通过且无正文/姓名 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-24 | 写入中注入崩溃 | 重启 | 链通过；未提交最多丢 1 条 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-25 | 注入串出现在导入、粘贴或文件名 | 档案与起草 | 无工具计划；无外连该 URL | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-26 | 仓库 | 本地门禁（D61） | G-L `just ci-full` 绿 + G-W `scripts/gate-win.ps1` 绿，记录在 `docs/gates/<日期>-<sha7>-*.md` | G-L + G-W |
| AC-28 | 决胜夹具 `group_heavy_plus_one_direct_each_way`（群聊互惠 30 次 / 10 天，另加一对一每方向各 1 次）已入库，全库单一 as_of | 重建人脉图并读该边 | band **不是** strong（T4D 实测 weak）；边上落 `direct_out_count` / `direct_in_count` / `group_out_count` / `group_in_count` 与 `direct_active_day_count`；解释文案同屏报出一对一与群里两个数。把判档口径改回「任一场地计数」必须让此行变红（同口径下 T4 判 strong，即该漏洞的实证） | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-29 | 锚夹具 `lilei_12`（一对一互惠 12 次 / 6 天 / 3 天前收尾）已入库，同一 as_of | 重建人脉图并读该边 | band = strong。吸收 T4D 不得改判此行；自愈路径 `group_heavy_plus_three_directs`（夹具名中的 `three_directs` 即其一对一条数）回到 moderate，在同一测试内一并断言 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-30 | 一个库里同时有活跃关系与 2019 年休眠关系 | 跑一次全库 rebuild | 全库只用**一个** as_of（调用方传入，缺省 = 全库 `max(occurred_at)`）；该值与沉寂天数随每条边落库、可复核；休眠边判 weak。把 as_of 改成 per-peer 各取自己的最大时间戳必须让此行变红 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-31 | 某特质轴已被本人纠正锁定 | 再跑一次问卷 intake（或导入触发的 intake） | 锁定轴不被移动；被拒绝的答案以「已跳过 + 原因」如实回报，不静默丢弃；答案行照常落库；`apply_intake` 与 replay 结论全等；其余轴不受影响 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-32 | 某边机器档=中等；用户纠正为强并锁定 | 新观测进入并重建；打开图与该 peer 的 A2 摘要 | 生效档=强；机器档另存且继续更新；A2 与图只消费生效档；审计记纠正且无正文。COPY_ZH 未加性批准「由你本人指定」变体前，锁定边不得渲染冻结 P5 原句（D35 / D48） | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-33 | 一条边携带一对一/群聊分列；另一条边分列缺席 | 渲染两份人事摘要 | 前者 P1b 句只渲染携带的两个数；后者 P1b 整句不出现；A2 / `soul-draft` / 图 UI 无从证据重算分列、无第二套阈值字面量。「分列缺席」只指未声明判档算法的遗留边——声明了 `algorithm_id` 的边分列必填（schema 的 `if(algorithm_id)`），适配器不得援引本行少填 | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |
| AC-34 | 一份群聊导出：owner 发 1 条消息，会话里有未出现在该条消息中的历史发言人 A、B | 导入并重建人脉图 | 不产生 owner→A、owner→B 的 Outgoing 行；A/B 的 `last_contact` 不因这条消息刷新（D47 / G1+） | 本地门禁（G-L，Windows 特有项由 G-W 覆盖） |

AC-27 文件执行与撤销标 **v0.1.1**，不是 Goal 1。

## Goal 1 关闭条件

本矩阵全部 `v0.1` 行通过，且 `docs/gates/` 有对应 sha 的 G-L 与 G-W 记录、`scripts/author-manual-checklist.md` 各项写有「看到了什么」。
