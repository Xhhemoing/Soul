# Cycle 1 Round 2 结论简报

未等 fable。四槽：opus-a/b、gpt-sol-a/b。

## 演进

- **P4 Hawkes 降为对照**，不是 v0.2 默认。事件率与采集开关/同意纠缠（丢弃中的会话不落库）；与 P1 预测对象不同（何时 vs 哪个 app），「P4 必须打赢 P1」提法不成立。对手应是挂在转移表上的条件间隙基线 C3。
- **APPM 不是与 P2 并列的第十个模型**：可变阶前缀并入 P2（max order 3）；命中加权并入 P7 备选（整数 pairwise wins，禁止除法）；TTU 与 P4 同轴另测；预取出局（产品锁禁止代启动）。
- **不变式 F** 在去抖合并与 heavy-hitter 驱逐下不「天然成立」；测试须 `evictions==0` 且删除事件做局部重算。
- 夹具与评测：SYN-MARKOV / SYN-RHYTHM / SYN-NOISE / ADV-DST；prequential；打乱泄漏探针 P1≈P0。指标只在评测器内部，不上屏。

## Round 3

冻结 Cycle 1 候选目录（可测并列清单 + 否决 + 测试日前置缺口）。不实现 crate。
