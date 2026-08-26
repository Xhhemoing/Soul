# Round 1 结论简报

父代理：`cursor-grok-4.6-high`。日期：2026-08-24。
编制：2×fable (`claude-fable-5-thinking-xhigh`) + 2×opus-fast (`claude-opus-5-thinking-high-fast`) + 2×gpt-sol (`gpt-5.6-sol-xhigh-fast`)。六份产出均自报指定 slug，无静默降级。

## 已实现 / 已证明

| 项 | 证据 |
|---|---|
| T0 与 Goal 1 `Tally::band` 逐条等价 | opus-a `goal1_fidelity.rs`：6500 组随机互动 band 全等；gpt-sol-a 独立复现同一常量 |
| 互惠门闩 + 「单日热聊不能 Strong」是全族共识 | 四规则在 `lilei_12` / `twenty_in_one_afternoon` / `one_sided_100` 上一致 |
| T0 吞吐 | gpt-sol-a：10k 互动 / 200 peer，约 129 ns/interaction |
| A0 纠正锁 / 无证据拒绝 / 非临床 | opus-b 72 测；Goal 1 `correction_lock.rs` 仍是产品锁正确实现 |
| A3 正文推断必须拒绝 | opus-b `a3_from_message_text` 恒 `Err`，测试钉死 |
| T0 群聊可被刷成 Strong | gpt-sol-b 探针 **故意失败 1 条** 证实；opus-a `group_only_50` T0=Strong |

独立对照矩阵（opus-a，同一证据）：

| fixture | T0 | T1 | T2 | T3 |
|---|---|---|---|---|
| 12 次/6 天/互惠/私聊 | strong | strong | strong | strong |
| 一下午 20 条 | moderate | moderate | moderate | moderate |
| 群聊 50 天 50 条 | **strong** | **strong** | **strong** | moderate |
| 2019 互惠、今=2026 | **strong** | weak | moderate | **strong** |
| 单向 100 出 | weak | weak | weak | weak |

## 遗留缺陷（按优先级）

**P0**

1. **群聊 N:1 放大**（fable-b）：Goal 1 导入把群里一条主人发言扇出成「对每个曾发言者一条 outgoing」。T0 再忽略 `Venue`。200 人群聊里发 10 条跨 3 天 → 全员 Strong。算法层必须 venue 封顶；导入扇出是管道债，Round 2 文档化，不在 `soul-algo` 里假装没发生。
2. **人脉图不可纠正**（fable-b）：产品锁写「用户可查看和纠正」，`rebuild` 把 `user_verdict` 写回 `Unreviewed`。这是产品契约，不是 tie-band 公式；Round 2 出接口草案，Round 3 验收话术。
3. **群聊可达 Strong**（全员共识）：T0/T1/T2 在 `group_only_50` 上错档。T1 的 0.4 降权**挡不住** mass≥8。

**P1**

4. **无近因**：2019 密友到 2026 仍 Strong（T0/T3）。T1 浮点衰减可修但用户无法「数消息」复核。
5. **A0 `intake` 绕锁**：重跑问卷可改已锁定轴（fable-b / opus-b）。
6. **A0 后写覆盖**：未锁定时一次 Weak 推断会抹掉问卷 Moderate 的引用。
7. **重复导入加倍计数**（gpt-sol-b）：rebuild 幂等边，但不幂等次数。
8. **WP10 摘要可能自带第二套阈值**（opus-b）：与图 band 会打架。goal1 已有 `soul-draft/src/analysis.rs`，A2 必须改成 **纯消费者**。

**P2**：DST/偏移时间戳、互惠单条闩（50 出 1 入）、证据列表无界。

## 性能瓶颈

T0 本身不是瓶颈（~10² ns/互动）。真实风险在 Goal 1 导入扇出（群成员 × 消息）和 WP10 按边扫证据的潜在超线性。参考实现保持 O(n) tally。

## 落选（本轮即可归档，Round 2 写墓碑）

| ID | 决定 | 理由 |
|---|---|---|
| **T2 RFM** | 杀 | 三分位/求和是百分位味道；同一个人的档会因别人数据变化；opus-a 无独特净胜夹具 |
| **A3 正文/LIWC** | 杀 | 要正文、临床相邻、不可解释；代码层永久拒绝 |
| **T1 单体** | 杀（度量可借用） | 一下午热聊与群聊仍可 Strong；浮点 `exp` 用户不可复核 |
| **T0 原样** | 不保留为规范 | 被 T3 在 F03 严格支配 |

## 仍在竞争（Round 2 必须消融）

| ID | 角色 |
|---|---|
| **T3** | Granovetter 门闩：强档要互惠+私聊+跨日；群聊封顶 Moderate。无衰减。回退单体。 |
| **T3R** | T3 门闩 × 分桶半衰计数（H=90，CUTOFF=360，权重 1 / ½ / ¼ / 0）。fable-a 临时胜者。 |
| **T4** | opus-a 建议：T3 + **整数降档**（last_contact>180d 降一档；≥360d 封 Weak）。比 T3R 更易口述。 |
| **A0** | 产品锁基底，不占「1–2 名额」；必须补 intake 绕锁。 |
| **A1** | A0 的升档插件；若 v0.1 只有问卷+纠正则可能空转 → 归档「正确但空转」。 |
| **A2** | 人事摘要，**禁止自带 band 阈值**；只渲染 T 族胜者的 `TieScore`。 |

## 临时保留对（PROVISIONAL，待 C8）

- 人脉：**T3R 或 T4**（Round 2 消融；若衰减从未改档 → 留 T3）。
- 档案/人事：**A0（政策）+ A2（渲染）**。A2 若完全无独立判断，可降为「不算独立算法」，则只保留 1 个人脉算法。

**禁止回退到未打补丁的 T0。**

## 下轮攻坚重点

1. 实现 T3、T3R、T4；用同一套夹具做 C8 消融（净胜 + 零回归）。`as_of` = 数据内最大时间戳，禁读墙钟。
2. `TieScore` 增加 `any_direct`；A2 只读 TieScore。
3. A0 锁在 replay/intake 两条路径都必须成立；A1 用 F16/F16b 决定去留。
4. 中文解释冻结：只谈次数/天数/是否私聊/是否互惠；禁 score/百分位/诊断词。
5. 文档化导入扇出与去重，标为管道债，不是算法胜者的责任。
