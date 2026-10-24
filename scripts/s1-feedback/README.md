# S1 本机反馈准备工具

周期 1 W1-4 的独立工具，依据战略 §4.3、§4.4、附录 B。只依赖现有 Node.js；不接产品、D107 留存、数据库、网络或自动发送。本目录的工具输出模板与汇总，本身不授予真实使用权限，也不负责自动删除。

2026-10-24 作者已批准一次本机 T1 试用及真实反馈：仅本机产品评估、不存正文和联系人关联、保存 30 天后整批删除，相关遗忘整批删除。该批使用仓库外的本机终端与限定批次清理器，反馈不经聊天上传；[作者操作说明](E:/SoulFeedback/s1-author-trial-20261024/AUTHOR-STEPS.md)提供显式入口，串行准备批次、注册并读回核对到期任务，成功后才进入录入。实际执行状态及未完成项见[战略 §13.6](../../docs/plans/2026-10-23-long-term-strategy.md#136-作者本机-t1-试用与反馈留存2026-10-24)。这一具体授权不扩大到任意真实会话、外部模型或研究用途；工具存在或合成验证通过均不代表真实批次、系统任务或到期删除已经执行。

## 记录格式与模板

JSONL：每个批次一行 `batch_start`、每次开始的任务一行 `task`、结束时一行 `batch_end`。批次可交错，必须各自闭合；不接受重复任务、重复批次、无开始/结束或超出批次时间的任务。时间使用含时区的 ISO 8601，模板默认 UTC `Z`；可填写例如 `2026-10-23T10:00:00-07:00`。`task.recorded_at` 是任务开始时间，填写反馈时保留它。取消任务也保留并填写结果。

以下命令只输出模板，不写文件。批次 UUID 从 start 的输出复制，task/end 不接受对象 ID：

```pwsh
$ErrorActionPreference = 'Stop'
node scripts/s1-feedback/template.mjs start
if ($LASTEXITCODE -ne 0) { throw 'Template failed' }
```

```pwsh
$ErrorActionPreference = 'Stop'
node scripts/s1-feedback/template.mjs task <batch_uuid> T1
if ($LASTEXITCODE -ne 0) { throw 'Template failed' }
node scripts/s1-feedback/template.mjs end <batch_uuid>
if ($LASTEXITCODE -ne 0) { throw 'Template failed' }
```

在实际开始时生成 task；模板默认 `not_generated`，完成后在同一行更新实际结果，不能把默认值当成已评反馈。`task_id` 和 `batch_id` 是随机 UUID v4，不是匿名化，也不得复用为人物、来源或产品记录 ID。作者选择仓库外本机文件保存输出；仓库不存逐任务记录。

`task` 严格要求以下字段；所有字段必须存在，不允许备注或其他键：

| 字段 | 允许值/含义 |
| --- | --- |
| record_type / task_id / batch_id / recorded_at | `task` / 随机任务 UUID / 随机批次 UUID / 开始时间含时区 |
| requested_mode | `T1`、`T3` |
| actual_source | `template`、`endpoint`、`none` |
| fallback_reason | `无`、`端点未配置`、`审批不匹配`、`网络拒绝`、`回复不可读`、`回复为空`、`其他` |
| origin / scope_used | `requested`、`organic` / boolean |
| outcome | `not_generated`、`abandoned`、`rejected`、`heavy_edit`、`light_edit`、`accepted`、`no_feedback` |
| content_acceptable | `yes`、`no`、`unassessed`、`not_applicable`；T1 和 template 均为不适用 |
| severe_error | `none`、`fact_error`、`commitment_or_boundary_reversal`、`relationship_error`、`privacy`、`unassessed`；未评不能填写 none |
| edit_ratio | [0,1] 实测数字或 `null`；没有草稿必须 null |
| task_seconds | ≥0 实测数字或 `null`；选定消息到定稿，含审批、等待、修改，不含标注 |
| correction_status | `none`、`d109_covered_new`、`d109_covered_repeated`、`unmemorable` |
| correction_opportunity | `yes`、`no`、`unassessed`；D109 已设偏好后的再次适用机会，无复发也填 yes |
| failure_layer | `感知`、`理解`、`召回`、`辅助`、`校正`、`平台`，或 `null` |
| failure_type | `缺信息`、`理解错`、`语气不像`、`召回漏`、`召回错`、`事实错误`、`承诺或边界反转`、`关系误判`、`隐私问题`、`卡顿`、`崩溃`、`操作负担过重`、`其他`，或 `null` |
| severity | 1 轻微、2 需重写、3 发出会造成实际问题，或 `null` |

失败层/类型/严重度必须同时填写或同时为空。T1 不能使用 endpoint；实际 endpoint 的 fallback_reason 必须为“无”，scope_used 必须 false。T3 回退 template 须写具体回退分类。`none` 只配 `not_generated` 或 `abandoned`，没有草稿不得填写编辑比例。`d109_covered_repeated` 必须对应机会 yes。

`batch_start` 只有 record_type/batch_id/recorded_at；`batch_end` 加上 `recording_seconds`（本批填写评测的独立实测总时间或 null）。业务时间与记录时间分开；缺测不记零。空批次合法，未结束批次拒绝汇总，补真实结束行后再运行。

## 编辑比例：正文仅经 stdin

```pwsh
$ErrorActionPreference = 'Stop'
node scripts/s1-feedback/edit-ratio.mjs
if ($LASTEXITCODE -ne 0) { throw 'Edit measurement failed' }
```

按提示先粘贴草稿，以独占一行的 `.end` 结束，再粘贴终稿并用 `.end` 结束。交互时隐藏输入回显；stdout 只输出比例数字。正文不得放入命令参数、脚本、文件或日志。Ctrl+C 取消不会输出比例。正文含独占一行 `.end` 时，本协议无法表示，应不测并保留 null，不改写正文求得假测量。

公式是 Unicode **码点** Levenshtein 距离除以两文码点长度最大值；空空为 0。CRLF/LF 在粘贴时统一为换行 LF，不裁剪空白、不做 Unicode 规范化；组合字符按各自码点计数，非字素簇。限制每段 20,000 码点且长度乘积 ≤25,000,000，超限保留缺测。脚本不写正文或建立历史文件；进程退出释放引用不代表安全擦除内存，剪贴板和外部终端录制也不由脚本清理。

## 汇总与合成演练

汇总只读显式传入的 stdin，不扫描目录、不默认读取记录、不接受路径参数。错误仅输出固定 `INVALID_INPUT`，不会打印非法键、值、正文、UUID 或路径。输入上限 8 MiB。输出的值只含数值/null，键只含固定字段与固定分类标签，无时间与行标识。

```pwsh
$ErrorActionPreference = 'Stop'
# 下列路径只保存本工具生成的合成元数据。
node scripts/s1-feedback/synthetic-demo.mjs | Set-Content -LiteralPath E:\SoulSyntheticLab\cycle1-s1-20261023\feedback-synthetic.jsonl -Encoding UTF8
if ($LASTEXITCODE -ne 0) { throw 'Synthetic fixture failed' }
Get-Content -LiteralPath E:\SoulSyntheticLab\cycle1-s1-20261023\feedback-synthetic.jsonl -Encoding UTF8 -Raw | node scripts/s1-feedback/summarize.mjs
if ($LASTEXITCODE -ne 0) { throw 'Summary failed' }
node --test scripts/s1-feedback/feedback.test.mjs
if ($LASTEXITCODE -ne 0) { throw 'Feedback tests failed' }
```

`total` 保留开始、已生成、来源、反馈、所有 outcome、主动/验证使用次数、业务投入总量与失败分类；质量率和中位数仅放在 `strata[requested_mode][actual_source]`，不混算 T1/T3。

- `total` 与每个模式×来源分层的 `unclassified_task_count` 表示未填写失败层/类型/严重度的任务数量，包含满意任务或尚未分类的任务，不能推断它们都是失败。`failures` 仅汇总已填写的失败分类，不再输出 `failures.unclassified`；是否满意及各指标分母仍按原字段计算。
- 可接受率只含已生成、有反馈（rejected/heavy_edit/light_edit/accepted）且已评任务。T1 按 outcome；T3 仅 endpoint 且内容 yes/no。T3 template 的 outcome 保留，但内容不适用，不能算 T3 内容成功。未生成、放弃、无反馈分别计数；内容未评与不适用另列。
- 编辑比例仅汇总实测值，输出中位数和固定区间分布，以及缺测/未生成数量。业务时间输出实测中位数、总量与缺测；记录时间另按批次输出总量与缺测批次。
- 首稿严重错误率仅 actual_source=endpoint 且 severe_error 已评的任务，按具体类型分列，未评单列。确认时发现隐私泄漏而取消的 none/abandoned 可保留 privacy 与失败分类，计入 `non_endpoint_severe_events`，不进入端点首稿错误率。
- 纠正复发率分母是机会 yes 的任务（包括 status=none），分子为其中 repeated；机会未知单列、不进分母；无分母输出 null（N/A）。无法记住次数与失败类型、主动使用次数分别计数。

不记录联系人关联意味着不能按联系人定位删除。遗忘联系人或来源时，作者按含时区的批次开始/结束时间识别涉及的批次，删除该批次的 **全部 start/task/end 行**；可保留已输出的不含对象关联的汇总数字。工具不提供自动按联系人删除，也不自动删除文件。

该局部 Node 测试不覆盖 Rust、加密存储、Session、Tauri IPC、React 或 Windows 原生窗口/剪贴板验收；这些链路未被本工具修改。无依赖安装、锁文件、schema、产品字段或新门禁。
