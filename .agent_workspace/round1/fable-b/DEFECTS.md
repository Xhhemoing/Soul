# DEFECTS — T0 / A0（Round 1 / fable-b）

分级依据（本轮派发定义）：P0 = 违反 PRODUCT_LOCK 或 AC 矩阵；P1 = 会在真实数据上错排关系或轴；P2 = 打磨。每条给出：现状 → 为什么错 → 保持可解释性的修法。

---

## P0

### P0-1 人脉图不可纠正，rebuild 复位一切用户裁决

- **现状**：不存在纠正/锁定一条边或其 band 的任何 API。`build.rs` rebuild 按 pair 匹配到既有边和 inference 后**整行覆写**，包括 `user_verdict: Some(UserVerdict::Unreviewed)`（`build.rs` L322）与全部 `tie_strength` / `types`。tie inference 的 falsifier 甚至承诺「或用户直接改写这条关系，即推翻本判断」——但改写入口不存在，手工改写也会在下次导入时被覆写。
- **为什么错**：PRODUCT_LOCK 灵魂层表格「人格、价值观、心理工作模型、自传记忆、**人脉图**、偏好。用户**可查看和纠正**」；不可协商约束 5「档案可查看、纠正」；SHARED_BRIEF 硬约束「工作假设必须可纠正」。tie band 是工作假设，当前不可纠正。schema 的 `user_verdict` 字段与 falsifier 文本让承诺**看起来**已兑现，实际是装饰——这比缺失更糟。
- **修法（保持可解释）**：与轴侧对称：`correct_tie(relationship_id, band)` 写一条 `UserCorrection` evidence + 在边上加 `locked_by_user`（relationship schema 的 `tie_strength` 是自由 object，可无痛加字段）；rebuild 继续更新计数与时间戳，但**锁定边的 band 不动**，机器认为的 band 照旧写进 tie inference（存而不用，与 `RefusedAxisLocked` 同一句式）；rebuild 对已有 `user_verdict != unreviewed` 的 inference 保留原裁决。用户解释一句话：「这条关系的档位是你亲手定的，导入不会改它；机器的看法在旁边。」

### P0-2 群聊扇出使 Strong 对整群发言者成立，「边=互动强度」失真且计数不可复核

- **现状**：`commit.rs` L198-207——owner 在群里的每条消息，对该会话**所有曾发言者**各生成一条 outgoing 观察（含 owner 发言之前退群/之后入群的人，因为 `speakers_by_conversation` 按整个文件汇总、无时间约束）；任何人在任何共享群里的每条消息都计入与 owner 的 incoming。`build.rs` 的 band 对 venue 视而不见，把这些计数与 DM 1:1 合并。
- **为什么错**：算术后果——owner 在 200 人活跃群发 10 条、跨 3 个 UTC 日，则**每个发过一句话的群成员**都满足互惠 ∧ count≥10 ∧ days≥3 → Strong。PRODUCT_LOCK 定义「边=互动强度」，而这些边表达的是「同群共存」不是互动；SHARED_BRIEF 硬约束「算法必须可向用户解释（能用中文说清为什么是强/中/弱）」——WP10 会对陌生群友输出「你和这个人一共有 137 次往来」，用户在 Telegram 里数不出这个数、也无法被说服。测试 `a_message_sent_to_a_group_is_evidence_of_contact_with_everyone_in_it` 把该行为钉成了规范，需要连测试一起改。
- **修法（保持可解释）**：数据已齐（`Venue` 逐条在库），只改判定与措辞：(a) tally 分列 `direct_count` 与 `group_count`；(b) Strong 只由 direct 计数达成（Granovetter/T3：强关系=互惠+多日+**私聊**）；group-only 且群内有互惠往来 → 封顶 Moderate；仅同群共存 → Weak；(c) 中文解释模板照写：「你们一对一往来 X 次、群里同场 Y 次；强/中/弱只看一对一的部分」。可选增强：importer 读 Telegram 的 `reply_to_message_id`（纯元数据，无正文），把「在群里直接回复对方」计入 direct 等价计数。

---

## P1

### P1-1 `intake` 重跑绕过轴纠正锁（A0）

- **现状**：`service.rs` intake 对轴调用 `place_axis(..., locked_by_user=None)`，无 `axis_is_locked` 检查；`place_axis` 无条件覆写 position/band/evidence。用户纠正（Strong、locked）后重答问卷 → 位置被改、band 降 Moderate、`locked_by_user` 仍为 true。
- **为什么错**：产出自相矛盾的档案状态；纠正的 Strong 被静默降档。这是 v0.1 唯一真实存在的「第二写入者」，恰好不受锁约束——被测试守住的 `record_axis_inference` 反而没有生产调用者。
- **修法**：intake 内对 `axis_is_locked` 的轴跳过 `place_axis`（答案照常落 evidence，如同 `RefusedAxisLocked` 的问卷版），返回值告知哪些轴因锁未动；或者明确「重答=解锁并覆写」，但必须原子地把 `locked_by_user` 翻回 false 且在审计里记 `ProfileCorrect`。二选一都可解释；不可接受的是现状的「半覆写」。

### P1-2 无 recency：死关系永远 Strong，falsifier 不可兑现

- **现状**：band 只看累计 count 与累计 active_days；`last_contact_utc` 在边上但不参与判定。falsifier 文本承诺按时间窗口推翻，无代码实现。
- **为什么错**：2019 年的旧友与昨天的挚友同档且**永不降档**（计数单调不减）；WP10 摘要、未来的起草语气选择都会被陈旧的 Strong 污染。图随数据积累单调变错。
- **修法**：休眠降档（不用指数衰减，保住整数可解释性）：以「本次导出文件的最大时间戳」为 as-of（确定性、可测，不读时钟），`as_of - last_contact > 180d` → 封顶 Moderate，`> 365d` → 封顶 Weak；解释一句话：「你们已经 N 个月没有往来，档位从强降为中；再次往来会自动恢复」。这同时把 falsifier 前半句变成真的。

### P1-3 互惠是单条消息闩，不是双向性度量

- **现状**：`is_reciprocal() = outgoing > 0 && incoming > 0`。out=50 / in=1 → 互惠成立，可达 Strong。
- **为什么错**：把「对方回过一句哦」与「对等往来」同权。单方面追着说话的关系会被排进最强档，正是 Granovetter 意义上最不该 Strong 的形态。
- **修法**：每侧下限：Strong 要求 `min(out, in) >= 3`，Moderate 要求 `min(out, in) >= 1`（即现状）。解释照旧是计数：「对方也至少回了 3 次」。

### P1-4 重复导入无去重，累积导出使计数翻倍

- **现状**：`commit.rs` 模块注释自认双写；`soulcore` commit 不去重；Telegram 导出天然累积（新导出含全部旧消息）。
- **为什么错**：最常见的用户行为（定期重新导出）直接把重叠区间计数 ×2，band 升档，且用户无法复核。rebuild 的幂等性只保护边的条数，不保护计数的真实性。
- **修法**：evidence/event 落库前按 `sha256(source | external_id)` 去重（哈希而非平台 id 明文，与 `conversation_ref` 同一先例；`external_id` 已在 `StagedMessage` 上，目前落库时被丢弃）。回执报告 `duplicates_skipped`，审计记条数。

### P1-5 单向 DM（对方从不回复）在图上完全不存在

- **现状**：peer 只有作为 sender 出现才成为 participant / speaker；owner 在 DM 里的消息若对方从未发言 → peers 为空 → 零 evidence、零 contact、零边。`TieType::OneSided` 的 outgoing 方向不可构造。
- **为什么错**：用户单方面维系的联系人（只看不回的长辈、暗恋对象、催款对象）是人脉图上真实且有情感意义的节点，现在整个隐形。模型枚举与管道能力不一致。
- **修法**：Telegram `personal_chat` 的 chat `id` 就是对端 user id——importer 为无发言的 DM 对端登记 participant（有名字则密封名字），owner 消息的 peer 取该对端。JSONL 侧可在 v1.1 加可选 `peer_id` 字段；无字段时保持现状并在 defect 报告里提示。

### P1-6 非 Z 偏移时间戳入库，破坏字典序比较与 UTC 日切

- **现状**：`instant.rs::is_date_time` 接受 `+08:00` 与小数秒；`soul_import_v1.rs` 原样入库。`build.rs` L97-98 声称全仓 writer 归一化到 Z 并据此做字符串比较与前 10 字符日切。
- **为什么错**：`...T09:00:00+08:00`（=01:00Z）会被字典序排在 `...T02:00:00Z`（=02:00Z）之后 → `last_contact_utc` 错；`utc_date` 切出的是带偏移的本地日期 → 活跃天桶混时区。声称的不变式已被同仓库代码违反，只是 fixture 恰好全用 Z 没炸。
- **修法**：importer 边界统一归一化到 Z（解析偏移做减法即可，无需日期库的历法功能），或 schema 收紧 `occurred_at` 只接受 `Z` 结尾并在 defect 里报可读句子。补一个 `+08:00` fixture 测试。

### P1-7 tie band 语义双载：断言与证据档共用一个值

- **现状**：`tie_inference` 把边的 band 同时写进 `statement_key`（`graph.tie.strong` = 断言）与 `evidence_band`（= 证据档）。500 行观察支撑的弱关系 → `evidence_band: weak`。
- **为什么错**：`evidence_band` 本应回答「这条推断有多可靠」，现在回答的是「关系有多强」。与轴侧语义（band=把握度）冲突，同一枚举在同一 UI 里两个意思，用户必然误读。
- **修法**：tie inference 的 `evidence_band` 改为按支撑度计算（例如观察 <10 条 → weak，≥10 条 → moderate，≥10 条且跨 ≥3 天 → strong——即「对这个判断我们看了多少数据」），断言仍在 `statement_key`。或最小修：文档钉死「图侧 evidence_band 恒等于 band」并让 UI 只显示一处——但这只是把混淆写成规范，不推荐。

### P1-8 规模成本：无界 evidence_ids、全表载入、二次方匹配

- **现状**：边与 tie inference 各存全量 evidence UUID（挚友边可达数万条，每次 rebuild 重写两份多 MB JSON）；`rebuild` 全表 `list_evidence()` 进内存；边/inference 匹配 O(P×I) 且每次比较分配字符串；WP10 `rows_where` 真 O(E²)。
- **为什么错**：C6 明令禁止隐式 O(n²)；真实 Telegram 导出（百万消息、数千 peer、含频道）会把 rebuild 与人事摘要拖进分钟级与 GB 级内存。
- **修法**：(a) 边引用「代表性证据」：每个活跃日每方向至多 1 条 + 首末各 1 条（解释语义不变：「这 N 天里每天的样本」），全量明细永远可经 conversation_ref 溯源；(b) 匹配用 `BTreeMap<(from,to), edge>` 与 `BTreeMap<relationship_id, inference>` 预索引；(c) WP10 用 `BTreeSet` 查 evidence id。频道/bot（`from_id` 前缀 `channel`/已知 bot 标记）降为不建边或单独 ContactClass。

---

## P2

### P2-1 Moderate 档零测试、全部阈值边界零测试
`ego_graph.rs` 只断言 Strong 与 Weak；count=2/3、9/10、days=2/3 无一覆盖；`STRONG_MIN_ACTIVE_DAYS` 的立法场景（单日 20 条互惠 → Moderate）没有 fixture。修法：一组纯 `Tally::band()` 参数化测试即可，不需要store。

### P2-2 rebuild 的降级策略不一致：缺 contact 计数跳过，坏观察整体致命
`peers_unresolved` 温和上报，但 `SelfLoop` / `DanglingContact` 直接 `Err` 中断整个 rebuild——一行坏 evidence 永久毒死全图且用户无法自愈。修法：同样计数上报（`observations_rejected`），审计留痕。

### P2-3 审计无法区分「已应用」与「因锁存而不用」
两者都记 `InferenceWrite / Allowed`。AC-23 审计回放读不出锁生效过。修法：拒用路径换 `ReasonCode`（如 `axis_locked`），决策仍是 Allowed（写入确实发生了）。

### P2-4 轴锁无解锁路径
`correct_axis` 只写 `Some(true)`。用户想恢复「让证据说话」做不到。修法：`release_axis` 写一条 UserCorrection 证据并置 false，审计记 `ProfileCorrect`。

### P2-5 两套问卷并存（STATUS 已自认）
八题自由文本（import 侧，不碰轴）与七题闭集（profile 侧）互不引用、证据两套、用户会被问两遍。若向导走 import 侧，五轴 onboarding 后全 Unknown。修法：profile 侧实现 `UserStatedSink`，题号合并为一套；AC-03 断言收紧为「五轴离开 Unknown 或明确记录未答」。

### P2-6 遗忘后陈旧边残留（需 Round 2 核实边界）
rebuild 只为当前 tally 的 peer 写边，从不删除失去支撑的边；被遗忘 contact 的 relationship 行（含计数、首末时间戳）是否被 store 的 forget 流程清除未见测试（`soul-store/src/forget.rs` 会经 `relationship_evidence` 把 inference 置 orphaned，但边行本身的去留未验证）。修法：rebuild 对「存在于库、不在 tally」的边显式删除或墓碑；补「遗忘→rebuild→图中无此人」测试。

### P2-7 tie falsifier 是硬编码散文，不携带实际阈值
所有边共用同一句中文，不含「多久算更长的时间窗口」。修法：由常量拼出（「若 180 天内无新往来……」），与 P1-2 的休眠降档同源。

### P2-8 audit `.about(&build.edges_written)` 随边数无界增长
数千 peer 时单条审计携带数千 UUID。修法：`about` 记 build 批次 id，`counts.items` 记边数。
