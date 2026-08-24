MODEL_SLUG: claude-fable-5-thinking-xhigh

# T4D_SPEC — 人脉关系强度 T4D（T4 的最终形态）精确规范（Round 3 / fable-b）

地位：落实 R2-SYNTHESIS（BINDING）「潜在边界风险 #2」：`any_direct` 布尔闩不够，
Strong/Moderate 的次数与天数门必须改在 **direct（一对一）计数** 上。
本文件是 T4D 的冻结级精确规范：每个量、每个比较、每条边界都写死，禁止实现再解释。

**采纳条件已核验**：R2-SYNTHESIS 规定「若 T4D 打挂 `lilei_12` 则放弃 T4D 留 T4」。
两个 Round 3 独立实现（gpt-sol-a `MATRIX.md`、gpt-sol-b `probes` 13 测）与本文第 7 节手算
一致给出 `lilei_12 → Strong`。条件通过，**T4D 即 T4 的最终形态**，随规范发布的名字为 T4D；
带 `any_direct` 布尔闩的 T4 原形态随本规范废止（去向见第 11 节）。

---

## 1. 输入与记号

对一个 peer 的观测多重集 `I = {i}`，每条观测恰有三个元数据字段（**无正文字段**，编译期封死）：

- `direction ∈ {out, in}`
- `venue ∈ {D, G}`（D=一对一会话，G=群聊；由导入管道标注，见 PIPELINE_DEBT PRE-3）
- `occurred_at`：UTC RFC 3339 时间戳（算术用其 Unix 秒 `t(i)`）

`as_of`：一次 rebuild **全库一个值**，由调用方显式传入；产品语义 = store 内全部观察的最大
`occurred_at`（PIPELINE_DEBT PRE-5，R2 已裁决 store 级，文件级作废）。
**禁止** per-peer `max(occurred_at)`（会让休眠关系假 Strong——gpt-sol-b Round 3
`trap_peer_local_fixture_max_wrongly_revives_f_old` 已演示）。禁读墙钟。

日算术全整数：`utc_day(t) = t.div_euclid(86400)`（负 Unix 秒安全）。

## 2. 汇总量（单遍 O(n)，全整数）

| 量 | 定义 |
|---|---|
| `direct_out` | `|{i : venue=D ∧ direction=out}|` |
| `direct_in` | `|{i : venue=D ∧ direction=in}|` |
| `group_out` | `|{i : venue=G ∧ direction=out}|` |
| `group_in` | `|{i : venue=G ∧ direction=in}|` |
| `direct_count` | `direct_out + direct_in` |
| `interaction_count` | 四列之和（向后兼容字段，= 原 `count`） |
| `direct_days` | `|{utc_day(t(i)) : venue=D}|`（**只数一对一事件的自然日**） |
| `active_days` | `|{utc_day(t(i))}|`（全场地，仅展示，不进判档门） |
| `last_contact` | `max(t(i))`，**任一场地**（判档降档与展示共用这一个） |
| `last_direct_contact` | `max(t(i) : venue=D)`（若无 D 事件则缺省；仅展示） |
| `direct_reciprocal` | `direct_out ≥ 1 ∧ direct_in ≥ 1` |
| `age_days` | `max(0, (as_of − last_contact).div_euclid(86400))`（未来时间戳钳 0） |

`interaction_count == 0` 时不做任何时间运算（`last_contact`/`age_days` 不定义）。

## 3. 判定公式（精确，两段，全整数比较）

```text
第一段（计数门，只消费 direct 列）：
band0 = Strong    if direct_reciprocal ∧ direct_count ≥ 10 ∧ direct_days ≥ 3
      = Moderate  if direct_reciprocal ∧ direct_count ≥ 3
      = Weak      otherwise

第二段（近因降档，时钟 = 任一场地的 last_contact）：
if interaction_count == 0:  band = Weak          # 空观测，不进入时间运算
elif age_days ≥ 360:        band = Weak          # 闭区间：恰 360 天 → Weak
elif age_days ≥ 180:        band = demote_one(band0)   # 闭区间：恰 180 天 → 降一档
else:                       band = band0

demote_one: Strong → Moderate；Moderate → Weak；Weak → Weak
```

与 T4 的全部差异在第一段消费哪些列；第二段与 T4 **逐字节相同**（opus-a Round 3
`recency.rs` 即共用实现）。

## 4. 常量表（单源，禁止第二处定义）

| 常量 | 值 | 出处 |
|---|---|---|
| `MODERATE_MIN_INTERACTIONS` | 3 | Goal 1 `graph_build.rs`，R2 冻结 |
| `STRONG_MIN_INTERACTIONS` | 10 | 同上（T3 的 count≥8 已删除） |
| `STRONG_MIN_ACTIVE_DAYS` | 3 | 同上 |
| `DEMOTE_AFTER_SILENT_DAYS` | 180 | R2 父代理裁决，**闭区间 `≥`** |
| `WEAK_AFTER_SILENT_DAYS` | 360 | 同上，**闭区间 `≥`** |

**T4D 没有 `GROUP_ONLY_CEILING` 常量，也没有 `STRONG_REQUIRES_DIRECT` 布尔闩**：
group-only 边的 `direct_count = 0`，在第一段结构性落 Weak，无需天花板；
`band = Strong ⟹ direct_count ≥ 10` 是公式的结构后果，不是约定（gpt-sol-b 用
direct 0–9 × group {0,1,12,100} 全交叉钉死）。

## 5. 语义钉子（逐条，含理由与残余风险）

**D1 — 三个门全部直连**：Moderate 与 Strong 的次数门、天数门、互惠门只消费 direct 列。
`any_direct` 布尔闩废止（被 `direct_count ≥ 3/10` 蕴含且更强）。

**D2 — 群计数存而不判**：`group_out/group_in` 照常汇总、入 TieScore、入展示语
（「群里同场 {Y} 次」，PIPELINE_DEBT M-A2），但**不开启 Strong，也不开启 Moderate**。
后果：group-only 互惠边从 T4 的 Moderate 变为 **Weak**。这是本规范唯一相对 R2 实测表的
改档（见第 7 节），属刻意裁决而非滑动：它同时关闭了 PIPELINE_DEBT §1.3-2 的
「200 人活跃群人人 Moderate」残余。所有 R2 钉死期望均为封顶式（「≤ Moderate」「非 Strong」），
Weak 全部满足，零违规。群关系不消失：边仍在图上，计数与 last_contact 照常展示。

**D3 — 降档时钟取任一场地**：`last_contact = max over 全部 venue`。
昨天在群里同场说过话，就不算休眠——群消息是「此人没有从你生活中消失」的证据，
即便它不构成亲密证据。后果与残余风险（如实钉死，禁止粉饰）：

- 保住的直觉：把日常闲聊搬进群里的老朋友，不会因为一对一沉寂而被降档。
- 付出的代价：不变式从「Strong ⟹ 180 天内有**一对一**往来」弱化为
  「Strong ⟹ 180 天内**任一场地**有往来」。一段一对一历史达标、此后只在群里同场的
  关系会无限期维持 Strong（夹具 X2 钉住此行为是**有意的**）。
  若 Round X 判此不可接受，唯一合法修法是把降档时钟改读 `last_direct_contact`
  ——那是常量表之外的一处单点改动，禁止加第三道门。

**D4 — 闭区间**：`≥ 180` 降一档、`≥ 360` 封 Weak（父代理 R2 裁决；gpt-sol-b Round 2
的 `> 180` 版本作废，其 Round 3 探针已自我更正并钉住 179/180/359/360 四格）。

**D5 — 空观测与未来时间戳**：`interaction_count == 0 → Weak`，零时间运算；
`t(i) > as_of` 的观测年龄钳 0（不制造负年龄，不 panic）。

**D6 — 纯函数纪律**：判定是 `(观测多重集, as_of) → band` 的纯函数；输入置换不变；
时间戳与 as_of 同步平移不变；无浮点；无墙钟；无正文字段可达；O(n) 单遍。

## 6. TieScore 分列（PIPELINE_DEBT M-A1 定型）

`TieStrength` 输出字段（`relationship.schema.json` 的 `tie_strength` 是自由 object，零契约变更）：

```text
band, direct_outgoing, direct_incoming, group_outgoing, group_incoming,
direct_active_days, active_days, interaction_count (=四列之和),
first_contact, last_contact (任一场地), last_direct_contact (可缺省),
conversations (沿用现状，仅展示)
```

`any_direct` 不再是独立字段，由 `direct_outgoing + direct_incoming > 0` 推导。
WP10/A2 只读 TieScore，不读证据行，不自带阈值（GRAPH_CORRECTION 的
`locked_by_user/user_band/machine_band` 三个可选字段与此并存，锁定语义见该规范）。

## 7. 钉死夹具

### 7.1 主夹具 `group_heavy_plus_one_direct_each_way`（本规范的存在理由）

per-peer 观测视图，`as_of = 2026-08-24T14:00:00Z`（Unix 1787580000，全夹具统一）：

- 群阶段：200 发言者群，owner 发 10 条，分布在距今 9/8/7 天的 3 个 UTC 日
  （4+3+3 条）。导入扇出 → 本 peer 得 `group_out = 10`。peer 本人在这几天群里发 5 条
  → `group_in = 5`。
- 直连阶段：距今 2 天 1 条 direct out；距今 1 天 1 条 direct in。

汇总：`direct_out=1, direct_in=1, direct_count=2, direct_days=2；group=15；
interaction_count=17；active_days=5；age_days=1`。

| 规则 | 推演 | 结果 |
|---|---|---|
| T4（原形态） | 互惠 ✓（11 出/6 入）∧ count 17≥10 ∧ days 5≥3 ∧ any_direct ✓ ∧ age 1<180 | **Strong ✗** |
| **T4D** | `direct_count 2 < 3` → band0 = Weak → Weak | **Weak ✓** |

**钉死期望：必须非 Strong**（PD-A2 的「≤ Moderate」由 Weak 满足）。规模含义：同一
10 条群消息扇出 ~200 条边，T4 下每个「同群 + 各一句问候」的成员都 Strong；T4D 下全为 Weak。

已由两个独立实现证实（参数变体，同一形状同一结论）：
gpt-sol-a（36 群 + 各 1 直连 → T4 Strong / T4D Weak）、
gpt-sol-b（100 群 + 各 1 直连 → T4 Strong / T4D Weak；并以 direct 0–9 × group ≤100
全交叉证明 T4D 在 `direct_count < 10` 下**不可能** Strong）。

### 7.2 降档时钟夹具（钉 D3）

| ID | 构造 | T4D 推演 | 钉死期望 |
|---|---|---|---|
| X1 `direct_strong_no_lifeline` | 12 条 direct 互惠（6 出 6 入）跨 6 UTC 日，全部在距今 200–205 天 | band0=Strong；age=200 ≥180 → 降 | **Moderate** |
| X2 `direct_strong_group_lifeline` | X1 + 距今 1 天 1 条 group in | band0=Strong；last_contact 任一场地 → age=1 <180 | **Strong**（昨天的群消息挡住休眠降档——D3 本体） |
| X3 边界混场 | 12 条 direct 互惠 6 日在距今 190 天 + 1 条 group 在恰距今 **180** 天 | age=180 ≥180 → 降 | **Moderate**（闭区间与任一场地时钟共同生效） |
| X4 边界混场 | 同 X3 但 group 在距今 **179** 天 | age=179 <180 | **Strong** |

### 7.3 已知限制保全（禁止静默修）

`F04c 复燃一响`（dormant_2019 的 20 条 direct + 昨天各 1 条 direct）：
`direct_count=22 ≥10, direct_days=11 ≥3, direct_reciprocal ✓, age=1` → T4D = **Strong**。
与 T4 同。R2-SYNTHESIS 裁定为 v0.1 已知限制，**禁止加第三道门**；判不可接受的出口
仍是整体回退 T3R，不是给 T4D 打补丁。

### 7.4 全量对照（opus-a Round 2 十六夹具 + 本轮新增；改档格加粗）

| # | 夹具 | direct 计数/天 | age(任一场地) | T4 | T4D | 钉死期望 | 判 |
|---|---|---|---:|---|---|---|---|
| 1 | empty | 0/0 | — | Weak | Weak | Weak | ✓ |
| 2 | single_inbound | 1/1 | 1 | Weak | Weak | Weak | ✓ |
| 3 | lilei_12 | 12/6 | 3 | Strong | Strong | Strong | ✓（采纳条件） |
| 4 | afternoon_20 | 20/1 | 1 | Moderate | Moderate | Moderate | ✓ |
| 5 | group_only_50 | 0/0 | 1 | Moderate | **Weak** | ≤ Moderate | ✓（改档，D2 裁决） |
| 6 | one_sided_100 | 100/100（0 入） | 1 | Weak | Weak | Weak | ✓ |
| 7 | flood_1000_in_one_day | 1000/1 | 1 | Moderate | Moderate | 非 Strong | ✓ |
| 8 | steady_16_over_8_weeks | 16/8 | 7 | Strong | Strong | Strong | ✓ |
| 9 | quiet_179_days | 12/6 | 179 | Strong | Strong | 未判 | 同 T4 |
| 10 | quiet_180_days | 12/6 | 180 | Moderate | Moderate | 非 Strong | ✓ |
| 11 | quiet_200_days | 12/6 | 200 | Moderate | Moderate | 非 Strong | ✓ |
| 12 | revived_after_gap | 32/12 | 4 | Strong | Strong | Strong | ✓ |
| 13 | group_only_quiet_200 | 0/0 | 200 | Weak | Weak | 非 Strong | ✓ |
| 14 | dormant_359_days | 12/6 | 359 | Moderate | Moderate | 非 Strong | ✓ |
| 15 | dormant_360_days | 12/6 | 360 | Weak | Weak | 非 Strong | ✓ |
| 16 | dormant_2019 | 20/10 | 2632 | Weak | Weak | 非 Strong | ✓ |
| 17 | group_heavy_plus_one_direct_each_way | 2/2 | 1 | **Strong ✗** | **Weak** | **非 Strong** | ✓（本轮净胜） |

T4D 相对 T4 恰改两格（#5、#17）：#17 是净胜（修 bug），#5 是 D2 的裁决后果；
两格都满足全部钉死期望，其余 15 格逐格相同。**零回归，净胜 1。**

## 8. 不变式（性质测试逐条钉）

| # | 不变式 |
|---|---|
| V1 | `band = Strong ⟹ direct_reciprocal ∧ direct_count ≥ 10 ∧ direct_days ≥ 3`（结构性，非约定） |
| V2 | `band ≥ Moderate ⟹ direct_reciprocal ∧ direct_count ≥ 3`；group-only ⟹ Weak |
| V3 | `Strong ⟹ age_days < 180`；`≥ Moderate ⟹ age_days < 360`（age 按任一场地；见 D3 弱化说明） |
| V4 | direct 单向（direct_in=0 或 direct_out=0）⟹ Weak，群计数再大也不例外 |
| V5 | 新增一条观测永不降档；遗忘一条观测永不升档（direct 列单调 + 梯子单调 + 降档只随 age 增大而加深） |
| V6 | 置换不变；(全部 t, as_of) 同步平移不变；纯函数、无浮点、无墙钟、O(n) 单遍 |
| V7 | 无群观测的输入上 T4D ≡ T4 ≡ T3 梯子（随机 direct-only 输入性质测试） |

## 9. 中文话术差分（草案；冻结权归 fable-a，基底 = COPY_ZH.md 第 3 节 T4 模板）

改动只有两类，其余逐字沿用 COPY_ZH T4 模板（透明句、收尾句、日期格式、禁词表全部不变）：

1. 判档句里的 `{次数}/{天数}` 一律改为**一对一**口径并明说：
   「你们一对一互相都发过消息，一对一往来 {一对一次数} 次，分布在 {一对一天数} 个自然日里…」
2. 凡 `group_count > 0` 的边追加分列句（M-A2，禁止混合总数单独示人）：
   「另外你们在群里同场往来 {群次数} 次，这部分单独展示，不参与档位判定。」
3. 新增 group-only 弱档句：「到目前为止你们只在群里说过话（同场 {群次数} 次），
   还没有一对一的记录，先算弱。这是工作假设，你可以直接改。」（替换 T3/T4 的群聊「最多算中等」句，后者随 T4 原形态作废。）

可数性判据（R2 平局裁决 S4 的延续）保持成立：用户复核 T4D 仍只需数**一对一**消息条数、
数其日期个数、算最后一次联系距截止日期的整数天数差——全程无乘法、无折算、无小数。

## 10. 验收测试（Given/When/Then；并入 ACCEPTANCE_GAP §2 清单）

| ID | Given | When | Then |
|---|---|---|---|
| TD-1 | §7.1 主夹具 | 判档 | T4D=Weak；断言 T4 原形态在同输入 = Strong（对照保留，防夹具退化） |
| TD-2 | direct 0–9 × group {0,1,12,100} 全交叉，互惠、多日、age<180 | 判档 | 无一 Strong |
| TD-3 | X1–X4 | 判档 | Moderate / Strong / Moderate / Strong（D3+D4 联合） |
| TD-4 | 179/180/359/360 四格（direct-only） | 判档 | Strong / Moderate / Moderate / Weak（闭区间） |
| TD-5 | §7.4 全表 | 矩阵打印 | 与表逐格相同；表由代码生成，禁手抄 |
| TD-6 | 任意夹具 | 读 TieScore | 四列分列与手数一致；`interaction_count=四列之和`；`direct_days` 只数 D 事件日 |
| TD-7 | 随机 direct-only 输入 ×1000 | T4D vs T4 | band 全等（V7） |
| TD-8 | 任意夹具 + 逐条遗忘/新增 | 重判 | V5 单调性成立 |
| TD-9 | F04c | 判档 | Strong（已知限制在案，非缺陷；防止有人静默加门） |
| TD-10 | 渲染 §9 模板 | `assert_non_clinical` + 结构匹配 | 通过；出现「一对一 X 次」「群里同场 Y 次」两个独立数字 |

## 11. 与 T4/T3R 的关系（墓碑安排，供 REJECTED.md 采字）

- **T4（any_direct 布尔闩形态）**：被 T4D 取代。否决夹具 = §7.1（Strong ✗）。
  复活条件：无（它与 T4D 在 direct-only 输入上全等，没有独立存在价值）。
- **T3R**：维持 R2 裁决，墓碑 + 回退候补。复活条件 = F04c 复燃被 Round X 判不可接受。
- **T3 / T0 / T1 / T2 / A3**：维持 R1/R2 墓碑裁决不变。

性能：gpt-sol-a Round 3 实测 T4D ≈ 1.001× T0（69 ns/互动，10k 互动/200 peer），
分列汇总的代价在噪声内。
