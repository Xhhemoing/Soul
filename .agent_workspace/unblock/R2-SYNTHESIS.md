# Round 2 结论简报（注入 Round 3）

Branch: `cursor/goal1-unblock-a073`. Parent: cursor-grok-4.6-high.
Do not start Goal 2. Do not add F04c third gate. Do not change T4D constants.
Do not invent COPY_ZH keys. Do not `cargo generate-lockfile`. Do not git commit/push
from a child agent. File ownership is exclusive (lost `build.rs` hunk in R2).

MODEL_SLUG of this brief: parent `cursor-grok-4.6-high`. Child outputs must start
with `MODEL_SLUG: <actual slug>`.

## 演进对比（相对 Round 1）

| 项 | R1 结束 | R2 结束 |
|---|---|---|
| G1 T4D band | 仅 `t4d_adapt` 预备，`Tally::band` 仍 T0 | **已换血** `814064e`：`soul_algo_tie::score`，全库单一 `as_of`，分列字段落边 |
| G3 verdict | rebuild 写 `Unreviewed` | **已保全**；`correct_tie` / `release_tie`；GC-9a 抑制锁定边归档句 |
| G3 R3 证据并集 | 规格有、rebuild 零调用 | **已接线** `9b12268`；`graph_correction.rs` 在 rebuild **之后**断言 cite |
| denylist | 扫到 algo crate `TieScore` | **已豁免** `crates/soul-algo-*`；产品合成 `score` 字段仍须红 |
| 产品门测 | 无 | `t4d_product.rs` 四门（Strong / 群洪 Weak / 仅群 Weak / 休眠 Weak） |

未重做（仍然禁止去碰）：DPAPI、`/import`、`/collect`、wizard 11Q、`/profile`、KnownIdentifiers。

## 潜在边界风险（R3 必须处理或定价）

1. **导入时间戳可毒化 rebuild（P0，可达）** — Telegram `date_unixtime` 任意 i64 → `rfc3339_utc` 产出非四位年 → `parse_rfc3339` 失败 → 此后每次 rebuild 永久 `UnreadableInteraction`。导入不是事务。修在 `soul-import`：年份不在 1970–9999 记 defect、拒该消息。v1 `is_date_time` 放行 `2026-02-31`：补日历日校验 + 负例。
2. **COPY_ZH `{群聊次数}` 与 G1+ 口径（P1，定价不改模板）** — owner 群消息不再归因 → `group_out_count` 结构为 0。A2 只渲染持久化分列，不得从证据重算。口径见 DECISIONS D33。
3. **soul-import-v1 按发言人数判 Direct/Group** — 单活跃发言人的群可铸 Direct，一对一门可误开。不改冻结契约；STATUS 钉死现状（D36）。
4. **单条未来时间戳摆动全库 as_of** — 合法 RFC3339 的 9999 年可把全库降 Weak。G4/P1，STATUS 定价，本轮不挡收口（D37 只挡不可解析年）。
5. **遗留零 vs 实测零** — P1b 以 `as_of_utc.is_some()`（或 `algorithm_id == "T4D"`）决定分列句是否出现，不得以计数==0 当缺席。

## SOTA 验收差距（R3 攻坚）

| ID | 差距 | R3 owner | 文件所有权（禁止越界） |
|---|---|---|---|
| A2 | `points_for` 仍用自制「往来不多/中等/密集」与自制方向/场合句；未走冻结 `a2_render`；无 P1b | opus-a | `crates/soul-draft/**`，`Cargo.toml` 该 crate |
| G2 回执 | `IntakeReceipt` 丢掉 `ignored`；`answered` 把拒答算进去 | opus-a | `crates/soulcore/src/commands/profile.rs` + 相关测试 + `apps/desktop/src/core.ts` + `fakeCore.ts` |
| 导入时钟 | 无界 `date_unixtime`；`is_civil_datetime` 月 31 日 | opus-b | `crates/soul-import/**` |
| 近阈值门 | store 侧缺 2/3、9/10、日 2/3、179/359 不降 | gpt-sol-a | `crates/soul-graph/tests/t4d_product.rs` 只加测试 |
| G2 对拍 | 无 `intake` vs `apply_intake` 映射后全等测试 | gpt-sol-b | `crates/soul-profile/tests/` 新文件 + `Cargo.toml` dev-dep |
| CI 可观测 | windows `cargo test` 无 `--no-fail-fast` | gpt-sol-b | `.github/workflows/ci.yml` 只加该 flag，不改触发分支 |
| 规格修订 | G3 §1.1 与落地冲突；GC-6/7 未裁 | fable-a | `.agent_workspace/unblock/round3/fable-a.md` + 修订 `round1/fable-b-G3.md` 冲突段 |
| 验收矩阵 | FORMAL AC / PRODUCT_LOCK 对 HEAD 的剩余红项 | fable-b | `.agent_workspace/unblock/round3/fable-b.md` 只读报告 |

## 本轮明确不实现（已拍板，见 `docs/DECISIONS.md`）

- **D34 GC-6/GC-7 / R4–R5**：遗忘压过锁、无观测锁定边 Option 时间戳 — 本轮 STATUS 遗留，不发明墓碑。
- **D35 GC-9b**：禁止产品非测试源码出现「由你本人指定」。测试里的反向断言字面量允许。
- **D36** v1 群/一对一发言人数启发：钉现状，不改冻结契约。
- **D32** 未锁边 `machine_band` 常在：`locked ⟺ user_band.is_some()`。规格改字，代码已钉。

## 父代理给 R3 的实现约束（A2）

- `soul-draft` 可依赖 `soul-algo-trait`（渲染器进产品 crate，方向合法）。
- **禁止改** `crates/soul-algo-trait/src/a2.rs` 模板字面量（COPY_ZH 与 crate 文案有漂移，记 STATUS，不在 Goal 1 静默改冻结 crate）。
- 适配：`TieStrength` → `TieScore`；UUID 证据 id 稠密 intern 成 u64 再映射回去。
- `direct_count` / `group_count`：仅当 `as_of_utc.is_some()` 时 `Some(out+in)`（含真实 0）；遗留边两者都 `None`。
- 锁定边：调用 `a2_render` 后 **丢掉** `personnel.tie.filed_band`（GC-9a 继续成立）。
- `DORMANT_AFTER_DAYS` 只存在于冻结 A2，soul-draft 不得再写 180。

## 父代理给 R3 的实现约束（IntakeReceipt）

- 加法字段 `ignored: Vec<{ question_id, reason }>`，`reason` 为 `axis_locked_by_user`。
- `answered` = 动了轴的回答数（`evidence_ids.len() - ignored.len()`），不再把拒答算「已答」。
- serde 加法；更新 TS 接口与 `anIntakeReceipt` 默认 `ignored: []`。
- 向导/档案页可以暂时不新造中文（COPY_ZH 无此句）；回执字段先诚实。
