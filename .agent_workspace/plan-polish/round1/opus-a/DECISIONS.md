# 自主拍板记录

作者 2026-08-24 声明：除已给出的产品方向外，其余问题可自主决定。本文件记录拍板，避免后续子代理重开方向战。

2026-08-25 增补 D32–D48：把 `docs/algorithms/DECISION.md`（`ALGO_FROZEN`）与 `docs/BLOCKERS.md`（`BLOCKERS_FROZEN`）里已经拍过的板收进同一张表，**不改产品方向、不新增工作包**。每条的出处写在「理由」列。

| ID | 问题 | 决定 | 理由 |
|---|---|---|---|
| D1 | Soul 是助手还是数字复刻 | 数字复刻为灵魂层，助手为代理层，复刻是本体 | 作者定义是电子版的你 |
| D2 | 要不要默认云端 | 默认全本地；v0.1 无云端 HTTP | 本地优先 |
| D3 | 社交数据怎么进来 | 官方导出或手动导入；v0.1 无 OAuth；禁止爬虫 | 合法可审计 |
| D4 | 能否自动回消息 | v0.1 只起草不发送 | 代发不可逆 |
| D5 | 心理模型是否临床 | 否。可替换特质轴 + 证据档 + 用户纠正 | 避免医疗声称 |
| D6 | 手机何时做 | Android 在 v0.3 | Windows 先闭环 |
| D7 | 研究数据如何用 | v0.1 只预览不写文件；与助手出网分离 | 防偷渡 |
| D8 | v0.1 是否要真预测模型 | 只要 schema 与离线入口 | 先有数据契约 |
| D9 | 技术栈 | Tauri2 + Rust + React | 常驻与打包 |
| D10 | 文件整理是否产品本体 | 否。v0.1 只做只读计划预览；执行在 v0.1.1 | 代理层证明不靠真写文件 |
| D11 | 出网怎么分级 | E0/E1/L，全部经 net_guard | 否则「零出网」不可测 |
| D12 | v0.1 的 E0 | 无实现、无域名、无 HTTP client | 关开关不够 |
| D13 | 第三人正文进 E1 | 默认占位；单次整条豁免；研究无豁免 | 第三人未同意 |
| D14 | 主库 | 加密 SQLite；JSONL 非存储 | R1 S3 |
| D15 | 遗忘 | 销毁内容密钥；不承诺物理擦除 | 可测 |
| D16 | 遗忘 vs 审计 | 审计无内容，可留孤立 UUID | 两承诺共存 |
| D17 | v0.1 导入 | soul-import-v1 + Telegram result.json | 具名才可测 |
| D18 | 研究导出形态 | 预览 only | 脱敏未红队前不落盘 |
| D19 | WP11 写执行 | Goal 1 不做 | 防误读成产品本体 |
| D20 | 记忆是否门禁 | 是 | 作者原意第 6 条 |
| D21 | 人事分析是否门禁 | 是；无 key 则统计降级 | 作者原意第 5 条 |
| D22 | 大五怎么写 | 方向轴 + 弱中强，禁 score/percentile | 非量表 |
| D23 | 采集面 | 仅前台应用时长 | 同意闭环一项即可 |
| D24 | HITL | 未知拒绝；plan_hash 变拒绝；令牌一次性 | 可测 |
| D25 | 注入 | 外部内容永不进指令位；红线进门禁 | R1 S14 |
| D26 | 写码前文档 | 锁、拍板、九 schema、SECURITY、STATUS | 14 份会空转 |
| D27 | 第二份 PRODUCT.md | 禁止 | 防双源 |
| D28 | Goal 2 | 拆到 GOAL2_POLISH_PROMPT.md | Goal 1 必须有终点 |
| D29 | 验收权威 | FORMAL 中 Given/When/Then 矩阵 | 散文不作门禁 |
| D30 | 工作包增减 | R2 后只减不增 | 防回弹 |
| D31 | 只读整理预览是否留在 Goal 1 | 留。执行仍在 v0.1.1。这是代理层只读证明，不是产品卖点 | 否决 R3-sol 删除 AC-18 的建议 |
| D32 | 计划权威放在哪 | 只在 `docs/`。`.agent_workspace/context/plan/` 与各轮过程稿一律只是过程稿，与 `docs/` 冲突时以 `docs/` 为准 | 同一事实两处漂移已经发生过；与 D27 同一条防双源纪律 |
| D33 | 人脉档位用哪套 | T4D，唯一权威。三道门（互惠、次数、自然日）全在一对一计数上；群聊行只供展示与近因，不判档 | `DECISION.md` §1–§3，`ALGO_FROZEN` |
| D34 | 特质轴与人事摘要 | 特质轴 A0（问卷 + 纠正锁）；A2 是档位的纯渲染器，源码级禁止第二套阈值；A1 是 A0 内部升档规则，v0.1 只有问卷一种来源，**空转是预期行为** | `DECISION.md` §2；把「A1 没触发」当缺陷会引出新算法 |
| D35 | F04c（久别之后一来一回回强） | 不加第三道降档门。不满意只能走 `DECISION.md` 第 5 节回退链，并在本文件留痕 | `DECISION.md` §4.2 / §5：静默修补等于换算法，须重开消融 |
| D36 | `as_of` 口径 | 一次重建全库只有一个 `as_of`（缺省 = 全库 `max(occurred_at)`，由调用方传入）。禁 per-peer，禁墙钟 | `DECISION.md` §3：per-peer 会让休眠关系假 Strong |
| D37 | intake 与轴锁的关系 | 锁定的轴不 `place_axis`；答案仍落证据，`ignored` 理由 `axis_locked_by_user`；保持 `LastWriteWins`，不开 A1 / `NoDowngrade` | 纠正锁定优先是 D5 的产品承诺；`BLOCKERS.md` G2 |
| D38 | owner 群消息如何归因 | 不对该会话的历史发言人写 Outgoing；incoming 仍记实际发送者。不加成员表，不在算法 crate 里加权重 | `BLOCKERS.md` G1+：伪造的 `last_contact` 会让关系永不降档 |
| D39 | 人脉图纠正 | 生效档 = 用户锁定时的档，机器档另存，A2 与图只消费生效档。GC-9 话术须先**加性**增补 `COPY_ZH.md`（新 P5 key）再改渲染；变体落地前锁定边不得渲染现冻结的 P5 原句 | `BLOCKERS.md` G3：照抄冻结原句等于对锁定档撒谎 |
| D40 | 唯一实现主干 | `cursor/soul-goal1-7b1c`。停 `agent/dev-sota`、关 PR #4；独有项事后樱桃摘，不挡合入。分支一律 `cursor/` 前缀 | `BLOCKERS.md` M1：两条实现线合不拢是最大合入阻塞 |
| D41 | 算法 crate 何时进应用工作区 | 按 `BLOCKERS.md` M2，在 Goal 1 线上一次 merge 时把两个 crate 加进成员表。**不在计划 PR 里做** | 计划 PR 不夹带应用代码（D26 同一条）；rebase 96 个共享提交代价过高 |
| D42 | 算法 crate 的依赖方向 | 箭头恒为应用 → 算法 crate。crate 保持纯函数、`forbid(unsafe_code)`、不读墙钟；禁止 SQLCipher、Tauri、HTTP 依赖进入 | `DECISION.md` §6.2：评分器只吃 `(peer 行, as_of)`，落库与取证留在应用侧 |
| D43 | 两个算法 crate 现在合并吗 | 不合，后置。两侧的 `DEMOTE_ONE_BAND_DAYS` 与 `DORMANT_AFTER_DAYS` 用工作区测试钉相等；产品 crate 里禁止再写第三个 `180` | `DECISION.md` §0：合并是冻结后的义务，不是选型阻塞 |
| D44 | 冻结文稿名 vs 仓库真名 | 实现跟真名：`soul-algo-tie` / `soul-algo-trait`、`crates/soul-graph/src/build.rs`。`DECISION.md` §6 写的 `crates/soul-algo` / `graph_build.rs` 是文稿名，不改冻结文件正文 | `BLOCKERS.md` §0；改 `ALGO_FROZEN` 正文要重走冻结程序 |
| D45 | Goal 1 怎么算关闭 | FORMAL 验收矩阵与 PRODUCT_LOCK 十三片切片**同时**过。矩阵没有的行不能否决切片，切片没有的行不能否决矩阵 | `BLOCKERS.md` 开篇：两套门各有盲区，互补不互斥 |
| D46 | 重复导入是否必须幂等 | 是 P1 设计目标，不进 v0.1 切片、不进验收矩阵。去重键设计见 `BLOCKERS.md` G4，且不得在算法 crate 里加权重补偿 | D30 只减不增；去重要身份/事务设计，不是小修 |
| D47 | AC-26 的「打包绿」怎么读 | 读作「package job 绿 + 作者签名安装包」。CI 里不下 NSIS、不跑完整 `tauri build` | `BLOCKERS.md` S5：在 CI 下 NSIS 是出网循环，且真机打包本就靠作者 |
| D48 | STATUS 怎么写 | 只写本文件所在树可验证的事实；跨分支进度必须带分支名与提交号并标「核于」日期；「阻塞：无」只覆盖本树 | `BLOCKERS.md` §0：旧 STATUS 的「阻塞：无」被误读成「可以合进 `main`」 |

编号纪律：`BLOCKERS.md` §0 / §1 提到的那个 `D32`（`agent/dev-sota` 线上的 e0 发送 crate 禁令）**不是**本文件的 D32。PR #4 关闭后若要樱桃摘那一条，按下一个空闲 ID 重新编号，禁止复用。

新增拍板只在本文件追加，禁止在 `.agent_workspace/` 或其他文档另立第二份（D27 / D32 的同一条纪律）。
