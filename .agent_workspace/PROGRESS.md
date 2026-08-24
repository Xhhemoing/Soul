# 算法验证优化 — 进度

- 分支：`cursor/algo-verify-opt-a073`
- 基线：`origin/main`（ea6f62f）。计划权威在 `origin/cursor/soul-product-lock-7b1c`；实现快照在 `origin/cursor/soul-goal1-7b1c`。
- 循环：Round 1–3 递进 + Round X 交叉验证统一（额外一轮）。
- 每轮编制：2×fable (`claude-fable-5-thinking-xhigh`) + 2×opus-fast (`claude-opus-5-thinking-high-fast`) + 2×gpt-sol (`gpt-5.6-sol-xhigh-fast`)。

## Round 1 — 进行中

目标：盘点计划算法、对照现实现、落地可跑基线与对照候选、建立基准/边界探针。

| 槽 | 模型 | 专属目录 | 主攻 |
|---|---|---|---|
| fable-a | claude-fable-5-thinking-xhigh | `.agent_workspace/round1/fable-a/` | 全局规划、SOTA 标准、评价矩阵 |
| fable-b | claude-fable-5-thinking-xhigh | `.agent_workspace/round1/fable-b/` | 现实现多维审计、产品锁契合 |
| opus-a | claude-opus-5-thinking-high-fast | `.agent_workspace/round1/opus-a/` | Tie-strength 族参考实现 |
| opus-b | claude-opus-5-thinking-high-fast | `.agent_workspace/round1/opus-b/` | Trait/人事分析族参考实现 |
| gpt-sol-a | gpt-5.6-sol-xhigh-fast | `.agent_workspace/round1/gpt-sol-a/` | 基准脚本、确定性夹具 |
| gpt-sol-b | gpt-5.6-sol-xhigh-fast | `.agent_workspace/round1/gpt-sol-b/` | 边界/对抗探针 |

## Round 2 — 未开始

## Round 3 — 未开始

## Round X 交叉验证 — 未开始

## 保留决策（未冻结）

- 候选池：T0/T1/T2/T3 × A0/A1/A2/A3
- 拟保留：待 Round 3 + Round X 仲裁
