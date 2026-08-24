MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 2 交叉验证 deltas — fable-b（BLOCKERS.md G1–G5 / S1–S2）

只列偏差。基准：PRODUCT_LOCK.md、FORMAL_WORK_PROMPT.md 验收矩阵、DECISION.md、
COPY_ZH.md、`crates/soul-algo-tie` / `soul-algo-trait` 实际源码。

## Δ1 G4 的 P0 引不出依据（既非锁违规，也不挡验收矩阵）

FORMAL_WORK_PROMPT 全文无「去重/幂等/重复导入」；AC-04/05/06/08 在重复导入下照常绿；
PRODUCT_LOCK 无导入幂等条款。§3 表头自定义 P0 =「不解决则不能声称验收矩阵通过」，G4 不满足。
处置二选一：(a) 走父代理 DECISIONS 给矩阵加一条「同文件导两次，事件数/档位/last_contact 不变」
的 AC，G4 保 P0；(b) 降 P1，但与 T4D 接线（工序 5）同批合入并入档「重复导入会伪造档位」。
不接受现状：挂着 P0 却点不出被挡的验收条目。

## Δ2 S2 是锁违规，平 P1 定级过低（漏引 AC-12）

AC-12 的 Then 明文「请求体无 ≥8 字原文子串，**也无未占位姓名/账号**」；锁文「姓名/账号同样
占位」+ 不可协商约束 6。KnownIdentifiers 恒空 → owner 自述中的第三人中文名（2–3 字，
≥8 字判据够不着）直接进默认 E1 请求体。与 S1 同构：应改双档「P1 合入 / **P0 关闭·发货**」
并点名 AC-12。若 CI 里 AC-12 现在是绿的，说明 fixture 没覆盖「owner 文本内嵌第三人名」，
fixture 同步补。

## Δ3 G4 内部定级自相矛盾（扇出 P1 vs 去重 P0）

G4 把「群聊重复行刷新任一场地 last_contact」算进去重 P0 的理由，却把扇出的同一机制
（对全体历史发言人伪造 Outgoing 行 → 压住 180/360 降档时钟）标 P1，「不制造 Strong」只考察了
次数门。DECISION §4.3 定价的是**真实同场**的近因；扇出制造的是虚构同场，降档失灵即档位错。
定级应同批：扇出收紧并入 Δ1 所选档位，或明文入档为发货已知限制。

## Δ4 G3「照抄 GC-1…GC-10」与模板冻结冲突（GC-9 现状不可执行）

COPY_ZH.md §4 冻结的 P5 原句是「按上面的计数……这是工作假设……」——对用户锁定边两个半句
都是假话；冻结的 `A2_STATEMENT_KEYS`（9 键）没有「由你本人指定」变体；GRAPH_CORRECTION.md §4
四句话术从未过冻结（其自述「冻结权归 fable-a」）。
处置：GC-1…GC-8、GC-10 照抄可行；GC-9 与 §4 话术改为「先经父代理 DECISIONS 增补 COPY_ZH.md
（加性新增 P5 锁定变体 + 对应 statement key），再动代码」（DECISION §6.4：先改 COPY_ZH 再改
代码）。过渡语义写明：变体落地前，锁定边**不得**渲染现冻结 P5 原句（那是对锁定档撒谎）。

## Δ5 G1.5「180 别名」在两 crate 现实下不可满足；§5 工序漏了合并义务

事实：`soul-algo-trait/src/a2.rs` 的 `DORMANT_AFTER_DAYS: i64 = 180` 是独立字面量；
`DEMOTE_ONE_BAND_DAYS` 在 `soul-algo-tie/src/constants.rs`。两 crate 零依赖 + M2「算法 crate
一个字节都不改」⇒ 跨 crate 别名做不到。DECISION §0/§6 明文把「统一 `crates/soul-algo`」列为
冻结后合并义务，BLOCKERS §5 十二道工序没有这一步。
处置二选一（走父代理 DECISIONS）：(a) 工序 5 前后补「合并两 crate」一步，别名在合并 crate 内
落实；(b) 接受双 crate 形态，在两 crate 之外加工作区级等值钉死测试
（`DORMANT_AFTER_DAYS == DEMOTE_ONE_BAND_DAYS`）。G1.5「禁止第二个 180 字面量」须改写为
只约束产品 crate，否则与 M2 字节冻结直接冲突。
（顺带：DECISION §3 写 `DORMANT_NOTE_DAYS`，crate 实名 `DORMANT_AFTER_DAYS`，引用用实名。）

## Δ6 G1.2 边界翻译漏两处，会让冻结话术渲染假话

(a) 实际 `Interaction` 是五字段，含 **`conversation_id`**；它喂 `conversation_count`，
冻结 `explain_zh` / A2 会把对话数念给用户。G1.2 只写「方向、是否一对一、Unix 秒」——须补
「会话 id → 稠密 u64」映射（Telegram chat id / JSONL 线程，与 peer 词典同纪律），禁止填
常量占位，否则全图对话数恒 1。
(b) 存在两个 TieScore：`soul_algo_tie::TieScore`（判档产出）与 `soul_algo_trait::a2::TieScore`
（渲染输入，direct/group 为 Option）。适配器必须把分列计数填成 `Some(..)`，否则 A2 静默丢掉
DECISION §2.3 保留的「一对一/群聊分列句」——加一条断言测试钉住。

## 核对通过（无 delta，仅记录）

- G2 与冻结 `apply_intake` 逐项吻合：锁轴跳过、答案仍落证据行、ignored 理由字符串
  `axis_locked_by_user` 与 `IntakeSkip::as_str()` 一致、replay 全等、明文禁改 NoDowngrade——
  冻结默认 `A0_DEFAULT_WRITE_MODE = LastWriteWins` 不被触碰。
- G1.1/1.3/1.6 依赖方向与 API 实存核对通过：`score` / `score_ego_network` / `as_of_max` 均在；
  箭头恒为 Goal 1 → 算法 crate；G1/M2 全文无任何会把 store/SQLCipher/Tauri/HTTP/墙钟拉进
  算法 crate 的措辞。
- G1、G3、G5、S1 的锁依据核对通过（DECISION §6.5、灵魂层「可查看和纠正」、AC-03 原文、
  数据面 DPAPI KEK）。
