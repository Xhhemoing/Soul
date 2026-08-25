# Round 3 结论简报

父代理：`cursor-grok-4.6-high`。六子代理均自报指定 slug。父代理复跑 Round 2 包测试已绿。

## 冻结

`docs/algorithms/DECISION.md` 声明 **`ALGO_FROZEN`**。

**保留两个：**

1. **T4D** — 人脉关系强度。一对一互惠 ∧ 一对一次数≥10 ∧ 一对一活跃天≥3 → Strong；一对一互惠 ∧ 次数≥3 → Moderate；否则 Weak。然后按任一场地 last_contact：≥180 天降一档，≥360 天 Weak。
2. **A0** — 特质轴问卷 + 纠正锁定（intake 不绕锁）。

A2 为渲染器。A1 默认 TwoKindsAcrossDays（v0.1 空转）。T4 为档内回退。T3R 仅在 F04c 被判不可接受时复活。

## 决胜夹具（三方独立复现一致）

| 夹具 | T4 | T4D |
|---|---|---|
| lilei_12 私聊 12/6 | Strong | Strong |
| 群扇出 + 各 1 条私聊 | Strong ✗ | Weak ✓ |
| 仅群聊 50/50 | Moderate | Weak |
| 2019 休眠 as_of=2026 | Weak | Weak |

## 与 Round 2 对比

- T4 的 `any_direct` 闩被 T4D 的「门开在 direct 列」取代，关闭群扇出+一句问候 = Strong。
- 群聊-only 从 Moderate 降为 Weak（有意代价，A2 仍展示群次数）。
- gpt-sol-a：T4D ≈ 1.00× T0。
- 边界统一为闭区间 ≥180 / ≥360。

## SOTA

T3 在 Granovetter 操作化上已满分；T4D 是产品补丁（venue 列 + 近因整数降档），靠 C8 净胜而非新理论。不验收预测准确率。

## 合并义务（本简报同期完成）

- `crates/soul-algo-tie`、`crates/soul-algo-trait` 纳入 workspace。
- 墓碑与话术已在 `docs/algorithms/`。

Round X：对合并后的 crate 做独立交叉验证；合法改判仅走 DECISION 第 5 节回退链。
