MODEL_SLUG: claude-fable-5-thinking-xhigh

# CROSS_CHECK — Round 3 终局交叉审计：opus-a 消融 × gpt-sol-a 矩阵 × gpt-sol-b 边界（fable-b）

审计对象：`round2/opus-a/ABLATION.md`、`round2/gpt-sol-a/REPORT.md`、`round2/gpt-sol-b/REPORT.md`，
及 Round 3 已到货件（`round3/gpt-sol-a`、`round3/gpt-sol-b`、`round3/opus-a` 在制源码）。
基准：R2-SYNTHESIS（BINDING）。**父代理裁决重申：T4 降档边界为 `age_days ≥ 180` 降一档、
`≥ 360` 封 Weak，双双闭区间。**

## 判定汇总

| # | 事项 | 性质 | 状态 |
|---|---|---|---|
| C1 | gpt-sol-b Round 2 写 `>180`（恰 180 不降）vs opus-a/gpt-sol-a/fable-a 的 `≥180` | **真矛盾** | 已由父代理裁决关闭（`≥180`/`≥360` 闭区间），gpt-sol-b R2 版作废；R3 全员合规 |
| C2 | opus-a 消融选出 T4 vs gpt-sol-a「未选出胜者」 | 非矛盾（夹具集范围差） | 无需动作，R2-SYNTHESIS 已说明 |
| C3 | T3R 权重单位：gpt-sol-a 四分之一单位（表头误标 milli）vs gpt-sol-b 真 milli（1000/500/250） | 口径漂移（档位同构） | 已裁决：4/2/1/0 四分之一单位为准，milliscale 废弃；T3R 本身已进墓碑，遗留影响为零 |
| C4 | fable-a 协议钉 F04c「非 Strong」（T4 挂）vs opus-a 判卷「T4 零违规」 | 夹具集范围差 + 已裁决 | F04c 不在 opus-a TRUTH 集；父代理裁定 F04c 为 T4 已知限制、禁止静默加门，可数性平局裁决独立于此格 |
| C5 | R2 实测「group-only = Moderate」vs T4D 下 group-only = Weak | 刻意改档，非矛盾 | 所有 R2 钉死期望均为封顶式，Weak 全满足；改档已在 T4D_SPEC D2 显式裁决留痕 |
| C6 | 主夹具参数三个版本不一致（PIPELINE_DEBT count≥100 / 任务书 10 条×3 日 / gpt-sol-a 36 条） | 口径漂移（形状与结论一致） | T4D_SPEC §7.1 钉唯一规范参数化，其余降为变体；gpt-sol-b 性质探针覆盖全部变体 |
| C7 | `as_of`：R1-SYNTHESIS「数据内最大时间戳」vs R2「调用方传入、全库一值」 | 措辞演进，非矛盾 | 已收敛：调用方传入，产品语义=store 级最大值；per-peer 取法为显式陷阱（两个独立陷阱测试） |
| C8 | opus-a R2 的 T3 带私货群聊门（≥5/≥2）vs CANDIDATE_SPEC T3 | 定义漂移 | fable-a ABLATION_PROTOCOL §1.5 已裁决绑定 CANDIDATE_SPEC 版，未带入后续轮次 |

## 逐条细节

### C1 — 180 天边界：`>180` vs `≥180`（唯一的实现级真矛盾）

- **漂移源头**在 R1-SYNTHESIS 自身：「last_contact**>**180d 降一档；**≥**360d 封 Weak」——
  同一行混用开闭区间，两处下游各取所需。
- opus-a Round 2：`≥180`（`quiet_180_days → T4 Moderate`，矩阵 #10）。
  fable-a ABLATION_PROTOCOL §1.3：「恰好 180×86400 秒 → 180 → 降」。
  gpt-sol-a Round 2：`≥180`，并明确记录了与 R1 散文的冲突、钉 179/180/359/360 四格。
  gpt-sol-b Round 2：「strictly more than 180 … Exactly 180 days remains unchanged」——**与其余三方相反**。
- 后果量化：分歧只在恰好 180 天这一格（`age_days` 为整数，181 起两种写法重合）；
  360 侧 gpt-sol-b R2 写「from 360 days」= `≥360`，与裁决一致，矛盾只在 180 侧。
- **裁决与现状**：R2-SYNTHESIS 冻结边界明文「`>180` 的版本作废」。Round 3 三方全部合规：
  gpt-sol-b 探针表逐格钉「`>= 180`, not `> 180`」（179→Strong，180→Moderate，359→Moderate，360→Weak）；
  gpt-sol-a MATRIX 同口径；opus-a `recency.rs` 常量注释直引裁决原文并测四格。**关闭。**

### C2 — opus-a「T4 胜」vs gpt-sol-a「平局」

非矛盾，是夹具集覆盖面之差：gpt-sol-a 的 7 条必测夹具（LILEI/BURST/OLD/GROUP/ONESIDE/EMPTY/DORMANT_200d）
恰好全部落在 T3R 与 T4 的一致区；opus-a 的 16 条扩展集含全部 4 个分歧格
（quiet_179 / revived_after_gap / group_only_quiet_200 / dormant_359）。逐格核对：两套矩阵在
全部重叠夹具上档位一致（含 DORMANT_200d = opus-a #11：T3=Strong、T3R=T4=Moderate）。
gpt-sol-a 如实报告「必测集不足以破平局」，正确而非失败；父代理用扩展集 + S4 可数性破平局。**一致。**

### C3 — T3R 单位口径

gpt-sol-a Round 2 表列「T3R event milli」实为**四分之一单位**（F_LILEI=48=12×4，F_GROUP=200=50×4），
标签误用；gpt-sol-b 用真 milli（1000/500/250/0）。两者档位判定同构（阈值同倍放大），
但违反「常量单源」纪律。R2-SYNTHESIS 已裁决四分之一单位 4/2/1/0 为规范口径、废弃 milliscale。
T3R 已进墓碑（复活条件=F04c 被判不可接受），若复活，**只准**按四分之一单位复活。**关闭，遗留影响为零。**

### C4 — F04c 的判卷归属

fable-a ABLATION_PROTOCOL 预注册 F04c 期望「非 Strong」且预测 T4 在此格挂；opus-a 的 16 条
TRUTH 集不含 F04c（其 #12 `revived_after_gap` 是不同夹具：主体历史约 200 天、判卷 strong），
故「T4 零违规」与协议预测并不冲突——分母不同。父代理 R2 裁决把 F04c 定性为 T4 已知限制
（禁止静默加第三道门），胜负由可数性平局裁决决定，不由 F04c 这一格决定。
T4D 保全该限制（T4D_SPEC §7.3 / TD-9：F04c 仍 Strong），未偷改。**在案，关闭。**

### C5 — group-only 改档（R3 内部需要留痕的唯一档位变化）

R2 实测三候选都给 `group_only_50 → Moderate`，且 PIPELINE_DEBT §1.3-2 曾「接受全群 Moderate 残余」。
T4D 把 Moderate 门也改到 direct 计数上（R2-SYNTHESIS 风险 #2 的原文要求「Strong/Moderate
次数与天数门改在 direct 计数上」），group-only 结构性落 Weak。gpt-sol-a R3（F_GROUP: T4
Moderate → T4D Weak）与 gpt-sol-b R3（group-only 50 → Weak）都如实单列了这一变化，
opus-a R3 `constants.rs` 注明「T4D 无天花板常量」并把代价记入其 REPORT。
无任何钉死期望被违反（全部是「≤ Moderate」「非 Strong」封顶式）；「接受残余」是容忍不是要求，
残余被顺带关闭是净改善。要求：REJECTED.md 中 T4 条目必须把 #5 与 #17 两格一起列出，
禁止只报净胜格。**已在 T4D_SPEC D2/§7.4 留痕，关闭。**

### C6 — 主夹具参数化漂移

三个版本：PIPELINE_DEBT §1.3「50 天扇出 count≥100 + 各 1 条 Direct」；Round 3 任务书
「200 人群 10 条 × 3 日 + 各 1 条 Direct」；gpt-sol-a 实跑「36 群 + 2 直连」；gpt-sol-b「100 群 + 2 直连」。
形状同一（群重度 + 双向各一句问候）、期望同一（非 Strong / ≤ Moderate）、结论同一
（T4 Strong ✗ / T4D Weak ✓）。T4D_SPEC §7.1 钉任务书参数为唯一规范参数化并写死逐项汇总量，
其余记为变体；gpt-sol-b 的 direct 0–9 × group ≤100 全交叉把整族变体一次钉死。**收敛，关闭。**

### C7 / C8 — 已裁决旧账（复核无新发现）

- `as_of`：调用方传入、一次 rebuild 全库一值、产品语义 = store 级最大 `occurred_at`；
  per-peer 取法被 opus-a（R2 `a_peer_local_as_of_would_hide_every_dormant_tie`）与
  gpt-sol-b（R3 trap 测试）从两个方向独立钉为陷阱。各文件口径一致。
- opus-a R1 的 T3 群聊附加门（≥5/≥2）未泄漏进 R2/R3 任何实现；绑定定义始终是 CANDIDATE_SPEC §T3。

## 结论

实现级真矛盾仅 C1 一处，且已被父代理裁决并被 Round 3 三个独立实现同口径钉死；
其余为已裁决的口径漂移或范围差异。**Round 3 到货件（gpt-sol-a、gpt-sol-b、opus-a 在制源码）
与 R2-SYNTHESIS 冻结边界零冲突；T4D 在两个独立实现上结论一致，无新矛盾进入冻结。**
唯一需要写进墓碑的留痕义务在 C5（双格披露）。
