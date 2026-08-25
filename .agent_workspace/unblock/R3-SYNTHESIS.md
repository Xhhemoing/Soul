# Round 3 结论简报

Branch: `cursor/goal1-unblock-a073`. Parent: cursor-grok-4.6-high.
Injected context: `.agent_workspace/unblock/R2-SYNTHESIS.md` + DECISIONS D32–D40.

## 六个槽位

| 槽位 | slug | 产出 |
|---|---|---|
| fable-a | claude-fable-5-thinking-xhigh | 修订 G3 §1.1（D32）；D34/D33/D35/D40 归档；A2 验收单 `.agent_workspace/unblock/round3/fable-a.md` |
| fable-b | claude-fable-5-thinking-xhigh | AC/切片矩阵 `.agent_workspace/unblock/round3/fable-b.md`；确认无第二套 3/10/3 |
| opus-a | claude-opus-5-thinking-high-fast | `soul-draft` 走 `a2_render`；`IntakeReceipt.ignored`；denylist 豁免 `a2_adapt.rs`（父代理批准，与 `t4d_adapt` 同形） |
| opus-b | claude-opus-5-thinking-high-fast | Telegram 年 1970–9999；`is_civil_datetime` 按月天数；v1 `2026-02-31` 负例 |
| gpt-sol-a | gpt-5.6-sol-xhigh-fast | store 近阈值 8 门（2/3、9/10、日 2/3、179/180/359/360）写入 `t4d_product.rs` |
| gpt-sol-b | gpt-5.6-sol-xhigh-fast | `intake_replay.rs` 对拍 A0；windows `cargo test --no-fail-fast` |

父代理补刀：`Graph.tsx` 档位词改为 COPY_ZH 的「弱 / 中等 / 强」（去掉自制「较少/较多」）。

## 演进（相对 Round 2）

R2 已换 T4D、已做 G3 纠正与证据并集。R3 收口的是渲染器、回执诚实、导入时钟、store 近阈值与跨 crate 对拍。

## 仍开放（明确不挡 Goal 1 收口）

| 项 | 处理 |
|---|---|
| GC-6/7 遗忘 vs 锁 | D34，STATUS 遗留 |
| GC-9b「由你本人指定」 | D35，等 COPY_ZH |
| COPY_ZH §4 vs `a2.rs` 文案漂移 | 不改冻结 crate |
| 合法 9999 年摆动 as_of | G4/P1，STATUS |
| v1 按发言人数判 Direct | D36，STATUS |
| 双份 `parse_rfc3339` | 可收进 `soul-policy::clock`，非 P0 |
| 作者手动托盘/UAC/NSIS | 作者清单 |
| Hosted Actions 空 runner | 分钟耗尽，不是代码任务 |
| STATUS 仍写 `soul-goal1-7b1c` 为 Goal 1 分支 | 合入后由父代理改口；本 PR 是集成分支 |

## SOTA 验收

- 无第二套 3/10/3（soul-graph 源码守卫仍在；soul-draft 无 180 字面量）
- A2 分列句以 `as_of_utc.is_some()` 决定在场
- 锁定边仍抑制 filed_band
- 产品合成 `score` 字段 denylist 仍红；adapter 文件豁免
- 不启动 Goal 2，不加 F04c 第三门

**不再开下一轮 6 槽循环**：原 P0（M2/M3/G2/G1+/G1/G3/A2）已在本分支落地；剩余项全部有拍板或作者/分钟阻塞。
