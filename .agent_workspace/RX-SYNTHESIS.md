# Round X 交叉验证简报

父代理：`cursor-grok-4.6-high`。额外一轮 6 模型交叉验证合并后的 `crates/` 与 `ALGO_FROZEN` 文档。

## 投票

| 槽 | slug | 裁决 |
|---|---|---|
| fable-a | claude-fable-5-thinking-xhigh | CONFIRM_FREEZE（P1：常量命名/话术未对齐，非档位） |
| fable-b | claude-fable-5-thinking-xhigh | CONFIRM_FREEZE |
| opus-a | claude-opus-5-thinking-high-fast | 档位算术确认；曾 BLOCK 解释层 D1/D2/D3 |
| opus-b | claude-opus-5-thinking-high-fast | PASS；补 A1/A2 钉扎测试 |
| gpt-sol-a | gpt-5.6-sol-xhigh-fast | CONFIRM_FREEZE（独立影子 9/9 与 crate 一致） |
| gpt-sol-b | gpt-5.6-sol-xhigh-fast | CONFIRM（path 依赖 crate 的对抗探针全绿） |

## 父代理仲裁后的修复（解释层，不改档位）

opus-a 的 BLOCK 三项均已修，且不触碰 T4D 门闩：

1. **D1** `explain_zh` 按一对一计数选句，不再用 `score.band`/`detail` 编造门槛。存档 T0 档与现行规则不一致时同时说出两个标签。
2. **D2** `Tally` 用 `saw_direct` 区分「还没有私聊」与「Unix 0 秒的真私聊」。
3. **D3** 任一场地时钟与一对一时钟不一致时，解释补上「一对一最近一次是 {日期}」。
4. A2 沉寂句改为闭区间 `>= 180`，与 T4D 降档共用同一天。

`cargo test --workspace` 绿。

## 统一结论（不变）

保留 **T4D + A0**。T3R 仍为 F04c 回退。Goal 1 采纳：`Tally::band` 换成 `soul_algo_tie::score`。
