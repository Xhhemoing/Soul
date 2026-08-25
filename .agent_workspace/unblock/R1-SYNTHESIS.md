# Round 1 结论简报（注入 Round 2）

Branch: `cursor/goal1-unblock-a073`. Parent: cursor-grok-4.6-high.
Do not start Goal 2. Do not add F04c third gate. Do not change T4D constants.

## 已实现

- **M2** merge main：`soul-algo-tie` / `soul-algo-trait` 在工作区。
- **M3** `7aee812`：运行时大小写探针 + 精确语料守卫。Bravo 仍无条件拒绝。
- **G2** `62840ff`：`intake` 锁轴不 `place_axis`；`IgnoredAnswer` / `axis_locked_by_user`；测试在 `correction_lock.rs`。
- **G1+** `90da257`：owner 群消息 `peers = []`；incoming 仍记发送者；1:1 owner 仍归因对话里其他人。
- **G1 预备** `4da2184`：`soul-graph/src/t4d_adapt.rs` intern + RFC3339。**尚未**替换 `Tally::band`。
- 规格：`unblock/round1/fable-a-T4D_WIRING.md`、`fable-b-G3.md`。

## 遗留缺陷

- G1：`build.rs` 仍是 T0 本地 3/10/3。
- G3：rebuild 仍写 `Unreviewed`。
- **denylist**：工作区现在扫到 `soul-algo-trait` 里的标识符 `TieScore`（`score`）。不要改算法规则；给 `crates/soul-algo-*` 加 denylist 豁免（与「禁止产品档案出现量表分数」同向：算法 crate 的类型名不是用户可见分数）。
- 锁轴忽略尚未出现在 soulcore `IntakeReceipt`（WP03 leftover 8）。

## 性能

非本轮重点。算法 crate 测试绿。

## Round 2 攻坚

1. opus-a：按 fable-a 规格把 `Tally::band` 换成 `soul_algo_tie::score`；用已有 `t4d_adapt`；加 T1–T12 能落地的测试。
2. opus-b：G3 字段 + rebuild 不抹 `user_verdict`；GC-9a 抑制锁定边的冻结 P5 句；不要发明 COPY_ZH 未冻结的中文。
3. gpt-sol-a：denylist 豁免 + 回归测试（合成 `score` 字段在产品 crate 仍红）。
4. gpt-sol-b：图边界夹具：群洪+每向 1 条一对一 → Weak；lilei_12 仍 Strong；as_of 全库单一。
5. fable-a / fable-b：只读复审上述 diff，写 round2 报告，不并行改同一业务文件。
