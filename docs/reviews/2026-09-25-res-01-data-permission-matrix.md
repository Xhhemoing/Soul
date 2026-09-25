# RES-01：当前字段、用途许可与研究问题矩阵

日期：2026-09-25。状态：**DONE（只读阶段评测）**。

## 1. 审查范围与证据等级

- 文档审查 HEAD：`825b0f110b6d79dec088f6a21aa529c9efc80c1f`。
- 被审研究实现版本：`ebff0c96325e0f297a1c397de6a3a1421fdd369d`。
- 对第 9 节列出的具体路径执行两个提交树之间的内容比较，`git diff --name-only` 无输出。两提交不是祖先关系；该结果只证明所列路径的 blob 内容相同，不表示线性继承、整棵产品树相同或新增了冻结 baseline。
- 审查方式：源码、schema 与测试源码静态核验；没有读取真实用户库，没有使用真实聊天、姓名、账号、密钥或个人样本。样本窗口：不适用。
- `[V-source]` 表示源码或 schema 可直接确定；`[V-test-source]` 表示当前测试源码覆盖该断言，但本轮未重新执行；`[C]` 表示实现事实与用途许可之间存在需要保留的张力；`[T]` 表示尚未取得运行或真实使用证据。

本文只判断当前数据形状能支持什么，不修改产品代码、schema、IPC、许可逻辑、PRODUCT_LOCK、Goal 1 验收或既有 gate。

## 2. 判定摘要

1. `[V-source]` 事件路径只有分组后的 `self/owner + bucket` 候选能进入预览；输出只包含事件类别、时间桶和聚合计数。
2. `[C]` 档案轴路径由实现直接合成 `Owner + Bucket`，没有读取 profile、axis 或 evidence 的研究许可；当前内置生产 evidence 构造点均写 `exportable_to_research:false`。因此“实现会输出”不能改写为“已获得独立研究许可”。
3. `[V-source]` `duration_bucket` 是契约预留字段，当前生产研究预览没有赋值路径。
4. `[C]` `ConsentTopic::ResearchPreview` 已定义，但 `Session::research()` 没有检查或授予它。当前产品状态不能证明独立研究同意已记录。
5. `[V-source]` 当前输出没有采集覆盖、设备在线、可用时段或缺测标志，且 Session 只展示筛选后的前 50 行。因此当前数据不足以启动 RES-03、形成有效时长分母或区分“无活动”和“未采集”。

## 3. 事件聚合路径

### 3.1 查询实际读取面

| 事件字段 | 是否被当前查询读取 | 对准入或输出的实际作用 |
|---|---|---|
| `kind`、`ts` | 是 | 生成事件类别、时间桶并参与分组；不是研究许可证明 |
| 存储列 `privacy_subject` | 是 | 区分 Owner、ThirdParty 与 Machine，是 subject 准入门之一 |
| 文档内 `privacy.egress.research_export` | 是 | 只有字符串 `bucket` 通过 disposition 门 |
| `privacy.purposes` | 否 | 即使数组包含 `research`，当前查询也不核验，不能替代独立研究同意 |
| `consent_id` | 否 | 当前查询不核验事件是否关联 consent 记录 |
| `privacy.derivation` | 否 | `raw`、`derived`、`aggregate` 不参与当前准入判断 |
| `actor_subject`、`source` | 否 | 不参与 subject/disposition 准入；不得据此推定采集来源已获研究许可 |
| `body_ref` | 否，且不解密 | 应用身份、正文和可能存在的实际时长不会进入研究行 |
| `event_id`、`schema_version`、`privacy.retention`、`privacy.egress.e0/e1` | 否 | 不参与当前研究预览查询 |

### 3.2 输入状态与排除矩阵

| 输入状态 | 当前实现处理 | 用途判定 |
|---|---|---|
| `privacy_subject = self` 且 `research_export = bucket` | 分组后可进入预览 | 只是行级技术准入；独立研究仍需另有可核验的研究同意 |
| `privacy_subject = third_party` 或 `mixed` | 排除，并按聚合后的候选组计入 `third_party_rows_excluded` | 不得用于研究预览；没有一次性正文豁免 |
| `privacy_subject = system`、未知值或其他值 | 归为 Machine，排除 | 不得把未知 subject 当作 owner |
| `research_export = deny`、`hash`、`allow` 或缺失 | 归为 Withheld，排除 | v0.1 只实现 bucket 形状，不猜测其他 disposition 的含义 |
| `research_export` 是无法按字符串读取的非法 JSON 类型 | 查询可能失败，而不是正常形成排除计数 | fail closed；不得声称所有非法值都会被正常计数 |

`third_party_rows_excluded` 与 `deny_rows_excluded` 是 SQL 聚合之后的候选组数量，不是原始事件数，也不是受影响人数。

运行期报告只提供粗粒度计数：system/未知 subject 被排除但没有单独计数；`deny`、`hash`、`allow` 与缺失 disposition 都折叠为 Withheld，只有 Owner + Withheld 组进入 `deny_rows_excluded`。各规则可以从源码逐项追踪，但不能从报告数值反推出每一类 disposition 或 subject 的数量。

### 3.3 研究行字段矩阵

| 当前字段 | 当前生产形状 | 在另有合法研究同意后可支持的问题 | 当前禁止或不支持的问题 |
|---|---|---|---|
| `event_kind` | 事件 `kind` 的聚合标签 | 描述已发布预览行中出现了哪些获准事件类别 | 具体应用、进程、正文、姓名、账号、联系人；预测下一次打开的应用 |
| `time_bucket_utc` | 结尾为 `Z` 的时间戳形成 UTC 小时桶；其他带偏移时间戳退化为日期字符串 | 描述已发布行发生于哪个小时桶或退化日期桶 | 精确时间、单条事件顺序、连续活动轨迹；把所有值都解释为 UTC 小时 |
| `aggregate_count` | 同 kind、时间桶、subject、disposition 的计数 | 描述某个已发布聚合组的事件数量 | 人数、实际使用时长、完整频率或发生率；把 count=1 当作匿名性保证 |

允许的问题只针对实际发布的预览行。由于截断和缺少覆盖字段，不能将未出现的桶解释为没有活动，也不能从预览计算总体发生率、长期趋势或完整时间序列。

## 4. 档案轴派生路径

| 当前字段 | 当前生产形状 | 当前许可状态 | 可支持的内部检查 | 当前禁止或不支持的问题 |
|---|---|---|---|---|
| `self_trait_axis` | 读取 profile 中的稳定 `axis_id`；仅与有效 evidence band 成对输出 | 实现直接标记 `Owner + Bucket`，未读取轴或 evidence 的研究许可 | 检查预览是否按契约产生轴 ID 行 | 不得据此声称独立研究许可充分；不得研究他人档案 |
| `self_trait_band` | 只接受 `weak`、`moderate`、`strong`；`none`、缺失或非法值跳过 | 当前内置生产 evidence 构造点均为 `exportable_to_research:false` | 检查档位筛选和输出形状 | 不得解释成人格强度、人格真值、临床结论或科学有效性；不得建立时间趋势 |

实现只从 profile 轴对象读取 `axis_id` 与 `evidence_band`；不读取 `position`、`label`、`evidence_ids`，也不加载 evidence 文档的 `privacy` 或 `exportable_to_research`。预览不直接输出问卷答案、纠正内容、evidence ID 或来源。不过轴 UUID 是稳定常量，拥有源码映射的人可以恢复轴标签和两端语义。因此“不得研究轴标签或方向”是用途与解释禁令，不能描述成技术上无法识别。

## 5. `duration_bucket`

| 项目 | 判定 |
|---|---|
| schema/API | `[V-source]` `export-manifest`、Rust 模型和 Session 视图允许该字段 |
| 当前生产路径 | `[V-source]` 事件与档案轴构造均未赋值；当前没有生产填充路径 |
| 可研究问题 | 当前无；只能记录“契约预留” |
| 禁止解释 | 缺失不表示时长为零，不表示未使用，也不能用于时长分布、留存或参与度分析 |

## 6. 控制与证据字段

以下字段用于描述预览和过滤是否发生，不是研究变量，也不能直接作为样本量、人数或事件数。

| 控制字段 | 当前含义 | 禁止解释 |
|---|---|---|
| `manifest_id` | 本次预览的 UUID | 用户 ID、样本 ID 或稳定实验对象 ID |
| `export_kind` | 当前为 `research_preview` | 已生成正式研究导出 |
| `fields` | 截断后保留行里实际出现的研究行字段清单 | 完整数据字典、完整数据集或未列字段永久不存在 |
| `third_party_rows` | 构造器核验后的输出行计数，必须为 0 | 库中没有第三人数据或查询没有发现第三人候选 |
| `written_to_disk` | 研究预览强制为 `false` | 其他产品数据从未落盘 |
| `candidate_rows_total` | 过滤前候选数量：事件是 SQL 分组后的候选组，档案轴是一轴一候选 | 原始事件数、人数、用户数或统计样本量 |
| `third_party_rows_excluded` | 被 subject 门排除的 ThirdParty 候选组数 | 原始第三人事件数、第三人人数或全部非 Owner 候选数 |
| `deny_rows_excluded` | Owner 且非 Bucket 的候选组数 | 各 disposition 的逐类计数；Machine/ThirdParty 的 withheld 计数 |
| `third_party_body` | Session 视图固定为 `excluded` | 正文已经匿名化后可用于研究 |
| `redaction_profile.one_time_override_allowed` | 当前 manifest 默认省略；schema 若出现只允许 `false`，研究路径没有正文豁免 | 可沿用 E1 的单条正文豁免 |
| `notice` | 固定的界面说明文字 | 运行期测量值或额外许可证明 |

## 7. 共同边界

- `[V-source]` `ResearchPreviewRequest::default()` 与 Session 界面使用 50 行；筛选后按事件候选在前、档案轴在后的顺序截断。`with_max_rows` 可由其他 Rust 调用方改写，因此 50 是默认值和当前 Session 限制，不是全系统硬上限。
- `[V-source]` `written_to_disk=false` 由构造器和 schema 强制；本文没有创建研究导出文件。
- `[V-source]` 研究行允许字段只有 `event_kind`、`time_bucket_utc`、`duration_bucket`、`self_trait_axis`、`self_trait_band`、`aggregate_count`；第 6 节控制字段不属于研究变量。当前没有覆盖率、在线状态、设备状态、可用时段或 missingness 字段。
- `[V-test-source]` 当前 store 与 Session 测试源码覆盖第三人/混合/deny 排除、owner bucket 聚合、轴行、`written_to_disk=false` 和目录不变化。本轮没有重新执行这些测试，不能把测试源码存在写成新的运行通过证据。
- 预测下一应用、心理或人格推断、临床判断、他人行为、长期趋势等禁令当前不是代码级“查询用途拦截”；它们是本矩阵记录的规划用途和解释边界。

## 8. 下游决定

| 工作包 | 本轮决定 |
|---|---|
| RES-01 | **DONE**：完成当前字段、用途许可、可研究问题和禁止问题矩阵 |
| RES-03 | **NOT STARTED / DATA INSUFFICIENT**：没有覆盖、在线状态和有效时长分母，不启动预测建模 |
| DATA-01 | 若未来增加覆盖、缺测或研究导出字段，先执行数据最小化、用途许可和兼容性决议；RES-01 不预先授权扩字段 |
| 产品许可 | 保留 `ResearchPreview` consent 未在 Session 入口执行的事实；本轮不擅自定性为新缺陷，也不修改 v0.1 边界 |

本文不关闭 Goal 1、Q2-00 或正式 Goal 2，不替代 G-L、G-M 或任何真实用户观察。

## 9. 证据索引与本轮验证

主要源码与契约：

- `crates/soul-store/src/research_preview.rs:51-81, 92-145, 149-261, 265-305`
- `crates/soul-store-api/src/research.rs:28-50, 88-131`
- `crates/soul-schema/src/export_manifest.rs:19-43, 46-126`
- `crates/soulcore/src/commands/session.rs:1503-1513`
- `crates/soulcore/src/commands/store.rs:89-175`
- `crates/soul-policy/src/consent.rs:22-58, 90-154`
- `crates/soul-profile/src/axes.rs`
- `crates/soul-profile/src/service.rs:352-367`
- `crates/soul-import/src/questionnaire.rs:351-363`
- `crates/soul-graph/src/interaction.rs:144`
- `crates/soul-graph/src/correct.rs:288`
- `docs/schemas/_defs.schema.json:26-60`
- `docs/schemas/event.schema.json:7-46`
- `docs/schemas/export-manifest.schema.json:19-72`
- `docs/schemas/profile.schema.json:12-33`
- `docs/schemas/evidence.schema.json`

相关测试源码：

- `crates/soul-store/tests/research_preview.rs`
- `crates/soulcore/tests/session_research.rs`
- `crates/soul-policy/tests/consent.rs`

版本等价比较使用的 PowerShell 路径清单如下；命令无输出且退出码为 0：

```powershell
$paths = @(
  'crates/soul-store/src/research_preview.rs',
  'crates/soul-store-api/src/research.rs',
  'crates/soul-schema/src/export_manifest.rs',
  'crates/soulcore/src/commands/session.rs',
  'crates/soulcore/src/commands/store.rs',
  'crates/soul-policy/src/consent.rs',
  'crates/soul-collect/src/collector.rs',
  'crates/soul-import/src/commit.rs',
  'crates/soul-import/src/questionnaire.rs',
  'crates/soul-profile/src/service.rs',
  'crates/soul-profile/src/axes.rs',
  'crates/soul-graph/src/interaction.rs',
  'crates/soul-graph/src/correct.rs',
  'docs/schemas/_defs.schema.json',
  'docs/schemas/event.schema.json',
  'docs/schemas/evidence.schema.json',
  'docs/schemas/profile.schema.json',
  'docs/schemas/export-manifest.schema.json',
  'crates/soul-store/tests/research_preview.rs',
  'crates/soulcore/tests/session_research.rs',
  'crates/soul-policy/tests/consent.rs'
)
git diff --name-only ebff0c96325e0f297a1c397de6a3a1421fdd369d 825b0f110b6d79dec088f6a21aa529c9efc80c1f -- $paths
```

本轮实际执行：Git 提交树与工作树查询、上述输入文件内容比较、相关字段与 consent 引用搜索，以及三项独立只读规格、源码证据和文档一致性审查。初审发现项修复后，三路最终复核均为 **PASS，0 Critical / 0 Important**。

尝试运行的目标测试命令为：

```powershell
$ErrorActionPreference = 'Stop'
cargo test -p soul-store --test research_preview
cargo test -p soulcore --test session_research
cargo test -p soul-policy --test consent
```

命令只观察到 `Compiling openssl-sys v0.9.117`，随后多次轮询无新增输出，由审查者向 PTY 发送 Ctrl+C 中断。未取得退出码，三个测试均未形成通过或失败结果；本轮运行状态因此记为 **NOT RUN / 无新增通过证据**，不能写成测试失败或通过。
