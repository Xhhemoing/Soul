MODEL_SLUG: claude-fable-5-thinking-xhigh

# Round 2 fable-a — R1 diff 复审（只记缺口）

范围：R1 的四个 diff——`7aee812`（M3 探针）、`62840ff`（G2 intake 锁 + 被并发扫入的 G1+ 代码与测试）、
`90da257`（G1+ 收尾：soulcore 断言 + STATUS）、`4da2184`（G1 预备 `t4d_adapt`）——对照
`docs/algorithms/DECISION.md`（ALGO_FROZEN）与 BLOCKERS 的 G1/G2/G1+/M3。只读复审，未改任何产品 crate。

验证方式：静态审阅全部四个 diff 与 `fable-a-T4D_WIRING.md`；Linux 定向测试全绿——
`soul-profile correction_lock`（5/5，含 `re_answering_the_questionnaire_does_not_move_a_corrected_axis`）、
`soul-graph` lib（2/2，t4d_adapt 单测）、`soul-import import_to_graph` 与 `soul-fileplan unauthorized_paths`（6+9）。
复审期间分支在继续推进：`4f0bee8`（denylist 豁免）、`fa25c2e`/`5724201`（T4D 产品门测）、`814064e`（band 换血
+ verdict/锁档保全）已落——它们是 R2 产出，本文不复审，只标注它们让哪些 R1 缺口失效。工作区有并发未提交改动
`crates/soul-draft/src/analysis.rs`，未触碰；依赖 soul-draft 的 soulcore 测试因此没跑。

一行结论：四个 diff 与 BLOCKERS/DECISION 方向全部一致，无违禁路径（无第三道门、无常量改动、算法 crate 零
diff、M3 没走 `cfg` 假探针、G1+ 没走「保留伪造行只滤时钟」）。缺口 12 条：2 条要在 G1 收口前处理（#7、#8），
3 条进 R3 工单（#9、#10、#11），其余是测试/文档补强。

## R1 简报里已失效的缺口（不要再当工作单）

- 「Band still T0」→ `814064e` 已换 `soul_algo_tie::score`（R2 opus 产出，另行复审）。
- 「denylist hits TieScore」→ `4f0bee8` 已豁免 `crates/soul-algo-*`（合成 `score` 字段在产品 crate 是否仍红，归 R2 gpt-sol-a 的回归测试验收）。
- 「G3 Unreviewed still」→ `814064e` 自称同时保全 `user_verdict` 与锁定档；是否满足 fable-b-G3.md 的 R1–R6，归 G3 复审，不在本文范围。

## M3（7aee812）

1. 【补强】`--no-fail-fast` 没进 `test-windows`。BLOCKERS M3 建议与夹具**同一次推送**给 test-windows 里每个
   `cargo test` 加 `--no-fail-fast`（可观测性，非第二道 P0 门）。`ci.yml` 现在仍是裸的
   `cargo test --workspace --all-targets`。一行改动，仍然值得补。
2. 【低】探针依赖一个未声明的假设：`fs::canonicalize` 在大小写折叠的文件系统上返回**盘上真实大小写**
   （NTFS 经 `GetFinalPathNameByHandle` 成立，现代 macOS 经 F_GETPATH 成立）。若某个折叠文件系统的
   canonicalize 保留调用方大小写，探针会误报 true，把 `decoy.txt` 写进已授权根——恰是 M3 要修的事故，且静默。
   ubuntu-latest/windows-latest 都不受影响，故只记不催；想免假设可换「写标记文件后经另一拼写 readdir」探针。
3. 【环境】M3 的关闭证据仍被空跑者挡着：windows-latest 从未真跑过 `unauthorized_paths` / `dpapi_key_chain`，
   BLOCKERS 工序 1 的 `Session::open_with_keys` 决定（先看 `Session::open` 通不通）也因此悬置。代码侧无事可做，
   分钟数是父代理/作者的事。

## G2（62840ff）

4. 【测试缺口】BLOCKERS G2 的验收句「与 `apply_intake` replay 在映射 ID 后全等」**没有对应测试**。
   `soul-profile` 不依赖 `soul-algo-trait`，工作区里没有任何测试把一次 `soul_profile::intake` 的
   applied/ignored/终态位置映射（Uuid→按序 u64）后与冻结的 `soul_algo_trait::apply_intake` 对拍。语义肉眼一致
   （同一锁检查、LastWriteWins、答案照录、`IntakeSkip::AxisLockedByUser`/`axis_locked_by_user` 连拼写都对齐），
   但没被钉住就可以漂。便宜做法：拿 `correction_lock.rs` 的夹具在工作区级测试里两边各跑一遍断言全等。
5. 【产品面】soulcore `IntakeReceipt::of` 丢掉 `outcome.ignored`，且 `answered: outcome.evidence_ids.len()`
   把被拒的那条也计成「answered」——锁轴重填后的回执与全部生效的一次**不可区分**，向导/`/profile` 没有任何
   地方能说「我们保留了你的纠正」。STATUS（`98a7f27`）已如实记为 WP03 遗留 8；仍开着，接 `ignored` 进
   `IntakeReceipt`（加性字段）是 G5/收口残余的一部分。
6. 【记录即可】被拒答案在审计链上无痕（链上仍是一条 intake），与被拒推断的既有形状一致、STATUS 已声明。
   若父代理要链级可见性，须走 DECISIONS 新增动作，不要顺手发明。

## G1+（62840ff 代码 + 90da257）

7. 【收口前处理】`soul-import-v1` 的群/一对一分类只看「发过言的非 owner 人数 > 1」（`crowded_conversations`）。
   一个真实群在导出窗口内只有一个 peer 说过话 → 整个会话判成 Direct：owner 的行变成对该 peer 的
   `Outgoing/Venue::Direct`，peer 的行是 `Incoming/Direct`——**双双进冻结的 3/10/3 一对一门**。这是扇出问题
   剩下的最后一条缝：单个活跃发言人的播报群可以铸出没有任何一对一会话挣来的 Strong。Telegram 侧无此问题
   （`group = type != "personal_chat"`，场地诚实）。R1 之前就存在、两种格式都没有成员表，但 G1 收口的边界
   夹具应加一条钉死现状并在 STATUS 定价；要改判据（比如 v1 行加显式 venue 字段）得动冻结契约，走 DECISIONS。
   附带同源小面：一对一会话里 peer 全程沉默 → owner 的行归给空集，该 peer 连 Weak 边都没有（T4D 反正判不出
   档，纯展示诚实面，记录即可）。
8. 【收口前处理，R3 拦截】owner 群消息不再写行 ⇒ 每条边的 `group_out_count` 结构性为 0，任何「群聊次数」
   只数 peer 的行。冻结 COPY_ZH 的 `{群聊次数}` 定义是「群里**同场往来**条数 / 群消息条数」（§0、§7-79 的
   用户复核路径是「数群消息条数」）——用户在自己客户端里数得出的数包含自己那侧，渲染值将系统性**偏小**，
   「算法必须可向用户解释」在这一句上会被打回。R3 把 A2 接上分列句**之前**，须经父代理 DECISIONS 给
   `{群聊次数}` 一个加性澄清（:= 库中该 peer 的群聊行数，即「TA 在群里说过 {n} 句」的口径），或改模板措辞；
   两者都不能在 COPY_ZH 冻结下静默做。

## G1 预备（4da2184 适配器）与 T4D 接线规格（c21bbc1）

9. 【R3 前必修，可达的库投毒】fail-hard 重建（规格 §1.3.3，`814064e` 已实现）×
   Telegram `date_unixtime` 无界。`read_instant` 接受任意 i64，`rfc3339_utc` 会渲染出 5+ 位或带负号的年份，
   而 `parse_rfc3339` 只认 4 位年 → `InvalidTimestamp` → **此后每次 rebuild 永久失败**。一条构造的（或损坏的）
   导出行即可：session 导入不是事务（STATUS 遗留 7），证据已落库，rebuild 在其后才炸，用户没有遗忘联系人之外
   的解法。修法在产品 crate：telegram 导入对超出可渲染/可解析范围（如 unix 秒对应年份不在 1970–9999）的
   `date_unixtime` 记 defect 拒文件。v1 路径被 `should_validate_formats(true)` 的 jsonschema 挡着，但
   `instant::is_date_time` 自己放行任意月份的 31 日（`2026-02-31`）——补一条负例测试钉住「schema 层确实拒它」，
   免得哪天 format 校验被关掉时静默漏进同一个火坑。
10. 【R3 定价或修】单条未来时间戳可摆动全库 as_of：一条 `occurred_at` 在 9999 年（合法 RFC 3339，全部校验放行）
    成为 `max(occurred_at)`，把库里每条真实关系降到 Weak（沉寂 ≥360）。冻结默认（DECISION §3 为归档导入定价）
    没有为对抗性导入定价，产品边界也无测试。便宜的诚实修法（产品侧）：v1 对 `occurred_at > exported_at` 记
    defect；Telegram 无导出时间戳，至少在 STATUS 定价一句。归 G4（P1）序列，不挡 G1 收口。
11. 【低】`AdaptError::InvalidTimestamp { timestamp }` 的 Display 携带原始存储时间戳，且 `lib.rs` 公开导出了
    该类型。`814064e` 在 build.rs 已按规格 `map_err(|_| …)` 丢弃载荷映射成无散文的
    `UnreadableInteraction { evidence_id }`——残余风险只是未来调用方直接打日志/落审计。给 AdaptError 补一句
    「不得入链」文档或干脆去掉载荷。
12. 【测试缺口】BLOCKERS G1 的产品边界夹具单要「2/3、9/10、日 2/3」近阈值与「179/180/359/360」四点闭区间；
    规格 T1–T12 只有 T5 的 180/360（降档侧），179/359（不降侧）与三组近阈值都缺；`fa25c2e` 落的四个产品门测
    （strong / 群洪 Weak / 仅群聊 Weak / 休眠 Weak）同样没盖。这些在算法 crate 内部有，但 BLOCKERS 点名要在
    **store 这一侧**穿过字符串→unix→整数天的管道各钉一次。给 R2/R3 的测试工单补上。

## R3 工单需要带走的一句（规格未写明处）

13. 【R3】遗留零 vs 实测零：`TieStrength` 分列字段是裸 `u64` + `serde(default)`，换血前写入的边加载后是
    0/0/0/0，与实测零（仅群聊边的一对一计数就是 0）无法从计数本身区分。COPY_ZH P1b「任一数缺席则整句不出现」
    的「缺席」，A2 重接时必须以 `as_of_utc.is_some()`（或 `algorithm_id == "T4D"`）为准，**不得**以计数为准——
    否则要么对遗留边渲染捏造的 0 次分列句，要么把仅群聊边的真实 0 吞掉。
