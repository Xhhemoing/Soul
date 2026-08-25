MODEL_SLUG: claude-fable-5-thinking-xhigh

ALGO_FROZEN

# Soul v0.1 算法冻结决议

裁决：Round 3 fable-a（SOTA 文档冻结槽位）。依据：`R2-SYNTHESIS.md`（BINDING）、`round2/fable-a/ABLATION_PROTOCOL.md` 裁决程序、本轮独立验证 `round3/fable-a/t4d-verify/`（13/13 测试通过）与两份独立实现的一致复现（round3/gpt-sol-a、round3/gpt-sol-b）。

## 0. 剩余阻塞项：无

R2-SYNTHESIS 列出的冻结前置项逐条清空：

| 前置项 | 状态 |
|---|---|
| T4D 夹具（`group_heavy_plus_one_direct_each_way` + `lilei_12` 保全） | **已跑**，三方独立实现结论一致（本槽位 t4d-verify、gpt-sol-a、gpt-sol-b） |
| 中文模板 | 胜者模板冻结于 `docs/algorithms/COPY_ZH.md`（本决议同批） |
| 墓碑 | `docs/algorithms/REJECTED.md`（本决议同批） |
| `ALGO_FROZEN` 声明 | 即本文件 |

「统一 `crates/soul-algo`」与「模板绑定测试进合并 crate」是**冻结后的合并义务**（见第 6 节），不是算法选型的阻塞项：本决议冻结的是规则、常量与话术；合并工作以本决议为规范执行。Round X 交叉验证照常进行，其唯一合法改判通道见第 5 节回退链。

## 1. 冻结条件的裁决

Round 3 冻结条件（R2-SYNTHESIS 边界风险 2）：**T4D 采纳，当且仅当 `lilei_12` 保持 Strong 且 `group_heavy_plus_one_direct_each_way` 非 Strong；否则留 T4。**

实测（as_of = 2026-08-24T14:00:00Z，全库单值）：

| 决胜夹具 | 构造 | T4 | T4D | 判 |
|---|---|---|---|---|
| `lilei_12` | 一对一互惠 12 次 / 6 天 / 3 天前收尾 | Strong | **Strong** | 锚测试保全 ✓ |
| `group_heavy_plus_one_direct_each_way` | 代码夹具：群聊互惠 30 次 / 10 天 + 每方向各 1 条一对一（叙述曾写 100/50，以测试源为准） | **Strong ✗**（漏洞实证） | **Weak** | 非 Strong ✓ |

两个条件同时满足。**采纳 T4D，作为 T4 的最终形态发布。**

同一验证还跑了 Round 2 绑定集全表与探针（empty / single_inbound / afternoon_20 / group_only_50 / one_sided_100 / steady_16 / dormant_2019 / F04b / F04c / 179‑180‑359‑360 边界 / 置换与时间平移不变式 / as_of 陷阱），T4D 相对 T4 的全部分歧只有两类，均记录在第 4 节。矩阵由代码打印，见 `.agent_workspace/round3/fable-a/REPORT.md`。

## 2. 保留清单（v0.1 发布形态）

1. **T4D — 人脉关系强度（唯一权威）**：T3 门闩 + 180/360 整数降档，且互惠、次数、天数三道门全部改在**一对一计数**上。群聊行保留展示与近因，不参与判档。
2. **A0 — 特质轴问卷 + 本人纠正锁定**：政策基底，不占算法名额；intake 不绕锁（Round 2 补丁，`apply_intake` 与 replay 全等）。
3. **A2 — 统计式人事摘要，纯渲染器**：band 的纯消费者，源码级禁止第二套阈值；新增一对一/群聊分列句（只渲染 T 族算好的两个数，禁止自行推导）。
4. **A1 — A0 内部升档规则**：默认 **TwoKindsAcrossDays**（两种不同来源、跨不同自然日才升档），杜绝「问卷连填三天误升 Strong」；v0.1 只有问卷一种来源，**空转是预期行为**，不是缺陷。
5. **T4 — 档内回退形态**：非发布默认；仅当第 5 节档内回退触发时按原冻结定义启用。其模板随 COPY_ZH.md 一并冻结。

## 3. 常量表（单点定义，全仓库禁止第二处出现数字字面量）

| 常量 | 值 | 语义 |
|---|---|---|
| `MODERATE_MIN_INTERACTIONS` | 3 | Moderate 次数门（T4D：一对一次数） |
| `STRONG_MIN_INTERACTIONS` | 10 | Strong 次数门（T4D：一对一次数） |
| `STRONG_MIN_ACTIVE_DAYS` | 3 | Strong 自然日门（T4D：一对一自然日） |
| `DEMOTE_ONE_BAND_DAYS` | 180 | 沉寂 ≥ 180 天降一档，**闭区间**（`>=`；任何 `>180` 写法作废） |
| `FORCE_WEAK_DAYS` | 360 | 沉寂 ≥ 360 天封 Weak，闭区间 |
| `DORMANT_NOTE_DAYS` | = `DEMOTE_ONE_BAND_DAYS` | A2「最近半年没有往来」句阈值，与降档共用同一常量，不得分裂 |
| `GROUP_ONLY_CEILING` | Moderate | **仅 T4 使用**。T4D 无此常量：仅群聊者一对一计数为 0，自然落 Weak（见 4.1） |

**as_of 纪律**：一次 rebuild 全库**一个** as_of，由调用方传入（缺省 = 整个 store 的 `max(occurred_at)`）；禁止 per-peer 取该 peer 自己的最大时间戳——那会让休眠关系假 Strong（t4d-verify `peer_local_as_of_wrongly_revives_the_dormant_tie`：2019 年休眠关系在 per-peer as_of 下错判 Strong，在全库 as_of 下正确判 Weak，沉寂 2632 天）。judgement 一律不读墙钟。

**T4D 直连计数规则（冻结定义）**：

```text
direct_* = venue 为一对一的行的计数（次数、自然日、双向）

Strong    iff  direct 双向都发过 ∧ direct 次数 ≥ 10 ∧ direct 自然日 ≥ 3
Moderate  iff  direct 双向都发过 ∧ direct 次数 ≥ 3
Weak      otherwise

降档时钟：沉寂天数 = max(0, as_of − 任一场地最后一次往来) / 86400（整数天，向下取整）
          ≥ 360 → Weak；≥ 180 → 降一档；空观测 → Weak，不进时间运算
```

时钟**刻意**读任一场地：T4D 拒绝用群聊行判档，但接受群聊行作为「此人没有从生活中消失」的近因证据。这保证 A2 的 P4 句（读任一场地 last_contact）与档位永不同屏矛盾——「最近半年没有往来」句一出现，降档必已发生。gpt-sol-b Round 3 的「direct-only 时钟」变体**作废**（会造成「最近一次是昨天」与「因久未联系而降档」同屏）。同理作废的历史漂移：Round 2 gpt-sol-b 的 1000× milliscale、Round 2 opus-a T3 的群聊 ≥5/≥2 附加门。

## 4. 已知代价与限制（如实入档，禁止静默修补）

### 4.1 仅群聊者从 Moderate 变 Weak

`group_only_50`（群聊互惠 50 次 / 50 天 / 从未一对一）：T4 判 Moderate，T4D 判 **Weak**。这是把门槛改在一对一计数上的直接后果，不是意外：Round 1「项目群同事不是陌生人」的论证让位于「群扇出不得制造档位」——同一机制也顺带消解了导入扇出债的放大面（200 人群刷全员 Moderate 不再可能）与「一句私聊解锁 Strong」漏洞。群聊往来的事实由 A2 场合句如实展示（「你们在群里同场往来 {群聊次数} 次」），只是不再决定档位。钉死期望「group-only ≤ Moderate」仍满足（Weak ⊂ ≤ Moderate）。自愈路径便宜：补够常量表规定的一对一次数即回 Moderate（实测夹具名 `group_heavy_plus_three_directs` → Moderate）。

### 4.2 F04c 复燃一响（继承自 T4，回退触发器）

七年休眠 + 昨天一来一回 → Strong（T4 与 T4D 同判）。v0.1 已知限制，**禁止**为此静默加第三道门（如「近 90 天 ≥3 次才免降」）——那是新候选，须重开消融。处置见第 5 节。

### 4.3 任一场地时钟的不对称面

一对一历史停在 200 天前、群聊昨天还活跃 → 不降档（实测 `direct_quiet_200_group_yesterday` → Strong）。定价：该关系的 Strong 资格来自真实的一对一历史，群聊活跃证明关系未死；且换用 direct-only 时钟的解释成本（模板需第二个日期概念）违反可数性纪律。若 Round X 判此面不可接受，改时钟属于**参数级变更**，走父代理 DECISIONS，不动候选结构。

## 5. 回退链（唯一合法改判通道，全部走父代理 DECISIONS 留痕）

1. **档内回退 T4D → T4**：若 4.1 的群聊代价被产品裁定不可接受。只换计数口径，常量与模板结构不动（T4 模板已随 COPY_ZH.md 冻结待命）。
2. **族外回退 → T3R**：若 **F04c（复燃一响）被裁定不可接受**。接受其对价：折算话术（半次 / 四分之一次）的可数性负债与 F04b 上「档位 vs 半年沉寂句」的自相矛盾风险。这是 T3R 墓碑的唯一复活条件。
3. **任何情况不回退 T0 / 裸 T3**：F03/F04 错档已被三轮钉死。

## 6. Goal 1 采纳方式（冻结后的合并义务）

1. **替换判档**：Goal 1 `graph_build.rs` 的 `Tally::band()`（现为 T0：无场合门、无近因）整体替换为 `crates/soul-algo` 的 T4D 判档；3/10/3 常量从 soul-algo 常量模块导入，`graph_build.rs` 本地常量删除，防两处漂移。Goal 1 的 `Tally` 需补一对一分列计数（direct out/in/自然日）与任一场地 last_contact——均为已有字段的口径细分，不动存储 schema 语义。
2. **依赖方向**：`crates/soul-algo` 保持纯函数、零重依赖、`#![forbid(unsafe_code)]`、不读墙钟；**禁止把 SQLCipher（及任何存储、Tauri、HTTP 依赖）拉进该 crate**。依赖箭头永远是 Goal 1 → soul-algo，评分器只吃 `(peer 行, as_of)`，落库与取证留在 Goal 1。
3. **as_of 传递**：rebuild 调用方计算全库 `max(occurred_at)`（或显式传值）后传入；soul-algo 不提供缺省墙钟路径。
4. **渲染面**：`TieScore` 携带 band、原始计数、一对一/群聊分列、last_contact、沉寂天数与 as_of；A2 与图 UI 只消费该结构。中文模板绑定测试（COPY_ZH.md 第 6 节断言）随 soul-algo 落地，模板改动先改 COPY_ZH.md 再改代码。
5. **过渡期语义**：在替换合并完成前，Goal 1 现行 `Tally::band` 只是遗留行为，不是规范；新写的任何解释/摘要一律以本决议为准。

## 7. 依据归档

- 独立验证：`.agent_workspace/round3/fable-a/t4d-verify/`（Rust 1.83，无依赖，13/13 通过，clippy/fmt 干净；矩阵由 `cargo run` 打印）。
- 交叉复现：`.agent_workspace/round3/gpt-sol-a/`（消融矩阵 + 性能：T4D ≈ 1.001× T0）、`.agent_workspace/round3/gpt-sol-b/`（对抗探针 13/13，含 as_of 陷阱与 direct<10 不可 Strong 性质测试）。
- 裁决程序：`.agent_workspace/round2/fable-a/ABLATION_PROTOCOL.md`；理论边界：C7 满分归 T3 门闩（Granovetter 操作化），T4/T4D 是产品补丁，靠 C8 净胜采纳，不吃理论分；**不验收预测准确率**。
