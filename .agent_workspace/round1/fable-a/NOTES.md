# Round 1 / fable-a — 留给 Round 2 的开放问题

按优先级排序。每条给出我的倾向（供 Round 2 直接采纳或推翻，不许挂起）。

## Q1（高）快照漂移：SHARED_BRIEF 说 WP10 未开工，但 goal1 分支 tip 已有实现
`origin/cursor/soul-goal1-7b1c` 的 `fbad85b` 已落地 `crates/soul-draft/src/analysis.rs`（SummaryPoint/PersonSummary/points_for/Counts 降级）。`.agent_workspace/context/impl/` 快照里没有它。
**倾向**：父代理应更新 SHARED_BRIEF 的第 4 条描述，并把 `analysis.rs` 加进 context 快照；A2 的评估基准应以分支 tip 为准（我已按此评估）。其他 Round 1 代理若按「A2 是白纸」做了设计，Round 2 需对齐。

## Q2（高）as_of 时钟政策：判档时刻 vs 展示时刻
我冻结了「as_of = store 内最大 occurred_at」保确定性。副作用：图从不 rebuild 就永不衰减；一次新导入会让所有旧边同时按新 as_of 重算（可能批量降档）。
**倾向**：接受。band 本来就只在 rebuild 时更新（导入/显式刷新触发），推断行里记录 `as_of` 即可审计复现。UI 若想显示「按今天算会是什么档」，那是渲染层的预览，不落库。Round 2 需要验证：批量降档时幂等更新推断行（F09 机制）够不够，会不会刷审计噪音（一次 rebuild 只该有一条审计聚合条目——现实现已如此，钉住别退化）。

## Q3（中）跨午夜凑天数（EVAL_MATRIX F13）
3 个连续午夜前后各发一条可在 48 小时内凑出 3 个自然日 → 配合 eff_count≥10 可拿 Strong。
**倾向**：v0.1 接受为文档化限制（真实滥用面小，攻击者是用户自己）。若 Round 2 想堵：加 `span = last−first ≥ 72h` 作为 Strong 第四闩，代价是解释话术多一句。做消融再定，不许拍脑袋加。

## Q4（中）A1 在 v0.1 数据面上是否空转
v0.1 的证据源只有问卷、纠正、导入的交互观测。轴方向的「独立证据」在 v0.1 实际只可能来自问卷重填（被 F16 排除）与纠正（直接锁死，轮不到 A1）。
**倾向**：A1 大概率是「正确但空转」，归档并注明复活条件（v0.2 行为证据接入）。Round 2 请先确认是否存在我漏掉的第三种同轴证据源（例如导入的 soul-import-v1 里带 self-statement 类记录？查 schema），再执行归档。

## Q5（中）conversation_count（多路性 multiplexity）该不该进判档
现实现统计了 distinct conversations 但没用于 band。Granovetter 谱系里多情境接触是强度信号。
**倾向**：v0.1 不进判档（多一个维度=多一句解释负担），保留为 A2 的描述句（P1 已含）。Round 2 若有夹具证明它修正某个错档再议。

## Q6（中）群聊方向语义：@提及与回复是否算「对我说」
Telegram 群里对方发言 ≠ 对我说话。现实现把群内对方消息计入 in（只要 importer 归到该 peer 的 InteractionRef）。这会抬高群重度联系人的互惠判定——被 T3R 的 group-only 封顶部分兜住，但「群内互惠」语义仍粗糙。
**倾向**：v0.1 靠封顶兜底即可；Round 2 请查 importer（`soul-import/telegram.rs`）实际怎么生成群聊 InteractionRef 的 direction/peer 归属，把行为写进夹具钉死，避免各实现假设不一致。

## Q7（低）band 抖动与滞回（hysteresis）
分桶衰减下，边可能在 rebuild 间在 Strong/Moderate 边界来回跳。滞回（升档阈值≠降档阈值）能稳定 UI，但双阈值让解释变复杂（「为什么他还是强？因为掉档要低于 8」）。
**倾向**：v0.1 不做滞回，接受跳档——每次跳档都有可复现算式，诚实优先。Round 3 若用户视角评审认为跳档观感差，再议。

## Q8（低）30 天半衰期的用途
我在 T1/T3R 里否决了 H=30 作判档参数（太抖）。但「最近 30 天」作为 A2 的描述句窗口（「近一个月你们往来 N 次」）无害且直观。
**倾向**：留给 A2 做可选描述句参数，不进 band 判定。

## Q9（低）参考实现的落点与依赖
SHARED_BRIEF 允许 `crates/soul-algo`，轻依赖。T3R 需要的全部输入是 `(direction, venue, occurred_at)` 三元组序列——建议 opus-a 的参考实现直接以此为输入类型，不搬 soul-schema 的重类型，保持纯函数 + proptest 幂等/单调性性质测试（加一条消息 eff_count 单调不减；时间平移不变性：全部时间戳整体平移，band 不变）。
**倾向**：性质测试清单：单调性、平移不变、置换不变（输入顺序无关）、遗忘子集一致性（删证据后重算 = 用剩余证据直接算）。这四条对所有 T 候选通用，建议 gpt-sol-a 做成共享夹具。

## Q10（低）「forgotten evidence 降档」与推断行的 falsifier 文案
现实现 falsifier 是固定中文句。T3R 下降档原因多了「衰减」一种，falsifier 应能区分「长期无往来（衰减）」与「证据被遗忘」。
**倾向**：falsifier 模板按 band 计算路径参数化，Round 2 实现时顺手做，不单开工作项。
