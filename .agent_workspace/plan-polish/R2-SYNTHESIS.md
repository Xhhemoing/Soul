# Round 2 结论简报 — 打磨和完善项目计划

父代理：cursor-grok-4.6-high。六路均按指定 slug 出活，无静默降级。
核于 2026-08-25。对照 Round 1：`.agent_workspace/plan-polish/R1-SYNTHESIS.md`。

## 演进对比

| Round 1 遗留 | Round 2 结果 |
|---|---|
| L1 `tie_strength` 裸 object | **关闭**：类型化 + lock 重算；补 D32 三字段 `machine_band`/`user_band`/`locked_by_user`（P0，否则 Goal 1 重建边会被 additionalProperties 打红）。探针：空对象过、score 拒、半迁移拒、T4D+machine 过 |
| L2 历史 Goal/planner 段 | **关闭**：FORMAL 标历史段；PLAN_VERIFY 横幅；开工第一动作是唯一可执行路径 |
| L3 SECURITY 短版 | **关闭**：指针节，不把 Goal 1 实证抄成 main 已落地。Goal 1 尖端统一为 `df5d2dd` |
| L4 过程目录副本 | **保留快照** + README 非权威；PRODUCT_LOCK 哈希已分叉（符合预期） |
| L5 BLOCKERS 不同树 | 仍不同树；脚注规定 PR #6 合入时处理 D32 撞号 |
| L6 产品路径 AC 不可在本分支跑 | 仍成立；本分支用 schema/文档探针代替 |
| D52 泄漏字面量 `180` | **关闭**：改为常量名 `DEMOTE_ONE_BAND_DAYS`。核心计划文件现无 `180` |
| G1+ 无验收行 | **补 AC-34** |

拍板追加 **D59**（schema 收紧已执行）。D32–D40 与 Goal 1 吸收线同号；D40 补上「锁定边仍丢掉 filed_band」。

## 潜在边界风险

1. **合并拓扑**：本 PR 合进 main 后，Goal 1 merge main 会在 STATUS / SECURITY / DECISIONS / relationship.schema + lock 上冲突。取法：计划权威面以本 PR 为准；Goal 1 实现纪事 STATUS 合成一份；schema 以本 PR 的类型化版为准再 `schema-freeze`。
2. **Goal 1 现行 `TieStrength` 若尚未序列化 machine_band**：空对象与无 algorithm_id 的八字段对象仍合法；一旦写 `algorithm_id` 就必须整包。unblock 线已写 machine_band，必须出现在 properties 里——已补。
3. **AC-34** 依赖导入归因，不是算法 crate 夹具；实现在 Goal 1/PR #7。
4. **D32 撞号**：PR #6 BLOCKERS 仍用「D32」指 e0 禁令。合入检查单已写在 DECISIONS 脚注。
5. 同树 Goal 1 尖端会继续前进；下次刷新须 STATUS 与 SECURITY 同一提交号（D57）。

## SOTA 验收差距（留给 Round 3）

- 新父代理只读 `PLAN_INDEX.md` 应能正确开工：引导面已注记。Round 3 做只读交叉核验 + 负向探针（OAuth/E0/文件写、常量泄漏、lock 哈希、AC-28–34 存在性）。
- 不把 BLOCKERS 整份拷进本 PR。
- 不改产品方向、不加 F04c 第三道门、不启动 Goal 2。
- 文档对齐后即可合 `main`（计划面无阻塞 ≠ Goal 1 关闭）。
