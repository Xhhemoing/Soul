# 自主拍板记录

作者 2026-08-24 声明：除已给出的产品方向外，其余问题可自主决定。本文件记录拍板，避免后续子代理重开方向战。

| ID | 问题 | 决定 | 理由 |
|---|---|---|---|
| D1 | Soul 是助手还是数字复刻 | 数字复刻为灵魂层，助手为代理层，复刻是本体 | 作者定义是电子版的你 |
| D2 | 要不要默认云端 | 默认全本地；v0.1 无云端 HTTP | 本地优先 |
| D3 | 社交数据怎么进来 | 官方导出或手动导入；v0.1 无 OAuth；禁止爬虫 | 合法可审计 |
| D4 | 能否自动回消息 | v0.1 只起草不发送 | 代发不可逆 |
| D5 | 心理模型是否临床 | 否。可替换特质轴 + 证据档 + 用户纠正 | 避免医疗声称 |
| D6 | 手机何时做 | Android 在 v0.3 | Windows 先闭环 |
| D7 | 研究数据如何用 | v0.1 只预览不写文件；与助手出网分离 | 防偷渡 |
| D8 | v0.1 是否要真预测模型 | 只要 schema 与离线入口 | 先有数据契约 |
| D9 | 技术栈 | Tauri2 + Rust + React | 常驻与打包 |
| D10 | 文件整理是否产品本体 | 否。v0.1 只做只读计划预览；执行在 v0.1.1 | 代理层证明不靠真写文件 |
| D11 | 出网怎么分级 | E0/E1/L，全部经 net_guard | 否则「零出网」不可测 |
| D12 | v0.1 的 E0 | 无实现、无域名、无 HTTP client | 关开关不够 |
| D13 | 第三人正文进 E1 | 默认占位；单次整条豁免；研究无豁免 | 第三人未同意 |
| D14 | 主库 | 加密 SQLite；JSONL 非存储 | R1 S3 |
| D15 | 遗忘 | 销毁内容密钥；不承诺物理擦除 | 可测 |
| D16 | 遗忘 vs 审计 | 审计无内容，可留孤立 UUID | 两承诺共存 |
| D17 | v0.1 导入 | soul-import-v1 + Telegram result.json | 具名才可测 |
| D18 | 研究导出形态 | 预览 only | 脱敏未红队前不落盘 |
| D19 | WP11 写执行 | Goal 1 不做 | 防误读成产品本体 |
| D20 | 记忆是否门禁 | 是 | 作者原意第 6 条 |
| D21 | 人事分析是否门禁 | 是；无 key 则统计降级 | 作者原意第 5 条 |
| D22 | 大五怎么写 | 方向轴 + 弱中强，禁 score/percentile | 非量表 |
| D23 | 采集面 | 仅前台应用时长 | 同意闭环一项即可 |
| D24 | HITL | 未知拒绝；plan_hash 变拒绝；令牌一次性 | 可测 |
| D25 | 注入 | 外部内容永不进指令位；红线进门禁 | R1 S14 |
| D26 | 写码前文档 | 锁、拍板、九 schema、SECURITY、STATUS | 14 份会空转 |
| D27 | 第二份 PRODUCT.md | 禁止 | 防双源 |
| D28 | Goal 2 | 拆到 GOAL2_POLISH_PROMPT.md | Goal 1 必须有终点 |
| D29 | 验收权威 | FORMAL 中 Given/When/Then 矩阵 | 散文不作门禁 |
| D30 | 工作包增减 | R2 后只减不增 | 防回弹 |
| D31 | 只读整理预览是否留在 Goal 1 | 留。执行仍在 v0.1.1。这是代理层只读证明，不是产品卖点 | 否决 R3-sol 删除 AC-18 的建议 |
| D32 | 未锁边是否写 `machine_band` | 写。`locked ⟺ user_band.is_some()`；重建边 `machine_band` 常为 `Some` | 缺席无法区分「未锁」与「换血前的行」；落地测试已钉 |
| D33 | G1+ 后 `{群聊次数}` 口径 | 渲染持久化 `group_out_count + group_in_count`（可含 0）；不从证据重算；不改 COPY_ZH 措辞 | owner 群消息不再归因后 `group_out_count` 结构为 0；改模板须先改 COPY_ZH |
| D34 | 遗忘 vs 锁（GC-6/7，R4/R5） | Goal 1 不实现。rebuild 仍只遍历有观测的 peer；时间戳保持非 Option | 墓碑/Option 化是迁移，不是默认；记 STATUS 遗留 |
| D35 | GC-9b「由你本人指定」 | 禁止。COPY_ZH 未冻结该 key 之前产品非测试源码与 fixture 不得出现该句 | 测试里的反向断言字面量除外 |
| D36 | v1 按发言人数判 Direct/Group | 维持。单活跃发言人的群可被判 Direct | 改判据要动冻结契约；STATUS 钉死现状 |
| D37 | Telegram `date_unixtime` 范围 | 对应 RFC3339 年必须在 1970–9999（四位年）；否则该消息 defect，不入库 | 否则 fail-hard rebuild 永久失败；导入非事务 |
| D38 | v1 / Telegram 民事日 | `is_civil_datetime` 按月天数（含闰年）校验；`2026-02-31` 为缺陷 | schema 层与本地校验双钉，关掉 format 校验时仍拒 |
| D39 | `IntakeReceipt.ignored` | 加法字段（question_id + reason token）；`answered` 不计被锁轴拒答 | WP03 遗留 8；链上不加新审计枚举 |
| D40 | Goal 1 人事摘要走哪套话术 | `soul-draft` 调用冻结 `soul_algo_trait::a2_render`；不改 a2.rs 模板；锁定边仍丢掉 filed_band | A2 是渲染器；COPY_ZH 与 crate 文案漂移另记，不在 Goal 1 改冻结 crate |
