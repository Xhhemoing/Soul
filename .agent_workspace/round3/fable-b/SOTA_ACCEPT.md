MODEL_SLUG: claude-fable-5-thinking-xhigh

# SOTA_ACCEPT — 对产品锁的剩余差距与逐项判定（Round 3 / fable-b 终局审计）

判定时点：Round 3 fable-b 交付时（2026-08-24）。opus-a/opus-b 的 Round 3 槽位有在制源码、
尚无报告，本审计按「已到货证据」判定，不预支未到货件。
基准：PRODUCT_LOCK（权威）、SHARED_BRIEF 硬约束、fable-a ACCEPTANCE.md G1–G8、
R2-SYNTHESIS「SOTA 验收差距」清单。

## 1. 头条判定

**算法内容：PASS。**（T4D 公式冻结级成文，两个独立实现同结论、零回归、净胜 1；
A0/A2/A1 的裁决全部落定；理论定位无欠账。）
**ALGO_FROZEN 宣告：FAIL（尚不可宣告）。** 剩余三件纯机械工事（§4），无一涉及算法内容再裁决。
**v0.1 端到端验收：FAIL。** 管道债与图纠正实现未落地（§5，不阻塞冻结，阻塞产品验收）。

## 2. 对产品锁逐项（只列约束到算法工作流的条目）

| 产品锁条目 | 判定 | 证据 / 缺口 |
|---|---|---|
| 灵魂层「人脉图…可查看和**纠正**」+ 不可协商 5/10 | **FAIL（实现缺）** | GRAPH_CORRECTION 规格完备（10 条 G/W/T），但 `correct_tie/release_tie` 未实现，Goal 1 `build.rs` 仍复位 `user_verdict`。§2/§3 语义（band=生效档、rebuild 保留裁决）**必须并入冻结文档**，否则「胜者输出的 band 是谁的 band」无答案 |
| 边=互动强度/最近接触/证据 | PASS | T4D_SPEC §6 TieScore 分列定型；每边全量 `evidence_ids`；`last_contact` 任一场地单一定义。「关系类型」为 v0.2 user-stated 预留位，不在本轮验收面 |
| 推断带证据与弱/中/强档 | PASS | 无证据拒绝（A0）；band 三词封闭词汇（gpt-sol-b 编译期钉死输出词表） |
| 用户纠正锁定且优先 | 部分 PASS | 轴侧：R2 补丁 `apply_intake` 与 replay 全等（opus-b）；**生产路径** `correction_lock.rs` 的 intake 回归测试仍缺（P1-1）。图侧：随图纠正实现走 |
| 禁 score/百分位/诊断词；可解释 | PASS | 三个实现的 denylist 扫描 + `assert_non_clinical` 全绿；T4D 保持整数可数性（复核=数一对一条数、数日期、算整数天差，无乘法） |
| 无 key 可跑（WP10 统计降级） | PASS（渲染器口径） | A2 = TieScore 纯消费者，源码级禁第二套阈值；goal1 `analysis.rs` L244 已核实纯消费 |
| 可遗忘 / orphan 兼容 | PASS（算法侧） | V5 单调性（遗忘永不升档）三实现钉死；管道侧遗忘→rebuild 无陈旧边测试仍在 §5 |
| 第三人数据不出本机 / 无正文 | PASS | 观测仅 3 个元数据字段，正文字段编译期不可达（gpt-sol-b 源码断言拒 body/text/content 等） |
| Linux 可测 / 无墙钟 / 确定性 | PASS | as_of 全库一值裁决 + 双向陷阱测试；置换/平移不变；无浮点 |
| 不承诺临床效度 / 不验收预测准确率 | PASS | ACCEPTANCE §5 纪律全轮维持，无人引入准确率声明 |

## 3. R2-SYNTHESIS「SOTA 验收差距」清单逐项

| 差距项（R2 原文） | 判定 | 现状 |
|---|---|---|
| T4D 夹具 | **PASS（本轮关闭）** | gpt-sol-a 矩阵 + gpt-sol-b 13 探针 + T4D_SPEC §7 规范参数化；`lilei_12` 采纳条件通过；`group_heavy_plus_one_direct_each_way` T4=Strong✗/T4D=Weak✓ 两实现一致 |
| 中文模板绑定测试 | **FAIL（开放）** | COPY_ZH 冻结的是 T4 原形态模板；T4D 差分（一对一口径 + 分列句 + group-only 弱档句）已在 T4D_SPEC §9 成稿，**冻结权在 fable-a**；渲染断言（TD-10）未实现。注意：T4D 使分列句从「建议」升为「必须」——判档只读 direct 列后，混合总数单独示人即误导 |
| 统一 `crates/soul-algo` | **FAIL（开放）** | 仓内无 `crates/` 目录；参考实现散在四个 agent 目录（opus-a R2/R3、gpt-sol-a、gpt-sol-b），常量单源纪律目前靠各自纪律而非结构保证。合并归父代理 |
| 墓碑进 `docs/algorithms/REJECTED.md` | **FAIL（开放）** | 文件不存在；`round2/fable-a/REJECTED_DRAFT.md` 有底稿。须覆盖 T0/T1/T2/T3-as-product/T3R/A3 + **T4 原形态**（否决夹具 §7.1，双格披露义务见 CROSS_CHECK C5）+ A1（TwoKindsAcrossDays 下可证明空转，复活条件 v0.2 第二证据 kind） |
| `ALGO_FROZEN` 声明 | **FAIL（开放）** | `docs/algorithms/README.md` 明文「尚未冻结」。前置的**内容性**缺口本轮全部清零：Strong 消费哪列（T4D）、TieScore 定型、as_of、180/360 闭区间、A1 裁决——剩余全部是 §4 的机械工事 |
| 不验收预测准确率 | PASS | 维持 |

## 4. 宣告 ALGO_FROZEN 前的剩余工事（全部机械，无再裁决面）

1. **墓碑文件**：REJECTED_DRAFT → `docs/algorithms/REJECTED.md`，按 §3 覆盖清单补 T4 原形态与 A1 条目。
2. **模板冻结 + 渲染断言**：fable-a 按 T4D_SPEC §9 差分定稿 COPY_ZH（含 S3 分列句、S7 去重回执句、
   S8 falsifier 常量拼装、S9「由你本人指定」变体），实现 TD-10 断言。
3. **统一 crate**：参考实现合并进 `crates/soul-algo`（常量一个模块、矩阵由代码打印、
   TD-1…TD-10 与 V1–V7 全绿），G3 grep 门把各 agent 目录的历史参考实现标注非规范。
   随后在 `docs/algorithms/DECISION.md` 写 `ALGO_FROZEN`：**保留 T4D + A0**（A2 渲染、A1 空转封存、
   T3R 回退候补），GRAPH_CORRECTION §2/§3 语义与 PIPELINE_DEBT §0 接口契约随冻结文档同批并入。

## 5. 不阻塞冻结、阻塞 v0.1 端到端验收（维持 ACCEPTANCE_GAP 归类，无变化）

去重六件套（PD-B1…B6）、图纠正十件套（GC-1…GC-10）、遗忘→rebuild 无陈旧边（P2-6）、
`+08:00` 偏移时间戳夹具（P1-6）、intake 绕锁生产路径回归（P1-1）、扇出性能债（M-A4）。

## 6. C7 理论定位补记（一句话，供冻结文档采字）

T4D 不新增理论债，反而收紧操作化：Granovetter 的「亲密/相互吐露」代理从布尔闩
（存在过一次一对一）升级为**计量证据**（一对一次数与天数直接过门），更贴近
Marsden & Campbell「亲密度是最佳指标」的结论；降档仍是 Burt/Hawkes 谱系许可的产品补丁。
维持 fable-a 复审的总纲：理论骨架静态，近因与场合是产品层补丁，补丁已在消融中自证（§3 第 1 行）。
