# Round 2 结论简报

父代理：`cursor-grok-4.6-high`。六子代理均自报指定 slug。

## 演进对比（相对 Round 1）

| 面 | Round 1 | Round 2 实测 |
|---|---|---|
| 人脉候选 | T3 / T3R / T4 待消融 | **T4 胜出**（opus-a 0 违规；fable-a 可数性平局裁决；gpt-sol-b 同建议） |
| T3R | 临时胜者 | 被 T4 在解释性与「Strong ⇒ 180 天内有往来」不变式上击败；`revived_after_gap` 差 0.5 次错降 Moderate |
| T3 | 回退单体 | 休眠夹具 5 张全挂；保留为无近因对照，不发布 |
| A2 | 可能占一个名额 | **降为渲染器**，源码级禁止第二套阈值 |
| A0 | 须补 intake 绕锁 | 补丁落地：`apply_intake` 与 replay 全等 |
| A1 | 可能空转 | KEEP 为 A0 内部规则；默认 `(kind,day)` 会在「问卷连填三天」误升 Strong。Round 3 默认改为 **两种来源跨日** 使 v0.1 空转 |
| 性能 | T0 ~129 ns/互动 | T3R ≈ 1.09× T0（gpt-sol-a） |

独立复现（gpt-sol-a，未引用 opus）在 7 条必测夹具上 **T3R 与 T4 档位全等**，未选出胜者——与 opus-a 扩展集（16 条）不矛盾：分歧只出现在 179/180 边界、复燃、群聊+休眠。父代理用扩展集 + 可数性打破平局。

## 冻结边界（Round 3 必须钉死，禁止再漂）

- `as_of`：一次 rebuild **全库一个值**，由调用方传入；禁止 per-peer max(occurred_at)（会让休眠关系假 Strong）。
- T4 降档：`age_days >= 180` 降一档；`>= 360` → Weak。闭区间（gpt-sol-b 写成 `>180` 的版本作废）。
- 阈值：Goal 1 的 3 / 10 / 3。T3 的 count≥8 已删除。
- T3R 权重：四分之一单位 4/2/1/0，不是 1000/500/250（gpt-sol-b 实现差 1000×，档位碰巧同构，Round 3 废弃该 milliscale）。
- 群聊：不能 Strong。

## 潜在边界风险

1. **T4 复燃一响（F04c）**：昨天互道一声好即可复活七年前 Strong。v0.1 **已知限制，禁止静默加第三道门**。若 Round 3/X 判定不可接受 → 回退 T3R。
2. **`any_direct` 闩不够**（fable-b）：群扇出 + 各一句私聊问候仍可 Strong。Round 3 必须跑夹具 `group_heavy_plus_one_direct_each_way`，并把 **Strong/Moderate 次数与天数门改在 direct 计数上**（T4D）。若 T4D 打挂 `lilei_12` 则放弃 T4D 留 T4。
3. **导入扇出 / 重复导入**：仍是管道债，算法对给定日志负责。
4. **图纠正 API**：规范已出，不阻塞算法冻结；Goal 1 后续落地。
5. **A1 三天连填问卷升 Strong**：Round 3 默认 `TwoKindsAcrossDays`。

## SOTA 验收差距

- C7：T3 已满分（Granovetter 操作化）。T4 是产品补丁，靠 C8 净胜，不吃理论分（fable-a 复审：Round 1 未把衰减错归 Granovetter）。
- 缺：T4D 夹具、中文模板绑定测试、统一 `crates/soul-algo`、墓碑进 `docs/algorithms/REJECTED.md`、`ALGO_FROZEN` 声明。
- 不验收预测准确率。

## 拟保留（尚未 ALGO_FROZEN）

1. **T4** — 人脉关系强度（T3 门闩 + 180/360 整数降档）。T4D 若通过则成为 T4 的最终形态。
2. **A0** — 特质轴问卷 + 纠正锁定（含 intake 不绕锁）。A1 为内部升档；A2 为渲染。

只保留这两个。T0/T1/T2/T3R/A3 进墓碑（T3R 复活条件 = T4 复燃被判不可接受）。
