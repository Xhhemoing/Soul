MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round X fable-b — C1 产品锁契合复核：T4D + A0

依据：`.agent_workspace/context/plan/PRODUCT_LOCK.md`（唯一产品权威）、`docs/algorithms/DECISION.md`（ALGO_FROZEN）、`docs/algorithms/COPY_ZH.md`、`crates/soul-algo-tie`、`crates/soul-algo-trait`、Goal 1 快照 `.agent_workspace/context/impl/`。
本轮独立复跑：`cargo test --workspace` 全绿（tie 侧全部套件 + trait 侧 7 套件含 Round X 新增探针）、`cargo clippy --workspace --all-targets` 干净、`cargo fmt --check` 干净、`cargo run --example matrix` 逐行核对 DECISION 第 1/3 节实测值（`lilei_12` → Strong；`group_heavy_plus_one_direct_each_way` → T4 Strong / T4D Weak；179/180/359/360 闭区间边界；`dormant_2019` 沉寂 2632 天 → Weak）。全部一致。

## 1. T4D 对产品锁逐条契合（C1）

| 锁条目 | T4D 的对应 | 判 |
|---|---|---|
| 作者原意 2「复刻人脉图」、5「分析生活中的人与事」 | T4D 的唯一改动就是「群扇出不得制造档位」：200 人群刷屏在 T4/T0 下全员 Strong/Moderate（矩阵 `group_heavy_*` 三个夹具实证），会把复刻出的人脉图污染成群成员名单。T4D 判档只认双方各自选择的一对一往来，档位重新对应「这个人对你重要吗」而不是「你们同群吗」 | ✓（且是 C1 的正向增强，不只是不违反） |
| 自主拍板「人脉图」行：边 = 互动强度 / 关系类型 / 最近接触 / 证据 | `TieScore` 携带 band + 四路分列计数（强度）、互惠/场地事实（关系类型的输入，Goal 1 `types()` 的 Direct/GroupOnly/Reciprocal/OneSided 不受影响）、`last_contact_unix` + `silent_days`（最近接触）；`evidence_ids` 由 Goal 1 rebuild 落库时附上（DECISION 6.2：评分器只吃 peer 行，取证留在 Goal 1） | ✓ |
| 不可协商 6「第三人数据默认不出本机」+ 锁「E0 无代码路径」 | 输入仅 `peer_id / outgoing / occurred_at / venue_direct / conversation_id`——无正文、无姓名、无窗口标题；crate 零运行时依赖、`unsafe_code = "forbid"`、无任何网络代码路径。中文解释里第三人恒为「对方」 | ✓ |
| 不可协商 5「推断必须有证据」+ 切片 3「推断带证据与证据档」 | 档位词 = `SupportedBand` 的 weak/moderate/strong；Goal 1 `tie_inference` 已带 `evidence_ids`、`method: Rule`、falsifier；T4D 不改这一层 | ✓ |
| 不可协商 10「不做不可纠正黑盒」+ 切片 4 | 判档纯函数、整数阈值、无浮点（消融测试钉死）；`explain_zh` 只用用户可自行数出的次数/天数/日期（绑定测试禁拉丁字母、禁小数、禁折算词）；tie 推断带 `UserVerdict`，纠正通道在 Goal 1，未被绕过 | ✓ |
| LLM 行「无 key 时…人脉图…仍可用」 | 确定性规则，与 LLM 零关联 | ✓ |
| 数据面「遗忘 = 销毁密钥 + 派生推断降为 orphaned」 | 分数无状态、可由证据重放：删行重跑，档位自己回落。orphan 约束（SHARED_BRIEF 硬约束）满足 | ✓ |
| 平台行「Linux 仅作 CI 宿主」 | 不读墙钟（as_of 纪律，`as_of_discipline.rs` 钉死 per-peer 陷阱）、不碰 Windows API，本轮全部在 Linux 复跑 | ✓ |
| 心理/临床红线 | 解释文本构造期过禁词筛（分数/评分/百分/诊断词全表） | ✓ |

已知代价与锁的关系（DECISION 第 4 节，逐条对锁复核）：
- **4.1 仅群聊 → Weak**：锁没有任何条目承诺群聊同事的档位；锁要求的是如实展示——A2 场合句与 T4D 分列句照实报群聊次数（「不判档 ≠ 不展示」），钉死期望 group-only ≤ Moderate 仍满足。**与锁无冲突。**
- **4.2 F04c 复燃一响**：锁无近因强度条款；falsifier + 用户纠正 + 第 5 节回退触发器覆盖。**与锁无冲突，如实入档即可。**
- **4.3 任一场地时钟**：恰恰是为了守住锁的「如实」文化——保证 A2 P4 句与档位永不同屏矛盾。**服务于锁。**

## 2. A0 对产品锁逐条契合（C1）

锁「心理模型」行几乎是 A0 的逐字规格：

| 锁原文 | A0 实现 | 判 |
|---|---|---|
| 可替换特质轴（默认五条类大五方向轴） | `AxisId::ALL = 5`（Curiosity/Orderliness/SocialEnergy/Accommodation/…），join key 对齐 `profile.schema.json` | ✓ |
| 不是分数、不是量表 | `Position` 只有 `leans_low/mixed/leans_high/unknown` 四词；`Band` 刻意不实现 `Ord`；denylist 测试扫全部可发布字符串 | ✓ |
| 推断带证据与弱/中/强档 | `EvidenceRef` + `AxisState.evidence_ids`，无证据推断被拒（COPY_ZH §3 有对应话术） | ✓ |
| **用户纠正锁定且优先** | `place()` 单点裁决：锁定轴上非纠正行一律 `RefusedLocked`，答案行仍落证据表（审计友好，对应切片 12）。Round 2 补丁修复的正是 Goal 1 快照里的真实缺陷：`profile_service.rs::intake` 调 `place_axis` **不查** `axis_is_locked`（本轮在快照 163–219 行复核属实），重填问卷会移动已纠正的轴且锁旗仍亮着 | ✓（A0 是把锁的这句话从「有缺陷的现状」修成「成立的规范」） |
| 切片 5「遗忘」 | `EvidenceRef.forgotten` + 重放 ⇒ 删行后轴自行回落，锁旗随纠正行消失而消失 | ✓ |
| 切片 4「纠正后起草语气立即改变」 | 重放确定性 ⇒ 纠正即时生效、后续推断只记录不覆盖（`frozen_defaults.rs` 钉死「Strong 只能来自本人」是 v0.1 数据面性质） | ✓ |

配套件：**A2 纯渲染器**（源码级无阈值，`a2_defines_no_band_thresholds` 读源码断言；P1b 分列句只渲染 T 族算好的两个数、任一缺席整句不出）服务切片 6 无 key 统计降级；**A1 不在保留路径上**（`a1_is_not_on_the_retained_path`），v0.1 单一问卷来源下 TwoKindsAcrossDays 永不触发，与 DECISION 2.4「空转是预期行为」观测等价——注意措辞差异：DECISION 说 A1 是「A0 内部升档规则（默认 TwoKindsAcrossDays）」，crate 实现为「已实现、已测、未接线」。v0.1 下二者不可区分；v0.2 引入第二证据来源时接线与否才成为真决策，届时走父代理 DECISIONS。

**C1 评分维持 5/5（T4D）与 5/5（A0）**：两者不但不违反锁，T4D 修复的漏洞（群扇出制造档位）与 A0 修复的漏洞（intake 绕锁）都是「电子版的你 / 人脉图 / 可纠正」三个 C1 关键词的直接落实。

## 3. Goal 1 剩余采纳步骤（替换 `Tally::band`，按 DECISION 第 6 节落到快照代码）

替换核心（`crates/soul-graph`，快照 `graph_build.rs`）：

1. **扩 `Tally`**：`outgoing/incoming/any_direct` 换成四路分列（`direct_out/direct_in/group_out/group_in`）+ `direct_days` 集合 + `last_direct_contact`。`InteractionRef` 已有 `venue/direction/occurred_at`，纯口径细分，不动存储 schema（DECISION 6.1）。参考形态即 soul-algo `types::Tally`。
2. **删 `Tally::band()`（T0）与本地常量**（`graph_build.rs` 40–44 行的 3/10/3），改为依赖 soul-algo：常量单点导入，判档调 T4D。时间戳在边界处 RFC 3339 → unix 秒（`goal1_fidelity.rs` 已证两种拼法在日界上逐点一致，含 1970 前）。
3. **as_of 穿线**：`rebuild(store)` 增加 as_of 入参（或调用方先算全库 `max(occurred_at)` 再传入）；soul-algo 无缺省墙钟路径，**禁止 per-peer as_of**（`peer_local_as_of_wrongly_revives_the_dormant_tie` 陷阱）。
4. **扩 `TieStrength`**（`graph_model.rs`，序列化进 `SoulRelationship.tie_strength`）：增 direct/group 分列、`direct_active_day_count`、`last_direct_contact`、`silent_days`、`as_of`、降档前档（`Detail::Demoted.band_before`）。纯增列。
5. **降档是 rebuild 的新行为**：`graph.tie.{band}` 语句键会随沉寂翻档——快照的 `is_tie_statement_about` 按前缀匹配、不按档位匹配，幂等更新已天然兼容，无需改重建逻辑；falsifier 文案已预写「更长时间窗口没有新往来即推翻」。
6. **`TieType::GroupOnly` 语义注记**：`types()` 照旧用 `any_direct`；采纳后 group-only 必然伴随 Weak，UI 文案按 COPY_ZH「只在群里说过话」句处理。

档案侧（快照 `profile_service.rs`）：

7. **修 `intake` 绕锁**：逐答案查 `axis_is_locked`；跳过的答案照写证据行与审计（reason `axis_locked_by_user`），并在返回里报告（`apply_intake` / `IntakeReport.ignored` 语义）；移植 `intake_matches_replay` 钉死 intake ≡ 重放。

渲染与合并义务（冻结时已声明为后置，本轮确认仍未完成）：

8. **统一 crate**：DECISION 通篇写 `crates/soul-algo`（单数），现为 `soul-algo-tie` + `soul-algo-trait` 两个 crate、两个 `TieScore` 结构（trait 侧是 A2 输入的 Option 版）。合并成一个 crate、一个 `TieScore`，A2 直接消费 tie 侧真身。
9. **常量归一**：`soul-algo-trait::a2::DORMANT_AFTER_DAYS = 180` 是 DECISION 第 3 节「不得分裂」的那个分裂（跨 crate 无法互引所致）；合并时改为别名 `DEMOTE_ONE_BAND_DAYS`。同时消解一处**一天宽的边界摆动**：降档是闭区间 `>=180`，而 `a2::is_dormant` 用 `>180`（与 COPY_ZH §4 P4「超过 180 天」一致、与 DECISION 常量表「共用同一常量＋闭区间」不一致）——沉寂恰好 180 天时已降档但 P4 追加句不出现。冻结不变式「句子一出现，降档必已发生」在两种写法下都成立，故不阻塞；合并时以 DECISION 为规范统一为 `>=`，COPY_ZH「超过」改「不少于」，走父代理 DECISIONS 留痕。常量名映射也一并对表（`DEMOTE_ONE_BAND_DAYS`↔`DEMOTE_AFTER_SILENT_DAYS`、`FORCE_WEAK_DAYS`↔`WEAK_AFTER_SILENT_DAYS`、`DORMANT_NOTE_DAYS`↔`DORMANT_AFTER_DAYS`：值与语义全对，名字与决议表不同）。
10. **话术对齐**：现 `explain_zh` 机械满足 COPY_ZH §5 断言（无拉丁字母、无小数/折算、数字可重数、T4D 分列句、档位词），但**不是** §1 冻结模板的结构——缺透明句「怎么算的：只统计次数和日期…」与收尾句「这是工作假设，你可以直接改。」，档位说「强联系/中等联系/弱联系」而模板说「达到强的标准/先算中等/先算弱」。COPY_ZH 自declared权威（「改模板先改本文件」）：合并时或改代码就模板、或先改 COPY_ZH 留痕，二选一，不得静默并存。另记一处笔误：DECISION 6.4 引「COPY_ZH.md 第 6 节断言」，实为 §5。
11. **纯度红线不动**：合并后 crate 保持零重依赖、`#![forbid(unsafe_code)]`、不读墙钟；依赖箭头只许 Goal 1 → soul-algo。**T4 保留在 crate 里**（回退链第 1 条的一行回退能力），不得在采纳时顺手删除。

以上 8–10 全部属于 DECISION 第 0 节预先声明的「冻结后的合并义务」，无一改变任何夹具的档位。
