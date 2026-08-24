MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 3 · fable-b · 终局交叉审计报告

产出三份 + 本报告，全部在 `.agent_workspace/round3/fable-b/`，未动他人目录，无 Rust。
证据基础：R2-SYNTHESIS（BINDING）、round2/fable-b/{PIPELINE_DEBT,GRAPH_CORRECTION,ACCEPTANCE_GAP}、
opus-a R2 消融、gpt-sol-a/b R2 报告、fable-a R2 协议与话术、以及 Round 3 已到货件
（gpt-sol-a 矩阵、gpt-sol-b 探针、opus-a 在制源码）。

## 1. T4D_SPEC.md — T4 最终形态的冻结级精确规范

- **公式**：Moderate 与 Strong 的三道门（次数、天数、互惠）全部只消费 **direct 列**
  （`direct_reciprocal ∧ direct_count ≥ 10 ∧ direct_days ≥ 3` / `∧ direct_count ≥ 3`）；
  群计数存而不判（不开 Strong 也不开 Moderate）；降档时钟 = **任一场地**的 `last_contact`
  （昨天的群消息挡住休眠降档，夹具 X2 钉为有意行为，残余风险如实写死）；
  `≥180` 降一档、`≥360` 封 Weak，闭区间（父代理裁决）。
- **主夹具钉死**：`group_heavy_plus_one_direct_each_way`（200 人群扇出 10 条 × 3 日
  + 双向各 1 条直连）→ T4 = Strong ✗ / T4D = Weak ✓。两个 Round 3 独立实现已证实。
- **采纳条件核验通过**：`lilei_12` 在 T4D 下 Strong（两实现 + 手算一致）→ 按 R2-SYNTHESIS，
  T4D 即 T4 最终形态。相对 T4 恰改两格：#17 净胜（修 bug）、#5 group-only Moderate→Weak
  （D2 显式裁决，顺带关闭「全群 Moderate」残余），零回归、全部钉死期望满足。
- F04c 复燃限制**保全不偷修**（TD-9 反向钉住）；V1–V7 不变式 + TD-1…TD-10 验收测试成表。

## 2. SOTA_ACCEPT.md — 对产品锁的剩余差距，逐项 pass/fail

- **算法内容 PASS；ALGO_FROZEN 宣告 FAIL（尚不可）；v0.1 端到端 FAIL。**
- 本轮关闭的内容性缺口：T4D 夹具、Strong 消费哪列、TieScore 分列定型、as_of、闭区间、A1 裁决。
- 冻结前剩余三件**纯机械**工事：墓碑文件（须含 T4 原形态条目、双格披露）、
  中文模板 T4D 差分冻结 + 渲染断言（分列句因判档改直连而从建议升为必须）、统一 `crates/soul-algo`
  （仓内尚无 `crates/` 目录）。
- 产品锁唯一实现级 FAIL 面：图纠正未实现（规格完备；§2/§3 语义须并入冻结文档）；
  管道债（去重等）维持「阻塞 v0.1 验收、不阻塞冻结」归类。

## 3. CROSS_CHECK.md — 三方矛盾清单

- **唯一实现级真矛盾 C1**：gpt-sol-b R2 的 `>180`（恰 180 不降）vs 其余三方 `≥180`；
  漂移源头是 R1-SYNTHESIS 同一行混用「>180d / ≥360d」。父代理裁决 `≥180`/`≥360` 闭区间，
  gpt-sol-b R2 版作废；Round 3 三方已同口径钉死 179/180/359/360 四格。关闭。
- **非矛盾**：opus-a「T4 胜」vs gpt-sol-a「平局」是夹具集覆盖差（7 条必测集恰在一致区，
  重叠格逐格核对全等）；as_of 各文口径已收敛。
- **口径漂移（已裁决）**：T3R 单位（gpt-sol-a 四分之一单位误标 milli vs gpt-sol-b 真 milli，
  档位同构，T3R 已进墓碑）；主夹具三种参数化（形状与结论同一，规范参数化已钉）；
  F04c 判卷归属（不在 opus-a TRUTH 集分母内，父代理已定性为已知限制）。
- 留痕义务一条：REJECTED.md 的 T4 条目必须同时披露 #17 净胜格与 #5 改档格。

## 4. 交接

- **父代理**：T4D_SPEC 可直接并入冻结文档；ALGO_FROZEN 只差 §4 三件机械工事；
  CROSS_CHECK 结论=Round 3 无新矛盾进入冻结。
- **fable-a**：COPY_ZH 按 T4D_SPEC §9 差分定稿（一对一口径句、分列句、group-only 弱档句）。
- **opus-a（在制）**：其 `gate.rs/recency.rs/constants.rs` 与本规范逐条相符（含 T4D 无天花板
  常量、闭区间、共用降档层）；完工后跑 TD-1…TD-10 即可作为规范参考实现候选。
