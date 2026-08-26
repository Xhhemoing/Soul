MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 3 / fable-a — SOTA 文档冻结报告：ALGO_FROZEN，胜者 T4D

## 1. 一句话结论

**冻结成立。** 冻结条件「T4D 采纳 iff `lilei_12` 保持 Strong ∧ `group_heavy_plus_one_direct_each_way` 非 Strong」由本槽位亲手编码验证（`t4d-verify/`，13/13 测试通过，clippy/fmt 干净），两个条件同时满足，且与本轮两份已完成的独立实现（gpt-sol-a 消融、gpt-sol-b 对抗探针）逐格一致。`docs/algorithms/DECISION.md` 以 `ALGO_FROZEN` 开头、剩余阻塞项为空清单。

## 2. 为什么本槽位自己写了代码

任务书规定：T4D 未经本槽位编码则只能写 `ALGO_PENDING_T4D`。开工时 round3 兄弟槽位尚在写作中（opus-a 的 `t4d.rs` 未落地、tests 目录为空），文档冻结不能建立在半成品传闻上。于是在本槽位目录内写了零依赖验证 crate `t4d-verify`（Rust 1.83，`#![forbid(unsafe_code)]`，无浮点、不读墙钟），把冻结条件变成可判定命题。判定后再与已完成的 gpt-sol-a / gpt-sol-b 结果对读，三方一致。

## 3. 实测矩阵（由 `cargo run` 打印，未手抄）

as_of = 2026-08-24T14:00:00Z（全库单值，调用方传入）：

| Fixture | count | days | direct | direct days | silent | T4 | T4D |
|---|---:|---:|---:|---:|---:|---|---|
| empty | 0 | 0 | 0 | 0 | 0 | Weak | Weak |
| single_inbound | 1 | 1 | 1 | 1 | 1 | Weak | Weak |
| lilei_12 | 12 | 6 | 12 | 6 | 3 | Strong | **Strong** |
| afternoon_20 | 20 | 1 | 20 | 1 | 1 | Moderate | Moderate |
| group_only_50 | 50 | 50 | 0 | 0 | 1 | Moderate | **Weak** |
| one_sided_100 | 100 | 100 | 100 | 100 | 1 | Weak | Weak |
| steady_16_over_8_weeks | 16 | 8 | 16 | 8 | 7 | Strong | Strong |
| dormant_2019 | 20 | 10 | 20 | 10 | 2632 | Weak | Weak |
| f04b_semi_dormant_44 | 44 | 22 | 44 | 22 | 190 | Moderate | Moderate |
| f04c_revived_one_ping | 22 | 11 | 22 | 11 | 1 | Strong | Strong |
| group_heavy_plus_one_direct_each_way | 102 | 50 | 2 | 2 | 1 | **Strong ✗** | **Weak ✓** |
| group_heavy_plus_three_direct_each_way | 106 | 50 | 6 | 3 | 1 | Strong | Moderate |
| direct_quiet_200_group_yesterday | 14 | 7 | 12 | 6 | 1 | Strong | Strong |
| direct_quiet_200_alone | 12 | 6 | 12 | 6 | 200 | Moderate | Moderate |

不变式测试另行钉死：179/180/359/360 闭区间边界、单向永 Weak、群聊永非 Strong、沉寂只降不升、T4D `Strong ⟹ direct≥10 ∧ direct天数≥3 ∧ 沉寂<180`、置换不变、时间平移不变、as_of 陷阱（per-peer as_of 会把 2019 休眠关系错判 Strong——沉寂 2632 天在全库 as_of 下正确封 Weak）、F04c 双规则同判（回退触发器完好）。

T4D 相对 T4 的全部翻档只有两类，均已在 DECISION.md 定价：(a) 群扇出 + 各一句问候从假 Strong 落到 Weak（本轮立法目标）；(b) 仅群聊者从 Moderate 落到 Weak（直连门的直接后果，顺带消解 200 人群刷全员 Moderate 的扇出债放大面；自愈便宜——每方向 3 条一对一即回 Moderate）。

## 4. 冻结中钉死的两处规范细节

1. **T4D 降档时钟读任一场地**（与 T4 同一时钟）。实测 `direct_quiet_200_group_yesterday`（一对一停 200 天、群聊昨天活跃）→ 不降档。理由：A2 的「最近半年没有往来」句读任一场地 last_contact，同一时钟才保证该句一出现降档必已发生，同屏永不矛盾。**gpt-sol-b 本轮的 direct-only 时钟变体作废**，已进 REJECTED.md 附录（该分歧不影响两张决胜夹具，两实现在其上仍逐格一致）。
2. **群聊 Moderate 门也走 direct 计数**（R2-SYNTHESIS 原文「Strong/Moderate 次数与天数门改在 direct 计数上」），故 `GROUP_ONLY_CEILING` 常量降为 T4 专用；T4D 无需封顶——一对一计数为 0 自然落 Weak。

## 5. 交付清单

| 文件 | 内容 |
|---|---|
| `docs/algorithms/DECISION.md` | `ALGO_FROZEN`；阻塞项空清单；保留 T4D+A0（A2 渲染器、A1 TwoKindsAcrossDays 空转、T4 档内回退）；常量表（3/10/3、180、360 闭区间、as_of 纪律、T4D 直连计数规则）；已知代价（F04c、群聊 Weak、任一场地时钟）；回退链（T4D→T4→T3R，F04c 为 T3R 唯一复活条件）；Goal 1 采纳方式（`Tally::band` 整体替换为 soul-algo T4D，常量单点导入，SQLCipher 禁入该 crate） |
| `docs/algorithms/REJECTED.md` | 六座墓碑：T0、T1、T2、T3-as-product、T3R-as-default、A3；附作废变体记录（direct-only 时钟、1000× milliscale、T3 群聊附加门） |
| `docs/algorithms/COPY_ZH.md` | 胜者模板：T4D（新冻结，含「弱（只在群里）」等直连门专属句）、T4（回退形态，自包含内联）、A0、A2（新增 P1b 场合分列句）；验收断言 7 条（新增门槛引用一致性、A2 禁自算分列） |
| `round3/fable-a/t4d-verify/` | 验证 crate：13 测试 + 矩阵打印器 |
| 本文件 | 报告 |

按任务书未做 git 提交；除上列 docs/ 三个白名单路径外只写入本槽位目录。

## 6. 留给父代理 / Round X 的事

1. **合并义务**（冻结后工作，非阻塞）：统一 `crates/soul-algo`（opus-a 本轮 crate 是天然底本，注意其 T4D 需与 DECISION.md 第 3 节冻结定义对读）；COPY_ZH 第 5 节断言写成绑定测试；Goal 1 `Tally::band` 替换。
2. **Round X 复核点**：两张决胜夹具在 opus-a 最终实现上的复跑（预期一致——其 `recency.rs` 已按任一场地时钟成文）；4.1「仅群聊 Weak」与 4.3「时钟不对称」两处定价是否被产品接受。改判只走 DECISION.md 第 5 节回退链 + 父代理 DECISIONS，不许就地改期望。
3. **A1 空转确认**：v0.1 单证据源下 TwoKindsAcrossDays 永不触发，属预期；若 v0.2 增加第二来源，触发条件已冻结，无需再议。
