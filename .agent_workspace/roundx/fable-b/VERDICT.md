MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round X fable-b 裁定

**CONFIRM_FREEZE**

## 依据

1. **冻结条件本轮独立复现**：`cargo run --example matrix`（as_of = 2026-08-24T14:00:00Z）逐行核对 DECISION 第 1 节——`lilei_12` 在 T4D 下保持 Strong ✓，`group_heavy_plus_one_direct_each_way` 在 T4D 下为 Weak（T4 为 Strong，漏洞实证复现）✓。R2 绑定集全表（含 179/180/359/360 闭区间边界、`dormant_2019` 沉寂 2632 天、`group_only_50`、`one_sided_100`、单日 1000 条）与 DECISION 第 3 节实测值逐项一致。
2. **工程面干净**：`cargo test --workspace` 全绿（含 Round X 其他槽位新增的探针测试）、`cargo clippy --workspace --all-targets` 零警告、`cargo fmt --check` 通过；两 crate 零运行时依赖、`unsafe_code = "forbid"`、无墙钟、无网络路径。
3. **C1 产品锁契合成立**（详 `LOCK_FIT.md` 第 1–2 节）：T4D 与 A0 不但逐条不违反 PRODUCT_LOCK，且各自修复了锁的关键词的真实缺口——T4D 堵「群扇出制造档位」（人脉图失真），A0 堵 Goal 1 `intake` 绕纠正锁（本轮在快照 `profile_service.rs` 复核该缺陷属实）。已知代价 4.1/4.2/4.3 均与锁无冲突且已如实入档，回退链完备。
4. **交叉一致**：与 roundx/gpt-sol-a 的独立影子实现比对结论一致（9/9 AGREE）。

## 随裁定记录的非阻塞项（全部属 DECISION 第 0 节预声明的合并义务，不改任何档位）

- crate 尚未统一为 `crates/soul-algo`；两个 `TieScore` 结构并存。
- `a2::DORMANT_AFTER_DAYS` 是常量表禁止的值分裂，且 `>180` 与降档 `>=180` 有一天宽的边界摆动（COPY_ZH P4「超过」与 DECISION「闭区间共用常量」互斥，冻结不变式「P4 句出现 ⇒ 已降档」在两种写法下均成立）；合并时以 DECISION 为规范统一为 `>=` 并留痕。
- `explain_zh` 满足 COPY_ZH §5 断言但未按 §1 冻结模板结构渲染（缺透明句与收尾句）；合并时代码就模板或先改 COPY_ZH 留痕。
- 常量名与 DECISION 第 3 节表不同名（值、语义、闭区间全对）；DECISION 6.4 引「COPY_ZH 第 6 节」实为 §5。

以上无一构成回退链触发条件（4.1 未被产品裁定不可接受，F04c 未被裁定不可接受），亦无一触及算法选型本身。冻结维持：**T4D + A0**。
