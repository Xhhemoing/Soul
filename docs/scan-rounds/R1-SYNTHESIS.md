# R1 综合稿

轮次：R1 扫描（只读）。模型：
- [R1 opus plan scan](bc-ae1f3731-3cff-57cf-aab3-8e77527156a5) `claude-opus-5-thinking-high-fast`
- [R1 sol plan scan](bc-4055f588-dada-5d05-8326-87bf6bf4a1ef) `gpt-5.6-sol-xhigh-fast`

结论：`PLAN_BLOCKED`。产品方向保留。当前计划不可开工。

## 共识（R2 必须闭合）

| ID | 议题 | 采纳 |
|---|---|---|
| S1 | v0.1 过宽且偏移 | 灵魂层验收不足，文件整理过重。砍范围，补记忆与人事分析 |
| S2 | 导入格式未具名 | v0.1 只做 `soul-import-v1` JSONL + Telegram Desktop `result.json`。不做 OAuth。微信/QQ 不绕过 ToS |
| S3 | jsonl vs 加密库 | 加密 SQLite 为主库；JSONL 只做研究导出产物 |
| S4 | 出网语义 | 拆 E0 业务 / E1 用户 LLM / 本地回环。E0 默认零。E1 默认把第三人正文换成占位符 |
| S5 | schema 缺失 | 写码前冻结 event / evidence / profile / memory / contact / relationship / audit / export-manifest |
| S6 | 验收是散文 | Goal 1 改成 Given/When/Then，并区分 CI / 作者手动 / mock |
| S7 | 遗忘 vs 审计链 | 密码学抹除内容；审计只留无内容 `forget` 记录 |
| S8 | 字段级隐私 | 分类 self/third_party、raw/derived、目的、同意、保留期、允许去向 |
| S9 | HITL 不可测 | 决策表：未知动作拒绝、计划哈希变化拒绝、令牌一次性 |
| S10 | Goal 2 无终点 | 从 v0.1 开工提示词拆出，Goal 1 有门禁即可结束 |
| S11 | 文件整理 | 移出 v0.1 发布门禁；最多 v0.1.1 仅授权根内可逆移动 |
| S12 | 大五 | 可替换特质轴，非临床、非分数；弱/中/强证据档 |
| S13 | 云端适配器 | v0.1 不发请求；开关文案「尚未启用」 |
| S14 | 注入 | 导入/粘贴内容进起草与工具计划必须有红线用例 |

## 分歧与仲裁

| 议题 | opus | sol | 父代理 |
|---|---|---|---|
| Telegram vs 仅 Telegram | JSONL 中间格式 + Telegram 参考适配器 | 只支持 Telegram `result.json` | 两者都要：中间格式是契约，Telegram 是唯一真实适配器 |
| 云端空壳 | 空壳可能带出网路径，可推迟 | 可见但无传输，测配置/策略 | 无传输、无出网代码路径；UI 写尚未启用 |
| 研究导出 | v0.1 只预览不写文件 | 允许脱敏导出但无第三人边/标识 | 只预览 + 字段清单；不写导出文件 |
| 文件 worker | v0.1.1 要策略校验，沙箱推后 | 严格可逆移动白名单 | 同意 opus 分期 + sol 操作白名单 |
| 记忆/心理/研究是否进 Goal 1 | 必须补记忆与人事分析验收 | 工作包在不等于闭环，研究入口要进验收 | 记忆 CRUD/遗忘与人事分析摘要进 v0.1；离线训练只验证特征文件，不承诺效果 |

## 驳回

- 不把产品改回通用助手。
- 不把微信/QQ 非官方抓取写进 v0.1。
- 不把 20 轮打磨当作 v0.1 完成条件。

## 下一轮指令（R2）

只改文档与 schema 草稿。禁止应用代码。禁止增加工作包。WP 只减不增。

必须交付：

1. 出网分类 + 第三人正文占位规则的完整补丁。
2. 存储/密钥/遗忘/审计分离的完整补丁。
3. `docs/schemas/` 下 JSON Schema 草稿：event, evidence, inference, profile, memory, contact, relationship, audit, export-manifest。
4. Goal 1 验收矩阵（Given/When/Then + 谁跑）。
5. v0.1 砍/留清单落到 PRODUCT_LOCK、DECISIONS、FORMAL_WORK_PROMPT。
6. 写码前文档清单缩到真正阻塞项；不新建第二份 PRODUCT.md。

两个模型只给提案和 diff 文本，不要抢写同一文件。
