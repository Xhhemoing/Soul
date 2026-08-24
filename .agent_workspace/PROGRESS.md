# 算法验证优化 — 进度

- 分支：`cursor/algo-verify-opt-a073`
- PR：https://github.com/Xhhemoing/Soul/pull/5
- 基线：`origin/main`。计划权威 `origin/cursor/soul-product-lock-7b1c`；实现快照 `origin/cursor/soul-goal1-7b1c`。
- 循环：Round 1–3 递进 + Round X 交叉验证统一。
- 每轮编制：2×fable (`claude-fable-5-thinking-xhigh`) + 2×opus-fast (`claude-opus-5-thinking-high-fast`) + 2×gpt-sol (`gpt-5.6-sol-xhigh-fast`)。

## Round 1 — 完成

见 `.agent_workspace/R1-SYNTHESIS.md`。杀 T2 / A3 / T1 单体 / T0 原样。人脉侧 T3 vs T3R vs T4 待消融。A0 补锁；A2 改为纯消费者。

## Round 2 — 进行中

目标：实现 T3/T3R/T4 并做 C8 消融；A0 锁补丁；A2 去阈值；写墓碑。

| 槽 | 模型 | 专属目录 | 主攻 |
|---|---|---|---|
| fable-a | claude-fable-5-thinking-xhigh | `.agent_workspace/round2/fable-a/` | C8 消融标准、中文话术冻结、SOTA 复审 |
| fable-b | claude-fable-5-thinking-xhigh | `.agent_workspace/round2/fable-b/` | P0 管道债规格、图纠正接口、验收差距 |
| opus-a | claude-opus-5-thinking-high-fast | `.agent_workspace/round2/opus-a/` | T3 / T3R / T4 实现与消融测试 |
| opus-b | claude-opus-5-thinking-high-fast | `.agent_workspace/round2/opus-b/` | A0 锁 + A1 + A2 纯渲染 |
| gpt-sol-a | gpt-5.6-sol-xhigh-fast | `.agent_workspace/round2/gpt-sol-a/` | 独立消融基准 |
| gpt-sol-b | gpt-5.6-sol-xhigh-fast | `.agent_workspace/round2/gpt-sol-b/` | 对抗探针必须在 T3R/T4 上转绿 |

## Round 3 — 未开始

## Round X 交叉验证 — 未开始

## 保留决策（未冻结）

- 已杀：T2、A3、T1 单体、T0 原样
- 竞争：T3 / T3R / T4 ；A0（基底）/ A1（插件）/ A2（渲染）
