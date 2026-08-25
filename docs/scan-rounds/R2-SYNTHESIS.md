# R2 综合稿

轮次：R2 规范补丁。模型：
- [R2 opus plan patches](bc-32d82525-2c83-5745-a723-d0af183e7c8a) `claude-opus-5-thinking-high-fast`
- [R2 sol plan patches](bc-513459d3-2f20-51ca-af15-5acddd931480) `gpt-5.6-sol-xhigh-fast`

父代理已按本综合稿落盘文档与 schema。结论仍待 R3 复核，当前不是 `PLAN_FROZEN`。

## 共识（已落盘）

- v0.1 证明灵魂层 + 一条可测代理层只读路径；文件写执行移出 Goal 1。
- 导入只允许 `soul-import-v1` JSONL 与 Telegram Desktop `result.json`；无 OAuth；无微信/QQ 抓取。
- 加密 SQLite 为主库；JSONL 只作导入契约与研究预览渲染。
- E0 无代码路径；E1 仅用户填写端点；L 仅回环。
- 第三人正文默认占位；研究预览第三人行数恒为 0，不写导出文件。
- 遗忘 = 销毁内容密钥；审计无正文。
- 记忆 CRUD/遗忘与人事分析摘要进 v0.1。
- 大五改为可替换特质轴 + 弱/中/强档，非分数非临床。
- Goal 2 从开工提示拆出。
- 九份 schema 进 `docs/schemas/`。

## 分歧仲裁

| 议题 | opus | sol | 采纳 |
|---|---|---|---|
| 第三人正文进 E1 | 默认占位；单次豁免整条消息 | 默认同；可按片段勾选 | 默认占位；单次豁免整条消息，不记住，不可用于研究 |
| 无 E1 时起草 | 模板降级 | 本地确定性回退必须能过验收 | 采纳 sol：无 key 时确定性语气模板仍算通过 |
| WP11 | Goal 1 只读预览 | 保留 v0.1.1 占位 | 清单保留 WP11 只读预览；执行标 v0.1.1 |
| 遗忘审计 | 允许孤立 UUID | 禁止对象 ID | 允许本地随机 UUID，禁止正文/姓名/内容哈希 |
| E0 | 无实现 | 运行期也不可打开 | 两者：无 feature、无域名、无 HTTP client |

## 已改路径

`docs/PRODUCT_LOCK.md`、`docs/DECISIONS.md`、`docs/FORMAL_WORK_PROMPT.md`、`docs/SECURITY.md`、`docs/GOAL2_POLISH_PROMPT.md`、`docs/schemas/*`、`docs/STATUS.md`。
