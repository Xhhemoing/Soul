# 开放任务原文合成材料

本目录含24个一次新编的独立中文场景。人物、活动、规则、编号和金额均为虚构，仅供离线合成评估；没有读取真实用户记录、聊天正文、数据库、凭据或外部服务。旧的 `../synthetic-task-context/` 和 `../synthetic-task-context-v2/` 保持原样。本批不是对旧 family 换名后生成六种变体，每个场景只出现一次。

任务目标是从普通自然语言中识别真正必要的信息，再从原文识别来源的对象、范围、时间和明确更正关系，判断已有支持、冲突、缺失或不可读，并选取足以支持判断的材料。每例4–6条来源；共105条来源、50项必要信息。23例各含2–3项需求，另有1例纯改写任务无需外部信息。

## 输入与离线答案隔离

只有单例的 `input` 可以进入开放任务的模型输入：

```json
{
  "case_id": "一次分配的12位随机十六进制ID",
  "task_text": "普通自然语言任务",
  "sources": [
    {"id": "s01", "title": "自然来源名称", "text": "可见原文", "readable": true}
  ]
}
```

输入没有 `information_options`、`source_metadata`、预设需求描述或评分答案。`case_id` 在作者编写时随机生成一次并保存，不是内容hash，不编码变体；后续修正不得重生ID。来源ID只作例内引用。

顶层 `variant`、`compare_luna`、`prepared_control` 和 `gold` 全部属于离线评估材料，不传给开放任务模型。每类预先选定2例 `compare_luna=true`，合计12例；每类另选1例 `prepared_control=true`，合计6例。两组不重叠，编写时已经固定，尚未依据任何模型结果选择或调整。`prepared_control` 只声明预定对照成员，不宣称已经运行对照。

## Gold 语义

`gold.needs` 列出任务真正需要从外部来源取得的信息。任务自身已给定的数量、两轮等约束不再当作检索需求。纯改写例只需把任务给定文本拆成两项待办，`needs=[]`，四条来源全部无关；模型若增加查价格、时间等需求应计为额外需求。

每项需求的 `status` 是以下之一：

- `supported`：存在充分可读证据，`support_groups` 列出全部作者识别的最小路径。
- `missing`：目标信息没有给出，且不可读来源不涉及此缺口。
- `conflict`：同对象、同事项、同有效期有当前矛盾且没有明确覆盖关系；`conflict_groups` 列出充分矛盾证据组。
- `unknown`：可能含目标信息的来源不可读，可见目录只表明相关性，未透露答案。

两种groups均为组内AND、组间OR；不要求把全部替代路径一并选入。本批有2个必须结合分配记录与折法说明的AND支持组，14项需求存在多个最小OR路径。4个冲突需求各有一个双来源最小冲突组。冲突只属于发生矛盾的字段，不能因此删除同一来源对其他字段的一致有效支持。

`gold.source_metadata` 与来源一一对应。所有非null `subject`、`scope`、`time` 必须是该来源 `title + "\n" + text` 的连续原文片段，不能补出原文没有的归一化名称或日期。无合适明示片段时用null。词语允许在原文出现多次；这些标注是作者选择的短原文跨度，不宣称自然语言只有一种等价跨度。

`supersedes` 仅用于原文明示的同事项更正或撤回，`correction_quote` 精确保存该句并包括被替代来源ID。无更正时为 `[]` 与null。局部更正只撤回指定字段：折纸赛和迷宫画页保留旧来源对未改字段的支持；拼句信箱明确撤回两份旧说明全部投稿内容，旧来源即使有相同答案也不计为当前支持。不能仅凭“新”“旧”、文件位置或来源排序臆造更正。

不可读来源的 `text` 只保留可见目录/邮件标题及“正文不可读”，不含隐藏答案，不进入支持或冲突组。`unrelated_unavailable` 的不可读资料属于其他对象或事项，不可把真正缺失的目标信息升级为unknown。引用式指令只是来源内容，没有控制任务的权限。

## 预选清单

| case_id | variant | 独立任务 | compare_luna | prepared_control |
| --- | --- | --- | --- | --- |
| bc2fa0e7d65c | complete | 纸卡拆成两项待办 | 是 | 否 |
| f75e3881cdf4 | complete | 拼图贴纸含运费总价 | 是 | 否 |
| b35a01ab549b | complete | 小册试装朝向与订钉边 | 否 | 是 |
| a2054acd28b1 | complete | 月洞棋开局 | 否 | 否 |
| 8527ae0b0af4 | missing | 音频第二段剪辑区间 | 是 | 否 |
| 185a5bb98c0a | missing | 两轮方格翻转流程 | 是 | 否 |
| 2e0d9e7d2e9c | missing | 纸飞机交流接班提醒 | 否 | 是 |
| 883ee7244683 | missing | 像素册封面规格 | 否 | 否 |
| dd96df05028b | conflict | 解谜最终答案提交 | 是 | 否 |
| ef4318be0f32 | conflict | 闻香盲测摆杯 | 是 | 否 |
| 968db5953144 | conflict | 小游戏得分表导出 | 否 | 是 |
| d31b9111425c | conflict | 合唱练习到场提醒 | 否 | 否 |
| 994eeac1b777 | unavailable | 拼字活动桌位 | 是 | 否 |
| 49a5236ad225 | unavailable | 彩石分类演示准备 | 是 | 否 |
| b1df440eb3ef | unavailable | 一页故事首页排版 | 否 | 是 |
| 025fdbcb47e0 | unavailable | 影子戏观众参与 | 否 | 否 |
| e461c0827ccf | resolved | 折纸赛开场提示 | 是 | 否 |
| 89cc1eafff30 | resolved | 词语接龙轮流回答 | 是 | 否 |
| f8b21498708a | resolved | 迷宫画页标记 | 否 | 是 |
| 5219bd7d95a5 | resolved | 拼句信箱投稿 | 否 | 否 |
| e10900bd57ae | unrelated_unavailable | 小火车展示乘看 | 是 | 否 |
| 38731928c73c | unrelated_unavailable | 天气本温度记录 | 是 | 否 |
| 489a488ffd48 | unrelated_unavailable | 响铃问答决胜轮 | 否 | 是 |
| ee3d33b72bc2 | unrelated_unavailable | 纸片拼地图工作台 | 否 | 否 |

## 作者验证记录

本轮归属 `E:/SoulSyntheticLab/task-open-flow-20261022/fixtures-author/`。执行者为本任务的fixture作者子Agent；日期为2026-10-22，时区America/Los_Angeles。起点由主线程提供：HEAD `6a56d852d23a051c7c4e9845642791295c541105`，dirty工作区；本子任务仅写本目录的 `cases.json`、`README.md` 及仓库外作者校验记录，不提交、不构建、不访问模型。

实际命令：

```pwsh
$ErrorActionPreference = 'Stop'
& 'E:/SoulSyntheticLab/task-open-flow-20261022/fixtures-author/validate-fixtures.ps1'
if ($LASTEXITCODE -ne 0) { throw "Fixture validation failed: $LASTEXITCODE" }
```

使用PowerShell 7.6.5。第1次退出码1：发现5处metadata字段不是连续原文片段，另有校验脚本将空needs误当作一个null状态。完整首轮报告保留为 `validation-attempt-01.json`。修正原文跨度和空数组计数后，第2次退出码0，完整报告为 `validation.json`。没有删除首次失败、重生ID或改变预选成员来提高结果。

第2次结构检查确认24个唯一ID、六类各4例、每例4–6来源、输入字段白名单、105条来源、50项需求（34 supported、8 missing、4 conflict、4 unknown）、9条不可读来源；验证所有引用存在且可读、原文跨度一致、4条更正句包含被替代ID、12个对比成员、6个对照成员以及唯一零需求例。作者逐例阅读检查了任务必要性、对象/日期/角色错配、最小支持路径与局部更正范围。仓库外记录仅在本机路径可访问，并未随仓库发布。

交付前另以 `git branch --show-current`、`git rev-parse HEAD`、`git status --short -- fixtures/synthetic-open-task-context` 以及PowerShell读取旧两批ID执行交付核对，退出码0。确认当前分支为 `codex/soul-rebuild-20261019`、HEAD与起点一致，本目录为新增未跟踪文件；24个新ID与旧两批无碰撞。核对输出保存为同目录外部证据 `delivery-check.json`。未对其他修改进行暂存或提交。

这些是材料作者校验，不是独立评审、模型表现、应用运行测试或正式验收。独立gold审查与模型调用由主线程另行安排。本批有限、短中文合成样本不能代表真实任务分布、长上下文或生产准确率；相似的纸面/社团日常任务占比较高，也未覆盖复杂多跳更正链。任何后续修正与模型结果应继续保留实际尝试、失败、缺测与运行范围，不据此声称全量功能通过。
