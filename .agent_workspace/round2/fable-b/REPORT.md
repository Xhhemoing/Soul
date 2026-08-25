MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 2 · fable-b · P0 管道债规格 + 图纠正接口 + 验收差距

产出三份规格 + 本报告 + NOTES，全部在 `.agent_workspace/round2/fable-b/`，未动任何他人目录，未 commit。
证据基础：R1-SYNTHESIS（BINDING）、round1/fable-b/DEFECTS、round1/gpt-sol-b/ATTACK_SURFACE、
PRODUCT_LOCK，以及 `origin/cursor/soul-goal1-7b1c` 的完整源码（`git show` 逐文件核对行号，未拷仓）。

## 1. PIPELINE_DEBT.md — 扇出与去重的 v0.1 处置

- **接口契约先行**：soul-algo 对拿到的互动日志负责打分，日志真实性归管道。给出 5 条输入前置条件、
  4 条输出后置条件和一张「症状→责任方」分派表，使 C8 消融与 ALGO_FROZEN 与管道状态解耦。
- **扇出（债 A）**：钉住机理（`commit.rs` L198-207 + 全文件无时间约束的 `speakers_by_conversation`
  L397-416，被 `import_to_graph.rs` 测试钉成规范）。评估删扇出/限窗/保留三案，v0.1 取
  **保留扇出 + 算法场合闩 + TieScore 分列（direct/group × out/in）+ 展示语分列 + 测试改义**；
  扇出行数的性能债（10⁶ 级 evidence）立案不展开。
- **本轮最重要的新发现**：场合闩若停留在 `any_direct` 布尔形态（fable-a 现规范），
  「群聊重度扇出 + 双方各一条一对一问候」仍到 Strong。已向 opus-a/gpt-sol 提出必做夹具
  `group_heavy_plus_one_direct_each_way`（期望 ≤ Moderate），并主张 Strong 的 count/days 门槛
  只消费 Direct 计数（与 gpt-sol-b ATTACK_SURFACE 的独立结论一致）。
- **去重（债 B）**：`sha256("soul.import.dedup.v1|source|external_id")`，存 store 内部索引
  （EventStore 加两个 trait 方法），零冻结契约变更；keep-first、遗忘不复活、回执报
  `duplicates_skipped`；六条 G/W/T 验收含 gpt-sol-b 探针转绿。external_id 两个解析器都已产出
  （Telegram `chat_id:message_id`、JSONL `id`），只是 commit 时被丢弃——修的是丢弃，不是发明新 id。

## 2. GRAPH_CORRECTION.md — 图纠正/锁定最小 API

- 两个函数：`correct_tie(relationship_id, band)` / `release_tie`，逐步对称于 `correct_axis`
  （UserCorrection 证据 Strong → 边加 `locked_by_user`/`user_band`/`machine_band` → inference
  `user_verdict=Corrected` → 审计复用 `ProfileCorrect`）。零 schema 变更（`tie_strength` 是自由 object，
  `Corrected` 枚举已存在）。
- **关键决定**：`TieStrength.band` 恒为生效档（锁定时=用户档），机器档另存 `machine_band`。
  这样 WP10/view/起草等全部现有消费者零改动即自动尊重纠正——反向设计要求每个消费者记得查锁，
  漏一处锁就形同虚设。
- rebuild 规则：锁定边计数照更新、band 不动、`user_verdict` 一律保留（修 `build.rs` L322 的复位）；
  遗忘压过锁；支撑清零的锁定边以纠正证据行为唯一证据存活（满足 `minItems:1` 且可解引用）。
- 10 条 G/W/T 验收 + 四句中文话术草案（冻结权归 fable-a）+ 明确不做清单（不造边、不改计数、
  不合并联系人）。体量核算 ~200 LOC，证明 v0.1-small。

## 3. ACCEPTANCE_GAP.md — ALGO_FROZEN 差距

主线全交付后仍有 9 个缺口，5 个阻塞冻结（消融裁决+墓碑、Strong 计数列问题、TieScore 定型、
话术冻结、as_of 定义冲突），4 个只阻塞 v0.1 端到端验收（图纠正实现、管道债实现、管道侧测试、A1 落判）。
其中两项是本轮新钉出来的：

- **as_of 定义冲突**：我自己 R1 DEFECTS 写「导出文件最大时间戳」，fable-a 与 SYNTHESIS 写
  「store 内最大时间戳」，在「后导入旧文件」时结果不同。裁决建议：store 级（单调、与导入顺序无关），
  我的 R1 措辞作废，须进冻结常量表并配「后导入旧文件不改档」测试。
- **A1 空转的算死**：v0.1 轴向证据只有 Questionnaire 与 UserCorrection 两种 kind；纠正不走升档路径；
  唯一触发是跨日重答同一份问卷 ≥3 次——这是 F16 同族错误在日界上的复发，不是三角化。
  建议独立性收紧为「≥3 组且 ≥2 种 kind」，则 A1 可证明永不触发，归档「正确但空转」，
  复活条件 v0.2 第二证据 kind 接入。附带把「保留 1–2 个」的清点收干净：人脉 1 + 档案/人事 1。

另核实一处 R1 遗留疑点：goal1 在位的 WP10（`soul-draft/src/analysis.rs`）**已经是**边 band 的纯消费者
（L244），「第二套阈值」风险实际在 opus-b Round 1 参考实现的 `sample_band`（其 REPORT F-2 自认）——
G3 grep 门必须覆盖参考实现目录或将其标注非规范。

## 4. 交接

- **opus-a / gpt-sol-a / gpt-sol-b**：夹具 `group_heavy_plus_one_direct_each_way` 必做（PIPELINE_DEBT §1.3）；
  TieScore 分列字段表（§1.4 M-A1）请在实现里定型。
- **fable-a**：话术清单九条（ACCEPTANCE_GAP §3），S3/S5/S7/S8/S9 为空白项。
- **opus-b**：`sample_band` 处置；`intake` 绕锁的生产路径测试。
- **父代理**：as_of 裁决、A1 裁决、审计枚举是否增补（`GraphCorrect` / `DuplicateImport`）三处拍板。
