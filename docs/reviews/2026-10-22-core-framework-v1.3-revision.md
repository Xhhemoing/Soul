# 核心框架 v1.3 文档修订核验

日期：2026-10-22，America/Los_Angeles。范围：用户要求修订 v1.3 并分析执行；只核验框架和[执行分析](../design/2026-10-22-core-framework-v1.3-execution.md)，不代签产品、迁移或平台验收。

候选业务版本为 `codex/soul-rebuild-20261019` @ `c5be3ed3aaa09b6ef5e2a8861a07d7c7c1fa8b46`，业务源码干净；文档 dirty。修订人：主会话 Codex。数据与任务路径定向取证：`claim_schema_review`、`task_loop_review`；候选独立核验：`framework_route_review`，均为 `gpt-6.1-sol / high`。

## 核验结果

| 核验对象 | 结果与证据边界 |
| --- | --- |
| v1.2 审核 F1–F5 | 已在候选中改为未知条件、按对象来源投影、有限偏好应用闭环、可重放 fixture 与复发观察机会、正确暂停来源及正式 VAL 前置；没有把建议写成已批准实施 |
| 原结构 | OOXML 保留 192 个直接正文块（含 TOC／书签）、0–10 节、8.1、附录和 11 张表，各表行列数一致；只修改 document.xml，其他包内文件不变 |
| 首切片范围 | 仅普通本地模板、明确人物／场合和现有语域；完整 Claim、自动召回、模型与真实数据不是合成工程切片的前置 |
| 数据与遗忘 | Memory＋typed 关联是建议；普通编辑失活、联系人／Memory／CK 遗忘传播、版本兼容与事务反例均列为待实施、待验证 |
| 独立发现 | 初稿未说明页面继续编辑如何进入后端失效流程，容易把 edit_revision 误当成后端可感知的变化。已补有序 discard／版本失效回执、前端同步撤 ownership、提交中锁编辑及确认先提交的明确解释 |
| Word 视觉 | 最终 layout3 共 15 页逐页检查，未见截断／重叠；长表重新分配列宽，同一表行不跨页，附录另起一页。保留原章节／段落样式和页面几何 |

主线程另在逐页检查中发现风险表行对应错误及 authority 交叉引用旧称，已改正并核对最终第 6、10 页。所有中间产物和失败保留，未覆盖成只有最终成功记录。

独立评审最终复核已完成，限定文档／方案范围**无剩余具体阻塞**。独立比对 `revision-map.json` 的 142 项原文和最终 OOXML，0 处不匹配；确认修文准确区分本地编辑、后端已收到失效与已提交确认。完整回执见外部 `v1.3-independent-review.md`。

## 原始证据与限制

持久目录 `E:/SoulSyntheticLab/framework-v1.3-20261022/`（仅本机，未发布）：

- `source-v1.2.docx`、`revision-map.json`、`framework-v1.3-extracted.txt`：原件与逐位置修订。
- `execution-data-notes.md`、`execution-loop-notes.md` 及对应 commands.log：源码位置、候选方案、未运行验收。数据调查有一次 `keys.rs` 搜索无匹配返回 1，随后在 `store.rs` 定位；失败保留。
- `v1.3-independent-review.md`、`v1.3-independent-review-commands.log`：独立内容核验、初始发现及定向复核。
- 独立映射辅助脚本首次按筛选后的 132 个 p/tbl 节点定位，误用了实际包含书签等元素的 192 个 body 坐标，退出 1；改用完整 body 后退出 0，142 项匹配。失败保留，不属于框架内容缺陷。
- `structural-validation.json`、`final-validation.json`、`visual-qa.md`、`layout3-page-01.png` 至 `15.png`：结构、文件保留与逐页视觉依据。
- `reference-render-attempt1.log`：bundled 渲染器缺 LibreOffice，退出 1；之后使用隐藏的 Microsoft Word 16 只读导出 PDF，再用 bundled Poppler 渲染。没有安装或调用桌面 LibreOffice。
- `authoring-attempt1-failure.txt`：首次 XML 序列化比较因继承 namespace 不同退出 1，发生在输出写入前；改为按元素、属性及内容比较后通过。后续 authoring-attempt2/3/4 分别保留文字修订、列宽调整及最终纠正。
- `main-static-checks.log`：主线程定向检查与一次提前读取尚未生成渲染日志的失败。原渲染任务随后退出 0，未重启或遮盖该失败。

未运行业务测试、新偏好闭环、旧库迁移、遗忘传播或真人观察；上轮 233 项回归只保留原版本和范围。本轮未恢复暂停研究、读取真实数据、调用模型或外部服务、运行平台门禁、提交或发布。
