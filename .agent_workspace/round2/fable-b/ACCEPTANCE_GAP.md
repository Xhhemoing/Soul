# ACCEPTANCE_GAP — Round 2 结束后距离 ALGO_FROZEN 还差什么（fable-b）

判定基准：fable-a ACCEPTANCE.md 的五条「保留」定义与 G1–G8 验收门（本文沿用其编号），
加上 R1-SYNTHESIS 的五条攻坚项。「Round 2 结束后」按 PROGRESS.md 编制假定主线交付：
opus-a 的 T3/T3R/T4 实现与消融、gpt-sol-a 独立基准、gpt-sol-b 探针转绿、opus-b 的 A0 锁 + A1 + A2、
fable-a 的消融标准与话术冻结、本槽的两份规格。

结论先行：**即使主线全部交付，仍有 9 个缺口；其中 5 个阻塞 ALGO_FROZEN 宣告，4 个只阻塞 v0.1 端到端验收。**

---

## 1. 缺口总表

| ID | 缺口 | 阻塞 ALGO_FROZEN | 阻塞 v0.1 验收 | 建议归属 |
|---|---|---|---|---|
| GAP-1 | C8 消融裁决未落判 + 墓碑文件不存在 | **是** | 是 | Round 2 末父代理 / Round 3 |
| GAP-2 | 胜者规范未回答「Strong 消费哪列计数」（any_direct 布尔闩 vs direct-only 计数） | **是** | 是 | opus-a + gpt-sol-b（夹具见 PIPELINE_DEBT §1.3） |
| GAP-3 | TieScore 分列字段未定型（direct/group × out/in） | **是** | 是 | opus-a 定型，本槽已给规格 |
| GAP-4 | 中文话术清单未冻结、无渲染断言 | **是** | 是 | fable-a（清单见 §3） |
| GAP-5 | as_of 定义冲突未钉死（store 级 vs 文件级） | **是** | 是 | 父代理拍板（PIPELINE_DEBT PRE-5 已给裁决建议） |
| GAP-6 | 图纠正未实现（P0-2，产品锁承诺） | 规格须并入冻结文档 | **是** | GRAPH_CORRECTION.md → Round 3 实现 |
| GAP-7 | 管道债未实现（去重、归一化、intake 绕锁、单向 DM） | 否（接口契约已隔离） | **是** | PIPELINE_DEBT / DEFECTS → Round 3 |
| GAP-8 | 测试清单缺口（见 §2） | 部分（算法本体测试）| 是 | opus-a / gpt-sol-a / Round 3 |
| GAP-9 | A1 去留未落判（本文 §4 给出裁决建议：归档「正确但空转」） | **是**（占不占名额必须定） | 否 | 父代理采纳 §4 即可关闭 |

G1–G8 对照：GAP-1 压 G2/G4，GAP-2/3 压 G1，GAP-4 压 G5，GAP-5 压 G7，GAP-8 压 G1/G6，
GAP-9 压「保留 1–2 个」的清点本身。G3（单源阈值 grep）与 G8（不越权）预计主线交付即满足，
但 G3 的 grep 范围必须把 opus-b Round 1 参考实现里的 `sample_band`（自带 3/10/3，其 REPORT F-2 自认）
划为非规范或删除——goal1 在位的 `soul-draft/src/analysis.rs` 已验证是纯消费者
（L244 直接读 `edge.tie_strength.band`，无本地阈值），风险不在仓库主干，在参考实现被误当规范。

---

## 2. 缺失测试清单（GAP-8 展开）

算法本体（阻塞冻结）：

1. `band()` 参数化边界表：count 2/3、9/10；days 2/3；「单日 20 条互惠 → Moderate」立法场景
   （`build.rs` L41-44 注释至今零断言，Moderate 档整档零覆盖 —— DEFECTS P2-1）。
2. 胜者场合闩：`group_only_50 → ≤ Moderate`；`one_sided_100/1000 → Weak`；
   **新增** `group_heavy_plus_one_direct_each_way`（GAP-2 的裁决夹具）。
3. 若 T3R/T4 胜出：衰减/降档边界 Δd = 89/90、179/180、359/360（T3R 桶界）或 179/180、359/360（T4 阶跃）；
   as_of 取 store 级最大时间戳、禁读墙钟的确定性断言（G7）；「后导入旧文件不改变既有边档位」用例
   （直接检验 GAP-5 的裁决）。
4. `+08:00` 偏移时间戳夹具（P1-6：现库零覆盖，字典序比较与 UTC 日切在偏移时间戳上双双失效）。

管道与政策（阻塞 v0.1 验收）：

5. 重复导入幂等六件套（PIPELINE_DEBT §2.3 PD-B1…B6，含 gpt-sol-b 探针转绿）。
6. 图纠正十件套（GRAPH_CORRECTION §6 GC-1…GC-10）。
7. `intake` 绕锁回归（P1-1）：纠正锁定后重答问卷，轴不动、回执点名被跳过的轴——
   opus-b 参考实现有对应用例，goal1 生产路径 `correction_lock.rs` 还没有。
8. 遗忘→rebuild→无陈旧边（P2-6 + gpt-sol-b 边界观察 #4：rebuild 从不删边）。
9. WP10 纯消费者 grep 门（G3）：`STRONG_MIN|MODERATE_MIN|半衰|桶权重` 只允许出现在 `soul-algo` 一处。

---

## 3. 中文话术冻结清单（GAP-4 展开，冻结权归 fable-a）

每条必须：进规范文档、有渲染测试匹配模板结构、过 `assert_non_clinical`、只谈
次数/天数/是否私聊/是否互惠 + 三档词。

| # | 字符串 | 现状 |
|---|---|---|
| S1 | 胜者三档判定解释（T3R 的桶折算句或 T4 的降档句） | fable-a CANDIDATE_SPEC 有草稿，未随胜者定稿 |
| S2 | 群聊封顶句（「只在群里见过…最多算中等」） | 草稿有，未冻结 |
| S3 | 计数分列句（「一对一 X 次、群里同场 Y 次」，PIPELINE_DEBT M-A2） | **无** |
| S4 | 休眠/降档句（「最近半年没有往来…」，T3R/T4 胜出才需要） | 草稿有 |
| S5 | 图纠正四句（GRAPH_CORRECTION §4） | 本轮新草稿 |
| S6 | 轴锁拒用句（「来自你本人的纠正，已锁定…」） | 草稿有 |
| S7 | 重复导入回执句（「同一条消息导入两次只算一次，本次跳过 N 条」） | **无** |
| S8 | tie falsifier 由常量拼装（P2-7：现为不含阈值的硬编码散文） | **无** |
| S9 | WP10 P5 档位句 + 「由你本人指定」变体 | 前者在位，变体**无** |

---

## 4. A1 的 v0.1 空转判定（GAP-9 展开，建议直接采纳为裁决）

R1-SYNTHESIS 留的口子是「若 v0.1 只有问卷+纠正则可能空转」。把它算死：

- v0.1 能产生**轴向证据**的来源只有两种 kind：`Questionnaire` 与 `UserCorrection`
  （`Message` 类证据只进图管道；A3 正文推断已枪毙且代码层恒拒绝）。
- `UserCorrection` 走 `correct_axis`，无条件 Strong + 锁——它根本不经过 A1 的升档路径。
- 于是 A1 唯一可能的触发是**跨日重答问卷 ≥3 次且同向**。按 fable-a 规范的独立性键
  `(kind, utc_date)`，这确实会升 Strong——但这是同一份七题量表重复作答，不是独立佐证；
  把「换了三天答同一份卷」当成三角化，恰是 F16 想挡的同族错误在日界上的复发。
- **裁决建议**：独立性定义收紧为「≥3 组且 ≥2 种 kind」（fable-b R1 INNOVATION #5 原案）。
  收紧后 v0.1 非纠正 kind 只有 1 种 ⇒ A1 **可证明地永不触发** ⇒ 归档「正确但空转」，
  墓碑写复活条件：v0.2 行为证据（前台使用时长聚合、图计数派生等第二 kind）接入时原样复活，
  opus-b 的 72 测参考实现随墓碑封存为复活时的验收基线。
- 附带收益：A0+A1 不再占用「1–2 名额」的讨论席位，终态清点干净：
  **人脉 1 个（T 族胜者）+ 档案/人事 1 个（A0 政策基底 + A2 纯渲染，二者合计一个名额）**。

---

## 5. 宣告 ALGO_FROZEN 前的最后一张检查单

1. C8 消融裁决书（净胜夹具列表 + 零回归证明）+ `docs/algorithms/REJECTED.md` 覆盖
   T0/T1/T2/A3 + 消融落败者 + A1（按 §4）。
2. 胜者规范里写死：Strong 消费的计数列（GAP-2 裁决）、TieScore 分列字段表、as_of = store 级最大
   `occurred_at`、全部常量单源表。
3. GRAPH_CORRECTION 的 §2/§3 语义（生效档在 `band` 字段、rebuild 保留 user_verdict）并入冻结规范——
   实现可以 Round 3 落地，但语义不冻结则「胜者输出的 band 是谁的 band」都没有答案。
4. PIPELINE_DEBT §0 接口契约进 `docs/algorithms/`，管道债票据（去重/归一化/单向 DM/性能）挂
   v0.1 验收而非算法冻结。
5. §3 话术九条随规范冻结，渲染断言绿。
6. §2 清单第 1–4 项测试在参考实现上绿（Linux、无网络、无墙钟）。
