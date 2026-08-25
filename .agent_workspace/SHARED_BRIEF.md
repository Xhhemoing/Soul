# Soul 算法验证与优化 — 共享任务书（全轮必读）

父代理：cursor-grok-4.6-high（本会话）。仓库 `github.com/Xhhemoing/Soul`。
工作分支：`cursor/algo-verify-opt-a073`（基于 `origin/main`）。
权威产品锁：`.agent_workspace/context/plan/PRODUCT_LOCK.md`（来自 `origin/cursor/soul-product-lock-7b1c`）。
现有实现快照：`.agent_workspace/context/impl/`（来自 `origin/cursor/soul-goal1-7b1c`）。
完整实现仍在远程分支 `origin/cursor/soul-goal1-7b1c`，可用 `git show origin/cursor/soul-goal1-7b1c:<path>` 读取，**禁止**把 Goal 1 整仓拷进本分支。

## 目标（不可稀释）

为项目计划中涉及的算法做验证、优化与创新建议，最后**只保留最优的 1 或 2 个算法**作为 v0.1 灵魂层的规范实现。其余候选归档为「不采用」并写明否决理由。

这不是重写产品，不是启动 Goal 2，不是把 LLM 人格推断做成黑盒。产品锁约束全部有效。

## 计划里真正的「算法」（不是政策管道）

必须正面评估的灵魂层算法：

1. **人脉图关系强度（Tie Strength / evidence band）**
   - 现实现（`graph_build.rs`）：ego 网；互惠 ∧ count≥10 ∧ 活跃天≥3 → Strong；互惠 ∧ count≥3 → Moderate；否则 Weak。
   - 常量：`MODERATE_MIN_INTERACTIONS=3`，`STRONG_MIN_INTERACTIONS=10`，`STRONG_MIN_ACTIVE_DAYS=3`。
   - 无近因衰减、无群聊降权、无时长窗口、无用户可解释公式以外的权重。
2. **特质轴推断（Trait Axes）**
   - 现实现：五条类大五方向轴（非分数、非量表）；问卷 = Moderate + `user_stated`；纠正锁定 = Strong；后续推断不覆盖锁定轴。
   - v0.1 **没有**从聊天正文做行为推断的规则引擎。WP10 人事分析未开工。
3. **证据档映射（weak/moderate/strong）** — 与 1、2 共用词汇，但阈值不同源。
4. **人事分析统计降级（WP10，计划有、代码无）** — 无 LLM key 时必须可跑、禁诊断词、每条有证据。
5. **遗忘后推断 orphan** — 协议已钉，不是本次选型对象（只作约束：任何新算法必须可 orphan）。

次要、默认不进入「保留 1–2 个」竞争（除非你能证明它才是瓶颈）：
- 第三人 redactor（≥8 字连续子串 + 姓名/账号占位）
- 注入检测、审计哈希链、HITL plan_hash、SQLCipher+AEAD

## 硬约束（违反即否决）

- 推断必须有可解引用 `evidence_ids`；无证据不得落库。
- 特质轴禁止 score / percentile / 诊断词；只有 `leans_low|mixed|leans_high|unknown` + 证据档。
- 第三人数据默认 `local_only`；算法输出不得把姓名/正文送出本机。
- 算法必须可向用户解释（能用中文说清「为什么是强/中/弱」）。
- v0.1 无 E0；无 key 时不得依赖 LLM。
- 不承诺临床效度；工作假设必须可纠正、可遗忘。
- Linux 可测；不依赖真实 Windows API。

## 评价维度（所有候选必须打分 1–5 并给证据）

| 维 | 含义 |
|---|---|
| C1 产品锁契合 | 是否服务「电子版的你 / 人脉图 / 可纠正」 |
| C2 可测性 | Given/When/Then；确定性；无 flaky 时间 |
| C3 可解释性 | 用户能复核计数 |
| C4 隐私 | 不需要正文；群聊/第三人安全 |
| C5 稳健性 | 刷屏、单向、群 @、时区、空图、单日 1000 条 |
| C6 计算成本 | O(n) 可接受；禁止隐式 O(n²) 全图 |
| C7 SOTA 对齐 | 是否吸收 Granovetter / RFM / recency-weighted 等已有理论，而不是装作发明 |
| C8 创新必要 | 新花样必须打败基线，否则不留 |

## 已知基线与建议对照候选（可增，不可无故删基线）

**Tie strength 族：**

| ID | 名称 | 要点 |
|---|---|---|
| T0 | Count+Reciprocity+Span（现实现） | 基线 |
| T1 | Recency-weighted exponential decay | `sum(exp(-λΔt))`，互惠门闩仍在；λ 用半衰期 30/90 天 |
| T2 | RFM-band | Recency / Frequency / (Active days as "monetary") 三分位映射到 band |
| T3 | Granovetter-span | 强关系=互惠+多日+私聊；群聊默认弱；单向永远不强 |

**Trait / 人事分析族：**

| ID | 名称 | 要点 |
|---|---|---|
| A0 | Questionnaire + correction lock（现实现） | 基线；v0.1 主路径 |
| A1 | Evidence-count band upgrade | 多条独立证据可把档从 moderate→strong，永不改用户锁定 |
| A2 | Statistical personnel summary | 只基于图计数+最近接触+互惠，无正文、无诊断；WP10 降级路径 |
| A3 | Lexicon/LIWC 从正文推断轴 | **默认高风险**：需要正文、易临床化、难解释；除非打分压过 A0+A1 否则否决 |

最终只保留 **1 或 2** 个：预期形态是「一个人脉算法 + 一个档案/人事算法」，但若数据证明只能留一个，也可以。

## 工作方式（防冲突）

- 你只允许写入你的专属目录（见本轮派发提示）。禁止改别人的目录、禁止 `git commit` / `git push` / 开 PR。
- 产出必须是文件，不要只停在聊天。
- 回复**第一行**必须是：`MODEL_SLUG: <实际使用的 slug>`
- 禁止静默降级。若指定模型不可用，第一行写失败原因并停止，不要换模型硬做。
- 代码用 Rust 1.83，无外部网络调用，无 `unsafe`。参考实现必须纯函数、可单测。
- 不要把 Goal 1 的 SQLCipher / Tauri 依赖拉进本分支。算法 crate 最多用 `serde` / `thiserror` 这类轻依赖。

## 本分支允许的最终落盘位置（由父代理合并，你先写到自己目录）

- `docs/algorithms/` 规范与决策
- `crates/soul-algo/` 参考实现 + 单测 + 基准
- `.agent_workspace/` 过程稿
