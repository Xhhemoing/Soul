MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 1 / fable-b — T0 与 A0 现状审计（对照 PRODUCT_LOCK 与 AC 矩阵）

审计对象（全部读自 `origin/cursor/soul-goal1-7b1c`，未拷贝入本分支）：

- T0：`crates/soul-graph/src/build.rs`（band 判定）、`src/interaction.rs`、`src/model.rs`、`src/view.rs`、`tests/ego_graph.rs`；上游 `crates/soul-import/src/{commit,telegram,soul_import_v1,instant}.rs` 与 `tests/import_to_graph.rs`；下游消费者 `crates/soul-draft/src/analysis.rs`（WP10 人事摘要）。
- A0：`crates/soul-profile/src/{service,axes,questionnaire,numeric}.rs`、`tests/{questionnaire_intake,correction_lock,axes_and_evidence}.rs`；以及 `crates/soul-import/src/questionnaire.rs`（并存的第二套问卷）。

结论先行：**T0 的 band 骨架（互惠 ∧ 计数 ∧ 跨天）方向正确，但它的输入管道在群聊上做了乘法扇出，使「Strong」在真实 Telegram 数据上对整群旁观者成立；且图完全不可纠正，rebuild 会无条件复位 `user_verdict`，这直接违反 PRODUCT_LOCK 灵魂层「人脉图……用户可查看和纠正」。A0 的锁语义在被测试的那条路径上是对的，但 `intake` 重跑会绕过锁改写已纠正的轴——被测试守住的是一个 v0.1 里根本不存在的推断生产者，而真实存在的第二个写入者（问卷）恰好没被守住。**

---

## 一、T0：Count + Reciprocity + Span

### 1.1 算法本体（引用）

```118:131:crates/soul-graph/src/build.rs
    fn band(&self) -> SupportedBand {
        let count = self.interaction_count();
        let days = self.active_days.len() as u64;
        if self.is_reciprocal()
            && count >= STRONG_MIN_INTERACTIONS
            && days >= STRONG_MIN_ACTIVE_DAYS
        {
            SupportedBand::Strong
        } else if self.is_reciprocal() && count >= MODERATE_MIN_INTERACTIONS {
            SupportedBand::Moderate
        } else {
            SupportedBand::Weak
        }
    }
```

常量：`MODERATE_MIN_INTERACTIONS=3`、`STRONG_MIN_INTERACTIONS=10`、`STRONG_MIN_ACTIVE_DAYS=3`。互惠 = `outgoing > 0 && incoming > 0`（单条回复即闩上）。`Venue`（direct/group）、`conversation_count`、`first/last_contact` 全部**采集了但不参与 band**。

### 1.2 测试证明了什么（逐条）

`ego_graph.rs` + `import_to_graph.rs` 实际证明：

1. 3/4 个对话对象 → 每人一条边，evidence 全部可解引用（AC-08、AC-06 图侧）。
2. 12 条消息、6 个 UTC 日、互惠、direct → Strong；2 条群内互换 → Weak + `GroupOnly`。
3. rebuild 幂等：二次导入更新同一批 edge/inference id，不长第二张图。
4. 缺失 contact 的 evidence 计入 `peers_unresolved` 而非静默丢弃或成边。
5. 空库 → 空图不报错；双 owner → `AmbiguousOwner` 拒绝。
6. 所有第三人节点与边 `local_only`、evidence `exportable_to_research=false`。
7. tie inference 有 `falsifier`、`statement_key` 前缀、与边共享同一份 evidence 列表。
8. 群内 1 条自发消息 → 每个发过言的人各得 1 条 outgoing 观察（`a_message_sent_to_a_group_is_evidence_of_contact_with_everyone_in_it`，**扇出是被测试钉住的有意行为**）。

### 1.3 没被测试的（同样逐条，全部可复现）

1. **Moderate 档从未被断言过。** 全部图测试只出现 `SupportedBand::Strong` 与 `Weak`（`ego_graph.rs` L209、L225）。fixture 里 wangxiao（out 1 / in 2，count=3，互惠）恰好是 Moderate，但没有任何 assert 碰它。三档算法测了两档。
2. **所有阈值边界未测**：count=9 vs 10、days=2 vs 3、count=2 vs 3（Weak/Moderate 边界）。
3. **`STRONG_MIN_ACTIVE_DAYS` 的立法理由未测**。`build.rs` L42-43 注释写明 "twenty messages in one afternoon is one conversation, not a habit"，但没有一个「单日 20 条互惠」的 fixture 断言它落在 Moderate。常量的辩护场景不在测试里。
4. **单向刷屏未测**：1000 条纯 incoming（频道/骚扰）应为 Weak，无测试。
5. **`TieType::OneSided` 无任何测试**，而且（见 1.5.4）它的 outgoing 方向在现有管道里**构造不出来**。
6. **跨 UTC 午夜、非 Z 偏移时间戳、同日多会话**均未测。
7. **重复导入同一文件**未测（`commit.rs` 文档自认会双写，见 1.5.2）。
8. **遗忘一个 contact 之后 rebuild** 未测：`import_to_graph.rs` 里名为 `evidence_that_survives_a_rebuild_is_evidence_that_still_resolves` 的测试**并没有遗忘任何东西**，只验证了引用闭合。旧边是否残留被遗忘者的计数与时间戳，无测试覆盖（rebuild 只为当前 tally 里的 peer 写边，从不删除失去支撑的旧边）。

### 1.4 对 PRODUCT_LOCK 而言空转（vacuously true）的承诺

1. **「用户可查看和纠正」（灵魂层表格，含人脉图）——纠正半边不存在。** 没有任何 API 能改写一条边的 band、类型或锁定它。更糟：rebuild 每次都整行覆写 tie inference，包括
   ```322:322:crates/soul-graph/src/build.rs
        user_verdict: Some(UserVerdict::Unreviewed),
   ```
   即使将来 UI 写入了 `accepted/corrected`，下一次导入就复位为 `Unreviewed`。schema 里的 `user_verdict` 字段在图侧是装饰。
2. **falsifier 两个半句都不可兑现。**
   ```324:326:crates/soul-graph/src/build.rs
        falsifier: Some(
            "双方在更长的时间窗口内都没有新的往来，或用户直接改写这条关系，即推翻本判断".into(),
        ),
   ```
   前半句：没有任何代码按时间窗口降档（无 recency）；后半句：没有改写入口，且手工改写 relationship 行会被下一次 rebuild 按 pair 匹配后覆写（`build.rs` L225-253）。这条 falsifier 是写给审计看的散文，不是可触发的推翻条件。
3. **「推断必须有证据」在字面上满足、在解释意义上退化。** 边把**全部**观察塞进 `evidence_ids`（强关系可达数万条 UUID）。引用一切等价于什么都没解释：用户无法从 3 万条引用里复核「为什么是强」。真正有判别力的证据（凑齐 10 条 / 3 天的那些行）没有被区分。
4. **ego 网的自我辩护双标。** `build.rs` 模块注释说不建第三人之间的边因为那是 "a guess"——但把「他在群里说话、你也在那个群」记成你们二人的往来，同样是 guess，却被记为可计数事实。

### 1.5 真实 Telegram 规模下会坏什么（按杀伤力排序）

**(1) 群聊扇出让 Strong 对整群成立。** `commit.rs` 的 peer 判定：

```198:207:crates/soul-import/src/commit.rs
        let peers: Vec<Uuid> = match sender.is_owner {
            // The user talking: everyone who has spoken in this conversation
            // heard it. Restricting to people who spoke keeps a large silent
            // group from generating an edge per lurker.
            true => speakers
                .get(&message.conversation_id)
                .map(|set| set.iter().copied().collect())
                .unwrap_or_default(),
            false => vec![sender.contact_id],
        };
```

推论（算术，不是猜测）：owner 在群 G 里发过 M 条，任一发言者 X 发过 K 条，则 tally(X) = outgoing M + incoming K。M≥1 且 K≥1 即互惠；M+K≥10 且散布 ≥3 个 UTC 日即 **Strong**。所以：**用户在一个 200 人活跃群里发 10 条消息，就把该群每一个发过一句话的人都变成 Strong**。题面问「群聊按 1:1 计入」——现实更糟，是 **N:1**：owner 的 1 条群消息被乘成 N 条 outgoing 观察。且 `speakers_by_conversation` 按整个文件汇总，**在 owner 发言之前就退群/之后才入群的人同样被记为「听到了」**。WP10 摘要随后会对这类人输出「你和这个人一共有 137 次往来」——用户用任何方式都数不出这个数，硬约束「算法必须可向用户解释」被输出内容本身证伪。

**(2) Telegram 导出是累积的，重复导入使计数翻倍。** `commit.rs` 模块注释自认："Committing the same file twice writes the events twice. v0.1 has no external-id index… The caller decides." 而 caller（`soulcore/commands/import.rs`）什么也没决定。用户三个月后再导出一次（新文件包含全部旧消息）→ 旧区间计数 ×2 → band 升档。可解释性（C3）在第二次导入后即失效。

**(3) 无 recency。** 2019 年密聊过 10 天的前同事与昨天的挚友同为 Strong，且**永远**是 Strong（计数单调不减）。图随时间只会变得更错。`last_contact_utc` 已在边上，修复成本极低（见 INNOVATION #2）。

**(4) 单条回复闩死互惠。** out=50 / in=1（对方仅回过一句「哦」）→ 互惠成立，count=51、days≥3 → Strong。WP10 的 `direction_statement` 会诚实地说「多数时候是你先开口」，但 band 已经说了「密集」。互惠应有每侧下限或比例，而不是存在性判定。

**(5) 单向 DM 完全隐形。** 在 DM 里对方从未回复时，对方从未作为 sender 出现 → 不在 participants、不在 speakers → **零条 evidence、零 contact、零边**。用户长期单方面联系的人（只读不回的家人）在人脉图上不存在。`model.rs` 声称 `OneSided = "Only one side has ever written"`，但 outgoing-only 这一侧在管道里不可构造——该枚举值只对「别人单方面找我」可达。

**(6) 时间戳双缺陷。** (a) `soul-import-v1` 的 `is_date_time` 接受 `+08:00` 偏移（`instant.rs` L61-73），importer 原样入库；而 `build.rs` L97-98 声称 "every writer in this workspace normalizes to `Z`" 并据此做字典序比较——含偏移的时间戳会把 first/last 排错、把 `utc_date`（取前 10 字符）切成本地日期。声称的不变式被同仓库的另一个 writer 违反。(b) 活跃「天」按 UTC 日切：美洲用户的晚间会话常跨 UTC 午夜（16:00 PST = 00:00 UTC），一次会话记 2 天，两次晚聊即凑满 days≥3；反向也失真（北京周一 23:00 与周二 07:00 落进同一个 UTC 日，两天记一天）。「习惯要跨数日」这条立法在实现里量的不是用户的日子。

**(7) 规模与成本。** 每次 rebuild `list_evidence()` 全表载入内存（每条消息一行，百万级导出 = 百万行 `serde_json::Value`）；边与 tie inference 各自复制全量 `evidence_ids`（5 万条消息的挚友边 = 两份约 1.8 MB 的 UUID 数组，每次 rebuild 重写）；边匹配 `existing_edges.iter().find(...)` 与 inference 匹配 `is_tie_statement_about`（内含每次比较都 `relationship_id.to_string()` 分配）都是 O(peers × existing) —— 数千 peer 时是准 O(n²)，违反 C6 的精神。下游 WP10 `rows_where` 里 `edge.evidence_ids.contains(&row.evidence_id)`（Vec 线性查找）套在 filter 里是**真 O(E²)**：5 万行的边做一次人事摘要要 25 亿次比较。另外频道/bot 不过滤：订阅的广播频道成为一个 Weak「人」，携带数万条 incoming evidence，污染图并放大以上所有成本。

### 1.6 对五个焦点问题的回答（T0 部分）

**「单日 20 条不能 Strong」是否正确？** 方向正确——这是 T0 里最好的一条设计，与 Granovetter 对强关系「时间投入 + 持续性」的定义一致，也是 T3 的核心。但有三点保留：(a) 该立法场景没有测试；(b) 「天」量的是 UTC 桶不是用户的日子（可被两次跨午夜会话凑满，也可把两天压成一天）；(c) 更诚实的度量是「会话段」（session，按 ≥4h 间隔切分）——单日 20 条=1 个 session，这比日历日更贴近「一次交谈 vs 一个习惯」。结论：**保留门槛，修正计量单位，补上测试**。

**群聊 1:1？** 见 1.5(1)：不是 1:1，是 N:1 放大。`Venue` 已经逐条记录，band 却不看它——修复所需的数据已经在库里，这是纯算法层的缺席，不是采集层的。

**无 recency？** 确认，且 falsifier 文本承诺了时间窗口推翻却无人实现（1.4.2）。`last_contact_utc` 就在 `TieStrength` 上，休眠降档（而非浮点衰减）既可解释又零新增采集。

---

## 二、A0：Questionnaire + Correction Lock

### 2.1 算法本体

两枚常量 + 一个锁位 + 一条「锁住则存而不用」的规则：

```51:55:crates/soul-profile/src/service.rs
pub const QUESTIONNAIRE_STRENGTH: SupportedBand = SupportedBand::Moderate;

/// What a correction is worth. The user is looking at the claim and rejecting
/// it, which is the strongest signal this product can get.
pub const CORRECTION_STRENGTH: SupportedBand = SupportedBand::Strong;
```

`record_axis_inference`：evidence 空 → 拒；轴锁定 → inference 入库但不应用（`RefusedAxisLocked`）；否则应用。所有写入过 `reject_numeric_rating` + `assert_non_clinical`。

### 2.2 测试证明了什么

1. AC-03 全链路：7 个回答 → 7 条 `SoulEvidence`（`kind: questionnaire`、`method: user_stated`）→ 5 轴离开 Unknown、band=Moderate、`locked_by_user=Some(false)`；部分作答的轴留 Unknown 且**不引用任何东西**；四类坏输入（数字位置/未知题/题型不符/空卷）原子拒绝——无半成品档案、无审计条目。这一组测试是整个 Goal 1 里质量最高的。
2. 纠正锁：`correct_axis` → Strong + locked + `UserCorrection` 证据可解引用；之后**更强**的推断返回 `RefusedAxisLocked`、轴纹丝不动、被拒推断仍入库可见；锁一条不影响其余四条。
3. 空/悬空 evidence 的推断在两层（service 与 store）都被拒且不留残行。
4. 五个 axis_id 是被 fixture 钉死的 uuid7 常量；denylist 已武装（>60 词）且证明能红；81 种语气组合渲染无禁词。
5. AC-07 语气侧：`set_voice` 后 `suggest_voice` 返回 false、读回用户值。

### 2.3 没被测试的 / 被绕过的

1. **`intake` 重跑绕过锁（本审计最重要的 A0 发现）。** `intake` 先 `read_profile` 读回已有档案（说明重跑是设计内路径），然后：
   ```183:192:crates/soul-profile/src/service.rs
            CheckedAnswer::Axis { axis, position, .. } => {
                place_axis(
                    &mut profile,
                    axis,
                    *position,
                    band_of(QUESTIONNAIRE_STRENGTH),
                    vec![evidence_id],
                    None,
                );
            }
   ```
   `place_axis` 无条件覆写 `position / evidence_band / evidence_ids`，`locked_by_user=None` 表示不动锁位。于是：用户纠正好奇轴（Strong、locked、LeansLow）→ 重答问卷选 Mixed → 轴变 Mixed、band 降 Moderate、证据换成问卷行，**而 `locked_by_user` 仍是 true**。产出状态自相矛盾（「已锁定」却载着非纠正的取值），且纠正的 Strong 被静默降档。按字面「后续**推断**不覆盖」未违约（问卷是 user_stated 不是推断），故定 P1 不定 P0——但这是唯一真实存在的第二写入者，恰好是没被锁住的那个。
2. **锁只能上不能下。** `correct_axis` 只写 `Some(true)`，不存在解锁 API。用户想让某轴重新接受证据修正，做不到（PRODUCT_LOCK「可纠正」的反向自由度缺失）。
3. **band 是调用方口头声明，不是算出来的。** `record_axis_inference` 对 band 无任何规则约束——`correction_lock.rs` L124-135 甚至用**单条** evidence 声明了 `SupportedBand::Strong` 并被接受。「证据档」在轴侧没有算法内容，只有非空性检查。这正是 A1（evidence-count band upgrade）要填的洞。
4. **两套问卷并存**（STATUS.md 自己承认「v0.1 收尾前应该并成一套题号，否则用户会被问两遍」）：`soul-import::questionnaire` 八题自由文本（不碰任何轴，`UserStatedSink` 无生产实现），`soul-profile::questionnaire` 七题闭集枚举。若向导实际走 import 侧，五条轴在 onboarding 后全部停在 Unknown，A0 的问卷半边在真实流程里是死代码——AC-03「非空档案」会被 voice/boundary 文本空转满足。
5. 连续两次 `correct_axis`、锁定后再 `intake`、审计区分「已应用 vs 因锁拒用」（两者都记 `InferenceWrite / Allowed`，审计回放无法区分）——均未测。

### 2.4 空转的承诺

- **「后续推断不覆盖锁定轴」被证明的对象是一个不存在的生产者。** 生产代码里没有任何东西构造 `AxisProposal`（`soulcore/commands/profile.rs` 只是转发层，无调用者；v0.1 无行为推断引擎）。锁是围着空地修的好栅栏——栅栏本身合格，但当下唯一会踩进这块地的是问卷重跑，而它从栅栏缺口走（2.3.1）。
- **轴侧 falsifier 可选且从未被写**：`intake` 与 `correct_axis` 都不产 falsifier；图侧强制写（却不可兑现，见 1.4.2）。两侧对同一 schema 字段一松一紧，无原则可循。

### 2.5 焦点问题：band 词汇是否与图一致？

拼写一致（同一个 `SupportedBand`），**语义不一致，而且不一致有两层**：

1. **轴侧**：band = 对断言的**支撑强度**（position 是断言，band 是把握）。问卷 Moderate < 纠正 Strong 的排序自洽且有据：凭记忆的自述弱于对着屏幕上的具体断言说「不对」。这个设计本身是 A0 里最值得保留的部分。
2. **图侧**：band **同时是断言本身**（`graph.tie.strong` = 「这是强关系」）**又是 inference 的 `evidence_band`**（= 「这条推断的证据档」）。于是一条被 500 行观察支撑的弱关系，其推断的 evidence_band = weak——把「关系弱」误写成「证据弱」。一个枚举扛两个概念，schema 层就把混淆固化了。
3. UI 后果：同屏出现「强（关系）」与「强（证据档）」，前者 10 条消息即达，后者意味着用户亲口纠正。用户会把两个「强」读成一个意思。WP10 已经在措辞上自救（往来密集/中等/不多），但轴侧渲染没有对应的改写。

**焦点问题：v0.1 不从聊天正文推断特质，是缺口还是特性？** 是特性，且应写进决策记录。理由：(a) A3（lexicon 推断）需要正文、天然临床化、与「无 evidence 不落库」的可解释证据观冲突，SHARED_BRIEF 已默认高风险；(b) 消息计量到人格的映射（如「发消息多 = 外向」）没有可辩护的规则形态，做了就是产品锁禁止的不可纠正黑盒的第一块砖；(c) 锁机制已就位，等一个值得放行的生产者不吃亏。但要诚实记下代价：在生产路径上，A0 = 问卷常量 + 纠正常量 + 一个锁位，把它称作「算法」名过其实——它的全部算法内容是锁语义，而锁语义当前有 2.3.1 的洞。

---

## 三、C1–C8 计分（1–5，按现状实现打，不按理想设计打）

| 维 | T0 | 证据 | A0 | 证据 |
|---|---|---|---|---|
| C1 产品锁契合 | 2 | 边有强度/类型/最近接触/证据 ✓；但「可纠正」缺失 + rebuild 复位 user_verdict（P0） | 4 | 纠正锁是锁文承诺的正面实现；扣分：intake 绕锁、无解锁 |
| C2 可测性 | 4 | 纯函数、无时钟、幂等测试在；扣分：Moderate 与全部边界未测 | 5 | 原子性/来源/拒绝路径测试是全仓最佳 |
| C3 可解释性 | 2 | 计数理论上可复核；群扇出 + 重导入翻倍后用户实际数不出来；引用全量 evidence 等于不解释 | 5 | 每轴引用可解引用到具体题目与答案，模范 |
| C4 隐私 | 4 | 无正文、会话 id 加盐哈希、local_only 全覆盖并可整图断言 | 5 | 仅 owner 主体，无第三人 |
| C5 稳健性 | 2 | 单条回复闩互惠、群扇出、频道/bot 不过滤、重导入翻倍、偏移时间戳、单向 DM 隐形 | 3 | intake 重跑绕锁；两套问卷并存 |
| C6 计算成本 | 2 | 全表 list_evidence 载入、O(P×I) 匹配、无界 evidence_ids、WP10 真 O(E²) | 5 | 五轴常量级 |
| C7 SOTA 对齐 | 3 | 跨天要求 ≈ Granovetter 时间维 ✓；无 recency、venue 不入判定，落后 RFM/T3 | 3 | 自述<纠正符合自陈量表常识；band 语义未定义 |
| C8 创新必要 | —（基线） | | —（基线） | |

---

## 四、给后续轮次的一句话定调

T0 值得留骨架：三档 + 互惠闩 + 跨期要求是对的分类学。要换的不是算法而是**计数的定义**（venue 分离、会话段、休眠降档、去重），以及补上图侧的纠正/锁定——这四件事全部能保持「用中文一句话解释每个档」。A0 值得留全部：修 intake 锁绕过（约 5 行 + 1 个测试）、合并两套问卷、给 band 一个哪怕最小的规则（A1），它就从「两个常量」升格为可以defend 的算法。
